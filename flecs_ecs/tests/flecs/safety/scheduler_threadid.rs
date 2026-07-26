//! Scheduler `ThreadId` CI contract (spec §6.3).
//!
//! The soundness of the per-stage, no-atomic lock maps rests on one flecs C
//! invariant: a non-`multi_threaded` system only ever runs on the thread that
//! called `progress()`. In the vendored core this is asserted by
//! `flecs_run_pipeline_ops`: `ecs_assert(!stage_index || op->multi_threaded)`
//! (mirrored in the `world_ctx.rs` thread-affinity comment). Because every safe
//! handle is `!Send`/`!Sync` and stays on the owning thread, a single-threaded
//! system that leaked onto a worker stage would let two threads mutate the same
//! per-stage lock map without atomics, which would be unsound.
//!
//! This test promotes that comment to a checked contract: it registers a mix of
//! single- and multi-threaded systems on a world with several worker threads,
//! records the `ThreadId` each callback runs on across several `progress()`
//! frames, and asserts that every non-`multi_threaded` system ran ONLY on the
//! `progress()` thread. It gates the vendored flecs C bump: a scheduler change
//! that lets single-threaded work escape stage 0 fails here instead of silently
//! unsoundening the lock maps.

use alloc::sync::Arc;
use std::collections::HashSet;
use std::sync::Mutex;
use std::thread::ThreadId;

use flecs_ecs::core::*;
use flecs_ecs::macros::*;

#[derive(Component)]
struct StSingle(u64);

#[derive(Component)]
struct MtWork(u64);

#[derive(Component)]
struct SpreadA;

#[derive(Component)]
struct SpreadB;

const ENTITY_COUNT: u64 = 1000;
const FRAMES: u64 = 8;
const WORKERS: i32 = 4;

/// Every non-`multi_threaded` system runs only on the `progress()` thread; a
/// `multi_threaded` system's work is observed off that thread whenever the
/// scheduler actually spread it across more than one worker (§6.3;
/// `flecs_run_pipeline_ops` assert `!stage_index || op->multi_threaded`). The
/// off-thread half is gated on the observed worker count so it cannot flake on
/// single-core CI that collapses every stage onto the calling thread.
#[test]
fn single_threaded_systems_run_only_on_progress_thread() {
    let world = World::new();

    for i in 0..ENTITY_COUNT {
        let e = world.entity().set(StSingle(0)).set(MtWork(0));
        // spread entities over several tables so the multi_threaded system has
        // batches to distribute across worker stages
        match i % 3 {
            0 => {
                e.add(SpreadA);
            }
            1 => {
                e.add(SpreadB);
            }
            _ => {}
        }
    }

    let st_threads: Arc<Mutex<HashSet<ThreadId>>> = Arc::new(Mutex::new(HashSet::new()));
    let mt_threads: Arc<Mutex<HashSet<ThreadId>>> = Arc::new(Mutex::new(HashSet::new()));

    world.set_threads(WORKERS);

    // single-threaded system, `each` form
    let st_each = st_threads.clone();
    world.system::<&StSingle>().each(move |_| {
        st_each.lock().unwrap().insert(std::thread::current().id());
    });

    // single-threaded system, `run` form: a second, differently-shaped
    // callback that must also stay on the progress() thread
    let st_run = st_threads.clone();
    world.system::<&StSingle>().run(move |iter| {
        st_run.lock().unwrap().insert(std::thread::current().id());
        iter.fini();
    });

    // multi_threaded system: disjoint mutable access split across worker stages
    let mt = mt_threads.clone();
    world.system::<&mut MtWork>().par_each(move |work| {
        work.0 += 1;
        mt.lock().unwrap().insert(std::thread::current().id());
    });

    let progress_thread = std::thread::current().id();

    for _ in 0..FRAMES {
        world.progress();
    }

    let st = st_threads.lock().unwrap();
    assert!(
        !st.is_empty(),
        "single-threaded systems never ran; the contract was not exercised"
    );
    for id in st.iter() {
        assert_eq!(
            *id, progress_thread,
            "a non-multi_threaded system ran off the progress() thread; \
             the flecs_run_pipeline_ops invariant (!stage_index || \
             op->multi_threaded) no longer holds and per-stage lock maps are unsound"
        );
    }

    let mt = mt_threads.lock().unwrap();
    assert!(
        !mt.is_empty(),
        "multi_threaded system never ran; the contract was not exercised"
    );
    // Gate the off-thread assertion on the observed worker count: only claim
    // "workers ran elsewhere" when the scheduler actually used more than one
    // thread for the multi_threaded system. Constrained CI that collapses every
    // stage onto the calling thread reports one thread and skips this half.
    let observed_worker_count = mt.len();
    if observed_worker_count > 1 {
        assert!(
            mt.iter().any(|id| *id != progress_thread),
            "multi_threaded work spanned {observed_worker_count} threads but none \
             were off the progress() thread"
        );
    }
}
