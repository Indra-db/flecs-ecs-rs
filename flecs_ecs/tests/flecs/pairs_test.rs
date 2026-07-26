#![allow(dead_code)]
#![allow(unused_imports)]
use crate::common_test::*;
use flecs_ecs::experimental::prelude::EntityGuardExt;
use flecs_ecs::prelude::*;

// Local types used across multiple tests in this file.
// Prefixed with `Local` to avoid conflicts with common_test fixtures.

#[derive(Component, Default, Clone, Copy)]
struct PairData {
    pub value: f32,
}

#[derive(Component, Default)]
struct LocalEats {
    pub amount: i32,
}

#[derive(Component, Default)]
struct LocalApples;

#[derive(Component, Default)]
struct LocalPears;

#[derive(Component, Default)]
struct LocalBegin;

#[derive(Component, Default)]
struct LocalEnd;

#[derive(Component, Default)]
struct LocalEvent {
    pub value: &'static str,
}

#[derive(Component, Default)]
struct LocalLikes;

#[derive(Component, Default)]
struct LocalBobTag;

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[test]
fn pairs_add_component_pair() {
    let world = World::new();

    let entity = world.entity().add((PairData::id(), Position::id()));

    assert!(entity.id() != 0);
    assert!(entity.has((PairData::id(), Position::id())));
    assert!(!entity.has((Position::id(), PairData::id())));

    assert_eq!(
        entity.archetype().to_string(),
        Some("(PairData,Position)".to_string())
    );
}

#[test]
fn pairs_add_tag_pair() {
    let world = World::new();

    world.component::<Position>();
    let pair_e = world.entity_named("Pair");

    let entity = world.entity().add((pair_e, Position::id()));

    assert!(entity.id() != 0);
    assert!(entity.has((pair_e, Position::id())));
    assert!(!entity.has((Position::id(), pair_e)));
    assert_eq!(
        entity.archetype().to_string(),
        Some("(Pair,Position)".to_string())
    );
}

#[test]
fn pairs_add_tag_pair_to_tag() {
    let world = World::new();

    let tag_e = world.entity_named("Tag");
    let pair_e = world.entity_named("Pair");

    let entity = world.entity().add((pair_e, tag_e));

    assert!(entity.id() != 0);
    assert!(entity.has((pair_e, tag_e)));
    assert_eq!(
        entity.archetype().to_string(),
        Some("(Pair,Tag)".to_string())
    );
}

#[test]
fn pairs_remove_component_pair() {
    let world = World::new();

    world.component::<Position>();
    world.component::<PairData>();

    let entity = world.entity().add((PairData::id(), Position::id()));

    assert!(entity.id() != 0);
    assert!(entity.has((PairData::id(), Position::id())));
    assert!(!entity.has((Position::id(), PairData::id())));

    assert_eq!(
        entity.archetype().to_string(),
        Some("(PairData,Position)".to_string())
    );

    entity.remove((Position::id(), PairData::id()));
    assert!(!entity.has((Position::id(), PairData::id())));
}

#[test]
fn pairs_remove_tag_pair() {
    let world = World::new();

    world.component::<Position>();
    let pair_e = world.entity_named("Pair");

    let entity = world.entity().add((pair_e, Position::id()));

    assert!(entity.id() != 0);
    assert!(entity.has((pair_e, Position::id())));
    assert!(!entity.has((Position::id(), pair_e)));
    assert_eq!(
        entity.archetype().to_string(),
        Some("(Pair,Position)".to_string())
    );

    entity.remove((pair_e, Position::id()));
    assert!(!entity.has((pair_e, Position::id())));
}

#[test]
fn pairs_remove_tag_pair_to_tag() {
    let world = World::new();

    let tag_e = world.entity_named("Tag");
    let pair_e = world.entity_named("Pair");

    let entity = world.entity().add((pair_e, tag_e));

    assert!(entity.id() != 0);
    assert!(entity.has((pair_e, tag_e)));
    assert_eq!(
        entity.archetype().to_string(),
        Some("(Pair,Tag)".to_string())
    );

    entity.remove((tag_e, pair_e));
    assert!(!entity.has((tag_e, pair_e)));
}

#[test]
fn pairs_set_component_pair() {
    let world = World::new();

    let entity = world
        .entity()
        .set_pair::<PairData, Position>(PairData { value: 10.0 });

    assert!(entity.id() != 0);
    assert!(entity.has((PairData::id(), Position::id())));
    assert!(!entity.has((Position::id(), PairData::id())));

    assert_eq!(
        entity.archetype().to_string(),
        Some("(PairData,Position)".to_string())
    );

    {
        let t = entity.get_ref::<&(PairData, Position)>().unwrap();
        assert_eq!(t.value as i32, 10);
    }
}

#[test]
fn pairs_set_tag_pair() {
    let world = World::new();

    let pair_e = world.entity_named("Pair");

    let entity = world.entity().set_second(pair_e, Position { x: 10, y: 20 });

    assert!(entity.id() != 0);
    assert!(entity.has((pair_e, Position::id())));
    assert_eq!(
        entity.archetype().to_string(),
        Some("(Pair,Position)".to_string())
    );

    let ptr = entity.get_second_untyped::<Position>(pair_e) as *const Position;
    assert!(!ptr.is_null());
    unsafe {
        assert_eq!((*ptr).x, 10);
        assert_eq!((*ptr).y, 20);
    }
}

#[test]
#[cfg(feature = "flecs_system")]
fn pairs_system_1_pair_instance() {
    let mut world = World::new();

    world
        .entity()
        .set_pair::<PairData, Position>(PairData { value: 10.0 });

    let invoke_count = alloc::sync::Arc::new(std::sync::Mutex::new(0i32));
    let entity_count = alloc::sync::Arc::new(std::sync::Mutex::new(0i32));
    let trait_value = alloc::sync::Arc::new(std::sync::Mutex::new(0i32));

    let ic = alloc::sync::Arc::clone(&invoke_count);
    let ec = alloc::sync::Arc::clone(&entity_count);
    let tv = alloc::sync::Arc::clone(&trait_value);

    world
        .system::<()>()
        .expr("(PairData, *)")
        .run(move |mut it| {
            while it.next() {
                let tr = it.field::<PairData>(0);
                *ic.lock().unwrap() += 1;
                for i in it.iter() {
                    *ec.lock().unwrap() += 1;
                    *tv.lock().unwrap() = tr[i].value as i32;
                }
            }
        })
        .expect("valid system query expression");

    world.progress();

    assert_eq!(*invoke_count.lock().unwrap(), 1);
    assert_eq!(*entity_count.lock().unwrap(), 1);
    assert_eq!(*trait_value.lock().unwrap(), 10);
}

#[test]
#[cfg(feature = "flecs_system")]
fn pairs_system_2_pair_instances() {
    let mut world = World::new();

    world
        .entity()
        .set_pair::<PairData, Position>(PairData { value: 10.0 })
        .set_pair::<PairData, Velocity>(PairData { value: 20.0 });

    let invoke_count = alloc::sync::Arc::new(std::sync::Mutex::new(0i32));
    let entity_count = alloc::sync::Arc::new(std::sync::Mutex::new(0i32));
    let trait_value = alloc::sync::Arc::new(std::sync::Mutex::new(0i32));

    let ic = alloc::sync::Arc::clone(&invoke_count);
    let ec = alloc::sync::Arc::clone(&entity_count);
    let tv = alloc::sync::Arc::clone(&trait_value);

    world
        .system::<()>()
        .expr("(PairData, *)")
        .run(move |mut it| {
            while it.next() {
                let tr = it.field::<PairData>(0);
                *ic.lock().unwrap() += 1;
                for i in it.iter() {
                    *ec.lock().unwrap() += 1;
                    *tv.lock().unwrap() += tr[i].value as i32;
                }
            }
        })
        .expect("valid system query expression");

    world.progress();

    assert_eq!(*invoke_count.lock().unwrap(), 2);
    assert_eq!(*entity_count.lock().unwrap(), 2);
    assert_eq!(*trait_value.lock().unwrap(), 30);
}

#[test]
fn pairs_override_pair() {
    let world = World::new();

    world
        .component::<PairData>()
        .add((flecs::OnInstantiate::ID, flecs::Inherit::ID));

    let base = world
        .entity()
        .set_pair::<PairData, Position>(PairData { value: 10.0 });

    let instance = world.entity().is_a(base);

    assert!(instance.has((PairData::id(), Position::id())));

    let pos_id = world.component_id::<Position>();
    let t_ptr = instance.get_first_untyped::<PairData>(pos_id) as *const PairData;
    let t2_ptr = base.get_first_untyped::<PairData>(pos_id) as *const PairData;
    unsafe {
        assert_eq!((*t_ptr).value as i32, 10);
        // inherited: same pointer as base
        assert_eq!(t_ptr, t2_ptr);
    }

    // Explicitly add to override (copies value from base)
    instance.add((PairData::id(), Position::id()));
    let t_ptr2 = instance.get_first_untyped::<PairData>(pos_id) as *const PairData;
    let t2_ptr2 = base.get_first_untyped::<PairData>(pos_id) as *const PairData;
    unsafe {
        assert_eq!((*t_ptr2).value as i32, 10);
        // now owns its own copy: different pointer
        assert_ne!(t_ptr2, t2_ptr2);
    }

    instance.remove((PairData::id(), Position::id()));
    let t_ptr3 = instance.get_first_untyped::<PairData>(pos_id) as *const PairData;
    let t2_ptr3 = base.get_first_untyped::<PairData>(pos_id) as *const PairData;
    unsafe {
        assert_eq!((*t_ptr3).value as i32, 10);
        // back to inherited: same as base
        assert_eq!(t_ptr3, t2_ptr3);
    }
}

#[test]
fn pairs_override_tag_pair() {
    let world = World::new();

    let pair_e = world
        .entity()
        .add((flecs::OnInstantiate::ID, flecs::Inherit::ID));

    let base = world.entity().set_second(pair_e, Position { x: 10, y: 20 });

    let instance = world.entity().is_a(base);

    assert!(instance.has((pair_e, Position::id())));

    let t_ptr = instance.get_second_untyped::<Position>(pair_e) as *const Position;
    let t2_ptr = base.get_second_untyped::<Position>(pair_e) as *const Position;
    unsafe {
        assert_eq!((*t_ptr).x, 10);
        assert_eq!((*t_ptr).y, 20);
        assert_eq!(t_ptr, t2_ptr);
    }

    // Override by setting explicitly on instance
    instance.set_second(pair_e, Position { x: 10, y: 20 });
    let t_ptr2 = instance.get_second_untyped::<Position>(pair_e) as *const Position;
    let t2_ptr2 = base.get_second_untyped::<Position>(pair_e) as *const Position;
    unsafe {
        assert_eq!((*t_ptr2).x, 10);
        assert_eq!((*t_ptr2).y, 20);
        assert_ne!(t_ptr2, t2_ptr2);
    }

    instance.remove((pair_e, Position::id()));
    let t_ptr3 = instance.get_second_untyped::<Position>(pair_e) as *const Position;
    let t2_ptr3 = base.get_second_untyped::<Position>(pair_e) as *const Position;
    unsafe {
        assert_eq!((*t_ptr3).x, 10);
        assert_eq!((*t_ptr3).y, 20);
        assert_eq!(t_ptr3, t2_ptr3);
    }
}

#[test]
fn pairs_ensure_pair() {
    // C++ ensure<Pair, Position>() returns &mut — initialises pair if absent.
    // In Rust: set_pair initialises and adds the pair, get verifies value.
    let world = World::new();

    let e = world.entity();
    e.set_pair::<PairData, Position>(PairData { value: 10.0 });

    {
        let t = e.get_ref::<&(PairData, Position)>().unwrap();
        assert_eq!(t.value as i32, 10);
    }
}

#[test]
fn pairs_ensure_pair_existing() {
    let world = World::new();

    let e = world
        .entity()
        .set_pair::<PairData, Position>(PairData { value: 20.0 });

    // Verify existing value, then mutate in-place
    {
        let mut t = e.get_ref::<&mut (PairData, Position)>().unwrap();
        assert_eq!(t.value as i32, 20);
        t.value = 10.0;
    }

    {
        let t = e.get_ref::<&(PairData, Position)>().unwrap();
        assert_eq!(t.value as i32, 10);
    }
}

#[test]
fn pairs_ensure_pair_tag() {
    // C++ ensure_second<Position>(pair_entity) returns &mut Position
    let world = World::new();

    let pair_e = world.entity();
    let e = world.entity().set_second(pair_e, Position { x: 10, y: 20 });

    let ptr = e.get_second_untyped::<Position>(pair_e) as *const Position;
    assert!(!ptr.is_null());
    unsafe {
        assert_eq!((*ptr).x, 10);
        assert_eq!((*ptr).y, 20);
    }
}

#[test]
fn pairs_ensure_pair_tag_existing() {
    let world = World::new();

    let pair_e = world.entity();
    let e = world.entity().set_second(pair_e, Position { x: 10, y: 20 });

    // already exists — get the value
    let ptr = e.get_second_untyped::<Position>(pair_e) as *const Position;
    assert!(!ptr.is_null());
    unsafe {
        assert_eq!((*ptr).x, 10);
        assert_eq!((*ptr).y, 20);
    }
}

#[test]
fn pairs_ensure_r_tag_o() {
    // C++ ensure<Tag, Position>() where Tag is ZST, Position is data — returns &mut Position
    let world = World::new();

    let e = world
        .entity()
        .set_pair::<Tag, Position>(Position { x: 10, y: 20 });

    {
        let mut t = e.get_ref::<&mut (Tag, Position)>().unwrap();
        assert_eq!(t.x, 10);
        assert_eq!(t.y, 20);
        t.x = 30;
        t.y = 40;
    }

    {
        let t = e.get_ref::<&(Tag, Position)>().unwrap();
        assert_eq!(t.x, 30);
        assert_eq!(t.y, 40);
    }
}

#[test]
fn pairs_get_relation_from_id() {
    let world = World::new();

    let rel = world.entity();
    let obj = world.entity();

    let pair = world.id_view_from((rel, obj));

    assert_eq!(pair.first_id(), rel);
    assert_ne!(pair.second_id(), rel);

    assert!(pair.first_id().is_alive());
    assert!(pair.first_id().is_valid());
}

#[test]
fn pairs_get_second_from_id() {
    let world = World::new();

    let rel = world.entity();
    let obj = world.entity();

    let pair = world.id_view_from((rel, obj));

    assert_ne!(pair.first_id(), obj);
    assert_eq!(pair.second_id(), obj);

    assert!(pair.second_id().is_alive());
    assert!(pair.second_id().is_valid());
}

#[test]
fn pairs_get_recycled_relation_from_id() {
    let world = World::new();

    let rel = world.entity();
    let obj = world.entity();

    rel.destruct();
    obj.destruct();

    let rel = world.entity();
    let obj = world.entity();

    // Make sure ids are recycled (generation bits set — high 32 bits non-zero)
    assert_ne!(*rel.id() as u32 as u64, *rel.id());
    assert_ne!(*obj.id() as u32 as u64, *obj.id());

    let pair = world.id_view_from((rel, obj));

    assert_eq!(pair.first_id(), rel);
    assert_ne!(pair.second_id(), rel);

    assert!(pair.first_id().is_alive());
    assert!(pair.first_id().is_valid());
}

#[test]
fn pairs_get_recycled_object_from_id() {
    let world = World::new();

    let rel = world.entity();
    let obj = world.entity();

    rel.destruct();
    obj.destruct();

    let rel = world.entity();
    let obj = world.entity();

    // Make sure ids are recycled
    assert_ne!(*rel.id() as u32 as u64, *rel.id());
    assert_ne!(*obj.id() as u32 as u64, *obj.id());

    let pair = world.id_view_from((rel, obj));

    assert_eq!(pair.first_id(), rel);
    assert_ne!(pair.second_id(), rel);

    assert!(pair.second_id().is_alive());
    assert!(pair.second_id().is_valid());
}

#[test]
fn pairs_get_rel_obj() {
    let world = World::new();

    let obj = world.entity();

    let e = world
        .entity()
        .set_first::<Position>(Position { x: 10, y: 20 }, obj);

    assert!(e.has((Position::id(), obj)));

    let ptr = e.get_first_untyped::<Position>(obj) as *const Position;
    assert!(!ptr.is_null());
    unsafe {
        assert_eq!((*ptr).x, 10);
        assert_eq!((*ptr).y, 20);
    }
}

#[test]
fn pairs_get_rel_obj_id() {
    // Same as pairs_get_rel_obj — C++ flecs::id vs flecs::id_t are both entity IDs in Rust
    let world = World::new();

    let obj = world.entity();

    let e = world
        .entity()
        .set_first::<Position>(Position { x: 10, y: 20 }, obj);

    assert!(e.has((Position::id(), obj)));

    let ptr = e.get_first_untyped::<Position>(obj) as *const Position;
    assert!(!ptr.is_null());
    unsafe {
        assert_eq!((*ptr).x, 10);
        assert_eq!((*ptr).y, 20);
    }
}

#[test]
fn pairs_get_rel_obj_id_t() {
    let world = World::new();

    let obj = world.entity();

    let e = world
        .entity()
        .set_first::<Position>(Position { x: 10, y: 20 }, obj);

    assert!(e.has((Position::id(), obj)));

    let ptr = e.get_first_untyped::<Position>(obj) as *const Position;
    assert!(!ptr.is_null());
    unsafe {
        assert_eq!((*ptr).x, 10);
        assert_eq!((*ptr).y, 20);
    }
}

#[test]
fn pairs_get_r_obj() {
    let world = World::new();

    let obj = world.entity();

    let e = world
        .entity()
        .set_first::<Position>(Position { x: 10, y: 20 }, obj);

    assert!(e.has((Position::id(), obj)));

    let ptr = e.get_first_untyped::<Position>(obj) as *const Position;
    assert!(!ptr.is_null());
    unsafe {
        assert_eq!((*ptr).x, 10);
        assert_eq!((*ptr).y, 20);
    }
}

#[test]
fn pairs_get_r_obj_id() {
    let world = World::new();

    let obj = world.entity();

    let e = world
        .entity()
        .set_first::<Position>(Position { x: 10, y: 20 }, obj);

    assert!(e.has((Position::id(), obj)));

    let ptr = e.get_first_untyped::<Position>(obj) as *const Position;
    assert!(!ptr.is_null());
    unsafe {
        assert_eq!((*ptr).x, 10);
        assert_eq!((*ptr).y, 20);
    }
}

#[test]
fn pairs_get_r_obj_id_t() {
    let world = World::new();

    let obj = world.entity();

    let e = world
        .entity()
        .set_first::<Position>(Position { x: 10, y: 20 }, obj);

    assert!(e.has((Position::id(), obj)));

    let ptr = e.get_first_untyped::<Position>(obj) as *const Position;
    assert!(!ptr.is_null());
    unsafe {
        assert_eq!((*ptr).x, 10);
        assert_eq!((*ptr).y, 20);
    }
}

#[test]
fn pairs_get_r_o() {
    let world = World::new();

    let e = world
        .entity()
        .set_pair::<Position, Tag>(Position { x: 10, y: 20 });

    assert!(e.has((Position::id(), Tag::id())));

    {
        let ptr = e.get_ref::<&(Position, Tag)>().unwrap();
        assert_eq!(ptr.x, 10);
        assert_eq!(ptr.y, 20);
    }
}

#[test]
fn pairs_get_r_tag_o() {
    let world = World::new();

    let e = world
        .entity()
        .set_pair::<Tag, Position>(Position { x: 10, y: 20 });

    assert!(e.has((Tag::id(), Position::id())));

    {
        let ptr = e.get_ref::<&(Tag, Position)>().unwrap();
        assert_eq!(ptr.x, 10);
        assert_eq!(ptr.y, 20);
    }
}

#[test]
fn pairs_get_second() {
    let world = World::new();

    let rel = world.entity();

    let e = world.entity().set_second(rel, Position { x: 10, y: 20 });

    assert!(e.has((rel, Position::id())));

    let ptr = e.get_second_untyped::<Position>(rel) as *const Position;
    assert!(!ptr.is_null());
    unsafe {
        assert_eq!((*ptr).x, 10);
        assert_eq!((*ptr).y, 20);
    }
}

#[test]
fn pairs_get_second_id() {
    let world = World::new();

    let rel = world.entity();

    let e = world.entity().set_second(rel, Position { x: 10, y: 20 });

    assert!(e.has((rel, Position::id())));

    let ptr = e.get_second_untyped::<Position>(rel) as *const Position;
    assert!(!ptr.is_null());
    unsafe {
        assert_eq!((*ptr).x, 10);
        assert_eq!((*ptr).y, 20);
    }
}

#[test]
fn pairs_get_second_id_t() {
    let world = World::new();

    let rel = world.entity();

    let e = world.entity().set_second(rel, Position { x: 10, y: 20 });

    assert!(e.has((rel, Position::id())));

    let ptr = e.get_second_untyped::<Position>(rel) as *const Position;
    assert!(!ptr.is_null());
    unsafe {
        assert_eq!((*ptr).x, 10);
        assert_eq!((*ptr).y, 20);
    }
}

#[test]
fn pairs_each() {
    let world = World::new();

    let p_1 = world.entity();
    let p_2 = world.entity();

    let e = world.entity().add(p_1).add(p_2);

    let mut count = 0i32;

    e.each_component(|id| {
        if count == 0 {
            assert_eq!(id, p_1);
        } else if count == 1 {
            assert_eq!(id, p_2);
        } else {
            panic!("unexpected id count");
        }
        count += 1;
    });

    assert_eq!(count, 2);
}

#[test]
fn pairs_each_pair() {
    let world = World::new();

    let pair_id = world.component_id::<PairData>();
    let pos = world.component_id::<Position>();
    let vel = world.component_id::<Velocity>();

    let e = world
        .entity()
        .add((PairData::id(), Position::id()))
        .add((PairData::id(), Velocity::id()));

    let mut count = 0i32;

    e.each_target(pair_id, |object| {
        if count == 0 {
            assert_eq!(object, pos);
        } else if count == 1 {
            assert_eq!(object, vel);
        } else {
            panic!("unexpected count");
        }
        count += 1;
    });

    assert_eq!(count, 2);
}

#[test]
fn pairs_each_pair_by_type() {
    let world = World::new();

    let pos = world.component_id::<Position>();
    let vel = world.component_id::<Velocity>();

    let e = world
        .entity()
        .add((PairData::id(), Position::id()))
        .add((PairData::id(), Velocity::id()));

    let mut count = 0i32;

    e.each_target(PairData::id(), |object| {
        if count == 0 {
            assert_eq!(object, pos);
        } else if count == 1 {
            assert_eq!(object, vel);
        } else {
            panic!("unexpected count");
        }
        count += 1;
    });

    assert_eq!(count, 2);
}

#[test]
fn pairs_each_pair_w_isa() {
    let world = World::new();

    let p_1 = world.entity();
    let p_2 = world.entity();

    let e = world.entity().is_a(p_1).is_a(p_2);

    let mut count = 0i32;

    e.each_target(flecs::IsA::ID, |object| {
        if count == 0 {
            assert_eq!(object, p_1);
        } else if count == 1 {
            assert_eq!(object, p_2);
        } else {
            panic!("unexpected count");
        }
        count += 1;
    });

    assert_eq!(count, 2);
}

#[test]
fn pairs_each_pair_w_recycled_rel() {
    let world = World::new();

    let e_1 = world.entity();
    let e_2 = world.entity();

    world.entity().destruct(); // force recycling

    let pair_e = world.entity();

    // ensure recycled (generation bits non-zero)
    assert_ne!(*pair_e.id() as u32 as u64, *pair_e.id());

    let e = world.entity().add((pair_e, e_1)).add((pair_e, e_2));

    let mut count = 0i32;

    e.each_target(pair_e, |object| {
        if count == 0 {
            assert_eq!(object, e_1);
        } else if count == 1 {
            assert_eq!(object, e_2);
        } else {
            panic!("unexpected count");
        }
        count += 1;
    });

    assert_eq!(count, 2);
}

#[test]
fn pairs_each_pair_w_recycled_obj() {
    let world = World::new();

    let pair_id = world.component_id::<PairData>();

    world.entity().destruct(); // force recycling
    let e_1 = world.entity();
    assert_ne!(*e_1.id() as u32 as u64, *e_1.id());

    world.entity().destruct();
    let e_2 = world.entity();
    assert_ne!(*e_2.id() as u32 as u64, *e_2.id());

    let e = world.entity().add((pair_id, e_1)).add((pair_id, e_2));

    let mut count = 0i32;

    e.each_target(pair_id, |object| {
        if count == 0 {
            assert_eq!(object, e_1);
        } else if count == 1 {
            assert_eq!(object, e_2);
        } else {
            panic!("unexpected count");
        }
        count += 1;
    });

    assert_eq!(count, 2);
}

#[test]
fn pairs_match_pair() {
    let world = World::new();

    let eats = world.entity();
    let dislikes = world.entity();
    let apples = world.entity();
    let pears = world.entity();
    let bananas = world.entity();

    let e = world
        .entity()
        .set(Position { x: 10, y: 20 }) // should not be matched
        .add((eats, apples))
        .add((eats, pears))
        .add((dislikes, bananas));

    let mut count = 0i32;

    e.each_pair(eats, apples, |id| {
        assert_eq!(id.first_id(), eats);
        assert_eq!(id.second_id(), apples);
        count += 1;
    });

    assert_eq!(count, 1);
}

#[test]
fn pairs_match_pair_obj_wildcard() {
    let world = World::new();

    let eats = world.entity();
    let dislikes = world.entity();
    let apples = world.entity();
    let pears = world.entity();
    let bananas = world.entity();

    let e = world
        .entity()
        .set(Position { x: 10, y: 20 }) // should not be matched
        .add((eats, apples))
        .add((eats, pears))
        .add((dislikes, bananas));

    let mut count = 0i32;

    e.each_pair(eats, flecs::Wildcard::ID, |id| {
        assert_eq!(id.first_id(), eats);
        let sec = id.second_id();
        assert!(sec == apples || sec == pears);
        count += 1;
    });

    assert_eq!(count, 2);
}

#[test]
fn pairs_match_pair_rel_wildcard() {
    let world = World::new();

    let eats = world.entity();
    let dislikes = world.entity();
    let apples = world.entity();
    let pears = world.entity();
    let bananas = world.entity();

    let e = world
        .entity()
        .set(Position { x: 10, y: 20 }) // should not be matched
        .add((eats, apples))
        .add((eats, pears))
        .add((dislikes, bananas));

    let mut count = 0i32;

    e.each_pair(flecs::Wildcard::ID, pears, |id| {
        assert_eq!(id.first_id(), eats);
        assert_eq!(id.second_id(), pears);
        count += 1;
    });

    assert_eq!(count, 1);
}

#[test]
fn pairs_match_pair_both_wildcard() {
    let world = World::new();

    let eats = world.entity();
    let dislikes = world.entity();
    let apples = world.entity();
    let pears = world.entity();
    let bananas = world.entity();

    let e = world
        .entity()
        .set(Position { x: 10, y: 20 }) // should not be matched
        .add((eats, apples))
        .add((eats, pears))
        .add((dislikes, bananas));

    let mut count = 0i32;

    e.each_pair(flecs::Wildcard::ID, flecs::Wildcard::ID, |_id| {
        count += 1;
    });

    assert_eq!(count, 3);
}

#[test]
fn pairs_has_tag_w_object() {
    let world = World::new();

    let bob = world.entity();
    let e = world.entity().add((LocalLikes::id(), bob));
    assert!(e.has((LocalLikes::id(), bob)));
}

#[test]
fn pairs_has_second_tag() {
    let world = World::new();

    let likes = world.entity();
    // C++: add_second<Bob>(Likes) — likes is first, Bob type is second (ZST/tag)
    // LocalBobTag is a tag (ZST), set_second requires DataComponent, so use add instead
    let e = world.entity().add((likes, LocalBobTag::id()));
    // (likes, LocalBobTag) pair
    assert!(e.has((likes, LocalBobTag::id())));
}

#[test]
fn pairs_add_pair_type() {
    // C++: using EatsApples = flecs::pair<Eats, Apples>; entity.add<EatsApples>()
    // Rust: add((LocalEats::id(), LocalApples::id()))
    let world = World::new();

    let e = world.entity().add((LocalEats::id(), LocalApples::id()));
    assert!(e.has((LocalEats::id(), LocalApples::id())));
}

#[test]
fn pairs_remove_pair_type() {
    let world = World::new();

    let e = world.entity().add((LocalEats::id(), LocalApples::id()));
    assert!(e.has((LocalEats::id(), LocalApples::id())));

    e.remove((LocalEats::id(), LocalApples::id()));
    assert!(!e.has((LocalEats::id(), LocalApples::id())));
}

#[test]
fn pairs_set_pair_type() {
    let world = World::new();

    let e = world
        .entity()
        .set_pair::<LocalEats, LocalApples>(LocalEats { amount: 10 });
    assert!(e.has((LocalEats::id(), LocalApples::id())));

    let apples_id = world.component_id::<LocalApples>();
    let ptr = e.get_first_untyped::<LocalEats>(apples_id) as *const LocalEats;
    unsafe {
        assert_eq!((*ptr).amount, 10);
    }

    // C++: ptr == e.try_get<Eats, Apples>() — both accesses yield same address
    let ptr2 = e.get_first_untyped::<LocalEats>(apples_id) as *const LocalEats;
    assert_eq!(ptr, ptr2);
}

#[test]
fn pairs_has_pair_type() {
    let world = World::new();

    let e = world.entity().add((LocalEats::id(), LocalApples::id()));
    assert!(e.has((LocalEats::id(), LocalApples::id())));
}

#[test]
fn pairs_get_1_pair_arg() {
    let world = World::new();

    let e = world
        .entity()
        .set_pair::<LocalEats, LocalApples>(LocalEats { amount: 10 });
    assert!(e.has((LocalEats::id(), LocalApples::id())));

    let result = e.get_ref::<&(LocalEats, LocalApples)>();
    assert!(result.is_some());
    assert_eq!(result.unwrap().amount, 10);
}

#[test]
fn pairs_get_2_pair_arg() {
    let world = World::new();

    let e = world
        .entity()
        .set_pair::<LocalEats, LocalApples>(LocalEats { amount: 10 })
        .set_pair::<LocalEats, LocalPears>(LocalEats { amount: 20 });

    assert!(e.has((LocalEats::id(), LocalApples::id())));
    assert!(e.has((LocalEats::id(), LocalPears::id())));

    let result = e.get_ref::<(&(LocalEats, LocalApples), &(LocalEats, LocalPears))>();
    assert!(result.is_some());
    let (a, p) = result.unwrap();
    assert_eq!(a.amount, 10);
    assert_eq!(p.amount, 20);
}

#[test]
fn pairs_set_1_pair_arg() {
    // C++: entity.insert([](EatsApples&& a) { a->amount = 10; })
    // No `insert` lambda API in Rust — use set_pair directly.
    let world = World::new();

    let e = world
        .entity()
        .set_pair::<LocalEats, LocalApples>(LocalEats { amount: 10 });

    {
        let eats = e.get_ref::<&(LocalEats, LocalApples)>().unwrap();
        assert_eq!(eats.amount, 10);
    }
}

#[test]
fn pairs_set_2_pair_arg() {
    // C++: entity.insert([](EatsApples&& a, EatsPears&& p) {...})
    // No `insert` lambda API in Rust — use two set_pair calls.
    let world = World::new();

    let e = world
        .entity()
        .set_pair::<LocalEats, LocalApples>(LocalEats { amount: 10 })
        .set_pair::<LocalEats, LocalPears>(LocalEats { amount: 20 });

    {
        let eats = e.get_ref::<&(LocalEats, LocalApples)>().unwrap();
        assert_eq!(eats.amount, 10);
    }

    {
        let eats = e.get_ref::<&(LocalEats, LocalPears)>().unwrap();
        assert_eq!(eats.amount, 20);
    }
}

#[test]
fn pairs_get_inline_pair_type() {
    // C++: e.get([](const flecs::pair<Eats, Apples>& a) {...})
    let world = World::new();

    let e = world
        .entity()
        .set_pair::<LocalEats, LocalApples>(LocalEats { amount: 10 });
    assert!(e.has((LocalEats::id(), LocalApples::id())));

    let result = e.get_ref::<&(LocalEats, LocalApples)>();
    assert!(result.is_some());
    assert_eq!(result.unwrap().amount, 10);
}

#[test]
fn pairs_set_inline_pair_type() {
    // C++: entity.insert([](flecs::pair<Eats, Apples>&& a) { a->amount = 10; })
    let world = World::new();

    let e = world
        .entity()
        .set_pair::<LocalEats, LocalApples>(LocalEats { amount: 10 });

    {
        let eats = e.get_ref::<&(LocalEats, LocalApples)>().unwrap();
        assert_eq!(eats.amount, 10);
    }
}

#[test]
fn pairs_get_pair_type_object() {
    // C++: set_second<Apples, Eats>({10}) — pair_object where Apples is first (ZST),
    // Eats is second (data). In Rust: set_pair::<LocalApples, LocalEats>.
    let world = World::new();

    let e = world
        .entity()
        .set_pair::<LocalApples, LocalEats>(LocalEats { amount: 10 });
    assert!(e.has((LocalApples::id(), LocalEats::id())));

    {
        let a = e.get_ref::<&(LocalApples, LocalEats)>().unwrap();
        assert_eq!(a.amount, 10);
    }
}

#[test]
fn pairs_set_pair_type_object() {
    // C++: entity.insert([](flecs::pair_object<Apples, Eats>&& a) { a->amount = 10; })
    let world = World::new();

    let e = world
        .entity()
        .set_pair::<LocalApples, LocalEats>(LocalEats { amount: 10 });

    {
        let eats = e.get_ref::<&(LocalApples, LocalEats)>().unwrap();
        assert_eq!(eats.amount, 10);
    }
}

#[test]
fn pairs_set_get_second_variants() {
    // C++ tests multiple equivalent ways to set (Begin, Event) with value "Big Bang".
    // In Rust, all variants reduce to set_pair::<LocalBegin, LocalEvent>.
    let world = World::new();

    // Variant 1: set_pair::<Begin, Event>
    let e1 = world
        .entity()
        .set_pair::<LocalBegin, LocalEvent>(LocalEvent { value: "Big Bang" });
    assert!(e1.has((LocalBegin::id(), LocalEvent::id())));
    {
        let v = e1.get_ref::<&(LocalBegin, LocalEvent)>().unwrap();
        assert_eq!(v.value, "Big Bang");
    }

    // Variant 2: same (C++ set<Begin,Event> == set_second<Begin,Event> == set_pair in Rust)
    let e2 = world
        .entity()
        .set_pair::<LocalBegin, LocalEvent>(LocalEvent { value: "Big Bang" });
    assert!(e2.has((LocalBegin::id(), LocalEvent::id())));
    {
        let v = e2.get_ref::<&(LocalBegin, LocalEvent)>().unwrap();
        assert_eq!(v.value, "Big Bang");
    }
}

#[test]
fn pairs_get_object_for_type_self() {
    let world = World::new();

    world
        .component::<Tag>()
        .add((flecs::OnInstantiate::ID, flecs::Inherit::ID));

    let base = world.entity().add(Tag::id());
    let self_e = world.entity().is_a(base).add(Tag::id());

    let obj = self_e.target_id_for(flecs::IsA::ID, Tag::id());
    assert!(obj.is_some());
    assert_eq!(obj.unwrap(), self_e);
}

#[test]
fn pairs_get_object_for_type_base() {
    let world = World::new();

    world
        .component::<Tag>()
        .add((flecs::OnInstantiate::ID, flecs::Inherit::ID));

    let base = world.entity().add(Tag::id());
    let self_e = world.entity().is_a(base);

    let obj = self_e.target_id_for(flecs::IsA::ID, Tag::id());
    assert!(obj.is_some());
    assert_eq!(obj.unwrap(), base);
}

#[test]
fn pairs_get_object_for_id_self() {
    let world = World::new();

    let tag = world
        .entity()
        .add((flecs::OnInstantiate::ID, flecs::Inherit::ID));
    let base = world.entity().add(tag);
    let self_e = world.entity().is_a(base).add(tag);

    let obj = self_e.target_id_for(flecs::IsA::ID, tag);
    assert!(obj.is_some());
    assert_eq!(obj.unwrap(), self_e);
}

#[test]
fn pairs_get_object_for_id_base() {
    let world = World::new();

    let tag = world
        .entity()
        .add((flecs::OnInstantiate::ID, flecs::Inherit::ID));
    let base = world.entity().add(tag);
    let self_e = world.entity().is_a(base);

    let obj = self_e.target_id_for(flecs::IsA::ID, tag);
    assert!(obj.is_some());
    assert_eq!(obj.unwrap(), base);
}

#[test]
fn pairs_get_object_for_id_not_found() {
    let world = World::new();

    let tag = world
        .entity()
        .add((flecs::OnInstantiate::ID, flecs::Inherit::ID));
    let base = world.entity();
    let self_e = world.entity().is_a(base);

    let obj = self_e.target_id_for(flecs::IsA::ID, tag);
    assert!(obj.is_none());
}

// The following four tests (deref_pair variants) test flecs::pair<R, O> as a standalone
// value-type wrapper that dereferences to the data component. No equivalent exists in Rust —
// pairs are purely ID-level constructs with no standalone wrapper value type.

#[test]
fn pairs_deref_pair() {
    // TODO: missing API: flecs::pair<R,O> standalone value-type wrapper with operator->/*
    // In Rust, pairs are component IDs only; no "pair as a value wrapper" concept exists.
    let _world = World::new();
}

#[test]
fn pairs_deref_const_pair() {
    // TODO: missing API: const flecs::pair<R,O> value-type wrapper
    let _world = World::new();
}

#[test]
fn pairs_deref_pair_obj() {
    // TODO: missing API: flecs::pair<Tag,Position> (second carries data) value-type wrapper
    let _world = World::new();
}

#[test]
fn pairs_deref_const_pair_obj() {
    // TODO: missing API: const flecs::pair<Tag,Position> value-type wrapper
    let _world = World::new();
}

#[test]
fn pairs_set_r_existing_value() {
    let world = World::new();

    let p = Position { x: 10, y: 20 };
    let e = world.entity().set_pair::<Position, Tag>(p);

    {
        let ptr = e.get_ref::<&(Position, Tag)>().unwrap();
        assert_eq!(ptr.x, 10);
        assert_eq!(ptr.y, 20);
    }
}

#[test]
fn pairs_symmetric_w_childof() {
    let world = World::new();

    world.component::<LocalLikes>().add(flecs::Symmetric::ID);

    let idk = world.entity_named("Idk");

    let bob = world.entity_named("Bob").child_of(idk);
    let alice = world
        .entity_named("Alice")
        .child_of(idk)
        .add((LocalLikes::id(), bob));

    assert!(bob.has((LocalLikes::id(), alice)));
}

#[test]
fn pairs_modified_tag_second() {
    // C++: observer<Position>().term_at(0).second<Tag>().event(OnSet).each(...)
    // then e.ensure<Position, Tag>() + e.modified<Position, Tag>()
    // In Rust: observer for (Position, Tag) pair triggered by set_pair
    let world = World::new();

    let count = alloc::sync::Arc::new(std::sync::Mutex::new(0i32));
    let count_c = alloc::sync::Arc::clone(&count);

    world
        .observer::<flecs::OnSet, &(Position, Tag)>()
        .each(move |p| {
            assert_eq!(p.x, 10);
            assert_eq!(p.y, 20);
            *count_c.lock().unwrap() += 1;
        });

    let e = world.entity();
    e.set_pair::<Position, Tag>(Position { x: 10, y: 20 });

    assert_eq!(*count.lock().unwrap(), 1);
}

#[test]
fn pairs_modified_tag_first() {
    // C++: observer().with<Tag, Position>().event(OnSet).each(iter, row, ...)
    // then e.ensure<Tag, Position>() + e.modified<Tag, Position>()
    // In Rust: observer for (Tag, Position) pair triggered by set_pair
    let world = World::new();

    let count = alloc::sync::Arc::new(std::sync::Mutex::new(0i32));
    let count_c = alloc::sync::Arc::clone(&count);

    world
        .observer::<flecs::OnSet, &(Tag, Position)>()
        .each(move |p| {
            assert_eq!(p.x, 10);
            assert_eq!(p.y, 20);
            *count_c.lock().unwrap() += 1;
        });

    let e = world.entity();
    e.set_pair::<Tag, Position>(Position { x: 10, y: 20 });

    assert_eq!(*count.lock().unwrap(), 1);
}
