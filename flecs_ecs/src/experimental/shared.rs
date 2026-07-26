//! Shared-register query terminals: `each_shared` / `each_entity_shared` /
//! `run_shared` (spec §4.4).
//!
//! These are the final-name Tier-1 terminals for iterating while other shared
//! borrows of the world are live. Each takes `&World` to thread the shared
//! world borrow (spec §2.3) and always runs the locked iteration internals:
//! every table batch registers its term borrows in the stage lock map, so a
//! conflict with a live entity guard panics like a `RefCell`. The Tier-0
//! lock-free skip belongs exclusively to the `&mut World` register
//! ([`QueryExclusiveExt`](super::exclusive::QueryExclusiveExt)); it never
//! applies here, proven-disjoint or not, because a concurrently held guard is
//! invisible to the build-time proof.

use crate::core::{ComponentId, EntityView, QueryAPI, QueryTuple, TableIter, World, WorldProvider};

/// Shared-register query iteration (spec §4.4): always-locked Tier-1 terminals
/// that are legal while other shared borrows of the world are live.
pub trait QuerySharedExt<'a, P, T>: QueryAPI<'a, P, T>
where
    T: QueryTuple,
{
    /// Iterate the query on the shared register. Every batch registers Tier-1
    /// locks; a conflict with a live borrow panics.
    ///
    /// # Panics
    /// If the `&World` belongs to a different world than the query, or if a
    /// batch borrow conflicts with a live guard.
    fn each_shared(&self, world: &World, func: impl FnMut(T::TupleType<'_>));

    /// Row form of [`each_shared`](QuerySharedExt::each_shared) carrying the
    /// entity.
    ///
    /// # Panics
    /// As [`each_shared`](QuerySharedExt::each_shared).
    fn each_entity_shared(&self, world: &World, func: impl FnMut(EntityView, T::TupleType<'_>));

    /// Manual table-batch iteration on the shared register: the closure is
    /// called once with a [`TableIter`] the body advances (`while it.next()`).
    /// Field access through the [`TableIter`] performs its own per-field borrow
    /// checks.
    ///
    /// # Panics
    /// If the `&World` belongs to a different world than the query.
    fn run_shared(&self, world: &World, func: impl FnMut(TableIter<true, P>))
    where
        P: ComponentId;
}

#[inline(always)]
#[track_caller]
fn assert_own_world<'a, P, T, Q>(query: &Q, world: &World, terminal: &str)
where
    Q: QueryAPI<'a, P, T>,
    T: QueryTuple,
{
    // Cached world identity (spec §4.7): read the query's real world and
    // pointer-compare against the &World. No FFI. A null query pointer means
    // the iterator was already consumed: skip the identity check and let the
    // consuming operation raise its own typed panic.
    let query_ptr = query.query_ptr();
    if query_ptr.is_null() {
        return;
    }
    assert!(
        core::ptr::eq(unsafe { (*query_ptr).real_world }, world.world_ptr()),
        "{terminal} requires the query's own world: the &World passed in belongs to a different \
         world than this query's storage"
    );
}

impl<'a, P, T, Q> QuerySharedExt<'a, P, T> for Q
where
    Q: QueryAPI<'a, P, T>,
    T: QueryTuple,
{
    #[inline]
    #[track_caller]
    fn each_shared(&self, world: &World, func: impl FnMut(T::TupleType<'_>)) {
        assert_own_world(self, world, "each_shared");
        self.each(func);
    }

    #[inline]
    #[track_caller]
    fn each_entity_shared(&self, world: &World, func: impl FnMut(EntityView, T::TupleType<'_>)) {
        assert_own_world(self, world, "each_entity_shared");
        self.each_entity(func);
    }

    #[inline]
    #[track_caller]
    fn run_shared(&self, world: &World, func: impl FnMut(TableIter<true, P>))
    where
        P: ComponentId,
    {
        assert_own_world(self, world, "run_shared");
        self.run(func);
    }
}
