use crate::z_ignore_test_common::*;

use flecs_ecs::prelude::*;
// Entity events are events that are emitted and observed for a specific entity.
// They are a thin wrapper around regular observers, which match against queries
// instead of single entities. While they work similarly under the hood, entity
// events provide a much simpler API.
//
// An entity event only needs two pieces of data:
// - The entity on which to emit the event
// - The event to emit

// An event without payload
#[derive(Component)]
struct Click;

// An event with payload
#[derive(Component)]
struct Resize {
    width: f32,
    height: f32,
}

#[derive(Debug, Copy, Clone)]
enum CloseReason {
    User,
    _System,
}

#[derive(Component)]
struct CloseRequested {
    reason: CloseReason,
}

fn main() {
    let world = World::new();

    // Create an observer for the CloseRequested event to listen to any entity.
    world
        .observer::<CloseRequested, &flecs::Any>()
        .each_iter(|it, _index, _| {
            let reason = it.param().reason;
            println!("Close request with reason: {:?}", reason);
        });

    let widget = world.entity_named("MyWidget");
    println!("widget: {:?}", widget);

    // Observe the Click event on the widget entity.
    widget.observe::<Click>(|| {
        println!("clicked!");
    });

    widget.observe_entity::<Click>(|entity| {
        println!("clicked on {:?}", entity.name());
    });

    // Observe the Resize event on the widget entity.
    widget.observe_payload(|payload: &Resize| {
        println!(
            "widget resized to {{ {}, {} }}!",
            payload.width, payload.height
        );
    });

    widget.observe_payload_entity(|entity, payload: &Resize| {
        println!(
            "{} resized to {{ {}, {} }}!",
            entity.name(),
            payload.width,
            payload.height
        );
    });

    widget.emit(&mut Click);

    widget.emit(&mut Resize {
        width: 100.0,
        height: 200.0,
    });

    widget.emit(&mut CloseRequested {
        reason: CloseReason::User,
    });

    // Output:
    //  widget: Entity name: MyWidget -- id: 506 -- archetype: (Identifier,Name)
    //  clicked on "MyWidget"
    //  clicked!
    //  MyWidget resized to { 100, 200 }!
    //  widget resized to { 100, 200 }!
    //  Close request with reason: User
}

#[cfg(feature = "flecs_nightly_tests")]
#[test]
fn test() {
    let output_capture = OutputCapture::capture().unwrap();
    main();
    output_capture.test("observer_entity_event".to_string());
}
