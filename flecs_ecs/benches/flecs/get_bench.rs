use crate::common_bench::*;

pub fn get(criterion: &mut Criterion) {
    let mut group = criterion.benchmark_group("flecs");

    bench_loop_entities!(
        group,
        "get_not_found",
        ENTITY_COUNT
        ; (register_component_range, (C, 1, 2)) // Registration so it doesn't affect the benchmark
        ; (add_component_range, (C, 1, 1)) // Preparation
        ; (get_component_range, (C, 2, 2)) // Benchmark
    );

    bench_loop_entities!(
        group,
        "get",
        ENTITY_COUNT
        ; // registration
        ; (add_component_range, (C, 1, 1)) // Preparation
        ; (get_component_range, (C, 1, 1)) // Benchmark
    );

    group.finish();
}

pub fn get_pair(criterion: &mut Criterion) {
    let mut group = criterion.benchmark_group("flecs");

    group.bench_function("get_pair", |bencher| {
        let world = World::new();
        let entities = create_entities(&world, ENTITY_COUNT as usize);

        for entity in &entities {
            entity.add((C1::id(), T1::id()));
        }
        bencher.iter_custom(|iters| {
            let start = Instant::now();
            for _ in 0..iters {
                for e in &entities {
                    e.get::<&(C1, T1)>(|c1| {
                        core::hint::black_box(c1);
                    });
                }
            }
            let elapsed = start.elapsed();
            elapsed / ENTITY_COUNT
        });
        reset_world_arrays(&world);
    });

    group.finish();
}

pub fn get_mut(criterion: &mut Criterion) {
    let mut group = criterion.benchmark_group("flecs");

    bench_loop_entities!(
        group,
        "get_mut_not_found",
        ENTITY_COUNT
        ; (register_component_range, (C, 1, 2)) // Registration so it doesn't affect the benchmark
        ;  (add_component_range, (C, 1, 1)) // Preparation
        ; (get_mut_component_range, (C, 2, 2)) // Benchmark
    );

    bench_loop_entities!(
        group,
        "get_mut",
        ENTITY_COUNT
        ;
        ; (add_component_range, (C, 1, 1)) // Preparation
        ; (get_mut_component_range, (C, 1, 1)) // Benchmark
    );

    bench_loop_entities!(
        group,
        "get_mut_sparse",
        ENTITY_COUNT
        ; (set_components_sparse, (C,1,1))
        ; (add_component_range, (C, 1, 1)) // Preparation
        ; (get_mut_component_range, (C, 1, 1)) // Benchmark
    );

    bench_loop_entities!(
        group,
        "get_mut_dont_fragment",
        ENTITY_COUNT
        ; (set_components_sparse, (C,1,1)),(set_components_dont_fragment, (C,1,1))
        ; (add_component_range, (C, 1, 1)) // Preparation
        ; (get_mut_component_range, (C, 1, 1)) // Benchmark
    );

    group.finish();
}

pub fn get_sparse(criterion: &mut Criterion) {
    let mut group = criterion.benchmark_group("flecs");

    bench_loop_entities!(
        group,
        "get_sparse_not_found",
        ENTITY_COUNT
        ; (set_components_sparse, (C, 1, 1)), (set_components_dont_fragment, (C, 1, 1)) // Registration
        ; // no preparation, entities have nothing added
        ; (get_component_range, (C, 1, 1)) // Benchmark
    );

    bench_loop_entities!(
        group,
        "get_sparse",
        ENTITY_COUNT
        ; (set_components_sparse, (C, 1, 1)), (set_components_dont_fragment, (C, 1, 1)) // Registration
        ; (add_component_range, (C, 1, 1)) // Preparation
        ; (get_component_range, (C, 1, 1)) // Benchmark
    );

    group.finish();
}

pub fn get_inherited_w_depth(criterion: &mut Criterion) {
    let mut group = criterion.benchmark_group("flecs");

    for depth in [1, 2, 16] {
        group.bench_function(format!("get_inherited_w_depth_{depth}"), |bencher| {
            let world = World::new();
            let entities = create_entities(&world, ENTITY_COUNT as usize);

            set_components_inheritable!(&world, C, 1, 1);

            let mut base = world.entity().add(C1::id());
            for _ in 0..depth {
                base = world.entity().is_a(base);
            }

            for entity in &entities {
                entity.is_a(base);
            }

            bencher.iter_custom(|iters| {
                let start = Instant::now();
                for _ in 0..iters {
                    for entity in &entities {
                        entity.get::<&C1>(|c1| {
                            core::hint::black_box(c1);
                        });
                    }
                }
                let elapsed = start.elapsed();
                elapsed / ENTITY_COUNT
            });
        });
    }

    group.finish();
}

pub fn get_target(criterion: &mut Criterion) {
    let mut group = criterion.benchmark_group("flecs");

    group.bench_function("get_target_not_found", |bencher| {
        let world = World::new();
        let mut entities = create_entities(&world, ENTITY_COUNT as usize);
        let rel = world.entity();

        bencher.iter_custom(|iters| {
            let start = Instant::now();
            for _ in 0..iters {
                for entity in &entities {
                    let ok = entity.target(rel, 0);
                    core::hint::black_box(ok);
                }
            }
            let elapsed = start.elapsed();
            elapsed / ENTITY_COUNT
        });
    });

    for exclusive in [false, true] {
        for fragment in [true, false] {
            if exclusive && fragment {
                continue; // Skip this combination as it doesn't make sense
            }
            let name = if !exclusive && fragment {
                "get_target"
            } else if !exclusive && !fragment {
                "get_target_dont_fragment"
            } else
            // exclusive && !fragment
            {
                "get_target_dont_fragment_exclusive"
            };
            group.bench_function(name, |bencher| {
                let world = World::new();
                let mut entities = create_entities(&world, ENTITY_COUNT as usize);
                let tgts = create_ids(&world, 8, 0, false, false, true);
                let rel = world.entity();

                if exclusive {
                    rel.add_trait::<flecs::Exclusive>();
                }
                if !fragment {
                    rel.add_trait::<flecs::DontFragment>();
                }

                for (i, entity) in entities.iter().enumerate() {
                    entity.add((rel, tgts[i % 8]));
                }

                bencher.iter_custom(|iters| {
                    let start = Instant::now();
                    for _ in 0..iters {
                        for entity in &entities {
                            let ok = entity.target(rel, 0);
                            core::hint::black_box(ok);
                        }
                    }
                    let elapsed = start.elapsed();
                    elapsed / ENTITY_COUNT
                });
            });
        }
    }

    group.finish();
}

pub fn get_parent(criterion: &mut Criterion) {
    let mut group = criterion.benchmark_group("flecs");

    group.bench_function("get_parent_not_found", |bencher| {
        let world = World::new();
        let mut entities = create_entities(&world, ENTITY_COUNT as usize);
        let rel = world.entity();

        bencher.iter_custom(|iters| {
            let start = Instant::now();
            for _ in 0..iters {
                for entity in &entities {
                    let ok = entity.parent();
                    core::hint::black_box(ok);
                }
            }
            let elapsed = start.elapsed();
            elapsed / ENTITY_COUNT
        });
    });

    group.bench_function("get_parent", |bencher| {
        let world = World::new();
        let mut entities = create_entities(&world, ENTITY_COUNT as usize);
        let tgts = create_ids(&world, 8, 0, false, false, true);

        for (i, entity) in entities.iter().enumerate() {
            entity.child_of(tgts[i % 8]);
        }

        bencher.iter_custom(|iters| {
            let start = Instant::now();
            for _ in 0..iters {
                for entity in &entities {
                    let ok = entity.parent();
                    core::hint::black_box(ok);
                }
            }
            let elapsed = start.elapsed();
            elapsed / ENTITY_COUNT
        });
    });

    group.finish();
}

// // pub fn ensure_mut_components_found(criterion: &mut Criterion) {
// //     let mut group = criterion.benchmark_group("flecs_ensure_mut_components_found");

// //     bench_loop_entities!(
// //         group,
// //         "1",
// //         ENTITY_COUNT
// //         ; (register_component_range, (C, 1, 1)) // Registration so it doesn't affect the benchmark
// //         ; (add_component_range, (C, 1, 1)) // Preparation
// //         ; (ensure_mut_component_range, (C, 1, 1)) // Benchmark
// //         ; (reset_component_range, (C, 1, 1)) // Cleanup
// //     );

// //     bench_loop_entities!(
// //         group,
// //         "2",
// //         ENTITY_COUNT
// //         ; (register_component_range, (C, 1, 2)) // Registration so it doesn't affect the benchmark
// //         ; (add_component_range, (C, 1, 2)) // Preparation
// //         ; (ensure_mut_component_range, (C, 1, 2)) // Benchmark
// //         ; (reset_component_range, (C, 1, 2)) // Cleanup
// //     );

// //     bench_loop_entities!(
// //         group,
// //         "16",
// //         ENTITY_COUNT
// //         ; (register_component_range, (C, 1, 16)) // Registration so it doesn't affect the benchmark
// //         ; (add_component_range, (C, 1, 16)) // Preparation
// //         ; (ensure_mut_component_range, (C, 1, 16)) // Benchmark
// //         ; (reset_component_range, (C, 1, 16)) // Cleanup
// //     );

// //     bench_loop_entities!(
// //         group,
// //         "64",
// //         ENTITY_COUNT
// //         ; (register_component_range, (C, 1, 64)) // Registration so it doesn't affect the benchmark
// //         ; (add_component_range, (C, 1, 64)) // Preparation
// //         ; (ensure_mut_component_range, (C, 1, 64)) // Benchmark
// //         ; (reset_component_range, (C, 1, 64)) // Cleanup
// //     );

// //     group.finish();
// // }

// // pub fn ensure_mut_components_found_cmd(criterion: &mut Criterion) {
// //     let mut group = criterion.benchmark_group("flecs_ensure_mut_components_found_cmd");

// //     bench_loop_entities!(
// //         group,
// //         "1",
// //         ENTITY_COUNT
// //         ; (register_component_range, (C, 1, 1)) // Registration so it doesn't affect the benchmark
// //         ; (add_component_range, (C, 1, 1)) // Preparation
// //         ; (ensure_mut_component_range_cmd, (C, 1, 1)) // Benchmark
// //         ; (reset_component_range, (C, 1, 1)) // Cleanup
// //     );

// //     bench_loop_entities!(
// //         group,
// //         "2",
// //         ENTITY_COUNT
// //         ; (register_component_range, (C, 1, 2)) // Registration so it doesn't affect the benchmark
// //         ; (add_component_range, (C, 1, 2)) // Preparation
// //         ; (ensure_mut_component_range_cmd, (C, 1, 2)) // Benchmark
// //         ; (reset_component_range, (C, 1, 2)) // Cleanup
// //     );

// //     bench_loop_entities!(
// //         group,
// //         "16",
// //         ENTITY_COUNT
// //         ; (register_component_range, (C, 1, 16)) // Registration so it doesn't affect the benchmark
// //         ; (add_component_range, (C, 1, 16)) // Preparation
// //         ; (ensure_mut_component_range_cmd, (C, 1, 16)) // Benchmark
// //         ; (reset_component_range, (C, 1, 16)) // Cleanup
// //     );

// //     bench_loop_entities!(
// //         group,
// //         "64",
// //         ENTITY_COUNT
// //         ; (register_component_range, (C, 1, 64)) // Registration so it doesn't affect the benchmark
// //         ; (add_component_range, (C, 1, 64)) // Preparation
// //         ; (ensure_mut_component_range_cmd, (C, 1, 64)) // Benchmark
// //         ; (reset_component_range, (C, 1, 64)) // Cleanup
// //     );

// //     group.finish();
// // }

// // pub fn ensure_mut_not_found_and_remove(criterion: &mut Criterion) {
// //     let mut group = criterion.benchmark_group("flecs_ensure_mut_not_found_and_remove");

// //     bench_loop_entities!(
// //         group,
// //         "1",
// //         ENTITY_COUNT
// //         ; (register_component_range, (C, 1, 1)) // Registration so it doesn't affect the benchmark
// //         ;  // Preparation
// //         ; (ensure_mut_component_range_cmd, (C, 1, 1)), (remove_component_range, (C, 1, 1))
// //         ; (reset_component_range, (C, 1, 1))
// //     );

// //     bench_loop_entities!(
// //         group,
// //         "2",
// //         ENTITY_COUNT
// //         ; (register_component_range, (C, 1, 2)) // Registration so it doesn't affect the benchmark
// //         ; // Preparation
// //         ; (ensure_mut_component_range_cmd, (C, 1, 2)), (remove_component_range, (C, 1, 2))
// //         ; (reset_component_range, (C, 1, 2))
// //     );

// //     bench_loop_entities!(
// //         group,
// //         "16",
// //         ENTITY_COUNT
// //         ; (register_component_range, (C, 1, 16)) // Registration so it doesn't affect the benchmark
// //         ; // Preparation
// //         ; (ensure_mut_component_range_cmd, (C, 1, 16)), (remove_component_range, (C, 1, 16))
// //         ; (reset_component_range, (C, 1, 16))
// //     );

// //     bench_loop_entities!(
// //         group,
// //         "64",
// //         ENTITY_COUNT
// //         ; (register_component_range, (C, 1, 64)) // Registration so it doesn't affect the benchmark
// //         ; // Preparation
// //         ; (ensure_mut_component_range_cmd, (C, 1, 64)), (remove_component_range, (C, 1, 64))
// //         ; (reset_component_range, (C, 1, 64))
// //     );

// //     group.finish();
// // }

// // pub fn ensure_mut_not_found_and_remove_cmd(criterion: &mut Criterion) {
// //     let mut group = criterion.benchmark_group("flecs_ensure_mut_not_found_and_remove_cmd");

// //     bench_loop_entities!(
// //         group,
// //         "1",
// //         ENTITY_COUNT
// //         ; (register_component_range, (C, 1, 1)) // Registration so it doesn't affect the benchmark
// //         ;  // Preparation
// //         ; (ensure_mut_component_range_cmd, (C, 1, 1)), (remove_component_range_cmd, (C, 1, 1))
// //         ; (reset_component_range, (C, 1, 1))
// //     );

// //     bench_loop_entities!(
// //         group,
// //         "2",
// //         ENTITY_COUNT
// //         ; (register_component_range, (C, 1, 2)) // Registration so it doesn't affect the benchmark
// //         ; // Preparation
// //         ; (ensure_mut_component_range_cmd, (C, 1, 2)), (remove_component_range_cmd, (C, 1, 2))
// //         ; (reset_component_range, (C, 1, 2))
// //     );

// //     bench_loop_entities!(
// //         group,
// //         "16",
// //         ENTITY_COUNT
// //         ; (register_component_range, (C, 1, 16)) // Registration so it doesn't affect the benchmark
// //         ; // Preparation
// //         ; (ensure_mut_component_range_cmd, (C, 1, 16)), (remove_component_range_cmd, (C, 1, 16))
// //         ; (reset_component_range, (C, 1, 16))
// //     );

// //     bench_loop_entities!(
// //         group,
// //         "64",
// //         ENTITY_COUNT
// //         ; (register_component_range, (C, 1, 64)) // Registration so it doesn't affect the benchmark
// //         ; // Preparation
// //         ; (ensure_mut_component_range_cmd, (C, 1, 64)), (remove_component_range_cmd, (C, 1, 64))
// //         ; (reset_component_range, (C, 1, 64))
// //     );

// //     group.finish();
// // }

// // #[allow(unused)]
// // pub fn c_get_components(criterion: &mut Criterion) {
// //     let mut group = criterion.benchmark_group("flecs_c_get_component");
// //     let counts = vec![1, 2, 16, 64];

// //     group.bench_function("not_empty_entity", |bencher| {
// //         unsafe {
// //             let world = ecs_mini();
// //             let entities = create_ids(world, ENTITY_COUNT as i32, 0, false);
// //             let ids = create_ids(world, 1, 4, true);

// //             bencher.iter_custom(|iters| {
// //                 let start = Instant::now();
// //                 for _ in 0..iters {
// //                     for entity in entities.iter().take(ENTITY_COUNT as usize) {
// //                         ecs_get_id(world, *entity, ids[0]);
// //                     }
// //                 }
// //                 start.elapsed() / ENTITY_COUNT // Return the total time per entity
// //             });
// //             ecs_fini(world);
// //         }
// //     });

// //     group.bench_function("not_entity", |bencher| {
// //         unsafe {
// //             let world = ecs_mini();
// //             let entities = create_ids(world, ENTITY_COUNT as i32, 0, false);
// //             let ids = create_ids(world, 2, 4, true);

// //             for entity in &entities {
// //                 ecs_add_id(world, *entity, ids[0]);
// //             }

// //             bencher.iter_custom(|iters| {
// //                 let start = Instant::now();
// //                 for _ in 0..iters {
// //                     for entity in entities.iter().take(ENTITY_COUNT as usize) {
// //                         ecs_get_id(world, *entity, ids[1]);
// //                     }
// //                 }
// //                 start.elapsed() / ENTITY_COUNT // Return the total time per entity
// //             });
// //             ecs_fini(world);
// //         }
// //     });

// //     for count in counts {
// //         group.bench_function(count.to_string(), |bencher| unsafe {
// //             let world = ecs_mini();
// //             let entities = create_ids(world, ENTITY_COUNT as i32, 0, false);
// //             let ids = create_ids(world, count, 4, true);

// //             for entity in entities.iter().take(ENTITY_COUNT as usize) {
// //                 for id in ids.iter().take(count as usize) {
// //                     ecs_add_id(world, *entity, *id);
// //                 }
// //             }

// //             bencher.iter_custom(|iters| {
// //                 let start = Instant::now();
// //                 for _ in 0..iters {
// //                     for entity in &entities {
// //                         for id in &ids {
// //                             ecs_get_id(world, *entity, *id);
// //                         }
// //                     }
// //                 }
// //                 start.elapsed() / (ENTITY_COUNT * count as u32) // Return the total time per entity per operation
// //             });
// //             ecs_fini(world);
// //         });
// //     }

// //     group.finish();
// // }
