//! Debug-only structural safety net for the shared-register guards (spec §7).
//!
//! While a guard ([`Ref`]/[`Mut`]) is live, every structural mutation issued
//! through the safe API defers behind the pin, so the guarded component pointer
//! is stable until the last guard drops. A bypass path that reaches flecs
//! without routing through the write-episode hook applies its change
//! immediately and can move the pinned storage. In debug builds the guard's
//! `Drop` re-resolves the storage and panics if it moved, turning a silent
//! use-after-free into a loud failure. These tests prove (a) the net never
//! fires on the legitimate deferred path and (b) it fires on a deliberately
//! constructed bypass reached through a raw `sys::` call (not the safe API).

use super::{Position, Velocity};
use flecs_ecs::core::*;
use flecs_ecs::macros::Component;

#[derive(Component, Default)]
struct Tag;

// --- category (a)/(b): the net stays silent across legitimate deferred ops ---

#[test]
fn net_silent_across_legit_structural_ops_under_guards() {
    let world = World::new();
    let e = world
        .entity()
        .set(Position { x: 1, y: 2 })
        .set(Velocity { x: 3, y: 4 });
    // A sibling sharing `e`'s archetype ({Position, Velocity}); a structural op
    // on it can reallocate the shared columns `e`'s guards point into, so a
    // bypass there would move the pinned storage just as one on `e` would.
    let sibling = world
        .entity()
        .set(Position { x: 9, y: 9 })
        .set(Velocity { x: 9, y: 9 });
    {
        // Two live guards on `e`'s dense storage.
        let p = e.get::<&Position>().unwrap();
        let v = e.get::<&mut Velocity>().unwrap();
        // A spread of safe structural ops, all deferred behind the pins: adds,
        // removes, entity creation into the guarded archetype, and destruct of
        // the sibling that shares its table. The guarded pointers stay valid,
        // so the net (which runs on each guard's drop) must not fire.
        e.add(Tag::id());
        world.entity().set(Position { x: 7, y: 7 }).set(Velocity { x: 7, y: 7 });
        sibling.remove(Velocity::id());
        sibling.destruct();
        assert_eq!(p.x, 1, "read guard pointer stable across the deferred ops");
        assert_eq!(v.x, 3, "write guard pointer stable across the deferred ops");
        // Dropping both guards here runs the net; it must not fire.
    }
    // The deferred ops landed once the last guard dropped.
    assert!(e.has(Tag::id()));
    assert_eq!(e.get::<&Position>().unwrap().x, 1);
}

#[test]
fn net_silent_for_cached_ref_across_deferred_write() {
    let world = World::new();
    let e = world.entity().set(Position { x: 1, y: 2 });
    let mut cached = world.entity_ref::<Position>(e).unwrap();
    {
        let g = cached.get_mut(&world).unwrap();
        // Deferred behind the pin; the cached pointer stays valid.
        e.add(Tag::id());
        let _ = g.x;
    }
    assert!(e.has(Tag::id()));
    assert_eq!(cached.get(&world).unwrap().x, 1);
}

// --- the net fires on a deliberate bypass reached through a raw sys:: call ---
//
// These reproduce the hazard the net guards against by moving/removing the
// pinned storage with a raw structural call that never passes the write-episode
// hook (the safe API would defer it). Only meaningful in debug builds, where the
// net is compiled in; in release it is compiled out entirely.

#[cfg(debug_assertions)]
#[test]
#[should_panic(expected = "component storage moved")]
fn net_fires_when_raw_add_moves_pinned_storage() {
    let world = World::new();
    let e = world.entity().set(Position { x: 1, y: 2 });
    // A plain tag entity used as a raw component id; registered before the guard.
    let marker = world.entity();

    let g = e.get::<&Position>().unwrap();
    // Bypass: raw add executes immediately (a read guard opens no defer level),
    // moving `e` to a new table and relocating its Position column. The safe
    // `e.add(..)` would have deferred behind the pin.
    unsafe {
        flecs_ecs::sys::ecs_add_id(world.ptr_mut(), *e.id(), **marker);
    }
    // Dropping the guard runs the net, which observes the moved storage.
    drop(g);
}

#[cfg(debug_assertions)]
#[test]
#[should_panic(expected = "component storage moved")]
fn net_fires_when_raw_remove_frees_pinned_storage() {
    let world = World::new();
    let e = world
        .entity()
        .set(Position { x: 1, y: 2 })
        .set(Velocity { x: 3, y: 4 });
    let pos_id = *world.component::<Position>().id();

    let g = e.get::<&Position>().unwrap();
    // Bypass: raw remove of the guarded component executes immediately; the
    // fresh re-resolve finds no Position, a move the net must catch.
    unsafe {
        flecs_ecs::sys::ecs_remove_id(world.ptr_mut(), *e.id(), pos_id);
    }
    drop(g);
}
