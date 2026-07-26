//! Shared-register terminals (`each_shared` / `each_entity_shared` /
//! `run_shared`, spec §4.4): always-locked Tier-1 iteration that is legal while
//! other shared borrows of the world are live, and conflicts with live guards.

use super::{Position, Velocity};
use flecs_ecs::core::*;
use flecs_ecs::experimental::prelude::*;
use flecs_ecs::macros::query;

fn seed(world: &World, n: i32) {
    for i in 0..n {
        world
            .entity()
            .set(Position { x: i, y: 0 })
            .set(Velocity { x: 1, y: 2 });
    }
}

#[test]
fn each_shared_matches_each_exclusive() {
    let mut world = World::new();
    seed(&world, 5);
    let q = query!(world, &mut Position, &Velocity).build();

    q.each_exclusive(&mut world, |(p, v)| {
        p.x += v.x;
        p.y += v.y;
    });

    let mut rows = 0;
    q.each_shared(&world, |(p, v)| {
        p.x += v.x;
        p.y += v.y;
        rows += 1;
    });
    assert_eq!(rows, 5);

    // Both terminals visited every row exactly once: x = i + 2, y = 4.
    let mut xs = Vec::new();
    q.each_shared(&world, |(p, _)| {
        xs.push(p.x);
        assert_eq!(p.y, 4);
    });
    xs.sort_unstable();
    assert_eq!(xs, vec![2, 3, 4, 5, 6]);
}

#[test]
fn each_entity_shared_matches_each_entity_exclusive() {
    let mut world = World::new();
    seed(&world, 4);
    let q = query!(world, &mut Position, &Velocity).build();

    let mut via_exclusive = Vec::new();
    q.each_entity_exclusive(&mut world, |e, _| via_exclusive.push(e.id()));

    let mut via_shared = Vec::new();
    q.each_entity_shared(&world, |e, _| via_shared.push(e.id()));

    assert_eq!(via_exclusive.len(), 4);
    assert_eq!(via_shared, via_exclusive);
}

#[test]
fn each_shared_legal_while_other_shared_borrow_live() {
    let world = World::new();
    seed(&world, 3);
    let unrelated = world.entity().set(super::Health(7));
    let q = query!(world, &mut Position, &Velocity).build();

    // A live guard on a component the query does not name must not conflict.
    let h = unrelated.get_ref::<&super::Health>().unwrap();
    let mut rows = 0;
    q.each_shared(&world, |(p, v)| {
        p.x += v.x;
        rows += 1;
    });
    assert_eq!(rows, 3);
    assert_eq!(h.0, 7);
}

#[test]
fn run_shared_basic() {
    let world = World::new();
    seed(&world, 3);
    let q = query!(world, &Position, &Velocity).build();

    let mut tables = 0;
    let mut entities = 0;
    q.run_shared(&world, |mut it| {
        while it.next() {
            tables += 1;
            let pos = it.field::<Position>(0);
            for i in it.iter() {
                entities += 1;
                let _ = pos[i].x;
            }
        }
    });
    assert_eq!(tables, 1);
    assert_eq!(entities, 3);
}

#[test]
fn run_exclusive_basic() {
    let mut world = World::new();
    seed(&world, 3);
    let q = query!(world, &mut Position, &Velocity).build();

    let mut entities = 0;
    q.run_exclusive(&mut world, |mut it| {
        while it.next() {
            let mut pos = it.field_mut::<Position>(0);
            for i in it.iter() {
                pos[i].x += 10;
                entities += 1;
            }
        }
    });
    assert_eq!(entities, 3);

    let mut xs = Vec::new();
    q.each_shared(&world, |(p, _)| xs.push(p.x));
    xs.sort_unstable();
    assert_eq!(xs, vec![10, 11, 12]);
}

// --- lock registration: the Tier-0 skip must never apply to the shared register ---

/// A proven-disjoint query skips locks on `each_exclusive`, but `each_shared`
/// must still register its batch borrows: a guard taken inside the callback on
/// a component the query writes has to conflict.
#[test]
#[should_panic]
fn each_shared_write_term_conflicts_with_callback_guard() {
    let world = World::new();
    seed(&world, 1);
    let e = world.entity().set(Position { x: 0, y: 0 }).set(Velocity {
        x: 0,
        y: 0,
    });
    let q = query!(world, &mut Position, &Velocity).build();
    q.each_shared(&world, |_| {
        let _p = e.get_ref::<&Position>();
    });
}

/// Same proof through the entity-carrying terminal, mirroring the
/// `batch_locks` suite: a batched write term vs a read guard on the same
/// component taken in the callback.
#[test]
#[should_panic]
fn each_entity_shared_write_term_conflicts_with_callback_guard() {
    let world = World::new();
    seed(&world, 1);
    let q = query!(world, &mut Position, &Velocity).build();
    q.each_entity_shared(&world, |e, _| {
        let _p = e.get_ref::<&mut Position>();
    });
}

/// A write guard held across the whole `each_shared` iteration leaves the stage
/// map non-empty on entry; a query term reading that component must still be
/// reported.
#[test]
#[should_panic]
fn write_guard_held_across_each_shared_conflicts() {
    let world = World::new();
    seed(&world, 1);
    let e = world.entity().set(Position { x: 0, y: 0 }).set(Velocity {
        x: 0,
        y: 0,
    });
    let q = query!(world, &Position, &Velocity).build();
    let _p = e.get_ref::<&mut Position>().unwrap();
    q.each_shared(&world, |_| {});
}

// --- world identity ---

#[test]
#[should_panic(expected = "each_shared requires the query's own world")]
fn each_shared_foreign_world_panics() {
    let world_a = World::new();
    seed(&world_a, 1);
    let q = query!(world_a, &Position, &Velocity).build();
    let world_b = World::new();
    q.each_shared(&world_b, |_| {});
}

#[test]
#[should_panic(expected = "run_shared requires the query's own world")]
fn run_shared_foreign_world_panics() {
    let world_a = World::new();
    seed(&world_a, 1);
    let q = query!(world_a, &Position, &Velocity).build();
    let world_b = World::new();
    q.run_shared(&world_b, |_| {});
}
