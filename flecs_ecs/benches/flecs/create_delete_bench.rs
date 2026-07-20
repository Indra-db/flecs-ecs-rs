use crate::common_bench::*;
use flecs_ecs::sys;

macro_rules! bench_instantiate_root_components {
    ($group:expr, $label:expr, $start:expr, $end:expr, $inherit:expr) => {{
        $group.bench_function($label, |bencher| {
            let world = World::new();

            register_component_range!(world, C, $start, $end);
            if $inherit {
                set_components_inheritable!(&world, C, $start, $end);
            }

            let root = world.prefab();
            set_component_range!(&world, root, C, $start, $end);

            bencher.iter_custom(|iters| {
                let start = Instant::now();
                for _ in 0..iters {
                    let e = world.entity().is_a(root);
                    e.destruct();
                }
                start.elapsed() / 2
            });

            reset_world_arrays(&world);
        });
    }};
}

pub fn entity_new_delete(criterion: &mut Criterion) {
    let mut group = criterion.benchmark_group("flecs");

    group.bench_function("entity_init_delete", |bencher| {
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
}

pub fn entity_new_w_name_delete(criterion: &mut Criterion) {
    let mut group = criterion.benchmark_group("flecs");

    group.bench_function("entity_init_w_name_delete", |bencher| {
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
}

pub fn create_delete(criterion: &mut Criterion) {
    let mut group = criterion.benchmark_group("flecs");

    group.bench_function("create_delete_empty", |bencher| {
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
            elapsed / (ENTITY_COUNT * 2)
        });
    });

    // tags
    bench_create_delete_entity!(group, "tag_1", ENTITY_COUNT, T, 1, 1, add_component_range);
    bench_create_delete_entity!(group, "tag_2", ENTITY_COUNT, T, 1, 2, add_component_range);
    bench_create_delete_entity!(group, "tag_16", ENTITY_COUNT, T, 1, 16, add_component_range);

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

    group.finish();
}

pub fn create_delete_tree(criterion: &mut Criterion) {
    let mut group = criterion.benchmark_group("flecs");

    for width in [1, 10, 100] {
        for depth in [1, 10, 100] {
            group.bench_function(format!("create_delete_tree_w{width}_d{depth}"), |bencher| {
                let world = World::new();

                bencher.iter_custom(|iters| {
                    let start = Instant::now();
                    for _ in 0..iters {
                        let root = world.entity();
                        let mut cur = root;

                        for _ in 0..depth {
                            for _ in 0..width - 1 {
                                world.entity_child_of(cur);
                            }
                            cur = world.entity_child_of(cur);
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

pub fn instantiate_delete_tree(criterion: &mut Criterion) {
    let mut group = criterion.benchmark_group("flecs");

    for width in [1, 5, 10, 50] {
        for depth in [1, 2] {
            group.bench_function(
                format!("instantiate_delete_tree_w{width}_d{depth}"),
                |bencher| {
                    let world = World::new();

                    //create 4 components of size 4 which fragment
                    let ids = create_ids(&world, 3, 4, true, false, true);

                    // In a prefab hierarchy children usually don't have the same archetype, so
                    // give each child a different tag.
                    let child_tags = create_ids(&world, width, 0, true, false, true);

                    let root = world.prefab();
                    let mut cur = root;

                    for id in &ids {
                        // unsafe: we're loosely adding component ids created from non-rust types
                        // this part is not being benched, so doesn't matter.
                        // by not using rust types, we can scale this test better with different parameters
                        // without using a bunch of macro magic
                        unsafe { root.add_id_unchecked(*id) };
                    }

                    for d in 0..depth {
                        for child_tag in &child_tags {
                            let e = world.entity_child_of(cur);

                            for id in &ids {
                                unsafe { e.add_id_unchecked(*id) };
                            }
                            e.add(*child_tag);
                        }
                        let e = world.entity_child_of(cur);
                        for id in &ids {
                            unsafe { e.add_id_unchecked(*id) };
                        }
                        cur = e;
                    }

                    bencher.iter_custom(|iters| {
                        let start = Instant::now();
                        for _ in 0..iters {
                            let e = world.entity().is_a(root);
                            e.destruct();
                        }
                        start.elapsed() / 2
                    });
                },
            );
        }
    }

    group.bench_function("instantiate_override_0_components", |bencher| {
        let world = World::new();
        let root = world.prefab();

        bencher.iter_custom(|iters| {
            let start = Instant::now();
            for _ in 0..iters {
                let e = world.entity().is_a(root);
                e.destruct();
            }
            start.elapsed() / 2
        });
    });

    bench_instantiate_root_components!(group, "instantiate_override_1_components", 1, 1, false);
    bench_instantiate_root_components!(group, "instantiate_override_8_components", 1, 8, false);
    bench_instantiate_root_components!(group, "instantiate_override_32_components", 1, 32, false);
    bench_instantiate_root_components!(group, "instantiate_override_64_components", 1, 64, false);

    group.bench_function("instantiate_inherit_0_components", |bencher| {
        let world = World::new();
        let root = world.prefab();

        bencher.iter_custom(|iters| {
            let start = Instant::now();
            for _ in 0..iters {
                let e = world.entity().is_a(root);
                e.destruct();
            }
            start.elapsed() / 2
        });
    });

    bench_instantiate_root_components!(group, "instantiate_inherit_1_components", 1, 1, true);
    bench_instantiate_root_components!(group, "instantiate_inherit_8_components", 1, 8, true);
    bench_instantiate_root_components!(group, "instantiate_inherit_32_components", 1, 32, true);
    bench_instantiate_root_components!(group, "instantiate_inherit_64_components", 1, 64, true);

    bench_instantiate_root_components!(group, "instantiate_tree_w0_d0", 1, 3, false);

    group.finish();
}

pub fn create_w_add_in_observer(criterion: &mut Criterion) {
    let mut group = criterion.benchmark_group("flecs");

    for entity_count in [100, 1000, 10_000, 50_000] {
        group.bench_function(
            format!("create_w_add_in_observer_entities_{entity_count}"),
            |bencher| {
                let world = World::new();

                // Create entities
                let entities: Vec<EntityView> = (0..entity_count).map(|_| world.entity()).collect();

                // Create observer
                let add_id = world.entity().id();

                // TODO: Should I do while it.next() + for it.iter()?
                world
                    .observer::<flecs::OnAdd, ()>()
                    .with(T1::id())
                    .run(move |mut it| {
                        while it.next() {
                            for i in it.iter() {
                                it.get_entity(i).unwrap().add(add_id);
                            }
                        }
                    });

                bencher.iter_custom(|iters| {
                    let start = Instant::now();
                    for _ in 0..iters {
                        for entity in &entities {
                            let id = T1::id();
                            entity.add(id);
                            entity.remove(id);
                        }
                    }
                    let elapsed = start.elapsed();
                    elapsed / (entity_count)
                });
            },
        );
    }

    group.finish();
}

pub fn create_children_w_reachable(criterion: &mut Criterion) {
    let mut group = criterion.benchmark_group("flecs");

    group.bench_function("create_children_w_reachable", |bencher| {
        let world = World::new();
        world.component::<T1>();
        let id = T1::id();

        bencher.iter_custom(|iters| {
            let start = Instant::now();
            for _ in 0..iters {
                let parent = world.entity().add(id);

                for _ in 0..ENTITY_COUNT {
                    let p1 = world.entity_child_of(parent);
                    let p2 = world.entity_child_of(p1);
                    world.entity_child_of(p1);
                    world.entity_child_of(p2);
                }

                parent.destruct();
            }
            let elapsed = start.elapsed();
            elapsed / (ENTITY_COUNT * 2)
        });
    });

    group.finish();
}

#[allow(unused)]
pub fn c_create_delete_tree(criterion: &mut Criterion) {
    let mut group = criterion.benchmark_group("flecs_c");
    for width in [1, 10, 100] {
        for depth in [1, 10, 100] {
            group.bench_function(format!("create_delete_tree_w{width}_d{depth}"), |bencher| {
                unsafe {
                    let world = sys::ecs_init();

                    bencher.iter_custom(|iters| {
                        let start = Instant::now();
                        for _ in 0..iters {
                            let root = sys::ecs_new(world);
                            let mut cur = root;

                            for _ in 0..depth {
                                for _ in 0..width - 1 {
                                    sys::ecs_new_w_id(world, ecs_pair(sys::EcsChildOf, cur));
                                }
                                cur = sys::ecs_new_w_id(world, ecs_pair(sys::EcsChildOf, cur));
                            }

                            sys::ecs_delete(world, root);
                        }
                        start.elapsed() / width // Return the total time per entity
                    });
                    sys::ecs_fini(world);
                }
            });
        }
    }

    group.finish();
}
