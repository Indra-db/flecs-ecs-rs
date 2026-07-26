use crate::z_ignore_test_common::*;

use flecs_ecs::experimental::prelude::*;
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
pub struct Mass {
    pub value: f32,
}

fn main() {
    let mut world = World::new();

    let query = world.new_query::<(&mut Position, &Velocity)>();

    // Create a few test entities for a Position, Velocity query
    world
        .entity_named("e1")
        .set(Position { x: 10.0, y: 20.0 })
        .set(Velocity { x: 1.0, y: 2.0 });

    world
        .entity_named("e2")
        .set(Position { x: 10.0, y: 20.0 })
        .set(Velocity { x: 3.0, y: 4.0 });

    world
        .entity_named("e3")
        .set(Position { x: 10.0, y: 20.0 })
        .set(Velocity { x: 4.0, y: 5.0 })
        .set(Mass { value: 50.0 });

    // The run() function provides a flecs::iter object which contains all sorts
    // of information on the entities currently being iterated.
    query.run_exclusive(&mut world, |mut it| {
        while it.next() {
            let mut position = it.field_mut::<Position>(0);
            let velocity = it.field::<Velocity>(1);

            println!();
            // Print the table & number of entities matched in current callback
            println!("Table: {:?}", it.archetype());
            println!(" - number of entities: {}", it.count());
            println!();

            // Print information about the components being matched
            for i in 0..it.field_count() {
                println!(" - term {i} : ");
                println!("   - component: {}", it.id(i).to_str());
                println!("   - type size: {}", it.size(i));
            }

            println!();

            for i in it.iter() {
                position[i].x += velocity[i].x;
                position[i].y += velocity[i].y;
                println!(" - entity {}: has {:?}", it.entity(i).name(), position[i]);
            }

            println!();
        }
    });

    // Output:
    //  Table: Position, Velocity, (Identifier,Name)
    //  - number of entities: 2
    //
    //  - term 1 :
    //    - component: Position
    //    - type size: 8
    //  - term 2 :
    //    - component: Velocity
    //    - type size: 8
    //
    //  - entity e1: has Position { x: 11.0, y: 22.0 }
    //  - entity e2: has Position { x: 13.0, y: 24.0 }
    //
    //
    // Table: Position, Velocity, Mass, (Identifier,Name)
    //  - number of entities: 1
    //
    //  - term 1 :
    //    - component: Position
    //    - type size: 8
    //  - term 2 :
    //    - component: Velocity
    //    - type size: 8
    //
    //  - entity e3: has Position { x: 14.0, y: 25.0 }
    //
}

#[cfg(feature = "flecs_nightly_tests")]
#[test]
fn test() {
    let output_capture = OutputCapture::capture().unwrap();
    main();
    output_capture.test("query_run_iter".to_string());
}
