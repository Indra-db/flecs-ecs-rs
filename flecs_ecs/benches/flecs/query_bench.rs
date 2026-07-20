use crate::{common_bench::*, create_typed_query};
use core::ffi::c_void;
use core::hint::black_box;
use flecs_ecs::core::term::internals;
use flecs_ecs::sys;

fn bench_query_iter_tags(
    group: &mut criterion::BenchmarkGroup<'_, criterion::measurement::WallTime>,
    name: &str,
    cache_kind: QueryCacheKind,
    id_count: usize,
    term_count: usize,
    sparse: bool,
    fragment: bool,
) {
    group.bench_function(format!("query_iter_{name}"), |bencher| {
        reset_srand();
        let world = World::new();

        let e = world.entity();

        let ids = get_component_ids!(&world, id_count, false, sparse, fragment);

        let mut query = world.query::<()>();

        query.set_cache_kind(cache_kind);

        for id in &ids[..term_count] {
            query.with(*id).self_().set_in();
        }

        let query_desc = internals::QueryConfig::query_desc_mut(&mut query);
        // Create 100 other queries to increase cache element fragmentation
        if cache_kind == QueryCacheKind::Auto {
            for i in 0..100 {
                unsafe { world.query_from_desc::<()>(query_desc).build() };
            }
        }

        let f = unsafe { world.query_from_desc::<()>(query_desc).build() };

        let mut result: u32 = 0;

        for _ in 0..QUERY_ENTITY_COUNT {
            let e = world.entity();
            for id in &ids {
                if flip_coin() {
                    unsafe { e.add_id_unchecked(*id) };
                }
            }
        }

        bencher.iter_custom(|iters| {
            let start = Instant::now();
            for _ in 0..iters {
                f.run(|mut it| {
                    while it.next() {
                        for i in it.iter() {
                            result += *it.entity_id(i) as u32;
                        }
                    }
                });
            }
            start.elapsed()
        });
        core::hint::black_box(result);
    });
}

fn bench_query_iter_components_setup(
    world: &World,
    cache_kind: QueryCacheKind,
    id_count: usize,
    term_count: usize,
    sparse: bool,
    fragment: bool,
) -> Query<()> {
    reset_srand();
    let e = world.entity();

    let ids = get_component_ids!(&world, id_count, true, sparse, fragment);

    let mut query = world.query::<()>();

    query.set_cache_kind(cache_kind);

    for id in &ids[..term_count] {
        query.with(*id).self_().set_in();
    }

    let f = query.build();

    // let query_desc = internals::QueryConfig::query_desc_mut(&mut query);
    // // Create 100 other queries to increase cache element fragmentation
    // if cache_kind == QueryCacheKind::Auto {
    //     for i in 0..100 {
    //         world.query_from_desc::<()>(query_desc).build();
    //     }
    // }

    //let mut f = world.query_from_desc::<()>(query_desc);

    for _ in 0..QUERY_ENTITY_COUNT {
        let e = world.entity();
        for (i, id) in ids.iter().enumerate() {
            if flip_coin() {
                unsafe {
                    sys::ecs_set_id(
                        world.ptr_mut(),
                        *e.id(),
                        *id,
                        4,
                        &((i + 1) as u32) as *const u32 as *const c_void,
                    );
                };
            }
        }
    }

    f
}

fn bench_query_iter_components_1_term(
    group: &mut criterion::BenchmarkGroup<'_, criterion::measurement::WallTime>,
    name: &str,
    cache_kind: QueryCacheKind,
    id_count: usize,
    sparse: bool,
    fragment: bool,
) {
    group.bench_function(format!("query_iter_{name}"), |bencher| {
        let world = World::new();

        let mut f =
            bench_query_iter_components_setup(&world, cache_kind, id_count, 1, sparse, fragment);

        let f = unsafe { core::mem::transmute::<Query<()>, Query<(&C1,)>>(f) };

        let mut result: u32 = 0;

        bencher.iter_custom(|iters| {
            let start = Instant::now();
            for _ in 0..iters {
                f.each_entity(|e, (c1,)| {
                    result += *e.id() as u32 + c1.0;
                });
            }
            start.elapsed()
        });

        core::hint::black_box(result);
    });
}

fn bench_query_iter_components_4_term(
    group: &mut criterion::BenchmarkGroup<'_, criterion::measurement::WallTime>,
    name: &str,
    cache_kind: QueryCacheKind,
    id_count: usize,
    sparse: bool,
    fragment: bool,
) {
    group.bench_function(format!("query_iter_{name}"), |bencher| {
        let world = World::new();

        let mut f =
            bench_query_iter_components_setup(&world, cache_kind, id_count, 4, sparse, fragment);

        let f = unsafe { core::mem::transmute::<Query<()>, Query<(&C1, &C2, &C3, &C4)>>(f) };

        let mut result: u32 = 0;

        bencher.iter_custom(|iters| {
            let start = Instant::now();
            for _ in 0..iters {
                result = 0;
                f.each_entity(|e, (c1, c2, c3, c4)| {
                    result += *e.id() as u32 + c1.0 + c2.0 + c3.0 + c4.0;
                });
            }
            start.elapsed()
        });

        core::hint::black_box(result);
    });
}

fn bench_query_iter_components_4_term_run(
    group: &mut criterion::BenchmarkGroup<'_, criterion::measurement::WallTime>,
    name: &str,
    cache_kind: QueryCacheKind,
    id_count: usize,
    sparse: bool,
    fragment: bool,
) {
    group.bench_function(format!("query_iter_{name}"), |bencher| {
        let world = World::new();

        let mut f =
            bench_query_iter_components_setup(&world, cache_kind, id_count, 4, sparse, fragment);

        let f = unsafe { core::mem::transmute::<Query<()>, Query<(&C1, &C2, &C3, &C4)>>(f) };

        let mut result: u32 = 0;

        bencher.iter_custom(|iters| {
            let start = Instant::now();
            for _ in 0..iters {
                f.run(|mut it| {
                    while it.next() {
                        let c1 = it.field::<C1>(0);
                        let c2 = it.field::<C2>(1);
                        let c3 = it.field::<C3>(2);
                        let c4 = it.field::<C4>(3);
                        for i in it.iter() {
                            result +=
                                *it.entity_id(i) as u32 + c1[i].0 + c2[i].0 + c3[i].0 + c4[i].0;
                        }
                    }
                });
            }
            start.elapsed()
        });

        core::hint::black_box(result);
    });
}

fn bench_query_iter_components_8_term(
    group: &mut criterion::BenchmarkGroup<'_, criterion::measurement::WallTime>,
    name: &str,
    cache_kind: QueryCacheKind,
    id_count: usize,
    sparse: bool,
    fragment: bool,
) {
    group.bench_function(format!("query_iter_{name}"), |bencher| {
        let world = World::new();

        let mut f =
            bench_query_iter_components_setup(&world, cache_kind, id_count, 8, sparse, fragment);

        // SAFETY: f was built via bench_query_iter_components_setup with 8 terms matching
        // C1..C8 in order, so the type-erased Query<()> layout matches Query<(&C1..&C8)>.
        let f = unsafe {
            core::mem::transmute::<Query<()>, Query<(&C1, &C2, &C3, &C4, &C5, &C6, &C7, &C8)>>(f)
        };

        let mut result: u32 = 0;

        bencher.iter_custom(|iters| {
            let start = Instant::now();
            for _ in 0..iters {
                f.each_entity(|e, (c1, c2, c3, c4, c5, c6, c7, c8)| {
                    result +=
                        *e.id() as u32 + c1.0 + c2.0 + c3.0 + c4.0 + c5.0 + c6.0 + c7.0 + c8.0;
                });
            }
            start.elapsed()
        });

        core::hint::black_box(result);
    });
}

struct BenchmarkConfig {
    name: &'static str,
    cache_kind: QueryCacheKind,
    id_count: usize,
    term_count: usize,
    sparse: bool,
    fragment: bool,
    component_benchmark: bool, // true for components, false for tags
}

pub fn query_iter(criterion: &mut Criterion) {
    let mut group = criterion.benchmark_group("flecs");

    #[rustfmt::skip]
    let benchmarks = [

    // Uncached query iter
       BenchmarkConfig { name: "uncach_6_tags_1_term", cache_kind: QueryCacheKind::Default, id_count: 6, term_count: 1, sparse: false, fragment: true, component_benchmark: false },
       BenchmarkConfig { name: "uncach_6_tags_4_terms", cache_kind: QueryCacheKind::Default, id_count: 6, term_count: 4, sparse: false, fragment: true, component_benchmark: false },
       BenchmarkConfig { name: "uncach_10_tags_1_term", cache_kind: QueryCacheKind::Default, id_count: 10, term_count: 1, sparse: false, fragment: true, component_benchmark: false },
       BenchmarkConfig { name: "uncach_10_tags_4_terms", cache_kind: QueryCacheKind::Default, id_count: 10, term_count: 4, sparse: false, fragment: true, component_benchmark: false },
       BenchmarkConfig { name: "uncach_10_tags_8_terms", cache_kind: QueryCacheKind::Default, id_count: 10, term_count: 8, sparse: false, fragment: true, component_benchmark: false },
       BenchmarkConfig { name: "uncach_6_comps_1_term", cache_kind: QueryCacheKind::Default, id_count: 6, term_count: 1, sparse: false, fragment: true, component_benchmark: true },
       BenchmarkConfig { name: "uncach_6_comps_4_terms", cache_kind: QueryCacheKind::Default, id_count: 6, term_count: 4, sparse: false, fragment: true, component_benchmark: true },
       BenchmarkConfig { name: "uncach_10_comps_1_term", cache_kind: QueryCacheKind::Default, id_count: 10, term_count: 1, sparse: false, fragment: true, component_benchmark: true },
       BenchmarkConfig { name: "uncach_10_comps_4_terms", cache_kind: QueryCacheKind::Default, id_count: 10, term_count: 4, sparse: false, fragment: true, component_benchmark: true },
       BenchmarkConfig { name: "uncach_10_comps_8_terms", cache_kind: QueryCacheKind::Default, id_count: 10, term_count: 8, sparse: false, fragment: true, component_benchmark: true },

       BenchmarkConfig { name: "uncach_10_sparse_tags_4_terms", cache_kind: QueryCacheKind::Default, id_count: 10, term_count: 4, sparse: true, fragment: true, component_benchmark: false },
       BenchmarkConfig { name: "uncach_10_sparse_comps_4_terms", cache_kind: QueryCacheKind::Default, id_count: 10, term_count: 4, sparse: true, fragment: true, component_benchmark: true },
       BenchmarkConfig { name: "uncach_10_nofrag_tags_4_terms", cache_kind: QueryCacheKind::Default, id_count: 10, term_count: 4, sparse: true, fragment: false, component_benchmark: false },
       BenchmarkConfig { name: "uncach_10_nofrag_comps_4_terms", cache_kind: QueryCacheKind::Default, id_count: 10, term_count: 4, sparse: true, fragment: false, component_benchmark: true },

    // Cached query iter
       BenchmarkConfig { name: "cached_6_tags_1_term", cache_kind: QueryCacheKind::Auto, id_count: 6, term_count: 1, sparse: false, fragment: true, component_benchmark: false },
       BenchmarkConfig { name: "cached_6_tags_4_terms", cache_kind: QueryCacheKind::Auto, id_count: 6, term_count: 4, sparse: false, fragment: true, component_benchmark: false },
       BenchmarkConfig { name: "cached_8_tags_1_term", cache_kind: QueryCacheKind::Auto, id_count: 8, term_count: 1, sparse: false, fragment: true, component_benchmark: false },
       BenchmarkConfig { name: "cached_8_tags_4_terms", cache_kind: QueryCacheKind::Auto, id_count: 8, term_count: 4, sparse: false, fragment: true, component_benchmark: false },
       BenchmarkConfig { name: "cached_10_tags_1_term", cache_kind: QueryCacheKind::Auto, id_count: 10, term_count: 1, sparse: false, fragment: true, component_benchmark: false },
       BenchmarkConfig { name: "cached_10_tags_4_terms", cache_kind: QueryCacheKind::Auto, id_count: 10, term_count: 4, sparse: false, fragment: true, component_benchmark: false },
       BenchmarkConfig { name: "cached_10_tags_8_terms", cache_kind: QueryCacheKind::Auto, id_count: 10, term_count: 8, sparse: false, fragment: true, component_benchmark: false },
       BenchmarkConfig { name: "cached_16_tags_1_term", cache_kind: QueryCacheKind::Auto, id_count: 16, term_count: 1, sparse: false, fragment: true, component_benchmark: false },
       BenchmarkConfig { name: "cached_16_tags_4_terms", cache_kind: QueryCacheKind::Auto, id_count: 16, term_count: 4, sparse: false, fragment: true, component_benchmark: false },
       BenchmarkConfig { name: "cached_16_tags_8_terms", cache_kind: QueryCacheKind::Auto, id_count: 16, term_count: 8, sparse: false, fragment: true, component_benchmark: false },

       BenchmarkConfig { name: "cached_6_comps_1_term", cache_kind: QueryCacheKind::Auto, id_count: 6, term_count: 1, sparse: false, fragment: true, component_benchmark: true },
       BenchmarkConfig { name: "cached_6_comps_4_terms", cache_kind: QueryCacheKind::Auto, id_count: 6, term_count: 4, sparse: false, fragment: true, component_benchmark: true },
       BenchmarkConfig { name: "cached_8_comps_1_term", cache_kind: QueryCacheKind::Auto, id_count: 8, term_count: 1, sparse: false, fragment: true, component_benchmark: true },
       BenchmarkConfig { name: "cached_8_comps_4_terms", cache_kind: QueryCacheKind::Auto, id_count: 8, term_count: 4, sparse: false, fragment: true, component_benchmark: true },
       BenchmarkConfig { name: "cached_10_comps_1_term", cache_kind: QueryCacheKind::Auto, id_count: 10, term_count: 1, sparse: false, fragment: true, component_benchmark: true },
       BenchmarkConfig { name: "cached_10_comps_4_terms", cache_kind: QueryCacheKind::Auto, id_count: 10, term_count: 4, sparse: false, fragment: true, component_benchmark: true },
       BenchmarkConfig { name: "cached_10_comps_8_terms", cache_kind: QueryCacheKind::Auto, id_count: 10, term_count: 8, sparse: false, fragment: true, component_benchmark: true },
       BenchmarkConfig { name: "cached_16_comps_1_term", cache_kind: QueryCacheKind::Auto, id_count: 16, term_count: 1, sparse: false, fragment: true, component_benchmark: true },
       BenchmarkConfig { name: "cached_16_comps_4_terms", cache_kind: QueryCacheKind::Auto, id_count: 16, term_count: 4, sparse: false, fragment: true, component_benchmark: true },
       BenchmarkConfig { name: "cached_16_comps_8_terms", cache_kind: QueryCacheKind::Auto, id_count: 16, term_count: 8, sparse: false, fragment: true, component_benchmark: true },

       BenchmarkConfig { name: "cached_10_sparse_comps_4_terms", cache_kind: QueryCacheKind::Auto, id_count: 10, term_count: 4, sparse: true, fragment: true, component_benchmark: true },
       BenchmarkConfig { name: "cached_10_nofrag_comps_4_terms", cache_kind: QueryCacheKind::Auto, id_count: 10, term_count: 4, sparse: true, fragment: false, component_benchmark: true },
    ];

    for benchmark in benchmarks {
        if benchmark.component_benchmark {
            match benchmark.term_count {
                1 => bench_query_iter_components_1_term(
                    &mut group,
                    benchmark.name,
                    benchmark.cache_kind,
                    benchmark.id_count,
                    benchmark.sparse,
                    benchmark.fragment,
                ),
                4 => bench_query_iter_components_4_term(
                    &mut group,
                    benchmark.name,
                    benchmark.cache_kind,
                    benchmark.id_count,
                    benchmark.sparse,
                    benchmark.fragment,
                ),
                8 => bench_query_iter_components_8_term(
                    &mut group,
                    benchmark.name,
                    benchmark.cache_kind,
                    benchmark.id_count,
                    benchmark.sparse,
                    benchmark.fragment,
                ),
                _ => panic!("Unsupported components count"),
            }
        } else {
            bench_query_iter_tags(
                &mut group,
                benchmark.name,
                benchmark.cache_kind,
                benchmark.id_count,
                benchmark.term_count,
                benchmark.sparse,
                benchmark.fragment,
            );
        }
    }

    // bench_query_iter_components_4_term_run(
    //     &mut group,
    //     "query_iter_read_run_4",
    //     QueryCacheKind::Default,
    //     6,
    //     false,
    //     true,
    // );
    // c_query_iter_read_4(&mut group, "c_query_iter_read_4", QueryCacheKind::Auto, 6);

    group.finish();
}

fn c_query_iter_read_4(
    group: &mut criterion::BenchmarkGroup<'_, criterion::measurement::WallTime>,
    name: &str,
    cache_kind: QueryCacheKind,
    id_count: usize,
) {
    group.bench_function(format!("c_query_iter_{name}"), |bencher| {
        reset_srand();
        // SAFETY: raw C API calls; world ptr from World::new is valid for the benchmark
        // duration, ids/desc are built from it, and field indices match the 4 registered terms.
        unsafe {
            let world_ref = World::new();
            let world = world_ref.ptr_mut();
            let ids = create_ids(&world_ref, id_count, 4, true, false, true);

            let mut desc: sys::ecs_query_desc_t = sys::ecs_query_desc_t {
                cache_kind: cache_kind as sys::ecs_query_cache_kind_t,
                ..Default::default()
            };

            for (i, id) in ids.iter().enumerate().take(4) {
                desc.terms[i].id = ids[i];
                desc.terms[i].flags_ = sys::EcsSelf as u16;
                desc.terms[i].inout = sys::ecs_inout_kind_t_EcsIn as i16;
            }

            let f = sys::ecs_query_init(world, &desc);
            let mut result: u32 = 0;

            for i in 0..QUERY_ENTITY_COUNT {
                let e = sys::ecs_new(world);
                for id in &ids {
                    if (flip_coin()) {
                        sys::ecs_set_id(world, e, *id, 4, &(i + 1) as *const u32 as *const c_void);
                    }
                }
            }

            bencher.iter_custom(|iters| {
                let start = Instant::now();
                for _ in 0..iters {
                    let mut it: sys::ecs_iter_t = sys::ecs_query_iter(world, f);
                    // optimized next call that's selected automatically by c++/system api
                    while (sys::ecs_query_next(&mut it as *mut sys::ecs_iter_t)) {
                        let count = it.count as usize;
                        let c1 = sys::ecs_field_w_size(&it, 4, 0) as *const u32;
                        let c2 = sys::ecs_field_w_size(&it, 4, 1) as *const u32;
                        let c3 = sys::ecs_field_w_size(&it, 4, 2) as *const u32;
                        let c4 = sys::ecs_field_w_size(&it, 4, 3) as *const u32;
                        for i in 0..count {
                            result += *it.entities.add(i) as u32;
                            result += *c1.add(i);
                            result += *c2.add(i);
                            result += *c3.add(i);
                            result += *c4.add(i);
                        }
                    }
                }
                start.elapsed()
            });

            core::hint::black_box(result);

            sys::ecs_query_fini(f);
        }
    });
}

fn bench_query_init_fini(
    group: &mut criterion::BenchmarkGroup<'_, criterion::measurement::WallTime>,
    label: &str,
    cache_kind: Option<QueryCacheKind>,
    id_count: usize,
) {
    group.bench_function(label, |bencher| {
        let world = World::new();
        let ids: Vec<EntityView> = (0..id_count).map(|_| world.entity()).collect();

        bencher.iter_custom(|iters| {
            let start = Instant::now();
            for _ in 0..iters {
                let mut query = world.query::<()>();
                if let Some(kind) = cache_kind {
                    query.set_cache_kind(kind);
                }
                for id in &ids {
                    query.with(*id).self_();
                }
                let q = query.build();
                drop(q);
            }
            start.elapsed() / 2
        });
    });
}

pub fn query_init_fini(criterion: &mut Criterion) {
    let mut group = criterion.benchmark_group("flecs");

    bench_query_init_fini(&mut group, "uncach_init_fini_1_ids", None, 1);
    bench_query_init_fini(&mut group, "uncach_init_fini_4_ids", None, 4);
    bench_query_init_fini(&mut group, "uncach_init_fini_8_ids", None, 8);
    bench_query_init_fini(&mut group, "uncach_init_fini_16_ids", None, 16);

    bench_query_init_fini(
        &mut group,
        "cached_init_fini_1_ids",
        Some(QueryCacheKind::Auto),
        1,
    );
    bench_query_init_fini(
        &mut group,
        "cached_init_fini_4_ids",
        Some(QueryCacheKind::Auto),
        4,
    );
    bench_query_init_fini(
        &mut group,
        "cached_init_fini_8_ids",
        Some(QueryCacheKind::Auto),
        8,
    );
    bench_query_init_fini(
        &mut group,
        "cached_init_fini_16_ids",
        Some(QueryCacheKind::Auto),
        16,
    );

    group.finish();
}

fn bench_query_iter_read_1(
    group: &mut criterion::BenchmarkGroup<'_, criterion::measurement::WallTime>,
    label: &str,
    id_count: usize,
) {
    group.bench_function(label, |bencher| {
        reset_srand();
        let world = World::new();
        let ids = get_component_ids!(&world, id_count + 1, true, false, true);

        let mut query = world.query::<()>();
        query.set_cache_kind(QueryCacheKind::Auto);
        query.with(ids[0]).self_().set_in();
        let f = query.build();

        for _ in 0..QUERY_ENTITY_COUNT {
            let e = world.entity();
            for id in &ids {
                if flip_coin() {
                    e.add(*id);
                }
            }
        }

        let mut result: u32 = 0;
        bencher.iter_custom(|iters| {
            let start = Instant::now();
            for _ in 0..iters {
                f.run(|mut it| {
                    while it.next() {
                        let c1 = it.field::<C1>(0);
                        for i in it.iter() {
                            result += *it.entity_id(i) as u32 + c1[i].0;
                        }
                    }
                });
            }
            start.elapsed()
        });
        core::hint::black_box(result);
        reset_world_arrays(&world);
    });
}

fn bench_query_iter_read_4(
    group: &mut criterion::BenchmarkGroup<'_, criterion::measurement::WallTime>,
    label: &str,
    id_count: usize,
) {
    group.bench_function(label, |bencher| {
        reset_srand();
        let world = World::new();
        let ids = get_component_ids!(&world, id_count + 1, true, false, true);

        let mut query = world.query::<()>();
        query.set_cache_kind(QueryCacheKind::Auto);
        for id in &ids[..4] {
            query.with(*id).self_().set_in();
        }
        let f = query.build();

        for _ in 0..QUERY_ENTITY_COUNT {
            let e = world.entity();
            for id in &ids {
                if flip_coin() {
                    e.add(*id);
                }
            }
        }

        let mut result: u32 = 0;
        bencher.iter_custom(|iters| {
            let start = Instant::now();
            for _ in 0..iters {
                f.run(|mut it| {
                    while it.next() {
                        let c1 = it.field::<C1>(0);
                        let c2 = it.field::<C2>(1);
                        let c3 = it.field::<C3>(2);
                        let c4 = it.field::<C4>(3);
                        for i in it.iter() {
                            result +=
                                *it.entity_id(i) as u32 + c1[i].0 + c2[i].0 + c3[i].0 + c4[i].0;
                        }
                    }
                });
            }
            start.elapsed()
        });
        core::hint::black_box(result);
        reset_world_arrays(&world);
    });
}

fn bench_query_iter_read_8(
    group: &mut criterion::BenchmarkGroup<'_, criterion::measurement::WallTime>,
    label: &str,
    id_count: usize,
) {
    group.bench_function(label, |bencher| {
        reset_srand();
        let world = World::new();
        let ids = get_component_ids!(&world, id_count + 1, true, false, true);

        let mut query = world.query::<()>();
        query.set_cache_kind(QueryCacheKind::Auto);
        for id in &ids[..8] {
            query.with(*id).self_().set_in();
        }
        let f = query.build();

        for _ in 0..QUERY_ENTITY_COUNT {
            let e = world.entity();
            for id in &ids {
                if flip_coin() {
                    e.add(*id);
                }
            }
        }

        let mut result: u32 = 0;
        bencher.iter_custom(|iters| {
            let start = Instant::now();
            for _ in 0..iters {
                f.run(|mut it| {
                    while it.next() {
                        let c1 = it.field::<C1>(0);
                        let c2 = it.field::<C2>(1);
                        let c3 = it.field::<C3>(2);
                        let c4 = it.field::<C4>(3);
                        let c5 = it.field::<C5>(4);
                        let c6 = it.field::<C6>(5);
                        let c7 = it.field::<C7>(6);
                        let c8 = it.field::<C8>(7);
                        for i in it.iter() {
                            result += *it.entity_id(i) as u32
                                + c1[i].0
                                + c2[i].0
                                + c3[i].0
                                + c4[i].0
                                + c5[i].0
                                + c6[i].0
                                + c7[i].0
                                + c8[i].0;
                        }
                    }
                });
            }
            start.elapsed()
        });
        core::hint::black_box(result);
        reset_world_arrays(&world);
    });
}

fn bench_query_iter_sparse_variant(
    group: &mut criterion::BenchmarkGroup<'_, criterion::measurement::WallTime>,
    label: &str,
    id_count: usize,
    sparse: bool,
    fragment: bool,
) {
    group.bench_function(label, |bencher| {
        reset_srand();
        let world = World::new();
        let ids = get_component_ids!(&world, id_count + 1, true, sparse, fragment);

        let mut query = world.query::<()>();
        query.set_cache_kind(QueryCacheKind::Auto);
        for id in &ids[..4] {
            query.with(*id).self_().set_in();
        }
        let f = query.build();

        for _ in 0..QUERY_ENTITY_COUNT {
            let e = world.entity();
            for id in &ids {
                if flip_coin() {
                    e.add(*id);
                }
            }
        }

        let mut result: u32 = 0;
        bencher.iter_custom(|iters| {
            let start = Instant::now();
            for _ in 0..iters {
                f.run(|mut it| {
                    while it.next() {
                        for i in it.iter() {
                            result += it.field_at::<C1>(0, i).0;
                            result += it.field_at::<C2>(1, i).0;
                            result += it.field_at::<C3>(2, i).0;
                            result += it.field_at::<C4>(3, i).0;
                            result += *it.entity_id(i) as u32;
                        }
                    }
                });
            }
            start.elapsed()
        });
        core::hint::black_box(result);
        reset_world_arrays(&world);
    });
}

pub fn query_iter_read(criterion: &mut Criterion) {
    let mut group = criterion.benchmark_group("flecs");

    bench_query_iter_read_1(&mut group, "cached_6_components_1_term", 6);
    bench_query_iter_read_4(&mut group, "cached_6_components_4_terms", 6);
    bench_query_iter_read_1(&mut group, "cached_8_components_1_term", 8);
    bench_query_iter_read_4(&mut group, "cached_8_components_4_terms", 8);
    bench_query_iter_read_1(&mut group, "cached_10_components_1_term", 10);
    bench_query_iter_read_4(&mut group, "cached_10_components_4_terms", 10);
    bench_query_iter_read_8(&mut group, "cached_10_components_8_terms", 10);
    bench_query_iter_read_1(&mut group, "cached_16_components_1_term", 16);
    bench_query_iter_read_4(&mut group, "cached_16_components_4_terms", 16);
    bench_query_iter_read_8(&mut group, "cached_16_components_8_terms", 16);

    bench_query_iter_sparse_variant(&mut group, "cached_10_sparse_4_terms", 10, true, true);
    bench_query_iter_sparse_variant(&mut group, "cached_10_nofrag_4_terms", 10, true, false);

    group.finish();
}

fn bench_query_iter_empty(
    group: &mut criterion::BenchmarkGroup<'_, criterion::measurement::WallTime>,
    label: &str,
    table_count: usize,
    cache_kind: QueryCacheKind,
) {
    group.bench_function(label, |bencher| {
        let world = World::new();
        let table_ids: Vec<EntityView> = (0..=table_count).map(|_| world.entity()).collect();

        for i in 0..table_count {
            let e = world.entity();
            e.add(table_ids[table_count]);
            e.add(table_ids[i]);
            e.destruct();
        }

        let e2 = world.entity();
        e2.add(table_ids[table_count]);

        let mut query = world.query::<()>();
        query.set_cache_kind(cache_kind);
        query.with(table_ids[table_count]).self_();
        let q = query.build();

        let mut result: u32 = 0;
        bencher.iter_custom(|iters| {
            let start = Instant::now();
            for _ in 0..iters {
                q.run(|mut it| {
                    while it.next() {
                        for i in it.iter() {
                            result += *it.entity_id(i) as u32;
                        }
                    }
                });
            }
            start.elapsed()
        });
        core::hint::black_box(result);
    });
}

pub fn query_iter_empty(criterion: &mut Criterion) {
    let mut group = criterion.benchmark_group("flecs");

    bench_query_iter_empty(
        &mut group,
        "uncach_255_empty_1_fill",
        256,
        QueryCacheKind::Default,
    );
    bench_query_iter_empty(
        &mut group,
        "uncach_1023_empty_1_fill",
        1024,
        QueryCacheKind::Default,
    );
    bench_query_iter_empty(
        &mut group,
        "cached_255_empty_1_fill",
        256,
        QueryCacheKind::Auto,
    );
    bench_query_iter_empty(
        &mut group,
        "cached_1023_empty_1_fill",
        1024,
        QueryCacheKind::Auto,
    );

    group.finish();
}

fn bench_query_iter_up(
    group: &mut criterion::BenchmarkGroup<'_, criterion::measurement::WallTime>,
    label: &str,
    cache_kind: QueryCacheKind,
    query_self: bool,
) {
    group.bench_function(label, |bencher| {
        let world = World::new();
        let id0 = world.entity();
        let id1 = world.entity();
        id0.add_trait::<(flecs::OnInstantiate, flecs::Inherit)>();

        let parent_with = world.entity();
        parent_with.add(id0);
        let parent_without = world.entity();
        parent_without.add(id1);

        for i in 0..QUERY_ENTITY_COUNT {
            let parent = if i < QUERY_ENTITY_COUNT / 2 {
                world.entity().child_of(parent_with)
            } else {
                world.entity().child_of(parent_without)
            };
            let e = world.entity().child_of(parent);
            e.add(id1);
        }

        let mut query = world.query::<()>();
        query.set_cache_kind(cache_kind);
        query.with(id0).up();
        if query_self {
            query.with(id1);
        }
        let f = query.build();

        let mut result: u32 = 0;
        bencher.iter_custom(|iters| {
            let start = Instant::now();
            for _ in 0..iters {
                f.run(|mut it| {
                    while it.next() {
                        for i in it.iter() {
                            result += *it.entity_id(i) as u32;
                        }
                    }
                });
            }
            start.elapsed()
        });
        core::hint::black_box(result);
    });
}

fn bench_query_iter_up_w_mut(
    group: &mut criterion::BenchmarkGroup<'_, criterion::measurement::WallTime>,
    label: &str,
    query_self: bool,
) {
    group.bench_function(label, |bencher| {
        let world = World::new();
        let id0 = world.entity();
        let id1 = world.entity();
        id0.add_trait::<(flecs::OnInstantiate, flecs::Inherit)>();

        let parent_with = world.entity();
        parent_with.add(id0);
        let parent_without = world.entity();
        parent_without.add(id1);

        for i in 0..QUERY_ENTITY_COUNT {
            let parent = if i < QUERY_ENTITY_COUNT / 2 {
                world.entity().child_of(parent_with)
            } else {
                world.entity().child_of(parent_without)
            };
            world.entity().child_of(parent);
        }

        let mut query = world.query::<()>();
        query.with(id0).up();
        if query_self {
            query.with(id1);
        }
        let f = query.build();

        let mut result: u32 = 0;
        bencher.iter_custom(|iters| {
            let start = Instant::now();
            for _ in 0..iters {
                f.run(|mut it| {
                    while it.next() {
                        for i in it.iter() {
                            result += *it.entity_id(i) as u32;
                        }
                    }
                });
                parent_with.remove(id0);
                parent_with.add(id0);
            }
            start.elapsed()
        });
        core::hint::black_box(result);
    });
}

pub fn query_iter_up(criterion: &mut Criterion) {
    let mut group = criterion.benchmark_group("flecs");

    bench_query_iter_up(&mut group, "uncach_up_tags", QueryCacheKind::Default, false);
    bench_query_iter_up(
        &mut group,
        "uncach_up_tags_w_self",
        QueryCacheKind::Default,
        true,
    );
    bench_query_iter_up_w_mut(&mut group, "uncach_up_w_mut_8_tags", false);
    bench_query_iter_up_w_mut(&mut group, "uncach_up_w_mut_8_tags_w_self", true);
    bench_query_iter_up(&mut group, "cached_up_tags", QueryCacheKind::Auto, false);
    bench_query_iter_up(
        &mut group,
        "cached_up_tags_w_self",
        QueryCacheKind::Auto,
        true,
    );

    group.finish();
}

fn bench_query_inheritance(
    group: &mut criterion::BenchmarkGroup<'_, criterion::measurement::WallTime>,
    label: &str,
    depth: usize,
    id_count: usize,
) {
    group.bench_function(label, |bencher| {
        reset_srand();
        let world = World::new();
        let ids: Vec<EntityView> = (0..id_count).map(|_| world.entity()).collect();

        let id = world.entity();
        let mut cur = id;
        for _ in 0..depth {
            cur = world.entity().is_a(cur);
        }

        for _ in 0..QUERY_ENTITY_COUNT {
            let e = world.entity();
            for t in &ids {
                if flip_coin() {
                    e.add(*t);
                }
            }
            e.add(cur);
        }

        let q = world.query::<()>().with(id).self_().build();

        let mut result: u32 = 0;
        bencher.iter_custom(|iters| {
            let start = Instant::now();
            for _ in 0..iters {
                q.run(|mut it| {
                    while it.next() {
                        for i in it.iter() {
                            result += *it.entity_id(i) as u32;
                        }
                    }
                });
            }
            start.elapsed()
        });
        core::hint::black_box(result);
    });
}

pub fn query_inheritance(criterion: &mut Criterion) {
    let mut group = criterion.benchmark_group("flecs");

    bench_query_inheritance(&mut group, "uncach_inherit_depth_1_tables_1", 1, 0);
    bench_query_inheritance(&mut group, "uncach_inherit_depth_1_tables_1024", 1, 10);
    bench_query_inheritance(&mut group, "uncach_inherit_depth_2_tables_1", 2, 0);
    bench_query_inheritance(&mut group, "uncach_inherit_depth_2_tables_1024", 2, 10);
    bench_query_inheritance(&mut group, "uncach_inherit_depth_8_tables_1", 8, 0);
    bench_query_inheritance(&mut group, "uncach_inherit_depth_8_tables_1024", 8, 10);
    bench_query_inheritance(&mut group, "uncach_inherit_depth_16_tables_1", 16, 0);
    bench_query_inheritance(&mut group, "uncach_inherit_depth_16_tables_1024", 16, 10);

    group.finish();
}

fn bench_query_w_vars(
    group: &mut criterion::BenchmarkGroup<'_, criterion::measurement::WallTime>,
    label: &str,
    cache_kind: QueryCacheKind,
) {
    group.bench_function(label, |bencher| {
        let world = World::new();
        let space_ship = world.entity_named("SpaceShip");
        let docked_to = world.entity_named("DockedTo");
        let planet_tag = world.entity_named("Planet");
        let moon_tag = world.entity_named("Moon");

        for _ in 0..ENTITY_COUNT {
            let planet = world.entity().add(planet_tag);
            for _ in 0..10 {
                let spaceship = world.entity().add(space_ship);
                spaceship.add((docked_to, planet));
                spaceship.add(world.entity());
            }
        }

        for _ in 0..ENTITY_COUNT {
            let moon = world.entity().add(moon_tag);
            for _ in 0..10 {
                let spaceship = world.entity().add(space_ship);
                spaceship.add((docked_to, moon));
                spaceship.add(world.entity());
            }
        }

        for _ in 0..ENTITY_COUNT {
            let spaceship = world.entity().add(space_ship);
            spaceship.add(world.entity());
        }

        let q = world
            .query::<()>()
            .expr("SpaceShip, (DockedTo, $planet), Planet($planet)")
            .set_cache_kind(cache_kind)
            .build();

        let mut result: u64 = 0;
        bencher.iter_custom(|iters| {
            let start = Instant::now();
            for _ in 0..iters {
                q.run(|mut it| {
                    while it.next() {
                        for i in it.iter() {
                            result += *it.entity_id(i);
                        }
                    }
                });
            }
            start.elapsed()
        });
        core::hint::black_box(result);
    });
}

fn bench_query_w_singleton(
    group: &mut criterion::BenchmarkGroup<'_, criterion::measurement::WallTime>,
    label: &str,
    cache_kind: QueryCacheKind,
) {
    group.bench_function(label, |bencher| {
        reset_srand();
        let world = World::new();
        let ids = get_component_ids!(&world, 6, true, false, true);
        let c5 = world.component::<C5>();
        c5.add(*c5.id());

        for _ in 0..QUERY_ENTITY_COUNT {
            let e = world.entity();
            for id in &ids[..4] {
                if flip_coin() {
                    e.add(*id);
                }
            }
        }

        let mut query = world.query::<()>();
        query.set_cache_kind(cache_kind);
        for id in &ids[..4] {
            query.with(*id).self_().set_in();
        }
        query.with(ids[4]).set_src(ids[4]).set_in();
        let q = query.build();

        let mut result: u64 = 0;
        bencher.iter_custom(|iters| {
            let start = Instant::now();
            for _ in 0..iters {
                q.run(|mut it| {
                    while it.next() {
                        let f0 = it.field::<C1>(0);
                        let f1 = it.field::<C2>(1);
                        let f2 = it.field::<C3>(2);
                        let f3 = it.field::<C4>(3);
                        let f4 = it.field::<C5>(4);
                        result += f0[0].0 as u64;
                        result += f1[0].0 as u64;
                        result += f2[0].0 as u64;
                        result += f3[0].0 as u64;
                        result += f4[0].0 as u64;
                        for i in it.iter() {
                            result += *it.entity_id(i);
                        }
                    }
                });
            }
            start.elapsed()
        });
        core::hint::black_box(result);
        reset_world_arrays(&world);
    });
}

fn bench_query_w_not(
    group: &mut criterion::BenchmarkGroup<'_, criterion::measurement::WallTime>,
    label: &str,
    cache_kind: QueryCacheKind,
) {
    group.bench_function(label, |bencher| {
        reset_srand();
        let world = World::new();
        let ids = get_component_ids!(&world, 7, true, false, true);
        let rel = world.entity();
        let tgt = world.entity();

        for _ in 0..QUERY_ENTITY_COUNT {
            let e = world.entity();
            for id in &ids {
                if flip_coin() {
                    e.add(*id);
                }
            }
            if flip_coin() {
                e.add((rel, tgt));
            }
        }

        let mut query = world.query::<()>();
        query.set_cache_kind(cache_kind);
        for id in &ids[..4] {
            query.with(*id).self_().set_in();
        }
        query.with(ids[4]).not();
        query.without((rel, id::<flecs::Wildcard>()));
        let q = query.build();

        let mut result: u64 = 0;
        bencher.iter_custom(|iters| {
            let start = Instant::now();
            for _ in 0..iters {
                q.run(|mut it| {
                    while it.next() {
                        let f0 = it.field::<C1>(0);
                        let f1 = it.field::<C2>(1);
                        let f2 = it.field::<C3>(2);
                        let f3 = it.field::<C4>(3);
                        result += f0[0].0 as u64;
                        result += f1[0].0 as u64;
                        result += f2[0].0 as u64;
                        result += f3[0].0 as u64;
                        for i in it.iter() {
                            result += *it.entity_id(i);
                        }
                    }
                });
            }
            start.elapsed()
        });
        core::hint::black_box(result);
        reset_world_arrays(&world);
    });
}

fn bench_query_w_optional(
    group: &mut criterion::BenchmarkGroup<'_, criterion::measurement::WallTime>,
    label: &str,
    cache_kind: QueryCacheKind,
) {
    group.bench_function(label, |bencher| {
        reset_srand();
        let world = World::new();
        let ids = get_component_ids!(&world, 7, true, false, true);
        let rel = world.entity();
        let tgt = world.entity();

        for _ in 0..QUERY_ENTITY_COUNT {
            let e = world.entity();
            for id in &ids {
                if flip_coin() {
                    e.add(*id);
                }
            }
            if flip_coin() {
                e.add((rel, tgt));
            }
        }

        let mut query = world.query::<()>();
        query.set_cache_kind(cache_kind);
        for id in &ids[..4] {
            query.with(*id).self_().set_in();
        }
        query.with(ids[4]).optional();
        let q = query.build();

        let mut result: u64 = 0;
        bencher.iter_custom(|iters| {
            let start = Instant::now();
            for _ in 0..iters {
                q.run(|mut it| {
                    while it.next() {
                        let f0 = it.field::<C1>(0);
                        let f1 = it.field::<C2>(1);
                        let f2 = it.field::<C3>(2);
                        let f3 = it.field::<C4>(3);
                        result += f0[0].0 as u64;
                        result += f1[0].0 as u64;
                        result += f2[0].0 as u64;
                        result += f3[0].0 as u64;
                        for i in it.iter() {
                            result += *it.entity_id(i);
                        }
                    }
                });
            }
            start.elapsed()
        });
        core::hint::black_box(result);
        reset_world_arrays(&world);
    });
}

pub fn query_special(criterion: &mut Criterion) {
    let mut group = criterion.benchmark_group("flecs");

    bench_query_w_vars(&mut group, "uncach_w_vars", QueryCacheKind::Default);
    bench_query_w_singleton(&mut group, "uncach_w_singleton", QueryCacheKind::Default);
    bench_query_w_not(&mut group, "uncach_w_not", QueryCacheKind::Default);
    bench_query_w_optional(&mut group, "uncach_w_optional", QueryCacheKind::Default);

    bench_query_w_vars(&mut group, "cached_w_vars", QueryCacheKind::Auto);
    bench_query_w_singleton(&mut group, "cached_w_singleton", QueryCacheKind::Auto);
    bench_query_w_not(&mut group, "cached_w_not", QueryCacheKind::Auto);
    bench_query_w_optional(&mut group, "cached_w_optional", QueryCacheKind::Auto);

    group.finish();
}

#[derive(Debug, Default, Clone, Component)]
struct LocalTransform {
    x: f32,
    y: f32,
    z: f32,
}

#[derive(Debug, Default, Clone, Component)]
struct WorldTransform {
    x: f32,
    y: f32,
    z: f32,
}

fn populate_transform_hierarchy(world: &World) {
    for i in 0..ENTITY_COUNT {
        let e = world.entity();
        e.set(LocalTransform {
            x: i as f32,
            y: (i * 2) as f32,
            z: 0.0,
        });

        for _ in 0..5 {
            let c = world.entity().child_of(e);
            c.set(LocalTransform {
                x: i as f32,
                y: (i * 2) as f32,
                z: 0.0,
            });

            for k in 0..5 {
                let gc = world.entity().child_of(c);
                gc.set(LocalTransform {
                    x: k as f32,
                    y: (k * 2) as f32,
                    z: 0.0,
                });

                for l in 0..2 {
                    let ggc = world.entity().child_of(gc);
                    ggc.set(LocalTransform {
                        x: l as f32,
                        y: (l * 2) as f32,
                        z: 0.0,
                    });
                }
            }
        }
    }
}

fn bench_query_transform(
    group: &mut criterion::BenchmarkGroup<'_, criterion::measurement::WallTime>,
    label: &str,
) {
    group.bench_function(label, |bencher| {
        let world = World::new();
        world
            .component::<LocalTransform>()
            .add_trait::<(flecs::With, WorldTransform)>();

        let q = world
            .query::<(&LocalTransform, Option<&WorldTransform>, &mut WorldTransform)>()
            .term_at(1)
            .parent()
            .cascade()
            .set_cache_kind(QueryCacheKind::Auto)
            .build();

        populate_transform_hierarchy(&world);

        bencher.iter_custom(|iters| {
            let start = Instant::now();
            for _ in 0..iters {
                q.each(|(t_local, t_parent, t_world)| {
                    t_world.x = t_local.x;
                    t_world.y = t_local.y;
                    t_world.z = t_local.z;
                    if let Some(t_parent) = t_parent {
                        t_world.x += t_parent.x;
                        t_world.y += t_parent.y;
                        t_world.z += t_parent.z;
                    }
                });
            }
            start.elapsed()
        });
    });
}

fn query_iter_dfs(
    q: &Query<(&LocalTransform, Option<&WorldTransform>, &mut WorldTransform)>,
    parent: EntityView<'_>,
) {
    q.with_group(parent).run(|mut it| {
        while it.next() {
            {
                let t = it.field::<LocalTransform>(0);
                let parent_wt = it.get_field::<WorldTransform>(1);
                let mut wt = it.field_mut::<WorldTransform>(2);
                for _ in it.iter() {
                    if let Some(parent_wt) = &parent_wt {
                        for i in it.iter() {
                            wt[i].x = t[i].x + parent_wt[0].x;
                            wt[i].y = t[i].y + parent_wt[0].y;
                            wt[i].z = t[i].z + parent_wt[0].z;
                        }
                    } else {
                        for i in it.iter() {
                            wt[i].x = t[i].x;
                            wt[i].y = t[i].y;
                            wt[i].z = t[i].z;
                        }
                    }
                }
            }
            for i in it.iter() {
                query_iter_dfs(q, it.entity(i));
            }
        }
    });
}

fn bench_query_depth_first(
    group: &mut criterion::BenchmarkGroup<'_, criterion::measurement::WallTime>,
    label: &str,
) {
    group.bench_function(label, |bencher| {
        let world = World::new();
        world
            .component::<LocalTransform>()
            .add_trait::<(flecs::With, WorldTransform)>();

        let q = world
            .query::<(&LocalTransform, Option<&WorldTransform>, &mut WorldTransform)>()
            .term_at(1)
            .up()
            .group_by(id::<flecs::ChildOf>())
            .set_cache_kind(QueryCacheKind::Auto)
            .build();

        populate_transform_hierarchy(&world);

        bencher.iter_custom(|iters| {
            let start = Instant::now();
            for _ in 0..iters {
                query_iter_dfs(&q, world.entity_from_id(0u64));
            }
            start.elapsed()
        });
    });
}

pub fn query_transform(criterion: &mut Criterion) {
    let mut group = criterion.benchmark_group("flecs");

    bench_query_transform(&mut group, "query_transform");
    bench_query_depth_first(&mut group, "query_depth_first");

    group.finish();
}

#[derive(Clone, Copy)]
enum ToggleMode {
    NoDisabled,
    HalfDisabled,
    AltDisabled,
}

fn bench_query_cantoggle(
    group: &mut criterion::BenchmarkGroup<'_, criterion::measurement::WallTime>,
    label: &str,
    term_count: usize,
    mode: ToggleMode,
) {
    group.bench_function(label, |bencher| {
        let world = World::new();
        let ids: Vec<EntityView> = (0..term_count)
            .map(|_| {
                let id = world.entity();
                id.add_trait::<flecs::CanToggle>();
                id
            })
            .collect();

        let entities: Vec<EntityView> = (0..ENTITY_COUNT).map(|_| world.entity()).collect();
        for e in &entities {
            for id in &ids {
                e.add(*id);
            }
        }

        match mode {
            ToggleMode::NoDisabled => {}
            ToggleMode::HalfDisabled => {
                for e in entities.iter().take((ENTITY_COUNT / 2) as usize) {
                    for id in &ids {
                        e.enable(*id);
                    }
                }
                for e in entities.iter().skip((ENTITY_COUNT / 2) as usize) {
                    for id in &ids {
                        e.disable(*id);
                    }
                }
            }
            ToggleMode::AltDisabled => {
                for (idx, e) in entities.iter().enumerate() {
                    if idx % 2 == 0 {
                        for id in &ids {
                            e.enable(*id);
                        }
                    } else {
                        for id in &ids {
                            e.disable(*id);
                        }
                    }
                }
            }
        }

        let mut query = world.query::<()>();
        query.set_cache_kind(QueryCacheKind::Auto);
        for id in &ids {
            query.with(*id);
        }
        let q = query.build();

        let mut result: u32 = 0;
        bencher.iter_custom(|iters| {
            let start = Instant::now();
            for _ in 0..iters {
                q.run(|mut it| {
                    while it.next() {
                        for i in it.iter() {
                            result += *it.entity_id(i) as u32;
                        }
                    }
                });
            }
            start.elapsed()
        });
        core::hint::black_box(result);
    });
}

pub fn query_toggle(criterion: &mut Criterion) {
    let mut group = criterion.benchmark_group("flecs");

    bench_query_cantoggle(
        &mut group,
        "cached_cantoggle_no_toggle_1_term",
        1,
        ToggleMode::NoDisabled,
    );
    bench_query_cantoggle(
        &mut group,
        "cached_cantoggle_no_toggle_4_terms",
        4,
        ToggleMode::NoDisabled,
    );
    bench_query_cantoggle(
        &mut group,
        "cached_cantoggle_no_toggle_8_terms",
        8,
        ToggleMode::NoDisabled,
    );

    bench_query_cantoggle(
        &mut group,
        "cached_cantoggle_half_toggle_1_term",
        1,
        ToggleMode::HalfDisabled,
    );
    bench_query_cantoggle(
        &mut group,
        "cached_cantoggle_half_toggle_4_terms",
        4,
        ToggleMode::HalfDisabled,
    );
    bench_query_cantoggle(
        &mut group,
        "cached_cantoggle_half_toggle_8_terms",
        8,
        ToggleMode::HalfDisabled,
    );

    bench_query_cantoggle(
        &mut group,
        "cached_cantoggle_alt_toggle_1_term",
        1,
        ToggleMode::AltDisabled,
    );
    bench_query_cantoggle(
        &mut group,
        "cached_cantoggle_alt_toggle_4_terms",
        4,
        ToggleMode::AltDisabled,
    );
    bench_query_cantoggle(
        &mut group,
        "cached_cantoggle_alt_toggle_8_terms",
        8,
        ToggleMode::AltDisabled,
    );

    group.finish();
}

fn bench_rematch_tables(
    group: &mut criterion::BenchmarkGroup<'_, criterion::measurement::WallTime>,
    label: &str,
    rematch_count: usize,
    total_count: usize,
) {
    group.bench_function(label, |bencher| {
        let world = World::new();
        let id0 = world.entity();
        let _id1 = world.entity();
        id0.add_trait::<(flecs::OnInstantiate, flecs::Inherit)>();

        let base_1 = world.entity().add(id0);
        let base_2 = world.entity().add(id0);

        for _ in 0..rematch_count {
            let tag = world.entity();
            world.entity().is_a(base_1).add(tag);
        }
        for _ in 0..(total_count - rematch_count) {
            let tag = world.entity();
            world.entity().is_a(base_2).add(tag);
        }

        let mut query = world.query::<()>();
        query.with(id0).self_().up();
        query.set_cache_kind(QueryCacheKind::Auto);
        let q = query.build();

        bencher.iter_custom(|iters| {
            let start = Instant::now();
            for _ in 0..iters {
                base_1.remove(id0);
                let _ = q.iter_stage(&world);
                base_1.add(id0);
                let _ = q.iter_stage(&world);
            }
            start.elapsed() / 2
        });
    });
}

pub fn query_rematch(criterion: &mut Criterion) {
    let mut group = criterion.benchmark_group("flecs");

    bench_rematch_tables(&mut group, "rematch_1_of_1000_tables", 1, 1000);
    bench_rematch_tables(&mut group, "rematch_10_of_1000_tables", 10, 1000);
    bench_rematch_tables(&mut group, "rematch_100_of_1000_tables", 100, 1000);
    bench_rematch_tables(&mut group, "rematch_1000_of_1000_tables", 1000, 1000);

    group.finish();
}
