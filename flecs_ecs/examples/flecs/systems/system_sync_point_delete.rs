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

    // This example shows how to annotate systems that delete entities, in a way
    // that allows the scheduler to correctly insert sync points. See the
    // sync_point example for more details on sync points.
    //
    // While annotating a system for a delete operation follows the same
    // design as other operations, one key difference is that a system often
    // does not know which components a to be deleted entity has. This makes it
    // impossible to annotate the system in advance for specific components.
    //
    // To ensure the scheduler is still able to insert the correct sync points,
    // a system can use a wildcard to indicate that any component could be
    // modified by the system, which forces the scheduler to insert a sync.

    // Basic move system.
    world
        .system_named::<(&mut Position, &Velocity)>("Move")
        .each(|(p, v)| {
            p.x += v.x;
            p.y += v.y;
        });

    // Delete entities when p.x >= 3. Add wildcard annotation to indicate any
    // component could be written by the system. Position itself is added as
    // const, since inside the system we're only reading it.
    world
        .system_named::<&Position>("DeleteEntity")
        .with(id::<&mut flecs::Wildcard>())
        .each_entity(|e, p| {
            if p.x >= 3.0 {
                println!("Delete entity {}", e.name());
                e.destruct();
            }
        });

    // Print resulting Position. Note that this system will never print entities
    // that have been deleted by the previous system.
    world
        .system_named::<&Position>("PrintPosition")
        .each_entity(|e, p| {
            println!("{}: {{ {}, {} }}", e.name(), p.x, p.y);
        });

    // Create a few test entities for a Position, Velocity query
    world
        .entity_named("e1")
        .set(Position { x: 0.0, y: 0.0 })
        .set(Velocity { x: 1.0, y: 2.0 });

    world
        .entity_named("e2")
        .set(Position { x: 1.0, y: 2.0 })
        .set(Velocity { x: 1.0, y: 2.0 });

    // Run systems. Debug logging enables us to see the generated schedule.
    // NOTE flecs C / flecs_ecs_sys needs to be build in debug mode to see the logging.
    // use the feature flag "sys_build_debug" to enable debug build of flecs C.

    set_log_level(1);

    while world.progress() {
        if world.count(Position::id()) == 0 {
            break; // No more entities left with Position
        }
    }
    set_log_level(-1);

    // world
    //     .get::<Snap>()
    //     .test("system_sync_point_delete".to_string()));

    // Output:
    //  info: pipeline rebuild
    //  info: | schedule: threading: 0, staging: 1:
    //  info: | | system Move
    //  info: | | system DeleteEntity
    //  info: | | merge
    //  info: | schedule: threading: 0, staging: 1:
    //  info: | | system PrintPosition
    //  info: | | merge
    //  e1: { 1, 2 }
    //  e2: { 2, 4 }
    //  Delete entity e2
    //  e1: { 2, 4 }
    //  Delete entity e1

    // Removing the wildcard annotation from the DeleteEntity system will
    // remove the first sync point.

    // Note how after both entities are deleted, all three systems will be de-activated and not ran by the scheduler
}

#[cfg(feature = "flecs_nightly_tests")]
#[test]
#[ignore = "`set_log_level` is not safe in parallel tests"]
fn test() {
    let output_capture = OutputCapture::capture().unwrap();
    main();
    output_capture.test("system_sync_point_delete".to_string());
}
