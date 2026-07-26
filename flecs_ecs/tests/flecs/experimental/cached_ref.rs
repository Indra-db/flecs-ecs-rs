//! Deliverable 1: experimental guard-model `CachedRef` (spec §3.7).

use super::{Health, Position, Velocity};
use flecs_ecs::core::*;
use flecs_ecs::experimental::prelude::*;

#[test]
fn entity_ref_warm_hit_reads_correct_value() {
    let world = World::new();
    let e = world.entity().set(Position { x: 3, y: 7 });
    let cached = world.entity_ref::<Position>(e).unwrap();
    // Repeated warm reads all see the current value.
    for _ in 0..4 {
        let p = cached.get(&world).unwrap();
        assert_eq!((p.x, p.y), (3, 7));
    }
}

#[test]
fn entity_ref_get_mut_writes_through() {
    let world = World::new();
    let e = world.entity().set(Position { x: 1, y: 1 });
    let mut cached = world.entity_ref::<Position>(e).unwrap();
    {
        let mut p = cached.get_mut(&world).unwrap();
        p.x = 42;
        p.y = 99;
    }
    let p = cached.get(&world).unwrap();
    assert_eq!((p.x, p.y), (42, 99));
}

#[test]
fn entity_ref_absent_component_is_none() {
    let world = World::new();
    let e = world.entity().set(Position::default());
    assert!(world.entity_ref::<Velocity>(e).is_none());
}

#[test]
fn entity_ref_dead_entity_is_none_at_build() {
    let world = World::new();
    let e = world.entity().set(Position::default());
    e.destruct();
    assert!(world.entity_ref::<Position>(e).is_none());
}

#[test]
fn cached_ref_revalidates_after_archetype_change() {
    let world = World::new();
    let e = world.entity().set(Position { x: 10, y: 20 });
    let cached = world.entity_ref::<Position>(e).unwrap();
    // Warm the cache in the original table.
    assert_eq!(cached.get(&world).unwrap().x, 10);

    // Move the entity to a new archetype: the cached table id is now stale, so
    // the next access must revalidate and still return the correct data.
    e.set(Velocity { x: 1, y: 1 });
    e.set(Health(5));
    {
        let p = cached.get(&world).unwrap();
        assert_eq!((p.x, p.y), (10, 20));
    }

    // Mutable access refreshes the cached table / column and still writes
    // through after the move.
    let mut cached = cached;
    cached.get_mut(&world).unwrap().x = 111;
    assert_eq!(cached.get(&world).unwrap().x, 111);
}

#[test]
fn cached_ref_get_after_death_is_none() {
    let world = World::new();
    let e = world.entity().set(Position { x: 1, y: 2 });
    let cached = world.entity_ref::<Position>(e).unwrap();
    assert!(cached.get(&world).is_some());
    e.destruct();
    assert!(cached.get(&world).is_none());
}

#[test]
fn cached_ref_read_read_ok() {
    let world = World::new();
    let e = world.entity().set(Position { x: 5, y: 6 });
    let cached = world.entity_ref::<Position>(e).unwrap();
    let a = cached.get(&world).unwrap();
    let b = cached.get(&world).unwrap();
    assert_eq!(a.x, b.x);
}

#[test]
#[should_panic]
fn cached_ref_conflicts_with_live_write_guard() {
    use flecs_ecs::experimental::EntityGuardExt;
    let world = World::new();
    let e = world.entity().set(Position::default());
    // A live write guard on the same storage must make a cached read conflict.
    let _w = e.get_ref::<&mut Position>().unwrap();
    let cached = world.entity_ref::<Position>(e).unwrap();
    let _r = cached.get(&world);
}

#[test]
fn cached_ref_guard_drop_releases_pin_teardown_clean() {
    // A guard taken and dropped leaves no stuck pin, so world teardown runs
    // cleanly (a leaked episode would abort ecs_fini).
    let world = World::new();
    let e = world.entity().set(Position { x: 1, y: 2 });
    let mut cached = world.entity_ref::<Position>(e).unwrap();
    for _i in 0..8 {
        let r = cached.get(&world).unwrap();
        assert_eq!(r.x, 1 + _i);
        drop(r);
        let mut m = cached.get_mut(&world).unwrap();
        m.x += 1;
        drop(m);
    }
    assert_eq!(cached.get(&world).unwrap().x, 9);
    // world drops here: no panic / abort.
}
