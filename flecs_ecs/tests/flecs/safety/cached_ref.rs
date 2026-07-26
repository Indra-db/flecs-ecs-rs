//! `CachedRef` accesses register in the same mut-alias tracking as
//! `EntityView::get`/`get_mut` and query iteration.

use flecs_ecs::core::*;
use flecs_ecs::experimental::QuerySharedExt;
use flecs_ecs::experimental::prelude::{EntityGuardExt, WorldEntityRefExt};
use flecs_ecs::macros::*;

#[derive(Component)]
struct Pos(i32);

#[derive(Component)]
struct Vel(i32);

#[test]
#[should_panic(expected = "Cannot set write")]
fn nested_ref_ref_same_component_panics() {
    let world = World::new();
    let e = world.entity().set(Pos(1));

    let mut r1 = world.entity_ref::<Pos>(e.id()).unwrap();
    let mut r2 = world.entity_ref::<Pos>(e.id()).unwrap();

    let _outer = r1.get_mut(&world).unwrap();
    let _inner = r2.get_mut(&world).unwrap();
}

#[test]
#[should_panic(expected = "Cannot set write")]
fn ref_nested_in_entity_get_panics() {
    let world = World::new();
    let e = world.entity().set(Pos(1));

    let mut r = world.entity_ref::<Pos>(e.id()).unwrap();

    let _outer = e.get::<&mut Pos>().unwrap();
    let _inner = r.get_mut(&world).unwrap();
}

#[test]
#[should_panic(expected = "Cannot set write")]
fn ref_nested_in_query_iteration_panics() {
    let world = World::new();
    let e = world.entity().set(Pos(1));
    world.entity().set(Pos(2));

    let mut r = world.entity_ref::<Pos>(e.id()).unwrap();

    let q = world.new_query::<&mut Pos>();
    q.each_shared(&world, |_pos| {
        r.get_mut(&world).unwrap();
    });
}

#[test]
fn refs_to_different_components_do_not_conflict() {
    let world = World::new();
    let e = world.entity().set(Pos(1)).set(Vel(2));

    let mut rp = world.entity_ref::<Pos>(e.id()).unwrap();
    let mut rv = world.entity_ref::<Vel>(e.id()).unwrap();

    {
        let mut pos = rp.get_mut(&world).unwrap();
        let vel = rv.get_mut(&world).unwrap();
        pos.0 += vel.0;
    }
    let pos = rp.get(&world).unwrap();
    assert_eq!(pos.0, 3);
}

#[test]
fn ref_key_refreshes_after_archetype_move() {
    let world = World::new();
    let e = world.entity().set(Pos(1));

    let mut r = world.entity_ref::<Pos>(e.id()).unwrap();
    {
        let mut pos = r.get_mut(&world).unwrap();
        pos.0 += 1;
    }

    // move the entity to another table; the cached lock key must refresh so
    // conflicts are still detected against the new storage
    e.set(Vel(0));

    {
        let mut pos = r.get_mut(&world).unwrap();
        pos.0 += 1;
    }
    {
        let pos = r.get(&world).unwrap();
        assert_eq!(pos.0, 3);
    }

    let result = std::panic::catch_unwind(core::panic::AssertUnwindSafe(|| {
        let mut r2 = world.entity_ref::<Pos>(e.id()).unwrap();
        let _outer = r.get_mut(&world).unwrap();
        let _inner = r2.get_mut(&world).unwrap();
    }));
    assert!(
        result.is_err(),
        "conflict on the post-move table must still be detected"
    );
}

#[test]
fn structural_change_in_ref_callback_is_deferred() {
    let world = World::new();
    let e = world.entity().set(Pos(1));
    let mut r = world.entity_ref::<Pos>(e.id()).unwrap();

    // spawning entities into Pos tables while the guard is live must not
    // invalidate the borrowed component (ops are deferred to guard drop)
    {
        let mut pos = r.get_mut(&world).unwrap();
        for _ in 0..64 {
            world.entity().set(Pos(9));
        }
        pos.0 += 1;
    }
    {
        let pos = r.get(&world).unwrap();
        assert_eq!(pos.0, 2);
    }
}
