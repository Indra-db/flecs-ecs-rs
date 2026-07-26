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

fn main() {
    let mut world = World::new();

    // Create a system for moving an entity
    world
        .system::<(&mut Position, &Velocity)>()
        .kind(id::<flecs::pipeline::OnUpdate>())
        .each(|(p, v)| {
            p.x += v.x;
            p.y += v.y;
        });

    // Create a system for printing the entity position
    world
        .system::<&Position>()
        .kind(id::<flecs::pipeline::PostUpdate>())
        .each_entity(|e, p| {
            println!("{}: {{ {}, {} }}", e.name(), p.x, p.y);
        });

    // Create a few test entities for a Position, Velocity query
    world
        .entity_named("e1")
        .set(Position { x: 10.0, y: 20.0 })
        .set(Velocity { x: 1.0, y: 2.0 });

    world
        .entity_named("e2")
        .set(Position { x: 10.0, y: 20.0 })
        .set(Velocity { x: 3.0, y: 4.0 });

    // Run the default pipeline. This will run all systems ordered by their
    // phase. Systems within the same phase are ran in declaration order. This
    // function is usually called in a loop.
    world.progress();

    // Output:
    //  e1: { 11, 22 }
    //  e2: { 13, 24 }
}

#[cfg(feature = "flecs_nightly_tests")]
#[test]
fn test() {
    let output_capture = OutputCapture::capture().unwrap();
    main();
    output_capture.test("system_pipeline".to_string());
}
