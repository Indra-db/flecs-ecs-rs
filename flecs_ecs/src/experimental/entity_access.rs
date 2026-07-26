//! Shared-register entity access: `get_ref` / `try_get_ref` / `cloned_owned`.
//!
//! These replace the closure-CPS `EntityView::get` with guard returns. A
//! successful call hands back [`Ref`] / [`Mut`] guards (or a tuple of them);
//! the borrow is registered in the stage lock map on acquire and released when
//! the guard drops, so borrow lifetimes follow ordinary Rust scoping / NLL.
//!
//! Tuple requests may mix required and optional elements: `&T` / `&mut T` gate
//! the whole acquire (a missing one yields `None` /
//! [`AccessError::MissingComponent`]), while `Option<&T>` / `Option<&mut T>`
//! yield an `Option<Ref<_>>` / `Option<Mut<_>>` element that is `None` when the
//! component is absent. An absent optional takes no lock and no pin and cannot
//! conflict; only present elements register a borrow and increment the stage
//! pin (spec §7.1). The acquire opens **no** flecs defer level: the read path is
//! zero defer FFI, and a shared-register write lazily opens one episode-scoped
//! level only while a guard is live. Owned copy-out of optional/absent
//! components stays on [`EntityView::try_cloned`], surfaced here as
//! [`EntityGuardExt::cloned_owned`].

use core::ffi::c_void;
use core::ptr::NonNull;

use crate::core::{
    alias_violation_panic, stage_locks_dyn, ClonedTuple, ComponentOrPairId, DataComponent, Entity,
    EntityView, GetComponentPointers, GetTuple, GetTupleTypeOperation, LockKey, SafetyInfo,
    StageLocks, WorldProvider, WorldRef,
};
use crate::sys;

use super::guard::{AccessError, Mut, Ref};

/// Decrements the `count` stage pins taken so far on drop unless disarmed with
/// [`PinRollback::disarm`]. Covers every early-return path (a borrow conflict)
/// and any unwind after the first pin is taken, so a mid-acquire panic cannot
/// leave a stale pin (spec §7.1, §7.2). No FFI: releasing a pin only touches the
/// Rust counter, and closes the episode level solely if the pin reaches zero
/// with a level open, which cannot happen mid-acquire (no write is issued here).
struct PinRollback {
    locks: NonNull<StageLocks>,
    world: *mut sys::ecs_world_t,
    count: usize,
}

impl PinRollback {
    #[inline(always)]
    fn disarm(mut self) {
        self.count = 0;
        core::mem::forget(self);
    }
}

impl Drop for PinRollback {
    #[inline(always)]
    fn drop(&mut self) {
        for _ in 0..self.count {
            // SAFETY: the stage map is owned by this thread; balances the pins
            // taken by this acquire.
            unsafe { (*self.locks.as_ptr()).pin_release(self.world) };
        }
    }
}

#[inline(always)]
fn safety_key(si: &SafetyInfo) -> LockKey {
    match si {
        SafetyInfo::Read(li) | SafetyInfo::Write(li) => li.key,
    }
}

/// Resolved, locked component data for a guard tuple: every present component's
/// borrow is registered in `locks` and its pin taken, and `world`/pin ownership
/// is handed to the returned guards by the caller.
struct Resolved<G: GetTuple> {
    world: NonNull<sys::ecs_world_t>,
    locks: NonNull<StageLocks>,
    data: G::Pointers,
}

/// Resolve pointers and take one borrow plus one stage pin per present
/// component, with rollback. Opens no defer level (spec §7.1): liveness and the
/// record lookup are one FFI call, borrows and pins are Rust-side. On success
/// returns [`Resolved`]; the caller builds the guards (each assumes one borrow
/// and one pin, and releases both on drop).
///
/// # Safety
/// `world` and `id` must belong to the same live world.
unsafe fn resolve_and_lock<'w, G: GetTuple>(
    world: WorldRef<'w>,
    id: Entity,
) -> Result<Resolved<G>, AccessError> {
    let wptr = world.world_ptr_mut();

    // Liveness check + record lookup, no defer level.
    let record = unsafe { sys::ecs_rust_get_record(wptr, *id) };
    if record.is_null() {
        return Err(AccessError::NotAlive);
    }

    // May panic on the static duplicate-mutable-component check; no pin has been
    // taken yet, so an unwind here needs no rollback.
    let data = unsafe { G::create_ptrs::<false>(world, id, record) };

    // A missing required element leaves has_all_components false; absent
    // optionals keep it true (they are allowed to be absent).
    if !data.has_all_components() {
        return Err(AccessError::MissingComponent);
    }
    let ptrs = data.component_ptrs();
    let k = ptrs.len();

    let locks = stage_locks_dyn(&world);
    // Present elements each take one borrow and one pin below; roll the pins
    // back on a conflict early-return or an unwind.
    let mut pins = PinRollback {
        locks,
        world: wptr,
        count: 0,
    };

    let safety = data.safety_info();
    for i in 0..k {
        // Absent optional: no borrow and no pin, so nothing can conflict here.
        if ptrs[i].is_null() {
            continue;
        }
        let conflict = match &safety[i] {
            SafetyInfo::Read(li) => {
                // SAFETY: stage map is owned by this thread.
                if unsafe { (*locks.as_ptr()).read_begin(li.key) } {
                    Some((li.id, false))
                } else {
                    None
                }
            }
            SafetyInfo::Write(li) => {
                if unsafe { (*locks.as_ptr()).write_begin(li.key) } {
                    Some((li.id, true))
                } else {
                    None
                }
            }
        };
        if let Some((component, write)) = conflict {
            // Roll back only the borrows actually taken this call: present
            // elements before i. Absent optionals took none. The pins taken for
            // those same elements are released by `pins` on drop.
            for j in 0..i {
                if ptrs[j].is_null() {
                    continue;
                }
                match &safety[j] {
                    SafetyInfo::Read(li) => unsafe { (*locks.as_ptr()).read_end(li.key) },
                    SafetyInfo::Write(li) => unsafe { (*locks.as_ptr()).write_end(li.key) },
                }
            }
            return Err(AccessError::Conflict { component, write });
        }
        // Borrow taken: take the matching pin (spec §7.1). Balanced by the
        // guard's Drop.
        // SAFETY: stage map is owned by this thread.
        unsafe { (*locks.as_ptr()).pin_inc() };
        pins.count += 1;
    }

    pins.disarm();
    Ok(Resolved {
        world: unsafe { NonNull::new_unchecked(wptr) },
        locks,
        data,
    })
}

/// Opaque carrier for the crate-private data a guard needs. Public so it can
/// appear in the [`GuardElement`] signature without leaking `pub(crate)` types;
/// its fields are private and it is only constructed inside this crate.
#[doc(hidden)]
pub struct GuardParts {
    ptr: *mut c_void,
    world: NonNull<sys::ecs_world_t>,
    locks: NonNull<StageLocks>,
    key: LockKey,
}

/// One element of a guard tuple: maps `&T` to [`Ref`] and `&mut T` to [`Mut`].
#[doc(hidden)]
#[diagnostic::on_unimplemented(
    message = "`{Self}` is not a valid guard-tuple element",
    label = "not a guard element",
    note = "a guard element must be `&T`, `&mut T`, `Option<&T>`, or `Option<&mut T>` for a component `T`"
)]
pub trait GuardElement<'w>: GetTupleTypeOperation + crate::experimental::sealed::Sealed {
    type Guard;

    /// # Safety
    /// The [`GuardParts`] must describe a non-null component pointer valid for
    /// `'w`, with its borrow already registered and its stage pin already taken.
    unsafe fn wrap(parts: GuardParts) -> Self::Guard;
}

impl<'w, T> GuardElement<'w> for &T
where
    T: ComponentOrPairId + DataComponent,
{
    type Guard = Ref<'w, <T as ComponentOrPairId>::CastType>;

    #[inline(always)]
    unsafe fn wrap(p: GuardParts) -> Self::Guard {
        unsafe { Ref::from_parts(NonNull::new_unchecked(p.ptr.cast()), p.world, p.locks, p.key) }
    }
}

impl<'w, T> GuardElement<'w> for &mut T
where
    T: ComponentOrPairId + DataComponent,
{
    type Guard = Mut<'w, <T as ComponentOrPairId>::CastType>;

    #[inline(always)]
    unsafe fn wrap(p: GuardParts) -> Self::Guard {
        unsafe { Mut::from_parts(NonNull::new_unchecked(p.ptr.cast()), p.world, p.locks, p.key) }
    }
}

impl<'w, T> GuardElement<'w> for Option<&T>
where
    T: ComponentOrPairId + DataComponent,
{
    type Guard = Option<Ref<'w, <T as ComponentOrPairId>::CastType>>;

    #[inline(always)]
    unsafe fn wrap(p: GuardParts) -> Self::Guard {
        if p.ptr.is_null() {
            None
        } else {
            Some(unsafe {
                Ref::from_parts(NonNull::new_unchecked(p.ptr.cast()), p.world, p.locks, p.key)
            })
        }
    }
}

impl<'w, T> GuardElement<'w> for Option<&mut T>
where
    T: ComponentOrPairId + DataComponent,
{
    type Guard = Option<Mut<'w, <T as ComponentOrPairId>::CastType>>;

    #[inline(always)]
    unsafe fn wrap(p: GuardParts) -> Self::Guard {
        if p.ptr.is_null() {
            None
        } else {
            Some(unsafe {
                Mut::from_parts(NonNull::new_unchecked(p.ptr.cast()), p.world, p.locks, p.key)
            })
        }
    }
}

/// A request shape for [`EntityGuardExt::get_ref`]: `&T`, `&mut T`, or a tuple
/// of those. Produces the matching guard or tuple of guards.
#[diagnostic::on_unimplemented(
    message = "`{Self}` is not a valid guard request",
    label = "not a `get_ref` request",
    note = "request one component as `&T` / `&mut T` (optional: `Option<&T>` / `Option<&mut T>`), or a tuple of up to five of those"
)]
pub trait GuardTuple<'w>: GetTuple {
    type Guards;

    /// # Safety
    /// `world` and `id` must belong to the same live world.
    unsafe fn acquire(world: WorldRef<'w>, id: Entity) -> Result<Self::Guards, AccessError>;
}

impl<'w, T> GuardTuple<'w> for &T
where
    T: ComponentOrPairId + DataComponent,
{
    type Guards = Ref<'w, <T as ComponentOrPairId>::CastType>;

    #[inline(always)]
    unsafe fn acquire(world: WorldRef<'w>, id: Entity) -> Result<Self::Guards, AccessError> {
        let r = unsafe { resolve_and_lock::<Self>(world, id)? };
        let parts = GuardParts {
            ptr: r.data.component_ptrs()[0],
            world: r.world,
            locks: r.locks,
            key: safety_key(&r.data.safety_info()[0]),
        };
        Ok(unsafe { <&T as GuardElement<'w>>::wrap(parts) })
    }
}

impl<'w, T> GuardTuple<'w> for &mut T
where
    T: ComponentOrPairId + DataComponent,
{
    type Guards = Mut<'w, <T as ComponentOrPairId>::CastType>;

    #[inline(always)]
    unsafe fn acquire(world: WorldRef<'w>, id: Entity) -> Result<Self::Guards, AccessError> {
        let r = unsafe { resolve_and_lock::<Self>(world, id)? };
        let parts = GuardParts {
            ptr: r.data.component_ptrs()[0],
            world: r.world,
            locks: r.locks,
            key: safety_key(&r.data.safety_info()[0]),
        };
        Ok(unsafe { <&mut T as GuardElement<'w>>::wrap(parts) })
    }
}

macro_rules! impl_guard_tuple {
    ($( $t:ident @ $idx:tt ),+ $(,)?) => {
        impl<'w, $($t),+> GuardTuple<'w> for ($($t,)+)
        where
            $($t: GuardElement<'w>,)+
            ($($t,)+): GetTuple,
        {
            type Guards = ($($t::Guard,)+);

            #[inline(always)]
            unsafe fn acquire(world: WorldRef<'w>, id: Entity) -> Result<Self::Guards, AccessError> {
                let r = unsafe { resolve_and_lock::<Self>(world, id)? };
                let ptrs = r.data.component_ptrs();
                let si = r.data.safety_info();
                Ok(( $(
                    unsafe { $t::wrap(GuardParts {
                        ptr: ptrs[$idx],
                        world: r.world,
                        locks: r.locks,
                        key: safety_key(&si[$idx]),
                    }) },
                )+ ))
            }
        }
    };
}

impl_guard_tuple!(A @ 0, B @ 1);
impl_guard_tuple!(A @ 0, B @ 1, C @ 2);
impl_guard_tuple!(A @ 0, B @ 1, C @ 2, D @ 3);
impl_guard_tuple!(A @ 0, B @ 1, C @ 2, D @ 3, E @ 4);

/// Shared-register component access on [`EntityView`]. Provisional names; the
/// intended final surface renames `get_ref` to `get`.
pub trait EntityGuardExt<'a> {
    /// Borrow one or more components, returning guards. Panics on a borrow
    /// conflict (like `RefCell::borrow`); returns `None` if the entity is not
    /// alive or a requested component is missing.
    ///
    /// The request `G` must be a [`GuardTuple`] — a `&T` / `&mut T` (optionally
    /// `Option<&T>` / `Option<&mut T>`), or a tuple of up to five of those.
    /// Requesting anything else (here a bare `i32`) fails to compile with the
    /// trait's `#[diagnostic::on_unimplemented]` message ("`i32` is not a valid
    /// guard request"); the message text is not asserted by the doctest, only
    /// that the misuse is rejected:
    ///
    /// ```compile_fail
    /// use flecs_ecs::prelude::*;
    /// use flecs_ecs::experimental::prelude::*;
    ///
    /// let world = World::new();
    /// let e = world.entity();
    /// let _ = e.get_ref::<i32>();
    /// ```
    fn get_ref<G: GuardTuple<'a>>(self) -> Option<G::Guards>;

    /// Fallible [`get_ref`](EntityGuardExt::get_ref): returns the conflict (or
    /// missing/not-alive) as an [`AccessError`] instead of panicking.
    fn try_get_ref<G: GuardTuple<'a>>(self) -> Result<G::Guards, AccessError>;

    /// Owned copy-out with no guard held after return. Thin alias for
    /// [`EntityView::try_cloned`], included for API symmetry.
    fn cloned_owned<T: ClonedTuple>(self) -> Option<T::TupleType<'a>>;
}

impl<'a> EntityGuardExt<'a> for EntityView<'a> {
    #[inline]
    fn try_get_ref<G: GuardTuple<'a>>(self) -> Result<G::Guards, AccessError> {
        // SAFETY: self.world and self.id name the same live world.
        unsafe { G::acquire(self.world, self.id) }
    }

    #[inline]
    #[track_caller]
    fn get_ref<G: GuardTuple<'a>>(self) -> Option<G::Guards> {
        // SAFETY: as try_get_ref.
        match unsafe { G::acquire(self.world, self.id) } {
            Ok(guards) => Some(guards),
            Err(AccessError::Conflict { component, write }) => {
                alias_violation_panic(&self.world, component, write)
            }
            Err(_) => None,
        }
    }

    #[inline]
    fn cloned_owned<T: ClonedTuple>(self) -> Option<T::TupleType<'a>> {
        self.try_cloned::<T>()
    }
}
