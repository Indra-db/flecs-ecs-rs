//! SW-3 deliverable 4: `System::run_with` (spec §5.7).
//!
//! `run_with(&mut World, RunArgs)` is the explicit, exclusive-register terminal
//! that replaces the `Drop`-driven `SystemRunnerFluent` (deleted in SW-15). The
//! `RunArgs` surface is exactly the knobs the vendored C `ecs_run` /
//! `ecs_run_worker` accept: `delta_time`, `param`, and an optional worker span.
//!
//! Today's `System` still carries the `&World` borrow of the world it was built
//! from, so `run_with`'s `&mut World` argument conflicts with a handle stored
//! from that same world binding. Until the handle is decoupled from the world
//! borrow (a later sub-wave), a detached handle is reconstructed here — exactly
//! what that decoupling does internally — to exercise the terminal.

use core::sync::atomic::{AtomicI32, Ordering};

use flecs_ecs::addons::system::{RunArgs, System, WorkerSpan};
use flecs_ecs::core::*;
use flecs_ecs::experimental::prelude::EntityGuardExt;
use flecs_ecs::macros::*;

#[derive(Component)]
struct Counter(i32);

/// Re-obtain a `System` handle detached from the world borrow, so `run_with`'s
/// `&mut World` argument does not conflict with it. `run_with` runs on the
/// `&mut World` it is passed and only reads the system id out of the handle.
fn detached_system(world: &World, id: Entity) -> System<'static> {
    let ptr = world.world_ptr_mut();
    // SAFETY: `ptr` is this world's live pointer; the returned handle is used
    // only to name the system id to `run_with`, whose own `&mut World` argument
    // is the world it runs on.
    let wref = unsafe { WorldRef::from_ptr(ptr) };
    System::new_from_existing(EntityView::new_from(wref, id))
}

#[test]
fn run_with_runs_the_system() {
    let mut world = World::new();
    let e = world.entity().set(Counter(0)).id();
    let sys_id = world.system::<&mut Counter>().each(|c| c.0 += 1).id();

    let sys = detached_system(&world, sys_id);
    sys.run_with(&mut world, RunArgs::default());
    sys.run_with(&mut world, RunArgs::default());

    {
        let c = world.entity_from_id(e).get_ref::<&Counter>().unwrap();
        assert_eq!(c.0, 2);
    };
}

#[test]
fn run_with_passes_delta_time() {
    let mut world = World::new();
    world.entity().set(Counter(0));

    static SEEN_MS: AtomicI32 = AtomicI32::new(-1);
    SEEN_MS.store(-1, Ordering::Relaxed);

    let sys_id = world
        .system::<&Counter>()
        .each_iter(|it, _row, _c| {
            SEEN_MS.store((it.delta_time() * 1000.0) as i32, Ordering::Relaxed);
        })
        .id();

    let sys = detached_system(&world, sys_id);
    sys.run_with(
        &mut world,
        RunArgs {
            delta_time: 0.5,
            ..Default::default()
        },
    );

    assert_eq!(SEEN_MS.load(Ordering::Relaxed), 500);
}

#[test]
fn run_with_and_progress_mix_in_one_frame() {
    // spec §5.7: both are &mut World, so manual run and progress serialise on the
    // borrow and can be mixed.
    let mut world = World::new();
    let e = world.entity().set(Counter(0)).id();

    // A manual (non-pipeline) system: no phase, so progress() does not run it.
    let sys_id = world
        .system::<&mut Counter>()
        .kind(0u64)
        .each(|c| c.0 += 1)
        .id();

    let sys = detached_system(&world, sys_id);
    sys.run_with(&mut world, RunArgs::default());
    world.progress();
    sys.run_with(&mut world, RunArgs::default());

    {
        let c = world.entity_from_id(e).get_ref::<&Counter>().unwrap();
        assert_eq!(c.0, 2);
    };
}

#[test]
fn run_with_worker_span_partitions() {
    let mut world = World::new();
    for _ in 0..8 {
        world.entity().set(Counter(0));
    }
    world.set_threads(2);

    let sys_id = world
        .system::<&mut Counter>()
        .multi_threaded()
        .each(|c| c.0 += 1)
        .id();

    // Run each worker partition explicitly via ecs_run_worker.
    let sys = detached_system(&world, sys_id);
    sys.run_with(
        &mut world,
        RunArgs {
            worker: Some(WorkerSpan {
                stage_current: 0,
                stage_count: 2,
            }),
            ..Default::default()
        },
    );
    sys.run_with(
        &mut world,
        RunArgs {
            worker: Some(WorkerSpan {
                stage_current: 1,
                stage_count: 2,
            }),
            ..Default::default()
        },
    );

    let mut total = 0;
    world.new_query::<&Counter>().each(|c| total += c.0);
    assert_eq!(total, 8, "each entity visited by exactly one worker partition");
}
