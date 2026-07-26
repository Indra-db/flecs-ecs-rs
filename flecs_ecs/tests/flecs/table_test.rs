#![allow(dead_code)]
use crate::common_test::*;
use flecs_ecs::experimental::prelude::EntityGuardExt;

// Local enum for Table_get_T_enum test
#[repr(C)]
#[derive(Component, Debug, PartialEq, Clone, Copy)]
pub enum Number {
    One,
    Two,
    Three,
}

// Local relation for Table_has_pair_R_t / Table_has_pair_R_T
#[derive(Component)]
struct R;

#[derive(Component)]
struct T1;

#[derive(Component)]
struct T2;

#[derive(Component)]
struct T3;

// Local target for Table_get_pair tests
#[derive(Component)]
struct Tgt;

// -- Table_each ------------------------------------------------------------

/// Inside a `query.each()`, structural changes to a *different* entity's table
/// must succeed (the modified entity is not in the locked table).
#[test]
fn table_each() {
    let world = World::new();

    world.entity().set(Position { x: 0, y: 0 });
    let e2 = world.entity().set(Velocity { x: 0, y: 0 });

    world.new_query::<&Position>().each_entity(|_e, _p| {
        e2.add(Mass::id());
    });

    assert!(e2.has(Mass::id()));
}

/// Structural change to the *same* entity inside `query.each()` must panic
/// when safety locks are enabled (table is locked during iteration).
#[test]
#[cfg(feature = "flecs_safety_locks")]
#[cfg_attr(not(debug_assertions), ignore)]
fn table_each_locked() {
    let world = World::new();
    let _guard = FlecsPanicAbortGuard::install();
    let e1 = world.entity().set(Position { x: 0, y: 0 });

    let result = std::panic::catch_unwind(core::panic::AssertUnwindSafe(|| {
        world.new_query::<&Position>().each_entity(|_e, _p| {
            e1.add(Mass::id());
        });
    }));
    // Leak world to prevent Drop from aborting on partially-iterated locked state.
    core::mem::forget(world);
    assert!(
        result.is_err(),
        "expected panic when mutating entity during locked iteration"
    );
}

/// Same as `table_each` but using the no-entity callback form.
#[test]
fn table_each_without_entity() {
    let world = World::new();

    world.entity().set(Position { x: 0, y: 0 });
    let e2 = world.entity().set(Velocity { x: 0, y: 0 });

    world.new_query::<&Position>().each(|_p| {
        e2.add(Mass::id());
    });

    assert!(e2.has(Mass::id()));
}

/// Locked variant of `each_without_entity`.
#[test]
#[cfg(feature = "flecs_safety_locks")]
#[cfg_attr(not(debug_assertions), ignore)]
fn table_each_without_entity_locked() {
    let world = World::new();
    let _guard = FlecsPanicAbortGuard::install();
    let e1 = world.entity().set(Position { x: 0, y: 0 });

    let result = std::panic::catch_unwind(core::panic::AssertUnwindSafe(|| {
        world.new_query::<&Position>().each(|_p| {
            e1.add(Mass::id());
        });
    }));
    core::mem::forget(world);
    assert!(
        result.is_err(),
        "expected panic when mutating entity during locked each iteration"
    );
}

// -- Table_iter ------------------------------------------------------------

/// Structural changes to a different entity inside `query.run()` must succeed.
#[test]
fn table_iter() {
    let world = World::new();

    world.entity().set(Position { x: 0, y: 0 });
    let e2 = world.entity().set(Velocity { x: 0, y: 0 });

    world.new_query::<&Position>().run(|mut it| {
        while it.next() {
            e2.add(Mass::id());
        }
    });

    assert!(e2.has(Mass::id()));
}

/// Structural change to iterated entity inside `query.run()` must panic
/// when safety locks are enabled.
#[test]
#[cfg(feature = "flecs_safety_locks")]
#[cfg_attr(not(debug_assertions), ignore)]
fn table_iter_locked() {
    let world = World::new();
    let _guard = FlecsPanicAbortGuard::install();
    let e1 = world.entity().set(Position { x: 0, y: 0 });

    let result = std::panic::catch_unwind(core::panic::AssertUnwindSafe(|| {
        world.new_query::<&Position>().run(|mut it| {
            while it.next() {
                e1.add(Mass::id());
            }
        });
    }));
    core::mem::forget(world);
    assert!(
        result.is_err(),
        "expected panic when mutating entity during locked run iteration"
    );
}

/// `query.run()` without component terms — structural changes to another entity
/// must succeed (no table is locked without component access).
#[test]
fn table_iter_without_components() {
    let world = World::new();

    world.entity().set(Position { x: 0, y: 0 });
    let e2 = world.entity().set(Velocity { x: 0, y: 0 });

    world.new_query::<&Position>().run(|mut it| {
        while it.next() {
            e2.add(Mass::id());
        }
    });

    assert!(e2.has(Mass::id()));
}

/// Locked variant of `iter_without_components`.
#[test]
#[cfg(feature = "flecs_safety_locks")]
#[cfg_attr(not(debug_assertions), ignore)]
fn table_iter_without_components_locked() {
    let world = World::new();
    let _guard = FlecsPanicAbortGuard::install();
    let e1 = world.entity().set(Position { x: 0, y: 0 });

    let result = std::panic::catch_unwind(core::panic::AssertUnwindSafe(|| {
        world.new_query::<&Position>().run(|mut it| {
            while it.next() {
                e1.add(Mass::id());
            }
        });
    }));
    core::mem::forget(world);
    assert!(
        result.is_err(),
        "expected panic when mutating entity during locked run iteration without components"
    );
}

// -- Table_multi_get -------------------------------------------------------

/// `entity.get()` with multiple components — structural change to another entity inside.
#[test]
fn table_multi_get() {
    let world = World::new();

    let e1 = world
        .entity()
        .set(Position { x: 0, y: 0 })
        .set(Velocity { x: 0, y: 0 });
    let e2 = world.entity().set(Position { x: 0, y: 0 });

    let found = e1.get_ref::<(&Position, &Velocity)>().map(|(_p, _v)| {
        e2.add(Mass::id());
    });
    assert!(found.is_some());
    assert!(e2.has(Mass::id()));
}

/// Structural change inside `entity.get()` is deferred (safe) with safety locks.
/// The add is queued and applied after the callback returns.
#[test]
#[cfg(feature = "flecs_safety_locks")]
fn table_multi_get_locked() {
    let world = World::new();

    let e1 = world
        .entity()
        .set(Position { x: 0, y: 0 })
        .set(Velocity { x: 0, y: 0 });
    let e2 = world.entity().set(Position { x: 0, y: 0 });

    // A shared-register write issued while a guard is live is queued, not
    // immediate; it flushes when the guard drops at the end of the block.
    {
        let (_p, _v) = e1.get_ref::<(&Position, &Velocity)>().unwrap();
        e2.add(Velocity::id());
    }

    // After the guard drops, deferred commands are flushed — e2 should now have Velocity.
    assert!(e2.has(Velocity::id()));
}

// -- Table_multi_set -------------------------------------------------------

/// `entity.get::<(&mut, &mut)>()` with mutable refs — structural change to another entity.
#[test]
fn table_multi_set() {
    let world = World::new();

    let e1 = world
        .entity()
        .set(Position { x: 0, y: 0 })
        .set(Velocity { x: 0, y: 0 });
    let e2 = world.entity().set(Position { x: 0, y: 0 });

    {
        let (_p, _v) = e1.get_ref::<(&mut Position, &mut Velocity)>().unwrap();
        e2.add(Mass::id());
    }

    assert!(e2.has(Mass::id()));
}

/// Structural change inside mutable `entity.get()` is deferred (safe) with safety locks.
#[test]
#[cfg(feature = "flecs_safety_locks")]
fn table_multi_set_locked() {
    let world = World::new();

    let e1 = world
        .entity()
        .set(Position { x: 0, y: 0 })
        .set(Velocity { x: 0, y: 0 });
    let e2 = world.entity().set(Position { x: 0, y: 0 });

    // A shared-register write issued while a guard is live is queued, not
    // immediate; it flushes when the guard drops at the end of the block.
    {
        let (_p, _v) = e1.get_ref::<(&mut Position, &mut Velocity)>().unwrap();
        e2.add(Velocity::id());
    }

    // After the guard drops, deferred commands are flushed.
    assert!(e2.has(Velocity::id()));
}

// -- Table_count -----------------------------------------------------------

#[test]
fn table_count() {
    let world = World::new();

    let e = world.entity().set(Position { x: 10, y: 20 });
    world.entity().set(Position { x: 20, y: 30 });
    world.entity().set(Position { x: 30, y: 40 });

    let table = e.table().unwrap();
    assert_eq!(table.count(), 3);
}

// -- Table_has_id ----------------------------------------------------------

#[test]
fn table_has_id() {
    let world = World::new();

    let t1 = world.entity();
    let t2 = world.entity();
    let t3 = world.entity();

    let e = world.entity().add(t1).add(t2);
    world.entity().add(t1).add(t2);
    world.entity().add(t1).add(t2);

    let table = e.table().unwrap();
    assert!(table.has(t1));
    assert!(table.has(t2));
    assert!(!table.has(t3));
}

// -- Table_has_T -----------------------------------------------------------

#[test]
fn table_has_t() {
    let world = World::new();

    let e = world
        .entity()
        .set(Position { x: 10, y: 20 })
        .set(Velocity { x: 1, y: 2 });
    world
        .entity()
        .set(Position { x: 20, y: 30 })
        .set(Velocity { x: 2, y: 3 });
    world
        .entity()
        .set(Position { x: 30, y: 40 })
        .set(Velocity { x: 3, y: 4 });

    let table = e.table().unwrap();

    assert!(table.has(Position::id()));
    assert!(table.has(Velocity::id()));
    assert!(!table.has(Mass::id()));
}

// -- Table_has_pair_r_t ----------------------------------------------------

#[test]
fn table_has_pair_r_t() {
    let world = World::new();

    let r = world.entity();
    let t1 = world.entity();
    let t2 = world.entity();
    let t3 = world.entity();

    let e = world.entity().add((r, t1)).add((r, t2));
    world.entity().add((r, t1)).add((r, t2));
    world.entity().add((r, t1)).add((r, t2));

    let table = e.table().unwrap();
    assert!(table.has((r, t1)));
    assert!(table.has((r, t2)));
    assert!(!table.has((r, t3)));
}

// -- Table_has_pair_R_t ----------------------------------------------------

#[test]
fn table_has_pair_r_t_typed() {
    let world = World::new();

    let t1 = world.entity();
    let t2 = world.entity();
    let t3 = world.entity();

    let e = world.entity().add((R::id(), t1)).add((R::id(), t2));
    world.entity().add((R::id(), t1)).add((R::id(), t2));
    world.entity().add((R::id(), t1)).add((R::id(), t2));

    let table = e.table().unwrap();
    assert!(table.has((R::id(), t1)));
    assert!(table.has((R::id(), t2)));
    assert!(!table.has((R::id(), t3)));
}

// -- Table_has_pair_R_T ----------------------------------------------------

#[test]
fn table_has_pair_r_t_both_typed() {
    let world = World::new();

    let e = world
        .entity()
        .add((R::id(), T1::id()))
        .add((R::id(), T2::id()));
    world
        .entity()
        .add((R::id(), T1::id()))
        .add((R::id(), T2::id()));
    world
        .entity()
        .add((R::id(), T1::id()))
        .add((R::id(), T2::id()));

    let table = e.table().unwrap();
    assert!(table.has((R::id(), T1::id())));
    assert!(table.has((R::id(), T2::id())));
    assert!(!table.has((R::id(), T3::id())));
}

// -- Table_get_id ----------------------------------------------------------

/// Get component column pointer from table using component id, then read values.
#[test]
fn table_get_id() {
    let world = World::new();

    let e = world
        .entity()
        .set(Position { x: 10, y: 20 })
        .set(Velocity { x: 1, y: 2 });
    world
        .entity()
        .set(Position { x: 20, y: 30 })
        .set(Velocity { x: 2, y: 3 });
    world
        .entity()
        .set(Position { x: 30, y: 40 })
        .set(Velocity { x: 3, y: 4 });

    let table = e.table().unwrap();

    let p_ptr = table
        .get_mut_untyped(*world.component_id::<Position>())
        .unwrap() as *mut Position;
    unsafe {
        assert_eq!((*p_ptr.add(0)).x, 10);
        assert_eq!((*p_ptr.add(0)).y, 20);
        assert_eq!((*p_ptr.add(1)).x, 20);
        assert_eq!((*p_ptr.add(1)).y, 30);
        assert_eq!((*p_ptr.add(2)).x, 30);
        assert_eq!((*p_ptr.add(2)).y, 40);
    }

    let v_ptr = table
        .get_mut_untyped(*world.component_id::<Velocity>())
        .unwrap() as *mut Velocity;
    unsafe {
        assert_eq!((*v_ptr.add(0)).x, 1);
        assert_eq!((*v_ptr.add(0)).y, 2);
        assert_eq!((*v_ptr.add(1)).x, 2);
        assert_eq!((*v_ptr.add(1)).y, 3);
        assert_eq!((*v_ptr.add(2)).x, 3);
        assert_eq!((*v_ptr.add(2)).y, 4);
    }
}

// -- Table_get_T -----------------------------------------------------------

/// Same as `get_id` but using the generic typed accessor (`get_mut` returns &mut [T]).
#[test]
fn table_get_t() {
    let world = World::new();

    let e = world
        .entity()
        .set(Position { x: 10, y: 20 })
        .set(Velocity { x: 1, y: 2 });
    world
        .entity()
        .set(Position { x: 20, y: 30 })
        .set(Velocity { x: 2, y: 3 });
    world
        .entity()
        .set(Position { x: 30, y: 40 })
        .set(Velocity { x: 3, y: 4 });

    let mut table = e.table().unwrap();

    let p = table.get_mut::<Position>().unwrap();
    assert_eq!(p[0].x, 10);
    assert_eq!(p[0].y, 20);
    assert_eq!(p[1].x, 20);
    assert_eq!(p[1].y, 30);
    assert_eq!(p[2].x, 30);
    assert_eq!(p[2].y, 40);

    let v = table.get_mut::<Velocity>().unwrap();
    assert_eq!(v[0].x, 1);
    assert_eq!(v[0].y, 2);
    assert_eq!(v[1].x, 2);
    assert_eq!(v[1].y, 3);
    assert_eq!(v[2].x, 3);
    assert_eq!(v[2].y, 4);
}

// -- Table_get_pair_r_t ----------------------------------------------------

/// Get pair component column using two raw entity ids.
#[test]
fn table_get_pair_r_t() {
    let world = World::new();

    let e = world
        .entity()
        .set_pair::<Position, Tgt>(Position { x: 10, y: 20 })
        .set_pair::<Velocity, Tgt>(Velocity { x: 1, y: 2 });
    world
        .entity()
        .set_pair::<Position, Tgt>(Position { x: 20, y: 30 })
        .set_pair::<Velocity, Tgt>(Velocity { x: 2, y: 3 });
    world
        .entity()
        .set_pair::<Position, Tgt>(Position { x: 30, y: 40 })
        .set_pair::<Velocity, Tgt>(Velocity { x: 3, y: 4 });

    let table = e.table().unwrap();

    let p = table.get_pair_ids_mut_untyped(
        world.component_id::<Position>(),
        world.component_id::<Tgt>(),
    );
    assert!(p.is_some());
    let p_ptr = p.unwrap() as *mut Position;
    unsafe {
        assert_eq!((*p_ptr.add(0)).x, 10);
        assert_eq!((*p_ptr.add(0)).y, 20);
        assert_eq!((*p_ptr.add(1)).x, 20);
        assert_eq!((*p_ptr.add(1)).y, 30);
        assert_eq!((*p_ptr.add(2)).x, 30);
        assert_eq!((*p_ptr.add(2)).y, 40);
    }

    let v = table.get_pair_ids_mut_untyped(
        world.component_id::<Velocity>(),
        world.component_id::<Tgt>(),
    );
    assert!(v.is_some());
    let v_ptr = v.unwrap() as *mut Velocity;
    unsafe {
        assert_eq!((*v_ptr.add(0)).x, 1);
        assert_eq!((*v_ptr.add(0)).y, 2);
        assert_eq!((*v_ptr.add(1)).x, 2);
        assert_eq!((*v_ptr.add(1)).y, 3);
        assert_eq!((*v_ptr.add(2)).x, 3);
        assert_eq!((*v_ptr.add(2)).y, 4);
    }
}

// -- Table_get_pair_R_t ----------------------------------------------------

/// Get pair column using typed first component and entity-id second.
#[test]
fn table_get_pair_r_t_typed_first() {
    let world = World::new();

    let e = world
        .entity()
        .set_pair::<Position, Tgt>(Position { x: 10, y: 20 })
        .set_pair::<Velocity, Tgt>(Velocity { x: 1, y: 2 });
    world
        .entity()
        .set_pair::<Position, Tgt>(Position { x: 20, y: 30 })
        .set_pair::<Velocity, Tgt>(Velocity { x: 2, y: 3 });
    world
        .entity()
        .set_pair::<Position, Tgt>(Position { x: 30, y: 40 })
        .set_pair::<Velocity, Tgt>(Velocity { x: 3, y: 4 });

    let table = e.table().unwrap();

    let p = table.get_pair_ids_mut_untyped(
        world.component_id::<Position>(),
        world.component_id::<Tgt>(),
    );
    assert!(p.is_some());
    let p_ptr = p.unwrap() as *mut Position;
    unsafe {
        assert_eq!((*p_ptr.add(0)).x, 10);
        assert_eq!((*p_ptr.add(0)).y, 20);
        assert_eq!((*p_ptr.add(1)).x, 20);
        assert_eq!((*p_ptr.add(1)).y, 30);
        assert_eq!((*p_ptr.add(2)).x, 30);
        assert_eq!((*p_ptr.add(2)).y, 40);
    }

    let v = table.get_pair_ids_mut_untyped(
        world.component_id::<Velocity>(),
        world.component_id::<Tgt>(),
    );
    assert!(v.is_some());
    let v_ptr = v.unwrap() as *mut Velocity;
    unsafe {
        assert_eq!((*v_ptr.add(0)).x, 1);
        assert_eq!((*v_ptr.add(0)).y, 2);
        assert_eq!((*v_ptr.add(1)).x, 2);
        assert_eq!((*v_ptr.add(1)).y, 3);
        assert_eq!((*v_ptr.add(2)).x, 3);
        assert_eq!((*v_ptr.add(2)).y, 4);
    }
}

// -- Table_get_pair_R_T ----------------------------------------------------

/// Get pair column using both typed first and second components.
#[test]
fn table_get_pair_r_t_both_typed_get() {
    let world = World::new();

    let e = world
        .entity()
        .set_pair::<Position, Tgt>(Position { x: 10, y: 20 })
        .set_pair::<Velocity, Tgt>(Velocity { x: 1, y: 2 });
    world
        .entity()
        .set_pair::<Position, Tgt>(Position { x: 20, y: 30 })
        .set_pair::<Velocity, Tgt>(Velocity { x: 2, y: 3 });
    world
        .entity()
        .set_pair::<Position, Tgt>(Position { x: 30, y: 40 })
        .set_pair::<Velocity, Tgt>(Velocity { x: 3, y: 4 });

    let table = e.table().unwrap();

    let p = table.get_pair_mut_untyped::<Position, Tgt>();
    assert!(p.is_some());
    let p_ptr = p.unwrap() as *mut Position;
    unsafe {
        assert_eq!((*p_ptr.add(0)).x, 10);
        assert_eq!((*p_ptr.add(0)).y, 20);
        assert_eq!((*p_ptr.add(1)).x, 20);
        assert_eq!((*p_ptr.add(1)).y, 30);
        assert_eq!((*p_ptr.add(2)).x, 30);
        assert_eq!((*p_ptr.add(2)).y, 40);
    }

    let v = table.get_pair_mut_untyped::<Velocity, Tgt>();
    assert!(v.is_some());
    let v_ptr = v.unwrap() as *mut Velocity;
    unsafe {
        assert_eq!((*v_ptr.add(0)).x, 1);
        assert_eq!((*v_ptr.add(0)).y, 2);
        assert_eq!((*v_ptr.add(1)).x, 2);
        assert_eq!((*v_ptr.add(1)).y, 3);
        assert_eq!((*v_ptr.add(2)).x, 3);
        assert_eq!((*v_ptr.add(2)).y, 4);
    }
}

// -- Table_range_get_id ----------------------------------------------------

/// Get component column from a table range (single entity range).
#[test]
fn table_range_get_id() {
    let world = World::new();

    world
        .entity()
        .set(Position { x: 10, y: 20 })
        .set(Velocity { x: 1, y: 2 });
    let e = world
        .entity()
        .set(Position { x: 20, y: 30 })
        .set(Velocity { x: 2, y: 3 });
    world
        .entity()
        .set(Position { x: 30, y: 40 })
        .set(Velocity { x: 3, y: 4 });

    let range = e.range().unwrap();

    let p_ptr = range
        .get_mut_untyped(*world.component_id::<Position>())
        .unwrap() as *mut Position;
    unsafe {
        assert_eq!((*p_ptr).x, 20);
        assert_eq!((*p_ptr).y, 30);
    }

    let v_ptr = range
        .get_mut_untyped(*world.component_id::<Velocity>())
        .unwrap() as *mut Velocity;
    unsafe {
        assert_eq!((*v_ptr).x, 2);
        assert_eq!((*v_ptr).y, 3);
    }
}

// -- Table_range_get_T -----------------------------------------------------

/// Same as `range_get_id` but using the typed accessor.
#[test]
fn table_range_get_t() {
    let world = World::new();

    world
        .entity()
        .set(Position { x: 10, y: 20 })
        .set(Velocity { x: 1, y: 2 });
    let e = world
        .entity()
        .set(Position { x: 20, y: 30 })
        .set(Velocity { x: 2, y: 3 });
    world
        .entity()
        .set(Position { x: 30, y: 40 })
        .set(Velocity { x: 3, y: 4 });

    let mut range = e.range().unwrap();

    let p = range.get_mut::<Position>().unwrap();
    assert_eq!(p[0].x, 20);
    assert_eq!(p[0].y, 30);

    let v = range.get_mut::<Velocity>().unwrap();
    assert_eq!(v[0].x, 2);
    assert_eq!(v[0].y, 3);
}

// -- Table_range_get_pair_r_t ----------------------------------------------

#[test]
fn table_range_get_pair_r_t() {
    let world = World::new();

    world
        .entity()
        .set_pair::<Position, Tgt>(Position { x: 10, y: 20 })
        .set_pair::<Velocity, Tgt>(Velocity { x: 1, y: 2 });
    let e = world
        .entity()
        .set_pair::<Position, Tgt>(Position { x: 20, y: 30 })
        .set_pair::<Velocity, Tgt>(Velocity { x: 2, y: 3 });
    world
        .entity()
        .set_pair::<Position, Tgt>(Position { x: 30, y: 40 })
        .set_pair::<Velocity, Tgt>(Velocity { x: 3, y: 4 });

    let range = e.range().unwrap();

    let p = range.get_pair_ids_mut_untyped(
        world.component_id::<Position>(),
        world.component_id::<Tgt>(),
    );
    assert!(p.is_some());
    let p_ptr = p.unwrap() as *mut Position;
    unsafe {
        assert_eq!((*p_ptr).x, 20);
        assert_eq!((*p_ptr).y, 30);
    }

    let v = range.get_pair_ids_mut_untyped(
        world.component_id::<Velocity>(),
        world.component_id::<Tgt>(),
    );
    assert!(v.is_some());
    let v_ptr = v.unwrap() as *mut Velocity;
    unsafe {
        assert_eq!((*v_ptr).x, 2);
        assert_eq!((*v_ptr).y, 3);
    }
}

// -- Table_range_get_pair_R_t ----------------------------------------------

#[test]
fn table_range_get_pair_r_t_typed_first() {
    let world = World::new();

    world
        .entity()
        .set_pair::<Position, Tgt>(Position { x: 10, y: 20 })
        .set_pair::<Velocity, Tgt>(Velocity { x: 1, y: 2 });
    let e = world
        .entity()
        .set_pair::<Position, Tgt>(Position { x: 20, y: 30 })
        .set_pair::<Velocity, Tgt>(Velocity { x: 2, y: 3 });
    world
        .entity()
        .set_pair::<Position, Tgt>(Position { x: 30, y: 40 })
        .set_pair::<Velocity, Tgt>(Velocity { x: 3, y: 4 });

    let range = e.range().unwrap();

    let p = range.get_pair_ids_mut_untyped(
        world.component_id::<Position>(),
        world.component_id::<Tgt>(),
    );
    assert!(p.is_some());
    let p_ptr = p.unwrap() as *mut Position;
    unsafe {
        assert_eq!((*p_ptr).x, 20);
        assert_eq!((*p_ptr).y, 30);
    }

    let v = range.get_pair_ids_mut_untyped(
        world.component_id::<Velocity>(),
        world.component_id::<Tgt>(),
    );
    assert!(v.is_some());
    let v_ptr = v.unwrap() as *mut Velocity;
    unsafe {
        assert_eq!((*v_ptr).x, 2);
        assert_eq!((*v_ptr).y, 3);
    }
}

// -- Table_range_get_pair_R_T ----------------------------------------------

#[test]
fn table_range_get_pair_r_t_both_typed() {
    let world = World::new();

    world
        .entity()
        .set_pair::<Position, Tgt>(Position { x: 10, y: 20 })
        .set_pair::<Velocity, Tgt>(Velocity { x: 1, y: 2 });
    let e = world
        .entity()
        .set_pair::<Position, Tgt>(Position { x: 20, y: 30 })
        .set_pair::<Velocity, Tgt>(Velocity { x: 2, y: 3 });
    world
        .entity()
        .set_pair::<Position, Tgt>(Position { x: 30, y: 40 })
        .set_pair::<Velocity, Tgt>(Velocity { x: 3, y: 4 });

    let range = e.range().unwrap();

    let p = range.get_pair_mut_untyped::<Position, Tgt>();
    assert!(p.is_some());
    let p_ptr = p.unwrap() as *mut Position;
    unsafe {
        assert_eq!((*p_ptr).x, 20);
        assert_eq!((*p_ptr).y, 30);
    }

    let v = range.get_pair_mut_untyped::<Velocity, Tgt>();
    assert!(v.is_some());
    let v_ptr = v.unwrap() as *mut Velocity;
    unsafe {
        assert_eq!((*v_ptr).x, 2);
        assert_eq!((*v_ptr).y, 3);
    }
}

// -- Table_get_depth -------------------------------------------------------

#[test]
fn table_get_depth() {
    let world = World::new();

    let e1 = world.entity();
    let e2 = world.entity().child_of(e1);
    let e3 = world.entity().child_of(e2);
    let e4 = world.entity().child_of(e3);

    assert_eq!(e2.table().unwrap().depth(*flecs::ChildOf), 1);
    assert_eq!(e3.table().unwrap().depth(*flecs::ChildOf), 2);
    assert_eq!(e4.table().unwrap().depth(*flecs::ChildOf), 3);
}

// -- Table_get_depth_w_type ------------------------------------------------

#[test]
fn table_get_depth_w_type() {
    let world = World::new();

    #[derive(Component)]
    struct Rel2;

    world.component::<Rel2>().add(*flecs::Traversable);

    let e1 = world.entity();
    let e2 = world.entity().add((Rel2::id(), e1));
    let e3 = world.entity().add((Rel2::id(), e2));
    let e4 = world.entity().add((Rel2::id(), e3));

    assert_eq!(e2.table().unwrap().depth(Rel2::id()), 1);
    assert_eq!(e3.table().unwrap().depth(Rel2::id()), 2);
    assert_eq!(e4.table().unwrap().depth(Rel2::id()), 3);
}

// -- Table_iter_type -------------------------------------------------------

/// Iterate the archetype of a table and verify it contains exactly the two
/// components we added.
#[test]
fn table_iter_type() {
    let world = World::new();

    let e = world.entity().add(Position::id()).add(Velocity::id());

    let table = e.table().unwrap();
    let arch = table.archetype();
    let ids = arch.as_slice();

    let mut count = 0;
    for &id in ids {
        count += 1;
        assert!(id == *world.component_id::<Position>() || id == *world.component_id::<Velocity>());
    }
    assert_eq!(count, 2);
}

// -- Table_get_T_enum ------------------------------------------------------

#[test]
fn table_get_t_enum() {
    let world = World::new();

    let e = world.entity().set(Number::One);
    world.entity().set(Number::Two);
    world.entity().set(Number::Three);

    let mut table = e.table().unwrap();

    let n = table.get_mut::<Number>().unwrap();
    assert_eq!(n[0], Number::One);
    assert_eq!(n[1], Number::Two);
    assert_eq!(n[2], Number::Three);
}

// -- Table_get_size --------------------------------------------------------

#[test]
fn table_get_size() {
    let world = World::new();

    let e = world.entity().set(Position { x: 10, y: 20 });
    world.entity().set(Position { x: 20, y: 30 });
    world.entity().set(Position { x: 30, y: 40 });

    let table = e.table().unwrap();
    assert_eq!(table.count(), 3);
    assert_eq!(table.size(), 4);
}

// -- Table_get_entities ----------------------------------------------------

#[test]
fn table_get_entities() {
    let world = World::new();

    let e1 = world.entity().set(Position { x: 10, y: 20 });
    let e2 = world.entity().set(Position { x: 20, y: 30 });
    let e3 = world.entity().set(Position { x: 30, y: 40 });

    let table = e1.table().unwrap();
    let entities = table.entities();
    assert!(!entities.is_empty());
    assert_eq!(entities[0], e1.id());
    assert_eq!(entities[1], e2.id());
    assert_eq!(entities[2], e3.id());
}

// -- Table_get_records -----------------------------------------------------

#[test]
fn table_get_records() {
    let world = World::new();
    let parent = world.entity();
    let e = world
        .entity()
        .child_of(parent)
        .set(Position { x: 10, y: 20 });

    let table = e.table().unwrap();
    let records = table.records();
    assert_eq!(records.len(), 6);
}

// -- Table_lock ------------------------------------------------------------

/// Explicitly locking a table prevents adding components that would move entities
/// into a different table.
#[test]
#[cfg(feature = "flecs_safety_locks")]
#[cfg_attr(not(debug_assertions), ignore)]
fn table_lock() {
    // Use catch_unwind instead of #[should_panic] so we can unlock the table
    // before dropping the world — otherwise the world destructor aborts on the
    // still-locked table.
    let world = World::new();
    let _guard = FlecsPanicAbortGuard::install();

    let e1 = world.entity().set(Position { x: 10, y: 20 });
    let e2 = world.entity();

    let table = e1.table().unwrap();
    table.lock();

    // Adding Position to e2 tries to move e2 into the Position table (locked) — panics.
    let result = std::panic::catch_unwind(core::panic::AssertUnwindSafe(|| {
        e2.set(Position { x: 20, y: 30 });
    }));

    // Unlock the table, then forget the world to prevent ecs_fini from
    // hitting an assert on the partially-aborted entity state.
    table.unlock();
    core::mem::forget(world);

    assert!(
        result.is_err(),
        "expected panic when setting component on locked table"
    );
}

// -- Table_unlock ----------------------------------------------------------

/// After unlock the table accepts structural changes again.
#[test]
fn table_unlock() {
    let world = World::new();

    let e1 = world.entity().set(Position { x: 10, y: 20 });
    let e2 = world.entity();

    let table = e1.table().unwrap();
    table.lock();
    table.unlock();

    e2.set(Position { x: 20, y: 30 });
    assert!(e2.has(Position::id()));
}

// -- Table_has_flags -------------------------------------------------------

#[test]
fn table_has_flags() {
    let world = World::new();

    let parent = world.entity();
    let e1 = world
        .entity()
        .child_of(parent)
        .set(Position { x: 10, y: 20 });

    let table = e1.table().unwrap();

    assert!(!table.has_flags(TableFlags::IsComplex));
    assert!(table.has_flags(TableFlags::HasPairs));
    assert!(table.has_flags(TableFlags::HasChildOf));
    assert!(!table.has_flags(TableFlags::HasIsA));
}

// -- Table_clear_entities --------------------------------------------------

#[test]
fn table_clear_entities() {
    let world = World::new();

    let e1 = world.entity().set(Position { x: 10, y: 20 });
    let e2 = world.entity().set(Position { x: 30, y: 40 });

    assert!(e1.is_alive());
    assert!(e2.is_alive());

    let table = e1.table().unwrap();
    assert_eq!(table.count(), 2);
    assert_eq!(table.size(), 2);

    table.clear_entities();

    assert!(!e1.is_alive());
    assert!(!e2.is_alive());

    assert_eq!(table.count(), 0);
    assert_eq!(table.size(), 2);
}

// -- Regression tests for table access soundness -----------------------------

/// Two independently obtained handles to the same table must not be able to
/// hand out two live mutable guards for the same column.
#[test]
#[cfg(feature = "flecs_safety_locks")]
fn table_get_mut_aliasing_panics() {
    let world = World::new();

    let e = world.entity().set(Position { x: 1, y: 2 });

    let mut t1 = e.table().unwrap();
    let mut t2 = e.table().unwrap();

    let guard = t1.get_mut::<Position>().unwrap();

    let result = std::panic::catch_unwind(core::panic::AssertUnwindSafe(|| {
        let _second = t2.get_mut::<Position>();
    }));
    assert!(result.is_err());

    assert_eq!(guard[0].x, 1);
    assert_eq!(guard[0].y, 2);
}

/// While an `entities()` guard is alive, structural changes are deferred, so the
/// entity array cannot be reallocated under the borrow.
#[test]
fn table_entities_guard_defers_structural_changes() {
    let world = World::new();

    let e1 = world.entity().set(Position { x: 10, y: 20 });

    {
        let table = e1.table().unwrap();
        let entities = table.entities();

        for _ in 0..64 {
            world.entity().set(Position { x: 1, y: 1 });
        }

        assert_eq!(entities.len(), 1);
        assert_eq!(entities[0], e1.id());
    }

    assert_eq!(e1.table().unwrap().count(), 65);
}

/// While a `get_mut()` guard is alive, structural changes are deferred, so the
/// column storage cannot be reallocated under the borrow.
#[test]
fn table_get_mut_guard_defers_structural_changes() {
    let world = World::new();

    let e1 = world.entity().set(Position { x: 10, y: 20 });

    {
        let mut table = e1.table().unwrap();
        let pos = table.get_mut::<Position>().unwrap();

        for _ in 0..64 {
            world.entity().set(Position { x: 1, y: 1 });
        }

        assert_eq!(pos.len(), 1);
        assert_eq!(pos[0].x, 10);
        assert_eq!(pos[0].y, 20);
    }

    assert_eq!(e1.table().unwrap().count(), 65);
}

/// `entities()` on a table range must honor the range offset.
#[test]
fn table_range_entities_offset() {
    let world = World::new();

    let e1 = world.entity().set(Position { x: 10, y: 20 });
    let e2 = world.entity().set(Position { x: 20, y: 30 });
    let e3 = world.entity().set(Position { x: 30, y: 40 });

    let range = e2.range().unwrap();
    let entities = range.entities();
    assert_eq!(entities.len(), 1);
    assert_eq!(entities[0], e2.id());
    drop(entities);

    let range = TableRange::new(e1.table().unwrap(), 1, 2);
    let entities = range.entities();
    assert_eq!(entities.len(), 2);
    assert_eq!(entities[0], e2.id());
    assert_eq!(entities[1], e3.id());
}

/// A table lock must be released when a panic unwinds through it and is later
/// caught, so the table can still be modified afterwards.
#[test]
fn table_lock_released_after_caught_panic() {
    let world = World::new();

    let e = world.entity().set(Position { x: 1, y: 2 });

    let result = std::panic::catch_unwind(core::panic::AssertUnwindSafe(|| {
        let table = e.table().unwrap();
        let _archetype = table.archetype();
        panic!("panic while table lock is held");
    }));
    assert!(result.is_err());

    e.set(Velocity { x: 3, y: 4 });
    assert!(e.has(Position::id()));
    assert!(e.has(Velocity::id()));
}
