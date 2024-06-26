use crate::z_ignore_test_common::*;

use flecs_ecs::prelude::*;

#[derive(Debug, Component)]
pub struct Position {
    pub x: f32,
    pub y: f32,
}

fn main() {
    let world = World::new();

    // Create an observer for three events
    world
        .observer::<flecs::OnAdd, &Position>()
        .add_event::<flecs::OnRemove>() //or .add_event_id(OnRemove::ID)
        .add_event::<flecs::OnSet>()
        .each_iter(|it, index, pos| {
            if it.event() == flecs::OnAdd::ID {
                // No assumptions about the component value should be made here. If
                // a ctor for the component was registered it will be called before
                // the EcsOnAdd event, but a value assigned by set won't be visible.
                println!(" - OnAdd: {}: {}", it.event_id().to_str(), it.entity(index));
            } else {
                println!(
                    " - {}: {}: {}: with {:?}",
                    it.event().name(),
                    it.event_id().to_str(),
                    it.entity(index),
                    pos
                );
            }
        });

    // Create entity, set Position (emits EcsOnAdd and EcsOnSet)
    let entity = world.entity_named("e1").set(Position { x: 10.0, y: 20.0 });

    // Remove Position (emits EcsOnRemove)
    entity.remove::<Position>();

    // Remove Position again (no event emitted)
    entity.remove::<Position>();

    // Output:
    //  - OnAdd: Position: e1
    //  - OnSet: Position: e1: with Position { x: 10.0, y: 20.0 }
    //  - OnRemove: Position: e1: with Position { x: 10.0, y: 20.0 }
}

#[cfg(feature = "flecs_nightly_tests")]
#[test]
fn test() {
    let output_capture = OutputCapture::capture().unwrap();
    main();
    output_capture.test("observer_basics".to_string());
}
