//! Tests from relationships.md

#![allow(unused_imports, unused_variables, dead_code, non_snake_case, path_statements, unreachable_code, unused_mut,clippy::print_stdout)]
#![cfg_attr(rustfmt, rustfmt_skip)]

use crate::common_test::*;
use flecs_ecs::experimental::prelude::EntityGuardExt;

#[test]
fn relationships_introduction_01() {
    let world = World::new();
    let likes = world.entity();
    let bob = world.entity();
    let alice = world.entity();

    // bob likes alice
    bob.add((likes, alice));

    // bob likes alice no more
    bob.remove((likes, alice));
}

#[test]
fn relationships_introduction_02() {
    let world = World::new();

    let bob = world.entity();

    let eats = world.entity();
    let apples = world.entity();
    let pears = world.entity();

    bob.add((eats, apples));
    bob.add((eats, pears));

    bob.has((eats, apples)); // true
    bob.has((eats, pears)); // true
}

#[test]
fn relationships_introduction_03() {
    let world = World::new();
    let eats = world.entity();
    let apples = world.entity();
    world.component_named::<Eats>("Eats");
    world.component_named::<Apples>("Apples");
    // Find all entities that eat apples
    let q = world
        .query::<()>()
        .expr("(Eats, Apples)")
        .build()
        .expect("valid query expression");

    // Find all entities that eat anything
    let q = world
        .query::<()>()
        .expr("(Eats, *)")
        .build()
        .expect("valid query expression");

    // With the query builder API:
    let q = world.query::<()>().with((eats, apples)).build();

    // Or when using pair types, when both relationship & target are compile time types, they can be represented as a tuple:

    let q = world.query::<()>().with((Eats::id(), Apples::id())).build();
}

#[test]
fn relationships_relationship_queries_test_if_entity_has_a_relationship_pair_04() {
    let world = World::new();
    let bob = world.entity();
    let eats = world.entity();
    let apples = world.entity();
    bob.has((eats, apples));
}

#[test]
fn relationships_relationship_queries_test_if_entity_has_a_relationship_wildcard_05() {
    let world = World::new();
    let bob = world.entity();
    let eats = world.entity();
    bob.has((eats, flecs::Wildcard));
}

#[test]
fn relationships_relationship_queries_get_parent_for_entity_06() {
    let world = World::new();
    let bob = world.entity();
    let parent = bob.parent();
}

#[test]
fn relationships_relationship_queries_get_parent_for_entity_07() {
    let world = World::new();
    let bob = world.entity();
    let parent = bob.parent();
}

#[test]
fn relationships_relationship_queries_find_first_target_of_a_relationship_for_entity_08() {
    let world = World::new();
    let bob = world.entity();
    let eats = world.entity();
    let food = bob.target(eats, 0); // first target
}

#[test]
fn relationships_relationship_queries_find_all_targets_of_a_relationship_for_entity_09() {
    let world = World::new();
    let bob = world.entity();
    let eats = world.entity();
    let mut index = 0;
    while bob.target(eats, index).is_some() {
        index += 1;
    }
}

#[test]
fn relationships_relationship_queries_find_target_of_a_relationship_with_component_10() {
    let world = World::new();
    let bob = world.entity();
    let parent = bob.target_for(flecs::ChildOf, Position::id());
}

#[test]
fn relationships_relationship_queries_iterate_all_pairs_for_entity_11() {
    let world = World::new();
    let bob = world.entity();
    bob.each_component(|id| {
        if id.is_pair() {
            let first = id.first_id();
            let second = id.second_id();
        }
    });
}

#[test]
fn relationships_relationship_queries_find_all_entities_with_a_pair_12() {
    let world = World::new();
    let eats = world.entity();
    let apples = world.entity();
    world
        .query::<()>()
        .with((eats, apples))
        .build()
        .each_entity(|e, _| {
            // Iterate as usual
        });
}

#[test]
fn relationships_relationship_queries_find_all_entities_with_a_pair_wildcard_13() {
    let world = World::new();
    let eats = world.entity();
    world
        .query::<()>()
        .with((eats, flecs::Wildcard))
        .build()
        .each_iter(|it, i, _| {
            let food = it.pair(0).second_id(); // Apples, ...
            let e = it.entity(i);
            // Iterate as usual
        });
}

#[test]
fn relationships_relationship_queries_iterate_all_children_for_a_parent_14() {
    let world = World::new();
    let parent = world.entity();
    parent.each_child(|child| {
        // ...
    });
}

#[test]
fn relationships_relationship_components_15() {
    let world = World::new();
    // Empty types (types without members) are letmatically interpreted as tags

    #[derive(Component)]
    struct Begin;

    #[derive(Component)]
    struct End;

    #[derive(Component)]
    pub struct Eats {
        amount: u32,
    }

    // Tags
    let likes = world.entity();
    let apples = world.entity();

    let e = world.entity();

    // Both likes and Apples are tags, so (likes, Apples) is a tag
    e.add((likes, apples));

    // Eats is a type and Apples is a tag, so (Eats, Apples) has type Eats
    e.set_pair::<Eats, Apples>(Eats { amount: 1 });

    // Begin is a tags and Position is a type, so (Begin, Position) has type Position
    e.set_pair::<Begin, Position>(Position { x: 10.0, y: 20.0 });
    e.set_pair::<End, Position>(Position { x: 100.0, y: 20.0 }); // Same for End

    // ChildOf has the Tag property, so even though Position is a type, the pair
    // does not assume the Position type
    e.add((flecs::ChildOf, world.component_id::<Position>()));
    e.add((flecs::ChildOf, Position::id()));
}

#[test]
fn relationships_relationship_components_using_relationships_to_add_components_multiple_times_16() {
    let world = World::new();
    let e = world.entity();

    let first = world.entity();
    let second = world.entity();
    let third = world.entity();

    // Add component position 3 times, for 3 different objects
    e.set_first::<Position>(Position { x: 1.0, y: 2.0 }, first);
    e.set_first::<Position>(Position { x: 3.0, y: 4.0 }, second);
    e.set_first::<Position>(Position { x: 5.0, y: 6.0 }, third);
}

#[test]
fn relationships_relationship_wildcards_17() {
    let world = World::new();
    let likes = world.entity();
    let q = world
        .query::<()>()
        .with((likes, flecs::Wildcard))
        .build();

    q.each_iter(|it, i, _| {
        println!(
            "entity {} has relationship {} {}",
            it.entity(i),
            it.pair(0).first_id().name(),
            it.pair(0).second_id().name()
        );
    });
}

#[test]
fn relationships_relationship_wildcards_18() {
    let world = World::new();
    world.entity_named("likes");
    let q = world
        .query::<()>()
        .expr("(likes, *)")
        .build()
        .expect("valid query expression");
}

#[test]
fn relationships_inspecting_relationships_19() {
    let world = World::new();
    // bob eats apples and pears
    let bob = world.entity();

    let eats = world.entity();
    let apples = world.entity();
    let pears = world.entity();

    bob.add((eats, apples));
    bob.add((eats, pears));

    // Find all (Eats, *) relationships in bob's type
    bob.each_pair(eats, flecs::Wildcard, |id| {
        println!("bob eats {}", id.second_id().name());
    });

    // For target wildcard pairs, each_target_id() can be used:
    bob.each_target(eats, |entity| {
        println!("bob eats {}", entity.name());
    });
}

#[test]
fn relationships_builtin_relationships_the_isa_relationship_20() {
    let world = World::new();
    let apple = world.entity();
    let fruit = world.entity();

    apple.add((flecs::IsA::ID, fruit));
}

#[test]
fn relationships_builtin_relationships_the_isa_relationship_21() {
    let world = World::new();
    let apple = world.entity();
    let fruit = world.entity();
    apple.is_a(fruit);
}

#[test]
fn relationships_builtin_relationships_the_isa_relationship_22() {
    let world = World::new();
    let apple = world.entity();
    let granny_smith = world.entity();
    granny_smith.add((flecs::IsA::ID, apple));
}

#[test]
fn relationships_builtin_relationships_the_isa_relationship_component_sharing_23() {
    let world = World::new();

    world.component::<MaxSpeed>().add_trait::<(flecs::OnInstantiate, flecs::Inherit)>();

    let spaceship = world
        .entity()
        .set(MaxSpeed { value: 100 })
        .set(Defense { value: 50 });

    let frigate = world
        .entity()
        .is_a(spaceship) // shorthand for .add(flecs::IsA, Spaceship)
        .set(Defense { value: 75 });
}

#[test]
fn relationships_builtin_relationships_the_isa_relationship_component_sharing_24() {
    let world = World::new();
    world.component::<MaxSpeed>().add_trait::<(flecs::OnInstantiate, flecs::Inherit)>();
    world.component::<Defense>().add_trait::<(flecs::OnInstantiate, flecs::Inherit)>();
    let spaceship = world.entity().set(MaxSpeed { value: 100 }).set(Defense { value: 50 });
    let frigate = world.entity().is_a(spaceship).set(Defense { value: 75 });
    // Obtain the inherited component from Spaceship
    let is_100 = {
        let v = frigate.get::<&MaxSpeed>().unwrap();
        v.value == 100 // True
    };
}

#[test]
fn relationships_builtin_relationships_the_isa_relationship_component_sharing_25() {
    let world = World::new();
    world.component::<MaxSpeed>().add_trait::<(flecs::OnInstantiate, flecs::Inherit)>();
    world.component::<Defense>().add_trait::<(flecs::OnInstantiate, flecs::Inherit)>();
    let spaceship = world.entity().set(MaxSpeed { value: 100 }).set(Defense { value: 50 });
    let frigate = world.entity().is_a(spaceship).set(Defense { value: 75 });
    // Obtain the overridden component from Frigate
    let is_75 = {
        let v = frigate.get::<&mut Defense>().unwrap();
        v.value == 75 // True
    };
}

#[test]
fn relationships_builtin_relationships_the_isa_relationship_component_sharing_26() {
    let world = World::new();
    world.component::<MaxSpeed>().add_trait::<(flecs::OnInstantiate, flecs::Inherit)>();
    world.component::<Defense>().add_trait::<(flecs::OnInstantiate, flecs::Inherit)>();
    let spaceship = world.entity().set(MaxSpeed { value: 100 }).set(Defense { value: 50 });
    let frigate = world.entity().is_a(spaceship).set(Defense { value: 75 });
    let fast_frigate = world.entity().is_a(frigate).set(MaxSpeed { value: 200 });

    // Obtain the overridden component from FastFrigate
    let is_200 = {
        let v = fast_frigate.get::<&mut MaxSpeed>().unwrap();
        v.value == 200 // True
    };

    // Obtain the inherited component from Frigate
    let is_75 = {
        let v = fast_frigate.get::<&Defense>().unwrap();
        v.value == 75 // True
    };
}

#[test]
fn relationships_builtin_relationships_the_childof_relationship_27() {
    let world = World::new();
    let spaceship = world.entity();
    let cockpit = world.entity();
    cockpit.add((flecs::ChildOf, spaceship));
}

#[test]
fn relationships_builtin_relationships_the_childof_relationship_28() {
    let world = World::new();
    let cockpit = world.entity();
    let spaceship = world.entity();
    cockpit.child_of(spaceship);
}

#[test]
fn relationships_builtin_relationships_the_childof_relationship_namespacing_29() {
    let world = World::new();
    let parent = world.entity_named("Parent");
    let child = world.entity_named("Child").child_of(parent);

    assert!(child == world.lookup("Parent::Child")); // true
    assert!(child == parent.lookup("Child")); // true
}

#[test]
fn relationships_builtin_relationships_the_childof_relationship_scoping_30() {
    let world = World::new();
    let parent = world.entity();

    let prev = world.set_scope(parent);

    let child_a = world.entity();
    let child_b = world.entity();

    // Restore the previous scope
    world.set_scope(prev);

    child_a.has((flecs::ChildOf, parent)); // true
    child_b.has((flecs::ChildOf, parent)); // true
}

#[test]
fn relationships_builtin_relationships_the_childof_relationship_scoping_31() {
    let world = World::new();
    let parent = world.entity();
    parent.run_in_scope(|| {
        let child_a = world.entity();
        let child_b = world.entity();
        child_a.has((flecs::ChildOf, parent)); // true
        child_b.has((flecs::ChildOf, parent)); // true
    });
}