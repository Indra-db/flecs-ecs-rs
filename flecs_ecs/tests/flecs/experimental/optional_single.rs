//! Bare `Option<&T>` / `Option<&mut T>` as single-element guard requests
//! (spec §3.4 / GAP-3): a lone optional is a valid `get` shape, not only
//! the `(Option<&T>,)` 1-tuple form.

use super::{Position, Velocity};
use flecs_ecs::core::*;
use flecs_ecs::experimental::prelude::*;

#[test]
fn option_ref_present_is_some_some_and_locks_read() {
    let world = World::new();
    let e = world.entity().set(Position { x: 1, y: 2 });

    let g = e.get::<Option<&Position>>().unwrap();
    assert_eq!(g.as_ref().unwrap().x, 1);
    assert_eq!(g.as_ref().unwrap().y, 2);
    // A present optional registers its borrow: a live read blocks a write.
    assert!(matches!(
        e.try_get::<&mut Position>(),
        Err(AccessError::Conflict { write: true, .. })
    ));
    drop(g);
}

#[test]
fn option_ref_absent_is_some_none_and_locks_nothing() {
    let world = World::new();
    let e = world.entity().set(Position { x: 1, y: 2 });

    // Velocity is absent: the all-optional acquire returns holding nothing.
    let a = e.get::<Option<&Velocity>>().unwrap();
    assert!(a.is_none());
    // No borrow or pin was taken, so a second (would-be conflicting) optional
    // on the same absent component succeeds instead of reporting a conflict.
    let b = e.get::<Option<&mut Velocity>>().unwrap();
    assert!(b.is_none());
    drop((a, b));
}

#[test]
fn option_mut_present_is_some_some_and_mutates() {
    let world = World::new();
    let e = world.entity().set(Position { x: 1, y: 2 });

    {
        let mut g = e.get::<Option<&mut Position>>().unwrap();
        g.as_mut().unwrap().x += 10;
    }
    assert_eq!(e.get::<&Position>().unwrap().x, 11);
}

#[test]
fn option_mut_present_locks_write() {
    let world = World::new();
    let e = world.entity().set(Position { x: 1, y: 2 });

    let g = e.get::<Option<&mut Position>>().unwrap();
    // A live write blocks a read of the same component.
    assert!(matches!(
        e.try_get::<&Position>(),
        Err(AccessError::Conflict { write: false, .. })
    ));
    drop(g);
}

#[test]
fn option_mut_absent_is_some_none() {
    let world = World::new();
    let e = world.entity().set(Position { x: 1, y: 2 });

    let a = e.get::<Option<&mut Velocity>>().unwrap();
    assert!(a.is_none());
    // A present write guard on a different component still succeeds, proving
    // the absent optional left the lock map clean.
    let w = e.get::<&mut Position>().unwrap();
    assert_eq!(w.x, 1);
    drop((a, w));
}
