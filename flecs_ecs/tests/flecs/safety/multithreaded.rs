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

#[derive(Component)]
struct Marked;

#[derive(Component)]
struct Bumped(u64);

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
    let mut world = World::new();
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
    let mut world = World::new();

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
    let mut world = World::new();
    world
        .component::<SparseCounter>()
        .add_trait::<flecs::Sparse>();

    let e = world.entity().set(SparseCounter(0)).id();

    world.set_threads(4);
    world.system::<&mut SparseCounter>().par_each(|counter| {
        counter.0 += 1;
    });
    world.progress();

    world.set_threads(1);

    // nested mutable access to the same sparse component on the same stage
    // must still panic
    let e = world.entity_from_id(e);
    e.get::<&mut SparseCounter>(|_outer| {
        e.get::<&mut SparseCounter>(|_inner| {});
    });
}

#[test]
fn par_each_entity_cached_ref_preserves_worker_stage() {
    let mut world = World::new();
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
    let mut world = World::new();
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

/// Stage isolation across workers (spec §6.1, §7.3): a `par_each_entity_with`
/// system running on multiple stages, each worker issuing Stage-deferred
/// structural commands on its own partition's entities. Every command must apply
/// after the pipeline sync point, with no false mut-alias conflict and no abort:
/// each worker enqueues onto its own single-owner stage command buffer, and flecs
/// merges them into the real world at the sync point.
#[test]
fn par_each_with_stage_deferred_structural_ops_apply_after_sync() {
    let mut world = World::new();
    // Register the deferred-added component up front: registration cannot happen
    // on a worker during the multithreaded phase.
    world.component::<Marked>();

    spawn_across_tables(&world, |e| {
        e.set(DenseCounter(0));
    });
    world.set_threads(4);

    world
        .system::<&DenseCounter>()
        .multi_threaded()
        .par_each_entity_with(|entity, _c, stage| {
            // deferred structural add on the visited entity, through this
            // worker's stage
            stage.entity_view(entity.id()).add(Marked::id());
        });

    world.progress();

    // Every entity received its deferred tag exactly once at the sync point.
    let mut marked = 0u64;
    world
        .query::<()>()
        .with(Marked::id())
        .build()
        .each(|_| marked += 1);
    assert_eq!(marked, ENTITY_COUNT);
}

/// Stage isolation for deferred data writes: each worker issues a deferred `set`
/// on its own partition's entities; all land after the sync point with no abort.
#[test]
fn par_each_with_stage_deferred_set_applies_after_sync() {
    let mut world = World::new();
    world.component::<Bumped>();

    spawn_across_tables(&world, |e| {
        e.set(DenseCounter(7));
    });
    world.set_threads(4);

    world
        .system::<&DenseCounter>()
        .multi_threaded()
        .par_each_entity_with(|entity, c, stage| {
            stage.entity_view(entity.id()).set(Bumped(c.0 + 1));
        });

    world.progress();

    let mut sum = 0u64;
    let mut count = 0u64;
    world.new_query::<&Bumped>().each(|b| {
        sum += b.0;
        count += 1;
    });
    assert_eq!(count, ENTITY_COUNT);
    assert_eq!(sum, ENTITY_COUNT * 8);
}
