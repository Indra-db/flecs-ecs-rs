//! Deliverable 2: world-level singleton surface (spec §3.5).

use core::sync::atomic::{AtomicU32, Ordering};

use super::{Health, Position};
use flecs_ecs::core::*;

#[test]
fn singleton_read_write_round_trip_both_registers() {
    let mut world = World::new();
    world.set(Position { x: 1, y: 2 });

    // Shared-register read.
    {
        let p = WorldSingletonExt::singleton::<Position>(&world).unwrap();
        assert_eq!((p.x, p.y), (1, 2));
    }

    // Exclusive-register write.
    {
        let p = world.singleton_mut::<Position>().unwrap();
        p.x = 10;
        p.y = 20;
    }

    // Read back through the shared register.
    {
        let p = WorldSingletonExt::singleton::<Position>(&world).unwrap();
        assert_eq!((p.x, p.y), (10, 20));
    }
}

#[test]
fn singleton_unset_is_none() {
    let mut world = World::new();
    assert!(WorldSingletonExt::singleton::<Position>(&world).is_none());
    assert!(world.singleton_mut::<Position>().is_none());
}

#[test]
#[should_panic]
fn singleton_ref_conflicts_with_singleton_write_guard() {
    let world = World::new();
    world.set(Health(3));
    // A live shared read guard on the singleton must conflict with a write guard
    // on the same singleton storage.
    let _r = WorldSingletonExt::singleton::<Health>(&world).unwrap();
    let component = world.entity_from_id(Health::entity_id(&world));
    let _w = component.get::<&mut Health>();
}

#[test]
fn singleton_set_under_live_guard_applies_at_last_guard_drop() {
    // A shared-register singleton `set` issued while a guard is live composes
    // with the episode model: it opens the lazy defer level, so its OnSet
    // observer does not fire until the last guard drops (spec §3.6).
    static CNT: AtomicU32 = AtomicU32::new(0);
    CNT.store(0, Ordering::Relaxed);

    let world = World::new();
    world
        .observer::<flecs::OnSet, &Health>()
        .each_entity(|_e, _h| {
            CNT.fetch_add(1, Ordering::Relaxed);
        });
    world.set(Health(1));
    CNT.store(0, Ordering::Relaxed);

    let guard = WorldSingletonExt::singleton::<Health>(&world).unwrap();
    assert!(!world.is_deferred());
    world.set(Health(99));
    assert!(
        world.is_deferred(),
        "singleton set opens the episode level behind a live guard"
    );
    assert_eq!(
        CNT.load(Ordering::Relaxed),
        0,
        "OnSet observer waits for last-guard-drop"
    );
    drop(guard);

    assert!(!world.is_deferred(), "episode closed at last-guard-drop");
    assert_eq!(
        CNT.load(Ordering::Relaxed),
        1,
        "OnSet observer fires at last-guard-drop"
    );
    let after = WorldSingletonExt::singleton::<Health>(&world).unwrap();
    assert_eq!(after.0, 99);
}
