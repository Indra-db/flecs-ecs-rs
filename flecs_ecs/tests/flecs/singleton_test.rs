#![allow(dead_code)]
#![allow(unused_imports)]
#![allow(clippy::float_cmp)]

use flecs_ecs::core::*;
use flecs_ecs::macros::*;

use crate::common_test::*;

// ─── tests ──────────────────────────────────────────────────────────────────

/// C++ `world.try_get<T>()` returns a nullable pointer.
/// Rust uses a callback pattern; `try_get` returns `Some(...)` only when the component exists.

#[test]
fn singleton_set_get_singleton() {
    let world = World::new();

    world.set(Position { x: 10, y: 20 });

    let result = world.try_get::<&Position>(|p| (p.x, p.y));
    assert!(result.is_some());
    let (x, y) = result.unwrap();
    assert_eq!(x, 10);
    assert_eq!(y, 20);
}

#[test]
fn singleton_ensure_singleton() {
    // C++ ensure<T>() = get-or-add mutable ref.
    // Rust equivalent: set default then mutate via get::<&mut T>.
    let world = World::new();

    world.set(Position::default());
    world.get::<&mut Position>(|p| {
        p.x = 10;
        p.y = 20;
    });

    let result = world.try_get::<&Position>(|p| (p.x, p.y));
    assert!(result.is_some());
    let (x, y) = result.unwrap();
    assert_eq!(x, 10);
    assert_eq!(y, 20);
}

#[test]
fn singleton_get_mut_singleton() {
    let world = World::new();

    // Before setting: try_get returns None (nullptr in C++)
    let result = world.try_get::<&mut Position>(|_p| ());
    assert!(result.is_none());

    world.set(Position { x: 10, y: 20 });

    // After setting: try_get returns Some
    let result = world.try_get::<&mut Position>(|p| (p.x, p.y));
    assert!(result.is_some());
    let (x, y) = result.unwrap();
    assert_eq!(x, 10);
    assert_eq!(y, 20);
}

#[test]
fn singleton_emplace_singleton() {
    // C++ world.emplace<T>(args) = in-place construct.
    // No direct Rust equivalent; use world.set(T { ... }).
    // TODO: missing API: world.emplace (in-place construction, use world.set instead)
    let world = World::new();

    world.set(Position { x: 10, y: 20 });

    let result = world.try_get::<&Position>(|p| (p.x, p.y));
    assert!(result.is_some());
    let (x, y) = result.unwrap();
    assert_eq!(x, 10);
    assert_eq!(y, 20);
}

#[test]
fn singleton_modified_singleton() {
    // C++: e.ensure<Position>() (no OnSet), then e.modified<Position>() triggers OnSet.
    // Rust: e.add(Position::id()) to add without triggering OnSet, then e.modified(Position::id()).
    let world = World::new();

    world.set(Count(0));

    world
        .observer::<flecs::OnSet, &Position>()
        .each_entity(|e, _p| {
            e.world().get::<&mut Count>(|c| c.0 += 1);
        });

    let e = world.entity();
    e.add(Position::id());

    // No OnSet yet
    world.get::<&Count>(|c| assert_eq!(c.0, 0));

    e.modified(Position::id());

    world.get::<&Count>(|c| assert_eq!(c.0, 1));
}

#[test]
fn singleton_add_singleton() {
    let world = World::new();

    world.set(Count(0));

    world
        .observer::<flecs::OnAdd, ()>()
        .with(Position::id())
        .each_entity(|e, _| {
            e.world().get::<&mut Count>(|c| c.0 += 1);
        });

    world.add(Position::id());

    world.get::<&Count>(|c| assert_eq!(c.0, 1));
}

#[test]
fn singleton_remove_singleton() {
    let world = World::new();

    world.set(Count(0));

    world
        .observer::<flecs::OnRemove, &Position>()
        .each_entity(|e, _p| {
            e.world().get::<&mut Count>(|c| c.0 += 1);
        });

    // C++ uses world.ensure<Position>() which just adds (no OnRemove).
    world.add(Position::id());

    world.get::<&Count>(|c| assert_eq!(c.0, 0));

    world.remove(Position::id());

    world.get::<&Count>(|c| assert_eq!(c.0, 1));
}

#[test]
fn singleton_has_singleton() {
    let world = World::new();

    assert!(!world.has(Position::id()));

    world.set(Position { x: 10, y: 20 });

    assert!(world.has(Position::id()));
}

#[test]
fn singleton_singleton_system() {
    let world = World::new();

    world
        .component::<Position>()
        .add_trait::<flecs::Singleton>();

    world.set(Position { x: 10, y: 20 });

    world.system::<()>().expr("[inout] Position").run(|mut it| {
        while it.next() {
            let mut p = it.field_mut::<Position>(0);
            assert_eq!(p[0].x, 10);
            assert_eq!(p[0].y, 20);
            p[0].x += 1;
            p[0].y += 1;
        }
    }).expect("valid system query expression");

    world.progress();

    let result = world.try_get::<&Position>(|p| (p.x, p.y));
    assert!(result.is_some());
    let (x, y) = result.unwrap();
    assert_eq!(x, 11);
    assert_eq!(y, 21);
}

#[test]
fn singleton_get_singleton() {
    let world = World::new();

    world.set(Position { x: 10, y: 20 });

    let s = world.singleton::<Position>();
    assert!(s.has(Position::id()));
    assert_eq!(*s.id(), *world.component_id::<Position>());

    s.get::<&Position>(|p| {
        assert_eq!(p.x, 10);
        assert_eq!(p.y, 20);
    });
}

#[test]
fn singleton_type_id_from_world() {
    let world = World::new();

    world.set(Position { x: 10, y: 20 });

    let id = world.component_id::<Position>();
    assert_eq!(id, world.component_id::<Position>());

    let s = world.singleton::<Position>();
    assert_eq!(*s.id(), *world.component_id::<Position>());
    assert_eq!(*s.id(), *world.component_id::<Position>());
}

#[test]
fn singleton_set_lambda() {
    // C++ `world.set(lambda)` mutates singleton via lambda.
    // Rust: set default then use get::<&mut T>(closure) to mutate.
    let world = World::new();

    world.set(Position::default());
    world.get::<&mut Position>(|p| {
        p.x = 10;
        p.y = 20;
    });

    let result = world.try_get::<&Position>(|p| (p.x, p.y));
    assert_eq!(result.unwrap(), (10, 20));

    // Second "set lambda": increment
    world.get::<&mut Position>(|p| {
        p.x += 1;
        p.y += 1;
    });

    let result = world.try_get::<&Position>(|p| (p.x, p.y));
    assert_eq!(result.unwrap(), (11, 21));
}

#[test]
fn singleton_get_lambda() {
    let world = World::new();

    world.set(Position { x: 10, y: 20 });

    let mut count = 0i32;
    world.get::<&Position>(|p| {
        assert_eq!(p.x, 10);
        assert_eq!(p.y, 20);
        count += 1;
    });

    assert_eq!(count, 1);
}

#[test]
fn singleton_get_write_lambda() {
    let world = World::new();

    world.set(Position { x: 10, y: 20 });

    let mut count = 0i32;
    world.get::<&mut Position>(|p| {
        assert_eq!(p.x, 10);
        assert_eq!(p.y, 20);
        p.x += 1;
        p.y += 1;
        count += 1;
    });

    assert_eq!(count, 1);

    let result = world.try_get::<&Position>(|p| (p.x, p.y));
    assert_eq!(result.unwrap(), (11, 21));
}

#[test]
fn singleton_get_set_singleton_pair_r_t() {
    // C++: world.set<Position, Tag>({10, 20})
    // Rust: world.set_pair::<Position, Tag>(Position { x: 10, y: 20 })
    let world = World::new();

    world.set_pair::<Position, Tag>(Position { x: 10, y: 20 });

    let result = world.try_get::<&(Position, Tag)>(|p| (p.x, p.y));
    assert!(result.is_some());
    let (x, y) = result.unwrap();
    assert_eq!(x, 10);
    assert_eq!(y, 20);
}

#[test]
fn singleton_get_set_singleton_pair_r_entity() {
    // C++: world.set<Position>(tgt, {10, 20}) — pair with runtime entity target
    // Rust: world.set_first::<Position>(tgt, data)
    // Data is stored on the Position entity as (Position, tgt) pair.
    let world = World::new();

    let tgt = world.entity();

    world.set_first::<Position>(tgt, Position { x: 10, y: 20 });

    // Retrieve via the singleton entity for Position using untyped pointer.
    let pos_entity = EntityView::new_from(&world, *world.component_id::<Position>());
    let ptr = pos_entity.get_first_untyped::<Position>(tgt);
    assert!(!ptr.is_null());
    let p = unsafe { &*(ptr as *const Position) };
    assert_eq!(p.x, 10);
    assert_eq!(p.y, 20);
}

#[test]
fn singleton_add_remove_singleton_pair_r_t_types() {
    // C++: world.add<Position, Tag>() / world.has<Position, Tag>() / world.remove<Position, Tag>()
    // Rust: world.add((Position::id(), Tag::id()))
    let world = World::new();

    world.add((Position::id(), Tag::id()));
    assert!(world.has((Position::id(), Tag::id())));
    world.remove((Position::id(), Tag::id()));
    assert!(!world.has((Position::id(), Tag::id())));
}

#[test]
fn singleton_add_remove_singleton_pair_r_entity() {
    // C++: world.add<Position>(tgt)
    let world = World::new();

    let tgt = world.entity();

    world.add((Position::id(), *tgt));
    assert!(world.has((Position::id(), *tgt)));
    world.remove((Position::id(), *tgt));
    assert!(!world.has((Position::id(), *tgt)));
}

#[test]
fn singleton_add_remove_singleton_pair_entities() {
    // C++: world.add(rel, tgt)
    let world = World::new();

    let rel = world.entity();
    let tgt = world.entity();

    world.add((*rel, *tgt));
    assert!(world.has((*rel, *tgt)));
    world.remove((*rel, *tgt));
    assert!(!world.has((*rel, *tgt)));
}

#[test]
fn singleton_get_target() {
    let world = World::new();

    let rel = world.singleton::<Tag>();

    let obj1 = world.entity().add(Position::id());
    let obj2 = world.entity().add(Velocity::id());
    let obj3 = world.entity().add(Mass::id());

    let entities = [obj1, obj2, obj3];

    // C++: world.add<Tag>(obj1) — adds (Tag singleton entity, obj1) relationship on Tag entity
    // == world.add((Tag::id(), *obj1))
    world.add((Tag::id(), *obj1));
    world.add((Tag::id(), *obj2));
    world.add((*rel.id(), *obj3));

    // world.target<Tag>() — first target of Tag relationship on its singleton entity
    let p = world.target(Tag::id(), None);
    assert!(*p != 0);
    assert_eq!(p, obj1);

    // world.target<Tag>(Rel) == world.target(rel, None) since Rel IS the Tag singleton entity
    let p = world.target(*rel.id(), None);
    assert!(*p != 0);
    assert_eq!(p, obj1);

    // world.target(Rel)
    let p = world.target(*rel.id(), None);
    assert!(*p != 0);
    assert_eq!(p, obj1);

    // world.target<Tag>(i) for i in 0..3
    for (i, entity) in entities.iter().enumerate() {
        let p = world.target(Tag::id(), Some(i));
        assert!(*p != 0);
        assert_eq!(p, *entity);
    }

    // world.target<Tag>(Rel, i)
    for (i, entity) in entities.iter().enumerate() {
        let p = world.target(*rel.id(), Some(i));
        assert!(*p != 0);
        assert_eq!(p, *entity);
    }

    // world.target(Rel, i)
    for (i, entity) in entities.iter().enumerate() {
        let p = world.target(*rel.id(), Some(i));
        assert!(*p != 0);
        assert_eq!(p, *entity);
    }
}

// Color enum for singleton_singleton_enum test.
#[derive(Debug, Clone, Copy, PartialEq, Component)]
#[repr(C)]
#[flecs(meta)]
enum SColor {
    Red,
    Green,
    Blue,
}

#[test]
fn singleton_singleton_enum() {
    let world = World::new();

    // Use set() to store enum as component value (like C++ world.set<Color>(Blue))
    world.set(SColor::Blue);
    assert!(world.has(SColor::id()));

    {
        let result = world.try_get::<&SColor>(|c| *c);
        assert!(result.is_some());
        assert_eq!(result.unwrap(), SColor::Blue);
    }

    // Replace with Green
    world.set(SColor::Green);
    assert!(world.has(SColor::id()));

    {
        let result = world.try_get::<&SColor>(|c| *c);
        assert!(result.is_some());
        assert_eq!(result.unwrap(), SColor::Green);
    }

    world.remove(SColor::id());
    assert!(!world.has(SColor::id()));
}

#[test]
fn singleton_get_w_id() {
    // C++: const Position* p = (const Position*) world.get(world.id<Position>())
    // Rust: world.try_get::<&Position>(callback)
    let world = World::new();

    world.set(Position { x: 10, y: 20 });

    let result = world.try_get::<&Position>(|p| (p.x, p.y));
    assert!(result.is_some());
    let (x, y) = result.unwrap();
    assert_eq!(x, 10);
    assert_eq!(y, 20);
}

#[test]
fn singleton_get_t() {
    // C++: const Position& p = world.get<Position>()  (panics if missing)
    let world = World::new();

    world.set(Position { x: 10, y: 20 });

    world.get::<&Position>(|p| {
        assert_eq!(p.x, 10);
        assert_eq!(p.y, 20);
    });
}

#[test]
fn singleton_get_r_t() {
    // C++: world.set<Position>(tgt, {10, 20}); world.get(world.id<Position>(), tgt)
    // Rust: set_first + get_first_untyped on Position singleton entity
    let world = World::new();

    let tgt = world.entity();
    world.set_first::<Position>(tgt, Position { x: 10, y: 20 });

    let pos_entity = EntityView::new_from(&world, *world.component_id::<Position>());
    let ptr = pos_entity.get_first_untyped::<Position>(tgt);
    assert!(!ptr.is_null());
    let p = unsafe { &*(ptr as *const Position) };
    assert_eq!(p.x, 10);
    assert_eq!(p.y, 20);
}

#[test]
fn singleton_get_r_t_typed() {
    // C++: const Position& p = world.get<Position>(tgt)
    // TODO: missing API: world.get::<Position>(entity_tgt) — typed singleton get with entity second
    // Approximated using get_first_untyped on the Position singleton entity.
    let world = World::new();

    let tgt = world.entity();
    world.set_first::<Position>(tgt, Position { x: 10, y: 20 });

    let pos_entity = EntityView::new_from(&world, *world.component_id::<Position>());
    let ptr = pos_entity.get_first_untyped::<Position>(tgt);
    assert!(!ptr.is_null());
    let p = unsafe { &*(ptr as *const Position) };
    assert_eq!(p.x, 10);
    assert_eq!(p.y, 20);
}

#[test]
fn singleton_get_r_t_pair_types() {
    // C++: world.set<Position, Tgt>({10, 20}); const Position& p = world.get<Position, Tgt>()
    #[derive(Component)]
    struct Tgt;

    let world = World::new();

    world.set_pair::<Position, Tgt>(Position { x: 10, y: 20 });

    world.get::<&(Position, Tgt)>(|p| {
        assert_eq!(p.x, 10);
        assert_eq!(p.y, 20);
    });
}

// ── "not found" tests — must panic (C++ calls test_expect_abort) ─────────────

#[test]
#[should_panic]
fn singleton_get_w_id_not_found() {
    let world = World::new();
    // No Position set — must panic
    world.get::<&Position>(|_p| {});
}

#[test]
#[should_panic]
fn singleton_get_t_not_found() {
    let world = World::new();
    world.get::<&Position>(|_p| {});
}

#[test]
#[should_panic]
fn singleton_get_r_t_not_found() {
    #[derive(Component)]
    struct Tgt2;

    let world = World::new();
    // No pair set — must panic
    world.get::<&(Position, Tgt2)>(|_p| {});
}

#[test]
#[should_panic]
fn singleton_get_r_t_pair_types_not_found() {
    #[derive(Component)]
    struct Tgt3;

    let world = World::new();
    world.get::<&(Position, Tgt3)>(|_p| {});
}

#[test]
#[should_panic]
fn singleton_get_r_t_both_typed_not_found() {
    #[derive(Component)]
    struct TgtBoth;

    let world = World::new();
    world.get::<&(Position, TgtBoth)>(|_p| {});
}

// ── try_get variants (return None when not found) ────────────────────────────

#[test]
fn singleton_try_get_w_id() {
    let world = World::new();

    // Before set: None
    let result = world.try_get::<&Position>(|p| (p.x, p.y));
    assert!(result.is_none());

    world.set(Position { x: 10, y: 20 });

    // After set: Some
    let result = world.try_get::<&Position>(|p| (p.x, p.y));
    assert!(result.is_some());
    let (x, y) = result.unwrap();
    assert_eq!(x, 10);
    assert_eq!(y, 20);
}

#[test]
fn singleton_try_get_t() {
    let world = World::new();

    let result = world.try_get::<&Position>(|p| (p.x, p.y));
    assert!(result.is_none());

    world.set(Position { x: 10, y: 20 });

    let result = world.try_get::<&Position>(|p| (p.x, p.y));
    assert!(result.is_some());
    let (x, y) = result.unwrap();
    assert_eq!(x, 10);
    assert_eq!(y, 20);
}

#[test]
fn singleton_try_get_r_entity() {
    // C++: world.try_get_mut(id, tgt) then world.get_mut(id, tgt)
    // TODO: missing API: world.try_get_mut(id, entity_tgt) — raw untyped mutable singleton pair
    // Approximated: check has before/after set, then get via untyped pointer.
    let world = World::new();

    let tgt = world.entity();

    // Before set: pair does not exist
    let pos_entity = EntityView::new_from(&world, *world.component_id::<Position>());
    let ptr_before = pos_entity.get_first_untyped::<Position>(tgt);
    assert!(ptr_before.is_null());

    world.set_first::<Position>(tgt, Position { x: 10, y: 20 });

    let ptr_after = pos_entity.get_first_untyped::<Position>(tgt);
    assert!(!ptr_after.is_null());
    let p = unsafe { &*(ptr_after as *const Position) };
    assert_eq!(p.x, 10);
    assert_eq!(p.y, 20);
}

#[test]
fn singleton_try_get_r_t_typed() {
    // C++: world.try_get<Position>(tgt)
    // TODO: missing API: world.try_get::<Position>(entity_tgt) — typed singleton get with entity
    let world = World::new();

    let tgt = world.entity();

    let pos_entity = EntityView::new_from(&world, *world.component_id::<Position>());

    // Before set: null
    let ptr_before = pos_entity.get_first_untyped::<Position>(tgt);
    assert!(ptr_before.is_null());

    world.set_first::<Position>(tgt, Position { x: 10, y: 20 });

    // After set: non-null
    let ptr_after = pos_entity.get_first_untyped::<Position>(tgt);
    assert!(!ptr_after.is_null());
    let p = unsafe { &*(ptr_after as *const Position) };
    assert_eq!(p.x, 10);
    assert_eq!(p.y, 20);
}

#[test]
fn singleton_try_get_r_t_pair_types() {
    // C++: world.try_get<Position, Tgt>()
    #[derive(Component)]
    struct Tgt4;

    let world = World::new();

    let result = world.try_get::<&(Position, Tgt4)>(|p| (p.x, p.y));
    assert!(result.is_none());

    world.set_pair::<Position, Tgt4>(Position { x: 10, y: 20 });

    let result = world.try_get::<&(Position, Tgt4)>(|p| (p.x, p.y));
    assert!(result.is_some());
    let (x, y) = result.unwrap();
    assert_eq!(x, 10);
    assert_eq!(y, 20);
}

// ── get_mut variants ─────────────────────────────────────────────────────────
// C++ get_mut returns mutable ref and panics when missing.
// Rust: world.get::<&mut T>(cb) panics when missing.

#[test]
fn singleton_get_mut_w_id() {
    let world = World::new();

    world.set(Position { x: 10, y: 20 });

    world.get::<&mut Position>(|p| {
        assert_eq!(p.x, 10);
        assert_eq!(p.y, 20);
    });
}

#[test]
fn singleton_get_mut_t() {
    let world = World::new();

    world.set(Position { x: 10, y: 20 });

    world.get::<&mut Position>(|p| {
        assert_eq!(p.x, 10);
        assert_eq!(p.y, 20);
    });
}

#[test]
fn singleton_get_mut_r_entity() {
    // C++: Position* p = (Position*) world.get_mut(world.id<Position>(), tgt)
    // TODO: missing API: world.get_mut(id, entity_tgt) — raw untyped mutable singleton pair
    // Approximated with get_first_untyped_mut.
    let world = World::new();

    let tgt = world.entity();
    world.set_first::<Position>(tgt, Position { x: 10, y: 20 });

    let pos_entity = EntityView::new_from(&world, *world.component_id::<Position>());
    let ptr = pos_entity.get_first_untyped::<Position>(tgt);
    assert!(!ptr.is_null());
    let p = unsafe { &*(ptr as *const Position) };
    assert_eq!(p.x, 10);
    assert_eq!(p.y, 20);
}

#[test]
fn singleton_get_mut_r_t_typed() {
    // C++: Position& p = world.get_mut<Position>(tgt)
    // TODO: missing API: world.get_mut::<Position>(entity_tgt)
    let world = World::new();

    let tgt = world.entity();
    world.set_first::<Position>(tgt, Position { x: 10, y: 20 });

    let pos_entity = EntityView::new_from(&world, *world.component_id::<Position>());
    let ptr = pos_entity.get_first_untyped::<Position>(tgt);
    assert!(!ptr.is_null());
    let p = unsafe { &*(ptr as *const Position) };
    assert_eq!(p.x, 10);
    assert_eq!(p.y, 20);
}

#[test]
fn singleton_get_mut_r_t_pair_types() {
    // C++: Position& p = world.get_mut<Position, Tgt>()
    #[derive(Component)]
    struct Tgt5;

    let world = World::new();
    world.set_pair::<Position, Tgt5>(Position { x: 10, y: 20 });

    world.get::<&mut (Position, Tgt5)>(|p| {
        assert_eq!(p.x, 10);
        assert_eq!(p.y, 20);
    });
}

// ── get_mut "not found" — must panic ─────────────────────────────────────────

#[test]
#[should_panic]
fn singleton_get_mut_w_id_not_found() {
    let world = World::new();
    world.get::<&mut Position>(|_p| {});
}

#[test]
#[should_panic]
fn singleton_get_mut_t_not_found() {
    let world = World::new();
    world.get::<&mut Position>(|_p| {});
}

#[test]
#[should_panic]
fn singleton_get_mut_r_t_not_found() {
    #[derive(Component)]
    struct Tgt6;

    let world = World::new();
    world.get::<&mut (Position, Tgt6)>(|_p| {});
}

#[test]
#[should_panic]
fn singleton_get_mut_r_t_typed_not_found() {
    #[derive(Component)]
    struct Tgt7;

    let world = World::new();
    world.get::<&mut (Position, Tgt7)>(|_p| {});
}

#[test]
#[should_panic]
fn singleton_get_mut_r_t_pair_types_not_found() {
    #[derive(Component)]
    struct Tgt8;

    let world = World::new();
    world.get::<&mut (Position, Tgt8)>(|_p| {});
}

// ── try_get_mut variants (return None when not found) ────────────────────────

#[test]
fn singleton_try_get_mut_w_id() {
    let world = World::new();

    // Before set: None
    let result = world.try_get::<&mut Position>(|p| (p.x, p.y));
    assert!(result.is_none());

    world.set(Position { x: 10, y: 20 });

    // After set: Some
    let result = world.try_get::<&mut Position>(|p| (p.x, p.y));
    assert!(result.is_some());
    let (x, y) = result.unwrap();
    assert_eq!(x, 10);
    assert_eq!(y, 20);
}

#[test]
fn singleton_try_get_mut_t() {
    let world = World::new();

    let result = world.try_get::<&mut Position>(|p| (p.x, p.y));
    assert!(result.is_none());

    world.set(Position { x: 10, y: 20 });

    let result = world.try_get::<&mut Position>(|p| (p.x, p.y));
    assert!(result.is_some());
    let (x, y) = result.unwrap();
    assert_eq!(x, 10);
    assert_eq!(y, 20);
}

#[test]
fn singleton_try_get_mut_r_entity() {
    // C++: world.try_get_mut(id, tgt) / world.get_mut(id, tgt)
    // TODO: missing API: world.try_get_mut(id, entity_tgt) — raw untyped mutable pair with entity
    let world = World::new();

    let tgt = world.entity();

    let pos_entity = EntityView::new_from(&world, *world.component_id::<Position>());

    let ptr_before = pos_entity.get_first_untyped::<Position>(tgt);
    assert!(ptr_before.is_null());

    world.set_first::<Position>(tgt, Position { x: 10, y: 20 });

    let ptr_after = pos_entity.get_first_untyped::<Position>(tgt);
    assert!(!ptr_after.is_null());
    let p = unsafe { &*(ptr_after as *const Position) };
    assert_eq!(p.x, 10);
    assert_eq!(p.y, 20);
}

#[test]
fn singleton_try_get_mut_r_t_typed() {
    // C++: world.try_get_mut<Position>(tgt)
    // TODO: missing API: world.try_get_mut::<Position>(entity_tgt)
    let world = World::new();

    let tgt = world.entity();

    let pos_entity = EntityView::new_from(&world, *world.component_id::<Position>());

    let ptr_before = pos_entity.get_first_untyped::<Position>(tgt);
    assert!(ptr_before.is_null());

    world.set_first::<Position>(tgt, Position { x: 10, y: 20 });

    let ptr_after = pos_entity.get_first_untyped::<Position>(tgt);
    assert!(!ptr_after.is_null());
    let p = unsafe { &*(ptr_after as *const Position) };
    assert_eq!(p.x, 10);
    assert_eq!(p.y, 20);
}

#[test]
fn singleton_try_get_mut_r_t_pair_types() {
    // C++: world.try_get_mut<Position, Tgt>()
    #[derive(Component)]
    struct Tgt9;

    let world = World::new();

    let result = world.try_get::<&mut (Position, Tgt9)>(|p| (p.x, p.y));
    assert!(result.is_none());

    world.set_pair::<Position, Tgt9>(Position { x: 10, y: 20 });

    let result = world.try_get::<&mut (Position, Tgt9)>(|p| (p.x, p.y));
    assert!(result.is_some());
    let (x, y) = result.unwrap();
    assert_eq!(x, 10);
    assert_eq!(y, 20);
}
