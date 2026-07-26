#![allow(dead_code)]
//! integration tests for the public API of the `flecs_ecs` crate
//! split into a single module due to the benefits it provides.
//! see <`https://matklad.github.io/2021/02/27/delete-cargo-integration-tests.html/`> for more information.
extern crate alloc;

pub mod common_test;

mod abi_test;
mod addons_misc_test;
mod aliasing_test;
#[cfg(feature = "flecs_app")]
mod app_test;
mod clone_default_impl_test;
mod component_index_growth_test;
mod component_lifecycle_test;
mod component_test;
mod component_traits_test;
mod derive_attr_component_traits;
mod entity_bulk_rust_test;
mod entity_rust_test;
mod entity_test;
mod entity_view_soundness_test;
mod enum_test;
mod eq_test;
mod event_test;
mod field_safety_rust_test;
mod flecs_docs_test;
mod flecs_ids;
mod implicit_components_test;
mod is_ref_test;
mod iterable_test;
mod meta_macro_test;
mod meta_test;
mod meta_trait_test;
mod module_test;
mod non_send_component_test;
mod observer_rust_test;
mod observer_test;
mod ordered_children_test;
mod pairs_test;
mod paths_test;
mod pretty_function_test;
mod query_builder_test;
mod query_rust_test;
mod query_test;
mod refs_test;
#[cfg(feature = "flecs_query_rust_traits")]
mod rust_trait_test;
#[cfg(feature = "flecs_safety_locks")]
mod safety;
#[cfg(feature = "flecs_experimental")]
mod experimental;
mod singleton_test;
mod soundness_test;
#[cfg(feature = "flecs_safety_locks")]
mod sys_bindings_test;
mod system_builder_test;
mod system_test;
mod table_test;
mod union_test;
mod world_factory_test;
mod world_test;
