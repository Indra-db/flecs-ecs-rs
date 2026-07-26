use crate::z_ignore_test_common::*;

// Import only the guard extension: the legacy each_iter used below stays on the
// stable QueryAPI (per the migration plan), so QueryIterCtxExt is kept out of
// scope to avoid a same-name ambiguity.
use flecs_ecs::experimental::prelude::EntityGuardExt;
use flecs_ecs::prelude::*;
// When an enumeration constant is added to an entity, it is added as a relationship
// pair where the relationship is the enum type, and the target is the constant. For
// example, this statement:
//   e.add(Color::Red)
//
// adds this relationship:
//   (Color, Color::Red)
//
// Enums are registered as exclusive relationships, which means that adding an
// enum constant will replace the previous constant for that enumeration:
//   e.add(Color::Green)
//
//  will replace Color::Red with Color::Green

//Regular C style enumerations are supported
//   rust style enum variants are not supported *yet* due to limitations in C flecs lib
//   where it expects each enum field to be 4 bytes.
//   I plan on adding support for rust style enums in the future.
#[derive(Component, Debug, PartialEq)]
#[repr(C)]
enum Tile {
    Grass,
    Sand,
    Stone,
}

#[derive(Component)]
#[repr(C)]
enum TileStatus {
    Free,
    Occupied,
}

fn main() {
    let world = World::new();

    // Create an entity with (Tile, Red) and (TileStatus, Free) relationships
    let tile = world
        .entity()
        .add_enum(Tile::Stone)
        .add_enum(TileStatus::Free);

    // (Tile, Tile.Stone), (TileStatus, TileStatus.Free)
    println!("{:?}", tile.archetype());

    // Replace (TileStatus, Free) with (TileStatus, Occupied)
    tile.add_enum(TileStatus::Occupied);

    // (Tile, Tile.Stone), (TileStatus, TileStatus.Occupied)
    println!("{:?}", tile.archetype());

    println!();

    // Check if the entity has the Tile relationship and the Tile::Stone pair
    println!("has tile enum: {}", tile.has(Tile::id())); // true
    println!(
        "is the enum from tile stone?: {}",
        tile.has_enum(Tile::Stone)
    ); // true

    // Get the current value of the enum
    if let Ok(tile) = tile.try_get::<&Tile>() {
        println!("is tile stone: {}", *tile == Tile::Stone); // true
    }

    // Create a few more entities that we can query
    world
        .entity()
        .add_enum(Tile::Grass)
        .add_enum(TileStatus::Free);

    world
        .entity()
        .add_enum(Tile::Sand)
        .add_enum(TileStatus::Occupied);

    println!();

    // Iterate all entities with a Tile relationship
    world
        .query::<()>()
        .with_enum_wildcard::<&Tile>()
        .build()
        .each_iter(|it, _, _| {
            let pair = it.pair(0);
            let tile_constant = pair.second_id();
            println!("{}", tile_constant.path().unwrap());
        });

    // Output:s:
    //  ::Tile::Stone
    //  ::Tile::Grass
    //  ::Tile::Sand

    println!();

    // Iterate only occupied tiles
    world
        .query::<()>()
        .with_enum_wildcard::<&Tile>()
        .with_enum(TileStatus::Occupied)
        .build()
        .each_iter(|it, _, _| {
            let pair = it.pair(0);
            let tile_constant = pair.second_id();
            println!("{}", tile_constant.path().unwrap());
        });

    // Output:s:
    //  ::Tile::Stone
    //  ::Tile::Sand

    println!();

    // Remove any instance of the TileStatus relationship
    tile.remove(TileStatus::id());

    // (Tile, Tile.Stone)
    println!("{:?}", tile.archetype());

    // Total Output:
    //  (relationships_enum.Tile,relationships_enum.Tile.Stone), (relationships_enum.TileStatus,relationships_enum.TileStatus.Free)
    //  (relationships_enum.Tile,relationships_enum.Tile.Stone), (relationships_enum.TileStatus,relationships_enum.TileStatus.Occupied)
    //
    //  has tile enum: true
    //  is the enum from tile stone?: true
    //  is tile stone: true
    //
    //  ::relationships_enum::Tile::Stone
    //  ::relationships_enum::Tile::Grass
    //  ::relationships_enum::Tile::Sand
    //
    //  ::relationships_enum::Tile::Stone
    //  ::relationships_enum::Tile::Sand
    //
    //  (relationships_enum.Tile,relationships_enum.Tile.Stone)
}

#[cfg(feature = "flecs_nightly_tests")]
#[test]
#[ignore = "is a hierarchy traversal not supported with new get callback"]
fn test() {
    let output_capture = OutputCapture::capture().unwrap();
    main();
    output_capture.test("relationships_enum".to_string());
}
