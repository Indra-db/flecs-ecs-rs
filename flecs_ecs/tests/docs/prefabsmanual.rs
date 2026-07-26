//! Tests from prefabsmanual.md

#![allow(unused_imports, unused_variables, dead_code, non_snake_case, path_statements, unreachable_code, unused_mut,clippy::print_stdout)]
#![cfg_attr(rustfmt, rustfmt_skip)]

use crate::common_test::*;
use flecs_ecs::experimental::prelude::EntityGuardExt;

#[test]
fn prefabs_introduction_01() {
    let world = World::new();

    #[derive(Component, Clone)]
    struct Defense {
    value: u32,
    }

    // Create a spaceship prefab with a Defense component.
    let spaceship = world.prefab_named("spaceship").set(Defense { value: 50 });

    // Create two prefab instances
    let inst_1 = world.entity().is_a(spaceship);
    let inst_2 = world.entity().is_a(spaceship);

    // Get instantiated component
    {
    let defense = inst_1.get::<&Defense>().unwrap();
    println!("Defense value: {}", defense.value);
    };
}

#[test]
fn prefabs_the_prefab_tag_02() {
    let world = World::new();
    let myprefab = world.entity().add(flecs::Prefab::id());

    // or the shortcut

    let myprefab = world.prefab();
}

#[test]
fn prefabs_the_prefab_tag_03() {
    let world = World::new();
    // Only match prefab entities
    world.query::<&Position>()
        .with(flecs::Prefab::id())
        .build();
}

#[test]
fn prefabs_the_prefab_tag_04() {
    let world = World::new();
    // Only match prefab entities
    world.query::<&Position>()
        .with(flecs::Prefab::id())
        .optional()
        .build();
}

#[test]
fn prefabs_the_prefab_tag_05() {
    let world = World::new();
    // Only match prefab entities
    world.query::<&Position>()
        .query_flags(QueryFlags::MatchPrefab)
        .build();
}

#[test]
fn prefabs_component_inheritance_06() {
    let world = World::new();
    // Make Defense component inheritable
    world
    .component::<Defense>()
    .add_trait::<(flecs::OnInstantiate, flecs::Inherit)>();

    // Create prefab
    let spaceship = world
    .prefab()
    .set(Health { value: 100 })
    .set(Defense { value: 50 });

    // Create prefab instance
    let inst = world.entity().is_a(spaceship);

    // Component is retrieved from instance
    {
    let health = inst.get::<&Health>().unwrap();
    println!("Health value: {}", health.value);
    };

    // Component is retrieved from prefab
    {
    let defense = inst.get::<&Defense>().unwrap();
    println!("Defense value: {}", defense.value);
    };
}

#[test]
fn prefabs_component_inheritance_07() {
    let world = World::new();
    let inst = world.entity();
    if inst.owns(Defense::id()) {
    // not inherited
    }
}

#[test]
fn prefabs_component_inheritance_08() {
    let world = World::new();
    let inst = world.entity();
    let inherited_from = inst.target(Defense::id(),0);
    if inherited_from.is_none() {
    // not inherited
    }
}

#[test]
fn prefabs_component_overriding_09() {
    let world = World::new();
    // Make Defense component inheritable
    world
    .component::<Defense>()
    .add_trait::<(flecs::OnInstantiate, flecs::Inherit)>();

    // Create prefab
    let spaceship = world.prefab().set(Defense { value: 50 });

    // Create prefab instance
    let inst_a = world.entity().is_a(spaceship);
    let inst_b = world.entity().is_a(spaceship);

    // Override Defense only for inst_a
    inst_a.set(Defense { value: 75 });
}

#[test]
fn prefabs_component_overriding_10() {
    let world = World::new();
    // Make Defense component inheritable
    world
    .component::<Defense>()
    .add_trait::<(flecs::OnInstantiate, flecs::Inherit)>();

    // Create prefab
    let spaceship = world.prefab().set(Defense { value: 50 });

    // Create prefab instance
    let inst_a = world.entity().is_a(spaceship);
    let inst_b = world.entity().is_a(spaceship);

    // Override Defense only for inst_a
    inst_a.add(Defense::id()); // Initialized with value 50
}

#[test]
fn prefabs_auto_overriding_11() {
    let world = World::new();
    // Make Defense component inheritable
    world
    .component::<Defense>()
    .add_trait::<(flecs::OnInstantiate, flecs::Inherit)>();

    // Create prefab
    let spaceship = world.prefab().set_auto_override(Defense { value: 50 }); // Set & auto override Defense

    // Create prefab instance
    let inst = world.entity().is_a(spaceship);
    inst.owns(Defense::id()); // true
}

#[test]
fn prefabs_prefab_variants_12() {
    let world = World::new();
    // Create prefab
    let spaceship = world
    .prefab_named("spaceship")
    .set(Defense { value: 50 })
    .set(Health { value: 100 });

    // Create prefab variant
    let freighter = world
    .prefab_named("Freighter")
    .is_a(spaceship)
    .set(Health { value: 150 }); // Override the Health component of the freighter

    // Create prefab instance
    let inst = world.entity().is_a(freighter);
    {
    let health = inst.get::<&Health>().unwrap();
    println!("Health value: {}", health.value); // 150
    };
    {
    let defense = inst.get::<&Defense>().unwrap();
    println!("Defense value: {}", defense.value); // 50
    };

}

#[test]
fn prefabs_prefab_hierarchies_13() {
    let world = World::new();
    let spaceship = world.prefab_named("spaceship");
    let cockpit = world.prefab_named("Cockpit").child_of(spaceship);

    // Instantiate the prefab hierarchy
    let inst = world.entity().is_a(spaceship);

    // Lookup instantiated child
    let inst_cockpit = inst.lookup("Cockpit");
}

#[test]
fn prefabs_prefab_slots_14() {
    let world = World::new();
    let spaceship = world.prefab_named("Spaceship");
    let cockpit = world.prefab_named("Cockpit").child_of(spaceship).slot(); // Defaults to (SlotOf, spaceship)

    // Instantiate the prefab hierarchy
    let inst = world.entity().is_a(spaceship);

    // Lookup instantiated child
    let inst_cockpit = inst.target(cockpit, 0);
}

#[test]
fn prefabs_prefab_types_c_c_15() {
    let world = World::new();
    #[derive(Component)]
    struct Spaceship;

    world.component_named::<Spaceship>("Spaceship");

    // Create prefab associated with the spaceship type
    world
    .prefab_type::<Spaceship>()
    .set(Defense { value: 50 })
    .set(Health { value: 100 });

    // Instantiate prefab with type
    let inst = world.entity().is_a(Spaceship::id());

    // Lookup prefab handle
    let prefab = world.lookup("Spaceship");
}