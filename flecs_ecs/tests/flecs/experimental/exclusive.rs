//! Deliverable 2: exclusive register (`get_exclusive`, `each_exclusive`).

use super::{Position, Velocity};
use flecs_ecs::core::*;
use flecs_ecs::experimental::prelude::*;
use flecs_ecs::macros::query;

#[test]
fn get_exclusive_reads_and_writes() {
    let mut world = World::new();
    let e = world.entity().set(Position { x: 1, y: 2 }).id();

    let p = world.get_exclusive::<Position>(e).unwrap();
    assert_eq!((p.x, p.y), (1, 2));
    p.x += 40;
    // Reference dropped at end of statement below; re-borrow to confirm.
    let p2 = world.get_exclusive::<Position>(e).unwrap();
    assert_eq!(p2.x, 41);
}

#[test]
fn get_exclusive_missing_is_none() {
    let mut world = World::new();
    let e = world.entity().set(Position::default()).id();
    assert!(world.get_exclusive::<Velocity>(e).is_none());
}

#[test]
fn get_exclusive_dead_is_none() {
    let mut world = World::new();
    let e = world.entity().set(Position::default()).id();
    world.entity_from_id(e).destruct();
    assert!(world.get_exclusive::<Position>(e).is_none());
}

#[test]
fn get_exclusive_takes_no_locks() {
    // Two sequential exclusive borrows of the same component in one scope must
    // both succeed: no runtime lock is registered, only the borrow checker
    // (which is satisfied because each reference is released at its last use).
    let mut world = World::new();
    let e = world.entity().set(Position { x: 0, y: 0 }).id();
    world.get_exclusive::<Position>(e).unwrap().x = 1;
    world.get_exclusive::<Position>(e).unwrap().x += 1;
    assert_eq!(world.get_exclusive::<Position>(e).unwrap().x, 2);
}

#[test]
fn each_exclusive_iterates() {
    let mut world = World::new();
    for i in 0..5 {
        world
            .entity()
            .set(Position { x: i, y: 0 })
            .set(Velocity { x: 1, y: 2 });
    }
    let q = query!(world, &mut Position, &Velocity).build();

    let mut sum = 0;
    q.each_exclusive(&mut world, |(p, v)| {
        p.x += v.x;
        p.y += v.y;
        sum += 1;
    });
    assert_eq!(sum, 5);

    let mut total_y = 0;
    q.each_exclusive(&mut world, |(p, _v)| {
        total_y += p.y;
    });
    assert_eq!(total_y, 10);
}

#[test]
fn each_exclusive_matches_locked_each() {
    let mut world = World::new();
    for i in 0..8 {
        world.entity().set(Position { x: i, y: i });
    }
    let q = query!(world, &mut Position).build();

    q.each_exclusive(&mut world, |p| p.x *= 2);

    let mut collected = Vec::new();
    q.each(|p| collected.push(p.x));
    collected.sort_unstable();
    assert_eq!(collected, vec![0, 2, 4, 6, 8, 10, 12, 14]);
}

#[test]
#[should_panic(expected = "each_exclusive requires the query's own world")]
fn each_exclusive_foreign_world_panics() {
    let world_a = World::new();
    world_a
        .entity()
        .set(Position::default())
        .set(Velocity::default());
    let q = query!(world_a, &mut Position, &Velocity).build();
    let mut world_b = World::new();
    q.each_exclusive(&mut world_b, |(_, _)| {});
}
