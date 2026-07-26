//! Pin-counter + lazy-episode semantics of the shared-register guards
//! (spec §3.6, §7.1, §7.2). A live guard increments a per-stage pin and opens
//! **no** flecs defer level; the first shared-register write while the pin is
//! nonzero lazily opens one episode-scoped level that closes when the last
//! guard drops. Writes issued under a live guard therefore defer and apply
//! (observers included) at last-guard-drop.

use super::{Position, Velocity};
use core::sync::atomic::{AtomicU32, Ordering};
use flecs_ecs::core::*;
use flecs_ecs::experimental::prelude::*;
use flecs_ecs::macros::Component;

#[derive(Component, Default)]
struct Tag;

// --- read-only guards open no defer level (spec §7.1) ---

#[test]
fn read_only_guard_scope_leaves_world_not_deferred() {
    let world = World::new();
    let e = world
        .entity()
        .set(Position { x: 1, y: 2 })
        .set(Velocity { x: 3, y: 4 });
    assert!(!world.is_deferred());
    {
        let g = e.get_ref::<&Position>().unwrap();
        assert!(
            !world.is_deferred(),
            "a read-only guard opens no defer level"
        );
        assert_eq!(g.x, 1);
        let _g2 = e.get_ref::<&Velocity>().unwrap();
        assert!(
            !world.is_deferred(),
            "a second read-only guard still opens no level"
        );
    }
    assert!(!world.is_deferred());
}

// --- mid-acquire panic / conflict leaves no stale pin (spec §7.1 item 5) ---

#[test]
fn mid_acquire_panic_leaves_no_stale_pin() {
    let world = World::new();
    let e = world
        .entity()
        .set(Position { x: 1, y: 1 })
        .set(Velocity { x: 2, y: 2 });
    // A duplicate-mutable request panics inside `create_ptrs`, before any pin is
    // taken. The pin state must be untouched afterwards.
    let r = std::panic::catch_unwind(core::panic::AssertUnwindSafe(|| {
        let _g = e.get_ref::<(&mut Position, &mut Position)>();
    }));
    assert!(r.is_err());
    assert!(!world.is_deferred(), "no stale pin/episode after the panic");
    // The pin machinery still counts correctly: a write under a fresh guard
    // defers and flushes at drop.
    {
        let g = e.get_ref::<&Velocity>().unwrap();
        e.set(Position { x: 9, y: 9 });
        assert!(
            world.is_deferred(),
            "a write under a live guard opens the episode"
        );
        let _ = g.x;
    }
    assert!(!world.is_deferred(), "episode closed at last-guard-drop");
    assert_eq!(e.get_ref::<&Position>().unwrap().x, 9);
}

#[test]
fn conflict_mid_tuple_rolls_back_pin() {
    let world = World::new();
    let e = world
        .entity()
        .set(Position { x: 1, y: 1 })
        .set(Velocity { x: 2, y: 2 });
    let w = e.get_ref::<&mut Velocity>().unwrap();
    // Position's borrow + pin are taken, then Velocity conflicts and the whole
    // acquire rolls back: the Position pin must be released too, so the only
    // live pin is `w`.
    assert!(e.try_get_ref::<(&Position, &mut Velocity)>().is_err());
    // A write now defers behind exactly one pin (`w`)...
    e.set(Position { x: 5, y: 5 });
    assert!(world.is_deferred());
    drop(w);
    // ...and dropping that one pin flushes it. If the rolled-back Position pin
    // had stuck, the episode would still be open here.
    assert!(!world.is_deferred(), "the rolled-back pin did not leak");
    assert_eq!(e.get_ref::<&Position>().unwrap().x, 5);
}

// --- one write category per test: deferred under a live guard, applied and
//     observers fired at last-guard-drop, guard deref valid throughout ---

#[test]
fn set_under_live_guard_defers_until_drop() {
    static CNT: AtomicU32 = AtomicU32::new(0);
    CNT.store(0, Ordering::Relaxed);
    let world = World::new();
    world
        .observer::<flecs::OnSet, &Velocity>()
        .each_entity(|_e, _v| {
            CNT.fetch_add(1, Ordering::Relaxed);
        });
    let e = world.entity().set(Position { x: 1, y: 2 });
    {
        let g = e.get_ref::<&Position>().unwrap();
        e.set(Velocity { x: 7, y: 8 });
        assert!(
            !e.has(Velocity::id()),
            "deferred set is not visible under a live guard"
        );
        assert_eq!(CNT.load(Ordering::Relaxed), 0, "observer waits for drop");
        assert_eq!(g.x, 1, "guard deref stays valid across the deferred set");
    }
    assert!(e.has(Velocity::id()));
    assert_eq!(CNT.load(Ordering::Relaxed), 1);
    assert_eq!(e.get_ref::<&Velocity>().unwrap().x, 7);
}

#[test]
fn add_under_live_guard_defers_until_drop() {
    let world = World::new();
    let e = world.entity().set(Position { x: 1, y: 2 });
    {
        let g = e.get_ref::<&Position>().unwrap();
        e.add(Tag::id());
        assert!(
            !e.has(Tag::id()),
            "deferred add is not visible under a live guard"
        );
        assert_eq!(g.x, 1, "guard deref stays valid across the deferred add");
    }
    assert!(e.has(Tag::id()));
}

#[test]
fn remove_under_live_guard_defers_until_drop() {
    static CNT: AtomicU32 = AtomicU32::new(0);
    CNT.store(0, Ordering::Relaxed);
    let world = World::new();
    world
        .observer::<flecs::OnRemove, &Velocity>()
        .each_entity(|_e, _v| {
            CNT.fetch_add(1, Ordering::Relaxed);
        });
    let e = world
        .entity()
        .set(Position { x: 1, y: 2 })
        .set(Velocity { x: 3, y: 4 });
    {
        // The read guard on Position would dangle if removing Velocity moved
        // the entity's table; the deferral keeps its pointer valid.
        let g = e.get_ref::<&Position>().unwrap();
        e.remove(Velocity::id());
        assert!(e.has(Velocity::id()), "deferred remove has not landed");
        assert_eq!(CNT.load(Ordering::Relaxed), 0, "on_remove waits for drop");
        assert_eq!(g.x, 1, "guard deref stays valid across the deferred remove");
    }
    assert!(!e.has(Velocity::id()));
    assert_eq!(CNT.load(Ordering::Relaxed), 1);
}

#[test]
fn destruct_under_live_guard_defers_until_drop() {
    let world = World::new();
    let e = world.entity().set(Position { x: 1, y: 2 });
    {
        let g = e.get_ref::<&Position>().unwrap();
        e.destruct();
        assert!(
            e.is_alive(),
            "deferred destruct has not landed under the guard"
        );
        assert_eq!(g.x, 1, "guard deref stays valid across the deferred destruct");
    }
    assert!(!e.is_alive());
}

#[test]
fn pair_set_under_live_guard_defers_until_drop() {
    let world = World::new();
    let target = world.entity();
    let e = world.entity().set(Velocity { x: 3, y: 4 });
    {
        let g = e.get_ref::<&Velocity>().unwrap();
        // (Position, target) pair carrying Position data.
        e.set_first::<Position>(Position { x: 5, y: 6 }, target);
        assert!(
            !e.has((Position::id(), target)),
            "deferred pair set is not visible under a live guard"
        );
        assert_eq!(g.x, 3, "guard deref stays valid across the deferred pair set");
    }
    assert!(e.has((Position::id(), target)));
}

// --- nested guards across two entities share ONE episode (spec §7.1) ---

#[test]
fn nested_guards_two_entities_share_one_episode() {
    let world = World::new();
    let e1 = world.entity().set(Position { x: 1, y: 1 });
    let e2 = world.entity().set(Position { x: 2, y: 2 });

    let g1 = e1.get_ref::<&Position>().unwrap();
    let g2 = e2.get_ref::<&Position>().unwrap();
    assert!(!world.is_deferred(), "two read guards open no level");

    e1.set(Velocity { x: 10, y: 10 });
    assert!(world.is_deferred(), "first write opens the episode");
    e2.set(Velocity { x: 20, y: 20 });
    assert!(world.is_deferred(), "second write reuses the same episode");
    assert!(!e1.has(Velocity::id()) && !e2.has(Velocity::id()));

    drop(g1);
    assert!(
        world.is_deferred(),
        "episode stays open while another guard is live"
    );
    assert!(
        !e1.has(Velocity::id()),
        "writes do not flush until the last guard drops"
    );

    drop(g2);
    assert!(
        !world.is_deferred(),
        "last guard drop closes the single episode"
    );
    assert!(e1.has(Velocity::id()) && e2.has(Velocity::id()));
}

// Deleted: episode_composes_with_open_legacy_get_scope pinned the interaction
// between the new guard episode and an OPEN legacy closure-`get` defer scope.
// The legacy closure-CPS `EntityViewGet::get` is removed in this flip, so there
// is no legacy scope left to compose with. The standalone episode open/close
// semantics stay covered by the sibling tests in this file.

// --- mem::forget contract (spec §7.2): leak fails safe, never wedges ---

#[test]
fn forgotten_guard_pin_leaks_but_world_destruction_is_safe() {
    static CNT: AtomicU32 = AtomicU32::new(0);
    CNT.store(0, Ordering::Relaxed);
    let world = World::new();
    world
        .observer::<flecs::OnSet, &Velocity>()
        .each_entity(|_e, _v| {
            CNT.fetch_add(1, Ordering::Relaxed);
        });
    let e = world.entity().set(Position { x: 1, y: 1 });

    let g = e.get_ref::<&Position>().unwrap();
    core::mem::forget(g); // read borrow + pin leak; no defer level yet

    // Monotonic access denial: the leaked read borrow refuses a later write
    // borrow of the same storage forever, it never aliases.
    assert!(
        e.try_get_ref::<&mut Position>().is_err(),
        "the leaked read borrow denies a conflicting write borrow"
    );

    // The stuck pin keeps shared-register writes on the deferred path (delay,
    // never a grant): the write opens an episode and is not visible.
    e.set(Velocity { x: 2, y: 2 });
    assert!(world.is_deferred(), "write deferred behind the stuck pin");
    assert!(!e.has(Velocity::id()));
    assert_eq!(CNT.load(Ordering::Relaxed), 0, "observer delayed");

    // World destruction drains the leaked episode: the queued write flushes and
    // its observer fires at teardown, and `ecs_fini` does NOT abort. Reaching
    // past `drop(world)` without a SIGABRT is the load-bearing assertion.
    drop(world);
    assert_eq!(
        CNT.load(Ordering::Relaxed),
        1,
        "leaked write flushed at teardown (delay-until-close)"
    );
}
