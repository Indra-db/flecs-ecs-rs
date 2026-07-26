use crate::z_ignore_test_common::*;

use flecs_ecs::experimental::prelude::*;
use flecs_ecs::prelude::*;

// The DontFragment trait stores the component in a sparse storage that doesn't
// fragment tables (see the ComponentTraits manual).
#[derive(Debug, Component)]
#[flecs(traits(DontFragment))]
pub struct Position {
    pub x: f64,
    pub y: f64,
}

// The Sparse trait stores the component in a sparse storage. Unlike
// DontFragment, adding or removing a sparse component still fragments tables,
// but the component data stays at a stable address and is never moved between
// tables.
#[derive(Debug, Component)]
#[flecs(traits(Sparse))]
pub struct Velocity {
    pub x: f64,
    pub y: f64,
}

// The OnInstantiate policy configures what happens with the component when an
// entity is instantiated from a prefab. Mass is registered with
// (OnInstantiate, Inherit), which makes instances share the component with
// their prefab instead of copying it.
#[derive(Debug, Component, Clone)]
#[flecs(traits((OnInstantiate, Inherit)))]
pub struct Mass {
    pub value: f64,
}

fn main() {
    let mut world = World::new();

    // Traits are applied on component registration (implicit or explicit).
    let pos = world.component::<Position>();
    println!(
        "Position has DontFragment: {}",
        pos.has(flecs::DontFragment)
    );

    let vel = world.component::<Velocity>();
    println!("Velocity has Sparse: {}", vel.has(flecs::Sparse));

    // Because Mass is inheritable, instances share it with the prefab.
    let base = world.prefab_named("Spaceship").set(Mass { value: 100.0 });
    let inst = world.entity_named("MySpaceship").is_a(base);
    println!("MySpaceship owns Mass: {}", inst.owns(Mass::id()));
    let mass = inst.get_ref::<&Mass>().unwrap();
    println!("MySpaceship mass: {}", mass.value);
    drop(mass); // release the read guard so the sets below apply immediately

    inst.set(Position { x: 10.0, y: 20.0 });

    // Sparse components are used just like regular components. Because
    // Velocity is stored in a sparse set, the component data stays valid even
    // as the entity moves between tables.
    inst.set(Velocity { x: 1.0, y: 2.0 });
    let v = inst.get_ref::<&Velocity>().unwrap();
    println!("MySpaceship velocity: {{{}, {}}}", v.x, v.y);
    drop(v);

    // When all components in a query have the DontFragment trait, a sparse
    // query can be used, which iterates the sparse component storages directly
    // and is faster than a regular query. Sparse queries are validated at
    // compile time through the traits declared with #[flecs(traits(...))].
    let sparse = world.sparse_query::<&Position>();
    sparse.each_entity(|e, p| {
        println!("{}: {{{}, {}}}", e.name(), p.x, p.y);
    });

    // Regular queries work as well and use the same sparse storage. Iterating on
    // the exclusive register borrows &mut World and needs no lock traffic.
    let q = world.new_query::<&Position>();
    q.each_entity_exclusive(&mut world, |e, p| {
        println!("{}: {{{}, {}}}", e.name(), p.x, p.y);
    });

    // Output:
    //  Position has DontFragment: true
    //  Velocity has Sparse: true
    //  MySpaceship owns Mass: false
    //  MySpaceship mass: 100
    //  MySpaceship velocity: {1, 2}
    //  MySpaceship: {10, 20}
    //  MySpaceship: {10, 20}
}

#[cfg(feature = "flecs_nightly_tests")]
#[test]
fn test() {
    let output_capture = OutputCapture::capture().unwrap();
    main();
    output_capture.test("derive_traits".to_string());
}
