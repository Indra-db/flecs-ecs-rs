//! Tests from quickstart.md

#![allow(unused_imports, unused_variables, dead_code, non_snake_case, path_statements, unreachable_code, unused_mut,clippy::print_stdout)]
#![cfg_attr(rustfmt, rustfmt_skip)]

use crate::common_test::*;
use flecs_ecs::experimental::prelude::{EntityGuardExt, WorldSingletonExt};

#[test]
fn flecs_quickstart_concepts_world_01() {
    let world = World::new();

    // Do the ECS stuff
}

#[test]
fn flecs_quickstart_concepts_entity_02() {
    let world = World::new();
    let e = world.entity();
    e.is_alive(); // true!

    e.destruct();
    e.is_alive(); // false!
}

#[test]
fn flecs_quickstart_concepts_entity_03() {
    let world = World::new();
    let world = World::new();
    let e = world.entity_named("bob");

    println!("Entity name: {}", e.name());
}

#[test]
fn flecs_quickstart_concepts_entity_04() {
    let world = World::new();
    world.entity_named("bob");
    //if you are sure it exists
    let e = world.lookup("bob");
    //else use
    let e = world.try_lookup("bob");
}

#[test]
fn flecs_quickstart_concepts_component_05() {
    //notice the Default trait impl
    #[derive(Default, Component)]
    pub struct Velocity {
        pub x: f32,
        pub y: f32,
    }

    let world = World::new();
    let e = world.entity();

    // Add a component. This creates the component in the ECS storage, and defaults it. This requires the Default trait impl
    e.add(Velocity::id());

    // Set the value for the Position & Velocity components. A component will be
    // added if the entity doesn't have it yet.
    e.set(Position { x: 10.0, y: 20.0 })
     .set(Velocity { x: 1.0, y: 2.0 });

    // Get a component
    {
        let p = e.get_ref::<&Position>().unwrap();
        println!("Position: ({}, {})", p.x, p.y);
    };

    // Remove component
    e.remove(Position::id());
}

#[test]
fn flecs_quickstart_concepts_component_06() {
    let world = World::new();
    let pos_e = world.entity_from::<Position>();

    println!("Name: {}", pos_e.name()); // outputs 'Name: Position'

    // It's possible to add components like you would for any entity
    pos_e.add(Serializable::id());
}

#[test]
fn flecs_quickstart_concepts_component_07() {
    let world = World::new();
    let pos_e = world.entity_from::<Position>();

    {
        let c = pos_e.get_ref::<&flecs::Component>().unwrap();
        println!("Component size: {}", c.size);
    };
}

#[test]
fn flecs_quickstart_concepts_tag_08() {
    let world = World::new();
    // Option 1: create Tag as empty struct
    #[derive(Component)]
    struct Enemy;

    // Create entity, add Enemy tag
    let e = world.entity().add(Enemy);
    e.has(Enemy::id()); // true!

    e.remove(Enemy::id());
    e.has(Enemy::id()); // false!

    // Option 2: create Tag as entity
    let enemy = world.entity();

    // Create entity, add Enemy tag
    let e = world.entity().add(enemy);
    e.has(enemy); // true!

    e.remove(enemy);
    e.has(enemy); // false!
}

#[test]
fn flecs_quickstart_concepts_pair_09() {
    let world = World::new();
    // Create Likes relationship as empty type (tag)
    #[derive(Component)]
    struct Likes;

    // Create a small graph with two entities that like each other
    let bob = world.entity();
    let alice = world.entity();

    bob.add((Likes::id(), alice.id())); // bob likes alice
    alice.add((Likes::id(), bob.id())); // alice likes bob
    bob.has((Likes::id(), alice.id())); // true!

    bob.remove((Likes::id(), alice.id()));
    bob.has((Likes::id(), alice.id())); // false!
}

#[test]
fn flecs_quickstart_concepts_pair_10() {
    let world = World::new();
    let bob = world.entity();
    let id = world.id_from((Likes::id(), bob.id()));
}

#[test]
fn flecs_quickstart_concepts_pair_11() {
    let world = World::new();
    let id = world.id_view_from((Likes::id(), Apples::id()));
    if id.is_pair() {
        let relationship = id.first_id();
        let target = id.second_id();
    }
}

#[test]
fn flecs_quickstart_concepts_pair_12() {
    let world = World::new();
    let eats = world.entity();
    let apples = world.entity();
    let pears = world.entity();  
    let grows = world.entity();
    let bob = world.entity();
    bob.add((eats, apples));
    bob.add((eats, pears));
    bob.add((grows, pears));

    bob.has((eats, apples)); // true!
    bob.has((eats, pears)); // true!
    bob.has((grows, pears)); // true!
}

#[test]
fn flecs_quickstart_concepts_pair_13() {
    let world = World::new();
    let bob = world.entity();
    let alice = world.entity().add((Likes,bob));
    let o = alice.target(Likes,0); // Returns bob
}

#[test]
fn flecs_quickstart_concepts_hierarchies_14() {
    let world = World::new();
    let parent = world.entity();
    let child = world.entity().child_of(parent);

    // Deleting the parent also deletes its children
    parent.destruct();
}

#[test]
fn flecs_quickstart_concepts_hierarchies_15() {
    let world = World::new();
    let parent = world.entity_named("parent");
    let child = world.entity_named("child").child_of(parent);

    println!("Child path: {}", child.path().unwrap()); // output: 'parent::child'

    world.lookup("parent::child"); // returns child
    parent.lookup("child"); // returns child
}

#[test]
fn flecs_quickstart_concepts_hierarchies_16() {
    let world = World::new();
    let q = world
        .query::<(&Position, &mut Position)>()
        .term_at(1)
        .parent()
        .cascade()
        .build();

    q.each(|(p, p_parent)| {
        // Do the thing
    });
}

#[test]
fn flecs_quickstart_concepts_type_17() {
    let world = World::new();
    // types added via add or defaulted. If no default trait is implemented, use set instead
    let e = world.entity().add(Position::id()).add(Velocity::id());

    println!("Components: {}", e.archetype().to_string().unwrap()); // output: 'Position,Velocity'
}

#[test]
fn flecs_quickstart_concepts_type_18() {
    let world = World::new();
    let e = world.entity().add(Position::id()).add(Velocity::id());
    e.each_component(|id| {
        if id == world.component_id::<Position>() {
            // Found Position component!
        }
    });
}

#[test]
fn flecs_quickstart_concepts_singleton_19() {
    let world = World::new();
    // Set singleton component
    world.set(Gravity { value: 9.8 });

    // Get singleton component
    {
        let g = WorldSingletonExt::singleton::<Gravity>(&world).unwrap();
        println!("Gravity: {}", g.value);
    };
}

#[test]
fn flecs_quickstart_concepts_singleton_20() {
    let world = World::new();
    let grav_e = world.entity_from::<Gravity>();

    grav_e.set(Gravity { value: 9.8 });

    {
        let g = grav_e.get_ref::<&Gravity>().unwrap();
        println!("Gravity: {}", g.value);
    };
}

#[test]
fn flecs_quickstart_concepts_singleton_21() {
    let world = World::new();
    world
        .query::<(&Velocity, &Gravity)>()
        .build();
}

#[test]
fn flecs_quickstart_concepts_query_22() {
    let world = World::new();
    // For simple queries the world::each function can be used
    world.each::<(&mut Position, &Velocity)>(|(p, v)| {
        // EntityView argument is optional, use each_entity to get it
        p.x += v.x;
        p.y += v.y;
    });

    // More complex queries can first be created, then iterated  
    let parent = world.entity();
    let q = world
        .query::<&Position>()
        .with((flecs::ChildOf::ID, parent))
        .build();

    // Option 1: the each() callback iterates over each entity
    q.each_entity(|e, p| {
        println!("{}: ({}, {})", e.name(), p.x, p.y);
    }); 

    // Option 2: the run() callback offers more control over the iteration
    q.run(|mut it| {
        while it.next() {
            let p = it.field::<Position>(0);

            for i in it.iter() {
                println!("{}: ({}, {})", it.entity(i).name(), p[i].x, p[i].y);
            }
        }
    });
}

#[test]
fn flecs_quickstart_concepts_query_23() {
    let world = World::new();
    let q = world
        .query::<()>()
        .with((flecs::ChildOf, flecs::Wildcard))
        .with(Position::id())
        .set_oper(OperKind::Not)
        .build();

    // Iteration code is the same
}

#[test]
fn flecs_quickstart_concepts_system_24() {
    let world = World::new();
    // Use each_entity() function that iterates each individual entity
    let move_sys = world
        .system::<(&mut Position, &Velocity)>()
        .each_iter(|it, i, (p, v)| {
            p.x += v.x * it.delta_time();
            p.y += v.y * it.delta_time();
        });

    // Just like with queries, systems have both the run() and
    // each() methods to iterate entities.

    move_sys.run();
}

#[test]
fn flecs_quickstart_concepts_system_25() {
    let world = World::new();
    let move_sys = world.system::<(&mut Position, &Velocity)>().each(|(p, v)| {});
    println!("System: {}", move_sys.name());
    move_sys.add(flecs::pipeline::OnUpdate);
    move_sys.destruct();
}

#[test]
fn flecs_quickstart_concepts_pipeline_26() {
    flecs::pipeline::OnLoad;
    flecs::pipeline::PostLoad;
    flecs::pipeline::PreUpdate;
    flecs::pipeline::OnUpdate;
    flecs::pipeline::OnValidate;
    flecs::pipeline::PostUpdate;
    flecs::pipeline::PreStore;
    flecs::pipeline::OnStore;
}

#[test]
fn flecs_quickstart_concepts_pipeline_27() {
    let world = World::new();
    world
        .system_named::<(&mut Position, &Velocity)>("Move")
        .kind(flecs::pipeline::OnUpdate)
        .each(|(p, v)| {});

    world
        .system_named::<(&mut Position, &Transform)>("Transform")
        .kind(flecs::pipeline::PostUpdate)
        .each(|(p, t)| {});

    world
        .system_named::<(&Transform, &mut Mesh)>("Render")
        .kind(flecs::pipeline::OnStore)
        .each(|(t, m)| {});

    world.progress();
}

#[test]
fn flecs_quickstart_concepts_pipeline_28() {
    let world = World::new();
    let move_sys = world.system::<(&mut Position, &Velocity)>().each(|(p, v)| {});
    move_sys.add(flecs::pipeline::OnUpdate);
    move_sys.remove(flecs::pipeline::PostUpdate);
}

#[test]
fn flecs_quickstart_concepts_observer_29() {
    let world = World::new();
    world
        .observer_named::<flecs::OnSet, (&Position, &Velocity)>("OnSetPosition")
        .each(|(p, v)| {}); // Callback code is same as system

    let e = world.entity(); // Doesn't invoke the observer
    e.set(Position { x: 10.0, y: 20.0 }); // Doesn't invoke the observer
    e.set(Velocity { x: 1.0, y: 2.0 }); // Invokes the observer
    e.set(Position { x: 30.0, y: 40.0 }); // Invokes the observer
}

#[test]
fn flecs_quickstart_concepts_module_30() {
    let world = World::new();
    #[derive(Component)]
    struct MyModule;

    impl Module for MyModule {
        fn module(world: &World) {
            world.module::<MyModule>("MyModule");
            // Define components, systems, triggers, ... as usual. They will be
            // automatically created inside the scope of the module.
        }
    }

    // Import code
    world.import::<MyModule>();
}