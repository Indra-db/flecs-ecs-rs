use std::sync::OnceLock;

use flecs_ecs_bridge::core::component::{CachedComponentData, ComponentData};
use flecs_ecs_bridge::core::world::World;
use flecs_ecs_bridge_derive::Component;

//use flecs_ecs_bridge_derive::print_foo;

#[macro_use]
extern crate debug_here;

#[derive(Clone, Default)]
struct Test {
    x: i32,
    v: Vec<i32>,
}

impl Drop for Test {
    fn drop(&mut self) {
        println!("dropped");
    }
}
#[derive(Clone, Default, Component)]
struct Test1 {
    x: i32,
    v: Test,
}

#[derive(Clone, Default, Component)]
struct Test2 {}

fn main() {
    println!("Hello, world!");
    //debug_here!();
    let world = World::new();
    let world2 = World::new();

    //print id of Test1 and Test2
    println!("Test1 id: {}", Test1::get_id(world.world));
    println!("Test2 id: {}", Test2::get_id(world.world));

    println!("Test2 id: {}", Test2::get_id(world2.world));
}
