#![allow(unused_imports)]
#![allow(warnings)]
use crate::z_ignore_test_common::*;

use flecs_ecs::experimental::prelude::*;
use flecs_ecs::prelude::*;

// Queries have a builtin mechanism for tracking changes per matched table. This
// is a cheap way of eliminating redundant work, as many entities can be skipped
// with a single check.
//
// This example shows how to use change tracking to skip tables that have not
// changed. This is useful when you have a query that writes to a component, but
// only want to write to tables that have changed.

#[derive(Debug, Component)]
pub struct Position {
    pub x: f32,
    pub y: f32,
}

#[derive(Debug, Component)]
pub struct WorldPosition {
    pub x: f32,
    pub y: f32,
}

// this is to make sure independent entity is in a different archetype
#[derive(Component)]
pub struct DummyTag;

// This system updates the WorldPosition component based on the Position and
// the optional parent's Position.
fn update_transforms(mut it: TableIter<true, ()>) {
    while it.next() {
        // With the it.changed() function we can check if the table we're
        // currently iterating has changed since last iteration. If the table
        // has not changed, we can skip it.
        if !it.is_changed() {
            println!("skip archetype: {:?}", it.archetype().unwrap());
            it.skip();
            continue;
        }

        println!("non-skip archetype: {:?}", it.archetype().unwrap());

        let pos = it.field::<Position>(0);
        let parent_pos = it.get_field::<Position>(1); // Optional
        let mut world_pos = it.field_mut::<WorldPosition>(2);

        match parent_pos {
            None => {
                for i in it.iter() {
                    world_pos[i].x = pos[i].x;
                    world_pos[i].y = pos[i].y;
                }
            }
            Some(parent_pos) => {
                for i in it.iter() {
                    world_pos[i].x = pos[i].x + parent_pos[i].x;
                    world_pos[i].y = pos[i].y + parent_pos[i].y;
                }
            }
        }
    }
}

fn print_world_positions(entity: EntityView, world_pos: &WorldPosition) {
    println!("{}: {:?}", entity.name(), world_pos);
}

fn main() {
    let mut world = World::new();

    let transform_query = world
        .query_named::<(&Position, Option<&Position>, &mut WorldPosition)>("update_transforms")
        .term_at(1)
        .parent()
        .build();

    let print_world_pos = world.query::<&WorldPosition>().build();

    // create entities with and without parent
    let parent = world
        .entity_named("parent")
        .set(Position { x: 100.0, y: 200.0 })
        .set(WorldPosition { x: 0.0, y: 0.0 });

    let child = world
        .entity_named("child")
        .set(Position { x: 10.0, y: 20.0 })
        .set(WorldPosition { x: 0.0, y: 0.0 })
        .child_of(parent)
        .id();

    let independent = world
        .entity_named("independent")
        .set(Position { x: 50.0, y: 30.0 })
        .set(WorldPosition { x: 0.0, y: 0.0 })
        // this is to make sure independent entity is in a different archetype
        .add(DummyTag)
        .id();

    // Since this is the first time the query is iterated, all tables
    // will show up as changed and not skipped
    transform_query.run_exclusive(&mut world, update_transforms);

    // Output:
    //  non-skip archetype: Position, WorldPosition
    //  non-skip archetype: Position, WorldPosition, (ChildOf,parent)
    //  non-skip archetype: Position, WorldPosition, DummyTag

    // Set the child position to a new value. This will change the table that
    // the child entity is in, which will cause the query to return true when
    // we call changed().
    world.entity_from_id(child).set(Position { x: 110.0, y: 210.0 });

    // When we iterate the query, we'll see that one table has changed and thus not skipped
    transform_query.run_exclusive(&mut world, update_transforms);

    println!();

    // Output:
    //  skip archetype: Position, WorldPosition
    //  non-skip archetype: Position, WorldPosition, (ChildOf,parent)
    //  skip archetype: Position, WorldPosition, DummyTag

    print_world_pos.each_entity_exclusive(&mut world, print_world_positions);

    // Output:
    //  parent: WorldPosition { x: 110.0, y: 210.0 }
    //  independent: WorldPosition { x: 50.0, y: 30.0 }
    //  child: WorldPosition { x: 120.0, y: 230.0 }

    // now the same, but for independent entity, which is in a different archetype.
    world.entity_from_id(independent).set(Position { x: 60.0, y: 40.0 });

    transform_query.run_exclusive(&mut world, update_transforms);

    println!();

    // Output:
    //  skip archetype: Position, WorldPosition
    //  skip archetype: Position, WorldPosition, (ChildOf,parent)
    //  non-skip archetype: Position, WorldPosition, DummyTag

    print_world_pos.each_entity_exclusive(&mut world, print_world_positions);

    // Output:
    //  parent: WorldPosition { x: 110.0, y: 210.0 }
    //  independent: WorldPosition { x: 60.0, y: 40.0 }
    //  child: WorldPosition { x: 120.0, y: 230.0 }
}

#[cfg(feature = "flecs_nightly_tests")]
#[test]
fn test() {
    let output_capture = OutputCapture::capture().unwrap();
    main();
    output_capture.test("query_change_tracking_1".to_string());
}
