#![allow(dead_code)]
use crate::common_test::*;
use flecs_ecs::experimental::{QueryExclusiveExt, QuerySharedExt};
use flecs_ecs::experimental::prelude::EntityGuardExt;

#[test]
fn term_each_component() {
    let world = World::new();

    let e_1 = world.entity().set(Position { x: 1, y: 2 });
    let e_2 = world.entity().set(Position { x: 3, y: 4 });
    let e_3 = world.entity().set(Position { x: 5, y: 6 });

    e_3.add(Tag::id());

    let mut count = 0;
    world.each_entity::<&Position>(|e, p| {
        if e == e_1 {
            assert_eq!(p.x, 1);
            assert_eq!(p.y, 2);
            count += 1;
        }
        if e == e_2 {
            assert_eq!(p.x, 3);
            assert_eq!(p.y, 4);
            count += 1;
        }
        if e == e_3 {
            assert_eq!(p.x, 5);
            assert_eq!(p.y, 6);
            count += 1;
        }
    });

    assert_eq!(count, 3);
}

#[test]
fn term_each_tag() {
    let world = World::new();

    let e_1 = world.entity().add(Foo::id());
    let e_2 = world.entity().add(Foo::id());
    let e_3 = world.entity().add(Foo::id());

    e_3.add(Tag::id());

    let mut count = 0;
    world
        .query::<()>()
        .with(Foo::id())
        .build()
        .each_entity_shared(&world, |e, ()| {
            if e == e_1 || e == e_2 || e == e_3 {
                count += 1;
            }
        });

    assert_eq!(count, 3);
}

#[test]
fn term_each_id() {
    let world = World::new();

    let foo = world.entity();

    let e_1 = world.entity().add(foo.id());
    let e_2 = world.entity().add(foo.id());
    let e_3 = world.entity().add(foo.id());

    e_3.add(Tag::id());

    let mut count = 0;
    world
        .query::<()>()
        .with(foo.id())
        .build()
        .each_entity_shared(&world, |e, ()| {
            if e == e_1 || e == e_2 || e == e_3 {
                count += 1;
            }
        });

    assert_eq!(count, 3);
}

#[test]
fn term_each_pair_type() {
    #[derive(Component)]
    struct PairRel;
    #[derive(Component)]
    struct PairObj;

    let world = World::new();

    let e_1 = world.entity().add((PairRel::id(), PairObj::id()));
    let e_2 = world.entity().add((PairRel::id(), PairObj::id()));
    let e_3 = world.entity().add((PairRel::id(), PairObj::id()));

    e_3.add(Tag::id());

    let mut count = 0;
    world
        .query::<()>()
        .with((PairRel::id(), PairObj::id()))
        .build()
        .each_entity_shared(&world, |e, ()| {
            if e == e_1 || e == e_2 || e == e_3 {
                count += 1;
            }
        });

    assert_eq!(count, 3);
}

#[test]
fn term_each_pair_id() {
    let world = World::new();

    let rel = world.entity();
    let obj = world.entity();

    let e_1 = world.entity().add((rel.id(), obj.id()));
    let e_2 = world.entity().add((rel.id(), obj.id()));
    let e_3 = world.entity().add((rel.id(), obj.id()));

    e_3.add(Tag::id());

    let mut count = 0;
    world
        .query::<()>()
        .with((rel.id(), obj.id()))
        .build()
        .each_entity_shared(&world, |e, ()| {
            if e == e_1 || e == e_2 || e == e_3 {
                count += 1;
            }
        });

    assert_eq!(count, 3);
}

#[test]
fn term_each_pair_relation_wildcard() {
    let world = World::new();

    let rel_1 = world.entity();
    let rel_2 = world.entity();
    let obj = world.entity();

    let e_1 = world.entity().add((rel_1.id(), obj.id()));
    let e_2 = world.entity().add((rel_1.id(), obj.id()));
    let e_3 = world.entity().add((rel_2.id(), obj.id()));

    e_3.add(Tag::id());

    let mut count = 0;
    world
        .query::<()>()
        .with((*flecs::Wildcard, obj.id()))
        .build()
        .each_entity_shared(&world, |e, ()| {
            if e == e_1 || e == e_2 || e == e_3 {
                count += 1;
            }
        });

    assert_eq!(count, 3);
}

#[test]
fn term_each_pair_object_wildcard() {
    let world = World::new();

    let rel = world.entity();
    let obj_1 = world.entity();
    let obj_2 = world.entity();

    let e_1 = world.entity().add((rel.id(), obj_1.id()));
    let e_2 = world.entity().add((rel.id(), obj_1.id()));
    let e_3 = world.entity().add((rel.id(), obj_2.id()));

    e_3.add(Tag::id());

    let mut count = 0;
    world
        .query::<()>()
        .with((rel.id(), *flecs::Wildcard))
        .build()
        .each_entity_shared(&world, |e, ()| {
            if e == e_1 || e == e_2 || e == e_3 {
                count += 1;
            }
        });

    assert_eq!(count, 3);
}

// In Rust there is no "default constructor" for Query - use Option<Query<T>> instead.
// This test just verifies that pattern compiles.
#[test]
fn default_ctor_no_assign() {
    let _world = World::new();
    let _q: Option<Query<(&Position,)>> = None;
}

#[test]
fn term_get_id() {
    let world = World::new();

    let foo = world.entity();
    let bar = world.entity();

    let q = world
        .query::<()>()
        .with(Position::id())
        .with(Velocity::id())
        .with((foo, bar))
        .build();

    assert_eq!(q.field_count(), 3);

    assert_eq!(
        world.id_view_from(q.term(0).id()),
        world.id_view_from(Position::id())
    );
    assert_eq!(
        world.id_view_from(q.term(1).id()),
        world.id_view_from(Velocity::id())
    );
    assert_eq!(
        world.id_view_from(q.term(2).id()),
        world.id_view_from((foo.id(), bar.id()))
    );
}

#[test]
fn term_get_subj() {
    let world = World::new();

    let foo = world.entity();
    let bar = world.entity();
    let src = world.entity();

    let q = world
        .query::<()>()
        .with(Position::id())
        .with(Velocity::id())
        .src()
        .entity(src)
        .with((foo, bar))
        .build();

    assert_eq!(q.field_count(), 3);

    assert_eq!(*q.term(0).src_id(), flecs::This_::ID);
    assert_eq!(*q.term(1).src_id(), *src.id());
    assert_eq!(*q.term(2).src_id(), flecs::This_::ID);
}

#[test]
fn term_get_pred() {
    let world = World::new();

    let foo = world.entity();
    let bar = world.entity();
    let src = world.entity();

    let q = world
        .query::<()>()
        .with(Position::id())
        .with(Velocity::id())
        .src()
        .entity(src)
        .with((foo, bar))
        .build();

    assert_eq!(q.field_count(), 3);

    assert_eq!(*q.term(0).first_id(), *world.component_id::<Position>());
    assert_eq!(*q.term(1).first_id(), *world.component_id::<Velocity>());
    assert_eq!(*q.term(2).first_id(), *foo.id());
}

#[test]
fn term_get_obj() {
    let world = World::new();

    let foo = world.entity();
    let bar = world.entity();

    let q = world
        .query::<()>()
        .with(Position::id())
        .with(Velocity::id())
        .with((foo, bar))
        .build();

    assert_eq!(q.field_count(), 3);

    assert_eq!(*q.term(0).second_id(), 0u64);
    assert_eq!(*q.term(1).second_id(), 0u64);
    assert_eq!(*q.term(2).second_id(), *bar.id());
}

#[test]
fn get_first() {
    let world = World::new();

    let e1 = world.entity().set(Position { x: 1, y: 2 });
    let _e2 = world.entity().set(Position { x: 3, y: 4 });
    let _e3 = world.entity().set(Position { x: 5, y: 6 });

    let q = world.new_query::<&Position>();
    let first = q.first_entity();
    assert_ne!(*first.id(), 0u64);
    assert_eq!(first.id(), e1.id());
}

#[test]
fn get_count_direct() {
    let world = World::new();

    let _e1 = world.entity().set(Position { x: 1, y: 2 });
    let _e2 = world.entity().set(Position { x: 3, y: 4 });
    let _e3 = world.entity().set(Position { x: 5, y: 6 });

    let q = world.new_query::<&Position>();
    assert_eq!(q.count(), 3);
}

#[test]
fn get_is_true_direct() {
    let world = World::new();

    let _e1 = world.entity().set(Position { x: 1, y: 2 });
    let _e2 = world.entity().set(Position { x: 3, y: 4 });
    let _e3 = world.entity().set(Position { x: 5, y: 6 });

    let q_1 = world.new_query::<&Position>();
    let q_2 = world.new_query::<&Velocity>();

    assert!(q_1.is_true());
    assert!(!q_2.is_true());
}

#[test]
fn get_first_direct() {
    let world = World::new();

    let e1 = world.entity().set(Position { x: 1, y: 2 });
    let _e2 = world.entity().set(Position { x: 3, y: 4 });
    let _e3 = world.entity().set(Position { x: 5, y: 6 });

    let q = world.new_query::<&Position>();
    let first = q.first_entity();
    assert_ne!(*first.id(), 0u64);
    assert_eq!(first.id(), e1.id());
}

#[test]
fn each_w_no_this() {
    let mut world = World::new();

    let e = world
        .entity()
        .set(Position { x: 10, y: 20 })
        .set(Velocity { x: 1, y: 2 });

    let q = world
        .query::<(&Position, &Velocity)>()
        .term_at(0)
        .src()
        .entity(e)
        .term_at(1)
        .src()
        .entity(e)
        .build();

    let mut count = 0;
    q.each_exclusive(&mut world, |(p, v)| {
        count += 1;
        assert_eq!(p.x, 10);
        assert_eq!(p.y, 20);
        assert_eq!(v.x, 1);
        assert_eq!(v.y, 2);
    });

    assert_eq!(count, 1);
}

#[test]
fn each_w_iter_no_this() {
    let world = World::new();

    let e = world
        .entity()
        .set(Position { x: 10, y: 20 })
        .set(Velocity { x: 1, y: 2 });

    let q = world
        .query::<(&Position, &Velocity)>()
        .term_at(0)
        .src()
        .entity(e)
        .term_at(1)
        .src()
        .entity(e)
        .build();

    let mut count = 0;
    q.each_iter(|it, index, (p, v)| {
        count += 1;
        assert_eq!(p.x, 10);
        assert_eq!(p.y, 20);
        assert_eq!(v.x, 1);
        assert_eq!(v.y, 2);
        assert_eq!(index, FieldIndex::from(0usize));
        assert_eq!(it.count(), 0);
    });

    assert_eq!(count, 1);
}

#[test]
#[should_panic]
#[cfg_attr(not(debug_assertions), ignore)]
fn invalid_each_w_no_this() {
    let mut world = World::new();
    let _guard = FlecsPanicAbortGuard::install();

    let e = world
        .entity()
        .set(Position { x: 10, y: 20 })
        .set(Velocity { x: 1, y: 2 });

    let q = world
        .query::<(&Position, &Velocity)>()
        .term_at(0)
        .src()
        .entity(e)
        .term_at(1)
        .src()
        .entity(e)
        .build();

    q.each_entity_exclusive(&mut world, |_e, (_p, _v)| {});
}

#[test]
fn named_query() {
    let world = World::new();

    let e1 = world.entity().add(Position::id());
    let e2 = world.entity().add(Position::id());

    let q = world.new_query_named::<&Position>("my_query");

    let mut count = 0;
    q.each_entity_shared(&world, |e, _pos| {
        assert!(e.id() == e1.id() || e.id() == e2.id());
        count += 1;
    });
    assert_eq!(count, 2);

    let qe = q.entity();
    assert_ne!(*qe.id(), 0u64);
    assert_eq!(qe.name(), "my_query");
}

#[test]
fn named_scoped_query() {
    let world = World::new();

    let e1 = world.entity().add(Position::id());
    let e2 = world.entity().add(Position::id());

    let q = world.new_query_named::<&Position>("my::query");

    let mut count = 0;
    q.each_entity_shared(&world, |e, _pos| {
        assert!(e.id() == e1.id() || e.id() == e2.id());
        count += 1;
    });
    assert_eq!(count, 2);

    let qe = q.entity();
    assert_ne!(*qe.id(), 0u64);
    assert_eq!(qe.name(), "query");
    assert_eq!(qe.path(), Some("::my::query".to_string()));
}

#[test]
fn set_this_var() {
    let world = World::new();

    let _e_1 = world.entity().set(Position { x: 1, y: 2 });
    let e_2 = world.entity().set(Position { x: 3, y: 4 });
    let _e_3 = world.entity().set(Position { x: 5, y: 6 });

    let q = world.new_query_named::<&Position>("my::query");

    let mut count = 0;
    q.with_var(0, e_2).each_entity_shared(&world, |e, _pos| {
        assert_eq!(e.id(), e_2.id());
        count += 1;
    });
    assert_eq!(count, 1);
}

#[test]
fn inspect_terms_w_expr() {
    let world = World::new();

    let q = world
        .query::<()>()
        .expr("(ChildOf,#0)")
        .build()
        .expect("valid query expression");

    let mut count = 0;
    q.each_term(|term| {
        assert!(world.id_view_from(term.id()).is_pair());
        count += 1;
    });

    assert_eq!(count, 1);
}

#[test]
fn find() {
    let world = World::new();

    let _e1 = world.entity().set(Position { x: 10, y: 20 });
    let e2 = world.entity().set(Position { x: 20, y: 30 });

    let q = world.new_query::<&Position>();

    let result = q.find(|p| p.x == 20);
    assert_eq!(result.unwrap(), e2);
}

#[test]
fn find_not_found() {
    let world = World::new();

    let _e1 = world.entity().set(Position { x: 10, y: 20 });
    let _e2 = world.entity().set(Position { x: 20, y: 30 });

    let q = world.new_query::<&Position>();

    let result = q.find(|p| p.x == 30);
    assert!(result.is_none());
}

#[test]
fn find_w_entity() {
    let world = World::new();

    let _e1 = world
        .entity()
        .set(Position { x: 10, y: 20 })
        .set(Velocity { x: 20, y: 30 });
    let e2 = world
        .entity()
        .set(Position { x: 20, y: 30 })
        .set(Velocity { x: 20, y: 30 });

    let q = world.new_query::<&Position>();

    let result = q.find_entity(|e, p| { let v = e.get_ref::<&Velocity>().unwrap(); p.x == v.x && p.y == v.y });

    assert_eq!(result.unwrap(), e2);
}

#[test]
fn find_w_match_empty_tables() {
    let world = World::new();

    let e1 = world
        .entity()
        .set(Position { x: 10, y: 20 })
        .add(Velocity::id());
    e1.destruct();
    let e2 = world.entity().set(Position { x: 20, y: 30 });

    let q = world
        .query::<&Position>()
        .query_flags(QueryFlags::MatchEmptyTables)
        .build();

    let result = q.find(|p| p.x == 20);
    assert_eq!(result.unwrap(), e2);
}

#[test]
fn find_w_entity_w_match_empty_tables() {
    let world = World::new();

    let e1 = world
        .entity()
        .set(Position { x: 10, y: 20 })
        .add(Velocity::id());
    e1.destruct();
    let e2 = world.entity().set(Position { x: 20, y: 30 });

    let q = world
        .query::<&Position>()
        .query_flags(QueryFlags::MatchEmptyTables)
        .build();

    let result = q.find_entity(|_e, p| p.x == 20);
    assert_eq!(result.unwrap(), e2);
}

#[test]
fn tag_w_each() {
    let world = World::new();

    let e = world.entity().add(Tag::id());

    let mut count = 0;
    // Tags (ZST) must use id-based query, not type parameter
    let q = world.query::<()>().with(Tag::id()).build();
    q.each_entity_shared(&world, |qe, ()| {
        assert_eq!(qe.id(), e.id());
        count += 1;
    });

    assert_eq!(count, 1);
}

#[test]
fn shared_tag_w_each() {
    let world = World::new();

    let base = world.prefab().add(Tag::id());
    let e = world.entity().is_a(base);

    let mut count = 0;
    let q = world.query::<()>().with(Tag::id()).build();
    q.each_entity_shared(&world, |qe, ()| {
        assert_eq!(qe.id(), e.id());
        count += 1;
    });

    assert_eq!(count, 1);
}

#[test]
fn changed() {
    let mut world = World::new();

    let e = world.entity().set(Position { x: 1, y: 0 });

    let q = world.query::<&Position>().detect_changes().build();
    let q_w = world.new_query::<&mut Position>();

    assert!(q.is_changed());

    q.each_shared(&world, |_pos| {});
    assert!(!q.is_changed());

    e.set(Position { x: 2, y: 0 });
    assert!(q.is_changed());

    q.each_exclusive(&mut world, |_pos| {});
    assert!(!q.is_changed());

    q_w.each_exclusive(&mut world, |_pos| {});
    assert!(q.is_changed());
}

#[test]
fn default_ctor() {
    let mut world = World::new();

    #[allow(unused_assignments)]
    let mut q_var: Option<Query<&Position>> = None;
    let q = world.query::<&Position>().build();

    let e = world.entity().set(Position { x: 10, y: 20 });
    let _ = e;

    q_var = Some(q);

    if let Some(q) = q_var {
        let mut count = 0;
        q.each_entity_exclusive(&mut world, |_e, p| {
            assert_eq!(p.x, 10);
            assert_eq!(p.y, 20);
            count += 1;
        });

        assert_eq!(count, 1);
    }
}

#[test]
fn inspect_terms() {
    let world = World::new();

    let p_entity = world.entity();

    let q = world
        .query::<()>()
        .with(Position::id())
        .with(Velocity::id())
        .with((flecs::ChildOf::ID, p_entity))
        .build();

    assert_eq!(q.field_count(), 3);

    let t = q.term(0);
    assert_eq!(
        world.id_view_from(t.id()),
        world.id_view_from(Position::id())
    );
    assert_eq!(t.oper(), OperKind::And);
    assert_eq!(t.inout(), InOutKind::Default);

    let t = q.term(1);
    assert_eq!(
        world.id_view_from(t.id()),
        world.id_view_from(Velocity::id())
    );
    assert_eq!(t.oper(), OperKind::And);
    assert_eq!(t.inout(), InOutKind::Default);

    let t = q.term(2);
    assert_eq!(
        world.id_view_from(t.id()),
        world.id_view_from((flecs::ChildOf::ID, p_entity.id()))
    );
    assert_eq!(t.oper(), OperKind::And);
    assert_eq!(t.inout(), InOutKind::Default);
    assert_eq!(*world.id_view_from(t.id()).second_id(), *p_entity.id());
}

#[test]
fn inspect_terms_w_each() {
    let world = World::new();

    let p_entity = world.entity();

    let q = world
        .query::<()>()
        .with(Position::id())
        .with(Velocity::id())
        .with((flecs::ChildOf::ID, p_entity))
        .build();

    let mut count = 0;
    q.each_term(|t| {
        if count == 0 {
            assert_eq!(
                world.id_view_from(t.id()),
                world.id_view_from(Position::id())
            );
            assert_eq!(t.inout(), InOutKind::Default);
        } else if count == 1 {
            assert_eq!(
                world.id_view_from(t.id()),
                world.id_view_from(Velocity::id())
            );
            assert_eq!(t.inout(), InOutKind::Default);
        } else if count == 2 {
            assert_eq!(
                world.id_view_from(t.id()),
                world.id_view_from((flecs::ChildOf::ID, p_entity.id()))
            );
            assert_eq!(*world.id_view_from(t.id()).second_id(), *p_entity.id());
            assert_eq!(t.inout(), InOutKind::Default);
        }
        assert_eq!(t.oper(), OperKind::And);
        count += 1;
    });

    assert_eq!(count, 3);
}

#[test]
fn comp_to_str() {
    let world = World::new();

    let q = world
        .query::<()>()
        .with(Position::id())
        .with(Velocity::id())
        .build();

    assert_eq!(q.to_string(), "Position($this), Velocity($this)");
}

#[test]
fn each_pair_type() {
    #[derive(Component, Default)]
    struct Eats {
        amount: i32,
    }
    #[derive(Component)]
    struct Apples;
    #[derive(Component)]
    struct Pears;

    let world = World::new();

    let e1 = world.entity().set_pair::<Eats, Apples>(Eats { amount: 10 });
    world.entity().set_pair::<Eats, Pears>(Eats { amount: 20 });

    let q = world.new_query::<&mut (Eats, Apples)>();

    let mut count = 0;
    q.each_entity_shared(&world, |e, a| {
        assert_eq!(a.amount, 10);
        assert_eq!(e, e1);
        a.amount += 1;
        count += 1;
    });

    assert_eq!(count, 1);

    { let v = e1.get_ref::<&(Eats, Apples)>().unwrap();
        assert_eq!(v.amount, 11);
    };
}

#[test]
fn each_no_entity_1_comp() {
    let world = World::new();

    let e = world.entity().set(Position { x: 1, y: 2 });

    let q = world.new_query::<&mut Position>();

    let mut count = 0;
    q.each_shared(&world, |p| {
        assert_eq!(p.x, 1);
        assert_eq!(p.y, 2);
        p.x += 1;
        p.y += 2;
        count += 1;
    });

    assert_eq!(count, 1);

    { let pos = e.get_ref::<&Position>().unwrap();
        assert_eq!(pos.x, 2);
        assert_eq!(pos.y, 4);
    };
}

#[test]
fn each_no_entity_2_comps() {
    let world = World::new();

    let e = world
        .entity()
        .set(Position { x: 1, y: 2 })
        .set(Velocity { x: 10, y: 20 });

    let q = world.new_query::<(&mut Position, &mut Velocity)>();

    let mut count = 0;
    q.each_shared(&world, |(p, v)| {
        assert_eq!(p.x, 1);
        assert_eq!(p.y, 2);
        assert_eq!(v.x, 10);
        assert_eq!(v.y, 20);
        p.x += 1;
        p.y += 2;
        v.x += 1;
        v.y += 2;
        count += 1;
    });

    assert_eq!(count, 1);

    { let (p, v) = e.get_ref::<(&Position, &Velocity)>().unwrap();
        assert_eq!(p.x, 2);
        assert_eq!(p.y, 4);
        assert_eq!(v.x, 11);
        assert_eq!(v.y, 22);
    };
}

#[test]
fn instanced_query_w_singleton_each() {
    let world = World::new();

    world
        .component::<Velocity>()
        .add_trait::<flecs::Singleton>();
    world.set(Velocity { x: 1, y: 2 });

    let e1 = world.entity().set(Position { x: 10, y: 20 });
    e1.set(SelfRef { value: *e1 });
    let e2 = world.entity().set(Position { x: 20, y: 30 });
    e2.set(SelfRef { value: *e2 });
    let e3 = world.entity().set(Position { x: 30, y: 40 });
    e3.set(SelfRef { value: *e3 });
    let e4 = world.entity().set(Position { x: 40, y: 50 }).add(Tag::id());
    e4.set(SelfRef { value: *e4 });
    let e5 = world.entity().set(Position { x: 50, y: 60 }).add(Tag::id());
    e5.set(SelfRef { value: *e5 });

    let q = world.new_query::<(&SelfRef, &mut Position, &Velocity)>();

    let mut count = 0;
    q.each_entity_shared(&world, |e, (s, p, v)| {
        assert_eq!(e, s.value);
        p.x += v.x;
        p.y += v.y;
        count += 1;
    });

    assert_eq!(count, 5);

    { let p = e1.get_ref::<&Position>().unwrap();
        assert_eq!(p.x, 11);
        assert_eq!(p.y, 22);
    };
    { let p = e2.get_ref::<&Position>().unwrap();
        assert_eq!(p.x, 21);
        assert_eq!(p.y, 32);
    };
    { let p = e3.get_ref::<&Position>().unwrap();
        assert_eq!(p.x, 31);
        assert_eq!(p.y, 42);
    };
    { let p = e4.get_ref::<&Position>().unwrap();
        assert_eq!(p.x, 41);
        assert_eq!(p.y, 52);
    };
    { let p = e5.get_ref::<&Position>().unwrap();
        assert_eq!(p.x, 51);
        assert_eq!(p.y, 62);
    };
}

#[test]
fn instanced_query_w_base_each() {
    let world = World::new();

    let base = world.entity().set(Velocity { x: 1, y: 2 });

    let e1 = world.entity().is_a(base).set(Position { x: 10, y: 20 });
    e1.set(SelfRef { value: *e1 });
    let e2 = world.entity().is_a(base).set(Position { x: 20, y: 30 });
    e2.set(SelfRef { value: *e2 });
    let e3 = world.entity().is_a(base).set(Position { x: 30, y: 40 });
    e3.set(SelfRef { value: *e3 });
    let e4 = world
        .entity()
        .is_a(base)
        .set(Position { x: 40, y: 50 })
        .add(Tag::id());
    e4.set(SelfRef { value: *e4 });
    let e5 = world
        .entity()
        .is_a(base)
        .set(Position { x: 50, y: 60 })
        .add(Tag::id());
    e5.set(SelfRef { value: *e5 });
    let e6 = world
        .entity()
        .set(Position { x: 60, y: 70 })
        .set(Velocity { x: 2, y: 3 });
    e6.set(SelfRef { value: *e6 });
    let e7 = world
        .entity()
        .set(Position { x: 70, y: 80 })
        .set(Velocity { x: 4, y: 5 });
    e7.set(SelfRef { value: *e7 });

    let q = world.new_query::<(&SelfRef, &mut Position, &Velocity)>();

    let mut count = 0;
    q.each_entity_shared(&world, |e, (s, p, v)| {
        assert_eq!(e, s.value);
        p.x += v.x;
        p.y += v.y;
        count += 1;
    });

    assert_eq!(count, 7);

    { let p = e1.get_ref::<&Position>().unwrap();
        assert_eq!(p.x, 11);
        assert_eq!(p.y, 22);
    };
    { let p = e2.get_ref::<&Position>().unwrap();
        assert_eq!(p.x, 21);
        assert_eq!(p.y, 32);
    };
    { let p = e3.get_ref::<&Position>().unwrap();
        assert_eq!(p.x, 31);
        assert_eq!(p.y, 42);
    };
    { let p = e4.get_ref::<&Position>().unwrap();
        assert_eq!(p.x, 41);
        assert_eq!(p.y, 52);
    };
    { let p = e5.get_ref::<&Position>().unwrap();
        assert_eq!(p.x, 51);
        assert_eq!(p.y, 62);
    };
    { let p = e6.get_ref::<&Position>().unwrap();
        assert_eq!(p.x, 62);
        assert_eq!(p.y, 73);
    };
    { let p = e7.get_ref::<&Position>().unwrap();
        assert_eq!(p.x, 74);
        assert_eq!(p.y, 85);
    };
}

#[test]
fn query_each_from_component() {
    #[derive(Component)]
    struct QueryComponent {
        q: Query<(&'static Position, &'static Velocity)>,
    }

    let world = World::new();

    world
        .entity()
        .set(Position { x: 0, y: 0 })
        .set(Velocity { x: 0, y: 0 });
    world
        .entity()
        .set(Position { x: 0, y: 0 })
        .set(Velocity { x: 0, y: 0 });

    let q = world.new_query::<(&Position, &Velocity)>();
    let e = world.entity().set(QueryComponent { q });

    let mut count = 0;
    { let qc = e.get_ref::<&QueryComponent>().unwrap();
        qc.q.each_shared(&world, |(_p, _v)| {
            count += 1;
        });
    };

    assert_eq!(count, 2);
}

#[test]
fn query_each_w_func_ptr() {
    thread_local! { static INVOKED: core::cell::Cell<i32> = const { core::cell::Cell::new(0) }; }
    fn each_func(_e: EntityView, p: &mut Position) {
        INVOKED.with(|c| c.set(c.get() + 1));
        p.x += 1;
        p.y += 1;
    }

    let world = World::new();

    let e = world.entity().set(Position { x: 10, y: 20 });

    let q = world.new_query::<&mut Position>();

    q.each_entity_shared(&world, each_func);

    INVOKED.with(|c| assert_eq!(c.get(), 1));

    { let p = e.get_ref::<&Position>().unwrap();
        assert_eq!(p.x, 11);
        assert_eq!(p.y, 21);
    };
}

#[test]
fn instanced_query_w_singleton_iter() {
    let world = World::new();

    world
        .component::<Velocity>()
        .add_trait::<flecs::Singleton>();
    world.set(Velocity { x: 1, y: 2 });

    let e1 = world.entity().set(Position { x: 10, y: 20 });
    e1.set(SelfRef { value: *e1 });
    let e2 = world.entity().set(Position { x: 20, y: 30 });
    e2.set(SelfRef { value: *e2 });
    let e3 = world.entity().set(Position { x: 30, y: 40 });
    e3.set(SelfRef { value: *e3 });
    let e4 = world.entity().set(Position { x: 40, y: 50 }).add(Tag::id());
    e4.set(SelfRef { value: *e4 });
    let e5 = world.entity().set(Position { x: 50, y: 60 }).add(Tag::id());
    e5.set(SelfRef { value: *e5 });

    let q = world.new_query::<(&SelfRef, &mut Position, &Velocity)>();

    let mut count = 0;
    q.run_shared(&world, |mut it| {
        while it.next() {
            let s = it.field::<SelfRef>(0);
            let v = it.field::<Velocity>(2);
            assert!(it.count() > 1);
            for i in it.iter() {
                let mut p = it.field_at_mut::<Position>(1, i);
                p.x += v[0].x;
                p.y += v[0].y;
                assert_eq!(it.entity_id(i), s[i].value);
                count += 1;
            }
        }
    });

    assert_eq!(count, 5);
    { let p = e1.get_ref::<&Position>().unwrap();
        assert_eq!(p.x, 11);
        assert_eq!(p.y, 22);
    };
    { let p = e2.get_ref::<&Position>().unwrap();
        assert_eq!(p.x, 21);
        assert_eq!(p.y, 32);
    };
    { let p = e3.get_ref::<&Position>().unwrap();
        assert_eq!(p.x, 31);
        assert_eq!(p.y, 42);
    };
    { let p = e4.get_ref::<&Position>().unwrap();
        assert_eq!(p.x, 41);
        assert_eq!(p.y, 52);
    };
    { let p = e5.get_ref::<&Position>().unwrap();
        assert_eq!(p.x, 51);
        assert_eq!(p.y, 62);
    };
}

#[test]
fn instanced_query_w_base_iter() {
    let world = World::new();

    let base = world.entity().set(Velocity { x: 1, y: 2 });

    let e1 = world.entity().is_a(base).set(Position { x: 10, y: 20 });
    e1.set(SelfRef { value: *e1 });
    let e2 = world.entity().is_a(base).set(Position { x: 20, y: 30 });
    e2.set(SelfRef { value: *e2 });
    let e3 = world.entity().is_a(base).set(Position { x: 30, y: 40 });
    e3.set(SelfRef { value: *e3 });
    let e4 = world
        .entity()
        .is_a(base)
        .set(Position { x: 40, y: 50 })
        .add(Tag::id());
    e4.set(SelfRef { value: *e4 });
    let e5 = world
        .entity()
        .is_a(base)
        .set(Position { x: 50, y: 60 })
        .add(Tag::id());
    e5.set(SelfRef { value: *e5 });
    let e6 = world
        .entity()
        .set(Position { x: 60, y: 70 })
        .set(Velocity { x: 2, y: 3 });
    e6.set(SelfRef { value: *e6 });
    let e7 = world
        .entity()
        .set(Position { x: 70, y: 80 })
        .set(Velocity { x: 4, y: 5 });
    e7.set(SelfRef { value: *e7 });

    let q = world.new_query::<(&SelfRef, &mut Position, &Velocity)>();

    let mut count = 0;
    q.run_shared(&world, |mut it| {
        while it.next() {
            let s = it.field::<SelfRef>(0);
            let v = it.field::<Velocity>(2);
            assert!(it.count() > 1);
            for i in it.iter() {
                let mut p = it.field_at_mut::<Position>(1, i);
                if it.is_self(2) {
                    p.x += v[i].x;
                    p.y += v[i].y;
                } else {
                    p.x += v[0].x;
                    p.y += v[0].y;
                }
                assert_eq!(it.entity_id(i), s[i].value);
                count += 1;
            }
        }
    });

    assert_eq!(count, 7);
    { let p = e1.get_ref::<&Position>().unwrap();
        assert_eq!(p.x, 11);
        assert_eq!(p.y, 22);
    };
    { let p = e2.get_ref::<&Position>().unwrap();
        assert_eq!(p.x, 21);
        assert_eq!(p.y, 32);
    };
    { let p = e3.get_ref::<&Position>().unwrap();
        assert_eq!(p.x, 31);
        assert_eq!(p.y, 42);
    };
    { let p = e4.get_ref::<&Position>().unwrap();
        assert_eq!(p.x, 41);
        assert_eq!(p.y, 52);
    };
    { let p = e5.get_ref::<&Position>().unwrap();
        assert_eq!(p.x, 51);
        assert_eq!(p.y, 62);
    };
    { let p = e6.get_ref::<&Position>().unwrap();
        assert_eq!(p.x, 62);
        assert_eq!(p.y, 73);
    };
    { let p = e7.get_ref::<&Position>().unwrap();
        assert_eq!(p.x, 74);
        assert_eq!(p.y, 85);
    };
}

#[test]
fn query_iter_from_component() {
    #[derive(Component)]
    struct QueryComponent2 {
        q: Query<(&'static Position, &'static Velocity)>,
    }

    let world = World::new();

    world
        .entity()
        .set(Position { x: 0, y: 0 })
        .set(Velocity { x: 0, y: 0 });
    world
        .entity()
        .set(Position { x: 0, y: 0 })
        .set(Velocity { x: 0, y: 0 });

    let q = world.new_query::<(&Position, &Velocity)>();
    let e = world.entity().set(QueryComponent2 { q });

    let mut count = 0;
    { let qc = e.get_ref::<&QueryComponent2>().unwrap();
        qc.q.run_shared(&world, |mut it| {
            while it.next() {
                count += it.count();
            }
        });
    };

    assert_eq!(count, 2);
}

#[test]
fn query_iter_w_func_ptr() {
    thread_local! { static INVOKED2: core::cell::Cell<i32> = const { core::cell::Cell::new(0) }; }

    let world = World::new();
    let e = world.entity().set(Position { x: 10, y: 20 });
    let q = world.new_query::<&mut Position>();

    q.run_shared(&world, |mut it| {
        assert!(it.next());
        assert_eq!(it.count(), 1);
        {
            let mut p = it.field_mut::<Position>(0);
            p[0].x += 1;
            p[0].y += 1;
        }
        assert!(!it.next());
        INVOKED2.with(|c| c.set(c.get() + 1));
    });

    INVOKED2.with(|c| assert_eq!(c.get(), 1));
    { let p = e.get_ref::<&Position>().unwrap();
        assert_eq!(p.x, 11);
        assert_eq!(p.y, 21);
    };
}

#[test]
fn query_each_w_func_no_ptr() {
    thread_local! { static INVOKED3: core::cell::Cell<i32> = const { core::cell::Cell::new(0) }; }
    fn each_func_no_ptr(_e: EntityView, p: &mut Position) {
        INVOKED3.with(|c| c.set(c.get() + 1));
        p.x += 1;
        p.y += 1;
    }

    let world = World::new();
    let e = world.entity().set(Position { x: 10, y: 20 });
    let q = world.new_query::<&mut Position>();
    q.each_entity_shared(&world, each_func_no_ptr);

    INVOKED3.with(|c| assert_eq!(c.get(), 1));
    { let p = e.get_ref::<&Position>().unwrap();
        assert_eq!(p.x, 11);
        assert_eq!(p.y, 21);
    };
}

#[test]
fn query_iter_w_func_no_ptr() {
    thread_local! { static INVOKED4: core::cell::Cell<i32> = const { core::cell::Cell::new(0) }; }

    let world = World::new();
    let e = world.entity().set(Position { x: 10, y: 20 });
    let q = world.new_query::<&mut Position>();

    q.run_shared(&world, |mut it| {
        while it.next() {
            let mut p = it.field_mut::<Position>(0);
            for i in it.iter() {
                p[i].x += 1;
                p[i].y += 1;
            }
        }
        INVOKED4.with(|c| c.set(c.get() + 1));
    });

    INVOKED4.with(|c| assert_eq!(c.get(), 1));
    { let p = e.get_ref::<&Position>().unwrap();
        assert_eq!(p.x, 11);
        assert_eq!(p.y, 21);
    };
}

#[test]
fn query_each_w_iter() {
    let world = World::new();

    let e1 = world.entity();
    e1.set(SelfRef { value: *e1 });
    e1.set(Position { x: 10, y: 20 });
    let e2 = world.entity();
    e2.set(SelfRef { value: *e2 });
    e2.set(Position { x: 20, y: 30 });

    let q = world.new_query::<(&SelfRef, &mut Position)>();

    let mut invoked = 0;
    q.each_iter(|it, i, (s, p)| {
        assert_eq!(it.count(), 2);
        assert_eq!(it.entity_id(i), s.value);
        p.x += 1;
        p.y += 1;
        invoked += 1;
    });

    assert_eq!(invoked, 2);
    { let p = e1.get_ref::<&Position>().unwrap();
        assert_eq!(p.x, 11);
        assert_eq!(p.y, 21);
    };
    { let p = e2.get_ref::<&Position>().unwrap();
        assert_eq!(p.x, 21);
        assert_eq!(p.y, 31);
    };
}

#[test]
#[should_panic]
#[cfg_attr(not(debug_assertions), ignore)]
fn invalid_field_from_each_w_iter() {
    let world = World::new();
    let _guard = FlecsPanicAbortGuard::install();

    world
        .entity()
        .set(Position { x: 10, y: 20 })
        .set(Velocity { x: 1, y: 2 });

    let q = world.query::<&Position>().with(Velocity::id()).build();

    q.each_iter(|it, _i, _p| {
        it.field::<Velocity>(1);
    });
}

#[test]
#[should_panic]
#[cfg_attr(not(debug_assertions), ignore)]
fn invalid_field_t_from_each_w_iter() {
    let world = World::new();
    let _guard = FlecsPanicAbortGuard::install();

    world
        .entity()
        .set(Position { x: 10, y: 20 })
        .set(Velocity { x: 1, y: 2 });

    let q = world.query::<&Position>().with(Velocity::id()).build();

    q.each_iter(|it, _i, _p| {
        it.field::<Velocity>(1);
    });
}

#[test]
#[should_panic]
#[cfg_attr(not(debug_assertions), ignore)]
fn invalid_field_const_t_from_each_w_iter() {
    let world = World::new();
    let _guard = FlecsPanicAbortGuard::install();

    world
        .entity()
        .set(Position { x: 10, y: 20 })
        .set(Velocity { x: 1, y: 2 });

    let q = world.query::<&Position>().with(Velocity::id()).build();

    q.each_iter(|it, _i, _p| {
        it.field::<Velocity>(1);
    });
}

#[test]
fn field_at_from_each_w_iter() {
    let world = World::new();

    let e1 = world
        .entity()
        .set(Position { x: 10, y: 20 })
        .set(Velocity { x: 1, y: 2 });
    let e2 = world
        .entity()
        .set(Position { x: 20, y: 30 })
        .set(Velocity { x: 3, y: 4 });

    let q = world.query::<&Position>().with(Velocity::id()).build();

    let mut count = 0;
    q.each_iter(|it, row, _p| {
        let v = it.field_at::<Velocity>(1, row);
        if it.entity_id(row) == e1.id() {
            assert_eq!(v.x, 1);
            assert_eq!(v.y, 2);
            count += 1;
        } else if it.entity_id(row) == e2.id() {
            assert_eq!(v.x, 3);
            assert_eq!(v.y, 4);
            count += 1;
        }
    });

    assert_eq!(count, 2);
}

#[test]
fn field_at_t_from_each_w_iter() {
    let world = World::new();

    let e1 = world
        .entity()
        .set(Position { x: 10, y: 20 })
        .set(Velocity { x: 1, y: 2 });
    let e2 = world
        .entity()
        .set(Position { x: 20, y: 30 })
        .set(Velocity { x: 3, y: 4 });

    let q = world.query::<&Position>().with(Velocity::id()).build();

    let mut count = 0;
    q.each_iter(|it, row, _p| {
        let v = it.field_at::<Velocity>(1, row);
        if it.entity_id(row) == e1.id() {
            assert_eq!(v.x, 1);
            assert_eq!(v.y, 2);
            count += 1;
        } else if it.entity_id(row) == e2.id() {
            assert_eq!(v.x, 3);
            assert_eq!(v.y, 4);
            count += 1;
        }
    });

    assert_eq!(count, 2);
}

#[test]
fn field_at_const_t_from_each_w_iter() {
    let world = World::new();

    let e1 = world
        .entity()
        .set(Position { x: 10, y: 20 })
        .set(Velocity { x: 1, y: 2 });
    let e2 = world
        .entity()
        .set(Position { x: 20, y: 30 })
        .set(Velocity { x: 3, y: 4 });

    let q = world.query::<&Position>().with(Velocity::id()).build();

    let mut count = 0;
    q.each_iter(|it, row, _p| {
        let v = it.field_at::<Velocity>(1, row);
        if it.entity_id(row) == e1.id() {
            assert_eq!(v.x, 1);
            assert_eq!(v.y, 2);
            count += 1;
        } else if it.entity_id(row) == e2.id() {
            assert_eq!(v.x, 3);
            assert_eq!(v.y, 4);
            count += 1;
        }
    });

    assert_eq!(count, 2);
}

#[test]
fn change_tracking() {
    let world = World::new();

    let qw = world.new_query::<&mut Position>();
    let qr = world.query::<&Position>().detect_changes().build();

    let e1 = world.entity().add(Tag::id()).set(Position { x: 10, y: 20 });
    world.entity().set(Position { x: 20, y: 30 });

    assert!(qr.is_changed());
    qr.run_shared(&world, |mut it| while it.next() {});
    assert!(!qr.is_changed());

    let mut count = 0;
    let mut change_count = 0;

    qw.run_shared(&world, |mut it| {
        while it.next() {
            assert_eq!(it.count(), 1);
            count += 1;

            if it.entity_id(0usize) == e1.id() {
                it.skip();
                continue;
            }

            change_count += 1;
        }
    });

    assert_eq!(count, 2);
    assert_eq!(change_count, 1);

    count = 0;
    change_count = 0;

    assert!(qr.is_changed());

    qr.run_shared(&world, |mut it| {
        while it.next() {
            assert_eq!(it.count(), 1);
            count += 1;

            if it.entity_id(0usize) == e1.id() {
                assert!(!it.is_changed());
            } else {
                assert!(it.is_changed());
                change_count += 1;
            }
        }
    });

    assert_eq!(count, 2);
    assert_eq!(change_count, 1);
}

#[test]
fn not_w_write() {
    let world = World::new();

    #[derive(Component)]
    struct A;

    #[derive(Component)]
    struct B;

    let q = world
        .query::<()>()
        .with(A::id())
        .with(B::id())
        .not()
        .write_curr()
        .build();

    let e = world.entity().add(A::id());

    let mut count = 0;
    world.defer(|| {
        q.each_iter(|it, i, ()| {
            it.entity(i).add(B::id());
            count += 1;
        });
    });

    assert_eq!(count, 1);
    assert!(e.has(B::id()));

    q.each_iter(|_it, _i, ()| {
        count += 1;
    });

    assert_eq!(count, 1);
}

#[test]
fn instanced_nested_query_w_iter() {
    let world = World::new();

    world.component::<Mass>().add_trait::<flecs::Singleton>();

    let q1 = world
        .query::<()>()
        .with(Position::id())
        .with(Mass::id())
        .build();

    let q2 = world.query::<()>().with(Velocity::id()).build();

    world.add(Mass::id());
    world.entity().add(Velocity::id());
    world.entity().add(Position::id());
    world.entity().add(Position::id());

    let mut count = 0;

    q2.run_shared(&world, |mut it_2| {
        while it_2.next() {
            q1.iter_stage(it_2.world()).run_shared(&world, |mut it_1| {
                while it_1.next() {
                    assert_eq!(it_1.count(), 2);
                    count += it_1.count();
                }
            });
        }
    });

    assert_eq!(count, 2);
}

#[test]
fn instanced_nested_query_w_entity() {
    let world = World::new();

    world.component::<Mass>().add_trait::<flecs::Singleton>();

    let q1 = world
        .query::<()>()
        .with(Position::id())
        .with(Mass::id())
        .build();

    let q2 = world.query::<()>().with(Velocity::id()).build();

    world.add(Mass::id());
    world.entity().add(Velocity::id());
    world.entity().add(Position::id());
    world.entity().add(Position::id());

    let mut count = 0;

    q2.each_entity_shared(&world, |e_2, ()| {
        q1.iter_stage(e_2).run_shared(&world, |mut it_1| {
            while it_1.next() {
                assert_eq!(it_1.count(), 2);
                count += it_1.count();
            }
        });
    });

    assert_eq!(count, 2);
}

#[test]
fn instanced_nested_query_w_world() {
    let world = World::new();

    world.component::<Mass>().add_trait::<flecs::Singleton>();

    let q1 = world
        .query::<()>()
        .with(Position::id())
        .with(Mass::id())
        .build();

    let q2 = world.query::<()>().with(Velocity::id()).build();

    world.add(Mass::id());
    world.entity().add(Velocity::id());
    world.entity().add(Position::id());
    world.entity().add(Position::id());

    let mut count = 0;

    q2.run_shared(&world, |mut it_2| {
        while it_2.next() {
            q1.iter_stage(it_2.world()).run_shared(&world, |mut it_1| {
                while it_1.next() {
                    assert_eq!(it_1.count(), 2);
                    count += it_1.count();
                }
            });
        }
    });

    assert_eq!(count, 2);
}

#[test]
fn captured_query() {
    let world = World::new();

    let q = world.new_query::<&Position>();
    let e1 = world.entity().set(Position { x: 10, y: 20 });

    let run_query = || {
        let mut count = 0;
        q.each_entity_shared(&world, |e, p| {
            assert_eq!(e.id(), e1.id());
            assert_eq!(p.x, 10);
            assert_eq!(p.y, 20);
            count += 1;
        });
        assert_eq!(count, 1);
    };
    run_query();
}

#[test]
fn page_iter_captured_query() {
    let world = World::new();

    let q = world.new_query::<&Position>();
    world.entity().set(Position { x: 10, y: 20 });
    let e2 = world.entity().set(Position { x: 20, y: 30 });
    world.entity().set(Position { x: 10, y: 20 });

    let mut count = 0;
    q.page(1, 1).each_entity_shared(&world, |e, p| {
        assert_eq!(e.id(), e2.id());
        assert_eq!(p.x, 20);
        assert_eq!(p.y, 30);
        count += 1;
    });
    assert_eq!(count, 1);
}

#[test]
fn worker_iter_captured_query() {
    let world = World::new();

    let q = world.new_query::<&Position>();
    world.entity().set(Position { x: 10, y: 20 });
    let e2 = world.entity().set(Position { x: 20, y: 30 });
    world.entity().set(Position { x: 10, y: 20 });

    let mut count = 0;
    q.worker(1, 3).each_entity_shared(&world, |e, p| {
        assert_eq!(e.id(), e2.id());
        assert_eq!(p.x, 20);
        assert_eq!(p.y, 30);
        count += 1;
    });
    assert_eq!(count, 1);
}

#[test]
fn set_group_captured_query() {
    let world = World::new();

    let rel = world.entity();
    let tgt_a = world.entity();
    let tgt_b = world.entity();

    let q = world.query::<&Position>().group_by(rel).build();

    world
        .entity()
        .set(Position { x: 10, y: 20 })
        .add((rel, tgt_a));
    let e2 = world
        .entity()
        .set(Position { x: 20, y: 30 })
        .add((rel, tgt_b));

    let mut count = 0;
    q.with_group(tgt_b).each_entity_shared(&world, |e, p| {
        assert_eq!(e.id(), e2.id());
        assert_eq!(p.x, 20);
        assert_eq!(p.y, 30);
        count += 1;
    });
    assert_eq!(count, 1);
}

#[test]
fn set_var_captured_query() {
    let world = World::new();

    let rel = world.entity();
    let tgt_a = world.entity();
    let tgt_b = world.entity();

    let q = world.query::<&Position>().with((rel, "$var")).build();

    world
        .entity()
        .set(Position { x: 10, y: 20 })
        .add((rel, tgt_a));
    let e2 = world
        .entity()
        .set(Position { x: 20, y: 30 })
        .add((rel, tgt_b));

    let mut count = 0;
    q.with_var_expr("var", tgt_b).each_entity_shared(&world, |e, p| {
        assert_eq!(e.id(), e2.id());
        assert_eq!(p.x, 20);
        assert_eq!(p.y, 30);
        count += 1;
    });
    assert_eq!(count, 1);
}

#[test]
fn set_var_id_captured_query() {
    let world = World::new();

    let rel = world.entity();
    let tgt_a = world.entity();
    let tgt_b = world.entity();

    let q = world.query::<&Position>().with((rel, "$var")).build();

    let var_id = q.find_var("var").expect("variable 'var' not found");

    world
        .entity()
        .set(Position { x: 10, y: 20 })
        .add((rel, tgt_a));
    let e2 = world
        .entity()
        .set(Position { x: 20, y: 30 })
        .add((rel, tgt_b));

    let mut count = 0;
    q.with_var(var_id, tgt_b).each_entity_shared(&world, |e, p| {
        assert_eq!(e.id(), e2.id());
        assert_eq!(p.x, 20);
        assert_eq!(p.y, 30);
        count += 1;
    });
    assert_eq!(count, 1);
}

#[test]
fn run() {
    let world = World::new();

    let entity = world
        .entity()
        .set(Position { x: 10, y: 20 })
        .set(Velocity { x: 1, y: 2 });

    let q = world.new_query::<(&mut Position, &Velocity)>();

    q.run_shared(&world, |mut it| {
        while it.next() {
            let v = it.field::<Velocity>(1);
            for i in it.iter() {
                let mut p = it.field_at_mut::<Position>(0, i);
                p.x += v[i].x;
                p.y += v[i].y;
            }
        }
    });

    { let p = entity.get_ref::<&Position>().unwrap();
        assert_eq!(p.x, 11);
        assert_eq!(p.y, 22);
    };
}

#[test]
fn run_const() {
    let world = World::new();

    let entity = world
        .entity()
        .set(Position { x: 10, y: 20 })
        .set(Velocity { x: 1, y: 2 });

    let q = world.new_query::<(&mut Position, &Velocity)>();

    q.run_shared(&world, |mut it| {
        while it.next() {
            let v = it.field::<Velocity>(1);
            for i in it.iter() {
                let mut p = it.field_at_mut::<Position>(0, i);
                p.x += v[i].x;
                p.y += v[i].y;
            }
        }
    });

    { let p = entity.get_ref::<&Position>().unwrap();
        assert_eq!(p.x, 11);
        assert_eq!(p.y, 22);
    };
}

#[test]
fn run_shared() {
    let world = World::new();

    world
        .component::<Position>()
        .add((flecs::OnInstantiate::ID, flecs::Inherit::ID));
    world
        .component::<Velocity>()
        .add((flecs::OnInstantiate::ID, flecs::Inherit::ID));

    let base = world.entity().set(Velocity { x: 1, y: 2 });

    let e1 = world.entity().set(Position { x: 10, y: 20 }).is_a(base);

    let e2 = world
        .entity()
        .set(Position { x: 10, y: 20 })
        .set(Velocity { x: 3, y: 4 });

    let q = world
        .query::<&mut Position>()
        .expr("Velocity(self|up IsA)")
        .build()
        .expect("valid query expression");

    q.run_shared(&world, |mut it| {
        while it.next() {
            let mut p = it.field_mut::<Position>(0);
            let v = it.field::<Velocity>(1);

            if !it.is_self(1) {
                for i in it.iter() {
                    p[i].x += v[0].x;
                    p[i].y += v[0].y;
                }
            } else {
                for i in it.iter() {
                    p[i].x += v[i].x;
                    p[i].y += v[i].y;
                }
            }
        }
    });

    { let p = e1.get_ref::<&Position>().unwrap();
        assert_eq!(p.x, 11);
        assert_eq!(p.y, 22);
    };
    { let p = e2.get_ref::<&Position>().unwrap();
        assert_eq!(p.x, 13);
        assert_eq!(p.y, 24);
    };
}

#[test]
fn run_optional() {
    let world = World::new();

    let e1 = world
        .entity()
        .set(Position { x: 10, y: 20 })
        .set(Velocity { x: 1, y: 2 })
        .set(Mass { value: 1 });
    let e2 = world
        .entity()
        .set(Position { x: 30, y: 40 })
        .set(Velocity { x: 3, y: 4 })
        .set(Mass { value: 1 });
    let e3 = world.entity().set(Position { x: 50, y: 60 });
    let e4 = world.entity().set(Position { x: 70, y: 80 });

    let q = world.new_query::<(&mut Position, Option<&Velocity>, Option<&Mass>)>();

    q.run_shared(&world, |mut it| {
        while it.next() {
            let mut p = it.field_mut::<Position>(0);
            let v = it.field::<Velocity>(1);
            let m = it.field::<Mass>(2);

            if it.is_set(1) && it.is_set(2) {
                for i in it.iter() {
                    p[i].x += v[i].x * m[i].value;
                    p[i].y += v[i].y * m[i].value;
                }
            } else {
                for i in it.iter() {
                    p[i].x += 1;
                    p[i].y += 1;
                }
            }
        }
    });

    { let p = e1.get_ref::<&Position>().unwrap();
        assert_eq!(p.x, 11);
        assert_eq!(p.y, 22);
    };
    { let p = e2.get_ref::<&Position>().unwrap();
        assert_eq!(p.x, 33);
        assert_eq!(p.y, 44);
    };
    { let p = e3.get_ref::<&Position>().unwrap();
        assert_eq!(p.x, 51);
        assert_eq!(p.y, 61);
    };
    { let p = e4.get_ref::<&Position>().unwrap();
        assert_eq!(p.x, 71);
        assert_eq!(p.y, 81);
    };
}

#[test]
fn run_sparse() {
    let world = World::new();

    world.component::<Position>().add_trait::<flecs::Sparse>();

    let entity = world
        .entity()
        .set(Position { x: 10, y: 20 })
        .set(Velocity { x: 1, y: 2 });

    let q = world.query::<(&mut Position, &Velocity)>().build();

    q.run_shared(&world, |mut it| {
        while it.next() {
            let v = it.field::<Velocity>(1);
            for i in it.iter() {
                let mut p = it.field_at_mut::<Position>(0, i);
                p.x += v[i].x;
                p.y += v[i].y;
            }
        }
    });

    { let p = entity.get_ref::<&Position>().unwrap();
        assert_eq!(p.x, 11);
        assert_eq!(p.y, 22);
    };
}

#[test]
fn run_sparse_w_with() {
    let world = World::new();

    world.component::<Position>().add_trait::<flecs::Sparse>();

    let entity = world
        .entity()
        .set(Position { x: 10, y: 20 })
        .set(Velocity { x: 1, y: 2 });

    let q = world
        .query::<()>()
        .with(&mut Position::id())
        .with(&Velocity::id())
        .build();

    q.run_shared(&world, |mut it| {
        while it.next() {
            let v = it.field::<Velocity>(1);
            for i in it.iter() {
                let mut p = it.field_at_mut::<Position>(0, i);
                p.x += v[i].x;
                p.y += v[i].y;
            }
        }
    });

    { let p = entity.get_ref::<&Position>().unwrap();
        assert_eq!(p.x, 11);
        assert_eq!(p.y, 22);
    };
}

#[test]
fn run_dont_fragment() {
    let world = World::new();

    world
        .component::<Position>()
        .add_trait::<flecs::DontFragment>();

    let entity = world
        .entity()
        .set(Position { x: 10, y: 20 })
        .set(Velocity { x: 1, y: 2 });

    let q = world.new_query::<(&mut Position, &Velocity)>();

    q.run_shared(&world, |mut it| {
        while it.next() {
            let v = it.field::<Velocity>(1);
            for i in it.iter() {
                let mut p = it.field_at_mut::<Position>(0, i);
                p.x += v[i].x;
                p.y += v[i].y;
            }
        }
    });

    { let p = entity.get_ref::<&Position>().unwrap();
        assert_eq!(p.x, 11);
        assert_eq!(p.y, 22);
    };
}

#[test]
fn run_dont_fragment_w_with() {
    let world = World::new();

    world
        .component::<Position>()
        .add_trait::<flecs::DontFragment>();

    let entity = world
        .entity()
        .set(Position { x: 10, y: 20 })
        .set(Velocity { x: 1, y: 2 });

    let q = world
        .query::<()>()
        .with(&mut Position::id())
        .with(&Velocity::id())
        .build();

    q.run_shared(&world, |mut it| {
        while it.next() {
            let v = it.field::<Velocity>(1);
            for i in it.iter() {
                let mut p = it.field_at_mut::<Position>(0, i);
                p.x += v[i].x;
                p.y += v[i].y;
            }
        }
    });

    { let p = entity.get_ref::<&Position>().unwrap();
        assert_eq!(p.x, 11);
        assert_eq!(p.y, 22);
    };
}

#[test]
fn run_dont_fragment_add() {
    let world = World::new();

    world
        .component::<Velocity>()
        .add_trait::<flecs::DontFragment>();

    let entity = world
        .entity()
        .set(Position { x: 10, y: 20 })
        .set(Velocity { x: 1, y: 2 });

    let q = world.new_query::<&mut Position>();

    q.run_shared(&world, |mut it| {
        while it.next() {
            for i in it.iter() {
                let e = it.get_entity(i).unwrap();
                e.add(Velocity::id());
                assert!(e.has(Velocity::id()));
                let mut p = it.field_at_mut::<Position>(0, i);
                p.x += 1;
                p.y += 2;
            }
        }
    });

    { let p = entity.get_ref::<&Position>().unwrap();
        assert_eq!(p.x, 11);
        assert_eq!(p.y, 22);
    };
    assert!(entity.has(Velocity::id()));
}

#[test]
fn run_dont_fragment_add_remove() {
    let world = World::new();

    world
        .component::<Velocity>()
        .add_trait::<flecs::DontFragment>();

    let entity = world
        .entity()
        .set(Position { x: 10, y: 20 })
        .set(Velocity { x: 1, y: 2 });

    let q = world.new_query::<&mut Position>();

    q.run_shared(&world, |mut it| {
        while it.next() {
            for i in it.iter() {
                let e = it.get_entity(i).unwrap();
                e.add(Velocity::id());
                assert!(e.has(Velocity::id()));
                e.remove(Velocity::id());
                assert!(!e.has(Velocity::id()));
                let mut p = it.field_at_mut::<Position>(0, i);
                p.x += 1;
                p.y += 2;
            }
        }
    });

    { let p = entity.get_ref::<&Position>().unwrap();
        assert_eq!(p.x, 11);
        assert_eq!(p.y, 22);
    };
    assert!(!entity.has(Velocity::id()));
}

#[test]
fn run_dont_fragment_set() {
    let world = World::new();

    world
        .component::<Velocity>()
        .add_trait::<flecs::DontFragment>();

    let entity = world
        .entity()
        .set(Position { x: 10, y: 20 })
        .set(Velocity { x: 1, y: 2 });

    let q = world.new_query::<&mut Position>();

    q.run_shared(&world, |mut it| {
        while it.next() {
            for i in it.iter() {
                let e = it.get_entity(i).unwrap();
                e.set(Velocity { x: 1, y: 2 });
                assert!(e.has(Velocity::id()));
                { let v = e.get_ref::<&Velocity>().unwrap();
                    assert_eq!(v.x, 1);
                    assert_eq!(v.y, 2);
                };
                let mut p = it.field_at_mut::<Position>(0, i);
                p.x += 1;
                p.y += 2;
            }
        }
    });

    { let p = entity.get_ref::<&Position>().unwrap();
        assert_eq!(p.x, 11);
        assert_eq!(p.y, 22);
    };
    { let v = entity.get_ref::<&Velocity>().unwrap();
        assert_eq!(v.x, 1);
        assert_eq!(v.y, 2);
    };
}

#[test]
fn each() {
    let world = World::new();

    let entity = world
        .entity()
        .set(Position { x: 10, y: 20 })
        .set(Velocity { x: 1, y: 2 });

    let q = world.new_query::<(&mut Position, &Velocity)>();

    q.each_entity_shared(&world, |_e, (p, v)| {
        p.x += v.x;
        p.y += v.y;
    });

    { let p = entity.get_ref::<&Position>().unwrap();
        assert_eq!(p.x, 11);
        assert_eq!(p.y, 22);
    };
}

#[test]
fn each_const() {
    let world = World::new();

    let entity = world
        .entity()
        .set(Position { x: 10, y: 20 })
        .set(Velocity { x: 1, y: 2 });

    let q = world.new_query::<(&mut Position, &Velocity)>();

    q.each_entity_shared(&world, |_e, (p, v)| {
        p.x += v.x;
        p.y += v.y;
    });

    { let p = entity.get_ref::<&Position>().unwrap();
        assert_eq!(p.x, 11);
        assert_eq!(p.y, 22);
    };
}

#[test]
fn each_shared() {
    let world = World::new();

    world
        .component::<Position>()
        .add((flecs::OnInstantiate::ID, flecs::Inherit::ID));
    world
        .component::<Velocity>()
        .add((flecs::OnInstantiate::ID, flecs::Inherit::ID));

    let base = world.entity().set(Velocity { x: 1, y: 2 });

    let e1 = world.entity().set(Position { x: 10, y: 20 }).is_a(base);

    let e2 = world.entity().set(Position { x: 20, y: 30 }).is_a(base);

    let e3 = world
        .entity()
        .set(Position { x: 10, y: 20 })
        .set(Velocity { x: 3, y: 4 });

    let q = world.new_query::<(&mut Position, &Velocity)>();

    q.each_entity_shared(&world, |_e, (p, v)| {
        p.x += v.x;
        p.y += v.y;
    });

    { let p = e1.get_ref::<&Position>().unwrap();
        assert_eq!(p.x, 11);
        assert_eq!(p.y, 22);
    };
    { let p = e2.get_ref::<&Position>().unwrap();
        assert_eq!(p.x, 21);
        assert_eq!(p.y, 32);
    };
    { let p = e3.get_ref::<&Position>().unwrap();
        assert_eq!(p.x, 13);
        assert_eq!(p.y, 24);
    };
}

#[test]
fn each_optional() {
    let world = World::new();

    let e1 = world
        .entity()
        .set(Position { x: 10, y: 20 })
        .set(Velocity { x: 1, y: 2 })
        .set(Mass { value: 1 });
    let e2 = world
        .entity()
        .set(Position { x: 30, y: 40 })
        .set(Velocity { x: 3, y: 4 })
        .set(Mass { value: 1 });
    let e3 = world.entity().set(Position { x: 50, y: 60 });
    let e4 = world.entity().set(Position { x: 70, y: 80 });

    let q = world.new_query::<(&mut Position, Option<&Velocity>, Option<&Mass>)>();

    q.each_shared(&world, |(p, v, m)| {
        if let (Some(v), Some(m)) = (v, m) {
            p.x += v.x * m.value;
            p.y += v.y * m.value;
        } else {
            p.x += 1;
            p.y += 1;
        }
    });

    { let p = e1.get_ref::<&Position>().unwrap();
        assert_eq!(p.x, 11);
        assert_eq!(p.y, 22);
    };
    { let p = e2.get_ref::<&Position>().unwrap();
        assert_eq!(p.x, 33);
        assert_eq!(p.y, 44);
    };
    { let p = e3.get_ref::<&Position>().unwrap();
        assert_eq!(p.x, 51);
        assert_eq!(p.y, 61);
    };
    { let p = e4.get_ref::<&Position>().unwrap();
        assert_eq!(p.x, 71);
        assert_eq!(p.y, 81);
    };
}

#[test]
fn each_sparse() {
    let world = World::new();

    world.component::<Position>().add_trait::<flecs::Sparse>();

    let entity = world
        .entity()
        .set(Position { x: 10, y: 20 })
        .set(Velocity { x: 1, y: 2 });

    let q = world.query::<(&mut Position, &Velocity)>().build();

    q.each_shared(&world, |(p, v)| {
        p.x += v.x;
        p.y += v.y;
    });

    { let p = entity.get_ref::<&Position>().unwrap();
        assert_eq!(p.x, 11);
        assert_eq!(p.y, 22);
    };
}

#[test]
fn each_sparse_w_with() {
    let world = World::new();

    world.component::<Position>().add_trait::<flecs::Sparse>();

    let entity = world
        .entity()
        .set(Position { x: 10, y: 20 })
        .set(Velocity { x: 1, y: 2 });

    let q = world
        .query::<()>()
        .with(&mut Position::id())
        .with(&Velocity::id())
        .build();

    q.each_iter(|it, row, ()| {
        let mut p = it.field_at_mut::<Position>(0, row);
        let v = it.field_at::<Velocity>(1, row);
        p.x += v.x;
        p.y += v.y;
    });

    { let p = entity.get_ref::<&Position>().unwrap();
        assert_eq!(p.x, 11);
        assert_eq!(p.y, 22);
    };
}

#[test]
fn each_sparse_many() {
    let mut world = World::new();

    world.component::<Position>().add_trait::<flecs::Sparse>();

    let mut entities = Vec::new();
    for i in 0..2000i32 {
        entities.push(
            world
                .entity()
                .set(Position {
                    x: 10 + i,
                    y: 20 + i,
                })
                .set(Velocity { x: i, y: i })
                .id(),
        );
    }

    let q = world.query::<(&mut Position, &Velocity)>().build();
    q.each_exclusive(&mut world, |(p, v)| {
        p.x += v.x;
        p.y += v.y;
    });

    for i in 0..2000i32 {
        let e = world.entity_from_id(entities[i as usize]);
        { let p = e.get_ref::<&Position>().unwrap();
            assert_eq!(p.x, 10 + i * 2);
            assert_eq!(p.y, 20 + i * 2);
        };
        { let v = e.get_ref::<&Velocity>().unwrap();
            assert_eq!(v.x, i);
            assert_eq!(v.y, i);
        };
    }
}

#[test]
fn each_dont_fragment() {
    let world = World::new();

    world
        .component::<Position>()
        .add_trait::<flecs::DontFragment>();

    let entity = world
        .entity()
        .set(Position { x: 10, y: 20 })
        .set(Velocity { x: 1, y: 2 });

    let q = world.new_query::<(&mut Position, &Velocity)>();
    q.each_shared(&world, |(p, v)| {
        p.x += v.x;
        p.y += v.y;
    });

    { let p = entity.get_ref::<&Position>().unwrap();
        assert_eq!(p.x, 11);
        assert_eq!(p.y, 22);
    };
}

#[test]
fn each_dont_fragment_w_with() {
    let world = World::new();

    world
        .component::<Position>()
        .add_trait::<flecs::DontFragment>();

    let entity = world
        .entity()
        .set(Position { x: 10, y: 20 })
        .set(Velocity { x: 1, y: 2 });

    let q = world
        .query::<()>()
        .with(&mut Position::id())
        .with(&Velocity::id())
        .build();

    q.each_iter(|it, row, ()| {
        let mut p = it.field_at_mut::<Position>(0, row);
        let v = it.field_at::<Velocity>(1, row);
        p.x += v.x;
        p.y += v.y;
    });

    { let p = entity.get_ref::<&Position>().unwrap();
        assert_eq!(p.x, 11);
        assert_eq!(p.y, 22);
    };
}

#[test]
fn each_dont_fragment_many() {
    let mut world = World::new();

    world
        .component::<Position>()
        .add_trait::<flecs::DontFragment>();

    let mut entities = Vec::new();
    for i in 0..2000i32 {
        entities.push(
            world
                .entity()
                .set(Position {
                    x: 10 + i,
                    y: 20 + i,
                })
                .set(Velocity { x: i, y: i })
                .id(),
        );
    }

    let q = world.new_query::<(&mut Position, &Velocity)>();
    q.each_exclusive(&mut world, |(p, v)| {
        p.x += v.x;
        p.y += v.y;
    });

    for i in 0..2000i32 {
        let e = world.entity_from_id(entities[i as usize]);
        { let p = e.get_ref::<&Position>().unwrap();
            assert_eq!(p.x, 10 + i * 2);
            assert_eq!(p.y, 20 + i * 2);
        };
        { let v = e.get_ref::<&Velocity>().unwrap();
            assert_eq!(v.x, i);
            assert_eq!(v.y, i);
        };
    }
}

#[test]
fn each_dont_fragment_add() {
    let world = World::new();

    world
        .component::<Velocity>()
        .add_trait::<flecs::DontFragment>();

    let entity = world
        .entity()
        .set(Position { x: 10, y: 20 })
        .set(Velocity { x: 1, y: 2 });

    let q = world.new_query::<&mut Position>();

    q.each_entity_shared(&world, |e, p| {
        e.add(Velocity::id());
        assert!(e.has(Velocity::id()));
        p.x += 1;
        p.y += 2;
    });

    { let p = entity.get_ref::<&Position>().unwrap();
        assert_eq!(p.x, 11);
        assert_eq!(p.y, 22);
    };
    assert!(entity.has(Velocity::id()));
}

#[test]
fn each_dont_fragment_add_remove() {
    let world = World::new();

    world
        .component::<Velocity>()
        .add_trait::<flecs::DontFragment>();

    let entity = world
        .entity()
        .set(Position { x: 10, y: 20 })
        .set(Velocity { x: 1, y: 2 });

    let q = world.new_query::<&mut Position>();

    q.each_entity_shared(&world, |e, p| {
        e.add(Velocity::id());
        assert!(e.has(Velocity::id()));
        e.remove(Velocity::id());
        assert!(!e.has(Velocity::id()));
        p.x += 1;
        p.y += 2;
    });

    { let p = entity.get_ref::<&Position>().unwrap();
        assert_eq!(p.x, 11);
        assert_eq!(p.y, 22);
    };
    assert!(!entity.has(Velocity::id()));
}

#[test]
fn each_dont_fragment_set() {
    let world = World::new();

    world
        .component::<Velocity>()
        .add_trait::<flecs::DontFragment>();

    let entity = world
        .entity()
        .set(Position { x: 10, y: 20 })
        .set(Velocity { x: 1, y: 2 });

    let q = world.new_query::<&mut Position>();

    q.each_entity_shared(&world, |e, p| {
        e.set(Velocity { x: 1, y: 2 });
        assert!(e.has(Velocity::id()));
        { let v = e.get_ref::<&Velocity>().unwrap();
            assert_eq!(v.x, 1);
            assert_eq!(v.y, 2);
        };
        p.x += 1;
        p.y += 2;
    });

    { let p = entity.get_ref::<&Position>().unwrap();
        assert_eq!(p.x, 11);
        assert_eq!(p.y, 22);
    };
    { let v = entity.get_ref::<&Velocity>().unwrap();
        assert_eq!(v.x, 1);
        assert_eq!(v.y, 2);
    };
}

#[test]
fn each_generic() {
    // Generic lambdas in Rust are normal closures that infer types.
    let mut world = World::new();
    let q = world.new_query::<(&mut Position, &Velocity, Option<&Mass>)>();
    q.each_exclusive(&mut world, |(_p, _v, _m)| {});
    q.each_entity_exclusive(&mut world, |_e, (_p, _v, _m)| {});
}

#[test]
fn signature() {
    let world = World::new();

    let entity = world
        .entity()
        .set(Position { x: 10, y: 20 })
        .set(Velocity { x: 1, y: 2 });

    let q = world
        .query::<()>()
        .expr("Position, Velocity")
        .build()
        .expect("valid query expression");

    q.run_shared(&world, |mut it| {
        while it.next() {
            let mut p = it.field_mut::<Position>(0);
            let v = it.field::<Velocity>(1);
            for i in it.iter() {
                p[i].x += v[i].x;
                p[i].y += v[i].y;
            }
        }
    });

    { let p = entity.get_ref::<&Position>().unwrap();
        assert_eq!(p.x, 11);
        assert_eq!(p.y, 22);
    };
}

#[test]
fn signature_const() {
    let world = World::new();

    let entity = world
        .entity()
        .set(Position { x: 10, y: 20 })
        .set(Velocity { x: 1, y: 2 });

    let q = world
        .query::<()>()
        .expr("Position, [in] Velocity")
        .build()
        .expect("valid query expression");

    q.run_shared(&world, |mut it| {
        while it.next() {
            let mut p = it.field_mut::<Position>(0);
            let v = it.field::<Velocity>(1);
            for i in it.iter() {
                p[i].x += v[i].x;
                p[i].y += v[i].y;
            }
        }
    });

    { let p = entity.get_ref::<&Position>().unwrap();
        assert_eq!(p.x, 11);
        assert_eq!(p.y, 22);
    };
}

#[test]
fn signature_shared() {
    let world = World::new();

    world
        .component::<Position>()
        .add((flecs::OnInstantiate::ID, flecs::Inherit::ID));
    world
        .component::<Velocity>()
        .add((flecs::OnInstantiate::ID, flecs::Inherit::ID));

    let base = world.entity().set(Velocity { x: 1, y: 2 });

    let e1 = world.entity().set(Position { x: 10, y: 20 }).is_a(base);

    let e2 = world
        .entity()
        .set(Position { x: 10, y: 20 })
        .set(Velocity { x: 3, y: 4 });

    let q = world
        .query::<()>()
        .expr("Position, [in] Velocity(self|up IsA)")
        .build()
        .expect("valid query expression");

    q.run_shared(&world, |mut it| {
        while it.next() {
            let mut p = it.field_mut::<Position>(0);
            let v = it.field::<Velocity>(1);

            if !it.is_self(1) {
                for i in it.iter() {
                    p[i].x += v[0].x;
                    p[i].y += v[0].y;
                }
            } else {
                for i in it.iter() {
                    p[i].x += v[i].x;
                    p[i].y += v[i].y;
                }
            }
        }
    });

    { let p = e1.get_ref::<&Position>().unwrap();
        assert_eq!(p.x, 11);
        assert_eq!(p.y, 22);
    };
    { let p = e2.get_ref::<&Position>().unwrap();
        assert_eq!(p.x, 13);
        assert_eq!(p.y, 24);
    };
}

#[test]
fn signature_optional() {
    let world = World::new();

    let e1 = world
        .entity()
        .set(Position { x: 10, y: 20 })
        .set(Velocity { x: 1, y: 2 })
        .set(Mass { value: 1 });
    let e2 = world
        .entity()
        .set(Position { x: 30, y: 40 })
        .set(Velocity { x: 3, y: 4 })
        .set(Mass { value: 1 });
    let e3 = world.entity().set(Position { x: 50, y: 60 });
    let e4 = world.entity().set(Position { x: 70, y: 80 });

    let q = world
        .query::<()>()
        .expr("Position, ?Velocity, ?Mass")
        .build()
        .expect("valid query expression");

    q.run_shared(&world, |mut it| {
        while it.next() {
            let mut p = it.field_mut::<Position>(0);
            let v = it.field::<Velocity>(1);
            let m = it.field::<Mass>(2);

            if it.is_set(1) && it.is_set(2) {
                for i in it.iter() {
                    p[i].x += v[i].x * m[i].value;
                    p[i].y += v[i].y * m[i].value;
                }
            } else {
                for i in it.iter() {
                    p[i].x += 1;
                    p[i].y += 1;
                }
            }
        }
    });

    { let p = e1.get_ref::<&Position>().unwrap();
        assert_eq!(p.x, 11);
        assert_eq!(p.y, 22);
    };
    { let p = e2.get_ref::<&Position>().unwrap();
        assert_eq!(p.x, 33);
        assert_eq!(p.y, 44);
    };
    { let p = e3.get_ref::<&Position>().unwrap();
        assert_eq!(p.x, 51);
        assert_eq!(p.y, 61);
    };
    { let p = e4.get_ref::<&Position>().unwrap();
        assert_eq!(p.x, 71);
        assert_eq!(p.y, 81);
    };
}

#[test]
fn query_single_pair() {
    #[derive(Component)]
    struct Pair;

    let world = World::new();

    world.entity().add((Pair::id(), Position::id()));
    let e2 = world.entity().add((Pair::id(), Velocity::id()));

    let q = world
        .query::<()>()
        .expr("(Pair, Velocity)")
        .build()
        .expect("valid query expression");

    let mut table_count = 0i32;
    let mut entity_count = 0i32;

    q.run_shared(&world, |mut it| {
        while it.next() {
            table_count += 1;
            for i in it.iter() {
                assert_eq!(it.entity_id(i), e2.id());
                entity_count += 1;
            }
        }
    });

    assert_eq!(table_count, 1);
    assert_eq!(entity_count, 1);
}

#[test]
fn sort_by() {
    let mut world = World::new();

    world.entity().set(Position { x: 1, y: 0 });
    world.entity().set(Position { x: 6, y: 0 });
    world.entity().set(Position { x: 2, y: 0 });
    world.entity().set(Position { x: 5, y: 0 });
    world.entity().set(Position { x: 4, y: 0 });

    let q = world
        .query::<&Position>()
        .order_by::<Position>(|_e1, p1: &Position, _e2, p2: &Position| {
            (p1.x > p2.x) as i32 - (p1.x < p2.x) as i32
        })
        .build();

    q.run_exclusive(&mut world, |mut it| {
        while it.next() {
            let p = it.field::<Position>(0);
            assert_eq!(it.count(), 5);
            assert_eq!(p[0].x, 1);
            assert_eq!(p[1].x, 2);
            assert_eq!(p[2].x, 4);
            assert_eq!(p[3].x, 5);
            assert_eq!(p[4].x, 6);
        }
    });
}

#[test]
fn expr_w_template() {
    #[derive(Component, Debug)]
    struct Template<T: Send + Sync + 'static> {
        x: T,
        y: T,
    }

    let mut world = World::new();

    let comp = world.component::<Template<i32>>();
    assert!(comp.name().contains("Template"));

    let mut count = 0;
    let q = world
        .query::<&Position>()
        .with(&Template::<i32>::id())
        .build();

    world
        .entity()
        .set(Position { x: 10, y: 20 })
        .set(Template::<i32> { x: 30, y: 40 });

    q.each_entity_exclusive(&mut world, |e, p| {
        assert_eq!(p.x, 10);
        assert_eq!(p.y, 20);
        { let t = e.get_ref::<&Template<i32>>().unwrap();
            assert_eq!(t.x, 30);
            assert_eq!(t.y, 40);
        };
        count += 1;
    });

    assert_eq!(count, 1);
}

#[test]
fn query_type_w_template() {
    #[derive(Component, Debug)]
    struct Template2<T: Send + Sync + 'static> {
        x: T,
        y: T,
    }

    let mut world = World::new();

    let comp = world.component::<Template2<i32>>();
    assert!(comp.name().contains("Template2"));

    let mut count = 0;
    let q = world.new_query::<(&Position, &Template2<i32>)>();

    world
        .entity()
        .set(Position { x: 10, y: 20 })
        .set(Template2::<i32> { x: 30, y: 40 });

    q.each_entity_exclusive(&mut world, |_e, (p, t)| {
        assert_eq!(p.x, 10);
        assert_eq!(p.y, 20);
        assert_eq!(t.x, 30);
        assert_eq!(t.y, 40);
        count += 1;
    });

    assert_eq!(count, 1);
}

#[test]
fn compare_term_id() {
    let world = World::new();

    let mut count = 0;
    let e = world.entity().add(Tag::id());

    let q = world.query::<()>().with(Tag::id()).build();

    q.run_shared(&world, |mut it| {
        while it.next() {
            assert_eq!(*it.id(0).entity_view().id(), *world.component_id::<Tag>());
            assert_eq!(it.entity_id(0_usize), e.id());
        }
        count += 1;
    });

    assert_eq!(count, 1);
}

#[test]
#[should_panic]
#[cfg_attr(not(debug_assertions), ignore)]
fn test_no_defer_each() {
    #[derive(Component)]
    struct Value {
        _value: i32,
    }

    let world = World::new();
    let _guard = FlecsPanicAbortGuard::install();

    world.entity().add(Tag::id()).set(Value { _value: 10 });

    let q = world.query::<&Value>().with(Tag::id()).build();

    q.each_entity_shared(&world, |e, _v| {
        e.remove(Tag::id());
    });
}

#[test]
#[should_panic]
#[cfg_attr(not(debug_assertions), ignore)]
fn test_no_defer_iter() {
    #[derive(Component)]
    struct Value {
        _value: i32,
    }

    let mut world = World::new();
    let _guard = FlecsPanicAbortGuard::install();

    world.entity().add(Tag::id()).set(Value { _value: 10 });

    let q = world.query::<&Value>().with(Tag::id()).build();

    q.run_exclusive(&mut world, |mut it| {
        while it.next() {
            for i in it.iter() {
                let e = it.get_entity(i).unwrap();
                e.remove(Tag::id());
            }
        }
    });
}

#[test]
fn pair_to_str() {
    let world = World::new();

    let q = world
        .query::<()>()
        .with(Position::id())
        .with(Velocity::id())
        .with((Eats::id(), Apples::id()))
        .build();

    assert_eq!(
        q.to_string(),
        "Position($this), Velocity($this), Eats($this,Apples)"
    );
}

#[test]
fn oper_not_to_str() {
    let world = World::new();

    let q = world
        .query::<()>()
        .with(Position::id())
        .with(Velocity::id())
        .not()
        .build();

    assert_eq!(q.to_string(), "Position($this), !Velocity($this)");
}

#[test]
fn oper_optional_to_str() {
    let world = World::new();

    let q = world
        .query::<()>()
        .with(Position::id())
        .with(Velocity::id())
        .optional()
        .build();

    assert_eq!(q.to_string(), "Position($this), ?Velocity($this)");
}

#[test]
fn oper_or_to_str() {
    let world = World::new();

    let q = world
        .query::<()>()
        .with(Position::id())
        .or()
        .with(Velocity::id())
        .build();

    assert_eq!(q.to_string(), "Position($this) || Velocity($this)");
}

#[test]
fn iter_pair_type() {
    #[derive(Component, Default)]
    struct EatsData2 {
        amount: i32,
    }
    #[derive(Component)]
    struct ApplesTag2;
    #[derive(Component)]
    struct PearsTag2;

    let world = World::new();

    let e1 = world
        .entity()
        .set_pair::<EatsData2, ApplesTag2>(EatsData2 { amount: 10 });
    world
        .entity()
        .set_pair::<EatsData2, PearsTag2>(EatsData2 { amount: 20 });

    let q = world.new_query::<&(EatsData2, ApplesTag2)>();

    let mut count = 0;
    q.run_shared(&world, |mut it| {
        while it.next() {
            let a = it.field::<EatsData2>(0);
            assert_eq!(it.count(), 1);
            assert_eq!(a[0].amount, 10);
            assert_eq!(it.entity_id(0_usize), e1.id());
            count += 1;
        }
    });

    assert_eq!(count, 1);

    { let v = e1.get_ref::<&(EatsData2, ApplesTag2)>().unwrap();
        assert_eq!(v.amount, 10);
    };
}

#[test]
fn term_pair_type() {
    #[derive(Component, Default)]
    struct EatsData3 {
        amount: i32,
    }
    #[derive(Component)]
    struct ApplesTag3;
    #[derive(Component)]
    struct PearsTag3;

    let world = World::new();

    let e1 = world
        .entity()
        .set_pair::<EatsData3, ApplesTag3>(EatsData3 { amount: 10 });
    world
        .entity()
        .set_pair::<EatsData3, PearsTag3>(EatsData3 { amount: 20 });

    let q = world.new_query::<&mut (EatsData3, ApplesTag3)>();

    let mut count = 0;
    q.run_shared(&world, |mut it| {
        while it.next() {
            let mut a = it.field_mut::<EatsData3>(0);
            assert_eq!(it.count(), 1);
            assert_eq!(a[0].amount, 10);
            assert_eq!(it.entity_id(0_usize), e1.id());
            a[0].amount += 1;
            count += 1;
        }
    });

    assert_eq!(count, 1);

    { let v = e1.get_ref::<&(EatsData3, ApplesTag3)>().unwrap();
        assert_eq!(v.amount, 11);
    };
}

#[test]
fn iter_no_comps_1_comp() {
    let mut world = World::new();

    world.entity().add(Position::id());
    world.entity().add(Position::id());
    world.entity().add(Position::id()).add(Velocity::id());
    world.entity().add(Velocity::id());

    let q = world.new_query::<&Position>();

    let mut count = 0;
    q.run_exclusive(&mut world, |mut it| {
        while it.next() {
            count += it.count();
        }
    });

    assert_eq!(count, 3);
}

#[test]
fn iter_no_comps_2_comps() {
    let mut world = World::new();

    world.entity().add(Velocity::id());
    world.entity().add(Position::id());
    world.entity().add(Position::id()).add(Velocity::id());
    world.entity().add(Position::id()).add(Velocity::id());

    let q = world.new_query::<(&Position, &Velocity)>();

    let mut count = 0;
    q.run_exclusive(&mut world, |mut it| {
        while it.next() {
            count += it.count();
        }
    });

    assert_eq!(count, 2);
}

#[test]
fn iter_no_comps_no_comps() {
    let mut world = World::new();

    world.entity().add(Velocity::id());
    world.entity().add(Position::id());
    world.entity().add(Position::id()).add(Velocity::id());
    world.entity().add(Position::id()).add(Velocity::id());

    let q = world.query::<()>().with(Position::id()).build();

    let mut count = 0;
    q.run_exclusive(&mut world, |mut it| {
        while it.next() {
            count += it.count();
        }
    });

    assert_eq!(count, 3);
}

#[test]
fn each_pair_object() {
    #[derive(Component)]
    struct Event {
        value: &'static str,
    }
    #[derive(Component)]
    struct Begin;
    #[derive(Component)]
    struct End;

    let world = World::new();

    let e1 = world
        .entity()
        .set_pair::<Event, Begin>(Event { value: "Big Bang" })
        .set_pair::<Event, End>(Event {
            value: "Heat Death",
        });

    let q = world.new_query::<(&(Event, Begin), &(Event, End))>();

    let mut count = 0;
    q.each_entity_shared(&world, |e, (b_e, e_e)| {
        assert_eq!(e, e1);
        assert_eq!(b_e.value, "Big Bang");
        assert_eq!(e_e.value, "Heat Death");
        count += 1;
    });

    assert_eq!(count, 1);
}

#[test]
fn iter_pair_object() {
    #[derive(Component)]
    struct EventVal {
        value: &'static str,
    }
    #[derive(Component)]
    struct BeginEvt;
    #[derive(Component)]
    struct EndEvt;

    let world = World::new();

    let e1 = world
        .entity()
        .set_pair::<EventVal, BeginEvt>(EventVal { value: "Big Bang" })
        .set_pair::<EventVal, EndEvt>(EventVal {
            value: "Heat Death",
        });

    let q = world.new_query::<(&(EventVal, BeginEvt), &(EventVal, EndEvt))>();

    let mut count = 0;
    q.run_shared(&world, |mut it| {
        while it.next() {
            let b_e = it.field::<EventVal>(0);
            let e_e = it.field::<EventVal>(1);
            for i in it.iter() {
                assert_eq!(it.entity_id(i), e1.id());
                assert_eq!(b_e[i].value, "Big Bang");
                assert_eq!(e_e[i].value, "Heat Death");
                count += 1;
            }
        }
    });

    assert_eq!(count, 1);
}

#[test]
fn iter_query_in_system() {
    thread_local! { static COUNT: core::cell::Cell<i32> = const { core::cell::Cell::new(0) }; }

    let mut world = World::new();

    world.entity().add(Position::id()).add(Velocity::id());

    let q = world.new_query::<&Velocity>();

    world.system::<&Position>().each_entity(move |_e1, _| {
        q.each_entity(|_e2, _| {});
        COUNT.with(|c| c.set(c.get() + 1));
    });

    world.progress();

    COUNT.with(|c| assert_eq!(c.get(), 1));
}

#[test]
fn optional_pair_term() {
    #[derive(Component, Default)]
    struct Tag0;

    #[derive(Component, Default)]
    struct PairRel {
        x: f32,
        y: f32,
    }

    #[derive(Component)]
    struct PairTgt;

    let mut world = World::new();

    world
        .entity()
        .add(Tag0::id())
        .set_pair::<PairRel, PairTgt>(PairRel { x: 1.0, y: 2.0 });
    world.entity().add(Tag0::id());

    let mut with_pair = 0i32;
    let mut without_pair = 0i32;

    let q = world
        .query::<Option<&(PairRel, PairTgt)>>()
        .with(Tag0::id())
        .build();

    q.each_exclusive(&mut world, |p| {
        if let Some(p) = p {
            with_pair += 1;
            assert!((p.x - 1.0f32).abs() < f32::EPSILON);
            assert!((p.y - 2.0f32).abs() < f32::EPSILON);
        } else {
            without_pair += 1;
        }
    });

    assert_eq!(with_pair, 1);
    assert_eq!(without_pair, 1);
}

#[test]
fn iter_targets() {
    let world = World::new();

    let likes = world.entity();
    let pizza = world.entity();
    let salad = world.entity();
    let alice = world.entity().add((likes, pizza)).add((likes, salad));

    let q = world
        .query::<()>()
        .with((likes, id::<flecs::Any>()))
        .build();

    let mut count = 0;
    let mut tgt_count = 0;

    q.each_iter(|it, row, ()| {
        let e = it.entity(row);
        assert_eq!(e.id(), alice.id());

        it.targets(0, |tgt| {
            if tgt_count == 0 {
                assert_eq!(tgt.id(), pizza.id());
            }
            if tgt_count == 1 {
                assert_eq!(tgt.id(), salad.id());
            }
            tgt_count += 1;
        });

        count += 1;
    });

    assert_eq!(count, 1);
    assert_eq!(tgt_count, 2);
}

#[test]
fn iter_targets_second_field() {
    let world = World::new();

    let likes = world.entity();
    let pizza = world.entity();
    let salad = world.entity();
    let alice = world
        .entity()
        .add(Position::id())
        .add((likes, pizza))
        .add((likes, salad));

    let q = world
        .query::<()>()
        .with(Position::id())
        .with((likes, id::<flecs::Any>()))
        .build();

    let mut count = 0;
    let mut tgt_count = 0;

    q.each_iter(|it, row, ()| {
        let e = it.entity(row);
        assert_eq!(e.id(), alice.id());

        it.targets(1, |tgt| {
            if tgt_count == 0 {
                assert_eq!(tgt.id(), pizza.id());
            }
            if tgt_count == 1 {
                assert_eq!(tgt.id(), salad.id());
            }
            tgt_count += 1;
        });

        count += 1;
    });

    assert_eq!(count, 1);
    assert_eq!(tgt_count, 2);
}

#[test]
#[should_panic]
#[cfg_attr(not(debug_assertions), ignore)]
fn iter_targets_field_out_of_range() {
    let world = World::new();
    let _guard = FlecsPanicAbortGuard::install();

    let likes = world.entity();
    let pizza = world.entity();
    let salad = world.entity();
    let alice = world.entity().add((likes, pizza)).add((likes, salad));

    let q = world
        .query::<()>()
        .with((likes, id::<flecs::Any>()))
        .build();

    q.each_iter(|it, row, ()| {
        let e = it.entity(row);
        assert_eq!(e.id(), alice.id());
        it.targets(1, |_tgt| {});
    });
}

#[test]
#[should_panic]
#[cfg_attr(not(debug_assertions), ignore)]
fn iter_targets_field_not_a_pair() {
    let world = World::new();
    let _guard = FlecsPanicAbortGuard::install();

    let likes = world.entity();
    let pizza = world.entity();
    let salad = world.entity();
    let alice = world
        .entity()
        .add(Position::id())
        .add((likes, pizza))
        .add((likes, salad));

    let q = world.query::<()>().with(Position::id()).build();

    q.each_iter(|it, row, ()| {
        let e = it.entity(row);
        assert_eq!(e.id(), alice.id());
        it.targets(1, |_tgt| {});
    });
}

#[test]
#[should_panic]
#[cfg_attr(not(debug_assertions), ignore)]
fn iter_targets_field_not_set() {
    let world = World::new();
    let _guard = FlecsPanicAbortGuard::install();

    let likes = world.entity();
    let alice = world.entity().add(Position::id());

    let q = world
        .query::<()>()
        .with(Position::id())
        .with((likes, id::<flecs::Any>()))
        .optional()
        .build();

    q.each_iter(|it, row, ()| {
        let e = it.entity(row);
        assert_eq!(e.id(), alice.id());
        assert!(!it.is_set(1));
        it.targets(1, |_tgt| {});
    });
}

#[test]
fn copy_operators() {
    let world = World::new();

    let q = world.query::<()>().with(Position::id()).build();

    let q_copy_ctor = q.clone();
    let q_copy_assign = q.clone();

    assert_eq!(q_copy_ctor.query_ptr(), q.query_ptr());
    assert_eq!(q_copy_assign.query_ptr(), q.query_ptr());
}

#[test]
fn optional_singleton() {
    let mut world = World::new();

    // Mark Mass as Singleton so optional query yields exactly 1 result (C++: component.add(flecs::Singleton))
    world.component::<Mass>().add_trait::<flecs::Singleton>();

    let mut invoked = 0;

    world.new_query::<Option<&Mass>>().each_exclusive(&mut world, |_m| {
        invoked += 1;
    });

    assert_eq!(invoked, 1);

    world.set(Mass { value: 5 });

    world.new_query::<Option<&Mass>>().each_exclusive(&mut world, |m| {
        if let Some(mass) = m {
            assert_eq!(mass.value, 5);
        }
        invoked += 1;
    });

    assert_eq!(invoked, 2);
}

#[derive(Component)]
struct TestComponent {
    a: i32,
    b: i32,
    c: i32,
    d: i32,
    e: i32,
}

impl Module for TestComponent {
    fn module(world: &World) {
        world.module::<TestComponent>("TestComponent");
        world.set(TestComponent {
            a: 10,
            b: 20,
            c: 30,
            d: 40,
            e: 50,
        });
    }
}

#[test]
fn optional_module() {
    let mut world = World::new();

    world.import::<TestComponent>();

    let mut invoked = 0;

    world.new_query::<Option<&TestComponent>>().each_exclusive(&mut world, |ptr| {
        let ptr = ptr.unwrap();
        assert_eq!(ptr.a, 10);
        assert_eq!(ptr.b, 20);
        assert_eq!(ptr.c, 30);
        assert_eq!(ptr.d, 40);
        assert_eq!(ptr.e, 50);
        invoked += 1;
    });

    assert_eq!(invoked, 1);
}

#[test]
fn has_entity() {
    let world = World::new();

    let e1 = world.entity().set(Position { x: 1, y: 2 });
    let e2 = world.entity().set(Velocity { x: 3, y: 4 });

    let q = world.new_query::<&Position>();

    assert!(q.has(e1));
    assert!(!q.has(e2));
}

#[test]
fn has_table() {
    let world = World::new();

    let e1 = world.entity().set(Position { x: 1, y: 2 });
    let e2 = world.entity().set(Velocity { x: 3, y: 4 });

    let q = world.new_query::<&Position>();

    assert!(q.has_table(e1.table().unwrap()));
    assert!(!q.has_table(e2.table().unwrap()));
}

#[test]
fn has_range() {
    let world = World::new();

    let e1 = world.entity().set(Position { x: 1, y: 2 });
    let e2 = world.entity().set(Velocity { x: 3, y: 4 });

    let q = world.new_query::<&Position>();

    assert!(q.has_table_range(e1.range().unwrap()));
    assert!(!q.has_table_range(e2.range().unwrap()));
}

#[test]
fn empty_tables_each() {
    let world = World::new();

    let e1 = world
        .entity()
        .set(Position { x: 10, y: 20 })
        .set(Velocity { x: 1, y: 2 });

    let e2 = world
        .entity()
        .set(Position { x: 20, y: 30 })
        .set(Velocity { x: 2, y: 3 });

    e2.add(Tag::id());
    e2.remove(Tag::id());

    let q = world
        .query::<(&mut Position, &Velocity)>()
        .query_flags(QueryFlags::MatchEmptyTables)
        .build();

    q.each_shared(&world, |(p, v)| {
        p.x += v.x;
        p.y += v.y;
    });

    { let p = e1.get_ref::<&Position>().unwrap();
        assert_eq!(p.x, 11);
        assert_eq!(p.y, 22);
    };
    { let p = e2.get_ref::<&Position>().unwrap();
        assert_eq!(p.x, 22);
        assert_eq!(p.y, 33);
    };
}

#[test]
fn empty_tables_each_w_entity() {
    let world = World::new();

    let e1 = world
        .entity()
        .set(Position { x: 10, y: 20 })
        .set(Velocity { x: 1, y: 2 });

    let e2 = world
        .entity()
        .set(Position { x: 20, y: 30 })
        .set(Velocity { x: 2, y: 3 });

    e2.add(Tag::id());
    e2.remove(Tag::id());

    let q = world
        .query::<(&mut Position, &Velocity)>()
        .query_flags(QueryFlags::MatchEmptyTables)
        .build();

    q.each_entity_shared(&world, |_e, (p, v)| {
        p.x += v.x;
        p.y += v.y;
    });

    { let p = e1.get_ref::<&Position>().unwrap();
        assert_eq!(p.x, 11);
        assert_eq!(p.y, 22);
    };
    { let p = e2.get_ref::<&Position>().unwrap();
        assert_eq!(p.x, 22);
        assert_eq!(p.y, 33);
    };
}

#[test]
fn empty_tables_each_w_iter() {
    let world = World::new();

    let e1 = world
        .entity()
        .set(Position { x: 10, y: 20 })
        .set(Velocity { x: 1, y: 2 });

    let e2 = world
        .entity()
        .set(Position { x: 20, y: 30 })
        .set(Velocity { x: 2, y: 3 });

    e2.add(Tag::id());
    e2.remove(Tag::id());

    let q = world
        .query::<(&mut Position, &Velocity)>()
        .query_flags(QueryFlags::MatchEmptyTables)
        .build();

    q.each_iter(|_it, _row, (p, v)| {
        p.x += v.x;
        p.y += v.y;
    });

    { let p = e1.get_ref::<&Position>().unwrap();
        assert_eq!(p.x, 11);
        assert_eq!(p.y, 22);
    };
    { let p = e2.get_ref::<&Position>().unwrap();
        assert_eq!(p.x, 22);
        assert_eq!(p.y, 33);
    };
}

#[test]
fn iter_entities() {
    let world = World::new();

    let e1 = world.entity().set(Position { x: 10, y: 20 });
    let e2 = world.entity().set(Position { x: 10, y: 20 });
    let e3 = world.entity().set(Position { x: 10, y: 20 });

    let q = world.new_query::<&Position>();

    q.run_shared(&world, |mut it| {
        while it.next() {
            let entities = it.entities();
            assert_eq!(entities.len(), 3);
            assert_eq!(entities[0], e1);
            assert_eq!(entities[1], e2);
            assert_eq!(entities[2], e3);
        }
    });
}

#[test]
fn iter_get_pair_w_id() {
    let world = World::new();

    let rel = world.entity();
    let tgt = world.entity();
    let e = world.entity().add((rel, tgt));

    let q = world
        .query::<()>()
        .with((rel, id::<flecs::Wildcard>()))
        .build();

    let mut count = 0;
    q.each_iter(|it, i, ()| {
        assert!(it.id(0).is_pair());
        assert_eq!(it.id(0).first_id().id(), rel.id());
        assert_eq!(it.id(0).second_id().id(), tgt.id());
        assert_eq!(it.entity_id(i), e.id());
        count += 1;
    });

    assert_eq!(count, 1);
}

#[test]
fn query_from_entity() {
    let world = World::new();

    let qe = world.entity();
    let q1 = world
        .query::<(&Position, &Velocity)>()
        .entity_set(qe)
        .build();

    world.entity().add(Position::id());
    let e2 = world.entity().add(Position::id()).add(Velocity::id());

    let mut count = 0;
    q1.each_entity_shared(&world, |e, (_p, _v)| {
        count += 1;
        assert_eq!(e.id(), e2.id());
    });
    assert_eq!(count, 1);

    let q2 = world.query_from(qe);
    q2.each_iter(|it, i, ()| {
        count += 1;
        assert_eq!(it.entity_id(i), e2.id());
    });
    assert_eq!(count, 2);
}

#[test]
fn query_from_entity_name() {
    let world = World::new();

    let q1 = world.query_named::<(&Position, &Velocity)>("qe").build();

    world.entity().add(Position::id());
    let e2 = world.entity().add(Position::id()).add(Velocity::id());

    let mut count = 0;
    q1.each_entity_shared(&world, |e, (_p, _v)| {
        count += 1;
        assert_eq!(e.id(), e2.id());
    });
    assert_eq!(count, 1);

    let q2 = world.query_from(world.lookup("qe"));
    q2.each_iter(|it, i, ()| {
        count += 1;
        assert_eq!(it.entity_id(i), e2.id());
    });
    assert_eq!(count, 2);
}

#[test]
fn run_w_iter_fini() {
    let mut world = World::new();

    let q = world.new_query::<&Position>();

    let mut count = 0;
    q.run_exclusive(&mut world, |it| {
        it.fini(); // explicit fini when not consuming via it.next() (C++: it.fini())
        count += 1;
    });

    assert_eq!(count, 1);
}

#[test]
fn run_w_iter_fini_interrupt() {
    let mut world = World::new();

    let _e1 = world.entity().set(Position { x: 10, y: 20 }).add(Tag::id());
    let _e2 = world.entity().set(Position { x: 10, y: 20 });
    let _e3 = world.entity().set(Position { x: 10, y: 20 });

    let q = world.new_query::<&Position>();

    let mut count = 0;
    q.run_exclusive(&mut world, |mut it| {
        if it.next() {
            count += 1;
        }
        it.fini();
    });

    assert_eq!(count, 1);
}

#[test]
fn run_w_iter_fini_empty() {
    let mut world = World::new();

    let q = world.new_query::<&Position>();

    let mut count = 0;
    q.run_exclusive(&mut world, |it| {
        it.fini();
        count += 1;
    });

    assert_eq!(count, 1);
}

#[test]
fn run_w_iter_fini_no_query() {
    let mut world = World::new();

    let q = world.query::<()>().build();

    let mut count = 0;
    q.run_exclusive(&mut world, |it| {
        it.fini();
        count += 1;
    });

    assert_eq!(count, 1);
}

#[test]
fn add_to_match_from_staged_query() {
    let world = World::new();

    world.component::<Position>();
    world.component::<Velocity>();

    let e = world.entity().add(Position::id());

    let stage = world.stage(0);

    world.readonly_begin(false);

    world.new_query::<&Position>().each_entity_shared(&world, |entity, _pos| {
        let entity = entity.mut_current_stage(stage);
        entity.add(Velocity::id());
        assert!(!entity.has(Velocity::id()));
    });

    world.readonly_end();

    assert!(e.has(Position::id()));
    assert!(e.has(Velocity::id()));
}

#[test]
fn add_to_match_from_staged_query_readonly_threaded() {
    let world = World::new();

    world.component::<Position>();
    world.component::<Velocity>();

    let e = world.entity().add(Position::id());

    let stage = world.stage(0);

    world.readonly_begin(true);

    stage.new_query::<&Position>().each_entity_shared(&world, |entity, _pos| {
        entity.add(Velocity::id());
        assert!(!entity.has(Velocity::id()));
    });

    world.readonly_end();

    assert!(e.has(Position::id()));
    assert!(e.has(Velocity::id()));
}

#[test]
fn iter_type() {
    let mut world = World::new();

    world.entity().add(Position::id());
    world.entity().add(Position::id()).add(Velocity::id());

    let q = world.new_query::<&Position>();

    q.run_exclusive(&mut world, |mut it| {
        while it.next() {
            let arch = it.archetype();
            assert!(arch.is_some());
            let table = it.table();
            assert!(table.unwrap().has(Position::id()));
        }
    });
}

#[test]
fn pair_with_variable_src() {
    let mut world = World::new();

    #[derive(Component)]
    struct Rel;

    #[derive(Component)]
    struct ThisComp {
        x: i32,
    }

    #[derive(Component)]
    struct OtherComp {
        x: i32,
    }

    let other = world.entity().set(OtherComp { x: 10 });

    for i in 0..3i32 {
        world
            .entity()
            .set(ThisComp { x: i })
            .add((Rel::id(), other));
    }

    let q = world
        .query::<()>()
        .with(Rel::id())
        .set_second("$other")
        .with(ThisComp::id())
        .with(OtherComp::id())
        .set_src("$other")
        .build();

    let mut is_present: usize = 0;
    q.run_exclusive(&mut world, |mut it| {
        while it.next() {
            let this_comps = it.field::<ThisComp>(1);
            let other_comps = it.field::<OtherComp>(2);
            for i in it.iter() {
                is_present |= 1 << this_comps[i].x;
                assert_eq!(other_comps[0].x, 10);
            }
        }
    });

    assert_eq!(is_present, 7);
}

#[test]
fn pair_with_variable_src_no_row_fields() {
    let mut world = World::new();

    #[derive(Component)]
    struct Rel2;

    #[derive(Component)]
    struct ThisComp2 {
        x: i32,
    }

    #[derive(Component)]
    struct OtherComp2 {
        x: i32,
    }

    let other = world.entity().set(OtherComp2 { x: 0 });

    world.entity().set(OtherComp2 { x: 1 });

    for i in 0..3i32 {
        world
            .entity()
            .set(ThisComp2 { x: i })
            .add((Rel2::id(), other));
    }

    let q = world
        .query::<()>()
        .with(Rel2::id())
        .set_second("$other")
        .with(ThisComp2::id())
        .with(OtherComp2::id())
        .set_src("$other")
        .build();

    let mut is_present: usize = 0;
    q.run_exclusive(&mut world, |mut it| {
        while it.next() {
            let this_comps = it.field::<ThisComp2>(1);
            let other_comps = it.field::<OtherComp2>(2);
            for i in it.iter() {
                is_present |= 1 << this_comps[i].x;
                assert_eq!(other_comps[0].x, 0);
            }
        }
    });

    assert_eq!(is_present, 7);
}

#[test]
fn is_true() {
    let world = World::new();

    let _e = world.entity().set(Position { x: 1, y: 2 });

    let q1 = world.new_query::<&Position>();
    let q2 = world.new_query::<&Velocity>();

    assert!(q1.is_true());
    assert!(!q2.is_true());
}

#[test]
fn count() {
    let world = World::new();

    let _e1 = world.entity().set(Position { x: 1, y: 2 });
    let _e2 = world.entity().set(Position { x: 3, y: 4 });
    let _e3 = world.entity().set(Position { x: 5, y: 6 });

    let q = world.new_query::<&Position>();

    assert_eq!(q.count(), 3);
}

#[test]
fn with_no_components() {
    let world = World::new();

    let _e1 = world.entity();
    let _e2 = world.entity();

    let q = world.query::<()>().build();

    assert!(q.count() >= 0);
}

#[test]
fn with_tag_only() {
    let world = World::new();

    world.entity().add(Tag::id());
    world.entity().add(Tag::id());

    let q = world.query::<()>().with(Tag::id()).build();

    assert_eq!(q.count(), 2);
}

#[test]
fn find_multiple_results() {
    let world = World::new();

    world.entity().set(Position { x: 5, y: 5 });
    world.entity().set(Position { x: 5, y: 5 });

    let q = world.new_query::<&Position>();

    let result = q.find(|p| p.x == 5);
    assert!(result.is_some());
}

#[test]
fn on_empty_world() {
    let world = World::new();

    let q = world.new_query::<&Position>();

    assert_eq!(q.count(), 0);
    assert!(!q.is_true());
}

#[test]
fn with_multiple_types() {
    let world = World::new();

    let _e = world
        .entity()
        .set(Position { x: 1, y: 2 })
        .set(Velocity { x: 3, y: 4 })
        .set(Mass { value: 5 });

    let q = world.new_query::<(&Position, &Velocity, &Mass)>();

    assert_eq!(q.count(), 1);
}

#[test]
fn each_mutable() {
    let world = World::new();

    let e = world.entity().set(Position { x: 10, y: 20 });

    let q = world.new_query::<&Position>();

    q.each_shared(&world, |p| {
        assert_eq!(p.x, 10);
        assert_eq!(p.y, 20);
    });

    { let p = e.get_ref::<&Position>().unwrap();
        assert_eq!(p.x, 10);
        assert_eq!(p.y, 20);
    };
}

#[test]
fn with_renamed_entity() {
    let world = World::new();

    let e = world.entity_named("MyEntity").set(Position { x: 1, y: 2 });

    let q = world.new_query::<&Position>();

    assert_eq!(q.count(), 1);
    assert_eq!(e.name(), "MyEntity");
}

#[test]
fn with_child_entity() {
    let world = World::new();

    let parent = world.entity_named("Parent");
    let _child = world
        .entity_named("Child")
        .child_of(parent)
        .set(Position { x: 1, y: 2 });

    let q = world.new_query::<&Position>();

    assert_eq!(q.count(), 1);
}

#[test]
fn with_prefab() {
    let world = World::new();

    let prefab = world.prefab().set(Position { x: 1, y: 2 });
    let _instance = world.entity().is_a(prefab);

    let q = world.new_query::<&Position>();

    // Prefabs are excluded from normal queries by default
    assert_eq!(q.count(), 1);
}

#[test]
fn first_empty() {
    let world = World::new();

    let q = world.new_query::<&Position>();

    // try_first_entity() returns None when no results
    assert!(q.try_first_entity().is_none());
}

#[test]
fn changed_detection() {
    let mut world = World::new();

    let _e = world.entity().set(Position { x: 1, y: 2 });

    let q = world.query::<&Position>().detect_changes().build();

    assert!(q.is_changed());

    q.each_exclusive(&mut world, |_p| {});

    assert!(!q.is_changed());
}

#[test]
fn iter_basic() {
    let mut world = World::new();

    world.entity().set(Position { x: 1, y: 2 });

    let q = world.new_query::<&Position>();

    let mut found = false;
    q.run_exclusive(&mut world, |mut it| {
        while it.next() {
            found = true;
            assert_eq!(it.count(), 1);
        }
    });

    assert!(found);
}

#[test]
fn with_default_component() {
    let world = World::new();

    let _e = world.entity().add(Tag::id());

    let q = world.query::<()>().with(Tag::id()).build();

    assert_eq!(q.count(), 1);
}

#[test]
fn modified_after_creation() {
    let world = World::new();

    let q = world.new_query::<&Position>();

    world.entity().set(Position { x: 1, y: 2 });

    assert_eq!(q.count(), 1);
}

#[test]
fn each_with_control_flow() {
    let mut world = World::new();

    let _e1 = world.entity().set(Position { x: 1, y: 2 });
    let _e2 = world.entity().set(Position { x: 3, y: 4 });

    let q = world.new_query::<&Position>();

    let mut count = 0;
    q.each_entity_exclusive(&mut world, |_e, _p| {
        count += 1;
    });

    assert_eq!(count, 2);
}

#[test]
fn result_consistency() {
    let world = World::new();

    let _e1 = world.entity().set(Position { x: 1, y: 2 });
    let _e2 = world.entity().set(Position { x: 3, y: 4 });

    let q = world.new_query::<&Position>();

    let count1 = q.count();
    let count2 = q.count();

    assert_eq!(count1, count2);
    assert_eq!(count1, 2);
}

#[test]
fn with_complex_tuple() {
    let mut world = World::new();

    let _e = world
        .entity()
        .set(Position { x: 1, y: 2 })
        .set(Velocity { x: 3, y: 4 });

    let q = world.new_query::<(&Position, &Velocity)>();

    q.each_exclusive(&mut world, |(p, v)| {
        assert_eq!(p.x + v.x, 4);
        assert_eq!(p.y + v.y, 6);
    });
}

#[test]
fn first_single_result() {
    let world = World::new();

    let e = world.entity().set(Position { x: 1, y: 2 });

    let q = world.new_query::<&Position>();

    assert_eq!(q.first_entity(), e);
}

#[test]
fn is_true_non_empty() {
    let world = World::new();

    let _e = world.entity().set(Position { x: 1, y: 2 });

    let q = world.new_query::<&Position>();

    assert!(q.is_true());
}

#[test]
fn multiple_entity_iteration() {
    let mut world = World::new();

    let _e1 = world.entity().set(Position { x: 1, y: 2 });
    let _e2 = world.entity().set(Position { x: 3, y: 4 });
    let _e3 = world.entity().set(Position { x: 5, y: 6 });

    let q = world.new_query::<&Position>();

    let mut entities = Vec::new();
    q.each_entity_exclusive(&mut world, |e, _p| {
        entities.push(e.id());
    });

    assert_eq!(entities.len(), 3);
}

#[test]
fn clone_consistency() {
    let world = World::new();

    let e = world.entity().set(Position { x: 1, y: 2 });

    let q1 = world.new_query::<&Position>();
    let q2 = q1.clone();

    assert_eq!(q1.count(), q2.count());
    assert_eq!(q1.first_entity(), e);
    assert_eq!(q2.first_entity(), e);
}

// ── sparse queries ──
// C++ `world.query<T...>()` returns flecs::sparse_query at compile time when
// all components are DontFragment; Rust uses an explicit
// `world.sparse_query::<T>()` since trait-based return-type dispatch is not
// possible. Query_sparse_query_type is covered by the explicit API.

#[derive(Component)]
#[flecs(traits(DontFragment, (OnInstantiate, Override)))]
struct PositionDfOr {
    x: i32,
    y: i32,
}

#[derive(Component)]
#[flecs(traits(DontFragment, (OnInstantiate, Override)))]
struct VelocityDfOr {
    x: i32,
    y: i32,
}

#[test]
fn sparse_query_each() {
    let world = World::new();

    let e1 = world
        .entity()
        .set(PositionDfOr { x: 10, y: 20 })
        .set(VelocityDfOr { x: 1, y: 2 });
    let e2 = world.entity().set(PositionDfOr { x: 30, y: 40 });
    let e3 = world.entity().set(VelocityDfOr { x: 3, y: 4 });

    let q = world.sparse_query::<(&mut PositionDfOr, &VelocityDfOr)>();

    let mut count = 0;
    q.each_entity(|e, (p, v)| {
        assert_eq!(e, e1);
        assert_eq!(p.x, 10);
        assert_eq!(p.y, 20);
        p.x += v.x;
        p.y += v.y;
        count += 1;
    });

    assert_eq!(count, 1);
    assert_eq!(q.count(), 1);

    { let p = e1.get_ref::<&PositionDfOr>().unwrap();
        assert_eq!(p.x, 11);
        assert_eq!(p.y, 22);
    };

    assert!(e2.is_alive());
    assert!(e3.is_alive());
}

#[test]
fn sparse_query_empty() {
    let world = World::new();

    world.component::<PositionDfOr>();
    world.component::<VelocityDfOr>();

    let q = world.sparse_query::<(&PositionDfOr, &VelocityDfOr)>();

    let mut count = 0;
    q.each(|(_p, _v)| {
        count += 1;
    });

    assert_eq!(count, 0);
    assert_eq!(q.count(), 0);
}

#[test]
fn sparse_query_recycled_entity() {
    let world = World::new();

    let e1 = world.entity().set(PositionDfOr { x: 10, y: 20 });
    e1.destruct();
    let e2 = world.entity().set(PositionDfOr { x: 30, y: 40 });
    assert!(e1 != e2);
    assert_eq!(*e1.id() as u32, *e2.id() as u32);

    let q = world.sparse_query::<(&PositionDfOr,)>();

    let mut count = 0;
    q.each_entity(|e, (p,)| {
        assert_eq!(e, e2);
        assert_eq!(p.x, 30);
        assert_eq!(p.y, 40);
        count += 1;
    });

    assert_eq!(count, 1);
}

#[test]
fn sparse_query_skip_prefab_disabled() {
    let world = World::new();

    let e = world.entity().set(PositionDfOr { x: 10, y: 20 });
    world
        .entity()
        .add(flecs::Prefab::ID)
        .set(PositionDfOr { x: 1, y: 2 });
    world
        .entity()
        .add(flecs::Disabled::ID)
        .set(PositionDfOr { x: 3, y: 4 });
    world
        .entity()
        .add(flecs::NotQueryable::ID)
        .set(PositionDfOr { x: 5, y: 6 });

    let q = world.sparse_query::<(&PositionDfOr,)>();

    let mut count = 0;
    q.each_entity(|ent, (pos,)| {
        assert_eq!(ent, e);
        assert_eq!(pos.x, 10);
        assert_eq!(pos.y, 20);
        count += 1;
    });

    assert_eq!(count, 1);
    assert_eq!(q.count(), 1);
}

#[test]
fn world_each_sparse() {
    let world = World::new();

    let e1 = world
        .entity()
        .set(PositionDfOr { x: 10, y: 20 })
        .set(VelocityDfOr { x: 1, y: 2 });
    world.entity().set(PositionDfOr { x: 30, y: 40 });

    let mut count = 0;
    world
        .sparse_query::<(&mut PositionDfOr, &VelocityDfOr)>()
        .each(|(p, v)| {
            assert_eq!(p.x, 10);
            assert_eq!(p.y, 20);
            assert_eq!(v.x, 1);
            assert_eq!(v.y, 2);
            p.x += v.x;
            p.y += v.y;
            count += 1;
        });

    assert_eq!(count, 1);

    { let p = e1.get_ref::<&PositionDfOr>().unwrap();
        assert_eq!(p.x, 11);
        assert_eq!(p.y, 22);
    };
}

#[test]
fn world_each_sparse_w_entity() {
    let world = World::new();

    let e1 = world.entity().set(PositionDfOr { x: 10, y: 20 });
    let e2 = world.entity().set(PositionDfOr { x: 30, y: 40 });

    let mut count = 0;
    world
        .sparse_query::<(&PositionDfOr,)>()
        .each_entity(|e, (p,)| {
            if e == e1 {
                assert_eq!(p.x, 10);
                assert_eq!(p.y, 20);
            } else {
                assert_eq!(e, e2);
                assert_eq!(p.x, 30);
                assert_eq!(p.y, 40);
            }
            count += 1;
        });

    assert_eq!(count, 2);
}

#[derive(Component)]
#[flecs(traits(DontFragment))]
struct PositionDf {
    x: i32,
    y: i32,
}

#[derive(Component)]
#[flecs(traits(DontFragment, (OnInstantiate, DontInherit)))]
struct PositionDfDi {
    x: i32,
    y: i32,
}

#[test]
fn sparse_query_type_no_on_instantiate() {
    let world = World::new();

    let e1 = world.entity().set(PositionDf { x: 10, y: 20 });
    let e2 = world.entity().set(PositionDf { x: 30, y: 40 });

    let q = world.sparse_query::<(&PositionDf,)>();

    let mut count = 0;
    q.each_entity(|e, (p,)| {
        if e == e1 {
            assert_eq!(p.x, 10);
            assert_eq!(p.y, 20);
        } else {
            assert_eq!(e, e2);
            assert_eq!(p.x, 30);
            assert_eq!(p.y, 40);
        }
        count += 1;
    });

    assert_eq!(count, 2);
}

#[test]
fn sparse_query_each_on_instantiate_override() {
    let world = World::new();

    let e1 = world.entity().set(PositionDfOr { x: 10, y: 20 });
    let e2 = world.entity().set(PositionDfOr { x: 30, y: 40 });
    let e3 = world.entity();

    let q = world.sparse_query::<(&mut PositionDfOr,)>();

    let mut count = 0;
    q.each_entity(|e, (p,)| {
        if e == e1 {
            assert_eq!(p.x, 10);
            assert_eq!(p.y, 20);
        } else {
            assert_eq!(e, e2);
            assert_eq!(p.x, 30);
            assert_eq!(p.y, 40);
        }
        p.x += 1;
        p.y += 1;
        count += 1;
    });

    assert_eq!(count, 2);
    assert_eq!(q.count(), 2);

    { let p = e1.get_ref::<&PositionDfOr>().unwrap();
        assert_eq!(p.x, 11);
        assert_eq!(p.y, 21);
    };
    { let p = e2.get_ref::<&PositionDfOr>().unwrap();
        assert_eq!(p.x, 31);
        assert_eq!(p.y, 41);
    };

    assert!(e3.is_alive());
}

#[test]
fn sparse_query_each_on_instantiate_dont_inherit() {
    let world = World::new();

    let e1 = world.entity().set(PositionDfDi { x: 10, y: 20 });
    let e2 = world.entity().set(PositionDfDi { x: 30, y: 40 });

    let q = world.sparse_query::<(&mut PositionDfDi,)>();

    let mut count = 0;
    q.each_entity(|e, (p,)| {
        if e == e1 {
            assert_eq!(p.x, 10);
            assert_eq!(p.y, 20);
        } else {
            assert_eq!(e, e2);
            assert_eq!(p.x, 30);
            assert_eq!(p.y, 40);
        }
        p.x += 1;
        p.y += 1;
        count += 1;
    });

    assert_eq!(count, 2);
    assert_eq!(q.count(), 2);

    { let p = e1.get_ref::<&PositionDfDi>().unwrap();
        assert_eq!(p.x, 11);
        assert_eq!(p.y, 21);
    };
    { let p = e2.get_ref::<&PositionDfDi>().unwrap();
        assert_eq!(p.x, 31);
        assert_eq!(p.y, 41);
    };
}

#[test]
fn sparse_query_each_on_instantiate_mixed_terms() {
    let world = World::new();

    let e1 = world
        .entity()
        .set(PositionDfOr { x: 10, y: 20 })
        .set(PositionDfDi { x: 1, y: 2 });
    let e2 = world.entity().set(PositionDfOr { x: 30, y: 40 });
    let e3 = world.entity().set(PositionDfDi { x: 3, y: 4 });

    let q = world.sparse_query::<(&mut PositionDfOr, &PositionDfDi)>();

    let mut count = 0;
    q.each_entity(|e, (p, d)| {
        assert_eq!(e, e1);
        assert_eq!(p.x, 10);
        assert_eq!(p.y, 20);
        p.x += d.x;
        p.y += d.y;
        count += 1;
    });

    assert_eq!(count, 1);
    assert_eq!(q.count(), 1);

    { let p = e1.get_ref::<&PositionDfOr>().unwrap();
        assert_eq!(p.x, 11);
        assert_eq!(p.y, 22);
    };

    assert!(e2.is_alive());
    assert!(e3.is_alive());
}

#[test]
fn sparse_query_dynamic_inherit_1_term() {
    let world = World::new();

    world
        .component::<PositionDfDi>()
        .add_trait::<(flecs::OnInstantiate, flecs::Inherit)>();

    let owner = world.entity().set(PositionDfDi { x: 10, y: 20 });
    let base = world.prefab().set(PositionDfDi { x: 1, y: 2 });
    let inst = world.entity().is_a(base);

    assert!(!inst.owns(PositionDfDi::id()));
    assert!(inst.has(PositionDfDi::id()));

    for cache_kind in [QueryCacheKind::None, QueryCacheKind::Auto] {
        let q = world
            .query::<&PositionDfDi>()
            .set_cache_kind(cache_kind)
            .build();

        let mut owner_count = 0;
        let mut inst_count = 0;
        q.each_entity_shared(&world, |e, p| {
            if e == owner {
                assert_eq!(p.x, 10);
                assert_eq!(p.y, 20);
                owner_count += 1;
            } else {
                assert_eq!(e, inst);
                assert_eq!(p.x, 1);
                assert_eq!(p.y, 2);
                inst_count += 1;
            }
        });

        assert_eq!(owner_count, 1);
        assert_eq!(inst_count, 1);
    }
}

#[test]
fn sparse_query_convert_to_query_1_term() {
    let world = World::new();

    let e1 = world.entity().set(PositionDfOr { x: 10, y: 20 });
    let e2 = world.entity().set(PositionDfOr { x: 30, y: 40 });

    let q = world.sparse_query::<(&PositionDfOr,)>().to_query();

    let mut count = 0;
    q.each_entity_shared(&world, |e, (p,)| {
        if e == e1 {
            assert_eq!(p.x, 10);
            assert_eq!(p.y, 20);
        } else {
            assert_eq!(e, e2);
            assert_eq!(p.x, 30);
            assert_eq!(p.y, 40);
        }
        count += 1;
    });

    assert_eq!(count, 2);
    assert_eq!(q.count(), 2);
}

#[test]
fn sparse_query_convert_to_query_3_terms() {
    let world = World::new();

    let e1 = world
        .entity()
        .set(PositionDfOr { x: 10, y: 20 })
        .set(VelocityDfOr { x: 1, y: 2 })
        .set(PositionDfDi { x: 3, y: 4 });
    world.entity().set(PositionDfOr { x: 30, y: 40 });

    let q = world
        .sparse_query::<(&PositionDfOr, &VelocityDfOr, &PositionDfDi)>()
        .to_query();

    let mut count = 0;
    q.each_entity_shared(&world, |e, (p, v, d)| {
        assert_eq!(e, e1);
        assert_eq!(p.x, 10);
        assert_eq!(p.y, 20);
        assert_eq!(v.x, 1);
        assert_eq!(v.y, 2);
        assert_eq!(d.x, 3);
        assert_eq!(d.y, 4);
        count += 1;
    });

    assert_eq!(count, 1);
    assert_eq!(q.count(), 1);
}

#[test]
fn dont_fragment_trait_registered() {
    let world = World::new();

    let c = world.component::<PositionDfOr>();
    assert!(c.has(flecs::DontFragment));
}

#[derive(Component)]
struct PositionSpecializedDf {
    #[allow(dead_code)]
    x: f32,
    #[allow(dead_code)]
    y: f32,
}

#[test]
fn dont_fragment_trait_specialized_registered() {
    let world = World::new();

    let c = world.component::<PositionSpecializedDf>();
    c.add_trait::<flecs::DontFragment>();
    assert!(c.has(flecs::DontFragment));
}

#[test]
fn each_dont_fragment_trait() {
    let world = World::new();

    let e1 = world.entity().set(PositionDfOr { x: 10, y: 20 });
    let e2 = world.entity().set(PositionDfOr { x: 30, y: 40 });

    let q = world.sparse_query::<(&mut PositionDfOr,)>();

    let mut count = 0;
    q.each_entity(|e, (p,)| {
        if e == e1 {
            assert_eq!(p.x, 10);
            assert_eq!(p.y, 20);
        } else if e == e2 {
            assert_eq!(p.x, 30);
            assert_eq!(p.y, 40);
        }
        p.x += 1;
        p.y += 1;
        count += 1;
    });

    assert_eq!(count, 2);

    { let p = e1.get_ref::<&PositionDfOr>().unwrap();
        assert_eq!(p.x, 11);
        assert_eq!(p.y, 21);
    };
    { let p = e2.get_ref::<&PositionDfOr>().unwrap();
        assert_eq!(p.x, 31);
        assert_eq!(p.y, 41);
    };
}

#[test]
fn each_dont_fragment_trait_mixed() {
    let world = World::new();

    let entity = world
        .entity()
        .set(PositionDfOr { x: 10, y: 20 })
        .set(Velocity { x: 1, y: 2 });

    let q = world.new_query::<(&mut PositionDfOr, &Velocity)>();

    q.each_shared(&world, |(p, v)| {
        p.x += v.x;
        p.y += v.y;
    });

    { let p = entity.get_ref::<&PositionDfOr>().unwrap();
        assert_eq!(p.x, 11);
        assert_eq!(p.y, 22);
    };
}

#[test]
fn each_dont_fragment_trait_shared() {
    let world = World::new();

    let parent = world.entity().set(Velocity { x: 1, y: 2 });
    let entity = world
        .entity()
        .child_of(parent)
        .set(PositionDfOr { x: 10, y: 20 });

    let q = world
        .query::<(&mut PositionDfOr, &Velocity)>()
        .term_at(1)
        .up_id(flecs::ChildOf::ID)
        .build();

    let mut count = 0;
    q.each_shared(&world, |(p, v)| {
        assert_eq!(v.x, 1);
        assert_eq!(v.y, 2);
        p.x += v.x;
        p.y += v.y;
        count += 1;
    });

    assert_eq!(count, 1);

    { let p = entity.get_ref::<&PositionDfOr>().unwrap();
        assert_eq!(p.x, 11);
        assert_eq!(p.y, 22);
    };
}

#[test]
fn each_optional_sparse() {
    let world = World::new();

    world.component::<Position>().add_trait::<flecs::Sparse>();

    let e1 = world
        .entity()
        .set(Velocity { x: 1, y: 2 })
        .set(Position { x: 10, y: 20 });
    let e2 = world.entity().set(Velocity { x: 3, y: 4 });

    let q = world.new_query::<(&Velocity, Option<&Position>)>();

    let mut count = 0;
    let mut with_p = 0;
    q.each_entity_shared(&world, |e, (v, p)| {
        if e == e1 {
            let p = p.unwrap();
            assert_eq!(p.x, 10);
            assert_eq!(p.y, 20);
            assert_eq!(v.x, 1);
            assert_eq!(v.y, 2);
            with_p += 1;
        } else {
            assert_eq!(e, e2);
            assert!(p.is_none());
            assert_eq!(v.x, 3);
            assert_eq!(v.y, 4);
        }
        count += 1;
    });

    assert_eq!(count, 2);
    assert_eq!(with_p, 1);
}

#[test]
fn sparse_query_dynamic_inherit_3_terms() {
    let world = World::new();

    world
        .component::<PositionDfDi>()
        .add_trait::<(flecs::OnInstantiate, flecs::Inherit)>();

    let owner = world
        .entity()
        .set(PositionDfDi { x: 10, y: 20 })
        .set(PositionDfOr { x: 30, y: 40 })
        .set(VelocityDfOr { x: 1, y: 2 });

    let base = world.prefab().set(PositionDfDi { x: 50, y: 60 });
    let inst = world
        .entity()
        .is_a(base)
        .set(PositionDfOr { x: 70, y: 80 })
        .set(VelocityDfOr { x: 3, y: 4 });

    assert!(!inst.owns(PositionDfDi::id()));
    assert!(inst.has(PositionDfDi::id()));

    for cache_kind in [QueryCacheKind::None, QueryCacheKind::Auto] {
        let q = world
            .query::<(&PositionDfDi, &PositionDfOr, &VelocityDfOr)>()
            .set_cache_kind(cache_kind)
            .build();

        let mut owner_count = 0;
        let mut inst_count = 0;
        q.each_entity_shared(&world, |e, (p, p2, v)| {
            if e == owner {
                assert_eq!(p.x, 10);
                assert_eq!(p.y, 20);
                assert_eq!(p2.x, 30);
                assert_eq!(p2.y, 40);
                assert_eq!(v.x, 1);
                assert_eq!(v.y, 2);
                owner_count += 1;
            } else {
                assert_eq!(e, inst);
                assert_eq!(p.x, 50);
                assert_eq!(p.y, 60);
                assert_eq!(p2.x, 70);
                assert_eq!(p2.y, 80);
                assert_eq!(v.x, 3);
                assert_eq!(v.y, 4);
                inst_count += 1;
            }
        });

        assert_eq!(owner_count, 1);
        assert_eq!(inst_count, 1);
    }
}

// C++ Query_sparse_query_dynamic_inherit_assert expects world.query<T>() to
// abort because the compile-time sparse query cannot match dynamically
// inherited components. Rust rejects compile-time declared Inherit policies
// at compile time (QueryTuple::IS_SPARSE_QUERY, see the compile_fail doctests
// on SparseQuery); policies added at runtime with add_trait are caught by the
// runtime assert in SparseQuery::assert_policies (debug builds), tested here.
#[test]
#[cfg(debug_assertions)]
fn sparse_query_dynamic_inherit_assert() {
    let world = World::new();

    world
        .component::<PositionDfDi>()
        .add_trait::<(flecs::OnInstantiate, flecs::Inherit)>();

    let result = std::panic::catch_unwind(core::panic::AssertUnwindSafe(|| {
        world.sparse_query::<(&PositionDfDi,)>();
    }));
    assert!(result.is_err());
}

#[test]
fn sparse_query_dynamic_dont_inherit() {
    let world = World::new();

    world
        .component::<PositionDf>()
        .add_trait::<(flecs::OnInstantiate, flecs::DontInherit)>();

    let e1 = world.entity().set(PositionDf { x: 10, y: 20 });
    let e2 = world.entity().set(PositionDf { x: 30, y: 40 });

    let q = world.sparse_query::<(&PositionDf,)>();

    let mut count = 0;
    q.each_entity(|e, (p,)| {
        if e == e1 {
            assert_eq!(p.x, 10);
            assert_eq!(p.y, 20);
        } else {
            assert_eq!(e, e2);
            assert_eq!(p.x, 30);
            assert_eq!(p.y, 40);
        }
        count += 1;
    });

    assert_eq!(count, 2);
    assert_eq!(q.count(), 2);
}
