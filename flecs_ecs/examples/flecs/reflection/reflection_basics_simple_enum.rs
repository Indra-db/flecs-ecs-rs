use crate::z_ignore_test_common::*;

use flecs_ecs::experimental::prelude::*;
use flecs_ecs::prelude::*;

#[derive(Debug, Component)]
#[repr(C)]
#[flecs(meta)]
pub enum Color {
    Red,
    Green,
    Blue,
}

#[derive(Debug, Component)]
#[flecs(meta)]
pub struct TypeWithEnum {
    pub color: Color,
}

fn main() {
    let mut world = World::new();

    /* Alternatively without the meta attribute,
    you can do it manually like so (without the derive macro)
    .constant("Red", Color::Red as i32)
    .constant("Green", Color::Green as i32)
    .constant("Blue", Color::Blue as i32);
    */

    /* Alternatively without the meta attribute,
    you can do it manually like so (without the derive macro)
    .member(Color::id(),"color", 1, core::mem::offset_of!(TypeWithEnum, color));
     */

    // Create a new entity
    let e = world.entity().set(TypeWithEnum {
        color: Color::Green,
    });

    // Convert TypeWithEnum component to flecs expression string
    let p = e.get::<&TypeWithEnum>().unwrap();
    let expr: String = world.to_expr(&*p);
    println!("TypeWithEnum: {expr}");

    // Output:
    //  TypeWithEnum: {color: Green}
}

#[cfg(feature = "flecs_nightly_tests")]
#[test]
fn test() {
    let output_capture = OutputCapture::capture().unwrap();
    main();
    output_capture.test("reflection_basics_simple_enum".to_string());
}
