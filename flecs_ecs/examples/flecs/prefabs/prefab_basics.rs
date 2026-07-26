use crate::z_ignore_test_common::*;

use flecs_ecs::experimental::prelude::*;
use flecs_ecs::prelude::*;
// Prefabs are entities that can be used as templates for other entities. They
// are created with a builtin Prefab tag, which by default excludes them from
// queries and systems.
//
// Prefab instances are entities that have an IsA relationship to the prefab.
// The IsA relationship causes instances to inherit the components from the
// prefab. By default all instances for a prefab share its components.
//
// Inherited components save memory as they only need to be stored once for all
// prefab instances. They also speed up the creation of prefabs, as inherited
// components don't need to be copied to the instances.
//
// To get a private copy of a component, an instance can add it which is called
// an override. Overrides can be manual (by using add) or automatic (see the
// auto_override example).
//
// If a prefab has children, adding the IsA relationship instantiates the prefab
// children for the instance (see hierarchy example).

#[derive(Component, Debug)]
pub struct Defence {
    pub value: f32,
}

fn main() {
    let mut world = World::new();

    // Add the traits to mark the component to be inherited
    world
        .component::<Defence>()
        .add_trait::<(flecs::OnInstantiate, flecs::Inherit)>();

    // Create a prefab with Position and Velocity components
    let spaceship = world.prefab_named("Prefab").set(Defence { value: 50.0 });

    // Create a prefab instance
    let inst = world.entity_named("my_spaceship").is_a(spaceship);

    // Drop down to plain entity ids so the &mut World reads below are free of any
    // live view borrow.
    let spaceship = spaceship.id();
    let inst = inst.id();

    // Because of the IsA relationship, the instance shares the Defence component
    // with the prefab, and can be read as a regular component. An EntityMut reads
    // straight from storage on the exclusive register, so the value is current.
    println!(
        "{:?}",
        world.entity_mut(inst).unwrap().get::<Defence>().unwrap()
    );

    // Because the component is shared, changing the value on the prefab also
    // changes it for the instance. On the exclusive register the write is
    // immediate, so the read-back below observes the new value with no deferral.
    world
        .entity_mut(spaceship)
        .unwrap()
        .set(Defence { value: 100.0 });

    println!(
        "after set: {:?}",
        world.entity_mut(inst).unwrap().get::<Defence>().unwrap()
    );

    // Prefab components can be iterated like regular components:
    let q = world.new_query::<&Defence>();
    q.each_entity_exclusive(&mut world, |entity, d| {
        println!("{}: defence: {}", entity.path().unwrap(), d.value);
    });

    // Output:
    //  Defence { value: 50.0 }
    //  after set: Defence { value: 100.0 }
    //  ::my_spaceship: 100
}

#[cfg(feature = "flecs_nightly_tests")]
#[test]
fn test() {
    let output_capture = OutputCapture::capture().unwrap();
    main();
    output_capture.test("prefab_basics".to_string());
}
