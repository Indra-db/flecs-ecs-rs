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

// --- shared-register conflict detection (deliverable 4) ---

#[test]
#[should_panic]
fn batches_conflicts_with_live_write_guard() {
    let world = World::new();
    let e = world.entity().set(Position { x: 1, y: 0 });
    let q = query!(world, &Position).build();

    // A live write guard on Position must make the batch's read borrow of the
    // same column report a conflict (shared-register contract).
    let _w = e.get::<&mut Position>().unwrap();
    let mut cursor = q.batches(&world);
    while cursor.next().is_some() {}
}

#[test]
fn batches_read_read_coexists_with_live_read_guard() {
    let world = World::new();
    let e = world.entity().set(Position { x: 5, y: 0 });
    world.entity().set(Position { x: 6, y: 0 });
    let q = query!(world, &Position).build();

    // A live read guard coexists with the batch's read borrows.
    let r = e.get::<&Position>().unwrap();
    assert_eq!(r.x, 5);
    let mut rows = 0;
    let mut cursor = q.batches(&world);
    while let Some(p) = cursor.next() {
        rows += p.len();
    }
    assert_eq!(rows, 2);
    drop(r);
}

// --- each! over chunks and over batches agree with locked each (deliverable 4) ---

#[test]
fn each_macro_chunks_and_batches_match_each() {
    fn seeded() -> World {
        let world = World::new();
        for i in 0..9 {
            world
                .entity()
                .set(Position { x: i, y: 0 })
                .set(Velocity { x: 10, y: 0 });
        }
        world
    }

    // Reference: locked `each`.
    let world_e = seeded();
    let q_e = query!(world_e, &mut Position, &Velocity).build();
    q_e.each(|(p, v)| p.x += v.x);
    let mut via_each = Vec::new();
    q_e.each(|(p, _)| via_each.push(p.x));
    via_each.sort_unstable();

    // each! over chunks (exclusive, proven-disjoint).
    let mut world_c = seeded();
    let q_c = query!(world_c, &mut Position, &Velocity).build();
    each!((pos, vel) in q_c.chunks(&mut world_c) {
        pos.x += vel.x;
    });
    let mut via_chunks = Vec::new();
    q_c.each(|(p, _)| via_chunks.push(p.x));
    via_chunks.sort_unstable();

    // each! over batches (shared, Tier-1 locked).
    let world_b = seeded();
    let q_b = query!(world_b, &mut Position, &Velocity).build();
    each!((pos, vel) in q_b.batches(&world_b) {
        pos.x += vel.x;
    });
    let mut via_batches = Vec::new();
    q_b.each(|(p, _)| via_batches.push(p.x));
    via_batches.sort_unstable();

    assert_eq!(via_chunks, via_each);
    assert_eq!(via_batches, via_each);
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
