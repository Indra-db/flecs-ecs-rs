#![allow(dead_code)]

use flecs_ecs::macros::*;

#[cfg(feature = "flecs_safety_locks")]
mod entity_guard;

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

#[derive(Component, Clone, Debug, Default)]
struct Health(i32);
