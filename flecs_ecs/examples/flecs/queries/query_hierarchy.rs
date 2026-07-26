use crate::z_ignore_test_common::*;

use flecs_ecs::experimental::prelude::*;
use flecs_ecs::prelude::*;

#[derive(Debug, Component, Default)]
struct LocalTransform {
    x: f32,
    y: f32,
}

#[derive(Debug, Component, Default)]
struct WorldTransform {
    x: f32,
    y: f32,
}

fn main() {
    let mut world = World::new();

    // Whenever we add LocalTransform, also add WorldTransform.
    world
        .component::<LocalTransform>()
        .add_trait::<(flecs::With, WorldTransform)>();

    // Create a hierarchy. For an explanation see the entity_hierarchy example.
    let sun = world
        .entity_named("Sun")
        .set(LocalTransform { x: 1.0, y: 1.0 });

    world
        .entity_named_child_of(sun, "Mercury")
        .set(LocalTransform { x: 1.0, y: 1.0 });

    world
        .entity_named_child_of(sun, "Venus")
        .set(LocalTransform { x: 2.0, y: 2.0 });

    let earth = world
        .entity_named_child_of(sun, "Earth")
        .set(LocalTransform { x: 3.0, y: 3.0 });

    world
        .entity_named_child_of(earth, "Moon")
        .set(LocalTransform { x: 0.1, y: 0.1 });

    // Create a hierarchical query to compute the global position from the
    // local position and the parent position. The three terms are:
    //  - Local position
    //  - World parent position (optional, so we match root entities)
    //  - World position
    let query = world
        .query::<(
            &LocalTransform,
            Option<&WorldTransform>,
            &mut WorldTransform,
        )>()
        // Select the second term from the parent entity with .parent(). The
        // cascade() modifier ensures that the query iterates parents before
        // children.
        .term_at(1)
        .parent()
        .cascade()
        .build();

    query.each_exclusive(&mut world, |(t_local, t_parent, t_world)| {
        t_world.x = t_local.x;
        t_world.y = t_local.y;
        if let Some(t_parent) = t_parent {
            t_world.x += t_parent.x;
            t_world.y += t_parent.y;
        }
    });

    // Print world positions
    let world_positions = world.new_query::<&WorldTransform>();
    world_positions.each_entity_exclusive(&mut world, |entity, p| {
        println!("{}: {{{}, {}}}", entity.name(), p.x, p.y);
    });

    // Output:
    //  Sun: {1, 1}
    //  Mercury: {2, 2}
    //  Venus: {3, 3}
    //  Earth: {4, 4}
    //  Moon: {4.1, 4.1}
}

#[cfg(feature = "flecs_nightly_tests")]
#[test]
fn test() {
    let output_capture = OutputCapture::capture().unwrap();
    main();
    output_capture.test("query_hierarchy".to_string());
}
