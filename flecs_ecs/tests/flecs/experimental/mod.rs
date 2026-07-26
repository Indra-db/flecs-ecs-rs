#![allow(dead_code)]

use flecs_ecs::macros::*;

mod bundle;
mod chunks;
mod disjoint;
#[cfg(feature = "flecs_safety_locks")]
mod entity_guard;
#[cfg(feature = "flecs_safety_locks")]
mod guard_pin;
mod exclusive;

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
