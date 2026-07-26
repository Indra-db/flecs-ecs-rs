use crate::z_ignore_test_common::*;

use flecs_ecs::prelude::*;

#[derive(Component)]
struct Timeout {
    pub value: f32,
}

fn main() {
    let mut world = World::new();

    // System that deletes an entity after a timeout expires
    world
        .system::<&mut Timeout>()
        .each_iter(|it, index, timeout| {
            timeout.value -= it.delta_time();
            if timeout.value <= 0.0 {
                // Delete the entity

                // To make sure that the storage doesn't change while a system
                // is iterating entities, and multiple threads can safely access
                // the data, mutations (like a delete) are added to a command
                // queue and executed when it's safe to do so.

                // When the entity to be mutated is not the same as the entity
                // provided by the system, an additional mut() call is required.
                // See the mutate_entity_handle example.
                let e = it.entity(index);
                e.destruct();
                println!("Expire: {} deleted!", e.name());
            }
        });

    // System that prints remaining expiry time
    world.system::<&Timeout>().each_entity(|e, timeout| {
        println!(
            "PrintExpire: {} has {:.2} seconds left",
            e.name(),
            timeout.value
        );
    });

    // Observer that triggers when entity is actually deleted
    world
        .observer::<flecs::OnRemove, &Timeout>()
        .each_entity(|e, _timeout| {
            println!("Expired: {} actually deleted", e.name());
        });

    let e = world.entity_named("MyEntity").set(Timeout { value: 2.5 }).id();

    world.set_target_fps(1.0);

    while world.progress() {
        // If entity is no longer alive, exit
        if !world.entity_from_id(e).is_alive() {
            break;
        }

        println!("Tick...");
    }

    // Output:
    //  PrintExpire: MyEntity has 2.00 seconds left
    //  Tick...
    //  PrintExpire: MyEntity has 0.99 seconds left
    //  Tick...
    //  Expire: MyEntity deleted!
    //  PrintExpire: MyEntity has -0.03 seconds left
    //  Expired: MyEntity actually deleted
}

#[cfg(feature = "flecs_nightly_tests")]
#[test]
fn test() {
    let output_capture = OutputCapture::capture().unwrap();
    main();
    assert!(output_capture.output_string().contains("deleted"));
}
