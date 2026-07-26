use crate::z_ignore_test_common::*;

use flecs_ecs::prelude::*;
// Startup systems are systems registered with the EcsOnStart phase, and are
// only ran during the first frame. Just like with regular phases, custom phases
// can depend on the EcsOnStart phase (see custom_phases example). Phases that
// depend on EcsOnStart are also only ran during the first frame.
//
// Other than that, startup systems behave just like regular systems (they can
// match components, can introduce merge points), with as only exception that
// they are guaranteed to always run on the main thread.

fn main() {
    let mut world = World::new();

    // Startup system
    world
        .system_named::<()>("Startup")
        .kind(id::<flecs::pipeline::OnStart>())
        .run(|mut it| {
            while it.next() {
                println!("{}", it.system().name());
            }
        });

    // Regular system
    world.system_named::<()>("Update").run(|mut it| {
        while it.next() {
            println!("{}", it.system().name());
        }
    });

    // First frame. This runs both the Startup and Update systems
    world.progress();

    // Second frame. This runs only the Update system
    world.progress();

    // Output:
    //  Startup
    //  Update
    //  Update
}

#[cfg(feature = "flecs_nightly_tests")]
#[test]
fn test() {
    let output_capture = OutputCapture::capture().unwrap();
    main();
    output_capture.test("system_startup_system".to_string());
}
