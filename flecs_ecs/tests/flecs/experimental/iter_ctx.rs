//! Deliverable 3: lightweight iteration context `Iter` (spec §5.6).

use super::Position;
use flecs_ecs::core::*;
use flecs_ecs::experimental::prelude::*;

#[test]
fn each_iter_delta_time_visible_after_progress() {
    let mut world = World::new();
    world.entity().set(Position { x: 1, y: 2 });
    // Advance a frame with an explicit timestep; the query iterator carries it.
    world.progress_time(0.25);

    let q = world.new_query::<&Position>();
    let mut seen = f32::NAN;
    QueryIterCtxExt::each_iter(&q, &mut world, |it, _p| {
        seen = it.delta_time();
    });
    assert!((seen - 0.25).abs() < 1e-6, "delta_time = {seen}");
}

#[test]
fn each_iter_count_matches_batch_size() {
    let mut world = World::new();
    // All-Position entities share one archetype: one batch of count 5.
    for _ in 0..5 {
        world.entity().set(Position { x: 0, y: 0 });
    }
    let q = world.new_query::<&Position>();

    let mut rows = 0usize;
    QueryIterCtxExt::each_iter(&q, &mut world, |it, _p| {
        assert_eq!(it.count(), 5, "batch count seen per row");
        rows += 1;
    });
    assert_eq!(rows, 5);
}

#[test]
fn each_iter_entity_matches_each_entity() {
    let mut world = World::new();
    for i in 0..4 {
        world.entity().set(Position { x: i, y: i });
    }
    let q = world.new_query::<&Position>();

    // Reference ordering from each_entity.
    let mut expected = alloc::vec::Vec::new();
    q.each_entity(|e, _p| expected.push(e.id()));

    // One archetype -> one batch, so the running row index is the batch row
    // index; Iter::entity(row) must match each_entity's order.
    let mut via_iter = alloc::vec::Vec::new();
    let mut row = 0usize;
    QueryIterCtxExt::each_iter(&q, &mut world, |it, _p| {
        via_iter.push(it.entity(row).id());
        row += 1;
    });
    assert_eq!(via_iter, expected);
}

#[test]
fn each_iter_shared_reads_while_shared_borrow_live() {
    let world = World::new();
    world.entity().set(Position { x: 7, y: 9 });
    let q = world.new_query::<&Position>();
    let mut sum = 0i32;
    QueryIterCtxExt::each_iter_shared(&q, &world, |it, p| {
        assert!(it.count() >= 1);
        sum += p.x + p.y;
    });
    assert_eq!(sum, 16);
}

// --- observer event metadata (spec §5.6) ---

#[test]
fn each_iter_event_metadata_zero_outside_observer() {
    let mut world = World::new();
    world.entity().set(Position { x: 1, y: 2 });
    let q = world.new_query::<&Position>();

    let mut rows = 0;
    QueryIterCtxExt::each_iter(&q, &mut world, |it, _p| {
        // Outside an observer invocation the iterator carries no event: the
        // documented returns are the zero entity and the zero id.
        assert_eq!(*it.event().id(), 0);
        assert_eq!(*it.event_id(), 0);
        rows += 1;
    });
    assert_eq!(rows, 1);
}

#[test]
fn each_iter_pair_returns_matched_pair_id() {
    let mut world = World::new();
    let likes = world.entity();
    let eva = world.entity();
    world
        .entity()
        .set(Position { x: 1, y: 2 })
        .add((likes, eva));

    // Wildcard pair term: `pair(1)` must report the concrete id matched for
    // the batch, not the wildcard the query was built with.
    let q = world
        .query::<&Position>()
        .with((likes, id::<flecs::Wildcard>()))
        .build();

    let expected = ecs_pair(*likes.id(), *eva.id());
    let mut matched = None;
    let mut plain = None;
    QueryIterCtxExt::each_iter(&q, &mut world, |it, _p| {
        matched = it.pair(1);
        plain = it.pair(0);
    });
    assert_eq!(matched.map(|id| *id), Some(expected));
    // Field 0 is a plain component, not a pair.
    assert!(plain.is_none());
}

extern crate alloc;
