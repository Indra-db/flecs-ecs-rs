use crate::z_ignore_test_common::*;

use flecs_ecs::experimental::prelude::*;
use flecs_ecs::prelude::*;

#[derive(Component)]
#[flecs(meta)]
pub struct TypeWithEntity {
    pub e: Entity,
}

fn main() {
    let mut world = World::new();

    /* Alternatively, you can do it manually like so (without the derive macro)
    .member(Entity::id(),"e", 1, core::mem::offset_of!(TypeWithEntity, e));
    */

    let bar = world.entity_named("bar");

    // Create a new entity with the TypeWithEntity component
    let e = world.entity().set(TypeWithEntity { e: bar.into() });

    // Convert TypeWithEntity component to flecs expression string
    let p = e.get_ref::<&TypeWithEntity>().unwrap();
    let expr: String = world.to_expr(&*p);
    println!("TypeWithEntity: {expr}");

    // Output:
    //  TypeWithEntity: {e: foo}
}
