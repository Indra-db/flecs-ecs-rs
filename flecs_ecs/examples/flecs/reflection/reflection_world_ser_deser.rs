use crate::z_ignore_test_common::*;

use flecs_ecs::prelude::*;
use json::WorldToJsonDesc;

#[derive(Debug, Component)]
#[flecs(meta)]
pub struct Position {
    pub x: f32,
    pub y: f32,
}

#[derive(Debug, Component)]
#[flecs(meta)]
pub struct Velocity {
    pub x: f32,
    pub y: f32,
}

#[derive(Component)]
pub struct Move;

// Register components and systems in a module. This excludes them by default
// from the serialized data, and makes it easier to import across worlds.
impl Module for Move {
    fn module(world: &World) {
        world
            .system_named::<(&mut Position, &Velocity)>("Move")
            .each_entity(|e, (pos, vel)| {
                pos.x += vel.x;
                pos.y += vel.y;

                println!(
                    "{} moved to {{x: {}, y: {}}}",
                    e.path().unwrap_or_default(),
                    pos.x,
                    pos.y
                );
            });
    }
}

/*

int main(int, char *[]) {
    flecs::world world_a;

    world_a.import<move>();

    world_a.entity("ent_1")
        .set<Position>({10, 20})
        .set<Velocity>({1, -1});

    world_a.entity("ent_2")
        .set<Position>({30, 40})
        .set<Velocity>({-1, 1});

    // Serialize world to JSON
    auto json = world_a.to_json();
    std::cout << json << std::endl << std::endl;

    // Output:
    // {
    //     "results": [{
    //         "ids": [
    //             ["my_module.Position"],
    //             ["my_module.Velocity"],
    //             ["flecs.core.Identifier", "flecs.core.Name"]
    //         ],
    //         "entities": ["ent_1", "ent_2"],
    //         "values": [
    //             [{
    //                 "x": 10,
    //                 "y": 20
    //             }, {
    //                 "x": 30,
    //                 "y": 40
    //             }],
    //             [{
    //                 "x": 1,
    //                 "y": -1
    //             }, {
    //                 "x": -1,
    //                 "y": 1
    //             }], 0
    //         ]
    //     }]
    // }

    // Create second world, import same module
    flecs::world world_b;
    world_b.import<move>();

    // Deserialize JSON into second world
    world_b.from_json(json);

    // Run system once for both worlds
    world_a.progress();
    std::cout << std::endl;
    world_b.progress();

    // Output
    //   ::ent_1 moved to {x: 11, y: 19}
    //   ::ent_2 moved to {x: 29, y: 41}
    //
    //   ::ent_1 moved to {x: 11, y: 19}
    //   ::ent_2 moved to {x: 29, y: 41}
}

*/

fn main() {
    let mut world_a = World::new();

    world_a.import::<Move>();

    world_a
        .entity_named("ent_1")
        .set(Position { x: 10.0, y: 20.0 })
        .set(Velocity { x: 1.0, y: -1.0 });

    world_a
        .entity_named("ent_2")
        .set(Position { x: 30.0, y: 40.0 })
        .set(Velocity { x: -1.0, y: 1.0 });

    // Serialize world to JSON
    let json = world_a.to_json_world(None);
    println!("{json}");

    // Output:
    // // component info is not included in the Output result, but it's there
    //  {
    //      "name": "ent_1",
    //      "id": 551,
    //      "components": {
    //        "Position": {
    //          "x": 10,
    //          "y": 20
    //        },
    //        "Velocity": {
    //          "x": 1,
    //          "y": -1
    //        },
    //        "(flecs.core.Identifier,flecs.core.Name)": null
    //      }
    //    },
    //    {
    //      "name": "ent_2",
    //      "id": 552,
    //      "components": {
    //        "Position": {
    //          "x": 30,
    //          "y": 40
    //        },
    //        "Velocity": {
    //          "x": -1,
    //          "y": 1
    //        },
    //        "(flecs.core.Identifier,flecs.core.Name)": null
    //      }
    //    }

    // Create second world, import same module
    let mut world_b = World::new();
    world_b.import::<Move>();

    // Deserialize JSON into second world
    world_b.from_json_world(json.as_str(), None);

    // Run system once for both worlds
    world_a.progress();
    println!();
    world_b.progress();

    // Output
    //   ::ent_1 moved to {x: 11, y: 19}
    //   ::ent_2 moved to {x: 29, y: 41}
    //
    //   ::ent_1 moved to {x: 11, y: 19}
    //   ::ent_2 moved to {x: 29, y: 41}
}

#[cfg(feature = "flecs_nightly_tests")]
#[test]
fn test() {
    let output_capture = OutputCapture::capture().unwrap();
    main();
    output_capture.test("reflection_world_ser_deser".to_string());
}
