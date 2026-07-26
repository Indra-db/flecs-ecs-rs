//! Multithreaded systems (spec §6, §13.4).
//!
//! Demonstrates the redesign's highest-stakes surface: a `.multi_threaded()`
//! system whose matched entities flecs partitions across worker stages, so each
//! row is written by exactly one worker (`par_each`), and a second multithreaded
//! system that issues **deferred commands** through the per-worker
//! [`Stage`](flecs_ecs::experimental::Stage) handle (`par_each_entity_with`).
//!
//! Deferred stage commands are merged into the world at the pipeline sync point,
//! so the final state is deterministic even though the work ran across threads.

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

/// Tag added, through a stage-deferred command, to entities that reached the
/// finish line this frame.
#[derive(Component)]
pub struct Arrived;

const FINISH_LINE: f32 = 100.0;

fn main() {
    let mut world = World::new();

    // Register `Arrived` up front: component registration mutates shared world
    // state and cannot happen on a worker during the multithreaded phase.
    world.component::<Arrived>();

    // Spread work across 4 worker stages.
    world.set_threads(4);

    // Partitioned parallel write: flecs hands each worker a disjoint slice of the
    // matched tables, so the `&mut Position` writes never alias across threads.
    world
        .system::<(&mut Position, &Velocity)>()
        .multi_threaded()
        .par_each(|(pos, vel)| {
            pos.x += vel.x;
            pos.y += vel.y;
        });

    // Multithreaded system issuing a stage-deferred command: each worker gets a
    // `Stage` handle; `add` on a stage-world entity view enqueues on that
    // worker's command buffer and merges at the sync point. (`par_each_with` is
    // the entity-less sibling for commands that do not target the visited row.)
    world
        .system::<&Position>()
        .multi_threaded()
        .par_each_entity_with(|entity, pos, stage| {
            if pos.x >= FINISH_LINE {
                stage.entity_view(entity.id()).add(Arrived::id());
            }
        });

    // A field of 8 runners.
    for i in 0..8 {
        world
            .entity()
            .set(Position {
                x: i as f32 * 10.0,
                y: 0.0,
            })
            .set(Velocity { x: 100.0, y: 0.0 });
    }

    // One frame: the move system runs (parallel write), then the finish-line
    // system defers an `Arrived` add per crossing runner; the deferred adds merge
    // at the end of the frame.
    world.progress();

    // Deterministic summary, read back single-threaded after the sync point.
    let mut arrived = 0;
    world
        .query::<()>()
        .with(Arrived::id())
        .build()
        .each(|_| arrived += 1);
    let mut total_x = 0.0;
    world.new_query::<&Position>().each(|p| total_x += p.x);

    println!("runners arrived: {arrived}");
    println!("total distance: {total_x}");

    // Output:
    //  runners arrived: 8
    //  total distance: 1080
}

#[cfg(feature = "flecs_nightly_tests")]
#[test]
fn test() {
    let output_capture = OutputCapture::capture().unwrap();
    main();
    output_capture.test("system_multithreaded".to_string());
}
