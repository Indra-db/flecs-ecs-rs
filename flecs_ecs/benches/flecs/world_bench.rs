use crate::common_bench::*;
use flecs_ecs::sys;

fn util_delete_empty_tables(world: &World, budget: f64) -> i32 {
    world.delete_empty_tables(sys::ecs_delete_empty_tables_desc_t {
        clear_generation: 0,
        delete_generation: 1,
        time_budget_seconds: budget,
        offset: 0,
    })
}

macro_rules! cleanup_tables_bench {
    ($group:expr, $label:expr, $table_count:expr, $empty_count:expr, $budget:expr) => {{
        $group.bench_function($label, |bencher| {
            bencher.iter_custom(|iters| {
                let mut total = Duration::ZERO;
                for _ in 0..iters {
                    let world = World::new_mini();
                    let mut entities = Vec::with_capacity($table_count as usize);
                    for _ in 0..$table_count {
                        let tag = world.entity();
                        let e = world.entity().add(tag);
                        entities.push(e);
                    }

                    util_delete_empty_tables(&world, $budget);
                    util_delete_empty_tables(&world, $budget);

                    for e in entities.iter().copied().take($empty_count as usize) {
                        e.destruct();
                    }

                    let start = Instant::now();
                    util_delete_empty_tables(&world, $budget);
                    util_delete_empty_tables(&world, $budget);
                    total += start.elapsed();
                }
                total
            });
        });
    }};
}

pub fn world_init_fini(criterion: &mut Criterion) {
    let mut group = criterion.benchmark_group("flecs");

    group.bench_function("world_init_fini", |bencher| {
        bencher.iter_custom(|iters| {
            let start = Instant::now();
            for _ in 0..iters {
                let world = World::new();
                drop(world);
            }
            start.elapsed()
        });
    });

    group.finish();
}

pub fn progress_tasks(criterion: &mut Criterion) {
    let mut group = criterion.benchmark_group("flecs");

    for (label, system_count) in [
        ("progress_0_tasks", 0u32),
        ("progress_1_tasks", 1),
        ("progress_10_tasks", 10),
        ("progress_100_tasks", 100),
    ] {
        group.bench_function(label, |bencher| {
            let world = World::new();

            for _ in 0..system_count {
                world.system::<()>().run(|mut it| {
                    while it.next() {}
                });
            }

            world.progress_time(1.0);

            bencher.iter_custom(|iters| {
                let start = Instant::now();
                for _ in 0..iters {
                    world.progress_time(1.0);
                }
                start.elapsed()
            });
        });
    }

    group.finish();
}

pub fn progress_systems(criterion: &mut Criterion) {
    let mut group = criterion.benchmark_group("flecs");

    for (label, system_count) in [
        ("progress_0_systems", 0u32),
        ("progress_1_systems", 1),
        ("progress_10_systems", 10),
        ("progress_100_systems", 100),
    ] {
        group.bench_function(label, |bencher| {
            let world = World::new();
            register_component_range!(world, T, 1, 1);

            for _ in 0..100 {
                let tag = world.entity();
                world.entity().add(T1::id()).add(tag);
            }

            for _ in 0..system_count {
                world.system::<()>().with(T1::id()).run(|mut it| {
                    while it.next() {}
                });
            }

            bencher.iter_custom(|iters| {
                let start = Instant::now();
                for _ in 0..iters {
                    world.progress_time(0.0);
                }
                start.elapsed()
            });

            reset_world_arrays(&world);
        });
    }

    group.finish();
}

pub fn cleanup_tables(criterion: &mut Criterion) {
    let mut group = criterion.benchmark_group("flecs");

    cleanup_tables_bench!(group, "cleanup_tables_0_empty", 32768, 0, 0.0);
    cleanup_tables_bench!(group, "cleanup_tables_half_empty", 32768, 16384, 0.0);
    cleanup_tables_bench!(group, "cleanup_tables_all_empty", 32768, 32768, 0.0);
    cleanup_tables_bench!(group, "cleanup_tables_0_empty_w_budget", 32768, 0, 1.0);
    cleanup_tables_bench!(group, "cleanup_tables_half_empty_w_budget", 32768, 16384, 1.0);
    cleanup_tables_bench!(group, "cleanup_tables_all_empty_w_budget", 32768, 32768, 1.0);

    group.finish();
}
