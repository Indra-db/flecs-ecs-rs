//! Pins the defer semantics of the `get` access scope: operations performed
//! inside the callback are queued and applied after the callback returns, so
//! the borrowed component pointers stay valid and observers never run while
//! the borrow is live.

use core::sync::atomic::{AtomicU32, Ordering};

use flecs_ecs::core::*;
use flecs_ecs::macros::*;

#[derive(Component)]
struct Foo(i32);

#[derive(Component, Default)]
struct Bar(i32);

/// A `set` inside a `get` callback is deferred: no observer fires while the
/// callback's borrow is live, the component is not yet visible inside the
/// callback, and both the value and its observer land after `get` returns.
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

    e.get::<&mut Foo>(|foo| {
        e.set(Bar(42));
        assert_eq!(
            ON_SET_COUNT.load(Ordering::Relaxed),
            0,
            "observers must not run while the get borrow is live"
        );
        assert!(
            !e.has(Bar::id()),
            "deferred set must not be visible inside the callback"
        );
        foo.0 += 1;
    });

    assert_eq!(ON_SET_COUNT.load(Ordering::Relaxed), 1);
    assert!(e.has(Bar::id()));
    e.get::<(&Foo, &Bar)>(|(foo, bar)| {
        assert_eq!(foo.0, 2);
        assert_eq!(bar.0, 42);
    });
}

/// Same guarantee for `try_get`.
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

    let ran = e.try_get::<&mut Foo>(|foo| {
        e.set(Bar(42));
        assert_eq!(ON_SET_COUNT.load(Ordering::Relaxed), 0);
        assert!(!e.has(Bar::id()));
        foo.0 += 1;
    });

    assert!(ran.is_some());
    assert_eq!(ON_SET_COUNT.load(Ordering::Relaxed), 1);
    assert!(e.has(Bar::id()));
}

/// Same guarantee for the singleton `world.get`.
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

    world.get::<&mut Foo>(|foo| {
        world.set(Bar(42));
        assert_eq!(
            ON_SET_COUNT.load(Ordering::Relaxed),
            0,
            "observers must not run while the singleton borrow is live"
        );
        foo.0 += 1;
    });

    assert_eq!(ON_SET_COUNT.load(Ordering::Relaxed), 1);
    world.get::<(&Foo, &Bar)>(|(foo, bar)| {
        assert_eq!(foo.0, 2);
        assert_eq!(bar.0, 42);
    });
}

#[derive(Component)]
struct MoveTag;

/// The staleness case from the pre-defer days: an archetype move of the
/// borrowed entity inside its own `get` callback. The move is deferred, the
/// borrow stays valid, and the value written through it survives the table
/// move at flush.
#[test]
fn archetype_move_in_own_get_callback_keeps_borrow_valid() {
    let world = World::new();
    let e = world.entity().set(Foo(1));

    e.get::<&mut Foo>(|foo| {
        e.add(MoveTag::id());
        assert!(
            !e.has(MoveTag::id()),
            "deferred add must not be visible inside the callback"
        );
        foo.0 = 99;
    });

    assert!(e.has(MoveTag::id()));
    e.get::<&Foo>(|foo| {
        assert_eq!(
            foo.0, 99,
            "value written through the borrow must survive the move"
        );
    });
}

/// Deleting the borrowed entity inside its own `get` callback is deferred:
/// the borrow stays valid for the rest of the callback, the deletion lands
/// after.
#[test]
fn destruct_in_own_get_callback_is_deferred() {
    let world = World::new();
    let e = world.entity().set(Foo(1));

    e.get::<&mut Foo>(|foo| {
        e.destruct();
        assert!(e.is_alive(), "deferred destruct must not land mid-callback");
        foo.0 = 5;
    });

    assert!(!e.is_alive());
}

/// Spawning entities into the borrowed entity's own table inside the callback
/// (which would reallocate the column) is deferred, so the borrow stays valid.
#[test]
fn same_table_spawns_in_get_callback_are_deferred() {
    let world = World::new();
    let e = world.entity().set(Foo(1));

    e.get::<&mut Foo>(|foo| {
        for i in 0..64 {
            world.entity().set(Foo(i));
        }
        foo.0 = 7;
    });

    e.get::<&Foo>(|foo| assert_eq!(foo.0, 7));
}

/// `remove` inside a `get` callback: the `on_remove` observer runs after the
/// callback returns, never while the borrow is live.
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

    e.get::<&mut Foo>(|foo| {
        e.remove(Bar::id());
        assert_eq!(
            ON_REMOVE_COUNT.load(Ordering::Relaxed),
            0,
            "on_remove must not run while the borrow is live"
        );
        assert!(
            e.has(Bar::id()),
            "deferred remove must not land mid-callback"
        );
        foo.0 += 1;
    });

    assert_eq!(ON_REMOVE_COUNT.load(Ordering::Relaxed), 1);
    assert!(!e.has(Bar::id()));
    e.get::<&Foo>(|foo| assert_eq!(foo.0, 2));
}
