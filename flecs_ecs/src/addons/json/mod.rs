//! The JSON addon provides serialization and deserialization of ECS data to/from JSON.
//!
//! This addon enables converting component values, entities, and query results to JSON
//! strings, and parsing JSON back into component data. Entity identifiers, enumerations,
//! and bitmasks are encoded as strings for human readability.
//!
//! # Features
//!
//! - **Component Serialization**: Convert component data to JSON
//! - **Entity Serialization**: Serialize entities with all their components
//! - **Query Results**: Convert query results to JSON arrays
//! - **Deserialization**: Parse JSON strings into component values
//! - **Type Safety**: Leverages reflection metadata for accurate serialization
//!
//! # Usage
//!
//! Components must have reflection metadata (using `#[flecs(meta)]`) to be serialized:
//!
//! ```
//! use flecs_ecs::prelude::*;
//! use flecs_ecs::experimental::prelude::*;
//!
//! #[derive(Component)]
//! #[flecs(meta)]
//! struct Position {
//!     x: f32,
//!     y: f32,
//! }
//!
//! let world = World::new();
//!
//! // Create entity with Position
//! let entity = world.entity().set(Position { x: 10.0, y: 20.0 });
//!
//! // Serialize component to JSON
//! {
//!     let pos = entity.get_ref::<&Position>().unwrap();
//!     let json = world.to_json::<Position>(&*pos);
//!     println!("Position: {}", json);
//!     // Output: Position: {"x":10, "y":20}
//! }
//!
//! // Serialize entire entity to JSON
//! let entity_json = entity.to_json(None);
//! println!("Entity: {}", entity_json);
//! // Output: Entity: {"name":"#123", "components":{"Position":{"x":10, "y":20}}}
//! ```
//!
//! # JSON Format
//!
//! For a detailed description of the JSON format used for serialization, including
//! how different types are encoded and the structure of serialized data, see the
//! [Flecs Remote API documentation](https://www.flecs.dev/flecs/md_docs_2FlecsRemoteApi.html).
//!
//! # Examples
//!
//! For comprehensive JSON serialization examples, see the [`examples/flecs/reflection/`]
//! directory, which includes:
//! - `reflection_basics_json.rs` - Basic JSON serialization
//! - `reflection_query_to_json.rs` - Serializing query results
//! - `reflection_query_to_custom_json.rs` - Custom JSON formatting
//! - `reflection_world_ser_deser.rs` - World serialization/deserialization
//!
//! [`examples/flecs/reflection/`]: https://github.com/Indra-db/Flecs-Rust/tree/main/flecs_ecs/examples/flecs/flecs/reflection
//!
//! # See also
//!
//! - [`World::to_json()`](crate::core::World::to_json) - Serialize component data to JSON
//! - [`EntityView::to_json()`](crate::core::EntityView::to_json) - Serialize entity to JSON
//! - [Flecs Remote API](https://www.flecs.dev/flecs/md_docs_2FlecsRemoteApi.html) - JSON format specification
//! - [`meta`](crate::addons::meta) - Reflection addon (required for JSON serialization)

/*
using from_json_desc_t = ecs_from_json_desc_t;
using entity_to_json_desc_t = ecs_entity_to_json_desc_t;
using iter_to_json_desc_t = ecs_iter_to_json_desc_t;
*/

use crate::sys;

use super::meta::FetchedId;

#[cfg(feature = "std")]
extern crate std;

extern crate alloc;

pub type FromJsonDesc = sys::ecs_from_json_desc_t;
pub type WorldToJsonDesc = sys::ecs_world_to_json_desc_t;
pub type EntityToJsonDesc = sys::ecs_entity_to_json_desc_t;
pub type IterToJsonDesc = sys::ecs_iter_to_json_desc_t;

mod entity_view;
mod world;
