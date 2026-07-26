#![allow(unused_imports)]
#![allow(unused)]

use core::time::Duration;

use criterion::{Criterion, criterion_group, criterion_main};
pub use seq_macro::seq;

pub mod common_bench;

mod add_remove_bench;
mod bundle_bench;
mod commands_bench;
mod create_delete_bench;
mod parenting_names_bench;

mod get_bench;
mod has_bench;
mod observer_bench;
mod query_bench;
mod set_bench;

use add_remove_bench::*;
use bundle_bench::*;
use commands_bench::*;
use create_delete_bench::*;

use get_bench::*;
use has_bench::*;
use observer_bench::*;
use parenting_names_bench::*;
use query_bench::*;
use set_bench::*;

fn ecs_default_criterion() -> Criterion {
    let mut criterion_config = Criterion::default().configure_from_args();
    criterion_config = criterion_config
        .warm_up_time(Duration::from_millis(200))
        .measurement_time(Duration::from_secs(1))
        //.sample_size(25)
        .noise_threshold(0.02)
        .confidence_level(0.95)
        .significance_level(0.05)
        .without_plots();
    criterion_config
}

criterion_main!(
    g_add_remove,
    g_commands,
    g_create_delete_entities,
    g_get,
    g_parenting,
    g_has,
    g_set,
    g_observers,
    g_query,
    g_bundle
);

criterion_group!(
    name = g_bundle;
    config = ecs_default_criterion();
    targets =
    bundle_spawn,
    bundle_spawn_batch,
    bundle_insert
);

criterion_group!(
    name = g_query;
    config = ecs_default_criterion();
    targets =
    query_iter,
    query_each,
    query_experimental,
);

criterion_group!(
    name = g_has;
    config = ecs_default_criterion();
    targets =
    has,
    owns
);

criterion_group!(
    name = g_parenting;
    config = ecs_default_criterion();
    targets =
    change_parent,
    lookup_depth,
    set_name,
);

criterion_group!(
    name = g_get;
    config = ecs_default_criterion();
    targets =
    get,
    get_pair,
    get_mut,
    get_target,
    get_parent,
    get_inherited_w_depth,
);

criterion_group!(
    name = g_commands;
    config = ecs_default_criterion();
    targets =
    add_remove_cmd,
    add_existing_cmd,
    create_delete_entities_cmd,
);

criterion_group!(
    name = g_add_remove;
    config = ecs_default_criterion();
    targets =
    add_remove,
    add_remove_hooks,
    add_existing,
    add_remove_1_tag_to_entity_with_n_components,
    add_remove_override,
    toggle_component,
);

criterion_group!(
    name = g_set;
    config = ecs_default_criterion();
    targets =
    set,
    set_remove
);

criterion_group!(
    name = g_create_delete_entities;
    config = ecs_default_criterion();
    targets =
    create_w_add_in_observer,
    create_delete,
    entity_new_delete,
    entity_new_w_name_delete,
    create_children_w_reachable,
    create_delete_tree,
    //c_create_delete_tree
    instantiate_delete_tree
);

criterion_group!(
    name = g_observers;
    config = ecs_default_criterion();
    targets =
    emit,
    emit_propagate,
    emit_forward,
    modified
);
