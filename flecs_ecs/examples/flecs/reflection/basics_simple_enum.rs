use crate::z_ignore_test_common::*;

use flecs_ecs::prelude::*;

#[derive(Debug, Component)]
#[repr(C)]
#[meta]
pub enum Color {
    Red,
    Green,
    Blue,
}

#[derive(Debug, Component)]
#[meta]
pub struct TypeWithEnum {
    pub color: Color,
}

#[test]
fn main() {
    let mut world = World::new();

    // Register the Color component
    world.component::<Color>().meta();
    /* Alternatively, you can do it manually like so (without the derive macro)
    .constant("Red", Color::Red as i32)
    .constant("Green", Color::Green as i32)
    .constant("Blue", Color::Blue as i32);
    */

    // Register the TypeWithEnum component
    world.component::<TypeWithEnum>().meta();

    /* Alternatively, you can do it manually like so (without the derive macro)
    .member::<Color>("color", 1, offset_of!(TypeWithEnum, color));
     */

    // Create a new entity
    let e = world.entity().set(TypeWithEnum {
        color: Color::Green,
    });

    // Convert TypeWithEnum component to flecs expression string
    e.get::<&TypeWithEnum>(|p| {
        let expr: String = world.to_expr(p);
        println!("TypeWithEnum: {}", expr);
    });

    // Output:
    //  TypeWithEnum: {color: Green}
}