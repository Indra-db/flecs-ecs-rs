//! Deliverable 1: guard-based entity access (shared register).

use super::{Health, Position, Velocity};
use flecs_ecs::core::*;
use flecs_ecs::experimental::prelude::*;
use flecs_ecs::macros::query;

#[test]
fn get_ref_read() {
    let world = World::new();
    let e = world.entity().set(Position { x: 1, y: 2 });
    let p = e.get_ref::<&Position>().unwrap();
    assert_eq!(p.x, 1);
    assert_eq!(p.y, 2);
}

#[test]
fn get_ref_write_mutates() {
    let world = World::new();
    let e = world.entity().set(Position { x: 1, y: 2 });
    {
        let mut p = e.get_ref::<&mut Position>().unwrap();
        p.x += 10;
    }
    let p = e.get_ref::<&Position>().unwrap();
    assert_eq!(p.x, 11);
}

#[test]
fn get_ref_tuple_fused() {
    let world = World::new();
    let e = world
        .entity()
        .set(Position { x: 1, y: 2 })
        .set(Velocity { x: 3, y: 4 });
    let (mut p, v) = e.get_ref::<(&mut Position, &Velocity)>().unwrap();
    p.x += v.x;
    p.y += v.y;
    drop((p, v));
    let p = e.get_ref::<&Position>().unwrap();
    assert_eq!((p.x, p.y), (4, 6));
}

#[test]
fn get_ref_missing_is_none() {
    let world = World::new();
    let e = world.entity().set(Position { x: 1, y: 2 });
    assert!(e.get_ref::<&Velocity>().is_none());
}

#[test]
fn get_ref_dead_entity_is_none() {
    let world = World::new();
    let e = world.entity().set(Position { x: 1, y: 2 });
    e.destruct();
    assert!(e.get_ref::<&Position>().is_none());
}

// --- conflict, guard vs guard ---

#[test]
#[should_panic]
fn get_ref_write_write_panics() {
    let world = World::new();
    let e = world.entity().set(Position::default());
    let _a = e.get_ref::<&mut Position>().unwrap();
    let _b = e.get_ref::<&mut Position>().unwrap();
}

#[test]
#[should_panic]
fn get_ref_read_write_panics() {
    let world = World::new();
    let e = world.entity().set(Position::default());
    let _a = e.get_ref::<&Position>().unwrap();
    let _b = e.get_ref::<&mut Position>().unwrap();
}

#[test]
#[should_panic]
fn get_ref_write_read_panics() {
    let world = World::new();
    let e = world.entity().set(Position::default());
    let _a = e.get_ref::<&mut Position>().unwrap();
    let _b = e.get_ref::<&Position>().unwrap();
}

#[test]
fn get_ref_read_read_ok() {
    let world = World::new();
    let e = world.entity().set(Position { x: 5, y: 6 });
    let a = e.get_ref::<&Position>().unwrap();
    let b = e.get_ref::<&Position>().unwrap();
    assert_eq!(a.x, b.x);
}

// --- try_get_ref returns the conflict as a value, both directions ---

#[test]
fn try_get_ref_read_then_write_is_conflict() {
    let world = World::new();
    let e = world.entity().set(Position::default());
    let _a = e.get_ref::<&Position>().unwrap();
    match e.try_get_ref::<&mut Position>().err() {
        Some(AccessError::Conflict { write, .. }) => assert!(write),
        other => panic!("expected write conflict, got {other:?}"),
    }
}

#[test]
fn try_get_ref_write_then_read_is_conflict() {
    let world = World::new();
    let e = world.entity().set(Position::default());
    let _a = e.get_ref::<&mut Position>().unwrap();
    match e.try_get_ref::<&Position>().err() {
        Some(AccessError::Conflict { write, .. }) => assert!(!write),
        other => panic!("expected read conflict, got {other:?}"),
    }
}

#[test]
fn try_get_ref_missing_component() {
    let world = World::new();
    let e = world.entity().set(Position::default());
    assert_eq!(
        e.try_get_ref::<&Velocity>().err(),
        Some(AccessError::MissingComponent)
    );
}

// --- guard vs query term ---

#[test]
#[should_panic]
fn query_write_then_guard_read_panics() {
    let world = World::new();
    world.entity().set(Position::default());
    query!(world, &mut Position)
        .build()
        .each_entity(|e, _| {
            let _g = e.get_ref::<&Position>().unwrap();
        });
}

#[test]
#[should_panic]
fn query_read_then_guard_write_panics() {
    let world = World::new();
    world.entity().set(Position::default());
    query!(world, &Position).build().each_entity(|e, _| {
        let _g = e.get_ref::<&mut Position>().unwrap();
    });
}

#[test]
fn query_read_then_guard_read_ok() {
    let world = World::new();
    world.entity().set(Position { x: 7, y: 8 });
    let mut seen = 0;
    query!(world, &Position).build().each_entity(|e, _| {
        let g = e.get_ref::<&Position>().unwrap();
        seen += g.x;
    });
    assert_eq!(seen, 7);
}

// --- NLL / temporary scoping ends a guard at last use ---

#[test]
fn sequential_mut_guards_in_one_scope() {
    let world = World::new();
    let e = world.entity().set(Health(0));
    // Each guard is a temporary released at the end of its statement, so the
    // second acquire does not conflict with the first.
    e.get_ref::<&mut Health>().unwrap().0 += 1;
    e.get_ref::<&mut Health>().unwrap().0 += 1;
    assert_eq!(e.get_ref::<&Health>().unwrap().0, 2);
}

#[test]
fn explicit_drop_then_reacquire() {
    let world = World::new();
    let e = world.entity().set(Health(0));
    let mut g = e.get_ref::<&mut Health>().unwrap();
    g.0 = 41;
    drop(g);
    let mut g2 = e.get_ref::<&mut Health>().unwrap();
    g2.0 += 1;
    drop(g2);
    assert_eq!(e.get_ref::<&Health>().unwrap().0, 42);
}

// --- cloned_owned: no guard held after return ---

#[test]
fn cloned_owned_releases_lock() {
    let world = World::new();
    let e = world.entity().set(Position { x: 9, y: 9 });
    let owned = e.cloned_owned::<&Position>().unwrap();
    assert_eq!(owned.x, 9);
    // No guard is held, so a fresh mutable borrow must succeed.
    let mut p = e.get_ref::<&mut Position>().unwrap();
    p.x = 0;
}

// --- panic safety: a guard live during a panic must not wedge the map ---

#[test]
fn guard_live_during_panic_does_not_wedge() {
    let world = World::new();
    let e = world.entity().set(Position { x: 1, y: 1 });

    let result = std::panic::catch_unwind(core::panic::AssertUnwindSafe(|| {
        let _g = e.get_ref::<&mut Position>().unwrap();
        panic!("boom while a guard is live");
    }));
    assert!(result.is_err());

    // The unwound guard released its borrow, so the map is usable again.
    let mut p = e.get_ref::<&mut Position>().unwrap();
    p.x = 100;
    drop(p);
    assert_eq!(e.get_ref::<&Position>().unwrap().x, 100);
}
