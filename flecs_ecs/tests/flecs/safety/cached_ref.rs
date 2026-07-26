//! `CachedRef` accesses register in the same mut-alias tracking as
//! `EntityView::get` and query iteration.

use flecs_ecs::core::*;
use flecs_ecs::macros::*;
use flecs_ecs::experimental::QuerySharedExt;

#[derive(Component)]
struct Pos(i32);

#[derive(Component)]
struct Vel(i32);

#[test]
#[should_panic(expected = "Cannot set write")]
fn nested_ref_ref_same_component_panics() {
    let world = World::new();
    let e = world.entity().set(Pos(1));

    let mut r1 = e.cached_ref(Pos::id());
    let mut r2 = e.cached_ref(Pos::id());

    r1.get(|_outer| {
        r2.get(|_inner| {});
    });
}

#[test]
#[should_panic(expected = "Cannot set write")]
fn ref_nested_in_entity_get_panics() {
    let world = World::new();
    let e = world.entity().set(Pos(1));

    let mut r = e.cached_ref(Pos::id());

    e.get::<&mut Pos>(|_outer| {
        r.get(|_inner| {});
    });
}

#[test]
#[should_panic(expected = "Cannot set write")]
fn ref_nested_in_query_iteration_panics() {
    let world = World::new();
    let e = world.entity().set(Pos(1));
    world.entity().set(Pos(2));

    let mut r = e.cached_ref(Pos::id());

    let q = world.new_query::<&mut Pos>();
    q.each_shared(&world, |_pos| {
        r.get(|_inner| {});
    });
}

#[test]
fn refs_to_different_components_do_not_conflict() {
    let world = World::new();
    let e = world.entity().set(Pos(1)).set(Vel(2));

    let mut rp = e.cached_ref(Pos::id());
    let mut rv = e.cached_ref(Vel::id());

    rp.get(|pos| {
        rv.get(|vel| {
            pos.0 += vel.0;
        });
    });
    rp.get(|pos| assert_eq!(pos.0, 3));
}

#[test]
fn ref_key_refreshes_after_archetype_move() {
    let world = World::new();
    let e = world.entity().set(Pos(1));

    let mut r = e.cached_ref(Pos::id());
    r.get(|pos| pos.0 += 1);

    // move the entity to another table; the cached lock key must refresh so
    // conflicts are still detected against the new storage
    e.set(Vel(0));

    r.get(|pos| pos.0 += 1);
    r.get(|pos| assert_eq!(pos.0, 3));

    let result = std::panic::catch_unwind(core::panic::AssertUnwindSafe(|| {
        let mut r2 = e.cached_ref(Pos::id());
        r.get(|_outer| {
            r2.get(|_inner| {});
        });
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
    let mut r = e.cached_ref(Pos::id());

    // spawning entities into Pos tables inside the callback must not
    // invalidate the borrowed component (ops are deferred to scope end)
    r.get(|pos| {
        for _ in 0..64 {
            world.entity().set(Pos(9));
        }
        pos.0 += 1;
    });
    r.get(|pos| assert_eq!(pos.0, 2));
}
