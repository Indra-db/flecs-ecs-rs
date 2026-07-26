//! Tests from componenttraits.md

#![allow(unused_imports, unused_variables, dead_code, non_snake_case, path_statements, unreachable_code, unused_mut,clippy::print_stdout,clippy::print_stdout)]
#![cfg_attr(rustfmt, rustfmt_skip)]

use crate::common_test::*;
use flecs_ecs::experimental::prelude::EntityGuardExt;

#[test]
fn component_traits_cantoggle_trait_01() {
    let world = World::new();
    world
    .component::<Position>()
    .add_trait::<flecs::CanToggle>();

    let e = world.entity().set(Position { x: 10.0, y: 20.0 });

    e.disable(Position::id()); // Disable component
    assert!(!e.is_enabled(Position::id()));

    e.enable(Position::id()); // Enable component
    assert!(e.is_enabled(Position::id()));
}

#[test]
fn component_traits_cleanup_traits_02() {
    let world = World::new();
    let parent = world.entity();
    let e = world.entity();
    #[derive(Component)]
    struct MyComponent {
        e: Entity, // Not covered by cleanup traits
    }

    e.child_of(parent); // Covered by cleanup traits
}

#[test]
fn component_traits_cleanup_traits_03() {
    let world = World::new();
    let archer = world.entity();
    world.remove_all(archer); //entity
    world.remove_all(Archer); //type

}

#[test]
fn component_traits_cleanup_traits_04() {
    let world = World::new();
    let archer = world.entity();
    world.remove_all(archer);
    world.remove_all((archer, flecs::Wildcard));
    world.remove_all((flecs::Wildcard, archer));
}

#[test]
fn component_traits_cleanup_traits_examples_ondelete_remove_05() {
    let world = World::new();
    // Remove Archer from entities when Archer is deleted
    world
        .component::<Archer>()
        .add_trait::<(flecs::OnDelete, flecs::Remove)>();

    let e = world.entity().add(Archer::id());
}

#[test]
fn component_traits_cleanup_traits_examples_ondelete_delete_06() {
    let world = World::new();
    // Remove Archer from entities when Archer is deleted
    world
        .component::<Archer>()
        .add_trait::<(flecs::OnDelete, flecs::Remove)>();

    let e = world.entity().add(Archer::id());

    // This will remove Archer from e
    world.component::<Archer>().destruct();
}

#[test]
fn component_traits_cleanup_traits_examples_ondeletetarget_delete_07() {
    let world = World::new();
    // Delete children when deleting parent
    world
        .component::<flecs::ChildOf>()
        .add_trait::<(flecs::OnDeleteTarget, flecs::Delete)>();

    let p = world.entity();
    let e = world.entity().add((flecs::ChildOf, p));

    // This will delete both p and e
    p.destruct();
}

#[test]
fn component_traits_cleanup_traits_cleanup_order_08() {
    let world = World::new();
    world
        .observer::<flecs::OnRemove, ()>()
        .with(Node)
        .each_entity(|e, _| {
    // This observer will be invoked when a Node is removed
    });

    let p = world.entity().add(Node::id());
    let c = world.entity().add(Node::id()).child_of(p);
}

#[test]
fn component_traits_dontfragment_trait_09() {
    let world = World::new();
    world.component::<Position>().add_trait::<flecs::DontFragment>();
}

#[test]
fn component_traits_exclusive_trait_10() {
    let world = World::new();
    let e = world.entity();
    let parent_a = world.entity();
    let parent_b = world.entity();
    e.child_of(parent_a);
    e.child_of(parent_b); // replaces (ChildOf, parent_a)
}

#[test]
fn component_traits_exclusive_trait_11() {
    let world = World::new();
    let married_to = world.entity().add_trait::<flecs::Exclusive>();
}

#[test]
fn component_traits_final_trait_12() {
    let world = World::new();
    let e = world.entity().add_trait::<flecs::Final>();

    /*
    let i = world.entity().is_a(e); // not allowed
    */
}

#[test]
fn component_traits_inheritable_trait_13() {
    let world = World::new();
    world.component::<Unit>().add_trait::<flecs::Inheritable>();

    let q = world.query::<()>()
      .with(Unit::id())
      .build();

    world.component::<Warrior>().is_a(Unit::id());

    q.each_entity(|e, _|  {
        // ...
    });
}

#[test]
fn component_traits_oneof_trait_14() {
    let world = World::new();
    // Enforce that target of relationship is child of Food
    let food = world.entity().add_trait::<flecs::OneOf>();
    let apples = world.entity().child_of(food);
    let fork = world.entity();

    // This is ok, Apples is a child of Food
    let a = world.entity().add((food, apples));

    /*
    // This is not ok, Fork is not a child of Food
    let b = world.entity().add((food, fork));
    */
}

#[test]
fn component_traits_oneof_trait_15() {
    let world = World::new();
    // Enforce that target of relationship is child of Food
    let food = world.entity();
    let eats = world.entity().add((flecs::OneOf::id(), food));
    let apples = world.entity().child_of(food);
    let fork = world.entity();

    // This is ok, Apples is a child of Food
    let a = world.entity().add((eats, apples));

    /*
    // This is not ok, Fork is not a child of Food
    let b = world.entity().add((eats, fork));
    */
}

#[test]
fn component_traits_oninstantiate_trait_override_16() {
    let world = World::new();
    // Register component with trait. Optional, since this is the default behavior.
    world
    .component::<Mass>()
    .add_trait::<(flecs::OnInstantiate, flecs::Override)>();

    let base = world.entity().set(Mass { value: 100.0 });
    let inst = world.entity().is_a(base); // Mass is copied to inst

    assert!(inst.owns(Mass::id()));
    assert!(base.cloned::<&Mass>().unwrap() == inst.cloned::<&Mass>().unwrap());
}

#[test]
fn component_traits_oninstantiate_trait_inherit_17() {
    let world = World::new();
    // Register component with trait
    world
    .component::<Mass>()
    .add_trait::<(flecs::OnInstantiate, flecs::Inherit)>();

    let base = world.entity().set(Mass { value: 100.0 });
    let inst = world.entity().is_a(base);

    assert!(inst.has(Mass::id()));
    assert!(!inst.owns(Mass::id()));
    assert!(base.cloned::<&Mass>().unwrap() == inst.cloned::<&Mass>().unwrap());
}

#[test]
fn component_traits_oninstantiate_trait_dontinherit_18() {
    let world = World::new();
    // Register component with trait
    world
    .component::<Mass>()
    .add_trait::<(flecs::OnInstantiate, flecs::DontInherit)>();

    let base = world.entity().set(Mass { value: 100.0 });
    let inst = world.entity().is_a(base);

    assert!(!inst.has(Mass::id()));
    assert!(!inst.owns(Mass::id()));
    assert!(inst.get::<&Mass>().is_none());
}

#[test]
fn component_traits_orderedchildren_trait_19() {
    let world = World::new();
    let parent = world.entity().add_trait::<flecs::OrderedChildren>();

    let child_1 = world.entity().child_of(parent);
    let child_2 = world.entity().child_of(parent);
    let child_3 = world.entity().child_of(parent);

    // Adding/removing components usually changes the order in which children are
    // iterated, but with the OrderedChildren trait order is preserved.
    child_2.set(Position { x: 10.0, y: 20.0 });

    parent.each_child(|child| {
        // 1st result: child_1
        // 2nd result: child_2
        // 3rd result: child_3
    });
}

#[test]
fn component_traits_pairistag_trait_20() {
    let world = World::new();
    #[derive(Component)]
    struct Serializable; // Tag, contains no data

    let e = world
        .entity()
        .set(Position { x: 10.0, y: 20.9 })
        .add((Serializable::id(), Position::id())); // Because Serializable is a tag, the pair
    // has a value of type Position

    // Gets value from Position component
    {
        let pos = e.get::<&Position>().unwrap();
        println!("Position: ({}, {})", pos.x, pos.y);
    };
    // Gets (unintended) value from (Serializable, Position) pair
    {
        let pos = e.get::<&(Serializable, Position)>().unwrap();
        println!("Serializable Position: ({}, {})", pos.x, pos.y);
    };
}

#[test]
fn component_traits_pairistag_trait_21() {
    let world = World::new();
    // This is currently not supported in Rust due to safety concerns.
}

#[test]
fn component_traits_relationship_trait_22() {
    let world = World::new();
    #[derive(Component)]
    struct Likes;

    #[derive(Component)]
    struct Apples;

    world
        .component::<Likes>()
        .add_trait::<flecs::Relationship>();

    let e = world
        .entity()
        /*
        .add(Likes::id()) // Panic, 'Likes' is not used as relationship
        .add((Apples::id(), Likes::id())) // Panic, 'Likes' is not used as relationship, but as target
        */
        .add((Likes::id(), Apples::id())); // OK
}

#[test]
fn component_traits_relationship_trait_23() {
    let world = World::new();
    #[derive(Component)]
    struct Likes;

    #[derive(Component)]
    struct Loves;

    world
        .component::<Likes>()
        .add_trait::<flecs::Relationship>();

    // Even though Likes is marked as relationship and used as target here, this
    // won't panic as With is marked as trait.
    world
        .component::<Loves>()
        .add_trait::<(flecs::With, Likes)>();
}

#[test]
fn component_traits_singleton_trait_24() {
    let world = World::new();
    world.component::<TimeOfDay>().add_trait::<flecs::Singleton>();

    world.set(TimeOfDay(0.0));
}

#[test]
fn component_traits_singleton_trait_25() {
    let world = World::new();
    // Automatically matches TimeOfDay as singleton
    let q = world.new_query::<(&Position, &Velocity, &TimeOfDay)>();

    // Is the same as
    let q = world.query::<(&Position, &Velocity, &TimeOfDay)>()
      .term_at(2).set_src(TimeOfDay::id())
      .build();
}

#[test]
fn component_traits_sparse_trait_26() {
    let world = World::new();
    world.component::<Position>().add_trait::<flecs::Sparse>();
}

#[test]
fn component_traits_symmetric_trait_27() {
    let world = World::new();
    let married_to = world.entity().add_trait::<flecs::Symmetric>();
    let bob = world.entity();
    let alice = world.entity();
    bob.add((married_to, alice)); // Also adds (MarriedTo, Bob) to Alice
}

#[test]
fn component_traits_target_trait_28() {
    let world = World::new();
    #[derive(Component)]
    struct Likes;

    #[derive(Component)]
    struct Apples;

    world.component::<Apples>().add_trait::<flecs::Target>();

    let e = world
        .entity()
        /*
        .add(Apples::id()) // Panic, 'Apples' is not used as target
        .add((Apples::id(), Likes::id())) // Panic, 'Apples' is not used as target, but as relationship
        */
        .add((Likes::id(), Apples::id())); // OK
}

#[test]
fn component_traits_trait_trait_29() {
    let world = World::new();
    #[derive(Component)]
    struct Serializable;

    world
        .component::<Serializable>()
        .add_trait::<flecs::Trait>();
}

#[test]
fn component_traits_transitive_trait_30() {
    let world = World::new();
    let locatedin = world.entity();
    let manhattan = world.entity();
    let newyork = world.entity();
    let usa = world.entity();

    manhattan.add((locatedin, newyork));
    newyork.add((locatedin, usa));
}

#[test]
fn component_traits_transitive_trait_31() {
    let world = World::new();
    let locatedin = world.entity();
    locatedin.add_trait::<flecs::Transitive>();
}

#[test]
fn component_traits_with_trait_32() {
    let world = World::new();
    let responsibility = world.entity();
    let power = world.entity().add((flecs::With::id(), responsibility));

    // Create new entity that has both Power and Responsibility
    let e = world.entity().add(power);
}

#[test]
fn component_traits_with_trait_33() {
    let world = World::new();
    let likes = world.entity();
    let loves = world.entity().add_trait::<(flecs::With, Likes)>();
    let pears = world.entity();

    // Create new entity with both (Loves, Pears) and (Likes, Pears)
    let e = world.entity().add((loves, pears));
}