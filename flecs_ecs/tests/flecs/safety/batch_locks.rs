//! Conflict detection across the multi-term batch lock path.
//!
//! A query with more than one term registers its term borrows for a table
//! batch in one shot (an empty stage map, no per-term lookup) and releases them
//! by range. These tests exercise that the batched borrows are still visible to
//! an entity `get`/`get_mut` guard taken inside the callback (same dense column,
//! or same sparse component record), that a disjoint access does not
//! false-positive, that a borrow held across the whole iteration (a non-empty
//! map, so the per-term path runs) is still detected, and that sparse and dense
//! batch keys never alias.

use super::*;
use flecs_ecs::experimental::QuerySharedExt;
use flecs_ecs::experimental::prelude::EntityGuardExt;

#[derive(Component)]
struct A(u8);

#[derive(Component)]
struct B(u8);

#[derive(Component)]
struct C(u8);

fn seed(world: &World) {
    world.entity().set(A(0)).set(B(0)).set(C(0));
}

/// A batched write term must conflict with a read guard taken on the same
/// component inside the callback.
#[test]
#[should_panic]
fn batch_write_term_view_read() {
    let world = World::new();
    seed(&world);
    query!(world, &mut A, &B).build().each_entity_shared(&world, |entity, _| {
        {
            let _guard = entity.get_ref::<&A>().unwrap();
        };
    });
}

/// A batched write term must conflict with a write guard on the same component.
#[test]
#[should_panic]
fn batch_write_term_view_write() {
    let world = World::new();
    seed(&world);
    query!(world, &mut A, &B).build().each_entity_shared(&world, |entity, _| {
        {
            let _guard = entity.get_ref::<&mut A>().unwrap();
        };
    });
}

/// A batched read term must conflict with a write guard on the same component.
#[test]
#[should_panic]
fn batch_read_term_view_write() {
    let world = World::new();
    seed(&world);
    query!(world, &A, &B).build().each_entity_shared(&world, |entity, _| {
        {
            let _guard = entity.get_ref::<&mut A>().unwrap();
        };
    });
}

/// A batched read term may coexist with a read guard on the same component.
#[test]
fn batch_read_term_view_read_ok() {
    let world = World::new();
    seed(&world);
    query!(world, &A, &B).build().each_entity_shared(&world, |entity, _| {
        {
            let _guard = entity.get_ref::<&A>().unwrap();
        };
    });
}

/// A guard on a component the query does not name must not false-positive
/// against the batched terms.
#[test]
fn batch_disjoint_view_ok() {
    let world = World::new();
    seed(&world);
    query!(world, &mut A, &B).build().each_entity_shared(&world, |entity, _| {
        {
            let _guard = entity.get_ref::<&mut C>().unwrap();
        };
    });
}

/// A write guard held across the whole iteration leaves the stage map non-empty
/// on entry, so the per-term acquire runs; a query term reading the held
/// component must still be reported.
#[test]
#[should_panic]
fn write_held_across_batch_read_term() {
    let world = World::new();
    let entity = world.entity().set(A(0)).set(B(0));
    {
        let _guard = entity.get_ref::<&mut A>().unwrap();
        query!(world, &A, &B).build().each_shared(&world, |_| {});
    };
}

/// A read guard held across the iteration coexists with batched read terms on
/// the same component.
#[test]
fn read_held_across_batch_read_term_ok() {
    let world = World::new();
    let entity = world.entity().set(A(0)).set(B(0));
    {
        let _guard = entity.get_ref::<&A>().unwrap();
        query!(world, &A, &B).build().each_shared(&world, |_| {});
    };
}

mod sparse {
    use super::*;

    fn seed_sparse(world: &World) {
        world.component::<A>().add_trait::<flecs::Sparse>();
        world.component::<B>().add_trait::<flecs::Sparse>();
        world.component::<C>().add_trait::<flecs::Sparse>();
        world.entity().set(A(0)).set(B(0)).set(C(0));
    }

    /// A batched sparse write term must conflict with a guard on the same sparse
    /// component taken inside the callback.
    #[test]
    #[should_panic]
    fn batch_write_term_view_read() {
        let world = World::new();
        seed_sparse(&world);
        query!(world, &mut A, &B).build().each_entity_shared(&world, |entity, _| {
            {
                let _guard = entity.get_ref::<&A>().unwrap();
            };
        });
    }

    /// A guard on a different sparse component must not false-positive: the
    /// sparse key carries a tag bit a dense key can never set, so the two key
    /// spaces do not alias either.
    #[test]
    fn batch_disjoint_view_ok() {
        let world = World::new();
        seed_sparse(&world);
        query!(world, &mut A, &B).build().each_entity_shared(&world, |entity, _| {
            {
                let _guard = entity.get_ref::<&mut C>().unwrap();
            };
        });
    }
}
