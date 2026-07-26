#![allow(dead_code)]

use core::sync::atomic::{AtomicUsize, Ordering::SeqCst};

use alloc::sync::Arc;

use flecs_ecs::core::*;
use flecs_ecs::experimental::prelude::*;
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
    let mut world = World::new();

    let e = world
        .spawn((Pos { x: 1, y: 2 }, Vel { x: 3, y: 4 }, Health(100)))
        .id();
    let ev = world.entity_from_id(e);

    assert!(ev.is_alive());
    assert!(ev.has(Pos::id()));
    assert!(ev.has(Vel::id()));
    assert!(ev.has(Health::id()));

    {
        let (p, v, h) = ev.get_ref::<(&Pos, &Vel, &Health)>().unwrap();
        assert_eq!(*p, Pos { x: 1, y: 2 });
        assert_eq!(*v, Vel { x: 3, y: 4 });
        assert_eq!(*h, Health(100));
    };
}

#[test]
fn spawn_single_component_bundle() {
    let mut world = World::new();
    let e = world.spawn((Health(7),)).id();
    {
        let h = world.entity_from_id(e).get_ref::<&Health>().unwrap();
        assert_eq!(*h, Health(7));
    };
}

#[test]
fn spawn_registers_unregistered_components() {
    #[derive(Component, PartialEq, Debug)]
    struct Fresh1(u32);
    #[derive(Component, PartialEq, Debug)]
    struct Fresh2(u32);

    let mut world = World::new();
    // Components have never been touched before this spawn.
    let e = world.spawn((Fresh1(11), Fresh2(22))).id();
    let ev = world.entity_from_id(e);
    assert!(ev.has(Fresh1::id()));
    assert!(ev.has(Fresh2::id()));
    {
        let (a, b) = ev.get_ref::<(&Fresh1, &Fresh2)>().unwrap();
        assert_eq!(*a, Fresh1(11));
        assert_eq!(*b, Fresh2(22));
    };
}

#[test]
fn spawn_with_zero_sized_tag() {
    let mut world = World::new();
    let e = world.spawn((Pos { x: 5, y: 6 }, Tag, Marker)).id();
    let ev = world.entity_from_id(e);
    assert!(ev.has(Pos::id()));
    assert!(ev.has(Tag::id()));
    assert!(ev.has(Marker::id()));
    {
        let p = ev.get_ref::<&Pos>().unwrap();
        assert_eq!(*p, Pos { x: 5, y: 6 });
    };
}

#[test]
fn spawn_only_tags() {
    let mut world = World::new();
    let e = world.spawn((Tag, Marker)).id();
    let ev = world.entity_from_id(e);
    assert!(ev.has(Tag::id()));
    assert!(ev.has(Marker::id()));
}

#[test]
#[should_panic(expected = "duplicate component type")]
fn spawn_duplicate_component_type_panics() {
    let mut world = World::new();
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
        let mut world = World::new();
        let _e = world.spawn((SpawnDropCounter(1), Health(2))).id();
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
    let mut world = World::new();
    let e = world.spawn((DeleteDropCounter(9),)).id();
    assert_eq!(SPAWN_DELETE_DROP.load(SeqCst), 0);
    world.entity_from_id(e).destruct();
    assert_eq!(SPAWN_DELETE_DROP.load(SeqCst), 1);
}

#[test]
fn spawn_observer_parity_with_set_sequence() {
    let mut world = World::new();

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
    let mut world = World::new();

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
    let mut world = World::new();
    let ids = world.spawn_batch((CPos { x: 7, y: 8 }, CHealth(3)), 16);
    for e in ids {
        let ev = world.entity_from_id(e);
        assert!(ev.has(CPos::id()));
        assert!(ev.has(CHealth::id()));
        {
            let (p, h) = ev.get_ref::<(&CPos, &CHealth)>().unwrap();
            assert_eq!(*p, CPos { x: 7, y: 8 });
            assert_eq!(*h, CHealth(3));
        };
    }
}

#[test]
fn spawn_batch_count_one() {
    let mut world = World::new();
    let ids = world.spawn_batch((CHealth(42),), 1);
    assert_eq!(ids.len(), 1);
    {
        let h = world.entity_from_id(ids[0]).get_ref::<&CHealth>().unwrap();
        assert_eq!(*h, CHealth(42));
    };
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
    let mut world = World::new();
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
        let mut world = World::new();
        let ids = world.spawn_batch((BatchDropCounter(9),), 100);
        assert_eq!(ids.len(), 100);
        // Values live in storage; clones and the moved original were forgotten.
        assert_eq!(BATCH_DROP_COUNT.load(SeqCst), 0);
    }
    assert_eq!(BATCH_DROP_COUNT.load(SeqCst), 100);
}

#[test]
fn insert_adds_all_components_with_values() {
    let world = World::new();
    let e = world.entity();
    let ret = e.insert((Pos { x: 1, y: 2 }, Vel { x: 3, y: 4 }, Health(5)));
    assert_eq!(ret.id(), e.id());

    assert!(e.has(Pos::id()));
    assert!(e.has(Vel::id()));
    assert!(e.has(Health::id()));
    {
        let (p, v, h) = e.get_ref::<(&Pos, &Vel, &Health)>().unwrap();
        assert_eq!(*p, Pos { x: 1, y: 2 });
        assert_eq!(*v, Vel { x: 3, y: 4 });
        assert_eq!(*h, Health(5));
    };
}

#[test]
fn insert_onto_entity_with_existing_components() {
    let world = World::new();
    let e = world.entity().set(Pos { x: 9, y: 9 });
    e.insert((Vel { x: 1, y: 1 }, Health(3)));
    assert!(e.has(Pos::id()));
    assert!(e.has(Vel::id()));
    assert!(e.has(Health::id()));
    {
        let (p, v, h) = e.get_ref::<(&Pos, &Vel, &Health)>().unwrap();
        assert_eq!(*p, Pos { x: 9, y: 9 });
        assert_eq!(*v, Vel { x: 1, y: 1 });
        assert_eq!(*h, Health(3));
    };
}

#[test]
fn insert_with_tag() {
    let world = World::new();
    let e = world.entity();
    e.insert((Health(7), Tag));
    assert!(e.has(Health::id()));
    assert!(e.has(Tag::id()));
    {
        let h = e.get_ref::<&Health>().unwrap();
        assert_eq!(*h, Health(7));
    };
}

#[test]
#[should_panic(expected = "duplicate component type")]
fn insert_duplicate_component_type_panics() {
    let world = World::new();
    let e = world.entity();
    e.insert((Health(1), Health(2)));
}

/// A single-move insert lands the entity in its FINAL table before any `OnAdd`
/// fires, so an `OnAdd` observer for the first component already sees the other
/// components present. A per-component (N-move) sequence would fire `OnAdd` for
/// the first component while the later ones are still absent.
#[test]
fn insert_is_a_single_table_move() {
    let world = World::new();

    let all_present = Arc::new(AtomicUsize::new(0));
    let fired = Arc::new(AtomicUsize::new(0));

    {
        let all_present = all_present.clone();
        let fired = fired.clone();
        world
            .observer::<flecs::OnAdd, ()>()
            .with(Pos::id())
            .each_entity(move |e, _| {
                fired.fetch_add(1, SeqCst);
                if e.has(Vel::id()) && e.has(Health::id()) {
                    all_present.fetch_add(1, SeqCst);
                }
            });
    }

    let e = world.entity();
    e.insert((Pos { x: 1, y: 1 }, Vel { x: 2, y: 2 }, Health(3)));

    assert_eq!(fired.load(SeqCst), 1, "OnAdd(Pos) fired once");
    assert_eq!(
        all_present.load(SeqCst),
        1,
        "entity already in final table (Vel + Health present) when OnAdd(Pos) fired -> single move"
    );
}

#[test]
fn insert_observer_parity_with_set_sequence() {
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

    world
        .entity()
        .insert((Pos { x: 1, y: 1 }, Vel { x: 2, y: 2 }));
    let insert_add = on_add.swap(0, SeqCst);
    let insert_set = on_set.swap(0, SeqCst);

    world.entity().set(Pos { x: 1, y: 1 }).set(Vel { x: 2, y: 2 });
    let seq_add = on_add.swap(0, SeqCst);
    let seq_set = on_set.swap(0, SeqCst);

    assert_eq!(insert_add, 2);
    assert_eq!(insert_set, 2);
    assert_eq!(insert_add, seq_add);
    assert_eq!(insert_set, seq_set);
}

#[test]
fn insert_in_deferred_context_merges() {
    let world = World::new();
    let e = world.entity();

    world.defer(|| {
        e.insert((Pos { x: 4, y: 5 }, Health(6)));
        // Deferred: not yet applied.
        assert!(!e.has(Pos::id()));
    });

    // Merged at the sync point.
    assert!(e.has(Pos::id()));
    assert!(e.has(Health::id()));
    {
        let (p, h) = e.get_ref::<(&Pos, &Health)>().unwrap();
        assert_eq!(*p, Pos { x: 4, y: 5 });
        assert_eq!(*h, Health(6));
    };
}

static INSERT_DROP_COUNT: AtomicUsize = AtomicUsize::new(0);

#[derive(Component)]
struct InsertDropCounter(u32);

impl Drop for InsertDropCounter {
    fn drop(&mut self) {
        INSERT_DROP_COUNT.fetch_add(1, SeqCst);
    }
}

#[test]
fn insert_drop_runs_exactly_once_per_value() {
    INSERT_DROP_COUNT.store(0, SeqCst);
    {
        let world = World::new();
        let e = world.entity();
        e.insert((InsertDropCounter(1), Health(2)));
        assert_eq!(INSERT_DROP_COUNT.load(SeqCst), 0);
        {
            let c = e.get_ref::<&InsertDropCounter>().unwrap();
            assert_eq!(c.0, 1);
        };
    }
    assert_eq!(INSERT_DROP_COUNT.load(SeqCst), 1);
}

static INSERT_DEFAULT_DROP: AtomicUsize = AtomicUsize::new(0);

#[derive(Component, Default)]
struct InsertDefaultDrop(u32);

impl Drop for InsertDefaultDrop {
    fn drop(&mut self) {
        INSERT_DEFAULT_DROP.fetch_add(1, SeqCst);
    }
}

/// A `Default` + `Drop` component newly added by `insert` has its slot
/// default-constructed by the commit; the value write must drop that transient
/// default (never leak it) before moving the real value in. So exactly two drops
/// occur overall: the discarded default, then the stored value at teardown. The
/// important guarantees are no leak and no double-free of the same object.
#[test]
fn insert_default_component_drops_transient_default_no_leak() {
    INSERT_DEFAULT_DROP.store(0, SeqCst);
    {
        let world = World::new();
        let e = world.entity();
        e.insert((InsertDefaultDrop(7),));
        // The default constructed by the commit was dropped when the real value
        // was moved in.
        assert_eq!(INSERT_DEFAULT_DROP.load(SeqCst), 1);
        {
            let c = e.get_ref::<&InsertDefaultDrop>().unwrap();
            assert_eq!(c.0, 7);
        };
    }
    // Plus the stored value at world teardown.
    assert_eq!(INSERT_DEFAULT_DROP.load(SeqCst), 2);
}

#[cfg(feature = "flecs_safety_locks")]
mod live_guard {
    use super::*;

    // The former `spawn_panics_with_live_guard` / `spawn_batch_panics_with_live_guard`
    // runtime refusals are now COMPILE errors: `spawn` / `spawn_batch` take
    // `&mut World`, so a live shared-register guard (which borrows the world
    // shared) cannot coexist with the call. The demonstrations are the
    // `compile_fail` doctests on `WorldBundleExt::spawn`.

    /// `spawn` still refuses an open defer scope at runtime: `ecs_bulk_init` must
    /// observe its ids immediately. The scope is opened with `defer_begin`
    /// (a `&World` op that returns), so `&mut World` remains available and the
    /// refusal is reachable from safe code.
    #[test]
    #[should_panic(expected = "while the world is deferred")]
    fn spawn_panics_in_defer_scope() {
        let mut world = World::new();
        world.defer_begin();
        let _ = world.spawn((Pos { x: 1, y: 2 },));
        world.defer_end();
    }

    /// `insert` under a live guard routes through the write episode: the guard
    /// stays valid across the call, nothing applies (and no observer fires)
    /// while the guard is live, and the bundle (with its `OnAdd` / `OnSet`) applies
    /// when the last guard drops.
    #[test]
    fn insert_under_live_guard_defers_and_applies_at_guard_drop() {
        let world = World::new();
        let e = world.entity().set(Pos { x: 11, y: 22 });

        let on_add = Arc::new(AtomicUsize::new(0));
        let on_set = Arc::new(AtomicUsize::new(0));
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
            world.observer::<flecs::OnSet, &Vel>().each(move |_| {
                s.fetch_add(1, SeqCst);
            });
        }

        {
            let g = e.get_ref::<&Pos>().unwrap();
            e.insert((Vel { x: 3, y: 4 }, Health(5)));

            // Guard data stays valid and unmoved across the call.
            assert_eq!(g.x, 11);
            assert_eq!(g.y, 22);

            // Nothing applied, no observer fired while the guard pins storage.
            assert!(!e.has(Vel::id()), "insert must defer under a live guard");
            assert!(!e.has(Health::id()), "insert must defer under a live guard");
            assert_eq!(on_add.load(SeqCst), 0, "OnAdd must wait for guard drop");
            assert_eq!(on_set.load(SeqCst), 0, "OnSet must wait for guard drop");
        }

        // Last guard dropped: the episode closes and the bundle applies.
        assert!(e.has(Vel::id()));
        assert!(e.has(Health::id()));
        assert_eq!(on_add.load(SeqCst), 1, "OnAdd fires once after guard drop");
        assert_eq!(on_set.load(SeqCst), 1, "OnSet fires once after guard drop");
        {
            let (v, h) = e.get_ref::<(&Vel, &Health)>().unwrap();
            assert_eq!(*v, Vel { x: 3, y: 4 });
            assert_eq!(*h, Health(5));
        };
    }
}
