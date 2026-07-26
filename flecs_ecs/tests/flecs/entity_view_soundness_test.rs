use crate::common_test::*;
use flecs_ecs::experimental::prelude::EntityGuardExt;

#[repr(u8)]
#[derive(Component, Debug, PartialEq)]
enum ReprU8Enum {
    A,
    B,
    C,
}

#[test]
fn entity_symbol_without_symbol_returns_empty_string() {
    let world = World::new();

    let unnamed = world.entity();
    assert_eq!(unnamed.symbol(), "");

    let named = world.entity_named("bob");
    assert_eq!(named.symbol(), "");
}

#[test]
fn entity_to_constant_non_i32_repr_enum() {
    let world = World::new();

    let e_a = world.entity_from_enum(ReprU8Enum::A);
    let e_b = world.entity_from_enum(ReprU8Enum::B);
    let e_c = world.entity_from_enum(ReprU8Enum::C);

    assert_eq!(e_a.to_constant::<ReprU8Enum>(), ReprU8Enum::A);
    assert_eq!(e_b.to_constant::<ReprU8Enum>(), ReprU8Enum::B);
    assert_eq!(e_c.to_constant::<ReprU8Enum>(), ReprU8Enum::C);
}

#[test]
#[should_panic(expected = "is not a constant of enum")]
fn entity_to_constant_on_non_constant_entity_panics() {
    let world = World::new();
    world.component::<ReprU8Enum>();

    let e = world.entity();
    let _ = e.to_constant::<ReprU8Enum>();
}

#[test]
fn entity_try_get_on_deleted_entity_returns_none() {
    let world = World::new();

    let e = world.entity().set(Position { x: 1, y: 2 });
    e.destruct();

    assert!(e.get::<&Position>().is_none());
    assert!(e.cloned::<&Position>().is_none());
}

#[test]
fn entity_get_on_deleted_entity_returns_none() {
    let world = World::new();

    let e = world.entity().set(Position { x: 1, y: 2 });
    e.destruct();

    assert!(e.get::<&Position>().is_none());
}

#[test]
fn entity_cloned_on_deleted_entity_returns_none() {
    let world = World::new();

    let e = world.entity().set(Position { x: 1, y: 2 });
    e.destruct();

    assert!(e.cloned::<&Position>().is_none());
}

#[test]
fn entity_range_on_deleted_entity_returns_none() {
    let world = World::new();

    let e = world.entity().set(Position { x: 1, y: 2 });
    e.destruct();

    assert!(e.range().is_none());
}
