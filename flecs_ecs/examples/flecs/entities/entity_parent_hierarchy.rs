use crate::z_ignore_test_common::*;

use flecs_ecs::experimental::prelude::*;
use flecs_ecs::prelude::*;

// ChildOf hierarchies are optimized for scenarios with few parents that have
// lots of children. Parent hierarchies are optimized for scenarios with lots
// of parents that have a small number of children. See the documentation for
// more information:
//   https://www.flecs.dev/flecs/md_docs_2HierarchiesManual.html#hierarchy-storage
//
// This example is the same as the entity_hierarchy example, but instead of
// using ChildOf hierarchies it uses Parent hierarchies.

#[derive(Debug, Clone, Component)]
pub struct Position {
    pub x: f64,
    pub y: f64,
}

#[derive(Component)]
pub struct Star;

#[derive(Component)]
pub struct Planet;

#[derive(Component)]
pub struct Moon;

fn iterate_tree(entity: EntityView, position_parent: &Position) {
    // Print hierarchical name of entity & the entity type
    println!("{} [{}]", entity.path().unwrap(), entity.archetype());

    // Get the position of the entity through a guard, then compute the actual
    // position as an ordinary expression.
    let position = entity.get::<&Position>().unwrap();
    let actual_position = Position {
        x: position.x + position_parent.x,
        y: position.y + position_parent.y,
    };
    drop(position); // release the read guard before recursing into the children
    println!("{{{}, {}}}\n", actual_position.x, actual_position.y);

    // Iterate children recursively
    entity.each_child(|child| {
        iterate_tree(child, &actual_position);
    });
}

fn main() {
    let world = World::new();

    // Create a simple hierarchy. Unlike the ChildOf hierarchy, parent
    // hierarchies store the parent in the non-fragmenting Parent component.

    let sun = world
        .entity_named("Sun")
        .add(Star)
        .set(Position { x: 1.0, y: 1.0 });

    world
        .entity_named_with_parent(sun, "Mercury")
        .add(Planet)
        .set(Position { x: 1.0, y: 1.0 });

    world
        .entity_named_with_parent(sun, "Venus")
        .add(Planet)
        .set(Position { x: 2.0, y: 2.0 });

    let earth = world
        .entity_named_with_parent(sun, "Earth")
        .add(Planet)
        .set(Position { x: 3.0, y: 3.0 });

    let moon = world
        .entity_named_with_parent(earth, "Moon")
        .add(Moon)
        .set(Position { x: 0.1, y: 0.1 });

    // Is the Moon a child of the Earth?
    println!(
        "Child of Earth? {}\n",
        moon.has((flecs::ChildOf::ID, earth))
    );

    // Lookup the moon by name
    let e = world.lookup("Sun::Earth::Moon");
    println!("Moon found: {}\n", e.path().unwrap());

    // Do a depth-first walk of the tree
    iterate_tree(sun, &Position { x: 0.0, y: 0.0 });

    // Output:
    //  Child of Earth? true
    //
    //  Moon found: ::Sun::Earth::Moon
    //
    //  ::Sun [Star, Position, (Identifier,Name)]
    //  {1, 1}
    //
    //  ::Sun::Mercury [Parent, Position, Planet, (Identifier,Name), (ParentDepth,@1)]
    //  {2, 2}
    //
    //  ::Sun::Venus [Parent, Position, Planet, (Identifier,Name), (ParentDepth,@1)]
    //  {3, 3}
    //
    //  ::Sun::Earth [Parent, Position, Planet, (Identifier,Name), (ParentDepth,@1)]
    //  {4, 4}
    //
    //  ::Sun::Earth::Moon [Parent, Position, Moon, (Identifier,Name), (ParentDepth,@2)]
    //  {4.1, 4.1}
}

#[cfg(feature = "flecs_nightly_tests")]
#[test]
fn test() {
    let output_capture = OutputCapture::capture().unwrap();
    main();
    output_capture.test("entity_parent_hierarchy".to_string());
}
