use crate::z_ignore_test_common::*;

use flecs_ecs::prelude::*;

fn main() {
    let mut world = World::new();

    // Create system that prints delta_time. This system doesn't query for any
    // components which means it won't match any entities, but will still be ran
    // once for each call to ecs_progress.
    world.system::<()>().run(|mut it| {
        while it.next() {
            println!("delta_time: {}", it.delta_time());
        }
    });

    // Call progress with 0.0f for the delta_time parameter. This will cause
    // ecs_progress to measure the time passed since the last frame. The
    // delta_time of the first frame is a best guess (16ms).
    world.progress();

    // The following calls should print a delta_time of approximately 100ms

    let os_sleep = unsafe { flecs_ecs_sys::ecs_os_api.sleep_ }.unwrap();

    unsafe { os_sleep(0, 100 * 1000 * 1000) };
    world.progress();

    unsafe { os_sleep(0, 100 * 1000 * 1000) };

    world.progress();

    // Output:
    //  delta_time: 0.016666668
    //  delta_time: 0.10155179
    //  delta_time: 0.10091246
}

#[cfg(feature = "flecs_nightly_tests")]
#[test]
fn test() {
    let output_capture = OutputCapture::capture().unwrap();
    main();
    assert!(output_capture.output().lock().unwrap().len() > 0);
}
