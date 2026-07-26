#![allow(dead_code)]

use flecs_ecs::macros::*;

mod batches;
mod bundle;
mod entity_mut;
#[cfg(feature = "flecs_safety_locks")]
mod cached_ref;
mod chunks;
mod disjoint;
#[cfg(feature = "flecs_safety_locks")]
mod entity_guard;
mod iter_ctx;
#[cfg(feature = "flecs_safety_locks")]
mod guard_pin;
#[cfg(feature = "flecs_safety_locks")]
mod guard_revalidation;
#[cfg(feature = "flecs_safety_locks")]
mod bypass_paths;
#[cfg(feature = "flecs_safety_locks")]
mod singleton;
mod exclusive;
mod system_run;
mod system_with;
mod typed_ctx;

#[derive(Component, Clone, Debug, Default)]
struct Position {
    x: i32,
    y: i32,
}

#[derive(Component, Clone, Debug, Default)]
struct Velocity {
    x: i32,
    y: i32,
}

#[derive(Component, Clone, Debug, Default, PartialEq)]
struct Health(i32);
