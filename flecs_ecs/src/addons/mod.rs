#[cfg(feature = "flecs_app")]
pub mod app;

#[cfg(feature = "flecs_doc")]
pub mod doc;

#[cfg(feature = "flecs_meta")]
pub mod meta;

#[cfg(feature = "flecs_module")]
pub mod module;

#[cfg(feature = "flecs_system")]
pub mod system;

#[cfg(feature = "flecs_pipeline")]
pub mod pipeline;

pub mod experimental;

#[cfg(feature = "flecs_stats")]
pub mod stats;
