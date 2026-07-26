#![allow(dead_code)]

use crate::common_test::*;
use flecs_ecs::experimental::prelude::EntityGuardExt;

#[test]
fn bulk_entity_builder_simple_add() {
    let world = World::new();

    let entities = world
        .entity_bulk(10)
        .add::<Position>()
        .add::<Velocity>()
        .build();

    assert_eq!(entities.len(), 10);

    for entity in entities {
        let entity = world.entity_from_id(entity);
        assert!(entity.has(Position::id()));
        assert!(entity.has(Velocity::id()));
    }
}

#[test]
fn bulk_entity_builder_simple_set() {
    let world = World::new();
    let positions: [Position; 10] = core::array::from_fn(|i| Position {
        x: i as i32,
        y: i as i32,
    });
    let velocities: [Velocity; 10] = core::array::from_fn(|i| Velocity {
        x: (i as i32) * 2,
        y: (i as i32) * 2,
    });

    let entities = world
        .entity_bulk(10)
        .set(&positions)
        .set(&velocities)
        .build();

    assert_eq!(entities.len(), 10);
    for (index, entity) in entities.into_iter().enumerate() {
        let entity = world.entity_from_id(entity);
        assert!(entity.has(Position::id()));
        assert!(entity.has(Velocity::id()));
        let position = entity.cloned_owned::<&Position>().unwrap();
        let velocity = entity.cloned_owned::<&Velocity>().unwrap();
        assert_eq!(position.x, velocity.x / 2);
        assert_eq!(position.y, velocity.y / 2);
        assert_eq!(position.x, index as i32);
        assert_eq!(position.y, index as i32);
    }
}

#[test]
fn bulk_entity_builder_table() {
    let world = World::new();

    let ent_id = world.entity();

    let ent = world
        .entity()
        .add(Position::id())
        .add(Velocity::id())
        .add(ent_id);

    let mut table = ent.table().unwrap();

    let positions: [Position; 10] = core::array::from_fn(|i| Position {
        x: i as i32,
        y: i as i32,
    });
    let velocities: [Velocity; 10] = core::array::from_fn(|i| Velocity {
        x: (i * 2) as i32,
        y: (i * 2) as i32,
    });

    let random_ent_id = world.entity();

    let entities = world
        .entity_bulk(10)
        .add::<Mass>()
        .set(&velocities)
        .add_id(random_ent_id)
        .set(&positions)
        .build_to_table(&mut table);

    assert_eq!(entities.len(), 10);

    for (index, entity) in entities.into_iter().enumerate() {
        let entity = world.entity_from_id(entity);
        assert!(entity.has(Position::id()));
        assert!(entity.has(Velocity::id()));
        assert!(entity.has(ent_id));

        assert!(!entity.has(Mass::id()));
        assert!(!entity.has(random_ent_id));

        let position = entity.cloned_owned::<&Position>().unwrap();
        let velocity = entity.cloned_owned::<&Velocity>().unwrap();
        assert_eq!(position.x, velocity.x / 2);
        assert_eq!(position.y, velocity.y / 2);
        assert_eq!(position.x, index as i32);
        assert_eq!(position.y, index as i32);
    }
}

// panics in C
// #[test]
// fn bulk_entity_builder_zero_entities() {
//     let world = World::new();

//     let result = std::panic::catch_unwind(|| {
//         world.entity_bulk(0).add::<Position>().build();
//     });

//     assert!(result.is_err());
// }

#[cfg(any(debug_assertions, feature = "flecs_force_enable_ecs_asserts"))]
mod debug_only {
    use super::*;

    #[test]
    fn bulk_entity_builder_max_entities() {
        let world = World::new();

        let max_count = i32::MAX as u32;
        let result = std::panic::catch_unwind(|| {
            world.entity_bulk(max_count + 1);
        });

        assert!(result.is_err());
    }

    #[test]
    fn bulk_entity_builder_invalid_component() {
        let world = World::new();

        #[derive(Component)]
        struct NonDefaultComponent {
            value: i32,
        }

        let id = world.component_id::<NonDefaultComponent>();

        let result = std::panic::catch_unwind(|| {
            world.entity_bulk(10).add_id(id).build();
        });

        assert!(result.is_err());
    }

    #[test]
    fn bulk_entity_builder_set_mismatched_length() {
        let world = World::new();
        let positions: [Position; 5] = [Position { x: 0, y: 0 }; 5];

        let result = std::panic::catch_unwind(|| {
            world.entity_bulk(10).set(&positions).build();
        });

        assert!(result.is_err());
    }
}

#[ignore = "ecs_bulk_init requires entity IDs that were never made alive (raw unregistered IDs). world.entity() creates live entities; deleting them changes generation so recycled IDs don't match. Needs IDs from a fresh range via ecs_set_entity_range or similar low-level allocation."]
#[test]
fn bulk_entity_builder_with_entity_ids() {
    let world = World::new();
    let _guard = FlecsPanicAbortGuard::install();

    // ecs_bulk_init requires entity IDs that are NOT alive (never created via ecs_new).
    // Create entities, collect their IDs, then delete them so the IDs become "not alive".
    // The bulk builder will then re-create those specific IDs with the specified components.
    let entity_ids: Vec<Entity> = (0..10).map(|_| world.entity().id()).collect();
    for &id in &entity_ids {
        world.entity_from_id(id).destruct();
    }

    let new_entities = world
        .entity_bulk_with_entity_ids(&entity_ids)
        .add::<Position>()
        .build();

    assert_eq!(new_entities.len(), 10);

    for &new_id in &new_entities {
        let entity = world.entity_from_id(new_id);
        assert!(entity.is_alive());
        assert!(entity.has(Position::id()));
    }
}

#[test]
fn bulk_entity_builder_add_and_set() {
    let world = World::new();
    let positions: [Position; 10] = core::array::from_fn(|i| Position {
        x: i as i32,
        y: i as i32,
    });

    let entities = world
        .entity_bulk(10)
        .add::<Velocity>()
        .set(&positions)
        .build();

    assert_eq!(entities.len(), 10);

    for entity in entities {
        let entity = world.entity_from_id(entity);
        assert!(entity.has(Position::id()));
        assert!(entity.has(Velocity::id()));
        let position = entity.cloned_owned::<&Position>().unwrap();
        assert_eq!(position.x, position.y);
    }
}

#[test]
#[should_panic]
fn bulk_entity_builder_build_to_table_missing_default() {
    let world = World::new();

    #[derive(Component)]
    struct NonDefaultComponent {
        value: i32,
    }

    let ent = world
        .entity()
        .add(Position::id())
        .set(NonDefaultComponent { value: 5 });

    let mut table = ent.table().unwrap();

    let positions: [Position; 10] = [Position { x: 0, y: 0 }; 10];

    world
        .entity_bulk(10)
        .set(&positions)
        .build_to_table(&mut table);
}

#[test]
fn bulk_entity_builder_duplicate_add() {
    let world = World::new();

    let entities = world
        .entity_bulk(10)
        .add::<Position>()
        .add::<Position>()
        .build();

    assert_eq!(entities.len(), 10);

    for entity in entities {
        let entity = world.entity_from_id(entity);
        assert!(entity.has(Position::id()));
    }
}

#[test]
fn bulk_entity_builder_set_after_add() {
    let world = World::new();
    let positions: [Position; 10] = core::array::from_fn(|i| Position {
        x: i as i32,
        y: i as i32,
    });

    let entities = world
        .entity_bulk(10)
        .add::<Position>()
        .set(&positions)
        .build();

    assert_eq!(entities.len(), 10);

    for entity in entities {
        let entity = world.entity_from_id(entity);
        assert!(entity.has(Position::id()));
        let position = entity.cloned_owned::<&Position>().unwrap();
        assert_eq!(position.x, position.y);
    }
}

#[test]
fn bulk_entity_builder_no_components() {
    let world = World::new();

    let entities = world.entity_bulk(10).build();

    assert_eq!(entities.len(), 10);

    for entity in entities {
        let entity = world.entity_from_id(entity);
        assert!(!entity.has(Position::id()));
    }
}

#[test]
fn bulk_entity_builder_set_same_component_multiple_times() {
    let world = World::new();
    let positions1: [Position; 10] = [Position { x: 1, y: 1 }; 10];
    let positions2: [Position; 10] = [Position { x: 2, y: 2 }; 10];

    let entities = world
        .entity_bulk(10)
        .set(&positions1)
        .set(&positions2)
        .build();

    assert_eq!(entities.len(), 10);

    for entity in entities {
        let entity = world.entity_from_id(entity);
        let position = entity.cloned_owned::<&Position>().unwrap();
        assert_eq!(position.x, 2);
        assert_eq!(position.y, 2);
    }
}

#[test]
fn bulk_entity_builder_with_entity_ids_accepts_entity_views() {
    let world = World::new();

    let ids: [u64; 3] = [100_000, 100_001, 100_002];
    let views: Vec<EntityView> = ids.iter().map(|&id| world.entity_from_id(id)).collect();

    let created = world
        .entity_bulk_with_entity_ids(&views)
        .add::<Position>()
        .build();

    assert_eq!(created.len(), 3);
    for (created_id, &expected) in created.iter().zip(ids.iter()) {
        assert_eq!(**created_id, expected);
        let entity = world.entity_from_id(*created_id);
        assert!(entity.is_alive());
        assert!(entity.has(Position::id()));
    }
}

#[test]
fn bulk_entity_builder_entity_ids_owned_by_builder() {
    let world = World::new();

    let ids: [u64; 3] = [200_000, 200_001, 200_002];
    let mut builder = {
        let temp: Vec<Entity> = ids.iter().map(|&id| Entity(id)).collect();
        world.entity_bulk_with_entity_ids(&temp)
    };

    let created = builder.add::<Position>().build();

    assert_eq!(created.len(), 3);
    for (created_id, &expected) in created.iter().zip(ids.iter()) {
        assert_eq!(**created_id, expected);
        assert!(world.entity_from_id(*created_id).has(Position::id()));
    }
}
