#![allow(dead_code)]
use crate::common_test::*;
use flecs_ecs::experimental::prelude::{CachedRef, EntityGuardExt, WorldEntityRefExt};

// Migrated from the legacy `EntityView::cached_ref` closure API to the guard-based
// `World::entity_ref` / `CachedRef::get` / `CachedRef::get_mut` surface (spec §3.7).
//
// The redesigned `CachedRef` is a typed, single-component cache: it resolves the
// component once and hands back `Ref` / `Mut` guards. The following legacy-only
// tests were deleted because they pinned capabilities the narrowed `CachedRef`
// deliberately no longer offers, and have no equivalent to migrate to:
//   - refs_ref_before_set: legacy allowed building a ref before the component
//     existed; `entity_ref` resolves once and returns `None` when absent.
//   - refs_pair_ref, refs_pair_ref_w_pair_type, refs_pair_ref_w_pair_type_second,
//     refs_pair_ref_w_entity, refs_pair_ref_second, refs_untyped_pair_ref:
//     pair-id refs; `entity_ref::<T>` takes a single typed data component.
//   - refs_base_type, refs_empty_base_type: a ref typed as one component but
//     backed by a different component's storage (C++ base-class aliasing).
//   - refs_untyped_get_ref_by_method, refs_untyped_runtime_component_ref: untyped
//     (`c_void`) refs; the typed cache has no untyped form.
//   - refs_get_component (`.component()`), refs_ref_world (`.world()`): accessors
//     removed from the typed cache.

// ─── Basic ref access ────────────────────────────────────────────────────────

#[test]
fn refs_get_ref_by_ptr() {
    let world = World::new();

    let e = world.entity().set(Position { x: 10, y: 20 });

    let r = world.entity_ref::<Position>(e.id()).unwrap();
    let pos = r.get(&world).unwrap();
    assert!(pos.x == 10);
    assert!(pos.y == 20);
}

#[test]
fn refs_get_ref_by_method() {
    let world = World::new();

    let e = world.entity().set(Position { x: 10, y: 20 });

    let r = world.entity_ref::<Position>(e.id()).unwrap();
    let pos = r.get(&world).unwrap();
    assert!(pos.x == 10);
    assert!(pos.y == 20);
}

// ─── Ref stability after structural changes ──────────────────────────────────

#[test]
fn refs_ref_after_add() {
    let world = World::new();

    let e = world.entity().set(Position { x: 10, y: 20 });

    let r = world.entity_ref::<Position>(e.id()).unwrap();

    e.add(id::<Velocity>());
    let pos = r.get(&world).unwrap();
    assert!(pos.x == 10);
    assert!(pos.y == 20);
}

#[test]
fn refs_ref_after_remove() {
    let world = World::new();

    let e = world
        .entity()
        .set(Position { x: 10, y: 20 })
        .set(Velocity { x: 1, y: 1 });

    let r = world.entity_ref::<Position>(e.id()).unwrap();

    e.remove(id::<Velocity>());
    let pos = r.get(&world).unwrap();
    assert!(pos.x == 10);
    assert!(pos.y == 20);
}

#[test]
fn refs_ref_after_set() {
    let world = World::new();

    let e = world.entity().set(Position { x: 10, y: 20 });

    let r = world.entity_ref::<Position>(e.id()).unwrap();

    e.set(Velocity { x: 1, y: 1 });
    let pos = r.get(&world).unwrap();
    assert!(pos.x == 10);
    assert!(pos.y == 20);
}

// ─── Mutable ref ─────────────────────────────────────────────────────────────

#[test]
fn refs_non_const_ref() {
    let world = World::new();

    let e = world.entity().set(Position { x: 10, y: 20 });
    let mut r = world.entity_ref::<Position>(e.id()).unwrap();
    {
        let mut pos = r.get_mut(&world).unwrap();
        pos.x += 1;
    }

    let pos = e.get_ref::<&Position>().unwrap();
    assert_eq!(pos.x, 11);
}

// ─── Stage ref ───────────────────────────────────────────────────────────────

#[test]
fn refs_from_stage() {
    let world = World::new();
    // world.stage(0) gives the default stage (mirrors C++ world.get_stage(0)).
    let stage = world.stage(0);
    let e = stage.entity().set(Position { x: 10, y: 20 });
    let r = world.entity_ref::<Position>(e.id()).unwrap();
    let pos = r.get(&world).unwrap();
    assert_eq!(pos.x, 10);
    assert_eq!(pos.y, 20);
}

// ─── Construction from entity ────────────────────────────────────────────────

#[test]
fn refs_default_ctor() {
    let world = World::new();

    let e = world.entity().set(Position { x: 10, y: 20 });

    let p = world.entity_ref::<Position>(e.id()).unwrap();
    let pos = p.get(&world).unwrap();
    assert_eq!(pos.x, 10);
    assert_eq!(pos.y, 20);
}

#[test]
fn refs_ctor_from_entity() {
    let world = World::new();

    let e = world.entity().set(Position { x: 10, y: 20 });

    let p = world.entity_ref::<Position>(e.id()).unwrap();
    let pos = p.get(&world).unwrap();
    assert_eq!(pos.x, 10);
    assert_eq!(pos.y, 20);
}

// ─── bool / has semantics ────────────────────────────────────────────────────

#[test]
fn refs_implicit_operator_bool() {
    let world = World::new();

    let e = world.entity().set(Position { x: 10, y: 20 });

    // `entity_ref` resolves the component once; presence is `Some`.
    assert!(world.entity_ref::<Position>(e.id()).is_some());
}

#[test]
fn refs_try_get() {
    let world = World::new();

    // An entity with no Position set; `entity_ref` returns None.
    let e = world.entity(); // no Position
    assert!(world.entity_ref::<Position>(e.id()).is_none());
}

#[test]
fn refs_try_get_after_delete() {
    let world = World::new();

    let e = world.entity().set(Position { x: 10, y: 20 });

    let p = world.entity_ref::<Position>(e.id()).unwrap();

    // Before delete: the cache resolves.
    {
        let pos = p.get(&world).unwrap();
        assert_eq!(pos.x, 10);
        assert_eq!(pos.y, 20);
    }

    // Delete the entity (destruct consumes e).
    e.destruct();

    // After delete: the same cache returns None.
    assert!(p.get(&world).is_none());
}

#[test]
fn refs_has() {
    let world = World::new();

    let e = world.entity();

    assert!(world.entity_ref::<Position>(e.id()).is_none());

    e.set(Position { x: 10, y: 20 });

    assert!(world.entity_ref::<Position>(e.id()).is_some());
}

#[test]
fn refs_bool_operator() {
    let world = World::new();

    let e = world.entity();

    assert!(world.entity_ref::<Position>(e.id()).is_none());

    e.set(Position { x: 10, y: 20 });

    assert!(world.entity_ref::<Position>(e.id()).is_some());
}

// ─── ref.entity() ────────────────────────────────────────────────────────────

#[test]
fn refs_ref_entity() {
    let world = World::new();

    let e = world.entity().set(Position { x: 10, y: 20 });

    let r = world.entity_ref::<Position>(e.id()).unwrap();
    assert_eq!(r.entity(), e.id());
}

fn _assert_cached_ref_in_scope<T: flecs_ecs::core::ComponentId + flecs_ecs::core::DataComponent>(
    _r: &CachedRef<T>,
) {
}
