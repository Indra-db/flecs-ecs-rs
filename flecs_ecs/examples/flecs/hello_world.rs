use crate::z_ignore_test_common::*;

use flecs_ecs::prelude::*;

#[derive(Debug, Component)]
pub struct Position {
    pub x: f32,
    pub y: f32,
}

#[derive(Debug, Component)]
pub struct Velocity {
    pub x: f32,
    pub y: f32,
}

#[derive(Component)]
pub struct Eats;

#[derive(Component)]
pub struct Apples;

fn main() {
    // Create a new world
    let mut world = World::new();

    // Register system
    let _sys = world
        .system::<(&mut Position, &Velocity)>()
        // .each_entity if you want the entity to be added in the parameter list
        .each(|(pos, vel)| {
            pos.x += vel.x;
            pos.y += vel.y;
        });

    // Create an entity with name Bob, add Position and food preference
    let bob = world
        .entity_named("Bob")
        .set(Position { x: 0.0, y: 0.0 })
        .set(Velocity { x: 1.0, y: 2.0 })
        .add((Eats, Apples));

    // Show us what you got
    // println!( "{}'s got [{:?}]", bob.name(), bob.archetype());
    println!("{}'s got [{:?}]", bob.name(), bob.archetype());

    // Take the id so the entity handle's borrow does not span the exclusive
    // `progress` frame step.
    let bob = bob.id();

    // Run systems twice. Usually this function is called once per frame
    world.progress();
    world.progress();

    // - get panics if the component is not present, use try_get for a non-panicking version which does not run the callback.
    // - or use Option to handle the individual component missing.
    world.entity_from_id(bob).get::<&Position>(|pos| {
        // See if Bob has moved (he has)
        println!("{}'s position: {:?}", world.entity_from_id(bob).name(), pos);
    });

    // Option example
    let has_run = world.entity_from_id(bob)
        .try_get::<Option<&Position>>(|pos| {
            if let Some(pos) = pos {
                // See if Bob has moved (he has)
                //println!( "{}'s try_get position: {:?}", bob.name(), pos);
                println!("{}'s try_get position: {:?}", world.entity_from_id(bob).name(), pos);
            }
        })
        .is_some();

    if has_run {
        println!("Bob has a position component, so the try_get callback ran.");
    }

    // Output:
    //  Bob's got [Position, Velocity, (Identifier,Name), (Eats,Apples)]
    //  Bob's position: Position { x: 2.0, y: 4.0 }
    //  Bob's try_get position: Position { x: 2.0, y: 4.0 }
    //  Bob has a position component, so the try_get callback ran.
}

#[cfg(feature = "flecs_nightly_tests")]
#[test]
fn test() {
    let output_capture = OutputCapture::capture().unwrap();
    main();
    output_capture.test("hello world".to_string());
}
