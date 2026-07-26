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

fn main() {
    let mut world = World::new();

    // Create a few test entities for a Position, Velocity query
    world
        .entity_named("e1")
        .set(Position { x: 10.0, y: 20.0 })
        .set(Velocity { x: 1.0, y: 2.0 });

    world
        .entity_named("e2")
        .set(Position { x: 10.0, y: 20.0 })
        .set(Velocity { x: 3.0, y: 4.0 });

    // This entity will not match as it does not have Position, Velocity
    world.entity_named("e3").set(Position { x: 10.0, y: 20.0 });

    // Ad hoc queries are bit slower to iterate than flecs::query, but are
    // faster to create, and in most cases require no allocations. Under the
    // hood this API uses flecs::query, which can be used directly for more
    // complex queries.

    let q = world.new_query::<(&mut Position, &Velocity)>();
    q.each_entity_exclusive(&mut world, |entity, (pos, vel)| {
        pos.x += vel.x;
        pos.y += vel.y;
        println!("Entity {}: {:?}", entity.name(), pos);
    });

    // Output:
    //  Entity e1: Position { x: 11.0, y: 22.0 }
    //  Entity e2: Position { x: 13.0, y: 24.0 }
}

#[cfg(feature = "flecs_nightly_tests")]
#[test]
fn test() {
    let output_capture = OutputCapture::capture().unwrap();
    main();
    output_capture.test("query_world_query".to_string());
}
