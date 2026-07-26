use crate::z_ignore_test_common::*;

use flecs_ecs::experimental::prelude::*;
use flecs_ecs::prelude::*;

// When an entity is instantiated from a prefab, components are by default
// copied from the prefab to the instance. This behavior can be customized with
// the OnInstantiate trait, which has three options:
//
// - Override (copy to instance)
// - Inherit (inherit from prefab)
// - DontInherit (don't copy or inherit)
//
// When a component is inheritable, it can be overridden manually by adding the
// component to the instance, which also copies the value from the prefab
// component. Additionally, when creating a prefab it is possible to flag a
// component as "auto override", which can change the behavior for a specific
// prefab from "inherit" to "override".
//
// This example shows how these different features can be used.

#[derive(Component, Clone)]
pub struct Attack {
    pub value: f32,
}

#[derive(Component, Debug, Clone)]
pub struct Defence {
    pub value: f32,
}

#[derive(Component, Clone)]
pub struct Damage {
    pub value: f32,
}

fn main() {
    let world = World::new();

    // Add the traits to mark the components to be inherited
    world
        .component::<Defence>()
        .add_trait::<(flecs::OnInstantiate, flecs::Inherit)>();
    world
        .component::<Attack>()
        .add_trait::<(flecs::OnInstantiate, flecs::Inherit)>();

    // Attack and Defense are properties that can be shared across many
    // spaceships. This saves memory, and speeds up prefab creation as we don't
    // have to copy the values of Attack and Defense to private components.
    let spaceship = world
        .prefab_named("SpaceShip")
        .set(Attack { value: 75.0 })
        .set(Defence { value: 100.0 })
        .set(Damage { value: 50.0 });

    // Create a prefab instance.
    let inst = world.entity_named("my_spaceship").is_a(spaceship);

    // The entity will now have a private copy of the Damage component, but not
    // of the Attack and Defense components. We can see this when we look at the
    // type of the instance:
    println!("{}", inst.archetype());

    // Even though Attack was not automatically overridden, we can always
    // override it manually afterwards by adding it:
    inst.override_type::<Attack>();

    // The Attack component now shows up in the entity type:
    println!("{}", inst.archetype());

    // We can get all components on the instance, regardless of whether they
    // are overridden or not. Note that the overridden components (Attack and
    // Damage) are initialized with the values from the prefab component:
    if let Ok((attack, defence, damage)) = inst.try_get::<(&Attack, &Defence, &Damage)>() {
        println!("attack: {}", attack.value);
        println!("defence: {}", defence.value);
        println!("damage: {}", damage.value);
    }

    // Output:
    //  Damage, (Identifier,Name), (IsA,SpaceShip)
    //  Attack, Damage, (Identifier,Name), (IsA,SpaceShip)
    //  attack: 75
    //  defence: 100
    //  damage: 50
}

#[cfg(feature = "flecs_nightly_tests")]
#[test]
fn test() {
    let output_capture = OutputCapture::capture().unwrap();
    main();
    output_capture.test("prefab_override".to_string());
}
