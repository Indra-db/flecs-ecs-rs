use crate::common_bench::*;

pub fn set(criterion: &mut Criterion) {
    let mut group = criterion.benchmark_group("flecs");

    bench_loop_entities!(
        group,
        "set",
        ENTITY_COUNT
        ; (register_component_range, (C, 1, 1))
        ; (add_component_range, (C, 1, 1))
        ; (set_component_range, (C, 1, 1))
    );

    group.finish();
}

pub fn set_remove(criterion: &mut Criterion) {
    let mut group = criterion.benchmark_group("flecs");

    bench_loop_entities!(
        group,
        "set_remove_1",
        ENTITY_COUNT
        ; (register_component_range, (C, 1, 1))
        ; // preparation
        ; (set_component_range, (C, 1, 1)), (remove_component_range, (C, 1, 1))
    );

    bench_loop_entities!(
        group,
        "set_remove_16",
        ENTITY_COUNT
        ; (register_component_range, (C, 1, 16))
        ; // preparation
        ; (set_component_range, (C, 1, 16)), (remove_component_range, (C, 1, 16))
    );

    bench_loop_entities!(
        group,
        "set_remove_32",
        ENTITY_COUNT
        ; (register_component_range, (C, 1, 32))
        ; // preparation
        ; (set_component_range, (C, 1, 32)), (remove_component_range, (C, 1, 32))
    );

    group.finish();
}

pub fn ensure(criterion: &mut Criterion) {
    let mut group = criterion.benchmark_group("flecs");

    bench_loop_entities!(
        group,
        "ensure",
        ENTITY_COUNT
        ; (register_component_range, (C, 1, 1))
        ; (add_component_range, (C, 1, 1)) // Preparation
        ; (get_mut_component_range, (C, 1, 1)) // Benchmark
    );

    group.finish();
}

pub fn set_id(criterion: &mut Criterion) {
    let mut group = criterion.benchmark_group("flecs");

    bench_loop_entities!(
        group,
        "set_id",
        ENTITY_COUNT
        ; (register_component_range, (C, 1, 1))
        ; (add_component_range, (C, 1, 1)) // Preparation
        ; (set_component_range, (C, 1, 1)) // Benchmark
    );

    group.finish();
}

pub fn ensure_remove(criterion: &mut Criterion) {
    let mut group = criterion.benchmark_group("flecs");

    bench_loop_entities!(
        group,
        "ensure_remove",
        ENTITY_COUNT
        ; (register_component_range, (C, 1, 1))
        ; // preparation
        ; (set_component_range, (C, 1, 1)), (remove_component_range, (C, 1, 1))
    );

    bench_loop_entities!(
        group,
        "ensure_remove_16",
        ENTITY_COUNT
        ; (register_component_range, (C, 1, 16))
        ; // preparation
        ; (set_component_range, (C, 1, 16)), (remove_component_range, (C, 1, 16))
    );

    bench_loop_entities!(
        group,
        "ensure_remove_32",
        ENTITY_COUNT
        ; (register_component_range, (C, 1, 32))
        ; // preparation
        ; (set_component_range, (C, 1, 32)), (remove_component_range, (C, 1, 32))
    );

    group.finish();
}
