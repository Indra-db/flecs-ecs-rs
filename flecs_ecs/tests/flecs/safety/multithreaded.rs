//! Multithreaded pipeline runs must not report mut-alias violations for
//! disjoint entities: stage lock maps are per stage, so workers processing
//! different partitions of the same component never conflict.

use core::sync::atomic::{AtomicU64, Ordering};

use flecs_ecs::core::*;
use flecs_ecs::macros::*;

#[derive(Component)]
struct SparseCounter(u64);

#[derive(Component)]
struct DenseCounter(u64);

#[derive(Component)]
struct StageProbe(u64);

#[derive(Component)]
struct TagA;

#[derive(Component)]
struct TagB;

#[derive(Component)]
struct TagC;

const ENTITY_COUNT: u64 = 1000;
const FRAMES: u64 = 8;

fn spawn_across_tables(world: &World, f: impl Fn(EntityView)) {
    for i in 0..ENTITY_COUNT {
        let e = world.entity();
        // spread entities over several tables so workers process the same
        // component from different batches concurrently
        match i % 4 {
            0 => {
                e.add(TagA);
            }
            1 => {
                e.add(TagB);
            }
            2 => {
                e.add(TagA).add(TagC);
            }
            _ => {}
        }
        f(e);
    }
}

/// Parallel mutable access to a sparse component on disjoint entities used to
/// false-positive with the C-side global sparse lock; per-stage tracking must
/// accept it.
#[test]
fn par_each_mut_sparse_disjoint_entities_no_violation() {
    let world = World::new();
    world
        .component::<SparseCounter>()
        .add_trait::<flecs::Sparse>();

    spawn_across_tables(&world, |e| {
        e.set(SparseCounter(0));
    });

    static PROCESSED: AtomicU64 = AtomicU64::new(0);
    PROCESSED.store(0, Ordering::Relaxed);

    world.set_threads(4);
    world.system::<&mut SparseCounter>().par_each(|counter| {
        counter.0 += 1;
        PROCESSED.fetch_add(1, Ordering::Relaxed);
    });

    for _ in 0..FRAMES {
        world.progress();
    }

    assert_eq!(PROCESSED.load(Ordering::Relaxed), ENTITY_COUNT * FRAMES);
}

/// Same guarantee for dense columns (already per stage before): workers
/// writing the same column type in different partitions must not conflict.
#[test]
fn par_each_mut_dense_disjoint_entities_no_violation() {
    let world = World::new();

    spawn_across_tables(&world, |e| {
        e.set(DenseCounter(0));
    });

    static PROCESSED: AtomicU64 = AtomicU64::new(0);
    PROCESSED.store(0, Ordering::Relaxed);

    world.set_threads(4);
    world.system::<&mut DenseCounter>().par_each(|counter| {
        counter.0 += 1;
        PROCESSED.fetch_add(1, Ordering::Relaxed);
    });

    for _ in 0..FRAMES {
        world.progress();
    }

    assert_eq!(PROCESSED.load(Ordering::Relaxed), ENTITY_COUNT * FRAMES);
}

/// Single-threaded conflicts must still be detected after the multithreaded
/// runs above: the per-stage maps go back to stage 0 once threads are reset.
#[test]
#[should_panic(expected = "Cannot set write")]
fn conflict_detection_still_active_after_multithreaded_run() {
    let world = World::new();
    world
        .component::<SparseCounter>()
        .add_trait::<flecs::Sparse>();

    let e = world.entity().set(SparseCounter(0));

    world.set_threads(4);
    world.system::<&mut SparseCounter>().par_each(|counter| {
        counter.0 += 1;
    });
    world.progress();

    world.set_threads(1);

    // nested mutable access to the same sparse component on the same stage
    // must still panic
    e.get::<&mut SparseCounter>(|_outer| {
        e.get::<&mut SparseCounter>(|_inner| {});
    });
}

#[test]
fn par_each_entity_cached_ref_preserves_worker_stage() {
    let world = World::new();
    spawn_across_tables(&world, |entity| {
        entity.set(DenseCounter(0)).set(StageProbe(0));
    });
    world.set_threads(4);

    world
        .system::<&mut DenseCounter>()
        .par_each_entity(move |entity, _| {
            let mut cached = entity.cached_ref(StageProbe::id());
            let cached_stage = cached.world().stage_id();
            cached.get(|probe| {
                assert_eq!(cached_stage, entity.world().stage_id());
                probe.0 += 1;
            });
        });

    world.progress();
}

#[test]
fn par_each_iter_entity_preserves_worker_stage() {
    let world = World::new();
    spawn_across_tables(&world, |entity| {
        entity.set(DenseCounter(0));
    });
    world.set_threads(4);

    world
        .system::<&mut DenseCounter>()
        .par_each_iter(move |iter, row, _| {
            let entity = iter.entity(row);
            assert_eq!(entity.world().stage_id(), iter.world().stage_id());
        });

    world.progress();
}
