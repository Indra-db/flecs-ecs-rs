//! Bundles: a set of components inserted as one archetype move (spec §4.11).
//!
//! A [`Bundle`] is implemented for tuples of components via the crate's
//! `tuples!` macro, so `(A, B, C, ..)` is a `Bundle` up to arity 31. The trait
//! is a sealed implementation detail: its kernel resolves the component id list
//! for a world and writes each element's value into a destination column
//! pointer. Downstream crates cannot implement it.

use core::ffi::c_void;

use crate::core::{ComponentId, EntityView, WorldRef};

mod private {
    pub trait Sealed {}
}

/// A set of components that can be inserted into an entity as a single
/// archetype move.
///
/// `Bundle` is implemented for tuples of `#[derive(Component)]` types up to
/// arity 31 (bounded by flecs' `FLECS_ID_DESC_MAX`, which must reserve one slot
/// for a null terminator). It powers [`World::spawn`](crate::core::World::spawn),
/// [`World::spawn_batch`](crate::core::World::spawn_batch) and
/// [`EntityView::insert`](crate::core::EntityView::insert), each of which
/// replaces N per-component archetype moves with one.
///
/// The trait is **sealed**: its kernel methods are a `#[doc(hidden)]`
/// implementation detail and cannot be implemented outside this crate.
#[diagnostic::on_unimplemented(
    message = "`{Self}` is not a `Bundle`.",
    label = "a `Bundle` is a tuple of `#[derive(Component)]` types, e.g. `(A, B, C)`, up to arity 31",
    note = "each element must implement `ComponentId` (via `#[derive(Component)]`)"
)]
pub trait Bundle: private::Sealed + Sized {
    /// Number of components in the bundle (its tuple arity).
    #[doc(hidden)]
    const ARITY: usize;

    /// Resolves (registering on first use) each component id for `world` into
    /// `out`, in tuple order. Performs the per-component thread-affinity check
    /// required to move the value into storage.
    ///
    /// `out.len()` must be `>= ARITY`.
    #[doc(hidden)]
    fn resolve_ids(world: WorldRef, out: &mut [u64]);

    /// Writes `size_of::<T>()` per component into `out`, in tuple order.
    #[doc(hidden)]
    fn sizes(out: &mut [usize]);

    /// Writes a data pointer per component into `out`, in tuple order. Zero-sized
    /// (tag) components get a null pointer, matching flecs' tag contract for
    /// `ecs_bulk_init`.
    ///
    /// # Safety
    ///
    /// `this` must point to a live, initialized `Self`. The returned pointers
    /// borrow from `*this`; the caller MUST NOT drop `*this` afterwards, because
    /// the pointed-to bytes are moved (memcpy) into flecs storage.
    #[doc(hidden)]
    unsafe fn data_ptrs(this: *mut Self, out: &mut [*mut c_void]);

    /// Consumes the bundle, issuing one `set` per component on `entity` in tuple
    /// order. Used by `insert` inside a defer scope so flecs merges the
    /// same-entity commands into a single table move.
    #[doc(hidden)]
    fn apply_set(self, entity: EntityView);
}

macro_rules! bundle_count {
    () => { 0usize };
    ($head:ident $(, $tail:ident)*) => { 1usize + bundle_count!($($tail),*) };
}

macro_rules! impl_bundle {
    ($($t:ident),*) => {
        impl<$($t: ComponentId),*> private::Sealed for ($($t,)*) {}

        impl<$($t: ComponentId),*> Bundle for ($($t,)*) {
            const ARITY: usize = bundle_count!($($t),*);

            #[inline]
            fn resolve_ids(_world: WorldRef, _out: &mut [u64]) {
                let mut _i = 0usize;
                $(
                    _world.check_thread_affinity_exclusive::<$t>();
                    _out[_i] = $t::entity_id(_world);
                    _i += 1;
                )*
            }

            #[inline]
            fn sizes(_out: &mut [usize]) {
                let mut _i = 0usize;
                $(
                    _out[_i] = core::mem::size_of::<$t>();
                    _i += 1;
                )*
            }

            #[inline]
            unsafe fn data_ptrs(_this: *mut Self, _out: &mut [*mut c_void]) {
                // SAFETY: caller guarantees `_this` points to a live, initialized
                // `Self`; we only take addresses of its fields, never move them.
                let ($($t,)*) = unsafe { &mut *_this };
                let mut _i = 0usize;
                $(
                    _out[_i] = if core::mem::size_of::<$t>() == 0 {
                        core::ptr::null_mut()
                    } else {
                        ($t as *mut $t).cast::<c_void>()
                    };
                    _i += 1;
                )*
            }

            #[inline]
            #[allow(clippy::unused_unit)]
            fn apply_set(self, _entity: EntityView) {
                let ($($t,)*) = self;
                $(
                    _entity.set($t);
                )*
            }
        }
    };
}

use flecs_ecs_derive::tuples;
tuples!(impl_bundle, 1, 31);
