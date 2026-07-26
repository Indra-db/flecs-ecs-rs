use crate::z_ignore_test_common::*;

use flecs_ecs::experimental::prelude::*;
use flecs_ecs::prelude::*;

#[derive(Default, Component)]
#[flecs(meta)]
pub struct Point {
    pub x: f32,
    pub y: f32,
}

#[derive(Default, Component)]
#[flecs(meta)]
pub struct Line {
    pub start: Point,
    pub stop: Point,
}

fn main() {
    let world = World::new();

    // Create entity, set Line component
    let e = world.entity().set(Line {
        start: Point { x: 10.0, y: 20.0 },
        stop: Point { x: 30.0, y: 40.0 },
    });

    // Convert Line component to flecs expression string
    let line = e.get_ref::<&mut Line>().unwrap();
    // Convert component to string
    println!("{}", world.to_expr(&*line));

    // Output:
    //  {start: {x: 10, y: 20}, stop: {x: 30, y: 40}}
}

#[cfg(feature = "flecs_nightly_tests")]
#[test]
fn test() {
    let output_capture = OutputCapture::capture().unwrap();
    main();
    output_capture.test("reflection_nested_struct".to_string());
}
