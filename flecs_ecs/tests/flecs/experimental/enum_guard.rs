//! Guard access on `#[derive(Component)]` enums (spec §7 net regression).
//!
//! An enum component's data pointer resolves into the enum CONSTANT entity's
//! storage, not the queried entity's. The debug-only revalidation net must
//! cache the identity of the storage it actually points into, so `get` on
//! an enum does not false-fire the "storage moved" panic in debug builds.

use flecs_ecs::core::*;
use flecs_ecs::macros::Component;

#[derive(Component, Default, PartialEq, Debug)]
#[repr(C)]
enum Color {
    #[default]
    Red = 0,
    Green = 1,
    Blue = 2,
}

#[test]
fn get_on_enum_component() {
    let world = World::new();
    let e = world.entity();
    e.add_enum(Color::Blue);

    {
        let c = e.get::<&Color>().unwrap();
        assert_eq!(*c, Color::Blue);
    }

    e.add_enum(Color::Green);
    let c = e.get::<&Color>().unwrap();
    assert_eq!(*c, Color::Green);
}
