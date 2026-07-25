//! The core ECS API: worlds, entities, ids, components, queries and tables.
//!
//! This is the primary module of the crate. It contains everything needed to create a
//! [`World`], spawn and inspect entities, register and query components, and build
//! relationships between entities.
//!
//! # Where to start
//!
//! - **[`world::World`]** - The container for all ECS data. Create one with `World::new()`.
//! - **[`entity_view::EntityView`]** - A lightweight, `Copy`able handle to an entity, used to
//!   add/remove/get components, build relationships and traverse hierarchies.
//! - **[`entity_view::EntityViewGet`]** - Extension trait for reading component data off an
//!   `EntityView`.
//! - **[`id::Id`]** / **[`id_view::IdView`]** - Identifiers for anything that can be added to an
//!   entity: components, tags, pair (relationship) ids and id flags.
//! - **[`query::Query`]** / **[`query_builder`]** - Build and iterate queries over entities that
//!   match a set of components/relationships.
//! - **[`component_registration`]** - Traits and machinery for registering Rust types as ECS
//!   components.
//! - **[`table`]** - Direct access to the underlying archetype storage for advanced iteration.
//! - **[`observer`]** / **[`observer_builder`]** - React to entity/component lifecycle events.
//! - **[`event`]** - Emit and handle custom events.
//!
//! # See also
//!
//! - The [addons][crate::addons] module for optional, feature-gated functionality (systems,
//!   reflection, JSON, scripting, and more).
//! - [`prelude`][crate::prelude] for the recommended `use flecs_ecs::prelude::*;` glob import.

pub mod archetype;
pub mod builder;
pub mod c_types;
pub(crate) mod cloned_tuple;
pub mod component_registration;
pub mod components;
pub mod ecs_os_api;
pub mod entity;
pub mod entity_view;
pub mod event;
pub mod flecs;
pub(crate) mod get_tuple;
pub mod id;
pub mod id_view;
pub mod observer;
pub mod observer_builder;
pub mod query;
pub mod query_builder;
pub mod query_iter;
pub(crate) mod query_tuple;
#[cfg(feature = "flecs_safety_locks")]
mod safety_map;
pub mod sparse_query;
pub mod table;
pub mod term;
pub(crate) mod tuple_alias;
pub mod utility;
pub mod world;
pub mod world_ctx;

pub use archetype::Archetype;
#[doc(hidden)]
pub use builder::*;
#[doc(hidden)]
pub use c_types::*;
pub use cloned_tuple::ClonedTuple;
pub(crate) use cloned_tuple::*;
#[doc(hidden)]
pub use component_registration::*;
#[doc(inline)]
pub use components::*;
pub use entity::Entity;
pub use entity_view::EntityView;
pub use entity_view::EntityViewGet;
pub use event::EventBuilder;
pub use get_tuple::GetTuple;
pub(crate) use get_tuple::*;
pub use id::Id;
pub use id_view::IdView;
pub use observer::Observer;
pub use observer_builder::{ObserverBuilder, ObserverUpdater};
pub use query::{Query, QueryHandle};
pub use query_builder::*;
pub use query_iter::{ChainedIter, QueryIter};
pub(crate) use query_iter::IterGuard;
#[doc(hidden)]
pub use query_tuple::*;
#[cfg(feature = "flecs_safety_locks")]
pub(crate) use safety_map::*;
#[doc(hidden)]
pub use sparse_query::SparseQuery;
#[doc(hidden)]
pub use table::*;
#[doc(hidden)]
pub use term::*;
#[doc(hidden)]
pub use utility::*;
pub use world::AsyncStage;
pub(crate) use world::FlecsArray;
pub use world::World;
pub use world::WorldGet;
pub(crate) use world_ctx::*;
