#![allow(non_camel_case_types)]
use crate::z_ignore_test_common::*;

use flecs_ecs::experimental::prelude::*;
use flecs_ecs::prelude::*;
// A group iterator iterates over a single group of a grouped query (see the
// group_by example for more details). This can be useful when an application
// may need to match different entities based on the context of the game, such
// as editor mode, day/night, inside/outside or location in the world.
//
// One example is that of an open game which is divided up into world
// cells. Even though a world may contain many entities, only the entities in
// cells close to the player need to be processed.
//
// Instead of creating a cached query per world cell, which could be expensive
// as there are more caches to keep in sync, applications can create a single
// query grouped by world cell, and use group iterators to only iterate the
// necessary cells.

// A world cell relationship with four cells

#[derive(Debug, Component)]
struct WorldCell;

#[derive(Debug, Component)]
struct Cell_0_0;

#[derive(Debug, Component)]
struct Cell_0_1;

#[derive(Debug, Component)]
struct Cell_1_0;

#[derive(Debug, Component)]
struct Cell_1_1;

// Npc tags
#[derive(Debug, Component)]
struct Npc;

#[derive(Debug, Component)]
struct Merchant;

#[derive(Debug, Component)]
struct Soldier;

#[derive(Debug, Component)]
struct Beggar;

#[derive(Debug, Component)]
struct Mage;

fn main() {
    let mut world = World::new();

    // Create npc's in world cell 0_0
    world
        .entity()
        .add((WorldCell, Cell_0_0))
        .add(Merchant)
        .add(Npc);
    world
        .entity()
        .add((WorldCell, Cell_0_0))
        .add(Merchant)
        .add(Npc);

    // Create npc's in world cell 0_1
    world
        .entity()
        .add((WorldCell, Cell_0_1))
        .add(Beggar)
        .add(Npc);
    world
        .entity()
        .add((WorldCell, Cell_0_1))
        .add(Soldier)
        .add(Npc);

    // Create npc's in world cell 1_0
    world.entity().add((WorldCell, Cell_1_0)).add(Mage).add(Npc);
    world
        .entity()
        .add((WorldCell, Cell_1_0))
        .add(Beggar)
        .add(Npc);

    // Create npc's in world cell 1_1
    world
        .entity()
        .add((WorldCell, Cell_1_1))
        .add(Soldier)
        .add(Npc);

    let mut query = world.query::<()>().with(&Npc).group_by(WorldCell).build();

    // Iterate all tables
    println!("All tables");

    query.run_exclusive(&mut world, |mut iter| {
        while iter.next() {
            let world = iter.world();
            let group = world.entity_from_id(iter.group_id());
            println!(
                "group: {:?} - Table [{}]",
                group.path().unwrap(),
                iter.table().unwrap().to_string().unwrap()
            );
        }
    });

    println!();

    println!("Tables for cell 1_0:");

    query.with_group(Cell_1_0).run_exclusive(&mut world, |mut iter| {
        while iter.next() {
            let world = iter.world();
            let group = world.entity_from_id(iter.group_id());
            println!(
                "group: {:?} - Table [{}]",
                group.path().unwrap(),
                iter.table().unwrap().to_string().unwrap()
            );
        }
    });

    // Output:
    //  All tables
    //  group: "::Cell_0_0" - Table [Merchant, Npc, (WorldCell,Cell_0_0)]
    //  group: "::Cell_0_1" - Table [Npc, Beggar, (WorldCell,Cell_0_1)]
    //  group: "::Cell_0_1" - Table [Npc, Soldier, (WorldCell,Cell_0_1)]
    //  group: "::Cell_1_0" - Table [Npc, Mage, (WorldCell,Cell_1_0)]
    //  group: "::Cell_1_0" - Table [Npc, Beggar, (WorldCell,Cell_1_0)]
    //  group: "::Cell_1_1" - Table [Npc, Soldier, (WorldCell,Cell_1_1)]

    //  Tables for cell 1_0:
    //  group: "::Cell_1_0" - Table [Npc, Mage, (WorldCell,Cell_1_0)]
    //  group: "::Cell_1_0" - Table [Npc, Beggar, (WorldCell,Cell_1_0)]
}

#[cfg(feature = "flecs_nightly_tests")]
#[test]
fn test() {
    let output_capture = OutputCapture::capture().unwrap();
    main();
    output_capture.test("query_group_iter".to_string());
}
