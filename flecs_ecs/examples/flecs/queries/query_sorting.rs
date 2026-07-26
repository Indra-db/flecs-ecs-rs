use crate::z_ignore_test_common::*;

use flecs_ecs::experimental::prelude::*;
use flecs_ecs::prelude::*;

#[derive(Debug, Component)]
pub struct Position {
    pub x: f32,
    pub y: f32,
}

fn compare_position(_e1: Entity, p1: &Position, _e2: Entity, p2: &Position) -> i32 {
    (p1.x > p2.x) as i32 - (p1.x < p2.x) as i32
}

fn print_query(query: &Query<&Position>, world: &mut World) {
    query.each_entity_exclusive(world, |entity, pos| println!("{pos:?}"));
}

fn main() {
    let mut world = World::new();

    // Create entities, set position in random order
    let entity = world.entity().set(Position { x: 1.0, y: 0.0 }).id();
    world.entity().set(Position { x: 6.0, y: 0.0 });
    world.entity().set(Position { x: 2.0, y: 0.0 });
    world.entity().set(Position { x: 5.0, y: 0.0 });
    world.entity().set(Position { x: 4.0, y: 0.0 });

    // Create a sorted query
    let query = world
        .query::<&Position>()
        .order_by::<Position>(|_e1, p1: &Position, _e2, p2: &Position| -> i32 {
            (p1.x > p2.x) as i32 - (p1.x < p2.x) as i32
        })
        .build();

    println!();
    println!("--- First iteration ---");
    print_query(&query, &mut world);

    // Change the value of one entity, invalidating the order
    world.entity_from_id(entity).set(Position { x: 7.0, y: 0.0 });

    // Iterate query again, printed values are still ordered
    println!();
    println!("--- Second iteration ---");
    print_query(&query, &mut world);

    // Create new entity to show that data is also sorted for new entities
    world.entity().set(Position { x: 3.0, y: 0.0 });

    // Create a sorted system
    let sys = world
        .system::<&Position>()
        .order_by(compare_position)
        .each_entity(|entity, pos| {
            println!("{pos:?}");
        });

    // Run system, printed values are ordered
    println!();
    println!("--- System iteration ---");
    sys.run();

    // Output:
    //
    //  --- First iteration ---
    //  Position { x: 1.0, y: 0.0 }
    //  Position { x: 2.0, y: 0.0 }
    //  Position { x: 4.0, y: 0.0 }
    //  Position { x: 5.0, y: 0.0 }
    //  Position { x: 6.0, y: 0.0 }
    //
    //  --- Second iteration ---
    //  Position { x: 2.0, y: 0.0 }
    //  Position { x: 4.0, y: 0.0 }
    //  Position { x: 5.0, y: 0.0 }
    //  Position { x: 6.0, y: 0.0 }
    //  Position { x: 7.0, y: 0.0 }
    //
    //  --- System iteration ---
    //  Position { x: 2.0, y: 0.0 }
    //  Position { x: 3.0, y: 0.0 }
    //  Position { x: 4.0, y: 0.0 }
    //  Position { x: 5.0, y: 0.0 }
    //  Position { x: 6.0, y: 0.0 }
    //  Position { x: 7.0, y: 0.0 }
}

#[cfg(feature = "flecs_nightly_tests")]
#[test]
fn test() {
    let output_capture = OutputCapture::capture().unwrap();
    main();
    output_capture.test("query_sorting".to_string());
}
