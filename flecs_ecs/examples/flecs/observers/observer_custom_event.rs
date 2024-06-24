use crate::z_ignore_test_common::*;

use flecs_ecs::prelude::*;

#[derive(Debug, Component)]
pub struct Position {
    pub x: f32,
    pub y: f32,
}

#[derive(Component)]
struct MyEvent;

fn main() {
    let world = World::new();

    // Create an observer for the custom event
    world
        .observer::<MyEvent, &Position>()
        .each_iter(|it, index, _pos| {
            println!(
                " - {}: {}: {}",
                it.event().name(),
                it.event_id().to_str(),
                it.entity(index)
            );
        });

    // The observer filter can be matched against the entity, so make sure it
    // has the Position component before emitting the event. This does not
    // trigger the observer yet.
    let entity = world.entity_named("e1").set(Position { x: 10.0, y: 20.0 });

    // Emit the custom event. This triggers the observer.
    world
        .event()
        .add::<Position>()
        .target(entity)
        .emit(&mut MyEvent);

    // Output:
    //  - MyEvent: Position: e1
}

#[cfg(feature = "flecs_nightly_tests")]
#[test]
fn test() {
    let output_capture = OutputCapture::capture().unwrap();
    main();
    output_capture.test("observer_custom_event".to_string());
}
