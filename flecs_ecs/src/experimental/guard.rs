//! RAII component access guards for the shared register.
//!
//! [`Ref`] and [`Mut`] are the shared-register return types: taken from a
//! `&World` / [`EntityView`](crate::core::EntityView) (a Copy view that only
//! proves shared access), they register a borrow in the calling stage's lock
//! map and release it in [`Drop`], exactly like [`core::cell::Ref`] /
//! [`core::cell::RefMut`] over a `RefCell`. A conflicting borrow panics; the
//! `try_*` entry points return the conflict as an [`AccessError`] instead.
//!
//! A guard holds **no** flecs defer level of its own. Instead each live guard
//! increments a per-stage **pin counter** on acquire and decrements it on drop
//! (spec §7.1); the read path issues zero defer FFI. The flecs defer level is
//! opened lazily, once per write episode, by the shared-register write wrappers:
//! the first `set` / `add` / `remove` (etc.) issued while the pin is nonzero
//! opens one `ecs_defer_begin`, and the last guard to drop closes it with one
//! `ecs_defer_end`. That is what keeps the borrowed component pointer valid:
//! structural mutations issued through the shared world while a guard is live
//! are queued and applied only once the last guard drops, so a `&World`-driven
//! `add`/`remove` cannot move the entity to another table and dangle the guard.
//!
//! # Leaking a guard (`mem::forget`)
//!
//! [`mem::forget`](core::mem::forget)ting a guard skips its [`Drop`], so its
//! borrow is never released and its pin is never decremented. This leaks, it
//! does not corrupt (spec §7.2): the stuck borrow makes every later conflicting
//! access to that storage panic (or `try_*`-error) forever, and the stuck pin
//! keeps shared-register writes on the deferred path forever. Every leaked
//! resource is a monotonic denial (a stuck lock denies, a stuck pin keeps
//! writes deferred), never a grant, so no use-after-free is reachable.
//!
//! A leaked *read* guard with no write issued leaks only the Rust pin, no C
//! defer level, so `progress()` and world destruction still run normally. A
//! guard leaked while a write episode's level is open also leaks that one open
//! level, which delays its queued writes and their observers: **world
//! destruction drains the leaked episode** (the writes flush and their observers
//! fire at teardown) so `ecs_fini` does not abort. Because a live pin cannot be
//! distinguished from a leaked one, the episode is *not* drained at a frame
//! boundary — calling [`World::progress`](crate::core::World::progress) while a
//! write episode is open aborts, exactly as holding any flecs defer scope across
//! a frame does. A leaked write episode therefore reaches the delay-until-close
//! drain at world teardown, not at the next `progress()`.

use core::marker::PhantomData;
use core::ops::{Deref, DerefMut};
use core::ptr::NonNull;

use crate::sys;

#[cfg(feature = "flecs_safety_locks")]
use crate::core::{LockKey, StageLocks};

/// Why a guarded component access could not be granted.
///
/// Returned by the `try_*` shared-register entry points. The panicking entry
/// points turn [`AccessError::Conflict`] into a panic and treat the other
/// variants as `None`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AccessError {
    /// The entity is not alive in the world.
    NotAlive,
    /// A requested (non-optional) component is not present on the entity.
    MissingComponent,
    /// The borrow conflicts with a live borrow of the same storage: a write
    /// against any live borrow, or a read against a live write. `write` is the
    /// kind of borrow that was refused; `component` is its id.
    Conflict { component: u64, write: bool },
}

impl core::fmt::Display for AccessError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            AccessError::NotAlive => write!(f, "entity is not alive"),
            AccessError::MissingComponent => write!(f, "entity is missing a requested component"),
            AccessError::Conflict { component, write } => write!(
                f,
                "borrow conflict on component {component}: cannot take a {} borrow while an \
                 incompatible borrow is live",
                if *write { "mutable" } else { "shared" }
            ),
        }
    }
}

impl core::error::Error for AccessError {}

/// Shared, immutable access to a single component. On drop it releases its read
/// borrow and decrements the stage pin (spec §7.1); it holds no defer level of
/// its own. The `mem::forget` leak contract is described in this file's module comment.
///
/// Provisional name; the intended final surface folds this into `get`. Deref
/// to `&T`.
pub struct Ref<'w, T> {
    ptr: NonNull<T>,
    world: NonNull<sys::ecs_world_t>,
    #[cfg(feature = "flecs_safety_locks")]
    locks: NonNull<StageLocks>,
    #[cfg(feature = "flecs_safety_locks")]
    key: LockKey,
    /// Debug-only backstop identity (spec §7, structural safety net): the entity
    /// and component id this guard resolved, used by [`Drop`] to re-resolve the
    /// storage and prove it did not move under the live pin. Compiled out of
    /// release builds entirely.
    #[cfg(all(debug_assertions, feature = "flecs_safety_locks"))]
    entity: u64,
    #[cfg(all(debug_assertions, feature = "flecs_safety_locks"))]
    component_id: u64,
    _marker: PhantomData<&'w T>,
}

/// Shared, mutable access to a single component. On drop it releases its write
/// borrow and decrements the stage pin (spec §7.1); it holds no defer level of
/// its own. The `mem::forget` leak contract is described in this file's module comment.
///
/// Provisional name; the intended final surface folds this into `get`. Deref /
/// `DerefMut` to `&mut T`.
pub struct Mut<'w, T> {
    ptr: NonNull<T>,
    world: NonNull<sys::ecs_world_t>,
    #[cfg(feature = "flecs_safety_locks")]
    locks: NonNull<StageLocks>,
    #[cfg(feature = "flecs_safety_locks")]
    key: LockKey,
    /// Debug-only backstop identity; see [`Ref`]. Compiled out of release builds.
    #[cfg(all(debug_assertions, feature = "flecs_safety_locks"))]
    entity: u64,
    #[cfg(all(debug_assertions, feature = "flecs_safety_locks"))]
    component_id: u64,
    _marker: PhantomData<&'w mut T>,
}

impl<'w, T> Ref<'w, T> {
    /// # Safety
    /// `ptr` must point to a live `T` in component storage kept valid for `'w`
    /// by the stage pin this guard holds; a read borrow for `key` must already
    /// be registered in `locks` and the pin already incremented (this guard
    /// assumes ownership of one borrow and one pin on `world`'s stage).
    #[inline(always)]
    pub(crate) unsafe fn from_parts(
        ptr: NonNull<T>,
        world: NonNull<sys::ecs_world_t>,
        #[cfg(feature = "flecs_safety_locks")] locks: NonNull<StageLocks>,
        #[cfg(feature = "flecs_safety_locks")] key: LockKey,
        #[cfg(all(debug_assertions, feature = "flecs_safety_locks"))] entity: u64,
        #[cfg(all(debug_assertions, feature = "flecs_safety_locks"))] component_id: u64,
    ) -> Self {
        Ref {
            ptr,
            world,
            #[cfg(feature = "flecs_safety_locks")]
            locks,
            #[cfg(feature = "flecs_safety_locks")]
            key,
            #[cfg(all(debug_assertions, feature = "flecs_safety_locks"))]
            entity,
            #[cfg(all(debug_assertions, feature = "flecs_safety_locks"))]
            component_id,
            _marker: PhantomData,
        }
    }
}

impl<'w, T> Mut<'w, T> {
    /// # Safety
    /// As [`Ref::from_parts`], but a write borrow for `key` must be registered
    /// and the pin already incremented.
    #[inline(always)]
    pub(crate) unsafe fn from_parts(
        ptr: NonNull<T>,
        world: NonNull<sys::ecs_world_t>,
        #[cfg(feature = "flecs_safety_locks")] locks: NonNull<StageLocks>,
        #[cfg(feature = "flecs_safety_locks")] key: LockKey,
        #[cfg(all(debug_assertions, feature = "flecs_safety_locks"))] entity: u64,
        #[cfg(all(debug_assertions, feature = "flecs_safety_locks"))] component_id: u64,
    ) -> Self {
        Mut {
            ptr,
            world,
            #[cfg(feature = "flecs_safety_locks")]
            locks,
            #[cfg(feature = "flecs_safety_locks")]
            key,
            #[cfg(all(debug_assertions, feature = "flecs_safety_locks"))]
            entity,
            #[cfg(all(debug_assertions, feature = "flecs_safety_locks"))]
            component_id,
            _marker: PhantomData,
        }
    }
}

impl<T> Deref for Ref<'_, T> {
    type Target = T;
    #[inline(always)]
    fn deref(&self) -> &T {
        // SAFETY: ptr is valid for 'w (see from_parts contract).
        unsafe { self.ptr.as_ref() }
    }
}

impl<T> Deref for Mut<'_, T> {
    type Target = T;
    #[inline(always)]
    fn deref(&self) -> &T {
        // SAFETY: ptr is valid for 'w; a write borrow is held, so no other
        // access aliases this one.
        unsafe { self.ptr.as_ref() }
    }
}

impl<T> DerefMut for Mut<'_, T> {
    #[inline(always)]
    fn deref_mut(&mut self) -> &mut T {
        // SAFETY: as Deref, plus the held write borrow proves uniqueness.
        unsafe { self.ptr.as_mut() }
    }
}

/// Forwards `Debug` to the borrowed `T`, so a guard formats like the component
/// it wraps.
impl<T: core::fmt::Debug> core::fmt::Debug for Ref<'_, T> {
    #[inline]
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        core::fmt::Debug::fmt(&**self, f)
    }
}

/// Forwards `Debug` to the borrowed `T`, so a guard formats like the component
/// it wraps.
impl<T: core::fmt::Debug> core::fmt::Debug for Mut<'_, T> {
    #[inline]
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        core::fmt::Debug::fmt(&**self, f)
    }
}

/// Forwards `Display` to the borrowed `T`.
impl<T: core::fmt::Display> core::fmt::Display for Ref<'_, T> {
    #[inline]
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        core::fmt::Display::fmt(&**self, f)
    }
}

/// Forwards `Display` to the borrowed `T`.
impl<T: core::fmt::Display> core::fmt::Display for Mut<'_, T> {
    #[inline]
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        core::fmt::Display::fmt(&**self, f)
    }
}

/// Compares two `Ref` guards by their borrowed values (`*self == *other`), so a
/// guard is interchangeable with the component it wraps in equality checks.
impl<T: PartialEq> PartialEq for Ref<'_, T> {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        **self == **other
    }
}

/// Compares two `Mut` guards by their borrowed values (`*self == *other`), so a
/// guard is interchangeable with the component it wraps in equality checks.
impl<T: PartialEq> PartialEq for Mut<'_, T> {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        **self == **other
    }
}

/// Debug-only structural safety net (spec §7): re-resolve the guarded storage
/// while the pin is still held and panic if it moved.
///
/// Correct paths defer every structural mutation issued while a guard is live
/// (the pin keeps them queued behind the borrow), so the guarded component
/// pointer is stable until the last guard drops. A bypass path that skips the
/// write-episode hook applies its structural change immediately, reallocating or
/// moving the pinned column; re-resolving the component here then yields a
/// different pointer (or none) and this fires, turning a silent use-after-free
/// into a loud debug panic naming the component. Ran before the guard releases
/// its pin, i.e. before the episode's queued writes are flushed, so the correct
/// path always observes the unchanged pointer. Compiled out of release builds.
#[cfg(all(debug_assertions, feature = "flecs_safety_locks"))]
#[inline]
fn revalidate_pinned_storage(
    world: NonNull<sys::ecs_world_t>,
    entity: u64,
    component_id: u64,
    cached: *const core::ffi::c_void,
) {
    // Re-resolve on the real world: entity records live there and
    // `ecs_rust_record_get_id` poly-asserts a world (not a stage) pointer.
    let real =
        unsafe { sys::ecs_get_world(world.as_ptr().cast_const().cast()) as *mut sys::ecs_world_t };
    // SAFETY: `real` is a live world pointer; `entity` / `component_id` are this
    // guard's own, resolved on acquire. Both calls are pure reads, sound even
    // while the episode's defer level is open.
    let record = unsafe { sys::ecs_rust_get_record(real, entity) };
    let fresh = if record.is_null() {
        core::ptr::null_mut()
    } else {
        unsafe { sys::ecs_rust_record_get_id(real, entity, record, component_id).ptr }
    };
    if !core::ptr::eq(fresh.cast_const(), cached) {
        storage_moved_panic(entity, component_id);
    }
}

#[cfg(all(debug_assertions, feature = "flecs_safety_locks"))]
#[cold]
#[inline(never)]
fn storage_moved_panic(entity: u64, component_id: u64) -> ! {
    panic!(
        "flecs safety net: component storage moved while a shared-register guard was live \
         (entity {entity}, component id {component_id}). A structural operation reached \
         flecs without routing through the write-episode hook and reallocated pinned \
         storage: a use-after-free hazard. See core::safety_map::ensure_write_episode."
    );
}

impl<T> Drop for Ref<'_, T> {
    #[inline(always)]
    fn drop(&mut self) {
        // SAFETY: the stage map is owned by this thread and outlives the guard;
        // world is a live world pointer. Releases this guard's read borrow and
        // pin; `pin_release` closes the episode defer level only if this is the
        // last guard and a write opened one. Runs even on unwind. Zero FFI on
        // the read-only path.
        #[cfg(feature = "flecs_safety_locks")]
        unsafe {
            #[cfg(debug_assertions)]
            revalidate_pinned_storage(
                self.world,
                self.entity,
                self.component_id,
                self.ptr.as_ptr().cast(),
            );
            let map = &mut *self.locks.as_ptr();
            map.read_end(self.key);
            map.pin_release(self.world.as_ptr());
        }
    }
}

impl<T> Drop for Mut<'_, T> {
    #[inline(always)]
    fn drop(&mut self) {
        // SAFETY: as Ref::drop, releasing a write borrow.
        #[cfg(feature = "flecs_safety_locks")]
        unsafe {
            #[cfg(debug_assertions)]
            revalidate_pinned_storage(
                self.world,
                self.entity,
                self.component_id,
                self.ptr.as_ptr().cast(),
            );
            let map = &mut *self.locks.as_ptr();
            map.write_end(self.key);
            map.pin_release(self.world.as_ptr());
        }
    }
}
