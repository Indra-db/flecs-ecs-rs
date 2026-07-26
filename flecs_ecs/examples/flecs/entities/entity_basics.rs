use crate::z_ignore_test_common::*;

use flecs_ecs::experimental::prelude::*;
use flecs_ecs::prelude::*;

#[derive(Debug, Component)]
pub struct Position {
    pub x: f32,
    pub y: f32,
}

#[derive(Debug, Component)]
pub struct Velocity {
    pub x: f32,
    pub y: f32,
}

#[derive(Component)]
pub struct Walking;

fn main() {
    let mut world = World::new();

    // Create an entity with name Bob
    let bob = world
        .entity_named("Bob")
        // The set operation finds or creates a component, and sets it.
        // Components are automatically registered with the world
        .set(Position { x: 10.0, y: 20.0 })
        // The add operation adds a component without setting a value. This is
        // useful for tags, or when adding a component with its default value.
        .add(Walking);

    // Get the value for the Position component. get_ref returns an RAII guard
    // borrowed straight from storage instead of running a callback.
    // - a required &Position returns None (via try_get_ref, an AccessError) when
    //   absent; wrapping the term in Option makes a missing component observable.
    // - a single Option term is spelled as a one-element tuple.
    let (pos,) = bob.get_ref::<(Option<&Position>,)>().unwrap();
    if let Some(pos) = &pos {
        println!("Bob's position: {pos:?}");
    }
    drop(pos); // release the read guard before writing Position again

    // Overwrite the value of the Position component
    bob.set(Position { x: 20.0, y: 30.0 });

    // Create another named entity
    let alice = world
        .entity_named("Alice")
        .set(Position { x: 10.0, y: 20.0 });

    // Add a tag after entity is created
    alice.add(Walking);

    // Print all of the components the entity has. This will output:
    //    Position, Walking, (Identifier,Name)
    println!("[{}]", alice.archetype());

    // Remove tag
    alice.remove(Walking);

    // Iterate all entities with position. Query iteration on the exclusive
    // register borrows &mut World, which proves no guard is outstanding and lets
    // the closure receive plain references with no lock traffic.
    let q = world.new_query::<&Position>();
    q.each_entity_exclusive(&mut world, |entity, pos| {
        println!("{} has {:?}", entity.name(), pos);
    });

    // Output:
    //  Bob's position: Position { x: 10.0, y: 20.0 }
    //  [Position, Walking, (Identifier,Name)]
    //  Alice has Position { x: 10.0, y: 20.0 }
    //  Bob has Position { x: 20.0, y: 30.0 }
}

#[cfg(feature = "flecs_nightly_tests")]
#[test]
fn test() {
    let output_capture = OutputCapture::capture().unwrap();
    main();
    output_capture.test("entity_basics".to_string());
}
