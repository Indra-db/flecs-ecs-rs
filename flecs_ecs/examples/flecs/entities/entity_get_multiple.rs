use crate::z_ignore_test_common::*;

use flecs_ecs::experimental::prelude::*;
use flecs_ecs::prelude::*;

// This code shows how to get multiple components in a single command

#[derive(Debug, Component)]
pub struct Position {
    pub x: f64,
    pub y: f64,
}

#[derive(Debug, Component)]
pub struct Mass {
    pub value: f64,
}

#[derive(Debug, Component)]
pub struct Velocity {
    pub x: f64,
    pub y: f64,
}

fn main() {
    let world = World::new();

    // Create new entity, set Position and Mass component
    let e = world
        .entity()
        .set(Position { x: 10.0, y: 20.0 })
        .set(Mass { value: 100.0 });

    // Multiple components can be fetched mutably in a single get by using a
    // tuple of mutable references. Each column comes back as its own guard, so
    // the fields are mutated directly with no callback.
    let (mut pos, mut mass) = e.get_ref::<(&mut Position, &mut Mass)>().unwrap();
    pos.x += 5.0;
    mass.value += 3.0;
    println!("Position: {{{}, {}}}", pos.x, pos.y);
    println!("Mass: {{{}}}", mass.value);
    // Release the write guards before the next read: a guard is held for its
    // whole scope, and overlapping borrows of the same component conflict.
    drop((pos, mass));

    println!();

    // The same works with immutable references, which do not allow the
    // components to be modified.
    let (pos, mass) = e.get_ref::<(&Position, &Mass)>().unwrap();
    println!("Position: {{{}, {}}}", pos.x, pos.y);
    println!("Mass: {{{}}}", mass.value);
    drop((pos, mass));

    println!();

    // A component that may be absent can be wrapped in an Option, which is
    // None when the entity does not have the component.
    let (pos, velocity, mass) = e.get_ref::<(&Position, Option<&Velocity>, &Mass)>().unwrap();
    println!("Position: {{{}, {}}}", pos.x, pos.y);
    if let Some(velocity) = velocity {
        println!("Velocity: {{{}, {}}}", velocity.x, velocity.y);
    } else {
        println!("Velocity: not found");
    }
    println!("Mass: {{{}}}", mass.value);

    // Output:
    //  Position: {15, 20}
    //  Mass: {103}
    //
    //  Position: {15, 20}
    //  Mass: {103}
    //
    //  Position: {15, 20}
    //  Velocity: not found
    //  Mass: {103}
}

#[cfg(feature = "flecs_nightly_tests")]
#[test]
fn test() {
    let output_capture = OutputCapture::capture().unwrap();
    main();
    output_capture.test("entity_get_multiple".to_string());
}
