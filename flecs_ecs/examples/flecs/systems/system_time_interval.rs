use crate::z_ignore_test_common::*;

use flecs_ecs::prelude::*;
// This example shows how to run a system at a specified time interval.

#[derive(Component)]
struct Timeout {
    pub value: f32,
}

fn tick(mut it: TableIter) {
    while it.next() {
        println!("{}", it.system().name());
    }
}

fn main() {
    let mut world = World::new();

    world.set(Timeout { value: 3.5 });

    world
        .system::<&mut Timeout>()
        .each_iter(|it, _index, timeout| {
            timeout.value -= it.delta_time();
        });

    world.system_named::<()>("Tick").set_interval(1.0).run(tick);

    world
        .system_named::<()>("FastTick")
        .set_interval(0.5)
        .run(tick);

    // Run the main loop at 60 FPS
    world.set_target_fps(60.0);

    while world.progress() {
        if world.get::<&Timeout>(|timeout| timeout.value <= 0.0) {
            println!("Timed out!");
            break;
        }
    }

    // Output:
    // FastTick
    // Tick
    // FastTick
    // FastTick
    // Tick
    // FastTick
    // FastTick
    // Tick
    // FastTick
    // FastTick
    // Timed out!
}

#[cfg(feature = "flecs_nightly_tests")]
#[test]
fn test() {
    let output_capture = OutputCapture::capture().unwrap();
    main();
    assert!(output_capture.output_string().contains("Timed out!"));
}
