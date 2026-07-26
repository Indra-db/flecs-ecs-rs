//! Tests from entitiescomponents.md

#![allow(unused_imports, unused_variables, dead_code, non_snake_case, path_statements, unreachable_code, unused_mut,clippy::print_stdout)]
#![cfg_attr(rustfmt, rustfmt_skip)]

use crate::common_test::*;
use flecs_ecs::experimental::prelude::{EntityGuardExt, WorldSingletonExt};

#[test]
fn entities_and_components_entities_creation_01() {
    let world = World::new();
    let my_entity = world.entity();
}

#[test]
fn entities_and_components_entities_deletion_02() {
    let world = World::new();
    let my_entity = world.entity();
    my_entity.destruct();
}

#[test]
fn entities_and_components_entities_deletion_03() {
    let world = World::new();
    let e1 = world.entity(); // Returns 500v0
    e1.destruct(); // Recycles 500
    let e2 = world.entity(); // Returns 500v1
    /*
    // Fails, 500v0 is not alive
    e1.add(Npc::id());
    */
    // OK, 500v1 is alive
    e2.add(Npc::id());
}

#[test]
fn entities_and_components_entities_deletion_04() {
    let world = World::new();
    let e1 = world.entity();
    e1.destruct();
    e1.destruct(); // OK: post condition is satisfied
}

#[test]
fn entities_and_components_entities_clearing_05() {
    let world = World::new();
    let my_entity = world.entity();
    my_entity.clear();
}

#[test]
fn entities_and_components_entities_liveliness_checking_06() {
    let world = World::new();
    let e1 = world.entity();
    let e2 = world.entity();
    e1.destruct();
    e1.is_alive(); // False
    e2.is_alive(); // True
}

#[test]
fn entities_and_components_entities_liveliness_checking_07() {
    let world = World::new();
    let e1 = world.entity();
    let e2 = world.entity();
    e1.destruct();
    e1.is_valid(); // False
    world.entity_from_id(0).is_valid(); // False
}

#[test]
fn entities_and_components_entities_manual_ids_08() {
    let world = World::new();
    let e = world.make_alive(1000);
}

#[test]
fn entities_and_components_entities_manual_versioning_09() {
    let world = World::new();
    let versioned_id = 1000;
    world.set_version(versioned_id);
}

#[test]
fn entities_and_components_entities_ranges_10() {
    let world = World::new();
    let range = world.entity_range_new(5000, 0);
    world.entity_range_set(range);
}

#[test]
fn entities_and_components_entities_ranges_11() {
    let world = World::new();
    let range = world.entity_range_new(5000, 10000);
    world.entity_range_set(range);
}

#[test]
fn entities_and_components_entities_ranges_12() {
    let world = World::new();
    let range = world.entity_range_new(5000, 10000);
    world.entity_range_set(range);
}

#[test]
fn entities_and_components_entities_names_13() {
    let world = World::new();
    let e = world.entity_named("MyEntity");
    if e == world.lookup("MyEntity") {
        // true
    }
    println!("{}", e.name());
}

#[test]
fn entities_and_components_entities_names_14() {
    let world = World::new();
    let p = world.entity_named("Parent");
    let e = world.entity_named("Child").child_of(p);
    if e == world.lookup("Parent::Child") {
        // true
    }
}

#[test]
fn entities_and_components_entities_names_15() {
    let world = World::new();
    let p = world.entity_named("Parent");
    let e = world.entity_named("Child").child_of(p);
    if e == p.lookup("Child") {
        // true
    }
}

#[test]
fn entities_and_components_entities_names_16() {
    let world = World::new();
    let p = world.entity_named("Parent");
    let e = world.entity_named("Child").child_of(p);
    // Returns entity name, does not allocate
    println!("{}", e.name()); // Child
    // Returns entity path, does allocate
    println!("{}", e.path().unwrap()); // Parent.Child
}

#[test]
fn entities_and_components_entities_names_17() {
    let world = World::new();
    let e1 = world.entity_named("Parent::Child");
    let e2 = world.entity_named("Parent::Child");
    if e1 == e2 {
        // true
    }
}

#[test]
fn entities_and_components_entities_names_18() {
    let world = World::new();
    let e = world.entity_named("Foo");
    // Change name
    e.set_name("Bar");
}

#[test]
fn entities_and_components_entities_names_19() {
    let world = World::new();
    let ten = world.entity_named("10");
    let twenty = world.entity_named("20");
}

#[test]
fn entities_and_components_entities_disabling_20() {
    let world = World::new();
    let e = world.entity();
    // Enable entity
    e.enable_self();
    // Disable entity
    e.disable_self();
}

#[test]
fn entities_and_components_entities_disabling_21() {
    let world = World::new();
    // Three entities to disable
    let e1 = world.entity();
    let e2 = world.entity();
    let e3 = world.entity();

    // Create prefab that has the three entities
    let p = world.prefab();
    p.add(e1);
    p.add(e2);
    p.add(e3);

    // Disable entities
    p.disable_self();

    // Enable entities
    p.enable_self();
}

#[test]
fn entities_and_components_entities_disabling_22() {
    return; //TODO bug flecs upstream
    let world = World::new();
    // Three entities to disable
    let e1 = world.entity();
    let e2 = world.entity();
    let e3 = world.entity();

    // Create prefab hierarchy with the three entities
    let p1 = world.prefab().add(e1);
    let p2 = world.prefab().is_a(p1).add(e2).add(e3);

    // Disable e1, e2, e3
    p2.disable_self();

    // Enable e1
    p1.enable_self();
}

#[test]
fn entities_and_components_entities_disabling_23() {
    let world = World::new();
    let e = world.entity();
    e.add(flecs::Disabled);
}

#[test]
fn entities_and_components_components_hooks_24() {
    let world = World::new();
    world
        .component::<Position>()
        .on_set(|entity, pos| {
            println!("{:?}", pos);
        });
}

#[test]
fn entities_and_components_components_hooks_25() {
    let world = World::new();
    world
        .component::<Position>()
        .on_replace(|entity, prev, next| {
            println!("prev = {:?}", prev);
            println!("next = {:?}", next);
        });
}

#[test]
fn entities_and_components_components_components_have_entity_handles_26() {
    let world = World::new();
    // Get the entity for the Position component
    let pos = world.component::<Position>();
    // Component entities have the Component component
    {
        let comp_data = pos.get_ref::<&flecs::Component>().unwrap();
        println!(
            "size: {}, alignment: {}",
            comp_data.size, comp_data.alignment
        );
    };
}

#[test]
fn entities_and_components_components_components_have_entity_handles_27() {
    let world = World::new();
    // Register a sparse component
    world.component::<Position>().add_trait::<flecs::Sparse>();
}

#[test]
fn entities_and_components_components_registration_28() {
    let world = World::new();
    fn main() {
        let world = World::new();
        let e1 = world
            .entity()
            .set(Position { x: 10.0, y: 20.0 }) // Position registered here
            .set(Velocity { x: 1.0, y: 2.0 }); // Velocity registered here

        let e2 = world
            .entity()
            .set(Position { x: 10.0, y: 20.0 }) // Position already registered
            .set(Velocity { x: 1.0, y: 2.0 }); // Velocity already registered
    }
}

#[test]
fn entities_and_components_components_registration_29() {
    let world = World::new();
    world.component::<Position>();
}

#[test]
fn entities_and_components_components_registration_30() {
    let world = World::new();

    use flecs_ecs::prelude::*;

    #[derive(Component)]
    struct Movement;

    impl Module for Movement {
        fn module(world: &World) {
            world.module::<Movement>("Movement");
            // Define components, systems, triggers, ... as usual. They will be
            // automatically created inside the scope of the module.
        }
    }

    let world = World::new();
    world.import::<Movement>();
}

#[test]
fn entities_and_components_components_unregistration_31() {
    let world = World::new();
    let pos = world.component::<Position>();

    // Create entity with Position
    let e = world.entity().add(Position::id());

    // Unregister the component
    pos.destruct();

    // Position is removed from e
}

#[test]
fn entities_and_components_components_singletons_32() {
    let world = World::new();

    world.component::<TimeOfDay>().add_trait::<flecs::Singleton>();

    // Set singleton
    world.set(TimeOfDay(0.5));

    // Get singleton
    {
        let time = WorldSingletonExt::singleton::<TimeOfDay>(&world).unwrap();
        println!("{}", time.0);
    };
}

#[test]
fn entities_and_components_components_singletons_33() {
    let world = World::new();

    world.component::<TimeOfDay>().add_trait::<flecs::Singleton>();

    // Set singleton
    world.set(TimeOfDay(0.5));

    // Equivalent to:
    world.component::<TimeOfDay>().set(TimeOfDay(0.5));
}

#[test]
fn entities_and_components_components_disabling_34() {
    let world = World::new();
    // Register toggle-able component
    world
        .component::<Position>()
        .add_trait::<flecs::CanToggle>();

    let e = world.entity().set(Position { x: 10.0, y: 20.0 });

    // Disable component
    e.disable(Position::id());

    e.is_enabled(Position::id()); // False

    // Enable component
    e.enable(Position::id());

    e.is_enabled(Position::id()); // True
}