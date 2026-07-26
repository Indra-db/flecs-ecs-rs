use core::hint::black_box;

use crate::common_bench::*;

const BATCH_COUNT: usize = 1000;

/// One entity from a 3-component bundle: `World::spawn` (one archetype move) vs
/// the fluent `entity().set().set().set()` (three moves).
pub fn bundle_spawn(criterion: &mut Criterion) {
    let mut group = criterion.benchmark_group("flecs");

    group.bench_function("bundle_spawn_3", |b| {
        b.iter_batched(
            World::new,
            |world| {
                for _ in 0..ENTITY_COUNT {
                    black_box(world.spawn((C1(1), C2(2), C3(3))));
                }
                world
            },
            criterion::BatchSize::SmallInput,
        );
    });

    group.bench_function("bundle_spawn_3_via_set", |b| {
        b.iter_batched(
            World::new,
            |world| {
                for _ in 0..ENTITY_COUNT {
                    black_box(world.entity().set(C1(1)).set(C2(2)).set(C3(3)));
                }
                world
            },
            criterion::BatchSize::SmallInput,
        );
    });

    group.finish();
}

/// `BATCH_COUNT` entities from a 3-component bundle: one `World::spawn_batch`
/// call vs a loop of `entity().set().set().set()`.
pub fn bundle_spawn_batch(criterion: &mut Criterion) {
    let mut group = criterion.benchmark_group("flecs");

    group.bench_function("bundle_spawn_batch_1000", |b| {
        b.iter_batched(
            World::new,
            |world| {
                black_box(world.spawn_batch((C1(1), C2(2), C3(3)), BATCH_COUNT));
                world
            },
            criterion::BatchSize::SmallInput,
        );
    });

    group.bench_function("bundle_spawn_batch_1000_via_set_loop", |b| {
        b.iter_batched(
            World::new,
            |world| {
                for _ in 0..BATCH_COUNT {
                    black_box(world.entity().set(C1(1)).set(C2(2)).set(C3(3)));
                }
                world
            },
            criterion::BatchSize::SmallInput,
        );
    });

    group.finish();
}

/// Adding a 3-component bundle to an existing entity: `insert` (one archetype
/// move) vs the fluent `set().set().set()` (three moves).
pub fn bundle_insert(criterion: &mut Criterion) {
    let mut group = criterion.benchmark_group("flecs");

    group.bench_function("bundle_insert_3", |b| {
        b.iter_batched(
            World::new,
            |world| {
                for _ in 0..ENTITY_COUNT {
                    black_box(world.entity().insert((C1(1), C2(2), C3(3))));
                }
                world
            },
            criterion::BatchSize::SmallInput,
        );
    });

    group.bench_function("bundle_insert_3_via_set", |b| {
        b.iter_batched(
            World::new,
            |world| {
                for _ in 0..ENTITY_COUNT {
                    black_box(world.entity().set(C1(1)).set(C2(2)).set(C3(3)));
                }
                world
            },
            criterion::BatchSize::SmallInput,
        );
    });

    group.finish();
}
