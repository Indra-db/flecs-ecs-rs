//! Deliverable 3: `Query::batches` locked cursor (shared register, unproven).

use super::{Position, Velocity};
use flecs_ecs::core::*;
use flecs_ecs::each;
use flecs_ecs::experimental::prelude::*;
use flecs_ecs::macros::query;

fn seed(world: &World, n: i32) {
    for i in 0..n {
        world
            .entity()
            .set(Position { x: i, y: 0 })
            .set(Velocity { x: 2, y: 3 });
    }
}

#[test]
fn batches_yields_column_slices() {
    let world = World::new();
    seed(&world, 6);
    let q = query!(world, &mut Position, &Velocity).build();

    let mut rows = 0;
    let mut cursor = q.batches(&world);
    while let Some((pos, vel)) = cursor.next() {
        assert_eq!(pos.len(), vel.len());
        for i in 0..pos.len() {
            pos[i].x += vel[i].x;
            rows += 1;
        }
    }
    assert_eq!(rows, 6);

    // Locks were balanced on drop: a fresh shared each still works.
    let mut xs = Vec::new();
    q.each(|(p, _)| xs.push(p.x));
    xs.sort_unstable();
    assert_eq!(xs, vec![2, 3, 4, 5, 6, 7]);
}

#[test]
fn batches_each_macro_body() {
    let world = World::new();
    seed(&world, 5);
    let q = query!(world, &mut Position, &Velocity).build();

    each!((pos, vel) in q.batches(&world) {
        pos.x += vel.x;
    });

    let mut xs = Vec::new();
    q.each(|(p, _)| xs.push(p.x));
    xs.sort_unstable();
    assert_eq!(xs, vec![2, 3, 4, 5, 6]);
}

#[test]
fn batches_unproven_query_iterates() {
    // A duplicate-id query is NOT proven disjoint, so `chunks` would panic; the
    // locked batch cursor handles it (read-read of the same column is legal).
    let world = World::new();
    seed(&world, 4);
    let q = query!(world, &Position, &Position).build();

    let mut rows = 0;
    let mut cursor = q.batches(&world);
    while let Some((a, b)) = cursor.next() {
        assert_eq!(a.len(), b.len());
        rows += a.len();
    }
    assert_eq!(rows, 4);
}

#[test]
fn batches_mid_yield_panic_restores_lock_map() {
    let world = World::new();
    seed(&world, 4);
    let q = query!(world, &mut Position, &Velocity).build();

    let result = std::panic::catch_unwind(core::panic::AssertUnwindSafe(|| {
        let mut cursor = q.batches(&world);
        // Panic while a chunk (and its batch locks) are live.
        if let Some((pos, _vel)) = cursor.next() {
            pos[0].x += 1;
            panic!("boom mid-batch");
        }
    }));
    assert!(result.is_err());

    // The IterGuard stage-lock scope restored the borrow map on unwind: a fresh
    // shared each does not report a spurious conflict and runs to completion.
    let mut count = 0;
    q.each(|(_, _)| count += 1);
    assert_eq!(count, 4);
}

#[test]
fn batches_early_break_releases_locks() {
    let world = World::new();
    seed(&world, 8);
    let q = query!(world, &mut Position, &Velocity).build();

    {
        let mut cursor = q.batches(&world);
        // Take one batch then break, leaving its locks held; drop must release.
        if let Some((pos, vel)) = cursor.next() {
            pos[0].x += vel[0].x;
        }
    }

    // No leaked lock: a fresh shared each still iterates every row.
    let mut count = 0;
    q.each(|(_, _)| count += 1);
    assert_eq!(count, 8);
}
