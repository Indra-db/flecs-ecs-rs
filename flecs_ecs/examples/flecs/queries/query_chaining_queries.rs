use crate::z_ignore_test_common::*;

use flecs_ecs::experimental::prelude::QueryExclusiveExt;
use flecs_ecs::prelude::*;
// this example is to showcase how you can chain queries together where the second query
// uses the results of the first query to query the results

#[derive(Component)]
struct Enchanted;

#[derive(Component)]
struct Location {
    x: f32,
    y: f32,
}

#[derive(Component)]
struct Ability {
    power: f32,
}

#[derive(Component)]
struct ArtifactPower {
    _magic_level: f32,
}

fn main() {
    let mut forest = World::new();

    // Populate the forest with creatures. Some are enchanted.
    for i in 0..10 {
        let creature = forest
            .entity()
            .set(Location {
                x: i as f32,
                y: i as f32,
            })
            .set(Ability {
                power: i as f32 * 1.5,
            });

        if i % 2 == 0 {
            creature.add(Enchanted);
        }
    }

    // Introduce mystical artifacts into the forest, some of which are also enchanted
    for i in 0..10 {
        let artifact = forest
            .entity()
            .set(Location { x: -1.0, y: -1.0 }) //to showcase we don't match this
            .set(ArtifactPower {
                _magic_level: i as f32 * 2.5,
            });

        if i % 2 != 0 {
            // Differentiate enchantment condition to diversify
            artifact.add(Enchanted);
        }
    }

    // Query for creatures based on their Location and Ability
    let query_creatures = forest.query::<(&Location, &Ability)>().set_cached().build();

    // Filter specifically for enchanted things in the world
    let mut query_enchanted = forest.query::<()>().with(&Enchanted).build();

    // Iterate over creatures to find the enchanted ones
    query_creatures.run_exclusive(&mut forest, |mut iter| {

        while iter.next() {

            let loc = iter.field::<Location>(0);
            let ability = iter.field::<Ability>(1);
            // Filter for enchanted creatures within the current iteration
            query_enchanted
            .with_var_table(0, iter.range().unwrap())
            .each_iter( |it, index ,_| {
                let pos = &loc[index];
                let abil_power = ability[index].power;
                let entity = it.entity(index);
                println!(
                    "Creature id: {entity} at location {},{} is enchanted with mystical energy, ability power: {} "
                    , pos.x, pos.y, abil_power
                );
            });
        }
    });

    // Output:
    //  Creature id: 525 at location 0,0 is enchanted with mystical energy, ability power: 0
    //  Creature id: 527 at location 2,2 is enchanted with mystical energy, ability power: 3
    //  Creature id: 529 at location 4,4 is enchanted with mystical energy, ability power: 6
    //  Creature id: 531 at location 6,6 is enchanted with mystical energy, ability power: 9
    //  Creature id: 533 at location 8,8 is enchanted with mystical energy, ability power: 12
}

#[cfg(feature = "flecs_nightly_tests")]
#[test]
fn test() {
    let output_capture = OutputCapture::capture().unwrap();
    main();
    output_capture.test("query_chaining_queries".to_string());
}
