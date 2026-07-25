use core::panic::AssertUnwindSafe;
use std::panic::catch_unwind;

use flecs_ecs::core::*;
use flecs_ecs::macros::*;

#[derive(Component)]
struct Foo(i32);

#[derive(Component)]
struct Bar(i32);

#[test]
fn entity_get_panic_releases_safety_scope() {
    let world = World::new();
    let entity = world.entity().set(Foo(1));

    let result = catch_unwind(AssertUnwindSafe(|| {
        entity.get::<&mut Foo>(|_| panic!("expected"));
    }));

    assert!(result.is_err());
    entity.get::<&mut Foo>(|foo| foo.0 += 1);
    entity.get::<&Foo>(|foo| assert_eq!(foo.0, 2));
}

#[test]
fn cached_ref_panic_releases_safety_scope() {
    let world = World::new();
    let entity = world.entity().set(Foo(1));
    let mut cached = entity.cached_ref(Foo::id());

    let result = catch_unwind(AssertUnwindSafe(|| {
        cached.get(|_| panic!("expected"));
    }));

    assert!(result.is_err());
    cached.get(|foo| foo.0 += 1);
    cached.get(|foo| assert_eq!(foo.0, 2));
}

#[test]
fn query_panic_releases_safety_scope() {
    let world = World::new();
    world.entity().set(Foo(1));
    let query = world.new_query::<&mut Foo>();

    let result = catch_unwind(AssertUnwindSafe(|| {
        query.each(|_| panic!("expected"));
    }));

    assert!(result.is_err());
    query.each(|foo| foo.0 += 1);
    query.each(|foo| assert_eq!(foo.0, 2));
}

#[test]
fn partial_tuple_acquisition_rolls_back_prior_keys() {
    let world = World::new();
    let entity = world.entity().set(Foo(1)).set(Bar(1));

    entity.get::<&mut Foo>(|_| {
        let result = catch_unwind(AssertUnwindSafe(|| {
            entity.get::<(&mut Bar, &mut Foo)>(|_| {});
        }));

        assert!(result.is_err());
        entity.get::<&mut Bar>(|bar| bar.0 += 1);
    });

    entity.get::<&Bar>(|bar| assert_eq!(bar.0, 2));
}
