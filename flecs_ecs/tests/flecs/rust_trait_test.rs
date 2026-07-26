#![allow(dead_code)]
use flecs_ecs::core::*;
use flecs_ecs::experimental::prelude::EntityGuardExt;
use flecs_ecs::macros::*;

pub trait Shapes {
    fn calculate(&self) -> u64;
    fn double(&mut self);
}

ecs_rust_trait!(Shapes);

#[derive(Component)]
pub struct Circle {
    value: u64,
}

impl Shapes for Circle {
    fn calculate(&self) -> u64 {
        self.value
    }
    fn double(&mut self) {
        self.value *= 2;
    }
}

#[derive(Component)]
pub struct Square {
    value: u64,
}

impl Shapes for Square {
    fn calculate(&self) -> u64 {
        self.value
    }
    fn double(&mut self) {
        self.value *= 2;
    }
}

#[test]
fn rust_trait_cast_dispatches_to_concrete_type() {
    let world = World::new();

    ShapesTrait::register_vtable::<Circle>(&world);
    ShapesTrait::register_vtable::<Square>(&world);

    world.entity().set(Circle { value: 1 });
    world.entity().set(Square { value: 2 });

    let query = world.query::<()>().with(ShapesTrait::id()).build();

    let mut total = 0;
    query.run(|mut it| {
        while it.next() {
            for i in it.iter() {
                let e = it.get_entity(i).unwrap();
                let id = it.id(0);
                let shape = unsafe { ShapesTrait::cast(e, id) };
                total += shape.calculate();
            }
        }
    });

    assert_eq!(total, 3);
}

#[test]
fn rust_trait_cast_mut_mutates_component() {
    let world = World::new();

    ShapesTrait::register_vtable::<Circle>(&world);

    let entity = world.entity().set(Circle { value: 3 });

    let query = world.query::<()>().with(ShapesTrait::id()).build();

    query.run(|mut it| {
        while it.next() {
            for i in it.iter() {
                let e = it.get_entity(i).unwrap();
                let id = it.id(0);
                let shape = unsafe { ShapesTrait::cast_mut(e, id) };
                shape.double();
            }
        }
    });

    {
        let circle = entity.get::<&Circle>().unwrap();
        assert_eq!(circle.value, 6);
    };
}

#[test]
#[should_panic(expected = "entity does not have the component")]
fn rust_trait_cast_panics_on_missing_component() {
    let world = World::new();

    ShapesTrait::register_vtable::<Circle>(&world);
    ShapesTrait::register_vtable::<Square>(&world);

    let entity = world.entity().set(Circle { value: 1 });
    let square_id = IdView::new_from_id(&world, world.component_id::<Square>());

    let _shape = unsafe { ShapesTrait::cast(entity, square_id) };
}
