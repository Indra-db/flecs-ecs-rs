#![allow(dead_code)]
use crate::z_ignore_test_common::*;

use flecs_ecs::experimental::prelude::*;
use flecs_ecs::prelude::*;

#[derive(Debug, Component)]
pub struct Position {
    pub x: f32,
    pub y: f32,
}

#[derive(Debug, Component)]
struct Star;

#[derive(Debug, Component)]
struct Planet;

#[derive(Debug, Component)]
struct Moon;

fn iterate_tree(entity: EntityView, position_parent: &Position) {
    // Print hierarchical name of entity & the entity type
    println!("{} [{:?}]", entity.path().unwrap(), entity.archetype());

    // A guard read borrows the component and the result composes like ordinary
    // Rust: no callback whose only job is to return a value.
    let pos = entity.get::<&Position>().unwrap();
    let pos_actual = Position {
        x: pos.x + position_parent.x,
        y: pos.y + position_parent.y,
    };
    drop(pos); // release the read guard before recursing into the children

    // Print the position
    println!("{pos_actual:?}");

    entity.each_child(|child| {
        iterate_tree(child, &pos_actual);
    });
}

fn main() {
    let world = World::new();

    // Create a simple hierarchy.
    // Hierarchies use ECS relationships and the builtin flecs::ChildOf relationship to
    // create entities as children of other entities.

    let sun = world
        .entity_named("Sun")
        .add(Star)
        .set(Position { x: 1.0, y: 1.0 });

    world
        .entity_named_child_of(sun, "Mercury")
        .set(Position { x: 1.0, y: 1.0 })
        .add(Planet);

    world
        .entity_named_child_of(sun, "Venus")
        .set(Position { x: 2.0, y: 2.0 })
        .add(Planet);

    let earth = world
        .entity_named_child_of(sun, "Earth")
        .set(Position { x: 3.0, y: 3.0 })
        .add(Planet);

    let moon = world
        .entity_named_child_of(earth, "Moon")
        .set(Position { x: 0.1, y: 0.1 })
        .add(Moon);

    // Is the Moon a child of the Earth?
    println!(
        "Is the Moon a child of the Earth? {} / {}",
        moon.has((flecs::ChildOf, earth)), //or you can do
        moon.has((flecs::ChildOf, earth))
    );

    println!();

    // Do a depth-first traversal of the tree
    iterate_tree(sun, &Position { x: 0.0, y: 0.0 });

    // Output:
    //  Is the Moon a child of the Earth? true / true
    //  ::Sun [Star, Position, (Identifier,Name)]
    //  Position { x: 1.0, y: 1.0 }
    //  ::Sun::Mercury [Position, Planet, (Identifier,Name), (ChildOf,Sun)]
    //  Position { x: 2.0, y: 2.0 }
    //  ::Sun::Venus [Position, Planet, (Identifier,Name), (ChildOf,Sun)]
    //  Position { x: 3.0, y: 3.0 }
    //  ::Sun::Earth [Position, Planet, (Identifier,Name), (ChildOf,Sun)]
    //  Position { x: 4.0, y: 4.0 }
    //  ::Sun::Earth::Moon [Position, Moon, (Identifier,Name), (ChildOf,Sun.Earth)]
    //  Position { x: 4.1, y: 4.1 }
}

#[cfg(feature = "flecs_nightly_tests")]
#[test]
fn test() {
    let output_capture = OutputCapture::capture().unwrap();
    main();
    output_capture.test("entity_hierarchy".to_string());
}
