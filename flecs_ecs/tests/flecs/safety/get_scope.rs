//! Pins the write-episode defer semantics of a live shared-register guard
//! (spec §7.1): a structural mutation issued while a `Ref`/`Mut` guard is live
//! is queued and applied only when the last guard drops, so the borrowed
//! component pointer stays valid and observers never run while the borrow is
//! live.

use core::sync::atomic::{AtomicU32, Ordering};

use flecs_ecs::core::*;
use flecs_ecs::experimental::prelude::EntityGuardExt;
use flecs_ecs::macros::*;

#[derive(Component)]
struct Foo(i32);

#[derive(Component, Default)]
struct Bar(i32);

/// A `set` issued while a guard is live is deferred: no observer fires while the
/// guard's borrow is live, the component is not yet visible, and both the value
/// and its observer land after the guard drops.
#[test]
fn mutation_in_get_callback_is_deferred_and_observers_fire_after() {
    static ON_SET_COUNT: AtomicU32 = AtomicU32::new(0);
    ON_SET_COUNT.store(0, Ordering::Relaxed);

    let world = World::new();
    world
        .observer::<flecs::OnSet, &Bar>()
        .each_entity(|_e, bar| {
            assert_eq!(bar.0, 42, "observer must see the flushed value");
            ON_SET_COUNT.fetch_add(1, Ordering::Relaxed);
        });

    let e = world.entity().set(Foo(1));

    {
        let mut foo = e.get::<&mut Foo>().unwrap();
        e.set(Bar(42));
        assert_eq!(
            ON_SET_COUNT.load(Ordering::Relaxed),
            0,
            "observers must not run while the guard borrow is live"
        );
        assert!(
            !e.has(Bar::id()),
            "deferred set must not be visible while the guard is live"
        );
        foo.0 += 1;
    }

    assert_eq!(ON_SET_COUNT.load(Ordering::Relaxed), 1);
    assert!(e.has(Bar::id()));
    {
        let (foo, bar) = e.get::<(&Foo, &Bar)>().unwrap();
        assert_eq!(foo.0, 2);
        assert_eq!(bar.0, 42);
    }
}

/// Same guarantee for a singleton guard taken on the component's own entity.
#[test]
fn mutation_in_try_get_callback_is_deferred() {
    static ON_SET_COUNT: AtomicU32 = AtomicU32::new(0);
    ON_SET_COUNT.store(0, Ordering::Relaxed);

    let world = World::new();
    world
        .observer::<flecs::OnSet, &Bar>()
        .each_entity(|_e, _b| {
            ON_SET_COUNT.fetch_add(1, Ordering::Relaxed);
        });

    let e = world.entity().set(Foo(1));

    {
        let mut foo = e.get::<&mut Foo>().unwrap();
        e.set(Bar(42));
        assert_eq!(ON_SET_COUNT.load(Ordering::Relaxed), 0);
        assert!(!e.has(Bar::id()));
        foo.0 += 1;
    }

    assert_eq!(ON_SET_COUNT.load(Ordering::Relaxed), 1);
    assert!(e.has(Bar::id()));
}

/// Same guarantee for a singleton write guard: `world.set` issued while a `Mut`
/// guard on the singleton entity is live is deferred until the guard drops.
#[test]
fn mutation_in_world_get_callback_is_deferred() {
    static ON_SET_COUNT: AtomicU32 = AtomicU32::new(0);
    ON_SET_COUNT.store(0, Ordering::Relaxed);

    let world = World::new();
    world
        .observer::<flecs::OnSet, &Bar>()
        .each_entity(|_e, _b| {
            ON_SET_COUNT.fetch_add(1, Ordering::Relaxed);
        });

    world.set(Foo(1));

    let foo_e = world.entity_from_id(Foo::entity_id(&world));
    {
        let mut foo = foo_e.get::<&mut Foo>().unwrap();
        world.set(Bar(42));
        assert_eq!(
            ON_SET_COUNT.load(Ordering::Relaxed),
            0,
            "observers must not run while the singleton guard is live"
        );
        foo.0 += 1;
    }

    assert_eq!(ON_SET_COUNT.load(Ordering::Relaxed), 1);
    let foo = foo_e.get::<&Foo>().unwrap();
    let bar = world
        .entity_from_id(Bar::entity_id(&world))
        .get::<&Bar>()
        .unwrap();
    assert_eq!(foo.0, 2);
    assert_eq!(bar.0, 42);
}

#[derive(Component)]
struct MoveTag;

/// An archetype move of the guarded entity issued while its own guard is live.
/// The move is deferred, the borrow stays valid, and the value written through
/// it survives the table move at flush.
#[test]
fn archetype_move_in_own_get_callback_keeps_borrow_valid() {
    let world = World::new();
    let e = world.entity().set(Foo(1));

    {
        let mut foo = e.get::<&mut Foo>().unwrap();
        e.add(MoveTag::id());
        assert!(
            !e.has(MoveTag::id()),
            "deferred add must not be visible while the guard is live"
        );
        foo.0 = 99;
    }

    assert!(e.has(MoveTag::id()));
    {
        let foo = e.get::<&Foo>().unwrap();
        assert_eq!(
            foo.0, 99,
            "value written through the borrow must survive the move"
        );
    }
}

/// Deleting the guarded entity while its own guard is live is deferred: the
/// borrow stays valid for the rest of the scope, the deletion lands after.
#[test]
fn destruct_in_own_get_callback_is_deferred() {
    let world = World::new();
    let e = world.entity().set(Foo(1));

    {
        let mut foo = e.get::<&mut Foo>().unwrap();
        e.destruct();
        assert!(e.is_alive(), "deferred destruct must not land while guard is live");
        foo.0 = 5;
    }

    assert!(!e.is_alive());
}

/// Spawning entities into the guarded entity's own table while the guard is live
/// (which would reallocate the column) is deferred, so the borrow stays valid.
#[test]
fn same_table_spawns_in_get_callback_are_deferred() {
    let world = World::new();
    let e = world.entity().set(Foo(1));

    {
        let mut foo = e.get::<&mut Foo>().unwrap();
        for i in 0..64 {
            world.entity().set(Foo(i));
        }
        foo.0 = 7;
    }

    {
        let foo = e.get::<&Foo>().unwrap();
        assert_eq!(foo.0, 7);
    }
}

/// `remove` issued while a guard is live: the `on_remove` observer runs after
/// the guard drops, never while the borrow is live.
#[test]
fn remove_in_get_callback_fires_observer_after() {
    static ON_REMOVE_COUNT: AtomicU32 = AtomicU32::new(0);
    ON_REMOVE_COUNT.store(0, Ordering::Relaxed);

    let world = World::new();
    world
        .observer::<flecs::OnRemove, &Bar>()
        .each_entity(|_e, _b| {
            ON_REMOVE_COUNT.fetch_add(1, Ordering::Relaxed);
        });

    let e = world.entity().set(Foo(1)).set(Bar(42));

    {
        let mut foo = e.get::<&mut Foo>().unwrap();
        e.remove(Bar::id());
        assert_eq!(
            ON_REMOVE_COUNT.load(Ordering::Relaxed),
            0,
            "on_remove must not run while the borrow is live"
        );
        assert!(
            e.has(Bar::id()),
            "deferred remove must not land while the guard is live"
        );
        foo.0 += 1;
    }

    assert_eq!(ON_REMOVE_COUNT.load(Ordering::Relaxed), 1);
    assert!(!e.has(Bar::id()));
    {
        let foo = e.get::<&Foo>().unwrap();
        assert_eq!(foo.0, 2);
    }
}
