//! Closing the pin-bypass hazard on paths outside the core 22 write wrappers
//! (spec §3.6, §7.1). These entry points reach flecs `ecs_*` structural
//! operations on an arbitrary caller-supplied entity, so under a live guard they
//! must either defer behind the pin (doc / json setters, routed through
//! `ensure_write_episode`) or refuse (bulk creation, which cannot defer).

use super::Position;
use flecs_ecs::core::*;
use flecs_ecs::experimental::prelude::*;

// --- doc setters defer behind a live guard (routed) ---

#[cfg(feature = "flecs_doc")]
#[test]
fn set_doc_name_under_live_guard_defers_until_drop() {
    let world = World::new();
    let e = world.entity().set(Position { x: 1, y: 2 });
    {
        // A read guard on Position would dangle if adding EcsDocDescription moved
        // the entity's table; the routed deferral keeps its pointer valid.
        let g = e.get_ref::<&Position>().unwrap();
        world.set_doc_name(e, "hero");
        assert!(
            world.is_deferred(),
            "the doc setter opened the write episode behind the live guard"
        );
        assert!(
            world.doc_name(e).is_none(),
            "deferred doc set is not visible under the live guard"
        );
        assert_eq!(g.x, 1, "guard deref stays valid across the deferred doc set");
    }
    assert!(!world.is_deferred(), "episode closed at last-guard-drop");
    assert_eq!(world.doc_name(e).as_deref(), Some("hero"));
}

// --- json set defers behind a live guard (routed) ---

#[cfg(all(feature = "flecs_json", feature = "flecs_meta"))]
#[test]
fn set_json_under_live_guard_defers_until_drop() {
    use super::Velocity;
    use core::mem::offset_of;
    use flecs_ecs::addons::meta::Count;
    let world = World::new();
    // Reflection so `set_json` can deserialize Velocity (registered up front,
    // before any guard is live).
    world
        .component::<Velocity>()
        .member(i32::id(), ("x", Count(0), offset_of!(Velocity, x)))
        .member(i32::id(), ("y", Count(0), offset_of!(Velocity, y)));
    let e = world.entity().set(Position { x: 1, y: 2 });
    {
        // A read guard on Position dangles if adding Velocity moves the entity's
        // table; the routed deferral keeps its pointer valid.
        let g = e.get_ref::<&Position>().unwrap();
        // `ecs_ensure_id(Velocity)` would move `e` to a new table; routed so it
        // defers behind the pin instead of relocating the pinned Position column.
        e.set_json(Velocity::id(), r#"{"x":5,"y":6}"#, None);
        assert!(
            world.is_deferred(),
            "the json setter opened the write episode behind the live guard"
        );
        assert!(
            !e.has(Velocity::id()),
            "deferred json add is not visible under the live guard"
        );
        assert_eq!(g.x, 1, "guard deref stays valid across the deferred json add");
    }
    assert!(!world.is_deferred());
    assert!(e.has(Velocity::id()));
    assert_eq!(e.get_ref::<&Velocity>().unwrap().x, 5);
}

// --- bulk creation refuses under a live guard (cannot defer) ---

#[test]
#[should_panic(expected = "component guards are live")]
fn entity_bulk_build_refuses_under_live_guard() {
    let world = World::new();
    let e = world.entity().set(Position { x: 1, y: 2 });
    let _g = e.get_ref::<&Position>().unwrap();
    // `ecs_bulk_init` appends rows to existing tables and must observe its ids
    // immediately, so it cannot defer behind the pin: it refuses.
    let positions: Vec<Position> = (0..4).map(|i| Position { x: i, y: i }).collect();
    let _ = world.entity_bulk(4).set(&positions).build();
}

// A live guard across `progress` / `run_pipeline` is no longer a runtime refusal
// but a compile error: `progress` takes `&mut self` (spec §5.7) and a
// shared-register guard borrows `&World`, so the two cannot coexist. The
// borrow-check refusal is pinned by a `compile_fail` doctest on
// `World::progress`. The old runtime `should_panic` test is therefore gone.

// --- meta unit/quantity setters defer behind a live guard (routed) ---

#[cfg(feature = "flecs_meta")]
#[test]
fn quantity_self_under_live_guard_defers() {
    let world = World::new();
    let e = world.entity().set(Position { x: 1, y: 2 });
    {
        let g = e.get_ref::<&Position>().unwrap();
        // Adds `Quantity` to `e`, moving it; routed so it defers behind the pin.
        e.quantity_self();
        assert!(
            world.is_deferred(),
            "the meta setter opened the write episode behind the live guard"
        );
        assert_eq!(g.x, 1, "guard deref stays valid across the deferred meta add");
    }
    assert!(!world.is_deferred(), "episode closed at last-guard-drop");
}

#[test]
fn entity_bulk_build_allowed_with_no_live_guard() {
    let world = World::new();
    let e = world.entity().set(Position { x: 1, y: 2 });
    {
        let g = e.get_ref::<&Position>().unwrap();
        assert_eq!(g.x, 1);
        // guard dropped here
    }
    let positions: Vec<Position> = (0..4).map(|i| Position { x: i, y: i }).collect();
    let ids = world.entity_bulk(4).set(&positions).build();
    assert_eq!(ids.len(), 4);
}
