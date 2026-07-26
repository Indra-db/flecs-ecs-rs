use crate::z_ignore_test_common::*;

use flecs_ecs::experimental::prelude::*;
use flecs_ecs::prelude::*;
#[derive(Debug, Component)]
struct Gravity {
    value: f32,
}

#[derive(Debug, Component)]
pub struct Velocity {
    pub x: f32,
    pub y: f32,
}

fn main() {
    let mut world = World::new();

    // Mark Gravity as singleton
    world.component::<Gravity>().add_trait::<flecs::Singleton>();

    // Set singleton
    world.set(Gravity { value: 9.81 });

    // Set Velocity
    world.entity_named("e1").set(Velocity { x: 0.0, y: 0.0 });
    world.entity_named("e2").set(Velocity { x: 0.0, y: 1.0 });
    world.entity_named("e3").set(Velocity { x: 0.0, y: 2.0 });

    // Create query that matches Gravity as singleton
    let query = world.query::<(&mut Velocity, &Gravity)>().build();

    query.each_entity_exclusive(&mut world, |entity, (velocity, gravity)| {
        velocity.y += gravity.value;
        println!("Entity {} has {:?}", entity.path().unwrap(), velocity);
    });

    // Output:
    // Entity ::e1 has Velocity { x: 0.0, y: 9.81 }
    // Entity ::e2 has Velocity { x: 0.0, y: 10.81 }
    // Entity ::e3 has Velocity { x: 0.0, y: 11.81 }
}

#[cfg(feature = "flecs_nightly_tests")]
#[test]
fn test() {
    let output_capture = OutputCapture::capture().unwrap();
    main();
    output_capture.test("query_singleton".to_string());
}
