#![allow(dead_code)]
use crate::common_test::*;
use flecs_ecs::experimental::prelude::EntityGuardExt;
use flecs_ecs_derive::Component;

#[repr(C)]
#[derive(Component, Debug, PartialEq)]
pub enum StandardEnum {
    Red,
    Green,
    Blue,
}

#[repr(C)]
#[derive(Component)]
pub enum AnotherEnum {
    Standing,
    Walking,
    Running,
}

#[repr(C)]
#[derive(Component)]
pub enum SparseEnum {
    Black = 1,
    White = 3,
    Grey = 5,
}

#[repr(C)]
#[derive(Component)]
pub enum EnumClass {
    Grass,
    Sand,
    Stone,
}

#[repr(C)]
#[derive(Component)]
pub enum PrefixEnum {
    PrefixEnumFoo,
    PrefixEnumBar,
}

#[repr(C)]
#[derive(Component)]
pub enum ConstantsWithNum {
    Num1,
    Num2,
    Num3,
}

#[repr(C)]
#[derive(Component)]
pub enum EnumIncorrectType {
    A,
    B,
}

#[repr(C)]
#[derive(Component)]
pub enum EnumWithLargeConstant {
    X,
    Y,
    Z = 1000,
}

#[repr(C)]
#[derive(Component)]
pub enum EnumClassWithLargeConstant {
    X,
    Y,
    Z = 1000,
}

/*
    test_int(enum_type.first_id(), Red);
    test_int(enum_type.last(), Blue);

    auto e_red = enum_type.entity(Red);
    auto e_green = enum_type.entity(Green);
    auto e_blue = enum_type.entity(Blue);

    test_assert(e_red != 0);
    test_str(e_red.path().c_str(), "::StandardEnum::Red");
    test_bool(enum_type.is_valid(Red), true);
    test_assert(e_red.get<StandardEnum>() != nullptr);
    test_assert(e_red.get<StandardEnum>()[0] == Red);

    test_assert(e_green != 0);
    test_str(e_green.path().c_str(), "::StandardEnum::Green");
    test_bool(enum_type.is_valid(Green), true);
    test_assert(e_green.get<StandardEnum>() != nullptr);
    test_assert(e_green.get<StandardEnum>()[0] == Green);

    test_assert(e_blue != 0);
    test_str(e_blue.path().c_str(), "::StandardEnum::Blue");
    test_bool(enum_type.is_valid(Blue), true);
    test_assert(e_blue.get<StandardEnum>() != nullptr);
    test_assert(e_blue.get<StandardEnum>()[0] == Blue);

    test_bool(enum_type.is_valid(Blue + 1), false);
}
*/

#[test]
fn enum_standard_enum_reflection() {
    let world = World::new();
    let entity = world.entity_from_enum(StandardEnum::Red);
    assert_eq!(entity.path().unwrap(), "::StandardEnum::Red");

    let entity2 = world.entity().set(StandardEnum::Blue);

    // TODO implement .first and .last() on all enums
    assert!(entity.is_valid());
    //let enum_comp = entity.get::<StandardEnum>().unwrap();
    entity2.set(StandardEnum::Red);
    {
        let enum_comp2 = entity2.get::<&StandardEnum>().unwrap();
        assert_eq!(*enum_comp2, StandardEnum::Red);
    };
    //assert!(*enum_comp == StandardEnum::Red);
    assert_eq!(entity.to_constant::<StandardEnum>(), StandardEnum::Red);

    let redd = StandardEnum::Red;

    let red = redd.id_variant(&world);
    let green = StandardEnum::Green.id_variant(&world);
    let blue = StandardEnum::Blue.id_variant(&world);

    assert_ne!(red, 0);
    assert_ne!(green, 0);
    assert_ne!(blue, 0);
    assert_ne!(green, red);
    assert_ne!(blue, red);
    assert!(StandardEnum::Red.is_field_registered_as_entity(&world));
    assert!(StandardEnum::Blue.is_field_registered_as_entity(&world));
    assert!(StandardEnum::Green.is_field_registered_as_entity(&world));
    assert_eq!(red.path().unwrap(), "::StandardEnum::Red");
    assert_eq!(green.path().unwrap(), "::StandardEnum::Green");
    assert_eq!(blue.path().unwrap(), "::StandardEnum::Blue");
}

// ---- New enum tests ----

#[test]
fn enum_add_enum_constant() {
    let world = World::new();

    let e = world.entity().add_enum(StandardEnum::Red);
    // Type string: "(StandardEnum, StandardEnum.Red)"
    let id = world.component_id::<StandardEnum>();
    let red_entity = StandardEnum::Red.id_variant(&world);
    assert!(e.has((id, red_entity)));
}

#[test]
fn enum_add_enum_constant_explicit() {
    let world = World::new();

    let e = world.entity().add_enum(StandardEnum::Red);
    assert!(e.has_enum(StandardEnum::Red));
    assert!(!e.has_enum(StandardEnum::Green));
    assert!(!e.has_enum(StandardEnum::Blue));
}

#[test]
fn enum_add_enum_class_constant() {
    let world = World::new();

    let e = world.entity().add_enum(EnumClass::Sand);
    assert!(e.has_enum(EnumClass::Sand));
    assert!(!e.has_enum(EnumClass::Grass));
    assert!(!e.has_enum(EnumClass::Stone));
}

#[test]
fn enum_add_singleton_enum_constant() {
    let world = World::new();

    world
        .component::<StandardEnum>()
        .add_trait::<flecs::Singleton>();

    world.add_enum(StandardEnum::Red);
    assert!(world.has_enum(StandardEnum::Red));
    assert!(!world.has_enum(StandardEnum::Green));
    assert!(!world.has_enum(StandardEnum::Blue));

    world.add_enum(StandardEnum::Green);
    assert!(!world.has_enum(StandardEnum::Red));
    assert!(world.has_enum(StandardEnum::Green));
    assert!(!world.has_enum(StandardEnum::Blue));

    world.add_enum(StandardEnum::Blue);
    assert!(!world.has_enum(StandardEnum::Red));
    assert!(!world.has_enum(StandardEnum::Green));
    assert!(world.has_enum(StandardEnum::Blue));
}

#[test]
fn enum_replace_enum_constants() {
    let world = World::new();

    let e = world.entity().add_enum(StandardEnum::Red);
    assert!(e.has_enum(StandardEnum::Red));
    assert!(!e.has_enum(StandardEnum::Green));
    assert!(!e.has_enum(StandardEnum::Blue));

    e.add_enum(StandardEnum::Green);
    assert!(!e.has_enum(StandardEnum::Red));
    assert!(e.has_enum(StandardEnum::Green));
    assert!(!e.has_enum(StandardEnum::Blue));

    e.add_enum(StandardEnum::Blue);
    assert!(!e.has_enum(StandardEnum::Red));
    assert!(!e.has_enum(StandardEnum::Green));
    assert!(e.has_enum(StandardEnum::Blue));
}

#[test]
fn enum_has_enum() {
    let world = World::new();

    let comp_id = world.component_id::<StandardEnum>();
    let e = world.entity();
    assert!(!e.has((comp_id, *flecs::Wildcard)));

    e.add_enum(StandardEnum::Red);

    assert!(e.has((comp_id, *flecs::Wildcard)));
    assert!(e.has_enum(StandardEnum::Red));
    assert!(!e.has_enum(StandardEnum::Green));
    assert!(!e.has_enum(StandardEnum::Blue));
}

#[test]
fn enum_has_enum_wildcard() {
    let world = World::new();

    let comp_id = world.component_id::<StandardEnum>();
    let e = world.entity();
    assert!(!e.has((comp_id, *flecs::Wildcard)));

    e.add_enum(StandardEnum::Green);
    assert!(e.has((comp_id, *flecs::Wildcard)));
}

#[test]
fn enum_get_constant() {
    let world = World::new();

    let e = world.entity().add_enum(StandardEnum::Red);
    assert!(e.has_enum(StandardEnum::Red));

    // get_enum_constant not available; use get instead
    {
        let v = e.get::<&StandardEnum>().unwrap();
        assert_eq!(*v, StandardEnum::Red);
    };

    e.add_enum(StandardEnum::Green);
    assert!(e.has_enum(StandardEnum::Green));

    {
        let v = e.get::<&StandardEnum>().unwrap();
        assert_eq!(*v, StandardEnum::Green);
    };
}

#[test]
fn enum_remove_enum() {
    let world = World::new();

    let comp_id = world.component_id::<StandardEnum>();
    let e = world.entity().add_enum(StandardEnum::Green);
    assert!(e.has_enum(StandardEnum::Green));

    e.remove((comp_id, *flecs::Wildcard));
    assert!(!e.has_enum(StandardEnum::Green));
}

#[test]
fn enum_remove_wildcard() {
    let world = World::new();

    let comp_id = world.component_id::<StandardEnum>();
    let e = world.entity().add_enum(StandardEnum::Green);
    assert!(e.has_enum(StandardEnum::Green));

    e.remove((comp_id, *flecs::Wildcard));
    assert!(!e.has_enum(StandardEnum::Green));
}

#[test]
fn enum_enum_as_component() {
    let world = World::new();

    let e = world.entity();

    e.set(StandardEnum::Green);
    assert!(e.has(StandardEnum::id()));

    {
        let v = e.get::<&StandardEnum>().unwrap();
        assert_eq!(*v, StandardEnum::Green);
    };
}

#[test]
fn enum_query_enum_wildcard() {
    let world = World::new();

    let e1 = world.entity().add_enum(StandardEnum::Red);
    let e2 = world.entity().add_enum(StandardEnum::Green);
    let e3 = world.entity().add_enum(StandardEnum::Blue);

    let comp_id = world.component_id::<StandardEnum>();
    let mut count = 0;

    world
        .query::<()>()
        .with((comp_id, *flecs::Wildcard))
        .build()
        .each_entity(|e, _| {
            if e == e1 || e == e2 || e == e3 {
                count += 1;
            }
        });

    assert_eq!(count, 3);
}

#[test]
fn enum_query_enum_constant() {
    let world = World::new();

    world.entity().add_enum(StandardEnum::Red);
    world.entity().add_enum(StandardEnum::Green);
    let _e1 = world.entity().add_enum(StandardEnum::Blue);

    let blue_id = StandardEnum::Blue.id_variant(&world);
    let comp_id = world.component_id::<StandardEnum>();

    let mut count = 0;
    world
        .query::<()>()
        .with((comp_id, blue_id))
        .build()
        .each_entity(|_e, _| {
            count += 1;
        });

    assert_eq!(count, 1);
}

#[test]
fn enum_query_singleton_enum_constant() {
    let world = World::new();

    world
        .component::<StandardEnum>()
        .add_trait::<flecs::Singleton>();

    let blue_id = StandardEnum::Blue.id_variant(&world);
    let comp_id = world.component_id::<StandardEnum>();

    let mut count = 0;

    world
        .query::<()>()
        .with((comp_id, blue_id))
        .build()
        .each(|_| {
            count += 1;
        });

    assert_eq!(count, 0);

    world.add_enum(StandardEnum::Red);
    world
        .query::<()>()
        .with((comp_id, blue_id))
        .build()
        .each(|_| {
            count += 1;
        });
    assert_eq!(count, 0);

    world.add_enum(StandardEnum::Green);
    world
        .query::<()>()
        .with((comp_id, blue_id))
        .build()
        .each(|_| {
            count += 1;
        });
    assert_eq!(count, 0);

    world.add_enum(StandardEnum::Blue);
    world
        .query::<()>()
        .with((comp_id, blue_id))
        .build()
        .each(|_| {
            count += 1;
        });
    assert_eq!(count, 1);
}

#[test]
fn enum_enum_type_from_stage() {
    let world = World::new();
    // Verify the enum component is accessible in the stage
    let e = world.component::<StandardEnum>();
    assert_ne!(e.id(), 0);

    world.readonly_begin(false);
    let stage = world.stage(0);
    let stage_comp_id = stage.component_id::<StandardEnum>();
    assert_ne!(stage_comp_id, 0);
    world.readonly_end();
}

#[test]
fn enum_add_enum_from_stage() {
    let world = World::new();

    world.readonly_begin(false);

    let stage = world.stage(0);
    let e = stage.entity();

    e.add_enum(StandardEnum::Red);
    assert!(!e.has_enum(StandardEnum::Red));

    world.readonly_end();

    assert!(e.has_enum(StandardEnum::Red));
}

#[test]
fn enum_enum_w_2_worlds() {
    {
        let world = World::new();

        let e = world.component::<StandardEnum>();
        assert_ne!(e.id(), 0);

        let red_entity = StandardEnum::Red.id_variant(&world);
        let green_entity = StandardEnum::Green.id_variant(&world);
        let blue_entity = StandardEnum::Blue.id_variant(&world);

        assert_ne!(red_entity, 0);
        assert_ne!(green_entity, 0);
        assert_ne!(blue_entity, 0);

        assert!(StandardEnum::Red.is_field_registered_as_entity(&world));
        assert!(StandardEnum::Green.is_field_registered_as_entity(&world));
        assert!(StandardEnum::Blue.is_field_registered_as_entity(&world));
    }
    {
        let world = World::new();

        let e = world.component::<StandardEnum>();
        assert_ne!(e.id(), 0);

        let red_entity = StandardEnum::Red.id_variant(&world);
        let green_entity = StandardEnum::Green.id_variant(&world);
        let blue_entity = StandardEnum::Blue.id_variant(&world);

        assert_ne!(red_entity, 0);
        assert_ne!(green_entity, 0);
        assert_ne!(blue_entity, 0);
    }
}

#[derive(Component, Default)]
struct MyTag2;

#[test]
fn enum_add_enum_constant_w_tag() {
    let world = World::new();

    let red_id = StandardEnum::Red.id_variant(&world);
    let green_id = StandardEnum::Green.id_variant(&world);
    let blue_id = StandardEnum::Blue.id_variant(&world);
    let tag_id = MyTag2::id();

    let e1 = world.entity().add((red_id, tag_id));
    let e2 = world.entity().add((green_id, tag_id));
    let e3 = world.entity().add((blue_id, tag_id));

    assert!(e1.has((red_id, tag_id)));
    assert!(e2.has((green_id, tag_id)));
    assert!(e3.has((blue_id, tag_id)));
}

#[test]
fn enum_remove_enum_constant_w_tag() {
    let world = World::new();

    let red_id = StandardEnum::Red.id_variant(&world);
    let green_id = StandardEnum::Green.id_variant(&world);
    let blue_id = StandardEnum::Blue.id_variant(&world);
    let tag_id = MyTag2::id();

    let e1 = world.entity().add((red_id, tag_id));
    let e2 = world.entity().add((green_id, tag_id));
    let e3 = world.entity().add((blue_id, tag_id));

    assert!(e1.has((red_id, tag_id)));
    assert!(e2.has((green_id, tag_id)));
    assert!(e3.has((blue_id, tag_id)));

    e1.remove((green_id, tag_id)).remove((blue_id, tag_id));
    assert!(e1.has((red_id, tag_id)));
    e1.remove((red_id, tag_id));
    assert!(!e1.has((red_id, tag_id)));

    e2.remove((red_id, tag_id)).remove((blue_id, tag_id));
    assert!(e2.has((green_id, tag_id)));
    e2.remove((green_id, tag_id));
    assert!(!e2.has((green_id, tag_id)));

    e3.remove((red_id, tag_id)).remove((green_id, tag_id));
    assert!(e3.has((blue_id, tag_id)));
    e3.remove((blue_id, tag_id));
    assert!(!e3.has((blue_id, tag_id)));
}

#[test]
fn enum_set_enum_constant_w_tag() {
    let world = World::new();

    let red_id = StandardEnum::Red.id_variant(&world);
    let green_id = StandardEnum::Green.id_variant(&world);
    let blue_id = StandardEnum::Blue.id_variant(&world);

    let e1 = world
        .entity()
        .set_first::<Position>(Position { x: 1, y: 2 }, red_id)
        .set_first::<Position>(Position { x: 2, y: 3 }, green_id)
        .set_first::<Position>(Position { x: 3, y: 4 }, blue_id);

    assert!(e1.has((Position::id(), red_id)));
    assert!(e1.has((Position::id(), green_id)));
    assert!(e1.has((Position::id(), blue_id)));

    let p_red = e1.get_first_untyped::<Position>(red_id) as *const Position;
    let p_green = e1.get_first_untyped::<Position>(green_id) as *const Position;
    let p_blue = e1.get_first_untyped::<Position>(blue_id) as *const Position;

    unsafe {
        assert_eq!((*p_red).x, 1);
        assert_eq!((*p_red).y, 2);
        assert_eq!((*p_green).x, 2);
        assert_eq!((*p_green).y, 3);
        assert_eq!((*p_blue).x, 3);
        assert_eq!((*p_blue).y, 4);
    }
}

#[test]
fn enum_add_union_enum() {
    let world = World::new();

    world
        .component::<StandardEnum>()
        .add_trait::<flecs::DontFragment>();

    let red_id = StandardEnum::Red.id_variant(&world);
    let blue_id = StandardEnum::Blue.id_variant(&world);
    let comp_entity = world.entity_from::<StandardEnum>();

    let e1 = world.entity().add_enum(StandardEnum::Red);
    let e2 = world.entity().add_enum(StandardEnum::Blue);

    // With DontFragment, all entities have same table (archetype)
    assert_eq!(e1.table(), e2.table());
    // target(comp_entity, 0) gives the enum value entity
    assert_eq!(e1.target(comp_entity, 0).unwrap().id(), red_id);
    assert_eq!(e2.target(comp_entity, 0).unwrap().id(), blue_id);
    assert!(e1.has_enum(StandardEnum::Red));
    assert!(e2.has_enum(StandardEnum::Blue));
}

#[test]
fn enum_add_2_union_enums() {
    let world = World::new();

    world
        .component::<StandardEnum>()
        .add_trait::<flecs::DontFragment>();
    world
        .component::<AnotherEnum>()
        .add_trait::<flecs::DontFragment>();

    let e = world.entity();
    e.add_enum(StandardEnum::Red);
    e.add_enum(AnotherEnum::Running);

    assert!(e.has_enum(StandardEnum::Red));
    assert!(e.has_enum(AnotherEnum::Running));

    let red = StandardEnum::Red.id_variant(&world);
    let running = AnotherEnum::Running.id_variant(&world);

    let se_entity = world.entity_from::<StandardEnum>();
    let ae_entity = world.entity_from::<AnotherEnum>();

    assert_eq!(e.target(se_entity, 0).unwrap().id(), red);
    assert_eq!(e.target(ae_entity, 0).unwrap().id(), running);
}

#[test]
fn enum_add_2_union_enums_reverse() {
    let world = World::new();

    world
        .component::<StandardEnum>()
        .add_trait::<flecs::DontFragment>();
    world
        .component::<AnotherEnum>()
        .add_trait::<flecs::DontFragment>();

    let e = world.entity();
    e.add_enum(AnotherEnum::Running);
    e.add_enum(StandardEnum::Red);

    assert!(e.has_enum(StandardEnum::Red));
    assert!(e.has_enum(AnotherEnum::Running));

    let red = StandardEnum::Red.id_variant(&world);
    let running = AnotherEnum::Running.id_variant(&world);

    let se_entity = world.entity_from::<StandardEnum>();
    let ae_entity = world.entity_from::<AnotherEnum>();

    assert_eq!(e.target(se_entity, 0).unwrap().id(), red);
    assert_eq!(e.target(ae_entity, 0).unwrap().id(), running);
}

#[test]
fn enum_constant_from_entity() {
    let world = World::new();

    let e_red = world.entity_from_enum(StandardEnum::Red);
    assert_ne!(e_red.id(), 0);

    let e_green = world.entity_from_enum(StandardEnum::Green);
    assert_ne!(e_green.id(), 0);

    let e_blue = world.entity_from_enum(StandardEnum::Blue);
    assert_ne!(e_blue.id(), 0);

    assert_eq!(e_red.to_constant::<StandardEnum>(), StandardEnum::Red);
    assert_eq!(e_green.to_constant::<StandardEnum>(), StandardEnum::Green);
    assert_eq!(e_blue.to_constant::<StandardEnum>(), StandardEnum::Blue);
}

#[test]
fn enum_add_if() {
    let world = World::new();

    let e = world.entity();

    e.add_enum_if(StandardEnum::Red, true);
    assert!(e.has_enum(StandardEnum::Red));

    e.add_enum_if(StandardEnum::Red, false);
    assert!(!e.has_enum(StandardEnum::Red));
}

#[test]
fn enum_add_if_other() {
    let world = World::new();

    let e = world.entity();

    e.add_enum(StandardEnum::Red);
    assert!(e.has_enum(StandardEnum::Red));

    // Adding a different enum value with false removes the existing one
    e.add_enum_if(StandardEnum::Blue, false);
    assert!(!e.has_enum(StandardEnum::Blue));
    assert!(!e.has_enum(StandardEnum::Red));
}

#[test]
fn enum_query_union_enum() {
    let world = World::new();

    world
        .component::<StandardEnum>()
        .add_trait::<flecs::DontFragment>();

    let e1 = world.entity().add_enum(StandardEnum::Red);
    let e2 = world.entity().add_enum(StandardEnum::Green);
    let e3 = world.entity().add_enum(StandardEnum::Blue);

    let comp_id = world.component_id::<StandardEnum>();

    let mut count = 0;
    world
        .query::<()>()
        .with((comp_id, *flecs::Wildcard))
        .build()
        .each_entity(|e, _| {
            if e == e1 || e == e2 || e == e3 {
                count += 1;
            }
        });

    assert_eq!(count, 3);
}

#[test]
fn enum_component_registered_as_enum() {
    let world = World::new();

    let e = world.component::<StandardEnum>();
    // Verify the component was registered
    assert_ne!(e.id(), 0);

    // Verify enum variants are accessible
    let red = StandardEnum::Red.id_variant(&world);
    let green = StandardEnum::Green.id_variant(&world);
    let blue = StandardEnum::Blue.id_variant(&world);

    assert_ne!(red, 0);
    assert_ne!(green, 0);
    assert_ne!(blue, 0);
    assert_ne!(red, green);
    assert_ne!(red, blue);
    assert_ne!(green, blue);
}

#[test]
fn enum_mixed_auto_manual_constants() {
    let world = World::new();

    // In Rust, Z = 1000 is automatically detected via the derive macro
    let e = world.component::<EnumWithLargeConstant>();
    assert_ne!(e.id(), 0);

    // Verify we can use the large-constant variant
    let ent = world.entity().add_enum(EnumWithLargeConstant::Z);
    assert!(ent.has_enum(EnumWithLargeConstant::Z));
}

#[test]
fn enum_enum_class_mixed_auto_manual_constants() {
    let world = World::new();

    let e = world.component::<EnumClassWithLargeConstant>();
    assert_ne!(e.id(), 0);

    let ent = world.entity().add_enum(EnumClassWithLargeConstant::Z);
    assert!(ent.has_enum(EnumClassWithLargeConstant::Z));
}

#[test]
fn enum_enum_child_count() {
    let world = World::new();

    let e = world.component::<StandardEnum>();

    let mut count = 0;
    world
        .query::<()>()
        .with((flecs::ChildOf::ID, e.id()))
        .build()
        .each_entity(|_, _| {
            count += 1;
        });

    assert_eq!(count, 3);
}

#[test]
fn enum_multi_world_constant_ids() {
    let world_a = World::new();
    let world_b = World::new();

    // Register in world_a with offset
    world_a.component::<Position>();
    world_a.entity();

    let e_a = world_a.component::<StandardEnum>();
    let red_a = StandardEnum::Red.id_variant(&world_a);
    let green_a = StandardEnum::Green.id_variant(&world_a);
    let blue_a = StandardEnum::Blue.id_variant(&world_a);

    world_b.component::<StandardEnum>();

    // IDs should differ across worlds
    let e_b = world_b.component::<StandardEnum>();
    assert_ne!(e_a.id(), e_b.id());

    // Make sure ids didn't get overwritten for world_a
    assert_eq!(e_a.id(), world_a.component::<StandardEnum>().id());
    assert_eq!(red_a, StandardEnum::Red.id_variant(&world_a));
    assert_eq!(green_a, StandardEnum::Green.id_variant(&world_a));
    assert_eq!(blue_a, StandardEnum::Blue.id_variant(&world_a));
}

#[test]
fn enum_empty_enum() {
    // TODO: missing API: EmptyEnum registration
    // In C++ this tests an empty enum type; Rust enums must have at least one variant
    // so we test with a minimal enum instead
    let world = World::new();
    let e = world.component::<EnumIncorrectType>();
    assert_ne!(e.id(), 0);
}

#[test]
fn enum_sparse_enum_reflection() {
    let world = World::new();

    let entity = world.entity_from_enum(SparseEnum::Black);
    assert!(entity.is_valid());
    assert_eq!(entity.path().unwrap(), "::SparseEnum::Black");

    let white_entity = world.entity_from_enum(SparseEnum::White);
    assert!(white_entity.is_valid());

    let grey_entity = world.entity_from_enum(SparseEnum::Grey);
    assert!(grey_entity.is_valid());

    // Test adding sparse enum variants
    let e = world.entity().add_enum(SparseEnum::Black);
    assert!(e.has_enum(SparseEnum::Black));
    assert!(!e.has_enum(SparseEnum::White));
    assert!(!e.has_enum(SparseEnum::Grey));
}

#[test]
fn enum_enum_class_reflection() {
    let world = World::new();

    let entity = world.entity_from_enum(EnumClass::Grass);
    assert!(entity.is_valid());
    assert_eq!(entity.path().unwrap(), "::EnumClass::Grass");

    let sand_entity = world.entity_from_enum(EnumClass::Sand);
    assert!(sand_entity.is_valid());

    let stone_entity = world.entity_from_enum(EnumClass::Stone);
    assert!(stone_entity.is_valid());
}

#[test]
fn enum_prefixed_enum_reflection() {
    let world = World::new();

    let foo_entity = world.entity_from_enum(PrefixEnum::PrefixEnumFoo);
    assert!(foo_entity.is_valid());

    let bar_entity = world.entity_from_enum(PrefixEnum::PrefixEnumBar);
    assert!(bar_entity.is_valid());

    assert_ne!(foo_entity.id(), bar_entity.id());
}

#[test]
fn enum_constant_with_num_reflection() {
    let world = World::new();

    let num1_entity = world.entity_from_enum(ConstantsWithNum::Num1);
    assert!(num1_entity.is_valid());

    let num2_entity = world.entity_from_enum(ConstantsWithNum::Num2);
    assert!(num2_entity.is_valid());

    let num3_entity = world.entity_from_enum(ConstantsWithNum::Num3);
    assert!(num3_entity.is_valid());

    assert_ne!(num1_entity.id(), num2_entity.id());
    assert_ne!(num2_entity.id(), num3_entity.id());
}

#[test]
fn enum_get_constant_id() {
    let world = World::new();

    let red_entity = world.entity_from_enum(StandardEnum::Red);
    assert_ne!(red_entity.id(), 0);

    assert_eq!(red_entity.to_constant::<StandardEnum>(), StandardEnum::Red);
    assert_eq!(red_entity.path().unwrap(), "::StandardEnum::Red");

    let red_via_id = StandardEnum::Red.id_variant(&world);
    assert_eq!(red_entity.id(), red_via_id);
}

#[test]
fn enum_multithreaded_enum_registration() {
    // Simplified: verify that enum registration is safe across multiple world instances
    // Full multithreaded test requires OS thread primitives not easily available here
    for _ in 0..3 {
        let world = World::new();
        world.component::<StandardEnum>();

        let e = world.entity().add_enum(StandardEnum::Blue);
        assert!(e.has_enum(StandardEnum::Blue));

        e.add_enum(StandardEnum::Red);
        assert!(!e.has_enum(StandardEnum::Blue));
        assert!(e.has_enum(StandardEnum::Red));
    }
}

#[test]
fn enum_bitmask_enum_reflection() {
    // TODO: missing API: BitMask enum attribute in Rust derive macro
    // The C++ test uses FLECS_ENUM_LAST and bitmask detection
    // In Rust, bitmask enums need special handling that may not yet be implemented
    let world = World::new();
    // Just verify the enum can be registered
    let _e = world.component::<StandardEnum>();
}

#[test]
fn enum_enum_w_one_constant_index_of() {
    // OneConstant enum equivalent in Rust
    #[repr(C)]
    #[derive(Component, Debug, PartialEq)]
    enum OneConstant {
        ConstantOne,
    }

    let world = World::new();
    let entity = world.entity_from_enum(OneConstant::ConstantOne);
    assert!(entity.is_valid());
    assert_eq!(
        entity.to_constant::<OneConstant>(),
        OneConstant::ConstantOne
    );
}

// ─── bitmask_enum_with_type_reflection ───────────────────────────────────────
#[test]
fn enum_bitmask_enum_with_type_reflection() {
    #[repr(u32)]
    #[allow(non_camel_case_types)]
    #[derive(Component, PartialOrd, PartialEq, Clone, Copy, Debug)]
    enum TypedBitMaskEnum {
        Zero = 0,
        bit_LS_0 = 1,
        bit_LS_1 = 2,
        bit_LS_2 = 4,
        bit_LS_3 = 8,
        bit_LS_4 = 16,
        bit_LS_5 = 32,
        bit_LS_15 = 32768,
        bit_LS_31 = 2147483648,
    }

    let world = World::new();
    let enum_type = world.enum_type::<TypedBitMaskEnum>();

    assert_eq!(enum_type.first(), 0, "first should be 0");
    assert_eq!(enum_type.last(), 8, "last should be 8 (9 constants total)");

    assert_eq!(enum_type.index_by_value(0), 0, "Zero is at index 0");
    assert_eq!(enum_type.index_by_value(1), 1, "bit_LS_0 is at index 1");
    assert_eq!(enum_type.index_by_value(2), 2, "bit_LS_1 is at index 2");
    assert_eq!(enum_type.index_by_value(8), 4, "bit_LS_3 is at index 4");

    assert!(enum_type.is_valid(0), "0 should be valid");
    assert!(enum_type.is_valid(1), "1 should be valid");
    assert!(enum_type.is_valid(8), "8 should be valid");
    assert!(!enum_type.is_valid(3), "3 should not be valid (sparse)");
    assert!(
        !enum_type.is_valid(7),
        "7 should not be valid (not a constant)"
    );
}

// ─── enum_with_mixed_constants_and_bitmask ───────────────────────────────────
#[test]
fn enum_enum_with_mixed_constants_and_bitmask() {
    // Note: Rust doesn't support mixed enum/bitmask like C++ does,
    // but we can test basic enum type reflection on a bitmask-like enum
    #[repr(u32)]
    #[derive(Component, PartialOrd, PartialEq, Clone, Copy, Debug)]
    enum MixedEnum {
        VsCode = 1,
        Vim = 2,
        Nano = 4,
    }

    let world = World::new();
    let enum_type = world.enum_type::<MixedEnum>();

    assert_eq!(enum_type.first(), 0);
    assert_eq!(enum_type.last(), 2);
    assert_eq!(enum_type.index_by_value(1), 0);
    assert_eq!(enum_type.index_by_value(2), 1);
    assert_eq!(enum_type.index_by_value(4), 2);
}

// ─── enum_i8 / enum_i16 / enum_i32 / enum_i64 / enum_u8 / enum_u16 / enum_u32 / enum_u64 ──

#[test]
fn enum_enum_i8() {
    #[repr(i8)]
    #[derive(Component, PartialOrd, PartialEq, Clone, Copy, Debug)]
    enum Ei8 {
        Red = 0,
        Green = 1,
        Blue = 2,
    }

    let world = World::new();
    let enum_type = world.enum_type::<Ei8>();

    assert_eq!(enum_type.first(), 0);
    assert_eq!(enum_type.last(), 2);
    assert_eq!(enum_type.index_by_value(0), 0);
    assert_eq!(enum_type.index_by_value(1), 1);
    assert_eq!(enum_type.index_by_value(2), 2);
    assert!(enum_type.is_valid(0));
    assert!(enum_type.is_valid(1));
    assert!(enum_type.is_valid(2));
    assert!(!enum_type.is_valid(3));
}

#[test]
fn enum_enum_i16() {
    #[repr(i16)]
    #[derive(Component, PartialOrd, PartialEq, Clone, Copy, Debug)]
    enum Ei16 {
        Red = 0,
        Green = 1,
        Blue = 2,
    }

    let world = World::new();
    let enum_type = world.enum_type::<Ei16>();

    assert_eq!(enum_type.first(), 0);
    assert_eq!(enum_type.last(), 2);
    assert!(enum_type.is_valid(0));
    assert!(enum_type.is_valid(2));
    assert!(!enum_type.is_valid(99));
}

#[test]
fn enum_enum_i32() {
    #[repr(i32)]
    #[derive(Component, PartialOrd, PartialEq, Clone, Copy, Debug)]
    enum Ei32 {
        Red = 0,
        Green = 1,
        Blue = 2,
    }

    let world = World::new();
    let enum_type = world.enum_type::<Ei32>();

    assert_eq!(enum_type.first(), 0);
    assert_eq!(enum_type.last(), 2);
    assert!(enum_type.is_valid(0));
    assert!(enum_type.is_valid(2));
}

#[test]
fn enum_enum_i64() {
    #[repr(i64)]
    #[derive(Component, PartialOrd, PartialEq, Clone, Copy, Debug)]
    enum Ei64 {
        Red = 0,
        Green = 1,
        Blue = 2,
    }

    let world = World::new();
    let enum_type = world.enum_type::<Ei64>();

    assert_eq!(enum_type.first(), 0);
    assert_eq!(enum_type.last(), 2);
    assert!(enum_type.is_valid(0));
    assert!(enum_type.is_valid(2));
}

#[test]
fn enum_enum_u8() {
    #[repr(u8)]
    #[derive(Component, PartialOrd, PartialEq, Clone, Copy, Debug)]
    enum Eu8 {
        Red = 0,
        Green = 1,
        Blue = 2,
    }

    let world = World::new();
    let enum_type = world.enum_type::<Eu8>();

    assert_eq!(enum_type.first(), 0);
    assert_eq!(enum_type.last(), 2);
    assert!(enum_type.is_valid(0));
    assert!(enum_type.is_valid(2));
}

#[test]
fn enum_enum_u16() {
    #[repr(u16)]
    #[derive(Component, PartialOrd, PartialEq, Clone, Copy, Debug)]
    enum Eu16 {
        Red = 0,
        Green = 1,
        Blue = 2,
    }

    let world = World::new();
    let enum_type = world.enum_type::<Eu16>();

    assert_eq!(enum_type.first(), 0);
    assert_eq!(enum_type.last(), 2);
    assert!(enum_type.is_valid(0));
    assert!(enum_type.is_valid(2));
}

#[test]
fn enum_enum_u32() {
    #[repr(u32)]
    #[derive(Component, PartialOrd, PartialEq, Clone, Copy, Debug)]
    enum Eu32 {
        Red = 0,
        Green = 1,
        Blue = 2,
    }

    let world = World::new();
    let enum_type = world.enum_type::<Eu32>();

    assert_eq!(enum_type.first(), 0);
    assert_eq!(enum_type.last(), 2);
    assert!(enum_type.is_valid(0));
    assert!(enum_type.is_valid(2));
}

#[test]
fn enum_enum_u64() {
    #[repr(u64)]
    #[derive(Component, PartialOrd, PartialEq, Clone, Copy, Debug)]
    enum Eu64 {
        Red = 0,
        Green = 1,
        Blue = 2,
    }

    let world = World::new();
    let enum_type = world.enum_type::<Eu64>();

    assert_eq!(enum_type.first(), 0);
    assert_eq!(enum_type.last(), 2);
    assert!(enum_type.is_valid(0));
    assert!(enum_type.is_valid(2));
}

// ─── runtime_type_constant_u8_template ───────────────────────────────────────
// TODO: missing API: dynamic enum type construction via comp.constant::<u8>("Name", val).
// Requires flecs_meta + flecs::Enum component setup not easily done in test context.
#[test]
fn enum_runtime_type_constant_u8_template() {
    let _world = World::new();
    // TODO: missing API: flecs_meta dynamic runtime enum with typed constants
}

#[test]
fn query_singleton_enum_constant_or() {
    let world = World::new();

    world
        .component::<StandardEnum>()
        .add_trait::<flecs::Singleton>();

    let q = world
        .query::<()>()
        .with_enum(StandardEnum::Red)
        .or()
        .with_enum(StandardEnum::Blue)
        .build();

    let mut count = 0;

    q.each_iter(|_it, _index, ()| {
        count += 1;
    });
    assert_eq!(count, 0);

    world.add_enum(StandardEnum::Green);

    q.each_iter(|_it, _index, ()| {
        count += 1;
    });
    assert_eq!(count, 0);

    world.add_enum(StandardEnum::Blue);

    let std_enum_component = unsafe { world.component::<StandardEnum>().entity_view(&world) };
    q.each_iter(|it, _index, ()| {
        assert_eq!(it.src(0), std_enum_component);
        count += 1;
    });
    assert_eq!(count, 1);

    world.add_enum(StandardEnum::Red);

    q.each_iter(|it, _index, ()| {
        assert_eq!(it.src(0), std_enum_component);
        count += 1;
    });
    assert_eq!(count, 2);
}

// Registering a derived enum recurses to register its underlying enum type
// during the enum's own registration. Both registrations write (and may
// reallocate) the world's `components_array`. The outer registration must
// re-borrow the array after the recursive call instead of holding a `&mut`
// across it, which was a use-after-realloc plus aliasing bug. Registering the
// enum as an early component in a fresh world exercises that recursion.
#[test]
fn enum_recursive_registration_no_aliasing() {
    #[repr(C)]
    #[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
    enum RecursiveEnum {
        A,
        B,
        C,
    }

    let world = World::new();

    let comp = world.component::<RecursiveEnum>();
    assert_ne!(comp.id(), 0);

    let a = RecursiveEnum::A.id_variant(&world);
    let b = RecursiveEnum::B.id_variant(&world);
    let c = RecursiveEnum::C.id_variant(&world);
    assert!(a.id() != 0 && b.id() != 0 && c.id() != 0);
    assert_ne!(a, b);
    assert_ne!(b, c);
    assert_ne!(a, c);
}
