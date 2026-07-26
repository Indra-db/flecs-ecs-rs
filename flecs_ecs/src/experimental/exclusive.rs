//! Exclusive register: access proven unique by `&mut World`.
//!
//! When the caller holds `&mut World`, the borrow checker already excludes every
//! other access to the world for the duration of the returned borrow, so no
//! runtime lock is taken and plain `&mut T` can be handed back.
//!
//! ## What the borrow checker excludes (and what it does not)
//!
//! [`WorldExclusiveExt::get_mut`] returns `&'w mut T` where `'w` is the
//! `&'w mut World` borrow. While that reference is live:
//!
//! * every `&self` / `&mut self` method on the same `World` is excluded — this
//!   is what makes skipping the lock sound;
//! * a *different* `World` is not excluded (it is a different borrow), but it
//!   owns different storage, so it cannot alias `T`.
//!
//! It does **not** attempt to exclude raw-pointer aliasing or a second world
//! that was unsafely made to share storage; those are already outside the safe
//! API. The compile-fail doc test below only asserts the guarantee the borrow
//! checker actually provides.
//!
//! ```compile_fail
//! use flecs_ecs::prelude::*;
//! use flecs_ecs::experimental::prelude::*;
//!
//! #[derive(Component)]
//! struct Position { x: i32 }
//!
//! let mut world = World::new();
//! let e = world.entity().set(Position { x: 0 });
//! let p = world.get_mut::<Position>(e).unwrap();
//! // Holding `p` (which borrows `&mut world`) while touching the world again
//! // must not compile:
//! let _q = world.get_mut::<Position>(e);
//! p.x += 1;
//! ```

use crate::core::{
    ComponentId, ComponentOrPairId, ComponentPointers, DataComponent, Entity, EntityView,
    GetComponentPointers, GetTuple, IterGuard, QueryAPI, QueryTuple, TableIter, World,
    WorldProvider, WorldRef,
};
use crate::sys;

/// Exclusive-register component access on [`World`]. Provisional name; the
/// intended final surface is `World::get_mut`.
pub trait WorldExclusiveExt {
    /// Borrow a component mutably with zero lock traffic. The returned
    /// reference borrows `&mut self`, so the borrow checker forbids every other
    /// access to this world while it is live — which is exactly what makes
    /// skipping the runtime lock sound.
    ///
    /// Returns `None` if the entity is not alive or does not have the component.
    fn get_mut<T>(
        &mut self,
        entity: impl Into<Entity>,
    ) -> Option<&mut <T as ComponentOrPairId>::CastType>
    where
        T: ComponentOrPairId + DataComponent;
}

impl WorldExclusiveExt for World {
    fn get_mut<T>(
        &mut self,
        entity: impl Into<Entity>,
    ) -> Option<&mut <T as ComponentOrPairId>::CastType>
    where
        T: ComponentOrPairId + DataComponent,
    {
        let entity: Entity = entity.into();
        let world_ptr = self.raw_world.as_ptr();
        // SAFETY: world_ptr is this world's live pointer.
        let world = unsafe { WorldRef::from_ptr(world_ptr) };

        // No defer scope and no lock: the &mut World borrow already excludes any
        // aliasing access for the whole lifetime of the returned reference.
        // ecs_record_find asserts on dead entities in debug builds, so gate on
        // liveness first (mirrors ecs_rust_get_scope_begin).
        if !unsafe { sys::ecs_is_alive(world_ptr, *entity) } {
            return None;
        }
        let record = unsafe { sys::ecs_record_find(world_ptr, *entity) };
        if record.is_null() {
            return None;
        }

        // SAFETY: record was just looked up for `entity` on this world.
        let data = unsafe { <&mut T as GetTuple>::create_ptrs::<false>(world, entity, record) };
        let ptr = data.component_ptrs()[0];
        if ptr.is_null() {
            return None;
        }
        // SAFETY: ptr is a valid, uniquely-owned pointer to the component; the
        // returned reference is tied to `&mut self`, so nothing else can alias
        // it for `'w`.
        Some(unsafe { &mut *(ptr as *mut <T as ComponentOrPairId>::CastType) })
    }
}

/// Iterate a batch without taking any borrow locks. Sound only on the exclusive
/// register, where `&mut World` proves no outstanding borrow exists.
#[inline(always)]
fn each_exclusive_batch<T: QueryTuple>(
    iter: &mut sys::ecs_iter_t,
    func: &mut impl FnMut(T::TupleType<'_>),
) {
    iter.flags |= sys::EcsIterCppEach;
    let (is_any_array, mut data) = T::create_ptrs(iter);
    let count = if iter.count == 0 && iter.table.is_null() {
        1_usize
    } else {
        iter.count as usize
    };

    if !is_any_array.a_ref && !is_any_array.a_row {
        for i in 0..count {
            func(data.get_tuple(i));
        }
    } else if is_any_array.a_row {
        for i in 0..count {
            func(data.get_tuple_with_row(iter, i));
        }
    } else {
        for i in 0..count {
            func(data.get_tuple_with_ref(i));
        }
    }
}

/// Iterate a batch without taking any borrow locks, handing the row's entity to
/// the callback. Sound only on the exclusive register, like
/// [`each_exclusive_batch`].
#[inline(always)]
fn each_entity_exclusive_batch<T: QueryTuple>(
    iter: &mut sys::ecs_iter_t,
    world: &WorldRef<'_>,
    func: &mut impl FnMut(EntityView, T::TupleType<'_>),
) {
    iter.flags |= sys::EcsIterCppEach;
    let (is_any_array, mut data) = T::create_ptrs(iter);
    let count = if iter.count == 0 && iter.table.is_null() {
        1_usize
    } else {
        iter.count as usize
    };
    assert!(
        !iter.entities.is_null(),
        "each_entity_exclusive called on an iteration that does not return entities ($this \
         variable is not populated)"
    );

    if !is_any_array.a_ref && !is_any_array.a_row {
        for i in 0..count {
            // SAFETY: i < count and entities has count valid entries.
            let entity = EntityView::new_from_raw(world, unsafe { *iter.entities.add(i) });
            func(entity, data.get_tuple(i));
        }
    } else if is_any_array.a_row {
        for i in 0..count {
            // SAFETY: as above.
            let entity = EntityView::new_from_raw(world, unsafe { *iter.entities.add(i) });
            func(entity, data.get_tuple_with_row(iter, i));
        }
    } else {
        for i in 0..count {
            // SAFETY: as above.
            let entity = EntityView::new_from_raw(world, unsafe { *iter.entities.add(i) });
            func(entity, data.get_tuple_with_ref(i));
        }
    }
}

#[cfg(feature = "flecs_safety_locks")]
#[inline(always)]
fn debug_assert_no_live_borrows(world_ref: &WorldRef<'_>, terminal: &str) {
    let locks = crate::core::stage_locks_dyn(world_ref);
    // SAFETY: the stage map is owned by this thread.
    debug_assert!(
        unsafe { (*locks.as_ptr()).is_empty() },
        "{terminal} entered with a live borrow registered in the stage lock map; the &mut World \
         exclusivity invariant was violated"
    );
}

#[inline(always)]
#[track_caller]
fn assert_own_world<'a, P, T, Q>(query: &Q, world: &World, terminal: &str)
where
    Q: QueryAPI<'a, P, T>,
    T: QueryTuple,
{
    // Cached world identity (spec §4.7): the query stores its real world, so
    // the per-call check is one field read plus one pointer compare against
    // the &mut World's pointer (always a real, unstaged world). No FFI. A null
    // query pointer means the iterator was already consumed: skip the identity
    // check and let the consuming operation raise its own typed panic.
    let query_ptr = query.query_ptr();
    if query_ptr.is_null() {
        return;
    }
    assert!(
        core::ptr::eq(unsafe { (*query_ptr).real_world }, world.world_ptr()),
        "{terminal} requires the query's own world: the &mut World passed in belongs to a \
         different world and cannot prove exclusive access to this query's storage"
    );
}

/// Exclusive-register query iteration. Provisional names; the intended final
/// surface is an `each` / `each_entity` / `run` overload taking `&mut World`.
pub trait QueryExclusiveExt<'a, P, T>: QueryAPI<'a, P, T>
where
    T: QueryTuple,
{
    /// Iterate the query with no lock acquire/release. The `&mut World` proves
    /// no entity guard or other borrow is outstanding, so the per-batch borrow
    /// bookkeeping of [`each`](QueryAPI::each) is skipped entirely.
    ///
    /// In debug builds this asserts the stage lock map is empty on entry.
    fn each_exclusive(&self, world: &mut World, func: impl FnMut(T::TupleType<'_>));

    /// Row form of [`each_exclusive`](QueryExclusiveExt::each_exclusive)
    /// carrying the entity. Same tier selection: lock-free when the query is
    /// proven disjoint (cached at build, spec §4.7), else the locked Tier-1
    /// path over the same `&mut World`.
    ///
    /// # Panics
    /// If the `&mut World` belongs to a different world than the query.
    fn each_entity_exclusive(&self, world: &mut World, func: impl FnMut(EntityView, T::TupleType<'_>));

    /// Manual table-batch iteration on the exclusive register: the closure is
    /// called once with a [`TableIter`] the body advances (`while it.next()`).
    ///
    /// The `&mut World` proves no entity guard is outstanding on entry (debug
    /// builds assert the stage lock map is empty). Field access through the
    /// [`TableIter`] still performs its own per-field borrow checks; the Tier-0
    /// batch-lock skip does not apply to manual iteration.
    ///
    /// # Panics
    /// If the `&mut World` belongs to a different world than the query.
    fn run_exclusive(&self, world: &mut World, func: impl FnMut(TableIter<true, P>))
    where
        P: ComponentId;
}

impl<'a, P, T, Q> QueryExclusiveExt<'a, P, T> for Q
where
    Q: QueryAPI<'a, P, T>,
    T: QueryTuple,
{
    #[track_caller]
    fn each_exclusive(&self, world: &mut World, func: impl FnMut(T::TupleType<'_>)) {
        let world_ref = self.world();
        assert_own_world(self, world, "each_exclusive");

        // The &mut World proves no outstanding entity guard, but the query must
        // still be intra-query disjoint for a lock-free run to be sound (nothing
        // would otherwise catch two mutable terms hitting the same storage).
        // When not proven, fall back to the locked path, which is always correct.
        // The verdict is cached on the query and computed at most once.
        if !super::disjoint::is_proven_disjoint_cached(
            self.disjoint_cache(),
            world_ref,
            self.query_ptr(),
        ) {
            self.each(func);
            return;
        }

        #[cfg(feature = "flecs_safety_locks")]
        debug_assert_no_live_borrows(&world_ref, "each_exclusive");

        let mut func = func;
        let mut iter = IterGuard::new(self.retrieve_iter());
        while iter.next(|i| self.iter_next(i)) {
            each_exclusive_batch::<T>(&mut iter, &mut func);
        }
        // Resume a panic stashed by a nested synchronous observer (spec §5.5).
        world_ref.rethrow_stashed_panic();
    }

    #[track_caller]
    fn each_entity_exclusive(
        &self,
        world: &mut World,
        func: impl FnMut(EntityView, T::TupleType<'_>),
    ) {
        let world_ref = self.world();
        assert_own_world(self, world, "each_entity_exclusive");

        // Same tier selection as each_exclusive: an unproven query falls back to
        // the locked Tier-1 path, which is always correct.
        if !super::disjoint::is_proven_disjoint_cached(
            self.disjoint_cache(),
            world_ref,
            self.query_ptr(),
        ) {
            self.each_entity(func);
            return;
        }

        #[cfg(feature = "flecs_safety_locks")]
        debug_assert_no_live_borrows(&world_ref, "each_entity_exclusive");

        let mut func = func;
        let mut iter = IterGuard::new(self.retrieve_iter());
        while iter.next(|i| self.iter_next(i)) {
            each_entity_exclusive_batch::<T>(&mut iter, &world_ref, &mut func);
        }
        // Resume a panic stashed by a nested synchronous observer (spec §5.5).
        world_ref.rethrow_stashed_panic();
    }

    #[track_caller]
    fn run_exclusive(&self, world: &mut World, func: impl FnMut(TableIter<true, P>))
    where
        P: ComponentId,
    {
        assert_own_world(self, world, "run_exclusive");

        #[cfg(feature = "flecs_safety_locks")]
        debug_assert_no_live_borrows(&self.world(), "run_exclusive");

        self.run(func);
    }
}
