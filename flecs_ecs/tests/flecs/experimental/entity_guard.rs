//! Deliverable 1: guard-based entity access (shared register).

use super::{Health, Position, Velocity};
use flecs_ecs::core::*;
use flecs_ecs::macros::query;

#[test]
fn get_read() {
    let world = World::new();
    let e = world.entity().set(Position { x: 1, y: 2 });
    let p = e.get::<&Position>().unwrap();
    assert_eq!(p.x, 1);
    assert_eq!(p.y, 2);
}

#[test]
fn get_write_mutates() {
    let world = World::new();
    let e = world.entity().set(Position { x: 1, y: 2 });
    {
        let mut p = e.get::<&mut Position>().unwrap();
        p.x += 10;
    }
    let p = e.get::<&Position>().unwrap();
    assert_eq!(p.x, 11);
}

#[test]
fn get_tuple_fused() {
    let world = World::new();
    let e = world
        .entity()
        .set(Position { x: 1, y: 2 })
        .set(Velocity { x: 3, y: 4 });
    let (mut p, v) = e.get::<(&mut Position, &Velocity)>().unwrap();
    p.x += v.x;
    p.y += v.y;
    drop((p, v));
    let p = e.get::<&Position>().unwrap();
    assert_eq!((p.x, p.y), (4, 6));
}

#[test]
fn get_missing_is_none() {
    let world = World::new();
    let e = world.entity().set(Position { x: 1, y: 2 });
    assert!(e.get::<&Velocity>().is_none());
}

#[test]
fn get_dead_entity_is_none() {
    let world = World::new();
    let e = world.entity().set(Position { x: 1, y: 2 });
    e.destruct();
    assert!(e.get::<&Position>().is_none());
}

// --- conflict, guard vs guard ---

#[test]
#[should_panic]
fn get_write_write_panics() {
    let world = World::new();
    let e = world.entity().set(Position::default());
    let _a = e.get::<&mut Position>().unwrap();
    let _b = e.get::<&mut Position>().unwrap();
}

#[test]
#[should_panic]
fn get_read_write_panics() {
    let world = World::new();
    let e = world.entity().set(Position::default());
    let _a = e.get::<&Position>().unwrap();
    let _b = e.get::<&mut Position>().unwrap();
}

#[test]
#[should_panic]
fn get_write_read_panics() {
    let world = World::new();
    let e = world.entity().set(Position::default());
    let _a = e.get::<&mut Position>().unwrap();
    let _b = e.get::<&Position>().unwrap();
}

#[test]
fn get_read_read_ok() {
    let world = World::new();
    let e = world.entity().set(Position { x: 5, y: 6 });
    let a = e.get::<&Position>().unwrap();
    let b = e.get::<&Position>().unwrap();
    assert_eq!(a.x, b.x);
}

// --- try_get returns the conflict as a value, both directions ---

#[test]
fn try_get_read_then_write_is_conflict() {
    let world = World::new();
    let e = world.entity().set(Position::default());
    let _a = e.get::<&Position>().unwrap();
    match e.try_get::<&mut Position>().err() {
        Some(AccessError::Conflict { write, .. }) => assert!(write),
        other => panic!("expected write conflict, got {other:?}"),
    }
}

#[test]
fn try_get_write_then_read_is_conflict() {
    let world = World::new();
    let e = world.entity().set(Position::default());
    let _a = e.get::<&mut Position>().unwrap();
    match e.try_get::<&Position>().err() {
        Some(AccessError::Conflict { write, .. }) => assert!(!write),
        other => panic!("expected read conflict, got {other:?}"),
    }
}

#[test]
fn try_get_missing_component() {
    let world = World::new();
    let e = world.entity().set(Position::default());
    assert_eq!(
        e.try_get::<&Velocity>().err(),
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
            let _g = e.get::<&Position>().unwrap();
        });
}

#[test]
#[should_panic]
fn query_read_then_guard_write_panics() {
    let world = World::new();
    world.entity().set(Position::default());
    query!(world, &Position).build().each_entity(|e, _| {
        let _g = e.get::<&mut Position>().unwrap();
    });
}

#[test]
fn query_read_then_guard_read_ok() {
    let world = World::new();
    world.entity().set(Position { x: 7, y: 8 });
    let mut seen = 0;
    query!(world, &Position).build().each_entity(|e, _| {
        let g = e.get::<&Position>().unwrap();
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
    e.get::<&mut Health>().unwrap().0 += 1;
    e.get::<&mut Health>().unwrap().0 += 1;
    assert_eq!(e.get::<&Health>().unwrap().0, 2);
}

#[test]
fn explicit_drop_then_reacquire() {
    let world = World::new();
    let e = world.entity().set(Health(0));
    let mut g = e.get::<&mut Health>().unwrap();
    g.0 = 41;
    drop(g);
    let mut g2 = e.get::<&mut Health>().unwrap();
    g2.0 += 1;
    drop(g2);
    assert_eq!(e.get::<&Health>().unwrap().0, 42);
}

// --- cloned: no guard held after return ---

#[test]
fn cloned_releases_lock() {
    let world = World::new();
    let e = world.entity().set(Position { x: 9, y: 9 });
    let owned = e.cloned::<&Position>().unwrap();
    assert_eq!(owned.x, 9);
    // No guard is held, so a fresh mutable borrow must succeed.
    let mut p = e.get::<&mut Position>().unwrap();
    p.x = 0;
}

// --- optional guard elements (spec 3.4) ---

#[test]
fn optional_present_yields_some() {
    let world = World::new();
    let e = world
        .entity()
        .set(Position { x: 1, y: 2 })
        .set(Velocity { x: 3, y: 4 });
    let (p, v) = e.get::<(&Position, Option<&Velocity>)>().unwrap();
    assert_eq!(p.x, 1);
    let v = v.expect("velocity present");
    assert_eq!(v.x, 3);
}

#[test]
fn optional_absent_yields_none_in_successful_tuple() {
    let world = World::new();
    let e = world.entity().set(Position { x: 1, y: 2 });
    let (p, v) = e.get::<(&Position, Option<&Velocity>)>().unwrap();
    assert_eq!((p.x, p.y), (1, 2));
    assert!(v.is_none());
}

#[test]
fn optional_present_conflicts_like_required() {
    let world = World::new();
    let e = world
        .entity()
        .set(Position::default())
        .set(Velocity::default());
    let _w = e.get::<&mut Velocity>().unwrap();
    // A present optional attempts a borrow, so it conflicts with the live write.
    match e.try_get::<(&Position, Option<&Velocity>)>().err() {
        Some(AccessError::Conflict { write, .. }) => assert!(!write),
        other => panic!("expected read conflict on present optional, got {other:?}"),
    }
}

#[test]
fn optional_absent_does_not_conflict_with_live_write() {
    let world = World::new();
    let e = world.entity().set(Position::default());
    // A live write guard on Position; the absent optional Velocity must not
    // conflict, and the required &Position read is not requested, so no clash.
    let _w = e.get::<&mut Position>().unwrap();
    let (v, h) = e
        .get::<(Option<&Velocity>, Option<&Health>)>()
        .unwrap();
    assert!(v.is_none());
    assert!(h.is_none());
}

#[test]
fn all_optional_all_absent_holds_nothing() {
    let world = World::new();
    let e = world.entity().set(Position { x: 1, y: 2 });
    {
        let (v, h) = e
            .get::<(Option<&Velocity>, Option<&mut Health>)>()
            .unwrap();
        assert!(v.is_none());
        assert!(h.is_none());
        // The all-absent optional acquire took no lock: a conflicting write
        // guard on Position (present) still succeeds while it is live, proving
        // the stage map holds nothing from the optional acquire.
        let mut p = e.get::<&mut Position>().unwrap();
        p.x = 9;
    }
    assert_eq!(e.get::<&Position>().unwrap().x, 9);
}

#[test]
fn mixed_tuple_rollback_releases_only_what_was_taken() {
    let world = World::new();
    let e = world
        .entity()
        .set(Position::default())
        .set(Velocity::default());
    // Hold a live write on Velocity so the second (optional) element conflicts
    // after the first (Position) borrow is taken; rollback must release the
    // Position borrow and leave nothing registered from this acquire.
    {
        let _w = e.get::<&mut Velocity>().unwrap();
        assert!(matches!(
            e.try_get::<(&Position, Option<&mut Velocity>)>().err(),
            Some(AccessError::Conflict { write: true, .. })
        ));
        // Position was rolled back: it must be re-acquirable while _w lives.
        let _p = e.get::<&Position>().unwrap();
    }
    // Everything released: a full mutable acquire of both must succeed.
    let (mut p, mut v) = e.get::<(&mut Position, &mut Velocity)>().unwrap();
    p.x = 1;
    v.x = 2;
}

#[test]
fn optional_absent_missing_required_is_none() {
    let world = World::new();
    let e = world.entity().set(Velocity::default());
    // Required Position missing gates the whole acquire even with an optional.
    assert!(e.get::<(&Position, Option<&Velocity>)>().is_none());
}

#[test]
fn panic_with_live_optional_guard_does_not_wedge() {
    let world = World::new();
    let e = world
        .entity()
        .set(Position { x: 1, y: 1 })
        .set(Velocity { x: 2, y: 2 });

    let result = std::panic::catch_unwind(core::panic::AssertUnwindSafe(|| {
        let _g = e
            .get::<(&mut Position, Option<&mut Velocity>)>()
            .unwrap();
        panic!("boom while a mixed optional guard is live");
    }));
    assert!(result.is_err());

    // Both present borrows released on unwind; a fresh mutable acquire works.
    let (mut p, v) = e.get::<(&mut Position, Option<&Velocity>)>().unwrap();
    p.x = 100;
    assert!(v.is_some());
    drop((p, v));
    assert_eq!(e.get::<&Position>().unwrap().x, 100);
}

// --- guard ergonomics: Debug / Display / PartialEq forward to T (spec 3.2) ---

#[test]
fn guard_partial_eq_forwards_to_target() {
    let world = World::new();
    let a = world.entity().set(Health(5));
    let b = world.entity().set(Health(5));
    let c = world.entity().set(Health(6));
    let ga = a.get::<&Health>().unwrap();
    let gb = b.get::<&Health>().unwrap();
    let gc = c.get::<&Health>().unwrap();
    assert_eq!(ga, gb);
    assert_ne!(ga, gc);
}

#[test]
fn guard_mut_partial_eq_forwards_to_target() {
    let world = World::new();
    // Distinct archetypes so the two Health columns are separate storage and the
    // two write guards do not conflict.
    let a = world.entity().set(Health(5));
    let b = world.entity().set(Health(5)).set(Position::default());
    let ga = a.get::<&mut Health>().unwrap();
    let gb = b.get::<&mut Health>().unwrap();
    assert_eq!(ga, gb);
}

#[test]
fn guard_debug_forwards_to_target() {
    let world = World::new();
    let e = world.entity().set(Position { x: 3, y: 4 });
    let g = e.get::<&Position>().unwrap();
    assert_eq!(format!("{g:?}"), format!("{:?}", Position { x: 3, y: 4 }));
    drop(g);
    let gm = e.get::<&mut Position>().unwrap();
    assert_eq!(format!("{gm:?}"), format!("{:?}", Position { x: 3, y: 4 }));
}

#[derive(flecs_ecs::macros::Component)]
struct Meters(i32);

impl core::fmt::Display for Meters {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{}m", self.0)
    }
}

#[test]
fn guard_display_forwards_to_target() {
    let world = World::new();
    let e = world.entity().set(Meters(42));
    {
        let g = e.get::<&Meters>().unwrap();
        assert_eq!(format!("{g}"), "42m");
    }
    let gm = e.get::<&mut Meters>().unwrap();
    assert_eq!(format!("{gm}"), "42m");
}

// --- panic safety: a guard live during a panic must not wedge the map ---

#[test]
fn guard_live_during_panic_does_not_wedge() {
    let world = World::new();
    let e = world.entity().set(Position { x: 1, y: 1 });

    let result = std::panic::catch_unwind(core::panic::AssertUnwindSafe(|| {
        let _g = e.get::<&mut Position>().unwrap();
        panic!("boom while a guard is live");
    }));
    assert!(result.is_err());

    // The unwound guard released its borrow, so the map is usable again.
    let mut p = e.get::<&mut Position>().unwrap();
    p.x = 100;
    drop(p);
    assert_eq!(e.get::<&Position>().unwrap().x, 100);
}
