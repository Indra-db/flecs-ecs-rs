//! `EntityViews` are wrappers around an [`Entity`][super::Entity] id with the world. It provides methods to build and interact with entities.

mod bulk_entity_builder;
mod entity_view_const;
mod entity_view_impl;
mod entity_view_mut;
mod macros;

pub use entity_view_const::EntityView;
