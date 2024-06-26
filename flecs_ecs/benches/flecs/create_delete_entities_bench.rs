use crate::common_bench::*;

pub fn create_delete_entities(criterion: &mut Criterion) {
    let mut group = criterion.benchmark_group("flecs_create_delete_entities");

    group.bench_function("empty", |bencher| {
        let world = World::new();

        bencher.iter_custom(|iters| {
            let start = Instant::now();
            for _ in 0..iters {
                for _ in 0..ENTITY_COUNT {
                    let entity = world.entity();
                    entity.destruct();
                }
            }
            let elapsed = start.elapsed();
            elapsed / (ENTITY_COUNT * 2) //time average per entity operation
        });
    });

    group.bench_function("empty_named", |bencher| {
        let world = World::new();

        bencher.iter_custom(|iters| {
            let start = Instant::now();
            for _ in 0..iters {
                for _ in 0..ENTITY_COUNT {
                    let entity = world.entity_named("hello");
                    entity.destruct();
                }
            }
            let elapsed = start.elapsed();
            elapsed / (ENTITY_COUNT * 2) //time average per entity operation
        });
    });

    // tags
    bench_create_delete_entity!(group, "tag_1", ENTITY_COUNT, T, 1, 1, add_component_range);
    bench_create_delete_entity!(group, "tag_2", ENTITY_COUNT, T, 1, 2, add_component_range);
    bench_create_delete_entity!(group, "tag_16", ENTITY_COUNT, T, 1, 16, add_component_range);
    bench_create_delete_entity!(group, "tag_64", ENTITY_COUNT, T, 1, 64, add_component_range);
    // components
    bench_create_delete_entity!(
        group,
        "component_1",
        ENTITY_COUNT,
        C,
        1,
        1,
        set_component_range
    );
    bench_create_delete_entity!(
        group,
        "component_2",
        ENTITY_COUNT,
        C,
        1,
        2,
        set_component_range
    );
    bench_create_delete_entity!(
        group,
        "component_16",
        ENTITY_COUNT,
        C,
        1,
        16,
        set_component_range
    );
    bench_create_delete_entity!(
        group,
        "component_64",
        ENTITY_COUNT,
        C,
        1,
        64,
        set_component_range
    );

    group.finish();
}

pub fn create_delete_entities_cmd(criterion: &mut Criterion) {
    let mut group = criterion.benchmark_group("flecs_create_delete_entities_cmd");

    group.bench_function("empty", |bencher| {
        let world = World::new();

        bencher.iter_custom(|iters| {
            let start = Instant::now();
            for _ in 0..iters {
                world.defer_begin();
                for _ in 0..ENTITY_COUNT {
                    let entity = world.entity();
                    entity.destruct();
                }
                world.defer_end();
            }
            let elapsed = start.elapsed();
            elapsed / 2 //time average per entity operation
        });
    });

    group.bench_function("empty_named", |bencher| {
        let world = World::new();

        bencher.iter_custom(|iters| {
            let start = Instant::now();
            for _ in 0..iters {
                world.defer_begin();
                for _ in 0..ENTITY_COUNT {
                    let entity = world.entity_named("hello");
                    entity.destruct();
                }
                world.defer_end();
            }
            let elapsed = start.elapsed();
            elapsed / (ENTITY_COUNT * 2) //time average per entity operation
        });
    });

    //tags
    bench_create_delete_entity_cmd!(group, "tag_1", ENTITY_COUNT, T, 1, 1, add_component_range);
    bench_create_delete_entity_cmd!(group, "tag_2", ENTITY_COUNT, T, 1, 2, add_component_range);
    bench_create_delete_entity_cmd!(group, "tag_16", ENTITY_COUNT, T, 1, 16, add_component_range);
    bench_create_delete_entity_cmd!(group, "tag_64", ENTITY_COUNT, T, 1, 64, add_component_range);
    // components
    bench_create_delete_entity_cmd!(
        group,
        "component_1",
        ENTITY_COUNT,
        C,
        1,
        1,
        set_component_range
    );
    bench_create_delete_entity_cmd!(
        group,
        "component_2",
        ENTITY_COUNT,
        C,
        1,
        2,
        set_component_range
    );
    bench_create_delete_entity_cmd!(
        group,
        "component_16",
        ENTITY_COUNT,
        C,
        1,
        16,
        set_component_range
    );
    bench_create_delete_entity_cmd!(
        group,
        "component_64",
        ENTITY_COUNT,
        C,
        1,
        64,
        set_component_range
    );

    group.finish();
}

pub fn create_delete_entities_w_tree(criterion: &mut Criterion) {
    let mut group = criterion.benchmark_group("flecs_create_delete_entities_w_tree");

    for width in [1, 10, 100, 1000] {
        for depth in [1, 10, 100, 1000] {
            group.bench_function(format!("w{}_d{}", width, depth), |bencher| {
                let world = World::new();

                bencher.iter_custom(|iters| {
                    let start = Instant::now();
                    for _ in 0..iters {
                        let root = world.entity();
                        let mut cur = root;

                        for _ in 0..depth {
                            for _ in 0..width - 1 {
                                let child = world.entity();
                                child.child_of_id(cur);
                            }
                            let child = world.entity();
                            child.child_of_id(cur);
                            cur = child;
                        }

                        root.destruct();
                    }
                    let elapsed = start.elapsed();
                    elapsed / width // calculate overhead per child
                });
            });
        }
    }
    group.finish();
}
