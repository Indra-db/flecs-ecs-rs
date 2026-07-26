//! Tests for the app addon: world lifecycle and custom frame/run actions.

use alloc::sync::Arc;
use core::ffi::c_void;
use core::sync::atomic::{AtomicI32, Ordering};

use flecs_ecs::prelude::*;
use flecs_ecs::sys;

fn world_refcount(world: &World) -> i32 {
    // SAFETY: world.ptr_mut() is a valid live world pointer.
    unsafe { sys::flecs_poly_refcount(world.ptr_mut() as *mut c_void) }
}

#[test]
fn app_run_keeps_world_refcount_balanced() {
    let world = World::new();

    let refcount_before = world_refcount(&world);

    world.app().set_frames(1).run();

    assert_eq!(
        world_refcount(&world),
        refcount_before,
        "App::run must not steal a world refcount from live World handles"
    );

    // The world must still be fully usable after the app quit.
    let entity = world.entity();
    assert!(entity.is_alive());
}

#[test]
fn app_dropped_without_run_keeps_world_refcount_balanced() {
    let world = World::new();
    let refcount_before = world_refcount(&world);

    {
        let mut app = world.app();
        app.set_frames(1);
        // dropped without calling run()
    }

    assert_eq!(world_refcount(&world), refcount_before);
    let entity = world.entity();
    assert!(entity.is_alive());
}

#[test]
fn app_frame_action_runs_every_frame() {
    let world = World::new();

    let frames = Arc::new(AtomicI32::new(0));
    let frames_clone = frames.clone();

    world
        .app()
        .set_frames(3)
        .frame_action(move |world, _desc| {
            frames_clone.fetch_add(1, Ordering::Relaxed);
            if unsafe { sys::ecs_progress(world.ptr_mut(), 0.0) } {
                0
            } else {
                1
            }
        })
        .run();

    assert_eq!(frames.load(Ordering::Relaxed), 3);
}
