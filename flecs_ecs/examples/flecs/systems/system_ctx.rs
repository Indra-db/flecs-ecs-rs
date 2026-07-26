use crate::z_ignore_test_common::*;

use flecs_ecs::prelude::*;
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Component)]
pub struct Position {
    pub x: f32,
    pub y: f32,
}

#[derive(Component)]
struct Radius {
    value: f32,
}

fn sqr(value: f32) -> f32 {
    value * value
}

fn distance_sqr(p1: &Position, p2: &Position) -> f32 {
    sqr(p2.x - p1.x) + sqr(p2.y - p1.y)
}

fn rand(max: u64) -> f32 {
    let start = SystemTime::now();

    //this rand method isn't great, but it's good enough for this example
    let since_the_epoch = start
        .duration_since(UNIX_EPOCH)
        .expect("Time went backwards");

    // Use some aspect of the current time to generate a number
    let random_number = since_the_epoch.as_secs() ^ since_the_epoch.subsec_nanos() as u64;

    (random_number % max) as f32
}

fn main() {
    let world = World::new();

    // Applications often need a system to reach a second query. A common use
    // case is a system that iterates more than one query, as in this simple
    // collision detection example. Instead of stashing the query behind a raw
    // context pointer, the closure simply captures it by value.

    let query_collide = world.new_query::<(&Position, &Radius)>();

    let sys = world
        .system::<(&Position, &Radius)>()
        .each_iter(move |it, index, (p1, r1)| {
            let e1 = it.entity(index);

            query_collide.each_entity(|e2, (p2, r2)| {
                if e1 == *e2 {
                    // don't collide with self
                    return;
                }

                if e1 > *e2 {
                    // Simple trick to prevent collisions from being detected
                    // twice with the entities reversed.
                    return;
                }

                // Check for collision
                let d_sqr = distance_sqr(p1, p2);
                let r_sqr = sqr(r1.value + r2.value);
                if r_sqr > d_sqr {
                    println!("{e1} and {e2} collided!");
                }
            });
        });

    // Create a few test entities
    for _ in 0..10 {
        world
            .entity()
            .set(Position {
                x: rand(50),
                y: rand(50),
            })
            .set(Radius {
                value: rand(10) + 1.0,
            });
    }

    // Run the system
    sys.run();

    // Output:
    //  532 and 539 collided!
    //  532 and 540 collided!
    //  534 and 538 collided!
    //  536 and 537 collided!
    //  536 and 540 collided!
    //  537 and 540 collided!
}

#[cfg(feature = "flecs_nightly_tests")]
#[test]
fn test() {
    let output_capture = OutputCapture::capture().unwrap();
    main();
    assert!(output_capture.output().lock().unwrap().len() > 0);
}
