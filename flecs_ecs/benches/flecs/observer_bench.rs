use crate::common_bench::*;

#[derive(Component)]
pub struct Event;

pub fn emit(criterion: &mut Criterion) {
    let mut group = criterion.benchmark_group("flecs");

    let term_count = 8;

    for observer_count in [0, 1, 10, 50] {
        group.bench_function(format!("emit_{observer_count}_observers"), |bencher| {
            reset_srand();
            let world = World::new();
            let ids = create_ids(&world, 8, 0, true, false, true);
            let mut entities: Vec<EntityView<'_>> = Vec::with_capacity(ENTITY_COUNT as usize);

            for i in 0..ENTITY_COUNT {
                let e = world.entity();
                unsafe { e.add_id_unchecked(ids[0]) };

                for id in &ids[1..] {
                    if flip_coin() {
                        unsafe { e.add_id_unchecked(*id) };
                    }
                }

                entities.push(e);
            }

            for _ in 0..observer_count {
                let mut o = world.observer::<Event, ()>();

                for id in &ids[..term_count] {
                    o.with(*id).self_();
                }

                o.run(|_| {});
            }

            bencher.iter_custom(|iters| {
                let start = Instant::now();
                for _ in 0..iters {
                    for entity in &entities {
                        world
                            .event::<Event>()
                            .add(ids[0])
                            .entity(*entity)
                            .emit(&Event);
                    }
                }
                let elapsed = start.elapsed();
                elapsed / ENTITY_COUNT //time average per entity operation
            });
        });
    }

    for term_count in [2, 4, 8] {
        let observer_count = 50;
        group.bench_function(
            format!("emit_multi_{term_count}_100_observers"),
            |bencher| {
                reset_srand();
                let world = World::new();
                let ids = create_ids(&world, 8, 0, true, false, true);
                let mut entities: Vec<EntityView<'_>> = Vec::with_capacity(ENTITY_COUNT as usize);

                for i in 0..ENTITY_COUNT {
                    let e = world.entity();
                    unsafe { e.add_id_unchecked(ids[0]) };

                    for id in &ids[1..] {
                        if flip_coin() {
                            unsafe { e.add_id_unchecked(*id) };
                        }
                    }

                    entities.push(e);
                }

                for _ in 0..observer_count {
                    let mut o = world.observer::<Event, ()>();

                    for id in &ids[..term_count] {
                        o.with(*id).self_();
                    }

                    o.run(|_| {});
                }

                bencher.iter_custom(|iters| {
                    let start = Instant::now();
                    for _ in 0..iters {
                        for entity in &entities {
                            world
                                .event::<Event>()
                                .add(ids[0])
                                .entity(*entity)
                                .emit(&Event);
                        }
                    }
                    let elapsed = start.elapsed();
                    elapsed / ENTITY_COUNT //time average per entity operation
                });
            },
        );
    }
}

pub fn emit_propagate(criterion: &mut Criterion) {
    let mut group = criterion.benchmark_group("flecs");

    for depth in [0, 10, 100] {
        group.bench_function(format!("emit_propagate_depth_{depth}"), |bencher| {
            let world = World::new();
            set_components_inheritable!(&world, T1, 1, 1);
            let root = world.entity().add(T1::id());
            let mut cur = root;
            let root_table = root.table().unwrap();

            for _ in 0..depth {
                cur = world.entity().child_of(cur);
            }

            world
                .observer::<Event, ()>()
                .with(&T1::id())
                .up()
                .parent()
                .run(|_| {});

            bencher.iter_custom(|iters| {
                let start = Instant::now();
                for _ in 0..iters {
                    world
                        .event::<Event>()
                        .add(T1::id())
                        .table(root_table, 0, 0)
                        .emit(&Event);
                }
                start.elapsed()
            });
        });
    }
}

pub fn emit_forward(criterion: &mut Criterion) {
    let mut group = criterion.benchmark_group("flecs");

    for id_count in [1, 16] {
        for depth in [1, 1000] {
            group.bench_function(
                format!("emit_forward_ids_{id_count}_depth_{depth}"),
                |bencher| {
                    let world = World::new();
                    let ids = create_ids(&world, id_count, 0, true, false, true);
                    let root = world.entity();

                    let mut cur = root;

                    for id in &ids {
                        world
                            .component_untyped_from(*id)
                            .add_trait::<(flecs::OnInstantiate, flecs::Inherit)>();

                        world
                            .observer::<flecs::OnAdd, ()>()
                            .with(*id)
                            .up()
                            .add_event(flecs::OnRemove)
                            .run(|_| {});

                        unsafe { root.add_id_unchecked(*id) };
                    }

                    for _ in 0..depth {
                        cur = world.entity().child_of(cur);
                    }

                    let e = world.entity();
                    bencher.iter_custom(|iters| {
                        let start = Instant::now();
                        for _ in 0..iters {
                            e.child_of(cur);
                            e.remove((flecs::ChildOf::id(), cur));
                        }
                        start.elapsed()
                    });
                },
            );
        }
    }
}

pub fn modified(criterion: &mut Criterion) {
    let mut group = criterion.benchmark_group("flecs");

    for observer_count in [0, 1, 10, 100] {
        group.bench_function(format!("modified_{observer_count}_observers"), |bencher| {
            let world = World::new();
            let e = world.entity().set(C1(0));

            for _ in 0..observer_count {
                world.observer::<flecs::OnSet, &C1>().self_().run(|_| {});
            }

            let id = *world.entity_from::<C1>();
            bencher.iter_custom(|iters| {
                let start = Instant::now();
                for _ in 0..iters {
                    e.modified(id);
                }
                start.elapsed()
            });
        });
    }
}
