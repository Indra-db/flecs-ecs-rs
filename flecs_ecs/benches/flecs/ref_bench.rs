use crate::common_bench::*;

pub fn ref_init(criterion: &mut Criterion) {
    let mut group = criterion.benchmark_group("flecs");

    group.bench_function("ref_init", |bencher| {
        let world = World::new_mini();
        let entities = create_entities(&world, ENTITY_COUNT as usize);
        register_component_range!(world, C, 1, 1);

        for entity in &entities {
            entity.set(C1(0));
        }

        bencher.iter_custom(|iters| {
            let start = Instant::now();
            for _ in 0..iters {
                for entity in &entities {
                    let r = entity.cached_ref(C1::id());
                    core::hint::black_box(r);
                }
            }
            let elapsed = start.elapsed();
            elapsed / ENTITY_COUNT
        });

        reset_world_arrays(&world);
    });

    group.finish();
}

pub fn ref_get(criterion: &mut Criterion) {
    let mut group = criterion.benchmark_group("flecs");

    group.bench_function("ref_get", |bencher| {
        let world = World::new_mini();
        let entities = create_entities(&world, ENTITY_COUNT as usize);
        register_component_range!(world, C, 1, 1);

        let mut refs: Vec<CachedRef<C1>> = Vec::with_capacity(ENTITY_COUNT as usize);
        for entity in &entities {
            entity.set(C1(0));
            refs.push(entity.cached_ref(C1::id()));
        }

        bencher.iter_custom(|iters| {
            let start = Instant::now();
            for _ in 0..iters {
                for r in &mut refs {
                    r.get(|c| {
                        core::hint::black_box(c);
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
