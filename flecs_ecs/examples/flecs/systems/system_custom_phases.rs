use crate::z_ignore_test_common::*;

use flecs_ecs::prelude::*;
// This application demonstrates how to use custom phases for systems. The
// default pipeline will automatically run systems for custom phases as long as
// they have the flecs::Phase tag.

// Dummy system
fn sys(mut it: TableIter) {
    while it.next() {
        println!("system {}", it.system().name());
    }
}

fn main() {
    let mut world = World::new();

    // Create two custom phases that branch off of EcsOnUpdate. Note that the
    // phases have the Phase tag, which is necessary for the builtin pipeline
    // to discover which systems it should run.
    let physics = world
        .entity()
        .add(flecs::pipeline::Phase)
        .depends_on(id::<flecs::pipeline::OnUpdate>());

    let collisions = world
        .entity()
        .add(id::<flecs::pipeline::Phase>())
        .depends_on(physics);

    // Create 3 dummy systems.
    world
        .system_named::<()>("CollisionSystem")
        .kind(collisions)
        .run(sys);

    world
        .system_named::<()>("PhysicsSystem")
        .kind(physics)
        .run(sys);

    world
        .system_named::<()>("GameSystem")
        .kind(id::<flecs::pipeline::OnUpdate>())
        .run(sys);

    // Run pipeline
    world.progress();

    // Output:
    //   system GameSystem
    //   system PhysicsSystem
    //   system CollisionSystem
}

#[cfg(feature = "flecs_nightly_tests")]
#[test]
fn test() {
    let output_capture = OutputCapture::capture().unwrap();
    main();
    output_capture.test("system_custom_phases".to_string());
}
