use crate::z_ignore_test_common::*;

use flecs_ecs::experimental::prelude::*;
use flecs_ecs::prelude::*;

// This example shows how relationships can be combined with components to attach
// data to a relationship.

// Some demo components:

#[derive(Debug, Component, Default)]
pub struct Position {
    pub x: f32,
    pub y: f32,
}

#[derive(Component)]
pub struct Tag;

#[derive(Component)]
struct Requires {
    amount: f32,
}

#[derive(Component)]
struct Gigawatts;

#[derive(Component)]
struct Expires {
    timeout: f32,
}

#[derive(Component)]
struct MustHave;

fn main() {
    let mut world = World::new();

    // When one element of a pair is a component and the other element is a tag,
    // the pair assumes the type of the component.
    let e1 = world
        .entity()
        .set_pair::<Requires, Gigawatts>(Requires { amount: 1.21 });

    // try_get returns the missing-relationship case as an Err instead of
    // routing it through an Option inside a callback.
    match e1.try_get::<&(Requires, Gigawatts)>() {
        Ok(req) => println!("e1: requires: {}", req.amount),
        Err(_) => println!("e1: does not have a relationship with Requires, Gigawatts"),
    }

    // The component can be either the first or second part of a pair:
    let e2 = world
        .entity()
        .set_pair::<Gigawatts, Requires>(Requires { amount: 1.5 });

    match e2.try_get::<&(Gigawatts, Requires)>() {
        Ok(req) => println!("e2: requires: {}", req.amount),
        Err(_) => println!("e2: does not have a relationship with Gigawatts, Requires"),
    }

    // Note that <Requires, Gigawatts> and <Gigawatts, Requires> are two
    // different pairs, and can be added to an entity at the same time.

    // If both parts of a pair are components, the pair assumes the type of
    // the first element:
    let e3 = world
        .entity()
        .set_pair::<Expires, Position>(Expires { timeout: 0.5 });

    if let Ok(expires) = e3.try_get::<&(Expires, Position)>() {
        println!("expires: {}", expires.timeout);
    }

    println!(
        "{}",
        world
            .id_view_from((Requires::id(), Gigawatts))
            .type_id()
            .path()
            .unwrap()
    );
    println!(
        "{}",
        world
            .id_view_from((Gigawatts, Requires::id()))
            .type_id()
            .path()
            .unwrap()
    );
    println!(
        "{}",
        world
            .id_view_from((Expires::id(), Position::id()))
            .type_id()
            .path()
            .unwrap()
    );

    // When querying for a relationship component, add the pair type as template
    // argument to the builder:
    let query = world.query::<&(Requires, Gigawatts)>().build();

    query.each_entity_exclusive(&mut world, |entity, requires| {
        println!("requires: {} gigawatts", requires.amount);
    });

    // Output:
    // e1: requires: 1.21
    // e1: requires: 1.5
    // expires: 0.5
    // ::Requires
    // ::Requires
    // ::Expires
    // requires: 1.21 gigawatts
}

#[cfg(feature = "flecs_nightly_tests")]
#[test]
fn test() {
    let output_capture = OutputCapture::capture().unwrap();
    main();
    // output_capture.test("relationships_component_data".to_string());
}
