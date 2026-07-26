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

    // Create entity, set value of Line using reflection API
    let e = world.entity().add(Line::id());

    let mut line = e.get::<&mut Line>().unwrap();
    let mut cur = world.cursor(&mut *line);

    cur.push(); // {
    cur.member("start"); //   start:
    cur.push(); //   {
    cur.member("x"); //     x:
    cur.set_float(10.0); //     10
    cur.member("y"); //     y:
    cur.set_float(20.0); //     20
    cur.pop(); //   }
    cur.member("stop"); //   stop:
    cur.push(); //   {
    cur.member("x"); //     x:
    cur.set_float(30.0); //     30
    cur.member("y"); //     y:
    cur.set_float(40.0); //     40
    cur.pop(); //   }
    cur.pop(); // }

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
    output_capture.test("reflection_nested_set_member".to_string());
}
