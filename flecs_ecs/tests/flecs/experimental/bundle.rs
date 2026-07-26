#![allow(dead_code)]

use core::sync::atomic::{AtomicUsize, Ordering::SeqCst};

use alloc::sync::Arc;

use flecs_ecs::core::*;
use flecs_ecs::macros::*;

#[derive(Component, Debug, PartialEq)]
struct Pos {
    x: i32,
    y: i32,
}

#[derive(Component, Debug, PartialEq)]
struct Vel {
    x: i32,
    y: i32,
}

#[derive(Component, Debug, PartialEq)]
struct Health(i32);

#[derive(Component)]
struct Tag;

#[derive(Component)]
struct Marker;

#[test]
fn spawn_creates_entity_with_all_components_and_values() {
    let world = World::new();

    let e = world.spawn((Pos { x: 1, y: 2 }, Vel { x: 3, y: 4 }, Health(100)));

    assert!(e.is_alive());
    assert!(e.has(Pos::id()));
    assert!(e.has(Vel::id()));
    assert!(e.has(Health::id()));

    e.get::<(&Pos, &Vel, &Health)>(|(p, v, h)| {
        assert_eq!(*p, Pos { x: 1, y: 2 });
        assert_eq!(*v, Vel { x: 3, y: 4 });
        assert_eq!(*h, Health(100));
    });
}

#[test]
fn spawn_single_component_bundle() {
    let world = World::new();
    let e = world.spawn((Health(7),));
    e.get::<&Health>(|h| assert_eq!(*h, Health(7)));
}

#[test]
fn spawn_registers_unregistered_components() {
    #[derive(Component, PartialEq, Debug)]
    struct Fresh1(u32);
    #[derive(Component, PartialEq, Debug)]
    struct Fresh2(u32);

    let world = World::new();
    // Components have never been touched before this spawn.
    let e = world.spawn((Fresh1(11), Fresh2(22)));
    assert!(e.has(Fresh1::id()));
    assert!(e.has(Fresh2::id()));
    e.get::<(&Fresh1, &Fresh2)>(|(a, b)| {
        assert_eq!(*a, Fresh1(11));
        assert_eq!(*b, Fresh2(22));
    });
}

#[test]
fn spawn_with_zero_sized_tag() {
    let world = World::new();
    let e = world.spawn((Pos { x: 5, y: 6 }, Tag, Marker));
    assert!(e.has(Pos::id()));
    assert!(e.has(Tag::id()));
    assert!(e.has(Marker::id()));
    e.get::<&Pos>(|p| assert_eq!(*p, Pos { x: 5, y: 6 }));
}

#[test]
fn spawn_only_tags() {
    let world = World::new();
    let e = world.spawn((Tag, Marker));
    assert!(e.has(Tag::id()));
    assert!(e.has(Marker::id()));
}

#[test]
#[should_panic(expected = "duplicate component type")]
fn spawn_duplicate_component_type_panics() {
    let world = World::new();
    let _ = world.spawn((Health(1), Health(2)));
}

static SPAWN_DROP_COUNT: AtomicUsize = AtomicUsize::new(0);

#[derive(Component)]
struct SpawnDropCounter(u32);

impl Drop for SpawnDropCounter {
    fn drop(&mut self) {
        SPAWN_DROP_COUNT.fetch_add(1, SeqCst);
    }
}

#[test]
fn spawn_drop_runs_exactly_once_per_value() {
    SPAWN_DROP_COUNT.store(0, SeqCst);
    {
        let world = World::new();
        let _e = world.spawn((SpawnDropCounter(1), Health(2)));
        // Value lives in storage; not yet dropped.
        assert_eq!(SPAWN_DROP_COUNT.load(SeqCst), 0);
    }
    // World dropped -> storage torn down -> exactly one drop.
    assert_eq!(SPAWN_DROP_COUNT.load(SeqCst), 1);
}

static SPAWN_DELETE_DROP: AtomicUsize = AtomicUsize::new(0);

#[derive(Component)]
struct DeleteDropCounter(u32);

impl Drop for DeleteDropCounter {
    fn drop(&mut self) {
        SPAWN_DELETE_DROP.fetch_add(1, SeqCst);
    }
}

#[test]
fn spawn_then_delete_drops_once() {
    SPAWN_DELETE_DROP.store(0, SeqCst);
    let world = World::new();
    let e = world.spawn((DeleteDropCounter(9),));
    assert_eq!(SPAWN_DELETE_DROP.load(SeqCst), 0);
    e.destruct();
    assert_eq!(SPAWN_DELETE_DROP.load(SeqCst), 1);
}

#[test]
fn spawn_observer_parity_with_set_sequence() {
    let world = World::new();

    let on_add = Arc::new(AtomicUsize::new(0));
    let on_set = Arc::new(AtomicUsize::new(0));

    {
        let a = on_add.clone();
        world
            .observer::<flecs::OnAdd, ()>()
            .with(Pos::id())
            .each(move |_| {
                a.fetch_add(1, SeqCst);
            });
    }
    {
        let a = on_add.clone();
        world
            .observer::<flecs::OnAdd, ()>()
            .with(Vel::id())
            .each(move |_| {
                a.fetch_add(1, SeqCst);
            });
    }
    {
        let s = on_set.clone();
        world.observer::<flecs::OnSet, &Pos>().each(move |_| {
            s.fetch_add(1, SeqCst);
        });
    }
    {
        let s = on_set.clone();
        world.observer::<flecs::OnSet, &Vel>().each(move |_| {
            s.fetch_add(1, SeqCst);
        });
    }

    // Spawn path.
    world.spawn((Pos { x: 1, y: 1 }, Vel { x: 2, y: 2 }));
    let spawn_add = on_add.swap(0, SeqCst);
    let spawn_set = on_set.swap(0, SeqCst);

    // Equivalent per-component set sequence.
    world.entity().set(Pos { x: 1, y: 1 }).set(Vel { x: 2, y: 2 });
    let seq_add = on_add.swap(0, SeqCst);
    let seq_set = on_set.swap(0, SeqCst);

    assert_eq!(spawn_add, 2, "one OnAdd per component");
    assert_eq!(spawn_set, 2, "one OnSet per component");
    assert_eq!(spawn_add, seq_add, "OnAdd count parity spawn vs set-sequence");
    assert_eq!(spawn_set, seq_set, "OnSet count parity spawn vs set-sequence");
}

#[derive(Component, Clone, Debug, PartialEq)]
struct CPos {
    x: i32,
    y: i32,
}

#[derive(Component, Clone, Debug, PartialEq)]
struct CHealth(i32);

#[test]
fn spawn_batch_count_and_distinct_ids() {
    let world = World::new();

    let ids = world.spawn_batch((CPos { x: 1, y: 2 }, CHealth(50)), 1000);
    assert_eq!(ids.len(), 1000);

    let mut unique: std::collections::HashSet<u64> = std::collections::HashSet::new();
    for e in &ids {
        assert!(unique.insert(**e), "duplicate id returned from spawn_batch");
        assert!(world.entity_from_id(*e).is_alive());
    }
    assert_eq!(unique.len(), 1000);
}

#[test]
fn spawn_batch_values_are_correct_across_rows() {
    let world = World::new();
    let ids = world.spawn_batch((CPos { x: 7, y: 8 }, CHealth(3)), 16);
    for e in ids {
        let ev = world.entity_from_id(e);
        assert!(ev.has(CPos::id()));
        assert!(ev.has(CHealth::id()));
        ev.get::<(&CPos, &CHealth)>(|(p, h)| {
            assert_eq!(*p, CPos { x: 7, y: 8 });
            assert_eq!(*h, CHealth(3));
        });
    }
}

#[test]
fn spawn_batch_count_one() {
    let world = World::new();
    let ids = world.spawn_batch((CHealth(42),), 1);
    assert_eq!(ids.len(), 1);
    world
        .entity_from_id(ids[0])
        .get::<&CHealth>(|h| assert_eq!(*h, CHealth(42)));
}

static BATCH_ZERO_DROP: AtomicUsize = AtomicUsize::new(0);

#[derive(Component, Clone)]
struct BatchZeroDrop(u32);

impl Drop for BatchZeroDrop {
    fn drop(&mut self) {
        BATCH_ZERO_DROP.fetch_add(1, SeqCst);
    }
}

#[test]
fn spawn_batch_count_zero_drops_original_and_returns_empty() {
    BATCH_ZERO_DROP.store(0, SeqCst);
    let world = World::new();
    let ids = world.spawn_batch((BatchZeroDrop(1),), 0);
    assert!(ids.is_empty());
    // The original bundle is not stored anywhere, so it is dropped exactly once.
    assert_eq!(BATCH_ZERO_DROP.load(SeqCst), 1);
    drop(world);
}

static BATCH_DROP_COUNT: AtomicUsize = AtomicUsize::new(0);

#[derive(Component, Clone)]
struct BatchDropCounter(u32);

impl Drop for BatchDropCounter {
    fn drop(&mut self) {
        BATCH_DROP_COUNT.fetch_add(1, SeqCst);
    }
}

#[test]
fn spawn_batch_drop_runs_exactly_once_per_stored_value() {
    BATCH_DROP_COUNT.store(0, SeqCst);
    {
        let world = World::new();
        let ids = world.spawn_batch((BatchDropCounter(9),), 100);
        assert_eq!(ids.len(), 100);
        // Values live in storage; clones and the moved original were forgotten.
        assert_eq!(BATCH_DROP_COUNT.load(SeqCst), 0);
    }
    assert_eq!(BATCH_DROP_COUNT.load(SeqCst), 100);
}
