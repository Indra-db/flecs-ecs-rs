//! Bundles: a set of components inserted as one archetype move (spec §4.11).
//!
//! A [`Bundle`] is implemented for tuples of components via the crate's
//! `tuples!` macro, so `(A, B, C, ..)` is a `Bundle` up to arity 31. The trait
//! is a sealed implementation detail: its kernel resolves the component id list
//! for a world and writes each element's value into a destination column
//! pointer. Downstream crates cannot implement it.

use core::any::TypeId;
use core::ffi::c_void;
use core::mem::ManuallyDrop;

extern crate alloc;
use alloc::vec::Vec;

use crate::core::{ComponentId, Entity, EntityView, World, WorldProvider, WorldRef};
use crate::sys;

mod private {
    pub trait Sealed {}
}

/// Largest supported bundle arity. flecs' `ecs_bulk_desc_t::ids` is a fixed
/// `[ecs_id_t; FLECS_ID_DESC_MAX]` (32) array that flecs reads until a null
/// terminator, so at most 31 real ids fit alongside the terminator.
const MAX_BUNDLE_ARITY: usize = 31;
const ID_BUF: usize = 32;

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
pub trait Bundle: private::Sealed + Sized + 'static {
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

impl World {
    /// Records (once per bundle type per world) the bundle's resolved component
    /// id set, keyed by `TypeId<B>` in the §8.2 per-world cache. Stores the
    /// SORTED, de-duplicated id array, never a table pointer: tables can be
    /// deleted, component ids cannot.
    ///
    /// The first call sorts the ids, rejects any duplicate component type (which
    /// would alias mutable storage), and memoizes the result so later spawns
    /// skip the sort and the duplicate check.
    ///
    /// # Panics
    ///
    /// Panics if the bundle contains a duplicate component type.
    fn record_bundle_ids<B: Bundle>(&self, ids: &[u64]) {
        let type_id = TypeId::of::<B>();
        let cache = &self.world_ctx().bundle_ids;
        if cache.borrow().contains_key(&type_id) {
            return;
        }

        let mut sorted: Vec<u64> = ids.to_vec();
        sorted.sort_unstable();
        for w in sorted.windows(2) {
            assert!(
                w[0] != w[1],
                "bundle contains a duplicate component type (id {}); \
                 each component may appear in a bundle at most once",
                w[0]
            );
        }
        cache
            .borrow_mut()
            .insert(type_id, sorted.into_boxed_slice());
    }

    /// Constructs one entity from a bundle in a single archetype move (spec
    /// §4.11).
    ///
    /// Resolves the bundle's component id list once per bundle type per world
    /// (caching the sorted id array), then moves the bundle's values into
    /// storage via one `ecs_bulk_init` call (count = 1). `OnAdd` fires for every
    /// component and `OnSet` fires for every non-tag component, exactly as for
    /// the equivalent `entity().set()...` sequence (same set and counts of
    /// observers; the per-component *interleaving* order differs because this is
    /// one move, not N).
    ///
    /// The bundle's values are **moved** into storage; they are not dropped on
    /// the Rust side. First use of a bundle type registers any unregistered
    /// components.
    ///
    /// Returns an [`EntityView`]. (This will return `EntityMut` once the
    /// exclusive-surface redesign lands; see spec §4.11.)
    ///
    /// # Panics
    ///
    /// Panics if the bundle contains a duplicate component type, if the bundle
    /// arity exceeds 31, or if the world is in its multithreaded execution
    /// phase. Must be called on an immediate (non-deferred) world.
    pub fn spawn<B: Bundle>(&self, bundle: B) -> EntityView<'_> {
        const {
            assert!(
                B::ARITY <= MAX_BUNDLE_ARITY,
                "bundle arity exceeds the maximum of 31 components"
            );
        }

        let world = self.world();
        let world_ptr = self.raw_world.as_ptr();
        crate::core::assert_not_in_multithreaded_phase(world_ptr);

        let arity = B::ARITY;
        let mut ids = [0u64; ID_BUF];
        B::resolve_ids(world, &mut ids[..arity]);
        self.record_bundle_ids::<B>(&ids[..arity]);

        // The bundle's bytes are moved (memcpy) into storage by flecs; wrap in
        // `ManuallyDrop` so Rust never drops the source and no double-drop can
        // occur.
        let mut bundle = ManuallyDrop::new(bundle);
        let mut data = [core::ptr::null_mut::<c_void>(); ID_BUF];
        // SAFETY: `bundle` is live and initialized; `data_ptrs` only reads field
        // addresses. We forget `bundle` below, so its bytes are owned solely by
        // storage after `ecs_bulk_init`.
        unsafe {
            B::data_ptrs(&mut *bundle as *mut B, &mut data[..arity]);
        }

        // `desc.table` is intentionally left null: with a preset table,
        // `ecs_bulk_init` takes a branch (flecs.c ~9371) that leaves the diff's
        // `added_flags` at zero, which gates OFF all `OnAdd` observers
        // (flecs.c:7323). Passing `desc.ids` with a null table instead drives
        // the table-graph branch, which computes `added_flags` correctly and
        // fires `OnAdd`, matching the per-component `set` path. flecs' table
        // edge graph makes this resolution cheap and never yields a table
        // pointer we would have to cache.
        let mut desc: sys::ecs_bulk_desc_t = unsafe { core::mem::zeroed() };
        desc.count = 1;
        desc.data = data.as_mut_ptr();
        desc.ids[..arity].copy_from_slice(&ids[..arity]);

        let id_ptr = unsafe { sys::ecs_bulk_init(world_ptr, &desc) };
        assert!(
            !id_ptr.is_null(),
            "ecs_bulk_init failed while spawning a bundle"
        );
        // Copy the id out immediately; the returned array aliases internal state.
        let id = unsafe { *id_ptr };

        EntityView {
            world,
            id: Entity::new(id),
        }
    }
}
