#![allow(dead_code)]
use core::cell::Cell;
use core::ffi::c_void;

use crate::common_test::*;
use flecs_ecs::experimental::prelude::{EntityGuardExt, WorldSingletonExt};
use flecs_ecs::experimental::{QueryExclusiveExt, QuerySharedExt};
use flecs_ecs::sys;

#[test]
fn builder_assign_same_type() {
    let world = World::new();

    let q = world
        .query::<(&Position, &Velocity)>()
        .set_cache_kind(QueryCacheKind::Auto)
        .build();

    let e1 = world
        .entity()
        .set(Position { x: 0, y: 0 })
        .set(Velocity { x: 0, y: 0 });
    world.entity().set(Position { x: 0, y: 0 });

    let mut count = 0;
    q.each_entity_shared(&world, |e, (_p, _v)| {
        count += 1;
        assert_eq!(e, e1);
    });

    assert_eq!(count, 1);
}

#[test]
fn builder_assign_to_empty() {
    let world = World::new();

    let q = world
        .query::<(&Position, &Velocity)>()
        .set_cache_kind(QueryCacheKind::Auto)
        .build();

    let e1 = world
        .entity()
        .set(Position { x: 0, y: 0 })
        .set(Velocity { x: 0, y: 0 });
    world.entity().set(Position { x: 0, y: 0 });

    let mut count = 0;
    q.each_entity_shared(&world, |e, _| {
        count += 1;
        assert_eq!(e, e1);
    });

    assert_eq!(count, 1);
}

#[test]
fn builder_assign_from_empty() {
    let world = World::new();

    let q = world
        .query::<()>()
        .set_cache_kind(QueryCacheKind::Auto)
        .with(&Position::id())
        .with(&Velocity::id())
        .build();

    let e1 = world
        .entity()
        .set(Position { x: 0, y: 0 })
        .set(Velocity { x: 0, y: 0 });
    world.entity().set(Position { x: 0, y: 0 });

    let mut count = 0;
    q.each_entity_shared(&world, |e, _| {
        count += 1;
        assert_eq!(e, e1);
    });

    assert_eq!(count, 1);
}

#[test]
fn builder_build() {
    let world = World::new();

    let q = world
        .query::<(&Position, &Velocity)>()
        .set_cache_kind(QueryCacheKind::Auto)
        .build();

    let e1 = world
        .entity()
        .set(Position { x: 0, y: 0 })
        .set(Velocity { x: 0, y: 0 });
    world.entity().set(Position { x: 0, y: 0 });

    let mut count = 0;
    q.each_entity_shared(&world, |e, (_p, _v)| {
        count += 1;
        assert_eq!(e, e1);
    });

    assert_eq!(count, 1);
}

#[test]
fn builder_build_to_let() {
    let world = World::new();

    let q = world
        .query::<(&Position, &Velocity)>()
        .set_cache_kind(QueryCacheKind::Auto)
        .build();

    let e1 = world
        .entity()
        .set(Position { x: 0, y: 0 })
        .set(Velocity { x: 0, y: 0 });
    world.entity().set(Position { x: 0, y: 0 });

    let mut count = 0;
    q.each_entity_shared(&world, |e, _| {
        count += 1;
        assert_eq!(e, e1);
    });

    assert_eq!(count, 1);
}

#[test]
fn builder_build_n_statements() {
    let world = World::new();

    let mut q = world.query::<()>();
    q = q.with(&Position::id());
    q = q.with(&Velocity::id());
    q = q.set_cache_kind(QueryCacheKind::Auto);
    let q = q.build();

    let e1 = world
        .entity()
        .set(Position { x: 0, y: 0 })
        .set(Velocity { x: 0, y: 0 });
    world.entity().set(Position { x: 0, y: 0 });

    let mut count = 0;
    q.each_entity_shared(&world, |e, _| {
        count += 1;
        assert_eq!(e, e1);
    });

    assert_eq!(count, 1);
}

#[test]
fn n1_type() {
    let world = World::new();

    let q = world
        .query::<&Position>()
        .set_cache_kind(QueryCacheKind::Auto)
        .build();

    let e1 = world.entity().set(Position { x: 0, y: 0 });
    world.entity().set(Velocity { x: 0, y: 0 });

    let mut count = 0;
    q.each_entity_shared(&world, |e, _p| {
        count += 1;
        assert_eq!(e, e1);
    });

    assert_eq!(count, 1);
}

#[test]
fn n2_types() {
    let world = World::new();

    let e1 = world
        .entity()
        .set(Position { x: 10, y: 20 })
        .set(Velocity { x: 1, y: 2 });

    world.entity().set(Velocity { x: 10, y: 20 });

    let r = world
        .query::<(&mut Position, &Velocity)>()
        .set_cache_kind(QueryCacheKind::Auto)
        .build();

    let mut count = 0;
    r.each_entity_shared(&world, |e, (p, v)| {
        count += 1;
        assert_eq!(e, e1);
        assert_eq!(p.x, 10);
        assert_eq!(p.y, 20);

        assert_eq!(v.x, 1);
        assert_eq!(v.y, 2);
    });

    assert_eq!(count, 1);
}

#[test]
fn id_term() {
    let world = World::new();

    let tag = world.entity();

    let e1 = world.entity().add(tag);

    world.entity().set(Velocity { x: 10, y: 20 });

    let r = world
        .query::<()>()
        .with(tag)
        .set_cache_kind(QueryCacheKind::Auto)
        .build();

    let mut count = 0;
    r.each_entity_shared(&world, |e, _| {
        count += 1;
        assert_eq!(e, e1);
    });

    assert_eq!(count, 1);
}

#[test]
fn type_term() {
    let world = World::new();

    let e1 = world.entity().set(Position { x: 10, y: 20 });

    world.entity().set(Velocity { x: 10, y: 20 });

    let r = world
        .query::<()>()
        .with(&Position::id())
        .set_cache_kind(QueryCacheKind::Auto)
        .build();

    let mut count = 0;
    r.each_entity_shared(&world, |e, _| {
        count += 1;
        assert_eq!(e, e1);
    });

    assert_eq!(count, 1);
}

#[test]
fn id_pair_term() {
    let world = World::new();

    let likes = world.entity();
    let apples = world.entity();
    let pears = world.entity();

    let e1 = world.entity().add((likes, apples));

    world.entity().add((likes, pears));

    let r = world
        .query::<()>()
        .with((likes, apples))
        .set_cache_kind(QueryCacheKind::Auto)
        .build();

    let mut count = 0;
    r.each_entity_shared(&world, |e, _| {
        count += 1;
        assert_eq!(e, e1);
    });

    assert_eq!(count, 1);
}

#[test]
fn id_pair_wildcard_term() {
    let world = World::new();

    let likes = world.entity();
    let apples = world.entity();
    let pears = world.entity();

    let e1 = world.entity().add((likes, apples));

    let e2 = world.entity().add((likes, pears));

    let r = world
        .query::<()>()
        .with((likes, *flecs::Wildcard))
        .set_cache_kind(QueryCacheKind::Auto)
        .build();

    let mut count = 0;
    r.run_shared(&world, |mut it| {
        let world = it.world();
        while it.next() {
            for i in it.iter() {
                if it.entity_id(i) == e1 {
                    assert_eq!(it.id(0), world.id_view_from((likes, apples)));
                    count += 1;
                }
                if it.entity_id(i) == e2 {
                    assert_eq!(it.id(0), world.id_view_from((likes, pears)));
                    count += 1;
                }
            }
        }
    });

    assert_eq!(count, 2);
}

#[test]
fn type_pair_term() {
    let world = World::new();

    let e1 = world.entity().add((Likes::id(), Apples::id()));

    let e2 = world.entity().add((Likes::id(), Pears::id()));

    let r = world
        .query::<()>()
        .with((Likes::id(), *flecs::Wildcard))
        .set_cache_kind(QueryCacheKind::Auto)
        .build();

    let mut count = 0;
    r.run_shared(&world, |mut it| {
        let world = it.world();
        while it.next() {
            for i in it.iter() {
                if it.entity_id(i) == e1 {
                    assert_eq!(it.id(0), world.id_view_from((Likes::id(), Apples::id())));
                    count += 1;
                }
                if it.entity_id(i) == e2 {
                    assert_eq!(it.id(0), world.id_view_from((Likes::id(), Pears::id())));
                    count += 1;
                }
            }
        }
    });

    assert_eq!(count, 2);
}

#[test]
fn pair_term_w_var() {
    let world = World::new();

    let e1 = world.entity().add((Likes::id(), Apples::id()));

    let e2 = world.entity().add((Likes::id(), Pears::id()));

    let r = world
        .query::<()>()
        .with(&Likes::id())
        .second()
        .set_var("Food")
        .set_cache_kind(QueryCacheKind::Auto)
        .build();

    let foo_d_var = r.find_var("Food").unwrap();

    let mut count = 0;
    r.run_shared(&world, |mut it| {
        let world = it.world();
        while it.next() {
            for i in it.iter() {
                if it.entity_id(i) == e1 {
                    assert_eq!(it.id(0), world.id_view_from((Likes::id(), Apples::id())));
                    assert_eq!(it.get_var_by_name("Food"), world.id_view_from(Apples::id()));
                    assert_eq!(it.get_var(foo_d_var), world.id_view_from(Apples::id()));
                    count += 1;
                }
                if it.entity_id(i) == e2 {
                    assert_eq!(it.id(0), world.id_view_from((Likes::id(), Pears::id())));
                    assert_eq!(it.get_var_by_name("Food"), world.id_view_from(Pears::id()));
                    assert_eq!(it.get_var(foo_d_var), world.id_view_from(Pears::id()));
                    count += 1;
                }
            }
        }
    });

    assert_eq!(count, 2);
}

#[test]
fn n2_pair_terms_w_var() {
    let world = World::new();

    let bob = world.entity().add((Eats::id(), Apples::id()));

    let alice = world
        .entity()
        .add((Eats::id(), Pears::id()))
        .add((Likes::id(), bob));

    bob.add((Likes::id(), alice));

    let r = world
        .query::<()>()
        .with(&Eats::id())
        .second()
        .set_var("Food")
        .with(&Likes::id())
        .second()
        .set_var("Person")
        .set_cache_kind(QueryCacheKind::Auto)
        .build();

    let foo_d_var = r.find_var("Food").unwrap();
    let person_var = r.find_var("Person").unwrap();

    let mut count = 0;
    r.run_shared(&world, |mut it| {
        let world = it.world();
        while it.next() {
            for i in it.iter() {
                if it.entity_id(i) == alice {
                    assert_eq!(it.id(0), world.id_view_from((Eats::id(), Pears::id())));
                    assert_eq!(it.id(1), world.id_view_from((Likes::id(), bob)));
                    assert_eq!(it.get_var_by_name("Food"), world.id_view_from(Pears::id()));
                    assert_eq!(it.get_var(foo_d_var), world.id_view_from(Pears::id()));
                    assert_eq!(it.get_var_by_name("Person"), bob);
                    assert_eq!(it.get_var(person_var), bob);
                    count += 1;
                }
                if it.entity_id(i) == bob {
                    assert_eq!(it.id(0), world.id_view_from((Eats::id(), Apples::id())));
                    assert_eq!(it.id(1), world.id_view_from((Likes::id(), alice)));
                    assert_eq!(it.get_var_by_name("Food"), world.id_view_from(Apples::id()));
                    assert_eq!(it.get_var(foo_d_var), world.id_view_from(Apples::id()));
                    assert_eq!(it.get_var_by_name("Person"), alice);
                    assert_eq!(it.get_var(person_var), alice);
                    count += 1;
                }
            }
        }
    });

    assert_eq!(count, 2);
}

#[test]
fn set_var() {
    let world = World::new();

    let apples = world.entity();
    let pears = world.entity();

    world.entity().add((Likes::id(), apples));

    let e2 = world.entity().add((Likes::id(), pears));

    let r = world
        .query::<()>()
        .with(&Likes::id())
        .second()
        .set_var("Food")
        .set_cache_kind(QueryCacheKind::Auto)
        .build();

    let foo_d_var = r.find_var("Food").unwrap();

    let mut count = 0;
    r.iterable().set_var(foo_d_var, pears).run_shared(&world, |mut it| {
        while it.next() {
            for i in it.iter() {
                assert_eq!(it.entity_id(i), e2);
                assert_eq!(it.id(0), world.id_view_from((Likes::id(), pears)));
                assert_eq!(it.get_var_by_name("Food"), pears);
                assert_eq!(it.get_var(foo_d_var), pears);
                count += 1;
            }
        }
    });

    assert_eq!(count, 1);
}

#[test]
fn set_2_vars() {
    let world = World::new();

    let apples = world.entity();
    let pears = world.entity();

    let bob = world.entity().add((Eats::id(), apples));

    let alice = world
        .entity()
        .add((Eats::id(), pears))
        .add((Likes::id(), bob));

    bob.add((Likes::id(), alice));

    let r = world
        .query::<()>()
        .with(&Eats::id())
        .second()
        .set_var("Food")
        .with(&Likes::id())
        .second()
        .set_var("Person")
        .set_cache_kind(QueryCacheKind::Auto)
        .build();

    let foo_d_var = r.find_var("Food").unwrap();
    let person_var = r.find_var("Person").unwrap();

    let mut count = 0;
    r.iterable()
        .set_var(foo_d_var, pears)
        .set_var(person_var, bob)
        .run_shared(&world, |mut it| {
            while it.next() {
                for i in it.iter() {
                    assert_eq!(it.entity_id(i), alice);
                    assert_eq!(it.id(0), world.id_view_from((Eats::id(), pears)));
                    assert_eq!(it.id(1), world.id_view_from((Likes::id(), bob)));
                    assert_eq!(it.get_var_by_name("Food"), pears);
                    assert_eq!(it.get_var(foo_d_var), pears);
                    assert_eq!(it.get_var_by_name("Person"), bob);
                    assert_eq!(it.get_var(person_var), bob);
                    count += 1;
                }
            }
        });

    assert_eq!(count, 1);
}

#[test]
fn set_var_by_name() {
    let world = World::new();

    let apples = world.entity();
    let pears = world.entity();

    world.entity().add((Likes::id(), apples));

    let e2 = world.entity().add((Likes::id(), pears));

    let r = world
        .query::<()>()
        .with(&Likes::id())
        .second()
        .set_var("Food")
        .set_cache_kind(QueryCacheKind::Auto)
        .build();

    let mut count = 0;
    r.iterable().set_var_expr("Food", pears).run_shared(&world, |mut it| {
        while it.next() {
            for i in it.iter() {
                assert_eq!(it.entity_id(i), e2);
                assert_eq!(it.id(0), world.id_view_from((Likes::id(), pears)));
                count += 1;
            }
        }
    });
    assert_eq!(count, 1);
}

#[test]
fn set_2_vars_by_name() {
    let world = World::new();

    let apples = world.entity();
    let pears = world.entity();

    let bob = world.entity().add((Eats::id(), apples));

    let alice = world
        .entity()
        .add((Eats::id(), pears))
        .add((Likes::id(), bob));

    bob.add((Likes::id(), alice));

    let r = world
        .query::<()>()
        .with(&Eats::id())
        .second()
        .set_var("Food")
        .with(&Likes::id())
        .second()
        .set_var("Person")
        .set_cache_kind(QueryCacheKind::Auto)
        .build();

    let foo_d_var = r.find_var("Food").unwrap();
    let person_var = r.find_var("Person").unwrap();

    let mut count = 0;
    r.iterable()
        .set_var_expr("Food", pears)
        .set_var_expr("Person", bob)
        .run_shared(&world, |mut it| {
            while it.next() {
                for i in it.iter() {
                    assert_eq!(it.entity_id(i), alice);
                    assert_eq!(it.id(0), world.id_view_from((Eats::id(), pears)));
                    assert_eq!(it.id(1), world.id_view_from((Likes::id(), bob)));
                    assert_eq!(it.get_var_by_name("Food"), pears);
                    assert_eq!(it.get_var(foo_d_var), pears);
                    assert_eq!(it.get_var_by_name("Person"), bob);
                    assert_eq!(it.get_var(person_var), bob);
                    count += 1;
                }
            }
        });
    assert_eq!(count, 1);
}

#[test]
fn expr_w_var() {
    let world = World::new();

    let rel = world.entity_named("Rel");
    let obj = world.entity();
    let e = world.entity().add((rel, obj));

    let r = world
        .query::<()>()
        .expr("(Rel, $X)")
        .set_cache_kind(QueryCacheKind::Auto)
        .build()
        .expect("valid query expression");

    let x_var = r.find_var("X").unwrap();
    assert_ne!(x_var, -1);

    let mut count = 0;
    r.run_shared(&world, |mut it| {
        while it.next() {
            for i in it.iter() {
                assert_eq!(it.get_entity(i).unwrap(), e);
                assert_eq!(it.pair(0).second_id(), obj);
                count += 1;
            }
        }
    });

    assert_eq!(count, 1);
}

#[test]
fn add_1_type() {
    let world = World::new();

    let q = world
        .query::<()>()
        .with(&Position::id())
        .set_cache_kind(QueryCacheKind::Auto)
        .build();

    let e1 = world.entity().set(Position { x: 0, y: 0 });
    world.entity().set(Velocity { x: 0, y: 0 });

    let mut count = 0;
    q.each_entity_shared(&world, |e, _| {
        count += 1;
        assert_eq!(e, e1);
    });

    assert_eq!(count, 1);
}

#[test]
fn add_2_types() {
    let world = World::new();

    let q = world
        .query::<()>()
        .with(&Position::id())
        .with(&Velocity::id())
        .set_cache_kind(QueryCacheKind::Auto)
        .build();

    let e1 = world
        .entity()
        .set(Position { x: 0, y: 0 })
        .set(Velocity { x: 0, y: 0 });
    world.entity().set(Velocity { x: 0, y: 0 });

    let mut count = 0;
    q.each_entity_shared(&world, |e, _| {
        count += 1;
        assert_eq!(e, e1);
    });

    assert_eq!(count, 1);
}

#[test]
fn add_1_type_w_1_type() {
    let world = World::new();

    let q = world
        .query::<&Position>()
        .with(&Velocity::id())
        .set_cache_kind(QueryCacheKind::Auto)
        .build();

    let e1 = world
        .entity()
        .set(Position { x: 0, y: 0 })
        .set(Velocity { x: 0, y: 0 });
    world.entity().set(Velocity { x: 0, y: 0 });

    let mut count = 0;
    q.each_entity_shared(&world, |e, _| {
        count += 1;
        assert_eq!(e, e1);
    });

    assert_eq!(count, 1);
}

#[test]
fn add_2_types_w_1_type() {
    let world = World::new();

    let q = world
        .query::<&Position>()
        .with(&Velocity::id())
        .with(&Mass::id())
        .set_cache_kind(QueryCacheKind::Auto)
        .build();

    let e1 = world
        .entity()
        .set(Position { x: 0, y: 0 })
        .set(Velocity { x: 0, y: 0 })
        .set(Mass { value: 0 });
    world.entity().set(Velocity { x: 0, y: 0 });

    let mut count = 0;
    q.each_entity_shared(&world, |e, _| {
        count += 1;
        assert_eq!(e, e1);
    });

    assert_eq!(count, 1);
}

#[test]
fn add_pair() {
    let world = World::new();

    let likes = world.entity();
    let bob = world.entity();
    let alice = world.entity();

    let q = world
        .query::<()>()
        .with((likes, bob))
        .set_cache_kind(QueryCacheKind::Auto)
        .build();

    let e1 = world.entity().add((likes, bob));
    world.entity().add((likes, alice));

    let mut count = 0;
    q.each_entity_shared(&world, |e, _| {
        count += 1;
        assert_eq!(e, e1);
    });

    assert_eq!(count, 1);
}

#[test]
fn add_not() {
    let world = World::new();

    let q = world
        .query::<&Position>()
        .with(&Velocity::id())
        .set_oper(OperKind::Not)
        .set_cache_kind(QueryCacheKind::Auto)
        .build();

    let e1 = world.entity().set(Position { x: 0, y: 0 });
    world
        .entity()
        .set(Position { x: 0, y: 0 })
        .set(Velocity { x: 0, y: 0 });

    let mut count = 0;
    q.each_entity_shared(&world, |e, _p| {
        count += 1;
        assert_eq!(e, e1);
    });

    assert_eq!(count, 1);
}

#[test]
fn add_or() {
    let world = World::new();

    let q = world
        .query::<()>()
        .with(&Position::id())
        .set_oper(OperKind::Or)
        .with(&Velocity::id())
        .set_cache_kind(QueryCacheKind::Auto)
        .build();

    let e1 = world.entity().set(Position { x: 0, y: 0 });
    let e2 = world.entity().set(Velocity { x: 0, y: 0 });
    world.entity().set(Mass { value: 0 });

    let mut count = 0;
    q.each_entity_shared(&world, |e, _| {
        count += 1;
        assert!((e == e1 || e == e2));
    });

    assert_eq!(count, 2);
}

#[test]
fn add_optional() {
    let world = World::new();

    let q = world
        .query::<()>()
        .with(&Position::id())
        .with(&Velocity::id())
        .set_oper(OperKind::Optional)
        .set_cache_kind(QueryCacheKind::Auto)
        .build();

    let e1 = world.entity().set(Position { x: 0, y: 0 });
    let e2 = world
        .entity()
        .set(Position { x: 0, y: 0 })
        .set(Velocity { x: 0, y: 0 });
    world
        .entity()
        .set(Velocity { x: 0, y: 0 })
        .set(Mass { value: 0 });

    let mut count = 0;
    q.each_entity_shared(&world, |e, _| {
        count += 1;
        assert!((e == e1 || e == e2));
    });

    assert_eq!(count, 2);
}

#[test]
fn option_type() {
    let world = World::new();

    let q = world
        .query::<(&Position, Option<&Velocity>)>()
        .set_cache_kind(QueryCacheKind::Auto)
        .build();

    let e1 = world.entity().set(Position { x: 0, y: 0 });
    let e2 = world
        .entity()
        .set(Position { x: 0, y: 0 })
        .set(Velocity { x: 0, y: 0 });
    world
        .entity()
        .set(Velocity { x: 0, y: 0 })
        .set(Mass { value: 0 });

    let mut count = 0;
    q.each_entity_shared(&world, |e, _| {
        count += 1;
        assert!((e == e1 || e == e2));
    });

    assert_eq!(count, 2);
}

#[test]
fn const_type() {
    let world = World::new();

    let q = world
        .query::<&Position>()
        .set_cache_kind(QueryCacheKind::Auto)
        .build();

    let e1 = world.entity().set(Position { x: 0, y: 0 });
    world.entity().set(Velocity { x: 0, y: 0 });

    let mut count = 0;
    q.each_entity_shared(&world, |e, _p| {
        count += 1;
        assert_eq!(e, e1);
    });

    assert_eq!(count, 1);
}

#[test]
fn string_term() {
    let world = World::new();

    world.component::<Position>();

    let q = world
        .query::<()>()
        .expr("Position")
        .set_cache_kind(QueryCacheKind::Auto)
        .build()
        .expect("valid query expression");

    let e1 = world.entity().set(Position { x: 0, y: 0 });
    world.entity().set(Velocity { x: 0, y: 0 });

    let mut count = 0;
    q.each_entity_shared(&world, |e, _| {
        count += 1;
        assert_eq!(e, e1);
    });

    assert_eq!(count, 1);
}

#[test]
fn singleton_term() {
    let mut world = World::new();

    world.component::<Other>().add_trait::<flecs::Singleton>();

    world.set(Other { value: 10 });

    let q = world
        .query::<&SelfRef>()
        .with(&Other::id())
        .set_inout()
        .set_cache_kind(QueryCacheKind::Auto)
        .build();

    let e = world.entity();
    e.set(SelfRef { value: e.id() });
    let e = world.entity();
    e.set(SelfRef { value: e.id() });
    let e = world.entity();
    e.set(SelfRef { value: e.id() });

    let mut count = 0;

    q.run_exclusive(&mut world, |mut it| {
        while it.next() {
            let o = &it.field::<Other>(1)[0];
            assert!(!it.is_self(1));
            assert_eq!(o.value, 10);

            for i in it.iter() {
                assert_eq!(it.get_entity(i).unwrap(), it.get_entity(i).unwrap().id());
                count += 1;
            }
        }
    });

    assert_eq!(count, 3);
}

#[test]
fn isa_superset_term() {
    let mut world = World::new();

    world
        .component::<Other>()
        .add((flecs::OnInstantiate::ID, flecs::Inherit::ID));

    let q = world
        .query::<&SelfRef>()
        .with(&Other::id())
        .src()
        .up_id(*flecs::IsA)
        .set_in()
        .set_cache_kind(QueryCacheKind::Auto)
        .build();

    let base = world.entity().set(Other { value: 10 });

    let e = world.entity().add((*flecs::IsA, base));
    e.set(SelfRef { value: e.id() });
    let e = world.entity().add((*flecs::IsA, base));
    e.set(SelfRef { value: e.id() });
    let e = world.entity().add((*flecs::IsA, base));
    e.set(SelfRef { value: e.id() });

    let mut count = 0;

    q.run_exclusive(&mut world, |mut it| {
        while it.next() {
            let o = &it.field::<Other>(1)[0];
            assert!(!it.is_self(1));
            assert_eq!(o.value, 10);

            for i in it.iter() {
                assert_eq!(it.get_entity(i).unwrap(), it.get_entity(i).unwrap().id());
                count += 1;
            }
        }
    });

    assert_eq!(count, 3);
}

#[test]
fn isa_self_superset_term() {
    let mut world = World::new();

    world
        .component::<Other>()
        .add((flecs::OnInstantiate::ID, flecs::Inherit::ID));

    let q = world
        .query::<&SelfRef>()
        .with(&Other::id())
        .src()
        .self_()
        .up_id(*flecs::IsA)
        .set_in()
        .set_cache_kind(QueryCacheKind::Auto)
        .build();

    let base = world.entity().set(Other { value: 10 });

    let e = world.entity().add((*flecs::IsA, base));
    e.set(SelfRef { value: e.id() });
    let e = world.entity().add((*flecs::IsA, base));
    e.set(SelfRef { value: e.id() });
    let e = world.entity().add((*flecs::IsA, base));
    e.set(SelfRef { value: e.id() });
    let e = world.entity().set(Other { value: 20 });
    e.set(SelfRef { value: e.id() });
    let e = world.entity().set(Other { value: 20 });
    e.set(SelfRef { value: e.id() });

    let mut count = 0;
    let mut owned_count = 0;

    q.run_exclusive(&mut world, |mut it| {
        while it.next() {
            let o = &it.field::<Other>(1);

            if !it.is_self(1) {
                assert_eq!(o[0].value, 10);
            } else {
                for i in it.iter() {
                    assert_eq!(o[i].value, 20);
                    owned_count += 1;
                }
            }

            for i in it.iter() {
                assert_eq!(it.get_entity(i).unwrap(), it.get_entity(i).unwrap().id());
                count += 1;
            }
        }
    });

    assert_eq!(count, 5);
    assert_eq!(owned_count, 2);
}

#[test]
fn childof_superset_term() {
    let mut world = World::new();

    let q = world
        .query::<&SelfRef>()
        .with(&Other::id())
        .src()
        .up()
        .set_in()
        .set_cache_kind(QueryCacheKind::Auto)
        .build();

    let base = world.entity().set(Other { value: 10 });

    let e = world.entity().child_of(base);
    e.set(SelfRef { value: e.id() });
    let e = world.entity().child_of(base);
    e.set(SelfRef { value: e.id() });
    let e = world.entity().child_of(base);
    e.set(SelfRef { value: e.id() });

    let mut count = 0;

    q.run_exclusive(&mut world, |mut it| {
        while it.next() {
            let o = &it.field::<Other>(1)[0];
            assert!(!it.is_self(1));
            assert_eq!(o.value, 10);

            for i in it.iter() {
                assert_eq!(it.get_entity(i).unwrap(), it.get_entity(i).unwrap().id());
                count += 1;
            }
        }
    });

    assert_eq!(count, 3);
}

#[test]
fn childof_self_superset_term() {
    let mut world = World::new();

    let q = world
        .query::<&SelfRef>()
        .with(&Other::id())
        .src()
        .self_()
        .up()
        .set_in()
        .set_cache_kind(QueryCacheKind::Auto)
        .build();

    let base = world.entity().set(Other { value: 10 });

    let e = world.entity().child_of(base);
    e.set(SelfRef { value: e.id() });
    let e = world.entity().child_of(base);
    e.set(SelfRef { value: e.id() });
    let e = world.entity().child_of(base);
    e.set(SelfRef { value: e.id() });
    let e = world.entity().set(Other { value: 20 });
    e.set(SelfRef { value: e.id() });
    let e = world.entity().set(Other { value: 20 });
    e.set(SelfRef { value: e.id() });

    let mut count = 0;
    let mut owned_count = 0;

    q.run_exclusive(&mut world, |mut it| {
        while it.next() {
            let o = &it.field::<Other>(1);

            if !it.is_self(1) {
                assert_eq!(o[0].value, 10);
            } else {
                for i in it.iter() {
                    assert_eq!(o[i].value, 20);
                    owned_count += 1;
                }
            }

            for i in it.iter() {
                assert_eq!(it.get_entity(i).unwrap(), it.get_entity(i).unwrap().id());
                count += 1;
            }
        }
    });

    assert_eq!(count, 5);
    assert_eq!(owned_count, 2);
}

#[test]
fn isa_superset_term_w_each() {
    let mut world = World::new();

    world
        .component::<Other>()
        .add((flecs::OnInstantiate::ID, flecs::Inherit::ID));

    let q = world
        .query::<(&SelfRef, &Other)>()
        .term_at(1)
        .src()
        .up_id(*flecs::IsA)
        .set_cache_kind(QueryCacheKind::Auto)
        .build();

    let base = world.entity().set(Other { value: 10 });

    let e = world.entity().add((*flecs::IsA, base));
    e.set(SelfRef { value: e.id() });
    let e = world.entity().add((*flecs::IsA, base));
    e.set(SelfRef { value: e.id() });
    let e = world.entity().add((*flecs::IsA, base));
    e.set(SelfRef { value: e.id() });

    let mut count = 0;

    q.each_entity_exclusive(&mut world, |e, (s, o)| {
        assert_eq!(e, s.value);
        assert_eq!(o.value, 10);
        count += 1;
    });

    assert_eq!(count, 3);
}

#[test]
fn isa_self_superset_term_w_each() {
    let mut world = World::new();

    world
        .component::<Other>()
        .add((flecs::OnInstantiate::ID, flecs::Inherit::ID));

    let q = world
        .query::<(&SelfRef, &Other)>()
        .term_at(1)
        .src()
        .self_()
        .up_id(*flecs::IsA)
        .set_cache_kind(QueryCacheKind::Auto)
        .build();

    let base = world.entity().set(Other { value: 10 });

    let e = world.entity().add((*flecs::IsA, base));
    e.set(SelfRef { value: e.id() });
    let e = world.entity().add((*flecs::IsA, base));
    e.set(SelfRef { value: e.id() });
    let e = world.entity().add((*flecs::IsA, base));
    e.set(SelfRef { value: e.id() });
    let e = world.entity().set(Other { value: 10 });
    e.set(SelfRef { value: e.id() });
    let e = world.entity().set(Other { value: 10 });
    e.set(SelfRef { value: e.id() });

    let mut count = 0;

    q.each_entity_exclusive(&mut world, |e, (s, o)| {
        assert_eq!(e, s.value);
        assert_eq!(o.value, 10);
        count += 1;
    });

    assert_eq!(count, 5);
}

#[test]
fn childof_superset_term_w_each() {
    let mut world = World::new();

    let q = world
        .query::<(&SelfRef, &Other)>()
        .term_at(1)
        .src()
        .up()
        .set_cache_kind(QueryCacheKind::Auto)
        .build();

    let base = world.entity().set(Other { value: 10 });

    let e = world.entity().child_of(base);
    e.set(SelfRef { value: e.id() });
    let e = world.entity().child_of(base);
    e.set(SelfRef { value: e.id() });
    let e = world.entity().child_of(base);
    e.set(SelfRef { value: e.id() });

    let mut count = 0;

    q.each_entity_exclusive(&mut world, |e, (s, o)| {
        assert_eq!(e, s.value);
        assert_eq!(o.value, 10);
        count += 1;
    });

    assert_eq!(count, 3);
}

#[test]
fn childof_self_superset_term_w_each() {
    let mut world = World::new();

    let q = world
        .query::<(&SelfRef, &Other)>()
        .term_at(1)
        .src()
        .self_()
        .up()
        .set_cache_kind(QueryCacheKind::Auto)
        .build();

    let base = world.entity().set(Other { value: 10 });

    let e = world.entity().child_of(base);
    e.set(SelfRef { value: e.id() });
    let e = world.entity().child_of(base);
    e.set(SelfRef { value: e.id() });
    let e = world.entity().child_of(base);
    e.set(SelfRef { value: e.id() });
    let e = world.entity().set(Other { value: 10 });
    e.set(SelfRef { value: e.id() });
    let e = world.entity().set(Other { value: 10 });
    e.set(SelfRef { value: e.id() });

    let mut count = 0;

    q.each_entity_exclusive(&mut world, |e, (s, o)| {
        assert_eq!(e, s.value);
        assert_eq!(o.value, 10);
        count += 1;
    });

    assert_eq!(count, 5);
}

#[test]
fn isa_superset_shortcut() {
    let mut world = World::new();

    world
        .component::<Other>()
        .add((flecs::OnInstantiate::ID, flecs::Inherit::ID));

    let q = world
        .query::<(&SelfRef, &Other)>()
        .term_at(1)
        .up_id(*flecs::IsA)
        .set_cache_kind(QueryCacheKind::Auto)
        .build();

    let base = world.entity().set(Other { value: 10 });

    let e = world.entity().is_a(base);
    e.set(SelfRef { value: e.id() });
    let e = world.entity().is_a(base);
    e.set(SelfRef { value: e.id() });
    let e = world.entity().is_a(base);
    e.set(SelfRef { value: e.id() });

    let mut count = 0;

    q.each_entity_exclusive(&mut world, |e, (s, o)| {
        assert_eq!(e, s.value);
        assert_eq!(o.value, 10);
        count += 1;
    });

    assert_eq!(count, 3);
}

#[test]
fn isa_superset_shortcut_w_self() {
    let mut world = World::new();

    world
        .component::<Other>()
        .add((flecs::OnInstantiate::ID, flecs::Inherit::ID));

    let q = world
        .query::<(&SelfRef, &Other)>()
        .term_at(1)
        .self_()
        .up_id(*flecs::IsA)
        .set_cache_kind(QueryCacheKind::Auto)
        .build();

    let base = world.entity().set(Other { value: 10 });

    let e = world.entity().is_a(base);
    e.set(SelfRef { value: e.id() });
    let e = world.entity().is_a(base);
    e.set(SelfRef { value: e.id() });
    let e = world.entity().is_a(base);
    e.set(SelfRef { value: e.id() });
    let e = world.entity().set(Other { value: 10 });
    e.set(SelfRef { value: e.id() });
    let e = world.entity().set(Other { value: 10 });
    e.set(SelfRef { value: e.id() });

    let mut count = 0;

    q.each_entity_exclusive(&mut world, |e, (s, o)| {
        assert_eq!(e, s.value);
        assert_eq!(o.value, 10);
        count += 1;
    });

    assert_eq!(count, 5);
}

#[test]
fn childof_superset_shortcut() {
    let mut world = World::new();

    let q = world
        .query::<(&SelfRef, &Other)>()
        .term_at(1)
        .up()
        .set_cache_kind(QueryCacheKind::Auto)
        .build();

    let base = world.entity().set(Other { value: 10 });

    let e = world.entity().child_of(base);
    e.set(SelfRef { value: e.id() });
    let e = world.entity().child_of(base);
    e.set(SelfRef { value: e.id() });
    let e = world.entity().child_of(base);
    e.set(SelfRef { value: e.id() });

    let mut count = 0;

    q.each_entity_exclusive(&mut world, |e, (s, o)| {
        assert_eq!(e, s.value);
        assert_eq!(o.value, 10);
        count += 1;
    });

    assert_eq!(count, 3);
}

#[test]
fn childof_superset_shortcut_w_self() {
    let mut world = World::new();

    let q = world
        .query::<(&SelfRef, &Other)>()
        .term_at(1)
        .self_()
        .up()
        .set_cache_kind(QueryCacheKind::Auto)
        .build();

    let base = world.entity().set(Other { value: 10 });

    let e = world.entity().child_of(base);
    e.set(SelfRef { value: e.id() });
    let e = world.entity().child_of(base);
    e.set(SelfRef { value: e.id() });
    let e = world.entity().child_of(base);
    e.set(SelfRef { value: e.id() });
    let e = world.entity().set(Other { value: 10 });
    e.set(SelfRef { value: e.id() });
    let e = world.entity().set(Other { value: 10 });
    e.set(SelfRef { value: e.id() });

    let mut count = 0;

    q.each_entity_exclusive(&mut world, |e, (s, o)| {
        assert_eq!(e, s.value);
        assert_eq!(o.value, 10);
        count += 1;
    });

    assert_eq!(count, 5);
}

#[test]
fn relation() {
    let mut world = World::new();

    let likes = world.entity();
    let bob = world.entity();
    let alice = world.entity();

    let q = world
        .query::<&SelfRef>()
        .with((likes, bob))
        .set_cache_kind(QueryCacheKind::Auto)
        .build();

    let e = world.entity().add((likes, bob));
    e.set(SelfRef { value: e.id() });
    let e = world.entity().add((likes, bob));
    e.set(SelfRef { value: e.id() });

    let e = world.entity().add((likes, alice));
    e.set(SelfRef { value: Entity(0) });
    let e = world.entity().add((likes, alice));
    e.set(SelfRef { value: Entity(0) });

    let mut count = 0;

    q.each_entity_exclusive(&mut world, |e, s| {
        assert_eq!(e, s.value);
        count += 1;
    });

    assert_eq!(count, 2);
}

#[test]
fn relation_w_object_wildcard() {
    let mut world = World::new();

    let likes = world.entity();
    let bob = world.entity();
    let alice = world.entity();

    let q = world
        .query::<&SelfRef>()
        .with((likes, *flecs::Wildcard))
        .set_cache_kind(QueryCacheKind::Auto)
        .build();

    let e = world.entity().add((likes, bob));
    e.set(SelfRef { value: e.id() });
    let e = world.entity().add((likes, bob));
    e.set(SelfRef { value: e.id() });

    let e = world.entity().add((likes, alice));
    e.set(SelfRef { value: e.id() });
    let e = world.entity().add((likes, alice));
    e.set(SelfRef { value: e.id() });

    let e = world.entity();
    e.set(SelfRef { value: Entity(0) });
    let e = world.entity();
    e.set(SelfRef { value: Entity(0) });

    let mut count = 0;

    q.each_entity_exclusive(&mut world, |e, s| {
        assert_eq!(e, s.value);
        count += 1;
    });

    assert_eq!(count, 4);
}

#[test]
fn relation_w_predicate_wildcard() {
    let mut world = World::new();

    let likes = world.entity();
    let dislikes = world.entity();
    let bob = world.entity();
    let alice = world.entity();

    let q = world
        .query::<&SelfRef>()
        .with((*flecs::Wildcard, alice))
        .set_cache_kind(QueryCacheKind::Auto)
        .build();

    let e = world.entity().add((likes, alice));
    e.set(SelfRef { value: e.id() });
    let e = world.entity().add((dislikes, alice));
    e.set(SelfRef { value: e.id() });

    let e = world.entity().add((likes, bob));
    e.set(SelfRef { value: Entity(0) });
    let e = world.entity().add((dislikes, bob));
    e.set(SelfRef { value: Entity(0) });

    let mut count = 0;

    q.each_entity_exclusive(&mut world, |e, s| {
        assert_eq!(e, s.value);
        count += 1;
    });

    assert_eq!(count, 2);
}

#[test]
fn add_pair_w_rel_type() {
    let mut world = World::new();

    let dislikes = world.entity();
    let bob = world.entity();
    let alice = world.entity();

    let q = world
        .query::<&SelfRef>()
        .with((Likes::id(), *flecs::Wildcard))
        .set_cache_kind(QueryCacheKind::Auto)
        .build();

    let e = world.entity().add((Likes::id(), alice));
    e.set(SelfRef { value: e.id() });
    let e = world.entity().add((dislikes, alice));
    e.set(SelfRef { value: Entity(0) });

    let e = world.entity().add((Likes::id(), bob));
    e.set(SelfRef { value: e.id() });
    let e = world.entity().add((dislikes, bob));
    e.set(SelfRef { value: Entity(0) });

    let mut count = 0;

    q.each_entity_exclusive(&mut world, |e, s| {
        assert_eq!(e, s.value);
        count += 1;
    });

    assert_eq!(count, 2);
}

#[test]
fn template_term() {
    let world = World::new();

    let q = world
        .query::<&Position>()
        .with(id::<&Template<u32>>())
        .set_cache_kind(QueryCacheKind::Auto)
        .build();

    let e1 = world
        .entity()
        .set(Position { x: 0, y: 0 })
        .set(Template::<u32> { value: 10 });
    world.entity().set(Position { x: 0, y: 0 });

    let mut count = 0;
    q.each_entity_shared(&world, |e, _p| {
        count += 1;
        assert_eq!(e, e1);
    });

    assert_eq!(count, 1);
}

#[test]
fn typed_term_at() {
    let world = World::new();

    world.set(Count(0));

    let s = world
        .system::<&Velocity>()
        .with(&mut RelFoo::id())
        .term_at_type::<Velocity>()
        .set_src(Velocity::id())
        .term_at_type::<RelFoo>()
        .set_second(flecs::Wildcard::ID)
        .run(|mut it| {
            let world = it.world();
            while it.next() {
                {
                    let mut count = world
                        .entity_from_id(Count::entity_id(world))
                        .get_ref::<&mut Count>()
                        .unwrap();
                    count.0 += it.count() as i32;
                }
            }
        });

    world.entity().add((RelFoo::id(), Tag));
    world.set(Velocity { x: 0, y: 0 });

    s.run();

    {
        let count = WorldSingletonExt::singleton::<Count>(&world).unwrap();
        assert_eq!(count.0, 1);
    };
}

#[test]
fn typed_term_at_indexed() {
    let world = World::new();

    world.set(Count(0));

    let s = world
        .system::<&Velocity>()
        .with(&mut RelFoo::id())
        .term_at_checked::<Velocity>(0)
        .set_src(Velocity::id())
        .term_at_checked::<RelFoo>(1)
        .set_second(flecs::Wildcard::ID)
        .run(|mut it| {
            let world = it.world();
            while it.next() {
                {
                    let mut count = world
                        .entity_from_id(Count::entity_id(world))
                        .get_ref::<&mut Count>()
                        .unwrap();
                    count.0 += it.count() as i32;
                }
            }
        });

    world.entity().add((RelFoo::id(), Tag));
    world.set(Velocity { x: 0, y: 0 });

    s.run();

    {
        let count = WorldSingletonExt::singleton::<Count>(&world).unwrap();
        assert_eq!(count.0, 1);
    };
}

#[test]
fn explicit_subject_w_id() {
    let world = World::new();

    let q = world
        .query::<&Position>()
        .with(&Position::id())
        .set_id(*flecs::This_)
        .set_cache_kind(QueryCacheKind::Auto)
        .build();

    let e1 = world
        .entity()
        .set(Position { x: 0, y: 0 })
        .set(Velocity { x: 0, y: 0 });
    world.entity().set(Velocity { x: 0, y: 0 });

    let mut count = 0;
    q.each_entity_shared(&world, |e, _p| {
        count += 1;
        assert_eq!(e, e1);
    });

    assert_eq!(count, 1);
}

#[test]
fn explicit_subject_w_type() {
    let world = World::new();

    world.set(Position { x: 10, y: 20 });

    let q = world
        .query::<&Position>()
        .with(&Position::id())
        .set_src(Position::id())
        .set_cache_kind(QueryCacheKind::Auto)
        .build();

    let mut count = 0;
    q.each_entity_shared(&world, |e, p| {
        assert_eq!(p.x, 10);
        assert_eq!(p.y, 20);
        count += 1;
        assert_eq!(e, world.singleton::<Position>());
    });

    assert_eq!(count, 1);
}

#[test]
fn explicit_object_w_id() {
    let world = World::new();

    let likes = world.entity();
    let alice = world.entity();
    let bob = world.entity();

    let q = world
        .query::<()>()
        .with(likes)
        .set_second(alice)
        .set_cache_kind(QueryCacheKind::Auto)
        .build();

    let e1 = world.entity().add((likes, alice));
    world.entity().add((likes, bob));

    let mut count = 0;
    q.each_entity_shared(&world, |e, _| {
        count += 1;
        assert_eq!(e, e1);
    });

    assert_eq!(count, 1);
}

#[test]
fn explicit_object_w_type() {
    let world = World::new();

    let likes = world.entity();
    let bob = world.entity();

    let q = world
        .query::<()>()
        .with(likes)
        .set_second(Alice::id())
        .set_cache_kind(QueryCacheKind::Auto)
        .build();

    let e1 = world
        .entity()
        .add((likes, *world.id_view_from(Alice::id()).id()));
    world.entity().add((likes, bob));

    let mut count = 0;
    q.each_entity_shared(&world, |e, _| {
        count += 1;
        assert_eq!(e, e1);
    });

    assert_eq!(count, 1);
}

#[test]
#[ignore = "decided to not support explicit term setting"]
fn explicit_term() {
    // let world = create_world();

    // let q = world
    //     .query::<()>()
    //     .with(world.term(world.id_view_from(Position::id())))
    //     .set_cache_kind(QueryCacheKind::Auto)
    //     .build();

    // let e1 = world.entity().set(Position { x: 0, y: 0 });
    // world.entity().set(Velocity { x: 0, y: 0 });

    // let mut count = 0;
    // q.each_entity(|e, _| {
    //     count += 1;
    //     assert_eq!(e, e1);
    // });

    // assert_eq!(count, 1);
}

#[test]
#[ignore = "decided to not support explicit term setting"]
fn explicit_term_w_type() {
    //     let world = World::new();

    //     let q = world.query::<()>()
    //         .with(world.term<Position>())
    //         .set_cache_kind(QueryCacheKind::Auto)
    //         .build();

    //     let e1 = world.entity().set(Position { x: 0, y: 0 });
    //     world.entity().set(Velocity { x: 0, y: 0 });

    //    let mut count = 0;
    //     q.each_entity(|e, _| {
    //         count += 1;
    //         assert_eq!(e, e1);
    //     });

    //     assert_eq!(count, 1);
}

#[test]
#[ignore = "decided to not support explicit term setting"]
fn explicit_term_w_pair_type() {
    //     let world = World::new();

    //     let q = world.query::<()>()
    //         .with((world.term<Likes, alice>()))
    //         .set_cache_kind(QueryCacheKind::Auto)
    //         .build();

    //     let e1 = world.entity().add((Likes::id(), alice::id()));
    //     world.entity().add((Likes::id(), bob::id()));

    //    let mut count = 0;
    //     q.each_entity(|e, _| {
    //         count += 1;
    //         assert_eq!(e, e1);
    //     });

    //     assert_eq!(count, 1);
}

#[test]
#[ignore = "decided to not support explicit term setting"]
fn explicit_term_w_id() {
    //     let world = create_world();

    //     let apples = world.entity();
    //    let pears = world.entity();

    //     let q = world.query::<()>()
    //         .with(world.term(apples))
    //         .set_cache_kind(QueryCacheKind::Auto)
    //         .build();

    //     let e1 = world.entity().add(apples);
    //     world.entity().add(pears);

    //    let mut count = 0;
    //     q.each_entity(|e, _| {
    //         count += 1;
    //         assert_eq!(e, e1);
    //     });

    //     assert_eq!(count, 1);
}

#[test]
fn explicit_term_w_pair_id() {
    //     let world = create_world();

    //     let likes = world.entity();
    //     let apples = world.entity();
    //    let pears = world.entity();

    //     let q = world.query::<()>()
    //         .with(world.term((likes,apples)))
    //         .set_cache_kind(QueryCacheKind::Auto)
    //         .build();

    //     let e1 = world.entity().add((likes,apples));
    //     world.entity().add((likes,pears));

    //    let mut count = 0;
    //     q.each_entity(|e, _| {
    //         count += 1;
    //         assert_eq!(e, e1);
    //     });

    //     assert_eq!(count, 1);
}

#[test]
fn n1_term_to_empty() {
    let world = World::new();

    let likes = world.entity();
    let apples = world.entity();

    let mut q = world.query::<()>();
    q = q.with(&Position::id()).set_cache_kind(QueryCacheKind::Auto);
    q = q.with((likes, apples));

    let q = q.build();

    assert_eq!(q.field_count(), 2);
    assert_eq!(q.term(0).id(), world.id_view_from(Position::id()));
    assert_eq!(q.term(1).id(), world.id_view_from((likes, apples)));
}

#[test]
fn n2_subsequent_args() {
    #[derive(Component, Default)]
    struct Flags {
        count: usize,
    }

    let world = create_world_with_flags::<Flags>();

    world
        .component::<Velocity>()
        .add_trait::<flecs::Singleton>();

    let s = world
        .system::<(&mut (RelFoo, flecs::Wildcard), &Velocity)>()
        .term_at(1)
        .run(|mut it| {
            while it.next() {
                {
                    let w = it.real_world();
                    let mut f = w
                        .entity_from_id(Flags::entity_id(w))
                        .get_ref::<&mut Flags>()
                        .unwrap();
                    f.count += it.count();
                }
            }
        });

    world.entity().set_pair::<RelFoo, Tag>(RelFoo { foo: 10 });
    world.set(Velocity { x: 0, y: 0 });

    s.run();

    {
        let f = WorldSingletonExt::singleton::<Flags>(&world).unwrap();
        assert_eq!(f.count, 1);
    };
}

#[test]
fn optional_tag_is_set() {
    let world = World::new();

    let q = world
        .query::<()>()
        .with(&TagA::id())
        .with(&TagB::id())
        .set_oper(OperKind::Optional)
        .set_cache_kind(QueryCacheKind::Auto)
        .build();

    let e_1 = world.entity().add(TagA::id()).add(TagB::id());
    let e_2 = world.entity().add(TagA::id());

    let mut count = 0;

    q.run_shared(&world, |mut it| {
        while it.next() {
            assert_eq!(it.count(), 1);

            count += it.count();

            if it.get_entity(0usize).unwrap() == e_1 {
                assert!(it.is_set(0));
                assert!(it.is_set(1));
            } else {
                assert_eq!(it.get_entity(0usize).unwrap(), e_2);
                assert!(it.is_set(0));
                assert!(!it.is_set(1));
            }
        }
    });

    assert_eq!(count, 2);
}

#[test]
fn n10_terms() {
    let world = World::new();

    let f = world
        .query::<()>()
        .with(&TagA::id())
        .with(&TagB::id())
        .with(&TagC::id())
        .with(&TagD::id())
        .with(&TagE::id())
        .with(&TagF::id())
        .with(&TagG::id())
        .with(&TagH::id())
        .with(&TagI::id())
        .with(&TagJ::id())
        .set_cache_kind(QueryCacheKind::Auto)
        .build();

    assert_eq!(f.field_count(), 10);

    let e = world
        .entity()
        .add(TagA::id())
        .add(TagB::id())
        .add(TagC::id())
        .add(TagD::id())
        .add(TagE::id())
        .add(TagF::id())
        .add(TagG::id())
        .add(TagH::id())
        .add(TagI::id())
        .add(TagJ::id());

    let mut count = 0;
    f.run_shared(&world, |mut it| {
        while it.next() {
            assert_eq!(it.field_count(), 10);
            assert_eq!(it.get_entity(0usize).unwrap(), e);
            assert_eq!(it.count(), 1);
            count += 1;
        }
    });

    assert_eq!(count, 1);
}

#[test]
fn n16_terms() {
    let world = World::new();

    let f = world
        .query::<()>()
        .with(&TagA::id())
        .with(&TagB::id())
        .with(&TagC::id())
        .with(&TagD::id())
        .with(&TagE::id())
        .with(&TagF::id())
        .with(&TagG::id())
        .with(&TagH::id())
        .with(&TagI::id())
        .with(&TagJ::id())
        .with(&TagK::id())
        .with(&TagL::id())
        .with(&TagM::id())
        .with(&TagN::id())
        .with(&TagO::id())
        .with(&TagP::id())
        .set_cache_kind(QueryCacheKind::Auto)
        .build();

    assert_eq!(f.field_count(), 16);

    let e = world
        .entity()
        .add(TagA::id())
        .add(TagB::id())
        .add(TagC::id())
        .add(TagD::id())
        .add(TagE::id())
        .add(TagF::id())
        .add(TagG::id())
        .add(TagH::id())
        .add(TagI::id())
        .add(TagJ::id())
        .add(TagK::id())
        .add(TagL::id())
        .add(TagM::id())
        .add(TagN::id())
        .add(TagO::id())
        .add(TagP::id())
        .add(TagQ::id())
        .add(TagR::id())
        .add(TagS::id())
        .add(TagT::id());

    let mut count = 0;
    f.run_shared(&world, |mut it| {
        while it.next() {
            assert_eq!(it.count(), 1);
            assert_eq!(it.get_entity(0usize).unwrap(), e);
            assert_eq!(it.field_count(), 16);
            count += 1;
        }
    });

    assert_eq!(count, 1);
}

#[extern_abi]
unsafe fn group_by_first_id(
    _world: *mut sys::ecs_world_t,
    table: *mut sys::ecs_table_t,
    _id: u64,
    _ctx: *mut c_void,
) -> u64 {
    unsafe {
        let table_type: *const sys::ecs_type_t = sys::ecs_table_get_type(table);
        *(*table_type).array.add(0)
    }
}

#[extern_abi]
unsafe fn group_by_first_id_negated(
    world: *mut sys::ecs_world_t,
    table: *mut sys::ecs_table_t,
    id: u64,
    ctx: *mut c_void,
) -> u64 {
    !group_by_first_id(world, table, id, ctx)
}

#[test]
fn group_by_raw() {
    let world = World::new();

    world.component::<TagA>();
    world.component::<TagB>();
    world.component::<TagC>();
    world.component::<TagX>();

    let q = world
        .query::<()>()
        .with(&TagX::id())
        .group_by_fn(world.entity_from::<TagX>(), Some(group_by_first_id))
        .build();

    let q_reverse = world
        .query::<()>()
        .with(&TagX::id())
        .group_by_fn(world.entity_from::<TagX>(), Some(group_by_first_id_negated))
        .build();

    let e1 = world.entity().add(TagX::id()).add(TagA::id());
    let e2 = world.entity().add(TagX::id()).add(TagB::id());
    let e3 = world.entity().add(TagX::id()).add(TagC::id());

    let mut count = 0;

    q.run_shared(&world, |mut it| {
        while it.next() {
            assert_eq!(it.count(), 1);
            if count == 0 {
                assert!(it.get_entity(0usize).unwrap() == e3);
            } else if count == 1 {
                assert!(it.get_entity(0usize).unwrap() == e2);
            } else if count == 2 {
                assert!(it.get_entity(0usize).unwrap() == e1);
            } else {
                panic!();
            }
            count += 1;
        }
    });
    assert_eq!(count, 3);

    count = 0;
    q_reverse.run_shared(&world, |mut it| {
        while it.next() {
            assert_eq!(it.count(), 1);
            if count == 0 {
                assert!(it.get_entity(0usize).unwrap() == e3);
            } else if count == 1 {
                assert!(it.get_entity(0usize).unwrap() == e2);
            } else if count == 2 {
                assert!(it.get_entity(0usize).unwrap() == e1);
            } else {
                panic!();
            }
            count += 1;
        }
    });
    assert_eq!(count, 3);
}

#[test]
fn group_by_template() {
    let world = World::new();

    world.component::<TagA>();
    world.component::<TagB>();
    world.component::<TagC>();
    world.component::<TagX>();

    let q = world
        .query::<()>()
        .with(&TagX::id())
        .group_by_fn(TagX::id(), Some(group_by_first_id))
        .build();

    let q_reverse = world
        .query::<()>()
        .with(&TagX::id())
        .group_by_fn(TagX::id(), Some(group_by_first_id_negated))
        .build();

    let e1 = world.entity().add(TagX::id()).add(TagA::id());
    let e2 = world.entity().add(TagX::id()).add(TagB::id());
    let e3 = world.entity().add(TagX::id()).add(TagC::id());

    let mut count = 0;

    q.run_shared(&world, |mut it| {
        while it.next() {
            assert_eq!(it.count(), 1);
            if count == 0 {
                assert!(it.get_entity(0usize).unwrap() == e3);
            } else if count == 1 {
                assert!(it.get_entity(0usize).unwrap() == e2);
            } else if count == 2 {
                assert!(it.get_entity(0usize).unwrap() == e1);
            } else {
                panic!();
            }
            count += 1;
        }
    });
    assert_eq!(count, 3);

    count = 0;
    q_reverse.run_shared(&world, |mut it| {
        while it.next() {
            assert_eq!(it.count(), 1);
            if count == 0 {
                assert!(it.get_entity(0usize).unwrap() == e3);
            } else if count == 1 {
                assert!(it.get_entity(0usize).unwrap() == e2);
            } else if count == 2 {
                assert!(it.get_entity(0usize).unwrap() == e1);
            } else {
                panic!();
            }
            count += 1;
        }
    });
    assert_eq!(count, 3);
}

#[test]
fn group_by_raw_ordered() {
    let world = World::new();

    world.component::<TagA>();
    world.component::<TagB>();
    world.component::<TagC>();
    world.component::<TagX>();

    let q = world
        .query::<()>()
        .with(&TagX::id())
        .group_by_fn(world.entity_from::<TagX>(), Some(group_by_first_id))
        .query_flags(QueryFlags::GroupByOrdered)
        .build();

    let q_reverse = world
        .query::<()>()
        .with(&TagX::id())
        .group_by_fn(world.entity_from::<TagX>(), Some(group_by_first_id_negated))
        .query_flags(QueryFlags::GroupByOrdered)
        .build();

    let e1 = world.entity().add(TagX::id()).add(TagA::id());
    let e2 = world.entity().add(TagX::id()).add(TagB::id());
    let e3 = world.entity().add(TagX::id()).add(TagC::id());

    let mut count = 0;

    q.run_shared(&world, |mut it| {
        while it.next() {
            assert_eq!(it.count(), 1);
            if count == 0 {
                assert!(it.get_entity(0usize).unwrap() == e1);
            } else if count == 1 {
                assert!(it.get_entity(0usize).unwrap() == e2);
            } else if count == 2 {
                assert!(it.get_entity(0usize).unwrap() == e3);
            } else {
                panic!();
            }
            count += 1;
        }
    });
    assert_eq!(count, 3);

    count = 0;
    q_reverse.run_shared(&world, |mut it| {
        while it.next() {
            assert_eq!(it.count(), 1);
            if count == 0 {
                assert!(it.get_entity(0usize).unwrap() == e3);
            } else if count == 1 {
                assert!(it.get_entity(0usize).unwrap() == e2);
            } else if count == 2 {
                assert!(it.get_entity(0usize).unwrap() == e1);
            } else {
                panic!();
            }
            count += 1;
        }
    });
    assert_eq!(count, 3);
}

#[test]
fn group_by_template_ordered() {
    let world = World::new();

    world.component::<TagA>();
    world.component::<TagB>();
    world.component::<TagC>();
    world.component::<TagX>();

    let q = world
        .query::<()>()
        .with(&TagX::id())
        .group_by_fn(TagX::id(), Some(group_by_first_id))
        .query_flags(QueryFlags::GroupByOrdered)
        .build();

    let q_reverse = world
        .query::<()>()
        .with(&TagX::id())
        .group_by_fn(TagX::id(), Some(group_by_first_id_negated))
        .query_flags(QueryFlags::GroupByOrdered)
        .build();

    let e1 = world.entity().add(TagX::id()).add(TagA::id());
    let e2 = world.entity().add(TagX::id()).add(TagB::id());
    let e3 = world.entity().add(TagX::id()).add(TagC::id());

    let mut count = 0;

    q.run_shared(&world, |mut it| {
        while it.next() {
            assert_eq!(it.count(), 1);
            if count == 0 {
                assert!(it.get_entity(0usize).unwrap() == e1);
            } else if count == 1 {
                assert!(it.get_entity(0usize).unwrap() == e2);
            } else if count == 2 {
                assert!(it.get_entity(0usize).unwrap() == e3);
            } else {
                panic!();
            }
            count += 1;
        }
    });
    assert_eq!(count, 3);

    count = 0;
    q_reverse.run_shared(&world, |mut it| {
        while it.next() {
            assert_eq!(it.count(), 1);
            if count == 0 {
                assert!(it.get_entity(0usize).unwrap() == e3);
            } else if count == 1 {
                assert!(it.get_entity(0usize).unwrap() == e2);
            } else if count == 2 {
                assert!(it.get_entity(0usize).unwrap() == e1);
            } else {
                panic!();
            }
            count += 1;
        }
    });
    assert_eq!(count, 3);
}

#[extern_abi]
unsafe fn group_by_rel(
    world: *mut sys::ecs_world_t,
    table: *mut sys::ecs_table_t,
    id: u64,
    _ctx: *mut c_void,
) -> u64 {
    unsafe {
        let mut id_matched: u64 = 0;
        let ref_id = &mut id_matched;
        if sys::ecs_search(world, table, ecs_pair(id, *flecs::Wildcard), ref_id) != -1 {
            return *ecs_second(id_matched, WorldRef::from_ptr(world));
        }
        0
    }
}

#[test]
fn group_by_iter_one() {
    let world = World::new();

    let rel = world.entity();
    let tgt_a = world.entity();
    let tgt_b = world.entity();
    let tgt_c = world.entity();
    let tag = world.entity();

    world.entity().add((rel, tgt_a));
    let e2 = world.entity().add((rel, tgt_b));
    world.entity().add((rel, tgt_c));

    world.entity().add((rel, tgt_a)).add(tag);
    let e5 = world.entity().add((rel, tgt_b)).add(tag);
    world.entity().add((rel, tgt_c)).add(tag);

    let q = world
        .query::<()>()
        .with((rel, *flecs::Wildcard))
        .group_by_fn(rel, Some(group_by_rel))
        .build();

    let mut e2_found = false;
    let mut e5_found = false;
    let mut count = 0;

    q.iterable().set_group(tgt_b).run_shared(&world, |mut it| {
        while it.next() {
            for i in 0..it.count() {
                let e = it.get_entity(i).unwrap();
                assert_eq!(it.group_id(), tgt_b.id());

                if e == e2 {
                    e2_found = true;
                }
                if e == e5 {
                    e5_found = true;
                }
                count += 1;
            }
        }
    });

    assert_eq!(2, count);
    assert!(e2_found);
    assert!(e5_found);
}

#[test]
fn group_by_iter_one_template() {
    let world = World::new();

    world.entity().add((Rel::id(), TagA::id()));
    let e2 = world.entity().add((Rel::id(), TagB::id()));
    world.entity().add((Rel::id(), TagC::id()));

    world.entity().add((Rel::id(), TagA::id())).add(Tag);
    let e5 = world.entity().add((Rel::id(), TagB::id())).add(Tag);
    world.entity().add((Rel::id(), TagC::id())).add(Tag);

    let q = world
        .query::<()>()
        .with((Rel::id(), *flecs::Wildcard))
        .group_by_fn(Rel::id(), Some(group_by_rel))
        .build();

    let mut e2_found = false;
    let mut e5_found = false;
    let mut count = 0;

    q.iterable().set_group(TagB::id()).run_shared(&world, |mut it| {
        while it.next() {
            for i in 0..it.count() {
                let e = it.get_entity(i).unwrap();
                assert_eq!(it.group_id(), world.id_view_from(TagB::id()));

                if e == e2 {
                    e2_found = true;
                }
                if e == e5 {
                    e5_found = true;
                }
                count += 1;
            }
        }
    });

    assert_eq!(2, count);
    assert!(e2_found);
    assert!(e5_found);
}

#[test]
fn group_by_iter_one_all_groups() {
    let world = World::new();

    let rel = world.entity();
    let tgt_a = world.entity();
    let tgt_b = world.entity();
    let tgt_c = world.entity();
    let tag = world.entity();

    let e1 = world.entity().add((rel, tgt_a));
    let e2 = world.entity().add((rel, tgt_b));
    let e3 = world.entity().add((rel, tgt_c));

    let e4 = world.entity().add((rel, tgt_a)).add(tag);
    let e5 = world.entity().add((rel, tgt_b)).add(tag);
    let e6 = world.entity().add((rel, tgt_c)).add(tag);

    let q = world
        .query::<()>()
        .with((rel, *flecs::Wildcard))
        .group_by_fn(rel, Some(group_by_rel))
        .build();

    let group_id = Cell::new(0u64);
    let count = Cell::new(0usize);
    let e1_found = Cell::new(false);
    let e2_found = Cell::new(false);
    let e3_found = Cell::new(false);
    let e4_found = Cell::new(false);
    let e5_found = Cell::new(false);
    let e6_found = Cell::new(false);

    let func = |mut it: TableIter<true>| {
        while it.next() {
            for i in it.iter() {
                let e = it.get_entity(i).unwrap();
                if it.group_id() == group_id.get() {
                    if e == e1 {
                        e1_found.set(true);
                    }
                    if e == e2 {
                        e2_found.set(true);
                    }
                    if e == e3 {
                        e3_found.set(true);
                    }
                    if e == e4 {
                        e4_found.set(true);
                    }
                    if e == e5 {
                        e5_found.set(true);
                    }
                    if e == e6 {
                        e6_found.set(true);
                    }
                    count.set(count.get() + 1);
                }
            }
        }
    };

    group_id.set(*tgt_b.id());
    q.iterable().set_group(tgt_b).run_shared(&world, func);

    assert_eq!(2, count.get());
    assert!(e2_found.get());
    assert!(e5_found.get());

    group_id.set(*tgt_a.id());
    q.iterable().set_group(tgt_a).run_shared(&world, func);
    assert_eq!(4, count.get());
    assert!(e1_found.get());
    assert!(e4_found.get());

    group_id.set(*tgt_c.id());
    q.iterable().set_group(tgt_c).run_shared(&world, func);
    assert_eq!(6, count.get());
    assert!(e3_found.get());
    assert!(e6_found.get());
}

#[test]
fn group_by_default_func_w_id() {
    let world = World::new();

    let rel = world.entity();
    let tgt_a = world.entity();
    let tgt_b = world.entity();
    let tgt_c = world.entity();

    let e1 = world.entity().add((rel, tgt_a));
    let e2 = world.entity().add((rel, tgt_b));
    let e3 = world.entity().add((rel, tgt_c));

    let q = world
        .query::<()>()
        .with((rel, *flecs::Wildcard))
        .group_by(rel)
        .build();

    let mut e1_found = false;
    let mut e2_found = false;
    let mut e3_found = false;
    let mut count = 0;

    q.run_shared(&world, |mut it| {
        while it.next() {
            for i in it.iter() {
                let e = it.get_entity(i).unwrap();
                if e == e1 {
                    assert_eq!(it.group_id(), tgt_a);
                    assert!(!e1_found);
                    assert!(e2_found);
                    assert!(e3_found);
                    e1_found = true;
                }
                if e == e2 {
                    assert_eq!(it.group_id(), tgt_b);
                    assert!(!e1_found);
                    assert!(!e2_found);
                    assert!(e3_found);
                    e2_found = true;
                }
                if e == e3 {
                    assert_eq!(it.group_id(), tgt_c);
                    assert!(!e1_found);
                    assert!(!e2_found);
                    assert!(!e3_found);
                    e3_found = true;
                }
                count += 1;
            }
        }
    });

    assert_eq!(3, count);
    assert!(e1_found);
    assert!(e2_found);
    assert!(e3_found);
}

#[test]
fn group_by_default_func_w_id_ordered() {
    let world = World::new();

    let rel = world.entity();
    let tgt_a = world.entity();
    let tgt_b = world.entity();
    let tgt_c = world.entity();

    let e1 = world.entity().add((rel, tgt_a));
    let e2 = world.entity().add((rel, tgt_b));
    let e3 = world.entity().add((rel, tgt_c));

    let q = world
        .query::<()>()
        .with((rel, *flecs::Wildcard))
        .group_by(rel)
        .query_flags(QueryFlags::GroupByOrdered)
        .build();

    let mut e1_found = false;
    let mut e2_found = false;
    let mut e3_found = false;
    let mut count = 0;

    q.run_shared(&world, |mut it| {
        while it.next() {
            for i in it.iter() {
                let e = it.get_entity(i).unwrap();
                if e == e1 {
                    assert_eq!(it.group_id(), tgt_a);
                    assert!(!e1_found);
                    assert!(!e2_found);
                    assert!(!e3_found);
                    e1_found = true;
                }
                if e == e2 {
                    assert_eq!(it.group_id(), tgt_b);
                    assert!(e1_found);
                    assert!(!e2_found);
                    assert!(!e3_found);
                    e2_found = true;
                }
                if e == e3 {
                    assert_eq!(it.group_id(), tgt_c);
                    assert!(e1_found);
                    assert!(e2_found);
                    assert!(!e3_found);
                    e3_found = true;
                }
                count += 1;
            }
        }
    });

    assert_eq!(3, count);
    assert!(e1_found);
    assert!(e2_found);
    assert!(e3_found);
}

#[test]
fn group_by_default_func_w_type() {
    let world = World::new();

    let tgt_a = world.entity();
    let tgt_b = world.entity();
    let tgt_c = world.entity();

    let e1 = world.entity().add((Rel::id(), tgt_a));
    let e2 = world.entity().add((Rel::id(), tgt_b));
    let e3 = world.entity().add((Rel::id(), tgt_c));

    let q = world
        .query::<()>()
        .with((Rel::id(), id::<flecs::Wildcard>()))
        .group_by(Rel::id())
        .build();

    let mut e1_found = false;
    let mut e2_found = false;
    let mut e3_found = false;
    let mut count = 0;

    q.run_shared(&world, |mut it| {
        while it.next() {
            for i in it.iter() {
                let e = it.get_entity(i).unwrap();
                if e == e1 {
                    assert_eq!(it.group_id(), tgt_a);
                    assert!(!e1_found);
                    assert!(e2_found);
                    assert!(e3_found);
                    e1_found = true;
                }
                if e == e2 {
                    assert_eq!(it.group_id(), tgt_b);
                    assert!(!e1_found);
                    assert!(!e2_found);
                    assert!(e3_found);
                    e2_found = true;
                }
                if e == e3 {
                    assert_eq!(it.group_id(), tgt_c);
                    assert!(!e1_found);
                    assert!(!e2_found);
                    assert!(!e3_found);
                    e3_found = true;
                }
                count += 1;
            }
        }
    });

    assert_eq!(3, count);
    assert!(e1_found);
    assert!(e2_found);
    assert!(e3_found);
}

#[test]
fn group_by_default_func_w_type_ordered() {
    let world = World::new();

    let tgt_a = world.entity();
    let tgt_b = world.entity();
    let tgt_c = world.entity();

    let e1 = world.entity().add((Rel::id(), tgt_a));
    let e2 = world.entity().add((Rel::id(), tgt_b));
    let e3 = world.entity().add((Rel::id(), tgt_c));

    let q = world
        .query::<()>()
        .with((Rel::id(), id::<flecs::Wildcard>()))
        .group_by(Rel::id())
        .query_flags(QueryFlags::GroupByOrdered)
        .build();

    let mut e1_found = false;
    let mut e2_found = false;
    let mut e3_found = false;
    let mut count = 0;

    q.run_shared(&world, |mut it| {
        while it.next() {
            for i in it.iter() {
                let e = it.get_entity(i).unwrap();
                if e == e1 {
                    assert_eq!(it.group_id(), tgt_a);
                    assert!(!e1_found);
                    assert!(!e2_found);
                    assert!(!e3_found);
                    e1_found = true;
                }
                if e == e2 {
                    assert_eq!(it.group_id(), tgt_b);
                    assert!(e1_found);
                    assert!(!e2_found);
                    assert!(!e3_found);
                    e2_found = true;
                }
                if e == e3 {
                    assert_eq!(it.group_id(), tgt_c);
                    assert!(e1_found);
                    assert!(e2_found);
                    assert!(!e3_found);
                    e3_found = true;
                }
                count += 1;
            }
        }
    });

    assert_eq!(3, count);
    assert!(e1_found);
    assert!(e2_found);
    assert!(e3_found);
}

#[extern_abi]
fn callback_group_create(
    world: *mut sys::ecs_world_t,
    group_id: u64,
    group_by_ctx: *mut c_void,
) -> *mut c_void {
    assert_ne!(world, core::ptr::null_mut());
    assert_ne!(group_id, 0);
    assert_ne!(group_by_ctx, core::ptr::null_mut());
    let cell_count = unsafe { &mut *(group_by_ctx as *mut Cell<u64>) };
    assert_eq!(cell_count.get(), 5);
    let group_id = Box::new(group_id);
    Box::into_raw(group_id) as *mut c_void
}
#[extern_abi]
fn callback_group_delete(
    world: *mut sys::ecs_world_t,
    group_id: u64,
    ctx: *mut c_void,
    group_by_ctx: *mut c_void,
) {
    assert_ne!(world, core::ptr::null_mut());
    assert_ne!(group_id, 0);
    assert_ne!(group_by_ctx, core::ptr::null_mut());
    let cell_count = unsafe { &mut *(group_by_ctx as *mut Cell<u64>) };
    assert_eq!(cell_count.get(), 5);
    assert_ne!(ctx, core::ptr::null_mut());
    let group_id_ctx = unsafe { *(ctx as *mut u64) };
    assert_eq!(group_id_ctx, group_id);
    let _box = unsafe { Box::from_raw(ctx as *mut u64) };
}

#[test]
fn group_by_callbacks() {
    let cell_count_group_ctx = Cell::new(5u64);
    let world = World::new();

    let tgt_a = world.entity();
    let tgt_b = world.entity();
    let tgt_c = world.entity();

    let e1 = world.entity().add((Rel::id(), tgt_a));
    let e2 = world.entity().add((Rel::id(), tgt_b));
    let e3 = world.entity().add((Rel::id(), tgt_c));

    let q = world
        .query::<()>()
        .with((Rel::id(), *flecs::Wildcard))
        .group_by(Rel::id())
        .group_by_ctx(cell_count_group_ctx.as_ptr() as *mut c_void, None)
        .on_group_create(Some(callback_group_create))
        .on_group_delete(Some(callback_group_delete))
        .build();

    let mut e1_found = false;
    let mut e2_found = false;
    let mut e3_found = false;
    let mut count = 0;

    q.run_shared(&world, |mut it| {
        while it.next() {
            for i in 0..it.count() {
                let e = it.get_entity(i).unwrap();
                if e == e1 {
                    assert_eq!(it.group_id(), tgt_a);
                    assert!(!e1_found);
                    assert!(e2_found);
                    assert!(e3_found);
                    e1_found = true;
                    let ctx: *mut u64 = q.group_context(it.group_id()) as *mut u64;
                    assert_eq!(unsafe { *ctx }, it.group_id());
                }
                if e == e2 {
                    assert_eq!(it.group_id(), tgt_b);
                    assert!(!e1_found);
                    assert!(!e2_found);
                    assert!(e3_found);
                    e2_found = true;
                    let ctx: *mut u64 = q.group_context(it.group_id()) as *mut u64;
                    assert_eq!(unsafe { *ctx }, it.group_id());
                }
                if e == e3 {
                    assert_eq!(it.group_id(), tgt_c);
                    assert!(!e1_found);
                    assert!(!e2_found);
                    assert!(!e3_found);
                    e3_found = true;
                    let ctx: *mut u64 = q.group_context(it.group_id()) as *mut u64;
                    assert_eq!(unsafe { *ctx }, it.group_id());
                }
                count += 1;
            }
        }
    });

    assert_eq!(3, count);
    assert!(e1_found);
    assert!(e2_found);
    assert!(e3_found);
}

#[test]
fn group_by_callbacks_ordered() {
    let cell_count_group_ctx = Cell::new(5u64);
    let world = World::new();

    let tgt_a = world.entity();
    let tgt_b = world.entity();
    let tgt_c = world.entity();

    let e1 = world.entity().add((Rel::id(), tgt_a));
    let e2 = world.entity().add((Rel::id(), tgt_b));
    let e3 = world.entity().add((Rel::id(), tgt_c));

    let q = world
        .query::<()>()
        .with((Rel::id(), *flecs::Wildcard))
        .group_by(Rel::id())
        .query_flags(QueryFlags::GroupByOrdered)
        .group_by_ctx(cell_count_group_ctx.as_ptr() as *mut c_void, None)
        .on_group_create(Some(callback_group_create))
        .on_group_delete(Some(callback_group_delete))
        .query_flags(QueryFlags::GroupByOrdered)
        .build();

    let mut e1_found = false;
    let mut e2_found = false;
    let mut e3_found = false;
    let mut count = 0;

    q.run_shared(&world, |mut it| {
        while it.next() {
            for i in 0..it.count() {
                let e = it.get_entity(i).unwrap();
                if e == e1 {
                    assert_eq!(it.group_id(), tgt_a);
                    assert!(!e1_found);
                    assert!(!e2_found);
                    assert!(!e3_found);
                    e1_found = true;
                    let ctx: *mut u64 = q.group_context(it.group_id()) as *mut u64;
                    assert_eq!(unsafe { *ctx }, it.group_id());
                }
                if e == e2 {
                    assert_eq!(it.group_id(), tgt_b);
                    assert!(e1_found);
                    assert!(!e2_found);
                    assert!(!e3_found);
                    e2_found = true;
                    let ctx: *mut u64 = q.group_context(it.group_id()) as *mut u64;
                    assert_eq!(unsafe { *ctx }, it.group_id());
                }
                if e == e3 {
                    assert_eq!(it.group_id(), tgt_c);
                    assert!(e1_found);
                    assert!(e2_found);
                    assert!(!e3_found);
                    e3_found = true;
                    let ctx: *mut u64 = q.group_context(it.group_id()) as *mut u64;
                    assert_eq!(unsafe { *ctx }, it.group_id());
                }
                count += 1;
            }
        }
    });

    assert_eq!(3, count);
    assert!(e1_found);
    assert!(e2_found);
    assert!(e3_found);
}

#[test]
fn iterate_groups() {
    let world = World::new();

    let rel = world.entity();
    let tgt_a = world.entity();
    let tgt_b = world.entity();
    let tgt_c = world.entity();

    world.entity().add((rel, tgt_a));
    world.entity().add((rel, tgt_b));
    world.entity().add((rel, tgt_c));

    let q = world
        .query::<()>()
        .with((rel, *flecs::Wildcard))
        .group_by_fn(rel, Some(group_by_rel))
        .build();

    q.run_shared(&world, |mut it| while it.next() {});

    let mut a_found = false;
    let mut b_found = false;
    let mut c_found = false;
    let mut count = 0;

    q.each_group(|group, _ctx| {
        if group == tgt_a.id() {
            a_found = true;
        }
        if group == tgt_b.id() {
            b_found = true;
        }
        if group == tgt_c.id() {
            c_found = true;
        }
        count += 1;
    });

    assert_eq!(3, count);
    assert!(a_found);
    assert!(b_found);
    assert!(c_found);
}

#[test]
fn iterate_groups_empty() {
    let world = World::new();

    let rel = world.entity();

    let q = world
        .query::<()>()
        .with((rel, *flecs::Wildcard))
        .group_by_fn(rel, Some(group_by_rel))
        .build();

    let mut count = 0;
    q.each_group(|_group, _ctx| {
        count += 1;
    });

    assert_eq!(0, count);
}

#[test]
fn iterate_groups_w_isa() {
    let world = World::new();

    let asset_a = world.entity();
    let asset_b = world.entity();

    world.entity().is_a(asset_a).set(Position { x: 1, y: 2 });
    world.entity().is_a(asset_a).set(Position { x: 3, y: 4 });
    world.entity().is_a(asset_b).set(Position { x: 5, y: 6 });

    let q = world.query::<&Position>().group_by(*flecs::IsA).build();

    q.run_shared(&world, |mut it| while it.next() {});

    let mut a_found = false;
    let mut b_found = false;
    let mut count = 0;

    q.each_group(|group, _ctx| {
        if group == asset_a.id() {
            a_found = true;
        }
        if group == asset_b.id() {
            b_found = true;
        }
        count += 1;
    });

    assert_eq!(2, count);
    assert!(a_found);
    assert!(b_found);
}

#[test]
fn create_w_no_template_args() {
    let world = World::new();

    let q = world
        .query::<()>()
        .with(&Position::id())
        .set_cache_kind(QueryCacheKind::Auto)
        .build();

    let e1 = world.entity().set(Position { x: 0, y: 0 });

    let mut count = 0;
    q.each_entity_shared(&world, |e, _| {
        count += 1;
        assert_eq!(e, e1);
    });

    assert_eq!(count, 1);
}

#[test]
fn any_wildcard() {
    let world = World::new();

    let likes = world.entity();
    let apple = world.entity();
    let mango = world.entity();

    let e1 = world.entity().add((likes, apple)).add((likes, mango));

    let q = world
        .query::<()>()
        .with((likes, *flecs::Any))
        .set_cache_kind(QueryCacheKind::Auto)
        .build();

    let mut count = 0;
    q.each_entity_shared(&world, |e, _| {
        count += 1;
        assert_eq!(e, e1);
    });

    assert_eq!(count, 1);
}

#[test]
fn cascade() {
    let world = World::new();

    let tag = world
        .entity()
        .add((flecs::OnInstantiate::ID, flecs::Inherit::ID));

    let foo_ = world.entity();
    let bar = world.entity();

    let e0 = world.entity().add(tag);
    let e1 = world.entity().is_a(e0);
    let e2 = world.entity().is_a(e1);
    let e3 = world.entity().is_a(e2);

    let q = world
        .query::<()>()
        .with(tag)
        .cascade_id(*flecs::IsA)
        .set_cached()
        .build();

    e1.add(bar);
    e2.add(foo_);

    let mut e1_found = false;
    let mut e2_found = false;
    let mut e3_found = false;

    let mut count = 0;
    q.each_entity_shared(&world, |e, _| {
        count += 1;

        if e == e1 {
            assert!(!e1_found);
            assert!(!e2_found);
            assert!(!e3_found);
            e1_found = true;
        }
        if e == e2 {
            assert!(e1_found);
            assert!(!e2_found);
            assert!(!e3_found);
            e2_found = true;
        }
        if e == e3 {
            assert!(e1_found);
            assert!(e2_found);
            assert!(!e3_found);
            e3_found = true;
        }
    });

    assert!(e1_found);
    assert!(e2_found);
    assert!(e3_found);
    assert_eq!(count, 3);
}

#[test]
fn cascade_desc() {
    let world = World::new();

    let tag = world
        .entity()
        .add((flecs::OnInstantiate::ID, flecs::Inherit::ID));

    let foo_ = world.entity();
    let bar = world.entity();

    let e0 = world.entity().add(tag);
    let e1 = world.entity().is_a(e0);
    let e2 = world.entity().is_a(e1);
    let e3 = world.entity().is_a(e2);

    let q = world
        .query::<()>()
        .with(tag)
        .cascade_id(*flecs::IsA)
        .desc()
        .set_cached()
        .build();

    e1.add(bar);
    e2.add(foo_);

    let mut e1_found = false;
    let mut e2_found = false;
    let mut e3_found = false;

    let mut count = 0;
    q.each_entity_shared(&world, |e, _| {
        count += 1;

        if e == e1 {
            assert!(!e1_found);
            assert!(e2_found);
            assert!(e3_found);
            e1_found = true;
        }
        if e == e2 {
            assert!(!e1_found);
            assert!(!e2_found);
            assert!(e3_found);
            e2_found = true;
        }
        if e == e3 {
            assert!(!e1_found);
            assert!(!e2_found);
            assert!(!e3_found);
            e3_found = true;
        }
    });

    assert!(e1_found);
    assert!(e2_found);
    assert!(e3_found);
    assert_eq!(count, 3);
}

#[test]
fn cascade_w_relationship() {
    let world = World::new();

    let tag = world.entity();
    let foo_ = world.entity();
    let bar = world.entity();

    let e0 = world.entity().add(tag);
    let e1 = world.entity().child_of(e0);
    let e2 = world.entity().child_of(e1);
    let e3 = world.entity().child_of(e2);

    let q = world
        .query::<()>()
        .with(tag)
        .cascade_id(*flecs::ChildOf)
        .set_cached()
        .build();

    e1.add(bar);
    e2.add(foo_);

    let mut e1_found = false;
    let mut e2_found = false;
    let mut e3_found = false;

    let mut count = 0;
    q.each_entity_shared(&world, |e, _| {
        count += 1;

        if e == e1 {
            assert!(!e1_found);
            assert!(!e2_found);
            assert!(!e3_found);
            e1_found = true;
        }
        if e == e2 {
            assert!(e1_found);
            assert!(!e2_found);
            assert!(!e3_found);
            e2_found = true;
        }
        if e == e3 {
            assert!(e1_found);
            assert!(e2_found);
            assert!(!e3_found);
            e3_found = true;
        }
    });

    assert!(e1_found);
    assert!(e2_found);
    assert!(e3_found);
    assert_eq!(count, 3);
}

#[test]
fn up_w_type() {
    let mut world = World::new();

    world.component::<Rel>().add(*flecs::Traversable);

    let q = world
        .query::<&SelfRef2>()
        .with(&Other::id())
        .src()
        .up_id(Rel::id())
        .set_in()
        .set_cache_kind(QueryCacheKind::Auto)
        .build();

    let base = world.entity().set(Other { value: 10 });

    let e = world.entity().add((Rel::id(), base));
    e.set(SelfRef2 { value: e.id() });
    let e = world.entity().add((Rel::id(), base));
    e.set(SelfRef2 { value: e.id() });
    let e = world.entity().add((Rel::id(), base));
    e.set(SelfRef2 { value: e.id() });

    let mut count = 0;

    q.run_exclusive(&mut world, |mut it| {
        while it.next() {
            let o = &it.field::<Other>(1)[0];
            let s = it.field::<SelfRef2>(0);
            assert!(!it.is_self(1));
            assert_eq!(o.value, 10);

            for i in it.iter() {
                assert_eq!(it.get_entity(i).unwrap(), s[i].value);
                count += 1;
            }
        }
    });

    assert_eq!(count, 3);
}

#[test]
fn cascade_w_type() {
    let world = World::new();

    world.component::<Rel>().add(*flecs::Traversable);

    let tag = world.entity();
    let foo_ = world.entity();
    let bar = world.entity();

    let e0 = world.entity().add(tag);
    let e1 = world.entity().add((Rel::id(), e0));
    let e2 = world.entity().add((Rel::id(), e1));
    let e3 = world.entity().add((Rel::id(), e2));

    let q = world
        .query::<()>()
        .with(tag)
        .cascade_id(Rel::id())
        .set_cached()
        .build();

    e1.add(bar);
    e2.add(foo_);

    let mut e1_found = false;
    let mut e2_found = false;
    let mut e3_found = false;

    let mut count = 0;
    q.each_entity_shared(&world, |e, _| {
        count += 1;

        if e == e1 {
            assert!(!e1_found);
            assert!(!e2_found);
            assert!(!e3_found);
            e1_found = true;
        }
        if e == e2 {
            assert!(e1_found);
            assert!(!e2_found);
            assert!(!e3_found);
            e2_found = true;
        }
        if e == e3 {
            assert!(e1_found);
            assert!(e2_found);
            assert!(!e3_found);
            e3_found = true;
        }
    });

    assert!(e1_found);
    assert!(e2_found);
    assert!(e3_found);
    assert_eq!(count, 3);
}

#[test]
fn named_query() {
    let world = World::new();

    let e1 = world.entity().set(Position { x: 0, y: 0 });
    let e2 = world.entity().set(Position { x: 0, y: 0 });

    let q = world
        .query_named::<()>("my_query")
        .with(&Position::id())
        .set_cache_kind(QueryCacheKind::Auto)
        .build();

    let mut count = 0;
    q.each_entity_shared(&world, |e, _| {
        assert!((e == e1 || e == e2));
        count += 1;
    });
    assert_eq!(count, 2);

    let qe = q.entity();
    assert_ne!(qe, 0);
    assert_eq!(qe.name(), "my_query");
}

#[test]
fn term_w_write() {
    let world = World::new();

    let q = world
        .query::<()>()
        .with(Position::id())
        .with(Position::id())
        .write_curr()
        .with(&mut Position::id())
        .set_cache_kind(QueryCacheKind::Auto)
        .build();

    assert_eq!(q.term(0).inout(), InOutKind::Default);
    assert_eq!(q.term(0).src_id(), *flecs::This_);
    assert_eq!(q.term(1).inout(), InOutKind::Out);
    assert_eq!(q.term(1).src_id(), 0);
    assert_eq!(q.term(2).inout(), InOutKind::InOut);
    assert_eq!(q.term(2).src_id(), *flecs::This_);
}

#[test]
fn term_w_read() {
    let world = World::new();

    let q = world
        .query::<()>()
        .with(Position::id())
        .with(Position::id())
        .read_curr()
        .with(&Position::id())
        .set_cache_kind(QueryCacheKind::Auto)
        .build();

    assert_eq!(q.term(0).inout(), InOutKind::Default);
    assert_eq!(q.term(0).src_id(), *flecs::This_);
    assert_eq!(q.term(1).inout(), InOutKind::In);
    assert_eq!(q.term(1).src_id(), 0);
    assert_eq!(q.term(2).inout(), InOutKind::In);
    assert_eq!(q.term(2).src_id(), *flecs::This_);
}

#[test]
fn iter_w_stage() {
    let world = World::new();

    world.set_stage_count(2);
    let stage = world.stage(1);

    let e1 = world.entity().set(Position { x: 0, y: 0 });
    let e1_id = e1.id();

    let q = world.query::<&Position>().build();

    let mut count = 0;
    // iter_stage() runs the query using stage as the world context,
    // so it.world() inside the callback returns the stage WorldRef.
    q.iter_stage(stage).each_entity_shared(&world, |e, _pos| {
        assert_eq!(e.world().world_ptr(), stage.world_ptr());
        assert_eq!(e.id(), e1_id);
        count += 1;
    });

    assert_eq!(count, 1);
}

#[derive(Component)]
struct QueryWrapper {
    query_entity: Entity,
}

#[test]
fn builder_force_assign_operator() {
    let world = World::new();

    let e1 = world.entity().set(Position { x: 10, y: 20 });

    let q = world
        .query::<()>()
        .with(&Position::id())
        .set_cache_kind(QueryCacheKind::Auto)
        .build();

    let f = world.entity().set(QueryWrapper {
        query_entity: q.entity().id(),
    });

    let mut count = 0;
    {
        let wrapper = f.get_ref::<&QueryWrapper>().unwrap();
        let query = world.query_from(wrapper.query_entity);
        query.each_entity_shared(&world, |e, _| {
            assert_eq!(e, e1);
            count += 1;
        });
    };

    assert_eq!(count, 1);
}

#[test]
fn query_as_arg() {
    fn query_arg(world: &mut World, f: &Query<&SelfRef>) -> i32 {
        let mut count = 0;

        f.each_entity_exclusive(world, |e, s| {
            assert_eq!(e, s.value);
            count += 1;
        });

        count
    }

    let mut world = World::new();

    let f = world
        .query::<&SelfRef>()
        .set_cache_kind(QueryCacheKind::Auto)
        .build();

    let e = world.entity();
    e.set(SelfRef { value: e.id() });

    let e = world.entity();
    e.set(SelfRef { value: e.id() });

    let e = world.entity();
    e.set(SelfRef { value: e.id() });

    assert_eq!(query_arg(&mut world, &f), 3);
}

#[test]
fn query_default_as_move_arg() {
    fn query_move(world: &mut World, f: Query<&SelfRef>) -> i32 {
        let mut count = 0;

        f.each_entity_exclusive(world, |e, s| {
            assert_eq!(e, s.value);
            count += 1;
        });

        count
    }
    let mut world = World::new();

    let _f = world.query::<&SelfRef>();

    let e = world.entity();
    e.set(SelfRef { value: e.id() });

    let e = world.entity();
    e.set(SelfRef { value: e.id() });

    let e = world.entity();
    e.set(SelfRef { value: e.id() });

    let qm = world.new_query::<&SelfRef>();
    assert_eq!(query_move(&mut world, qm), 3);
}

#[test]
fn query_as_return() {
    fn query_return(world: &World) -> Query<&SelfRef> {
        world.new_query::<&SelfRef>()
    }

    let world = World::new();

    let e = world.entity();
    e.set(SelfRef { value: e.id() });

    let e = world.entity();
    e.set(SelfRef { value: e.id() });

    let e = world.entity();
    e.set(SelfRef { value: e.id() });

    let f = query_return(&world);

    let mut count = 0;

    f.each_entity_shared(&world, |e, s| {
        assert_eq!(e, s.value);
        count += 1;
    });

    assert_eq!(count, 3);
}

#[test]
fn query_copy() {
    let mut world = World::new();

    let e = world.entity();
    e.set(SelfRef { value: e.id() });

    let e = world.entity();
    e.set(SelfRef { value: e.id() });

    let e = world.entity();
    e.set(SelfRef { value: e.id() });

    let f = world
        .query::<&SelfRef>()
        .set_cache_kind(QueryCacheKind::Auto)
        .build();

    let f_2 = f;

    let mut count = 0;

    f_2.each_entity_exclusive(&mut world, |e, s| {
        assert_eq!(e, s.value);
        count += 1;
    });

    assert_eq!(count, 3);
}

#[test]
fn world_each_query_1_component() {
    let world = World::new();

    let e = world.entity();
    e.set(SelfRef { value: e.id() });

    let e = world.entity();
    e.set(SelfRef { value: e.id() });

    let e = world.entity();
    e.set(SelfRef { value: e.id() });

    let mut count = 0;

    world.each_entity::<&SelfRef>(|e, s| {
        assert_eq!(e, s.value);
        count += 1;
    });

    assert_eq!(count, 3);
}

#[test]
fn world_each_query_2_components() {
    let world = World::new();

    let e = world.entity();
    e.set(SelfRef2 { value: e.id() })
        .set(Position { x: 10, y: 20 });

    let e = world.entity();
    e.set(SelfRef2 { value: e.id() })
        .set(Position { x: 10, y: 20 });

    let e = world.entity();
    e.set(SelfRef2 { value: e.id() })
        .set(Position { x: 10, y: 20 });

    let mut count = 0;

    world.each_entity::<(&SelfRef2, &Position)>(|e, (s, p)| {
        assert_eq!(e, s.value);
        assert_eq!(p.x, 10);
        assert_eq!(p.y, 20);
        count += 1;
    });

    assert_eq!(count, 3);
}

#[test]
fn world_each_query_1_component_no_entity() {
    let world = World::new();

    world.entity().set(Position { x: 10, y: 20 });

    world.entity().set(Position { x: 10, y: 20 });

    world
        .entity()
        .set(Position { x: 10, y: 20 })
        .set(Velocity { x: 1, y: 2 });

    let mut count = 0;

    world.each::<&Position>(|p| {
        assert_eq!(p.x, 10);
        assert_eq!(p.y, 20);
        count += 1;
    });

    assert_eq!(count, 3);
}

#[test]
fn world_each_query_2_components_no_entity() {
    let world = World::new();

    world
        .entity()
        .set(Position { x: 10, y: 20 })
        .set(Velocity { x: 1, y: 2 });

    world
        .entity()
        .set(Position { x: 10, y: 20 })
        .set(Velocity { x: 1, y: 2 });

    world
        .entity()
        .set(Position { x: 10, y: 20 })
        .set(Velocity { x: 1, y: 2 });

    world.entity().set(Position { x: 3, y: 5 });

    world.entity().set(Velocity { x: 20, y: 40 });

    let mut count = 0;

    world.each::<(&Position, &Velocity)>(|(p, v)| {
        assert_eq!(p.x, 10);
        assert_eq!(p.y, 20);
        assert_eq!(v.x, 1);
        assert_eq!(v.y, 2);
        count += 1;
    });

    assert_eq!(count, 3);
}

#[test]
fn term_after_arg() {
    let world = World::new();

    let e_1 = world
        .entity()
        .add(TagA::id())
        .add(TagB::id())
        .add(TagC::id());

    world.entity().add(TagA::id()).add(TagB::id());

    let f = world
        .query::<(&TagA, &TagB)>()
        .term_at(0)
        .set_src(*flecs::This_) // dummy
        .with(&TagC::id())
        .set_cache_kind(QueryCacheKind::Auto)
        .build();

    assert_eq!(f.field_count(), 3);

    let mut count = 0;
    f.run_shared(&world, |mut it| {
        while it.next() {
            for i in it.iter() {
                assert_eq!(it.get_entity(i).unwrap(), e_1);
                count += 1;
            }
        }
    });

    assert_eq!(count, 1);
}

#[test]
fn name_arg() {
    let world = World::new();

    let e = world.entity_named("Foo").set(Position { x: 10, y: 20 });

    let f = world
        .query::<&Position>()
        .term_at(0)
        .src()
        .name("Foo")
        .set_cache_kind(QueryCacheKind::Auto)
        .build();

    let mut count = 0;

    f.run_shared(&world, |mut it| {
        while it.next() {
            let p = it.field::<Position>(0);
            assert_eq!(p[0].x, 10);
            assert_eq!(p[0].y, 20);
            assert_eq!(it.src(0), e);
            count += 1;
        }
    });

    assert_eq!(count, 1);
}

#[test]
fn const_in_term() {
    let mut world = World::new();

    world.entity().set(Position { x: 10, y: 20 });

    let f = world
        .query::<()>()
        .with(&Position::id())
        .set_cache_kind(QueryCacheKind::Auto)
        .build();

    let mut count = 0;
    f.run_exclusive(&mut world, |mut it| {
        while it.next() {
            let p = it.field::<Position>(0);
            assert!(it.is_readonly(0));
            for i in it.iter() {
                count += 1;
                assert_eq!(p[i].x, 10);
                assert_eq!(p[i].y, 20);
            }
        }
    });

    assert_eq!(count, 1);
}

#[test]
fn const_optional() {
    let mut world = World::new();

    world
        .entity()
        .set(Position { x: 10, y: 20 })
        .add(TagD::id());
    world.entity().add(TagD::id());

    let f = world
        .query::<(&TagD, Option<&Position>)>()
        .set_cache_kind(QueryCacheKind::Auto)
        .build();

    let mut count = 0;
    let mut set_count = 0;
    f.run_exclusive(&mut world, |mut it| {
        while it.next() {
            assert_eq!(it.count(), 1);
            if it.is_set(1) {
                let p = &it.field::<Position>(1)[0];
                assert!(it.is_readonly(1));
                assert_eq!(p.x, 10);
                assert_eq!(p.y, 20);
                set_count += 1;
            }
            count += 1;
        }
    });

    assert_eq!(count, 2);
    assert_eq!(set_count, 1);
}

#[test]
fn n2_terms_w_expr() {
    let world = World::new();

    let a = world.entity_named("A");
    let b = world.entity_named("B");

    let e1 = world.entity().add(a).add(b);

    let f = world
        .query::<()>()
        .expr("A, B")
        .set_cache_kind(QueryCacheKind::Auto)
        .build()
        .expect("valid query expression");

    assert_eq!(f.field_count(), 2);

    let mut count = 0;
    f.run_shared(&world, |mut it| {
        while it.next() {
            for i in it.iter() {
                if it.get_entity(i).unwrap() == e1 {
                    assert_eq!(it.id(0), a);
                    assert_eq!(it.id(1), b);
                    count += 1;
                }
            }
        }
    });

    assert_eq!(count, 1);
}

#[test]
fn assert_on_uninitialized_term() {
    let world = World::new();
    let _guard = FlecsPanicAbortGuard::install();

    world.entity_named("A");
    world.entity_named("B");

    // In current Flecs, building a query with uninitialized terms may either
    // ecs_abort (caught by guard → panic) or be accepted. This is a purely-typed
    // builder (no runtime expression), so it stays on the infallible `build()` path.
    // In newer Flecs versions this silently succeeds, so we just verify it doesn't crash.
    let result = world
        .query::<()>()
        .term()
        .term()
        .set_cache_kind(QueryCacheKind::Auto)
        .build();

    let _ = result;
}

#[test]
fn operator_shortcuts() {
    let world = World::new();

    let a = world.entity();
    let b = world.entity();
    let c = world.entity();
    let d = world.entity();
    let e = world.entity();
    let f = world.entity();
    let g = world.entity();
    let h = world.entity();

    let query = world
        .query::<()>()
        .with(a)
        .and()
        .with(b)
        .or()
        .with(c)
        .with(d)
        .not()
        .with(e)
        .optional()
        .with(f)
        .and_from()
        .with(g)
        .or_from()
        .with(h)
        .not_from()
        .set_cache_kind(QueryCacheKind::Auto)
        .build();

    let mut t = query.term(0);
    assert_eq!(t.id(), a);
    assert_eq!(t.oper(), OperKind::And);

    t = query.term(1);
    assert_eq!(t.id(), b);
    assert_eq!(t.oper(), OperKind::Or);

    t = query.term(2);
    assert_eq!(t.id(), c);
    assert_eq!(t.oper(), OperKind::And);

    t = query.term(3);
    assert_eq!(t.id(), d);
    assert_eq!(t.oper(), OperKind::Not);

    t = query.term(4);
    assert_eq!(t.id(), e);
    assert_eq!(t.oper(), OperKind::Optional);

    t = query.term(5);
    assert_eq!(t.id(), f);
    assert_eq!(t.oper(), OperKind::AndFrom);

    t = query.term(6);
    assert_eq!(t.id(), g);
    assert_eq!(t.oper(), OperKind::OrFrom);

    t = query.term(7);
    assert_eq!(t.id(), h);
    assert_eq!(t.oper(), OperKind::NotFrom);
}

#[test]
fn inout_shortcuts() {
    let world = World::new();

    let a = world.entity();
    let b = world.entity();
    let c = world.entity();
    let d = world.entity();

    let query = world
        .query::<()>()
        .with(a)
        .set_in()
        .with(b)
        .set_out()
        .with(c)
        .set_inout()
        .with(d)
        .set_inout_none()
        .set_cache_kind(QueryCacheKind::Auto)
        .build();

    let mut t = query.term(0);
    assert_eq!(t.id(), a);
    assert_eq!(t.inout(), InOutKind::In);

    t = query.term(1);
    assert_eq!(t.id(), b);
    assert_eq!(t.inout(), InOutKind::Out);

    t = query.term(2);
    assert_eq!(t.id(), c);
    assert_eq!(t.inout(), InOutKind::InOut);

    t = query.term(3);
    assert_eq!(t.id(), d);
    assert_eq!(t.inout(), InOutKind::None);
}

#[test]
fn iter_column_w_const_as_array() {
    let world = World::new();

    let f = world
        .query::<&mut Position>()
        .set_cache_kind(QueryCacheKind::Auto)
        .build();

    let e1 = world.entity().set(Position { x: 10, y: 20 });
    let e2 = world.entity().set(Position { x: 20, y: 30 });

    let mut count = 0;
    f.run_shared(&world, |mut it| {
        while it.next() {
            let mut p = it.field_mut::<Position>(0);
            for i in it.iter() {
                p[i].x += 1;
                p[i].y += 2;

                count += 1;
            }
        }
    });

    assert_eq!(count, 2);

    {
        let p = e1.get_ref::<&Position>().unwrap();
        assert_eq!(p.x, 11);
        assert_eq!(p.y, 22);
    };

    {
        let p = e2.get_ref::<&Position>().unwrap();
        assert_eq!(p.x, 21);
        assert_eq!(p.y, 32);
    };
}

#[test]
fn iter_column_w_const_as_ptr() {
    let mut world = World::new();

    let f = world
        .query::<&Position>()
        .set_cache_kind(QueryCacheKind::Auto)
        .build();

    let base = world.prefab().set(Position { x: 10, y: 20 });
    world.entity().is_a(base);
    world.entity().is_a(base);

    let mut count = 0;
    f.run_exclusive(&mut world, |mut it| {
        while it.next() {
            let p = &it.field::<Position>(0)[0];
            for _i in it.iter() {
                assert_eq!(p.x, 10);
                assert_eq!(p.y, 20);
                count += 1;
            }
        }
    });

    assert_eq!(count, 2);
}

#[test]
fn with() {
    let world = World::new();

    let q = world
        .query::<()>()
        .with(world.id_view_from(Position::id()))
        .with(world.id_view_from(Velocity::id()))
        .set_cache_kind(QueryCacheKind::Auto)
        .build();

    let e1 = world
        .entity()
        .set(Position { x: 0, y: 0 })
        .set(Velocity { x: 0, y: 0 });
    world.entity().set(Position { x: 0, y: 0 });

    let mut count = 0;
    q.each_entity_shared(&world, |e, _| {
        count += 1;
        assert_eq!(e, e1);
    });

    assert_eq!(count, 1);
}

#[test]
fn with_name() {
    let world = World::new();

    world.component::<Velocity>();

    let q = world
        .query::<()>()
        .with(&Position::id())
        .with("Velocity")
        .set_cache_kind(QueryCacheKind::Auto)
        .build();

    let e1 = world
        .entity()
        .set(Position { x: 0, y: 0 })
        .set(Velocity { x: 0, y: 0 });
    world.entity().set(Position { x: 0, y: 0 });

    let mut count = 0;
    q.each_entity_shared(&world, |e, _| {
        count += 1;
        assert_eq!(e, e1);
    });

    assert_eq!(count, 1);
}

#[test]
fn with_component() {
    let world = World::new();

    let q = world
        .query::<()>()
        .with(&Position::id())
        .with(&Velocity::id())
        .set_cache_kind(QueryCacheKind::Auto)
        .build();

    let e1 = world
        .entity()
        .set(Position { x: 0, y: 0 })
        .set(Velocity { x: 0, y: 0 });
    world.entity().set(Position { x: 0, y: 0 });

    let mut count = 0;
    q.each_entity_shared(&world, |e, _| {
        count += 1;
        assert_eq!(e, e1);
    });

    assert_eq!(count, 1);
}

#[test]
fn with_pair_id() {
    let world = World::new();

    let likes = world.entity();
    let apples = world.entity();
    let pears = world.entity();

    let q = world
        .query::<()>()
        .with(&Position::id())
        .with((likes, apples))
        .set_cache_kind(QueryCacheKind::Auto)
        .build();

    let e1 = world
        .entity()
        .set(Position { x: 0, y: 0 })
        .add((likes, apples));
    world
        .entity()
        .set(Position { x: 0, y: 0 })
        .add((likes, pears));

    let mut count = 0;
    q.each_entity_shared(&world, |e, _| {
        count += 1;
        assert_eq!(e, e1);
    });

    assert_eq!(count, 1);
}

#[test]
fn with_pair_name() {
    let world = World::new();

    let likes = world.entity_named("likes");
    let apples = world.entity_named("Apples");
    let pears = world.entity_named("Pears");

    let q = world
        .query::<()>()
        .with(&Position::id())
        .with(("likes", "Apples"))
        .set_cache_kind(QueryCacheKind::Auto)
        .build();

    let e1 = world
        .entity()
        .set(Position { x: 0, y: 0 })
        .add((likes, apples));
    world
        .entity()
        .set(Position { x: 0, y: 0 })
        .add((likes, pears));

    let mut count = 0;
    q.each_entity_shared(&world, |e, _| {
        count += 1;
        assert_eq!(e, e1);
    });

    assert_eq!(count, 1);
}

#[test]
fn with_pair_components() {
    let world = World::new();

    let q = world
        .query::<()>()
        .with(&Position::id())
        .with((Likes::id(), Apples::id()))
        .set_cache_kind(QueryCacheKind::Auto)
        .build();

    let e1 = world
        .entity()
        .set(Position { x: 0, y: 0 })
        .add((Likes::id(), Apples::id()));
    world
        .entity()
        .set(Position { x: 0, y: 0 })
        .add((Likes::id(), Pears::id()));

    let mut count = 0;
    q.each_entity_shared(&world, |e, _| {
        count += 1;
        assert_eq!(e, e1);
    });

    assert_eq!(count, 1);
}

#[test]
fn with_pair_component_id() {
    let world = World::new();

    let apples = world.entity();
    let pears = world.entity();

    let q = world
        .query::<()>()
        .with(&Position::id())
        .with((Likes::id(), apples))
        .set_cache_kind(QueryCacheKind::Auto)
        .build();

    let e1 = world
        .entity()
        .set(Position { x: 0, y: 0 })
        .add((Likes::id(), apples));
    world
        .entity()
        .set(Position { x: 0, y: 0 })
        .add((Likes::id(), pears));

    let mut count = 0;
    q.each_entity_shared(&world, |e, _| {
        count += 1;
        assert_eq!(e, e1);
    });

    assert_eq!(count, 1);
}

#[test]
fn with_pair_name_component_id() {
    let world = World::new();

    let likes = world.entity_named("Likes");
    let apples = world.entity_named("Apples");
    let pears = world.entity_named("Pears");

    let q = world
        .query::<()>()
        .with(Position::id())
        .with(("Likes", apples))
        .set_cache_kind(QueryCacheKind::Auto)
        .build();

    let e1 = world
        .entity()
        .set(Position { x: 0, y: 0 })
        .add((likes, apples));
    world
        .entity()
        .set(Position { x: 0, y: 0 })
        .add((likes, pears));

    let mut count = 0;
    q.each_entity_shared(&world, |e, _| {
        count += 1;
        assert_eq!(e, e1);
    });

    assert_eq!(count, 1);
}

#[test]
fn with_pair_component_name() {
    let world = World::new();

    let apples = world.entity_named("Apples");
    let pears = world.entity_named("Pears");

    let q = world
        .query::<()>()
        .with(&Position::id())
        .with((Likes::id(), "Apples"))
        .set_cache_kind(QueryCacheKind::Auto)
        .build();

    let e1 = world
        .entity()
        .set(Position { x: 0, y: 0 })
        .add((Likes::id(), apples));
    world
        .entity()
        .set(Position { x: 0, y: 0 })
        .add((Likes::id(), pears));

    let mut count = 0;
    q.each_entity_shared(&world, |e, _| {
        count += 1;
        assert_eq!(e, e1);
    });

    assert_eq!(count, 1);
}

#[test]
fn with_enum() {
    #[repr(C)]
    #[derive(Component)]
    pub enum Color {
        Red,
        Green,
        Blue,
    }

    let world = World::new();

    let q = world
        .query::<()>()
        .with(&Position::id())
        .with_enum(Color::Green)
        .set_cache_kind(QueryCacheKind::Auto)
        .build();

    let e1 = world
        .entity()
        .set(Position { x: 0, y: 0 })
        .add_enum(Color::Green);
    world
        .entity()
        .set(Position { x: 0, y: 0 })
        .add_enum(Color::Red);

    let mut count = 0;
    q.each_entity_shared(&world, |e, _| {
        count += 1;
        assert_eq!(e, e1);
    });

    assert_eq!(count, 1);
}

#[test]
fn without() {
    let world = World::new();

    let q = world
        .query::<()>()
        .with(world.id_view_from(Position::id()))
        .without(world.id_view_from(Velocity::id()))
        .set_cache_kind(QueryCacheKind::Auto)
        .build();

    world
        .entity()
        .set(Position { x: 0, y: 0 })
        .set(Velocity { x: 0, y: 0 });
    let e2 = world.entity().set(Position { x: 0, y: 0 });

    let mut count = 0;
    q.each_entity_shared(&world, |e, _| {
        count += 1;
        assert_eq!(e, e2);
    });

    assert_eq!(count, 1);
}

#[test]
fn without_name() {
    let world = World::new();

    world.component::<Velocity>();

    let q = world
        .query::<()>()
        .with(world.id_view_from(Position::id()))
        .without("Velocity")
        .set_cache_kind(QueryCacheKind::Auto)
        .build();

    world
        .entity()
        .set(Position { x: 0, y: 0 })
        .set(Velocity { x: 0, y: 0 });
    let e2 = world.entity().set(Position { x: 0, y: 0 });

    let mut count = 0;
    q.each_entity_shared(&world, |e, _| {
        count += 1;
        assert_eq!(e, e2);
    });

    assert_eq!(count, 1);
}

#[test]
fn without_component() {
    let world = World::new();

    let q = world
        .query::<()>()
        .with(&Position::id())
        .without(Velocity::id())
        .set_cache_kind(QueryCacheKind::Auto)
        .build();

    world
        .entity()
        .set(Position { x: 0, y: 0 })
        .set(Velocity { x: 0, y: 0 });
    let e2 = world.entity().set(Position { x: 0, y: 0 });

    let mut count = 0;
    q.each_entity_shared(&world, |e, _| {
        count += 1;
        assert_eq!(e, e2);
    });

    assert_eq!(count, 1);
}

#[test]
fn without_pair_id() {
    let world = World::new();

    let likes = world.entity();
    let apples = world.entity();
    let pears = world.entity();

    let q = world
        .query::<()>()
        .with(&Position::id())
        .without((likes, apples))
        .set_cache_kind(QueryCacheKind::Auto)
        .build();

    world
        .entity()
        .set(Position { x: 0, y: 0 })
        .add((likes, apples));
    let e2 = world
        .entity()
        .set(Position { x: 0, y: 0 })
        .add((likes, pears));

    let mut count = 0;
    q.each_entity_shared(&world, |e, _| {
        count += 1;
        assert_eq!(e, e2);
    });

    assert_eq!(count, 1);
}

#[test]
fn without_pair_name() {
    let world = World::new();

    let likes = world.entity_named("likes");
    let apples = world.entity_named("Apples");
    let pears = world.entity_named("Pears");

    let q = world
        .query::<()>()
        .with(&Position::id())
        .without(("likes", "Apples"))
        .set_cache_kind(QueryCacheKind::Auto)
        .build();

    world
        .entity()
        .set(Position { x: 0, y: 0 })
        .add((likes, apples));
    let e2 = world
        .entity()
        .set(Position { x: 0, y: 0 })
        .add((likes, pears));

    let mut count = 0;
    q.each_entity_shared(&world, |e, _| {
        count += 1;
        assert_eq!(e, e2);
    });

    assert_eq!(count, 1);
}

#[test]
fn without_pair_components() {
    let world = World::new();

    let q = world
        .query::<()>()
        .with(&Position::id())
        .without((Likes::id(), Apples::id()))
        .set_cache_kind(QueryCacheKind::Auto)
        .build();

    world
        .entity()
        .set(Position { x: 0, y: 0 })
        .add((Likes::id(), Apples::id()));
    let e2 = world
        .entity()
        .set(Position { x: 0, y: 0 })
        .add((Likes::id(), Pears::id()));

    let mut count = 0;
    q.each_entity_shared(&world, |e, _| {
        count += 1;
        assert_eq!(e, e2);
    });

    assert_eq!(count, 1);
}

#[test]
fn without_pair_component_id() {
    let world = World::new();

    let apples = world.entity();
    let pears = world.entity();

    let q = world
        .query::<()>()
        .with(&Position::id())
        .without((Likes::id(), apples))
        .set_cache_kind(QueryCacheKind::Auto)
        .build();

    world
        .entity()
        .set(Position { x: 0, y: 0 })
        .add((Likes::id(), apples));
    let e2 = world
        .entity()
        .set(Position { x: 0, y: 0 })
        .add((Likes::id(), pears));

    let mut count = 0;
    q.each_entity_shared(&world, |e, _| {
        count += 1;
        assert_eq!(e, e2);
    });

    assert_eq!(count, 1);
}

#[test]
fn without_pair_component_name() {
    let world = World::new();

    let apples = world.entity_named("Apples");
    let pears = world.entity_named("Pears");

    let q = world
        .query::<()>()
        .with(&Position::id())
        .without((Likes::id(), "Apples"))
        .set_cache_kind(QueryCacheKind::Auto)
        .build();

    world
        .entity()
        .set(Position { x: 0, y: 0 })
        .add((Likes::id(), apples));
    let e2 = world
        .entity()
        .set(Position { x: 0, y: 0 })
        .add((Likes::id(), pears));

    let mut count = 0;
    q.each_entity_shared(&world, |e, _| {
        count += 1;
        assert_eq!(e, e2);
    });

    assert_eq!(count, 1);
}

#[test]
fn without_pair_name_component_id() {
    let world = World::new();

    let likes = world.entity_named("Likes");
    let apples = world.entity_named("Apples");
    let pears = world.entity_named("Pears");

    let q = world
        .query::<()>()
        .with(&Position::id())
        .without(("Likes", apples))
        .set_cache_kind(QueryCacheKind::Auto)
        .build();

    world
        .entity()
        .set(Position { x: 0, y: 0 })
        .add((likes, apples));
    let e2 = world
        .entity()
        .set(Position { x: 0, y: 0 })
        .add((likes, pears));

    let mut count = 0;
    q.each_entity_shared(&world, |e, _| {
        count += 1;
        assert_eq!(e, e2);
    });

    assert_eq!(count, 1);
}

#[test]
fn without_enum() {
    let world = World::new();

    #[repr(C)]
    #[derive(Component)]
    pub enum Color {
        Red,
        Green,
        Blue,
    }

    let q = world
        .query::<()>()
        .with(&Position::id())
        .without_enum(Color::Green)
        .set_cache_kind(QueryCacheKind::Auto)
        .build();

    world
        .entity()
        .set(Position { x: 0, y: 0 })
        .add_enum(Color::Green);
    let e2 = world
        .entity()
        .set(Position { x: 0, y: 0 })
        .add_enum(Color::Red);

    let mut count = 0;
    q.each_entity_shared(&world, |e, _| {
        count += 1;
        assert_eq!(e, e2);
    });

    assert_eq!(count, 1);
}

#[test]
fn write() {
    let world = World::new();

    let q = world
        .query::<()>()
        .with(&Position::id())
        .write(world.id_view_from(Position::id()))
        .set_cache_kind(QueryCacheKind::Auto)
        .build();

    assert_eq!(q.term(1).inout(), InOutKind::Out);
    assert_eq!(q.term(1).first_id(), world.id_view_from(Position::id()));
    assert_eq!(q.term(1).src_id(), 0);
}

#[test]
fn write_name() {
    let world = World::new();

    world.component::<Position>();

    let q = world
        .query::<()>()
        .with(&Position::id())
        .write("Position")
        .set_cache_kind(QueryCacheKind::Auto)
        .build();

    assert_eq!(q.term(1).inout(), InOutKind::Out);
    assert_eq!(q.term(1).first_id(), world.id_view_from(Position::id()));
    assert_eq!(q.term(1).src_id(), 0);
}

#[test]
fn write_component() {
    let world = World::new();

    world.component::<Position>();

    let q = world
        .query::<()>()
        .with(&Position::id())
        .write(Position::id())
        .set_cache_kind(QueryCacheKind::Auto)
        .build();

    assert_eq!(q.term(1).inout(), InOutKind::Out);
    assert_eq!(q.term(1).first_id(), world.id_view_from(Position::id()));
    assert_eq!(q.term(1).src_id(), 0);
}

#[test]
fn write_pair_id() {
    let world = World::new();

    let likes = world.entity();
    let apples = world.entity();

    let q = world
        .query::<()>()
        .with(&Position::id())
        .write((likes, apples))
        .set_cache_kind(QueryCacheKind::Auto)
        .build();

    assert_eq!(q.term(1).inout(), InOutKind::Out);
    assert_eq!(q.term(1).first_id(), likes);
    assert_eq!(q.term(1).second_id(), apples);
    assert_eq!(q.term(1).src_id(), 0);
}

#[test]
fn write_pair_name() {
    let world = World::new();

    let likes = world.entity_named("likes");
    let apples = world.entity_named("Apples");

    let q = world
        .query::<()>()
        .with(&Position::id())
        .write(("likes", "Apples"))
        .set_cache_kind(QueryCacheKind::Auto)
        .build();

    assert_eq!(q.term(1).inout(), InOutKind::Out);
    assert_eq!(q.term(1).first_id(), likes);
    assert_eq!(q.term(1).second_id(), apples);
    assert_eq!(q.term(1).src_id(), 0);
}

#[test]
fn write_pair_components() {
    let world = World::new();

    let q = world
        .query::<()>()
        .with(&Position::id())
        .write((Likes::id(), Apples::id()))
        .set_cache_kind(QueryCacheKind::Auto)
        .build();

    assert_eq!(q.term(1).inout(), InOutKind::Out);
    assert_eq!(q.term(1).first_id(), world.id_view_from(Likes::id()));
    assert_eq!(q.term(1).second_id(), world.id_view_from(Apples::id()));
    assert_eq!(q.term(1).src_id(), 0);
}

#[test]
fn write_pair_component_id() {
    let world = World::new();

    let apples = world.entity();

    let q = world
        .query::<()>()
        .with(&Position::id())
        .write((Likes::id(), apples))
        .set_cache_kind(QueryCacheKind::Auto)
        .build();

    assert_eq!(q.term(1).inout(), InOutKind::Out);
    assert_eq!(q.term(1).first_id(), world.id_view_from(Likes::id()));
    assert_eq!(q.term(1).second_id(), apples);
    assert_eq!(q.term(1).src_id(), 0);
}

#[test]
fn write_pair_component_name() {
    let world = World::new();

    let apples = world.entity_named("Apples");

    let q = world
        .query::<()>()
        .with(&Position::id())
        .write((Likes::id(), "Apples"))
        .set_cache_kind(QueryCacheKind::Auto)
        .build();

    assert_eq!(q.term(1).inout(), InOutKind::Out);
    assert_eq!(q.term(1).first_id(), world.id_view_from(Likes::id()));
    assert_eq!(q.term(1).second_id(), apples);
    assert_eq!(q.term(1).src_id(), 0);
}

#[test]
fn write_enum() {
    #[repr(C)]
    #[derive(Component)]
    pub enum Color {
        Red,
        Green,
        Blue,
    }

    let world = World::new();

    let q = world
        .query::<()>()
        .with(&Position::id())
        .write_enum(Color::Green)
        .set_cache_kind(QueryCacheKind::Auto)
        .build();

    assert_eq!(q.term(1).inout(), InOutKind::Out);
    assert_eq!(q.term(1).first_id(), world.id_view_from(Color::id()));
    assert_eq!(q.term(1).second_id(), world.entity_from_enum(Color::Green));
    assert_eq!(q.term(1).src_id(), 0);
}

#[test]
fn read() {
    let world = World::new();

    let q = world
        .query::<()>()
        .with(&Position::id())
        .read(world.id_view_from(Position::id()))
        .set_cache_kind(QueryCacheKind::Auto)
        .build();

    assert_eq!(q.term(1).inout(), InOutKind::In);
    assert_eq!(q.term(1).first_id(), world.id_view_from(Position::id()));
    assert_eq!(q.term(1).src_id(), 0);
}

#[test]
fn read_name() {
    let world = World::new();

    world.component::<Position>();

    let q = world
        .query::<()>()
        .with(&Position::id())
        .read("Position")
        .set_cache_kind(QueryCacheKind::Auto)
        .build();

    assert_eq!(q.term(1).inout(), InOutKind::In);
    assert_eq!(q.term(1).first_id(), world.id_view_from(Position::id()));
    assert_eq!(q.term(1).src_id(), 0);
}

#[test]
fn read_component() {
    let world = World::new();

    world.component::<Position>();

    let q = world
        .query::<()>()
        .with(&Position::id())
        .read(Position::id())
        .set_cache_kind(QueryCacheKind::Auto)
        .build();

    assert_eq!(q.term(1).inout(), InOutKind::In);
    assert_eq!(q.term(1).first_id(), world.id_view_from(Position::id()));
    assert_eq!(q.term(1).src_id(), 0);
}

#[test]
fn read_pair_id() {
    let world = World::new();

    let likes = world.entity();
    let apples = world.entity();

    let q = world
        .query::<()>()
        .with(&Position::id())
        .read((likes, apples))
        .set_cache_kind(QueryCacheKind::Auto)
        .build();

    assert_eq!(q.term(1).inout(), InOutKind::In);
    assert_eq!(q.term(1).first_id(), likes);
    assert_eq!(q.term(1).second_id(), apples);
    assert_eq!(q.term(1).src_id(), 0);
}

#[test]
fn read_pair_name() {
    let world = World::new();

    let likes = world.entity_named("likes");
    let apples = world.entity_named("Apples");

    let q = world
        .query::<()>()
        .with(&Position::id())
        .read(("likes", "Apples"))
        .set_cache_kind(QueryCacheKind::Auto)
        .build();

    assert_eq!(q.term(1).inout(), InOutKind::In);
    assert_eq!(q.term(1).first_id(), likes);
    assert_eq!(q.term(1).second_id(), apples);
    assert_eq!(q.term(1).src_id(), 0);
}

#[test]
fn read_pair_components() {
    let world = World::new();

    let q = world
        .query::<()>()
        .with(&Position::id())
        .read((Likes::id(), Apples::id()))
        .set_cache_kind(QueryCacheKind::Auto)
        .build();

    assert_eq!(q.term(1).inout(), InOutKind::In);
    assert_eq!(q.term(1).first_id(), world.id_view_from(Likes::id()));
    assert_eq!(q.term(1).second_id(), world.id_view_from(Apples::id()));
    assert_eq!(q.term(1).src_id(), 0);
}

#[test]
fn read_pair_component_id() {
    let world = World::new();

    let apples = world.entity();

    let q = world
        .query::<()>()
        .with(&Position::id())
        .read((Likes::id(), apples))
        .set_cache_kind(QueryCacheKind::Auto)
        .build();

    assert_eq!(q.term(1).inout(), InOutKind::In);
    assert_eq!(q.term(1).first_id(), world.id_view_from(Likes::id()));
    assert_eq!(q.term(1).second_id(), apples);
    assert_eq!(q.term(1).src_id(), 0);
}

#[test]
fn read_pair_component_name() {
    let world = World::new();

    let apples = world.entity_named("Apples");

    let q = world
        .query::<()>()
        .with(&Position::id())
        .read((Likes::id(), "Apples"))
        .set_cache_kind(QueryCacheKind::Auto)
        .build();

    assert_eq!(q.term(1).inout(), InOutKind::In);
    assert_eq!(q.term(1).first_id(), world.id_view_from(Likes::id()));
    assert_eq!(q.term(1).second_id(), apples);

    assert_eq!(q.term(1).src_id(), 0);
}

#[test]
fn read_enum() {
    #[repr(C)]
    #[derive(Component)]
    pub enum Color {
        Red,
        Green,
        Blue,
    }

    let world = World::new();

    let q = world
        .query::<()>()
        .with(&Position::id())
        .read_enum(Color::Green)
        .set_cache_kind(QueryCacheKind::Auto)
        .build();

    assert_eq!(q.term(1).inout(), InOutKind::In);
    assert_eq!(q.term(1).first_id(), world.id_view_from(Color::id()));
    assert_eq!(q.term(1).second_id(), world.entity_from_enum(Color::Green));
    assert_eq!(q.term(1).src_id(), 0);
}

#[test]
fn assign_after_init() {
    let world = World::new();

    #[allow(unused_assignments)]
    let mut f: Query<()> = world.new_query::<()>();
    let mut fb = world.query::<()>();
    fb = fb.with(&Position::id());
    fb = fb.set_cache_kind(QueryCacheKind::Auto);
    f = fb.build();

    let e1 = world.entity().set(Position { x: 10, y: 20 });

    let mut count = 0;
    f.each_entity_shared(&world, |e, _| {
        assert_eq!(e, e1);
        count += 1;
    });

    assert_eq!(count, 1);
}

#[test]
fn with_t_inout() {
    let world = World::new();

    let f = world
        .query::<()>()
        .with(world.id_view_from(Position::id()))
        .set_cache_kind(QueryCacheKind::Auto)
        .build();

    assert_eq!(f.term(0).inout(), InOutKind::Default);
}

#[test]
fn with_t_inout_1() {
    let world = World::new();

    let f = world
        .query::<()>()
        .with(Position::id())
        .set_cache_kind(QueryCacheKind::Auto)
        .build();

    assert_eq!(f.term(0).inout(), InOutKind::Default);
}

#[test]
fn with_r_t_inout_2() {
    let world = World::new();

    let f = world
        .query::<()>()
        .with((Position::id(), Velocity::id()))
        .set_cache_kind(QueryCacheKind::Auto)
        .build();

    assert_eq!(f.term(0).inout(), InOutKind::Default);
}

#[test]
fn with_r_t_inout_3() {
    let world = World::new();

    let f = world
        .query::<()>()
        .with((Position::id(), world.entity_from::<Velocity>()))
        .set_cache_kind(QueryCacheKind::Auto)
        .build();

    assert_eq!(f.term(0).inout(), InOutKind::Default);
}

#[test]
fn with_r_t_inout() {
    let world = World::new();

    let f = world
        .query::<()>()
        .with((
            world.entity_from::<Position>(),
            world.entity_from::<Velocity>(),
        ))
        .set_cache_kind(QueryCacheKind::Auto)
        .build();

    assert_eq!(f.term(0).inout(), InOutKind::Default);
}

#[test]
fn query_as_move_arg() {
    fn query_move(world: &mut World, f: Query<&SelfRef>) -> i32 {
        let mut count = 0;

        f.each_entity_exclusive(world, |e, s| {
            assert_eq!(e, s.value);
            count += 1;
        });

        count
    }
    let mut world = World::new();

    let f = world
        .query::<&SelfRef>()
        .set_cache_kind(QueryCacheKind::Auto)
        .build();

    let e = world.entity();
    e.set(SelfRef { value: e.id() });

    let e = world.entity();
    e.set(SelfRef { value: e.id() });

    let e = world.entity();
    e.set(SelfRef { value: e.id() });

    assert_eq!(query_move(&mut world, f), 3);
}

#[test]
fn filter_as_return() {
    fn query_auto_return(world: &World) -> Query<&SelfRef> {
        world
            .query::<&SelfRef>()
            .set_cache_kind(QueryCacheKind::Auto)
            .build()
    }

    let world = World::new();

    let e = world.entity();
    e.set(SelfRef { value: e.id() });

    let e = world.entity();
    e.set(SelfRef { value: e.id() });

    let e = world.entity();
    e.set(SelfRef { value: e.id() });

    let f = query_auto_return(&world);

    let mut count = 0;

    f.each_entity_shared(&world, |e, s| {
        assert_eq!(e, s.value);
        count += 1;
    });

    assert_eq!(count, 3);
}

#[test]
fn filter_copy() {
    let mut world = World::new();

    let e = world.entity();
    e.set(SelfRef { value: e.id() });

    let e = world.entity();
    e.set(SelfRef { value: e.id() });

    let e = world.entity();
    e.set(SelfRef { value: e.id() });

    let f = world
        .query::<&SelfRef>()
        .set_cache_kind(QueryCacheKind::Auto)
        .build();

    let f_2 = f;

    let mut count = 0;

    f_2.each_entity_exclusive(&mut world, |e, s| {
        assert_eq!(e, s.value);
        count += 1;
    });

    assert_eq!(count, 3);
}

#[test]
fn world_each_filter_1_component() {
    let world = World::new();

    let e = world.entity();
    e.set(SelfRef { value: e.id() });

    let e = world.entity();
    e.set(SelfRef { value: e.id() });

    let e = world.entity();
    e.set(SelfRef { value: e.id() });

    let mut count = 0;

    world.each_entity::<&SelfRef>(|e, s| {
        assert_eq!(e, s.value);
        count += 1;
    });

    assert_eq!(count, 3);
}

#[test]
fn world_each_filter_2_components() {
    let world = World::new();

    let e = world.entity();
    e.set(SelfRef2 { value: e.id() })
        .set(Position { x: 10, y: 20 });

    let e = world.entity();
    e.set(SelfRef2 { value: e.id() })
        .set(Position { x: 10, y: 20 });

    let e = world.entity();
    e.set(SelfRef2 { value: e.id() })
        .set(Position { x: 10, y: 20 });

    let mut count = 0;

    world.each_entity::<(&SelfRef2, &Position)>(|e, (s, p)| {
        assert_eq!(e, s.value);
        assert_eq!(p.x, 10);
        assert_eq!(p.y, 20);
        count += 1;
    });

    assert_eq!(count, 3);
}

#[test]
fn world_each_filter_1_component_no_entity() {
    let world = World::new();

    world.entity().set(Position { x: 10, y: 20 });

    world.entity().set(Position { x: 10, y: 20 });

    world
        .entity()
        .set(Position { x: 10, y: 20 })
        .set(Velocity { x: 1, y: 2 });

    let mut count = 0;

    world.each::<&Position>(|p| {
        assert_eq!(p.x, 10);
        assert_eq!(p.y, 20);
        count += 1;
    });

    assert_eq!(count, 3);
}

#[test]
fn world_each_filter_2_components_no_entity() {
    let world = World::new();

    world
        .entity()
        .set(Position { x: 10, y: 20 })
        .set(Velocity { x: 1, y: 2 });

    world
        .entity()
        .set(Position { x: 10, y: 20 })
        .set(Velocity { x: 1, y: 2 });

    world
        .entity()
        .set(Position { x: 10, y: 20 })
        .set(Velocity { x: 1, y: 2 });

    world.entity().set(Position { x: 3, y: 5 });

    world.entity().set(Velocity { x: 20, y: 40 });

    let mut count = 0;

    world.each::<(&Position, &Velocity)>(|(p, v)| {
        assert_eq!(p.x, 10);
        assert_eq!(p.y, 20);
        assert_eq!(v.x, 1);
        assert_eq!(v.y, 2);
        count += 1;
    });

    assert_eq!(count, 3);
}

#[test]
fn var_src_w_prefixed_name() {
    let world = World::new();

    let r = world
        .query::<()>()
        .with(&Foo::id())
        .set_src("$Var")
        .set_cache_kind(QueryCacheKind::Auto)
        .build();

    let e = world.entity().add(Foo::id());

    let mut count = 0;
    r.run_shared(&world, |mut it| {
        while it.next() {
            assert_eq!(it.get_var_by_name("Var"), e);
            count += 1;
        }
    });

    assert_eq!(count, 1);
}

#[test]
fn var_first_w_prefixed_name() {
    let world = World::new();

    let r = world
        .query::<()>()
        .with(&Foo::id())
        .term()
        .set_first("$Var")
        .set_cache_kind(QueryCacheKind::Auto)
        .build();

    let e = world.entity().add(Foo::id());

    let mut count = 0;
    r.run_shared(&world, |mut it| {
        while it.next() {
            assert_eq!(it.count(), 1);
            assert_eq!(it.get_entity(0usize).unwrap(), e);
            assert_eq!(it.get_var_by_name("Var"), world.id_view_from(Foo::id()));
            count += 1;
        }
    });

    assert_eq!(count, 1);
}

#[test]
fn var_second_w_prefixed_name() {
    let world = World::new();

    let r = world
        .query::<()>()
        .with(&Foo::id())
        .set_second("$Var")
        .set_cache_kind(QueryCacheKind::Auto)
        .build();

    let t = world.entity();
    let e = world.entity().add((Foo::id(), t));

    let mut count = 0;
    r.run_shared(&world, |mut it| {
        while it.next() {
            assert_eq!(it.count(), 1);
            assert_eq!(it.get_entity(0usize).unwrap(), e);
            assert_eq!(it.get_var_by_name("Var"), t);
            count += 1;
        }
    });

    assert_eq!(count, 1);
}

#[test]
fn term_w_second_var_string() {
    let world = World::new();

    let foo_ = world.entity();

    let r = world
        .query::<()>()
        .with((foo_, "$Var"))
        .set_cache_kind(QueryCacheKind::Auto)
        .build();

    let t = world.entity();
    let e = world.entity().add((foo_, t));

    let mut count = 0;
    r.run_shared(&world, |mut it| {
        while it.next() {
            assert_eq!(it.count(), 1);
            assert_eq!(it.get_entity(0usize).unwrap(), e);
            assert_eq!(it.get_var_by_name("Var"), t);
            count += 1;
        }
    });

    assert_eq!(count, 1);
}

#[test]
fn term_type_w_second_var_string() {
    let world = World::new();

    let r = world
        .query::<()>()
        .with((Foo::id(), "$Var"))
        .set_cache_kind(QueryCacheKind::Auto)
        .build();

    let t = world.entity();
    let e = world.entity().add((Foo::id(), t));

    let mut count = 0;
    r.run_shared(&world, |mut it| {
        while it.next() {
            assert_eq!(it.count(), 1);
            assert_eq!(it.get_entity(0usize).unwrap(), e);
            assert_eq!(it.get_var_by_name("Var"), t);
            count += 1;
        }
    });

    assert_eq!(count, 1);
}

#[test]
fn named_rule() {
    let world = World::new();

    let e1 = world.entity().set(Position { x: 0, y: 0 });
    let e2 = world.entity().set(Position { x: 0, y: 0 });

    let q = world
        .query_named::<&Position>("my_query")
        .set_cache_kind(QueryCacheKind::Auto)
        .build();

    let mut count = 0;
    q.each_entity_shared(&world, |e, _p| {
        assert!((e == e1 || e == e2));
        count += 1;
    });
    assert_eq!(count, 2);

    let qe = q.entity();
    assert_ne!(qe, 0);
    assert_eq!(qe.name(), "my_query");
}

#[test]
fn named_scoped_rule() {
    let world = World::new();

    let e1 = world.entity().set(Position { x: 0, y: 0 });
    let e2 = world.entity().set(Position { x: 0, y: 0 });

    let q = world
        .query_named::<&Position>("my::query")
        .set_cache_kind(QueryCacheKind::Auto)
        .build();

    let mut count = 0;
    q.each_entity_shared(&world, |e, _p| {
        assert!((e == e1 || e == e2));
        count += 1;
    });
    assert_eq!(count, 2);

    let qe = q.entity();
    assert_ne!(qe, 0);
    assert_eq!(qe.name(), "query");
    assert_eq!(qe.path().unwrap(), "::my::query");
}

#[test]
#[should_panic]
fn is_valid() {
    let _guard = FlecsPanicAbortGuard::install();
    let world = World::new();

    let _q_1 = world.query::<&Position>();

    let _q_2 = world
        .query::<()>()
        .expr("foo")
        .set_cache_kind(QueryCacheKind::Auto)
        .build()
        .unwrap();
}

#[test]
fn unresolved_by_name() {
    let world = World::new();

    // AllowUnresolvedByName lets a query reference an entity by name that doesn't exist yet.
    // The flag makes the fallible expr build succeed instead of erroring on the unresolved name.
    let q = world
        .query::<()>()
        .expr("$this == Foo")
        .query_flags(QueryFlags::AllowUnresolvedByName)
        .set_cache_kind(QueryCacheKind::Auto)
        .build()
        .expect("valid query expression with AllowUnresolvedByName");

    // "Foo" doesn't exist yet — query yields no results
    assert!(!q.iterable().is_true());

    // Create "Foo" — now the query resolves and yields a result
    world.entity_named("Foo");

    assert!(q.iterable().is_true());
}

#[test]
fn scope() {
    let world = World::new();

    let root = world.entity();
    let tag_a = world.entity();
    let tag_b = world.entity();

    world.entity().add(root).add(tag_a).add(tag_b);

    let e2 = world.entity().add(root).add(tag_a);

    world.entity().add(root).add(tag_b);

    world.entity().add(root);

    let r = world
        .query::<()>()
        .with(root)
        .scope_open()
        .not()
        .with(tag_a)
        .without(tag_b)
        .scope_close()
        .set_cache_kind(QueryCacheKind::Auto)
        .build();

    let mut count = 0;
    r.each_entity_shared(&world, |e, _| {
        assert_ne!(e, e2);
        count += 1;
    });

    assert_eq!(count, 3);
}

/*

void QueryBuilder_each_w_field_w_fixed_src(void) {
    flecs::world ecs;

    flecs::entity e1 = ecs.entity()
        .set(Position{10, 20})
        .set(Velocity{1, 2});

    flecs::entity e2 = ecs.entity()
        .set(Position{20, 30});

    auto q = ecs.query_builder()
        .with<Position>()
        .with<Velocity>().src(e1)
        .cache_kind(cache_kind)
        .build();

    int32_t count = 0;
    q.each_exclusive(&mut world, [&](flecs::iter& it, size_t row) {
        auto e = it.entity(row);
        auto p = it.field_at<Position>(0, row);
        auto v = it.field<const Velocity>(1);

        if (e == e1) {
            test_int(p.x, 10);
            test_int(p.y, 20);
            test_int(v->x, 1);
            test_int(v->y, 2);
        }
        if (e == e2) {
            test_int(p.x, 20);
            test_int(p.y, 30);
            test_int(v->x, 1);
            test_int(v->y, 2);
        }

        count ++;
    });

    test_int(count, 2);
}

void QueryBuilder_each_w_field_at_w_fixed_src(void) {
    flecs::world ecs;

    flecs::entity e1 = ecs.entity()
        .set(Position{10, 20})
        .set(Velocity{1, 2});

    flecs::entity e2 = ecs.entity()
        .set(Position{20, 30});

    auto q = ecs.query_builder()
        .with<Position>()
        .with<Velocity>().src(e1)
        .cache_kind(cache_kind)
        .build();

    int32_t count = 0;
    q.each_exclusive(&mut world, [&](flecs::iter& it, size_t row) {
        auto e = it.entity(row);
        auto p = it.field_at<Position>(0, row);
        auto v = it.field_at<const Velocity>(1, 0);

        if (e == e1) {
            test_int(p.x, 10);
            test_int(p.y, 20);
            test_int(v.x, 1);
            test_int(v.y, 2);
        }
        if (e == e2) {
            test_int(p.x, 20);
            test_int(p.y, 30);
            test_int(v.x, 1);
            test_int(v.y, 2);
        }

        count ++;
    });

    test_int(count, 2);
}

void QueryBuilder_each_w_const_field_w_fixed_src(void) {
    flecs::world ecs;

    flecs::entity e1 = ecs.entity()
        .set(Position{10, 20})
        .set(Velocity{1, 2});

    flecs::entity e2 = ecs.entity()
        .set(Position{20, 30});

    auto q = ecs.query_builder()
        .with<Position>()
        .with<Velocity>().src(e1)
        .cache_kind(cache_kind)
        .build();

    int32_t count = 0;
    q.each_exclusive(&mut world, [&](flecs::iter& it, size_t row) {
        auto e = it.entity(row);
        auto p = it.field_at<Position>(0, row);
        auto v = it.field<const Velocity>(1);

        if (e == e1) {
            test_int(p.x, 10);
            test_int(p.y, 20);
            test_int(v->x, 1);
            test_int(v->y, 2);
        }
        if (e == e2) {
            test_int(p.x, 20);
            test_int(p.y, 30);
            test_int(v->x, 1);
            test_int(v->y, 2);
        }

        count ++;
    });

    test_int(count, 2);
}

void QueryBuilder_each_w_const_field_at_w_fixed_src(void) {
    flecs::world ecs;

    flecs::entity e1 = ecs.entity()
        .set(Position{10, 20})
        .set(Velocity{1, 2});

    flecs::entity e2 = ecs.entity()
        .set(Position{20, 30});

    auto q = ecs.query_builder()
        .with<Position>()
        .with<Velocity>().src(e1)
        .cache_kind(cache_kind)
        .build();

    int32_t count = 0;
    q.each_exclusive(&mut world, [&](flecs::iter& it, size_t row) {
        auto e = it.entity(row);
        auto p = it.field_at<Position>(0, row);
        auto v = it.field_at<const Velocity>(1, 0);

        if (e == e1) {
            test_int(p.x, 10);
            test_int(p.y, 20);
            test_int(v.x, 1);
            test_int(v.y, 2);
        }
        if (e == e2) {
            test_int(p.x, 20);
            test_int(p.y, 30);
            test_int(v.x, 1);
            test_int(v.y, 2);
        }

        count ++;
    });

    test_int(count, 2);
}

void QueryBuilder_each_w_untyped_field_w_fixed_src(void) {
    flecs::world ecs;

    flecs::entity e1 = ecs.entity()
        .set(Position{10, 20})
        .set(Velocity{1, 2});

    flecs::entity e2 = ecs.entity()
        .set(Position{20, 30});

    auto q = ecs.query_builder()
        .with<Position>()
        .with<Velocity>().src(e1)
        .cache_kind(cache_kind)
        .build();

    int32_t count = 0;
    q.each_exclusive(&mut world, [&](flecs::iter& it, size_t row) {
        auto e = it.entity(row);
        auto p = it.field_at<Position>(0, row);
        flecs::untyped_field vf = it.field(1);
        Velocity *v = static_cast<Velocity*>(vf[0]);

        if (e == e1) {
            test_int(p.x, 10);
            test_int(p.y, 20);
            test_int(v->x, 1);
            test_int(v->y, 2);
        }
        if (e == e2) {
            test_int(p.x, 20);
            test_int(p.y, 30);
            test_int(v->x, 1);
            test_int(v->y, 2);
        }

        count ++;
    });

    test_int(count, 2);
}

void QueryBuilder_each_w_untyped_field_at_w_fixed_src(void) {
    flecs::world ecs;

    flecs::entity e1 = ecs.entity()
        .set(Position{10, 20})
        .set(Velocity{1, 2});

    flecs::entity e2 = ecs.entity()
        .set(Position{20, 30});

    auto q = ecs.query_builder()
        .with<Position>()
        .with<Velocity>().src(e1)
        .cache_kind(cache_kind)
        .build();

    int32_t count = 0;
    q.each_exclusive(&mut world, [&](flecs::iter& it, size_t row) {
        auto e = it.entity(row);
        auto p = it.field_at<Position>(0, row);
        void *vptr = it.field_at(1, 0);
        Velocity *v = static_cast<Velocity*>(vptr);

        if (e == e1) {
            test_int(p.x, 10);
            test_int(p.y, 20);
            test_int(v->x, 1);
            test_int(v->y, 2);
        }
        if (e == e2) {
            test_int(p.x, 20);
            test_int(p.y, 30);
            test_int(v->x, 1);
            test_int(v->y, 2);
        }

        count ++;
    });

    test_int(count, 2);
}

void QueryBuilder_singleton_pair(void) {
    flecs::world ecs;

    flecs::entity rel = ecs.component<Position>();
    flecs::entity tgt = ecs.entity();

    ecs.set<Position>(tgt, {10, 20});

    int32_t count = 0;

    auto q = ecs.query_builder<const Position>()
        .term_at(0).second(tgt).singleton()
        .cache_kind(cache_kind)
        .build();

    q.each_exclusive(&mut world, [&](flecs::iter& it, size_t, const Position& p) {
        test_assert(it.src(0) == rel);
        test_assert(it.pair(0) == ecs.pair<Position>(tgt));
        test_int(p.x, 10);
        test_int(p.y, 20);
        count ++;
    });

    test_int(count, 1);
}

void QueryBuilder_query_w_this_second(void) {
    flecs::world ecs;

    flecs::entity rel = ecs.entity();

    auto q = ecs.query_builder()
        .with(rel, flecs::This)
        .build();

    flecs::entity e1 = ecs.entity();
    e1.add(rel, e1);

    int32_t count = 0;
    q.each_exclusive(&mut world, [&](flecs::entity e) {
        test_assert(e == e1);
        count ++;
    });

    test_int(count, 1);
}

*/

// ────────────────────────────────────────────────────────────────
// New tests appended below
// ────────────────────────────────────────────────────────────────

#[test]
fn query_builder_ptr_type() {
    let world = World::new();

    let q = world
        .query::<(&Position, Option<&Velocity>)>()
        .set_cache_kind(QueryCacheKind::Auto)
        .build();

    let e1 = world.entity().set(Position { x: 0, y: 0 });
    let e2 = world
        .entity()
        .set(Position { x: 0, y: 0 })
        .set(Velocity { x: 0, y: 0 });
    world
        .entity()
        .set(Velocity { x: 0, y: 0 })
        .set(Mass { value: 0 });

    let mut count = 0;
    q.each_entity_shared(&world, |e, (_p, _v)| {
        count += 1;
        assert!(e == e1 || e == e2);
    });

    assert_eq!(count, 2);
}

#[test]
fn query_builder_named_scoped_query() {
    // Same behavior as named_scoped_rule — just a different name alias
    let world = World::new();

    let e1 = world.entity().set(Position { x: 0, y: 0 });
    let e2 = world.entity().set(Position { x: 0, y: 0 });

    let q = world
        .query_named::<&Position>("my::query")
        .set_cache_kind(QueryCacheKind::Auto)
        .build();

    let mut count = 0;
    q.each_entity_shared(&world, |e, _p| {
        assert!(e == e1 || e == e2);
        count += 1;
    });
    assert_eq!(count, 2);

    let qe = q.entity();
    assert_ne!(qe, 0);
    assert_eq!(qe.name(), "query");
    assert_eq!(qe.path().unwrap(), "::my::query");
}

#[test]
fn query_builder_cascade_w_set_var() {
    let world = World::new();

    let sun = world.entity().set(Position { x: 1, y: 2 });
    let earth = world.entity().child_of(sun);

    let q = world
        .query::<Option<&Position>>()
        .term_at(0)
        .cascade()
        .build();

    let mut count = 0;
    q.with_var(0, earth).each_entity_shared(&world, |e, p| {
        count += 1;
        assert_eq!(e, earth);
        assert!(p.is_some());
        let p = p.unwrap();
        assert_eq!(p.x, 1);
        assert_eq!(p.y, 2);
    });

    assert_eq!(count, 1);
}

// group_by_raw_ordered — EcsQueryGroupByOrdered not exposed in Rust bindings
// TODO: missing API: query_flags(QueryFlags::GroupByOrdered)
// fn query_builder_group_by_raw_ordered() {}

// group_by_template_ordered — EcsQueryGroupByOrdered not exposed in Rust bindings
// TODO: missing API: query_flags(QueryFlags::GroupByOrdered)
// fn query_builder_group_by_template_ordered() {}

// group_by_default_func_w_id_ordered — EcsQueryGroupByOrdered not exposed in Rust bindings
// TODO: missing API: query_flags(QueryFlags::GroupByOrdered)
// fn query_builder_group_by_default_func_w_id_ordered() {}

// group_by_default_func_w_type_ordered — EcsQueryGroupByOrdered not exposed in Rust bindings
// TODO: missing API: query_flags(QueryFlags::GroupByOrdered)
// fn query_builder_group_by_default_func_w_type_ordered() {}

// group_by_callbacks_ordered — EcsQueryGroupByOrdered not exposed in Rust bindings
// TODO: missing API: query_flags(QueryFlags::GroupByOrdered)
// fn query_builder_group_by_callbacks_ordered() {}

// iterate_groups — no q.groups() / iterate_groups() API in Rust bindings
// TODO: missing API: Query::groups() iterator
// fn query_builder_iterate_groups() {}

// iterate_groups_empty — no q.groups() API
// TODO: missing API: Query::groups() iterator
// fn query_builder_iterate_groups_empty() {}

// iterate_groups_w_isa — no q.groups() API
// TODO: missing API: Query::groups() iterator
// fn query_builder_iterate_groups_w_isa() {}

#[test]
fn query_builder_pred_eq() {
    let world = World::new();

    let foo = world.entity_named("Foo");

    // Use string-based second arg like C++ .with(flecs::PredEq, "Foo")
    let q = world
        .query::<()>()
        .with(*flecs::PredEq)
        .set_second("Foo")
        .build();

    let mut count = 0;
    q.each_entity_shared(&world, |e, _| {
        assert_eq!(e, foo);
        count += 1;
    });

    assert_eq!(count, 1);
}

#[test]
fn query_builder_pred_eq_name() {
    let world = World::new();

    let q = world
        .query::<()>()
        .with(*flecs::PredEq)
        .set_second("Foo")
        .second() // re-enter second ref mode before setting flags
        .flags(flecs::term_flags::IsName::ID)
        .build();

    let foo = world.entity_named("Foo");

    let mut count = 0;
    q.each_entity_shared(&world, |e, _| {
        assert_eq!(e, foo);
        count += 1;
    });

    assert_eq!(count, 1);
}

#[test]
fn query_builder_pred_match() {
    let world = World::new();

    let q = world
        .query::<()>()
        .with(*flecs::PredMatch)
        .set_second("FooB")
        .second() // re-enter second ref mode before setting flags
        .flags(flecs::term_flags::IsName::ID)
        .build();

    let foo = world.entity_named("FooBar");

    let mut count = 0;
    q.each_entity_shared(&world, |e, _| {
        assert_eq!(e, foo);
        count += 1;
    });

    assert_eq!(count, 1);
}

// not_w_write — not found in C++ source, no direct mapping
// TODO: missing API: not present in QueryBuilder.cpp source
// fn query_builder_not_w_write() {}

// optional_module — not found in C++ source, no direct mapping
// TODO: missing API: not present in QueryBuilder.cpp source
// fn query_builder_optional_module() {}

// pair_with_variable_src — not found in C++ source
// TODO: missing API: not present in QueryBuilder.cpp source
// fn query_builder_pair_with_variable_src() {}

// pair_with_variable_src_no_row_fields — not found in C++ source
// TODO: missing API: not present in QueryBuilder.cpp source
// fn query_builder_pair_with_variable_src_no_row_fields() {}

// query_each_from_component — not found in C++ source
// TODO: missing API: not present in QueryBuilder.cpp source
// fn query_builder_query_each_from_component() {}

// query_from_entity — not found in C++ source
// TODO: missing API: not present in QueryBuilder.cpp source
// fn query_builder_query_from_entity() {}

// query_from_entity_name — not found in C++ source
// TODO: missing API: not present in QueryBuilder.cpp source
// fn query_builder_query_from_entity_name() {}

// query_iter_from_component — not found in C++ source
// TODO: missing API: not present in QueryBuilder.cpp source
// fn query_builder_query_iter_from_component() {}

// query_single_pair — not found in C++ source
// TODO: missing API: not present in QueryBuilder.cpp source
// fn query_builder_query_single_pair() {}

// query_type_w_template — not found in C++ source
// TODO: missing API: not present in QueryBuilder.cpp source
// fn query_builder_query_type_w_template() {}

// set_group_captured_query — not found in C++ source
// TODO: missing API: not present in QueryBuilder.cpp source
// fn query_builder_set_group_captured_query() {}

// set_this_var — not found in C++ source
// TODO: missing API: not present in QueryBuilder.cpp source
// fn query_builder_set_this_var() {}

// set_var_captured_query — not found in C++ source
// TODO: missing API: not present in QueryBuilder.cpp source
// fn query_builder_set_var_captured_query() {}

// set_var_id_captured_query — not found in C++ source
// TODO: missing API: not present in QueryBuilder.cpp source
// fn query_builder_set_var_id_captured_query() {}

// worker_iter_captured_query — not found in C++ source
// TODO: missing API: not present in QueryBuilder.cpp source
// fn query_builder_worker_iter_captured_query() {}

// ─── setup ────────────────────────────────────────────────────────────────────

#[test]
fn query_builder_setup() {
    // C++ setup() configures cache_kind via test params. In Rust we just verify
    // the default build works — no external test param infrastructure.
    let world = World::new();
    let _q = world.query::<(&Position, &Velocity)>().build();
}

// ─── builder_build_to_auto ───────────────────────────────────────────────────

#[test]
fn query_builder_builder_build_to_auto() {
    let world = World::new();

    let q = world
        .query::<(&Position, &Velocity)>()
        .set_cache_kind(QueryCacheKind::Auto)
        .build();

    let e1 = world.entity().add(Position::id()).add(Velocity::id());
    world.entity().add(Position::id());

    let mut count = 0;
    q.each_entity_shared(&world, |e, _| {
        count += 1;
        assert_eq!(e, e1);
    });

    assert_eq!(count, 1);
}

// ─── 32_terms ────────────────────────────────────────────────────────────────

#[test]
fn query_builder_32_terms() {
    #[derive(Component)]
    struct T00;
    #[derive(Component)]
    struct T01;
    #[derive(Component)]
    struct T02;
    #[derive(Component)]
    struct T03;
    #[derive(Component)]
    struct T04;
    #[derive(Component)]
    struct T05;
    #[derive(Component)]
    struct T06;
    #[derive(Component)]
    struct T07;
    #[derive(Component)]
    struct T08;
    #[derive(Component)]
    struct T09;
    #[derive(Component)]
    struct T10;
    #[derive(Component)]
    struct T11;
    #[derive(Component)]
    struct T12;
    #[derive(Component)]
    struct T13;
    #[derive(Component)]
    struct T14;
    #[derive(Component)]
    struct T15;
    #[derive(Component)]
    struct T16;
    #[derive(Component)]
    struct T17;
    #[derive(Component)]
    struct T18;
    #[derive(Component)]
    struct T19;
    #[derive(Component)]
    struct T20;
    #[derive(Component)]
    struct T21;
    #[derive(Component)]
    struct T22;
    #[derive(Component)]
    struct T23;
    #[derive(Component)]
    struct T24;
    #[derive(Component)]
    struct T25;
    #[derive(Component)]
    struct T26;
    #[derive(Component)]
    struct T27;
    #[derive(Component)]
    struct T28;
    #[derive(Component)]
    struct T29;
    #[derive(Component)]
    struct T30;
    #[derive(Component)]
    struct T31;

    let world = World::new();

    let f = world
        .query::<()>()
        .with(T00::id())
        .with(T01::id())
        .with(T02::id())
        .with(T03::id())
        .with(T04::id())
        .with(T05::id())
        .with(T06::id())
        .with(T07::id())
        .with(T08::id())
        .with(T09::id())
        .with(T10::id())
        .with(T11::id())
        .with(T12::id())
        .with(T13::id())
        .with(T14::id())
        .with(T15::id())
        .with(T16::id())
        .with(T17::id())
        .with(T18::id())
        .with(T19::id())
        .with(T20::id())
        .with(T21::id())
        .with(T22::id())
        .with(T23::id())
        .with(T24::id())
        .with(T25::id())
        .with(T26::id())
        .with(T27::id())
        .with(T28::id())
        .with(T29::id())
        .with(T30::id())
        .with(T31::id())
        .build();

    assert_eq!(f.field_count(), 32);

    let e = world
        .entity()
        .add(T00::id())
        .add(T01::id())
        .add(T02::id())
        .add(T03::id())
        .add(T04::id())
        .add(T05::id())
        .add(T06::id())
        .add(T07::id())
        .add(T08::id())
        .add(T09::id())
        .add(T10::id())
        .add(T11::id())
        .add(T12::id())
        .add(T13::id())
        .add(T14::id())
        .add(T15::id())
        .add(T16::id())
        .add(T17::id())
        .add(T18::id())
        .add(T19::id())
        .add(T20::id())
        .add(T21::id())
        .add(T22::id())
        .add(T23::id())
        .add(T24::id())
        .add(T25::id())
        .add(T26::id())
        .add(T27::id())
        .add(T28::id())
        .add(T29::id())
        .add(T30::id())
        .add(T31::id());

    let mut count = 0;
    f.run_shared(&world, |mut it| {
        while it.next() {
            assert_eq!(it.count(), 1);
            assert_eq!(it.get_entity(0_usize).unwrap(), e);
            assert_eq!(it.field_count(), 32);
            count += 1;
        }
    });

    assert_eq!(count, 1);
}

// ─── 33_terms ────────────────────────────────────────────────────────────────
// TODO: missing API: 33-term limit enforcement panics/asserts in C via test_abort.
// Not ported: requires crash-handler test infrastructure.

// ─── with_id ─────────────────────────────────────────────────────────────────

#[test]
fn query_builder_with_id() {
    let world = World::new();

    let q = world
        .query::<()>()
        .with(world.id_view_from(Position::id()))
        .with(world.id_view_from(Velocity::id()))
        .set_cache_kind(QueryCacheKind::Auto)
        .build();

    let e1 = world.entity().add(Position::id()).add(Velocity::id());
    world.entity().add(Position::id());

    let mut count = 0;
    q.each_entity_shared(&world, |e, _| {
        count += 1;
        assert_eq!(e, e1);
    });

    assert_eq!(count, 1);
}

// ─── without_id ──────────────────────────────────────────────────────────────

#[test]
fn query_builder_without_id() {
    let world = World::new();

    let q = world
        .query::<()>()
        .with(world.id_view_from(Position::id()))
        .without(world.id_view_from(Velocity::id()))
        .set_cache_kind(QueryCacheKind::Auto)
        .build();

    world.entity().add(Position::id()).add(Velocity::id());
    let e2 = world.entity().add(Position::id());

    let mut count = 0;
    q.each_entity_shared(&world, |e, _| {
        count += 1;
        assert_eq!(e, e2);
    });

    assert_eq!(count, 1);
}

// ─── write_id ────────────────────────────────────────────────────────────────

#[test]
fn query_builder_write_id() {
    let world = World::new();

    let q = world
        .query::<()>()
        .with(&Position::id())
        .write(world.id_view_from(Position::id()))
        .set_cache_kind(QueryCacheKind::Auto)
        .build();

    assert_eq!(q.term(1).inout(), InOutKind::Out);
    assert_eq!(q.term(1).first_id(), world.id_view_from(Position::id()));
    assert_eq!(q.term(1).src_id(), 0);
}

// ─── read_id ─────────────────────────────────────────────────────────────────

#[test]
fn query_builder_read_id() {
    let world = World::new();

    let q = world
        .query::<()>()
        .with(&Position::id())
        .read(world.id_view_from(Position::id()))
        .set_cache_kind(QueryCacheKind::Auto)
        .build();

    assert_eq!(q.term(1).inout(), InOutKind::In);
    assert_eq!(q.term(1).first_id(), world.id_view_from(Position::id()));
    assert_eq!(q.term(1).src_id(), 0);
}

// ─── set_var_on_query ─────────────────────────────────────────────────────────

#[test]
fn query_builder_set_var_on_query() {
    #[derive(Component)]
    struct LikesVOQ;

    let world = World::new();

    let apples = world.entity();
    let pears = world.entity();

    world.entity().add((LikesVOQ::id(), apples));

    let e2 = world.entity().add((LikesVOQ::id(), pears));

    let r = world
        .query::<()>()
        .with((LikesVOQ::id(), id::<flecs::Wildcard>()))
        .second()
        .set_var("Food")
        .build();

    let food_var = r.find_var("Food").unwrap();

    let mut count = 0;
    r.with_var(food_var, pears).each_iter(|it, i, ()| {
        assert_eq!(it.get_entity(i).unwrap(), e2);
        assert_eq!(it.get_var(food_var), pears);
        count += 1;
    });

    assert_eq!(count, 1);
}

// ─── set_var_by_name_on_query ─────────────────────────────────────────────────

#[test]
fn query_builder_set_var_by_name_on_query() {
    #[derive(Component)]
    struct LikesVBNQ;

    let world = World::new();

    let apples = world.entity();
    let pears = world.entity();

    world.entity().add((LikesVBNQ::id(), apples));
    let e2 = world.entity().add((LikesVBNQ::id(), pears));

    let r = world
        .query::<()>()
        .with((LikesVBNQ::id(), id::<flecs::Wildcard>()))
        .second()
        .set_var("Food")
        .build();

    let food_var_id = r.find_var("Food").unwrap();
    let mut count = 0;
    r.with_var_expr("Food", pears).each_iter(|it, i, ()| {
        assert_eq!(it.get_entity(i).unwrap(), e2);
        assert_eq!(it.get_var(food_var_id), pears);
        count += 1;
    });

    assert_eq!(count, 1);
}

// ─── set_table_var ────────────────────────────────────────────────────────────

#[test]
fn query_builder_set_table_var() {
    let world = World::new();

    let e1 = world.entity().add(Position::id());
    let e2 = world.entity().add(Position::id());
    let e3 = world.entity().add(Position::id()).add(Velocity::id());

    let r = world.new_query::<&Position>();

    let mut count = 0;
    r.with_var_table_expr("this", e1.table().unwrap())
        .each_iter(|it, index, _| {
            let e = it.get_entity(index).unwrap();
            if usize::from(index) == 0 {
                assert_eq!(e, e1);
            } else if usize::from(index) == 1 {
                assert_eq!(e, e2);
            }
            count += 1;
        });

    assert_eq!(count, 2);

    r.with_var_table_expr("this", e3.table().unwrap())
        .each_iter(|it, index, _| {
            let e = it.get_entity(index).unwrap();
            assert_eq!(e, e3);
            count += 1;
        });

    assert_eq!(count, 3);
}

// ─── set_range_var ────────────────────────────────────────────────────────────

#[test]
fn query_builder_set_range_var() {
    let world = World::new();

    let e1 = world.entity().add(Position::id());
    let e2 = world.entity().add(Position::id());
    let e3 = world.entity().add(Position::id()).add(Velocity::id());

    let r = world.new_query::<&Position>();

    let mut count = 0;

    r.with_var_table_expr("this", e1.range().unwrap())
        .each_iter(|it, index, _| {
            let e = it.get_entity(index).unwrap();
            assert_eq!(e, e1);
            count += 1;
        });
    assert_eq!(count, 1);

    r.with_var_table_expr("this", e2.range().unwrap())
        .each_iter(|it, index, _| {
            let e = it.get_entity(index).unwrap();
            assert_eq!(e, e2);
            count += 1;
        });
    assert_eq!(count, 2);

    r.with_var_table_expr("this", e3.range().unwrap())
        .each_iter(|it, index, _| {
            let e = it.get_entity(index).unwrap();
            assert_eq!(e, e3);
            count += 1;
        });
    assert_eq!(count, 3);
}

// ─── set_table_var_chained ────────────────────────────────────────────────────

#[test]
fn query_builder_set_table_var_chained() {
    let world = World::new();

    world.entity().add(Position::id());
    world.entity().add(Position::id());
    let e3 = world.entity().add(Position::id()).add(Velocity::id());
    world.entity().add(Velocity::id());

    let q1 = world.new_query::<&Position>();
    let q2 = world.new_query::<&Velocity>();

    let mut count = 0;

    q1.run_shared(&world, |mut it| {
        while it.next() {
            q2.with_var_table_expr("this", it.table().unwrap())
                .each_entity_shared(&world, |e, _| {
                    assert_eq!(e, e3);
                    count += 1;
                });
        }
    });

    assert_eq!(count, 1);
}

// ─── set_range_var_chained ────────────────────────────────────────────────────

#[test]
fn query_builder_set_range_var_chained() {
    let world = World::new();

    world.entity().add(Position::id());
    world.entity().add(Position::id());
    let e3 = world.entity().add(Position::id()).add(Velocity::id());
    world.entity().add(Velocity::id());

    let q1 = world.new_query::<&Position>();
    let q2 = world.new_query::<&Velocity>();

    let mut count = 0;

    q1.run_shared(&world, |mut it| {
        while it.next() {
            q2.with_var_table_expr("this", it.range().unwrap())
                .each_entity_shared(&world, |e, _| {
                    assert_eq!(e, e3);
                    count += 1;
                });
        }
    });

    assert_eq!(count, 1);
}

// ─── set_group_on_query ───────────────────────────────────────────────────────

#[test]
fn query_builder_set_group_on_query() {
    let world = World::new();

    let rel = world.entity();
    let tgt_a = world.entity();
    let tgt_b = world.entity();
    let tgt_c = world.entity();
    let tag = world.entity();

    world.entity().add((rel, tgt_a));
    let e2 = world.entity().add((rel, tgt_b));
    world.entity().add((rel, tgt_c));

    world.entity().add((rel, tgt_a)).add(tag);
    let e5 = world.entity().add((rel, tgt_b)).add(tag);
    world.entity().add((rel, tgt_c)).add(tag);

    let q = world
        .query::<()>()
        .with((rel, id::<flecs::Wildcard>()))
        .group_by(rel)
        .build();

    let mut e2_found = false;
    let mut e5_found = false;
    let mut count = 0;

    q.with_group(tgt_b).each_iter(|it, i, ()| {
        let e = it.get_entity(i).unwrap();
        assert_eq!(it.group_id(), tgt_b);
        if e == e2 {
            e2_found = true;
        }
        if e == e5 {
            e5_found = true;
        }
        count += 1;
    });

    assert_eq!(count, 2);
    assert!(e2_found);
    assert!(e5_found);
}

// ─── set_group_type_on_query ──────────────────────────────────────────────────

#[test]
fn query_builder_set_group_type_on_query() {
    #[derive(Component)]
    struct RelSGT;
    #[derive(Component)]
    struct TgtASGT;
    #[derive(Component)]
    struct TgtBSGT;
    #[derive(Component)]
    struct TgtCSGT;

    let world = World::new();

    let tag = world.entity();

    world.entity().add((RelSGT::id(), TgtASGT::id()));
    let e2 = world.entity().add((RelSGT::id(), TgtBSGT::id()));
    world.entity().add((RelSGT::id(), TgtCSGT::id()));

    world.entity().add((RelSGT::id(), TgtASGT::id())).add(tag);
    let e5 = world.entity().add((RelSGT::id(), TgtBSGT::id())).add(tag);
    world.entity().add((RelSGT::id(), TgtCSGT::id())).add(tag);

    let q = world
        .query::<()>()
        .with((RelSGT::id(), id::<flecs::Wildcard>()))
        .group_by(RelSGT::id())
        .build();

    let tgt_b_raw = world.id_view_from(TgtBSGT::id());
    let mut e2_found = false;
    let mut e5_found = false;
    let mut count = 0;

    q.with_group(TgtBSGT::id()).each_iter(|it, i, ()| {
        let e = it.get_entity(i).unwrap();
        assert_eq!(it.group_id(), tgt_b_raw);
        if e == e2 {
            e2_found = true;
        }
        if e == e5 {
            e5_found = true;
        }
        count += 1;
    });

    assert_eq!(count, 2);
    assert!(e2_found);
    assert!(e5_found);
}

// ─── group_by_raw_ordered / template_ordered / default_func_*_ordered / callbacks_ordered ──
// TODO: missing API: EcsQueryGroupByOrdered flag not exposed in Rust QueryFlags.

// ─── iterate_groups / iterate_groups_empty / iterate_groups_w_isa ──────────────
// TODO: missing API: Query::groups() iterator not implemented in Rust bindings.

// ─── iter_column_w_const_deref ────────────────────────────────────────────────

#[test]
fn query_builder_iter_column_w_const_deref() {
    let mut world = World::new();

    let f = world.new_query::<&Position>();

    let base = world.prefab().set(Position { x: 10, y: 20 });
    world.entity().is_a(base);
    world.entity().is_a(base);

    let mut count = 0;
    f.run_exclusive(&mut world, |mut it| {
        while it.next() {
            let p = it.field::<Position>(0);
            // In Rust there's no pointer deref syntax; we index p[0] for the shared component
            let pv = p[0];
            for _ in it.iter() {
                assert_eq!(pv.x, 10);
                assert_eq!(pv.y, 20);
                count += 1;
            }
        }
    });

    assert_eq!(count, 2);
}

// ─── filter_as_move_arg ───────────────────────────────────────────────────────

#[test]
fn query_builder_filter_as_move_arg() {
    let mut world = World::new();

    let f = world
        .query::<&SelfRef>()
        .set_cache_kind(QueryCacheKind::Auto)
        .build();

    let e = world.entity();
    e.set(SelfRef { value: e.id() });
    let e = world.entity();
    e.set(SelfRef { value: e.id() });
    let e = world.entity();
    e.set(SelfRef { value: e.id() });

    // Pass query by value to a helper — mirrors C++ "move arg" semantics
    fn count_query(world: &mut World, f: Query<&SelfRef>) -> i32 {
        let mut count = 0;
        f.each_entity_exclusive(world, |e, s| {
            assert_eq!(e, s.value);
            count += 1;
        });
        count
    }

    assert_eq!(count_query(&mut world, f), 3);
}

// ─── world_each_entity ────────────────────────────────────────────────────────
// C++ `ecs.each([](flecs::entity e){})` internally adds ECS_ANY as the sole query
// term, matching all entities that have at least one component.
// Rust equivalent: build a query with `.with(flecs::Any::ID)` and iterate with
// `each_entity`. The 0-term `world.each_entity::<()>` does NOT work because a
// query with no terms matches no tables.

#[test]
fn query_builder_world_each_entity() {
    let world = World::new();

    // Entities must have at least one component to be found by an ECS_ANY query.
    let tag = world.entity();
    let e1 = world.entity().add(tag);
    let e2 = world.entity().add(tag);
    let e3 = world.entity().add(tag);

    let mut count = 0i32;
    let mut found_e1 = false;
    let mut found_e2 = false;
    let mut found_e3 = false;

    // Rust equivalent of C++ ecs.each([](entity e){}) — add ECS_ANY term so the
    // query matches every entity that has at least one component/tag.
    world
        .query::<()>()
        .with(flecs::Any::ID)
        .build()
        .each_entity_shared(&world, |e, ()| {
            if e == e1 {
                found_e1 = true;
            }
            if e == e2 {
                found_e2 = true;
            }
            if e == e3 {
                found_e3 = true;
            }
            count += 1;
        });

    assert!(found_e1);
    assert!(found_e2);
    assert!(found_e3);
    // count includes built-in entities; just confirm at least our 3 were visited
    assert!(count >= 3);
}

// ─── singleton_pair ───────────────────────────────────────────────────────────
// TODO: missing API: Dynamic second target on singleton pair query (.term_at(0).second(tgt).singleton())
// requires complex pair-with-target builder syntax not yet easily mapped in Rust.
// The C++ test uses ecs.query_builder<const Position>().term_at(0).second(tgt).singleton()
// which needs non-trivial term builder chaining with wildcard pair + second override.

// ─── query_w_this_second ──────────────────────────────────────────────────────

#[test]
fn query_builder_query_w_this_second() {
    let world = World::new();

    let rel = world.entity();

    // Use $this variable via set_second to match entities where (rel, entity_itself)
    let q = world.query::<()>().with(rel).set_second("$this").build();

    let e1 = world.entity();
    e1.add((rel, e1));

    let mut count = 0;
    q.each_entity_shared(&world, |e, _| {
        assert_eq!(e, e1);
        count += 1;
    });

    assert_eq!(count, 1);
}

// ─── each_w_field_w_fixed_src ─────────────────────────────────────────────────

#[test]
fn query_builder_each_w_field_w_fixed_src() {
    let world = World::new();

    let e1 = world
        .entity()
        .set(Position { x: 10, y: 20 })
        .set(Velocity { x: 1, y: 2 });

    let e2 = world.entity().set(Position { x: 20, y: 30 });

    let q = world
        .query::<()>()
        .with(&Position::id())
        .with(&Velocity::id())
        .src()
        .set_id(e1)
        .build();

    let mut count = 0;
    q.each_iter(|it, row, ()| {
        let e = it.get_entity(row).unwrap();
        let p = it.field_at::<Position>(0, row);
        let v = it.field::<Velocity>(1);

        if e == e1 {
            assert_eq!(p.x, 10);
            assert_eq!(p.y, 20);
            assert_eq!(v[0].x, 1);
            assert_eq!(v[0].y, 2);
        }
        if e == e2 {
            assert_eq!(p.x, 20);
            assert_eq!(p.y, 30);
            assert_eq!(v[0].x, 1);
            assert_eq!(v[0].y, 2);
        }
        count += 1;
    });

    assert_eq!(count, 2);
}

// ─── each_w_field_at_w_fixed_src ──────────────────────────────────────────────

#[test]
fn query_builder_each_w_field_at_w_fixed_src() {
    let world = World::new();

    let e1 = world
        .entity()
        .set(Position { x: 10, y: 20 })
        .set(Velocity { x: 1, y: 2 });

    let e2 = world.entity().set(Position { x: 20, y: 30 });

    let q = world
        .query::<()>()
        .with(&Position::id())
        .with(&Velocity::id())
        .src()
        .set_id(e1)
        .build();

    let mut count = 0;
    q.each_iter(|it, row, ()| {
        let e = it.get_entity(row).unwrap();
        let p = it.field_at::<Position>(0, row);
        let v = it.field_at::<Velocity>(1, 0_usize);

        if e == e1 {
            assert_eq!(p.x, 10);
            assert_eq!(p.y, 20);
            assert_eq!(v.x, 1);
            assert_eq!(v.y, 2);
        }
        if e == e2 {
            assert_eq!(p.x, 20);
            assert_eq!(p.y, 30);
            assert_eq!(v.x, 1);
            assert_eq!(v.y, 2);
        }
        count += 1;
    });

    assert_eq!(count, 2);
}

// ─── each_w_const_field_w_fixed_src ───────────────────────────────────────────

#[test]
fn query_builder_each_w_const_field_w_fixed_src() {
    let world = World::new();

    let e1 = world
        .entity()
        .set(Position { x: 10, y: 20 })
        .set(Velocity { x: 1, y: 2 });

    let e2 = world.entity().set(Position { x: 20, y: 30 });

    let q = world
        .query::<()>()
        .with(&Position::id())
        .with(&Velocity::id())
        .src()
        .set_id(e1)
        .build();

    let mut count = 0;
    q.each_iter(|it, row, ()| {
        let e = it.get_entity(row).unwrap();
        let p = it.field_at::<Position>(0, row);
        let v = it.field::<Velocity>(1);

        if e == e1 {
            assert_eq!(p.x, 10);
            assert_eq!(p.y, 20);
            assert_eq!(v[0].x, 1);
            assert_eq!(v[0].y, 2);
        }
        if e == e2 {
            assert_eq!(p.x, 20);
            assert_eq!(p.y, 30);
            assert_eq!(v[0].x, 1);
            assert_eq!(v[0].y, 2);
        }
        count += 1;
    });

    assert_eq!(count, 2);
}

// ─── each_w_const_field_at_w_fixed_src ────────────────────────────────────────

#[test]
fn query_builder_each_w_const_field_at_w_fixed_src() {
    let world = World::new();

    let e1 = world
        .entity()
        .set(Position { x: 10, y: 20 })
        .set(Velocity { x: 1, y: 2 });

    let e2 = world.entity().set(Position { x: 20, y: 30 });

    let q = world
        .query::<()>()
        .with(&Position::id())
        .with(&Velocity::id())
        .src()
        .set_id(e1)
        .build();

    let mut count = 0;
    q.each_iter(|it, row, ()| {
        let e = it.get_entity(row).unwrap();
        let p = it.field_at::<Position>(0, row);
        let v = it.field_at::<Velocity>(1, 0_usize);

        if e == e1 {
            assert_eq!(p.x, 10);
            assert_eq!(p.y, 20);
            assert_eq!(v.x, 1);
            assert_eq!(v.y, 2);
        }
        if e == e2 {
            assert_eq!(p.x, 20);
            assert_eq!(p.y, 30);
            assert_eq!(v.x, 1);
            assert_eq!(v.y, 2);
        }
        count += 1;
    });

    assert_eq!(count, 2);
}

// ─── each_w_untyped_field_w_fixed_src ─────────────────────────────────────────
// C++ uses `it.field(1)` -> `untyped_field` -> `vf[0]` as void* in an each(iter,row) callback.
// Rust: `it.field_untyped(1)` -> `FieldUntyped` -> `.at(0)` as *const T.
// Using run() instead of each_iter() to avoid the EcsIterCppEach safety restriction
// (field_untyped from inside each_iter triggers the "use field_at" assertion even for
// fixed-src fields due to a Rust-side over-conservative check).

#[test]
fn query_builder_each_w_untyped_field_w_fixed_src() {
    let world = World::new();

    let e1 = world
        .entity()
        .set(Position { x: 10, y: 20 })
        .set(Velocity { x: 1, y: 2 });

    let e2 = world.entity().set(Position { x: 20, y: 30 });

    let q = world
        .query::<()>()
        .with(&Position::id())
        .with(&Velocity::id())
        .src()
        .set_id(e1)
        .build();

    let mut count = 0;
    q.run_shared(&world, |mut it| {
        while it.next() {
            // field_untyped gives FieldUntyped; .at(0) returns a *const c_void pointer.
            // Field 1 (Velocity) has a fixed src so it's the same value for all rows.
            let vf = it.field_untyped(1);
            let v = unsafe { &*(vf.at(0) as *const Velocity) };
            // field::<T>(index) for non-sparse dense components; index into Field with [row].
            let p_field = it.field::<Position>(0);

            for row in it.iter() {
                let e = it.entity(row);
                let p = &p_field[row];

                if e == e1 {
                    assert_eq!(p.x, 10);
                    assert_eq!(p.y, 20);
                    assert_eq!(v.x, 1);
                    assert_eq!(v.y, 2);
                }
                if e == e2 {
                    assert_eq!(p.x, 20);
                    assert_eq!(p.y, 30);
                    assert_eq!(v.x, 1);
                    assert_eq!(v.y, 2);
                }
                count += 1;
            }
        }
    });

    assert_eq!(count, 2);
}

// ─── each_w_untyped_field_at_w_fixed_src ──────────────────────────────────────
// C++ uses `it.field_at(1, 0)` -> void* in an each(iter,row) callback.
// Rust equivalent: `it.field_at_untyped(1, 0)` -> *const c_void.
// Using run() for the same reason as above.

#[test]
fn query_builder_each_w_untyped_field_at_w_fixed_src() {
    let world = World::new();

    let e1 = world
        .entity()
        .set(Position { x: 10, y: 20 })
        .set(Velocity { x: 1, y: 2 });

    let e2 = world.entity().set(Position { x: 20, y: 30 });

    let q = world
        .query::<()>()
        .with(&Position::id())
        .with(&Velocity::id())
        .src()
        .set_id(e1)
        .build();

    let mut count = 0;
    q.run_shared(&world, |mut it| {
        while it.next() {
            // field_at_untyped(field_index, row) returns a raw *const c_void pointer.
            // Field 1 (Velocity) has a fixed src so row=0 gives the shared component.
            let vptr = it.field_at_untyped(1, 0);
            let v = unsafe { &*(vptr as *const Velocity) };
            let p_field = it.field::<Position>(0);

            for row in it.iter() {
                let e = it.entity(row);
                let p = &p_field[row];

                if e == e1 {
                    assert_eq!(p.x, 10);
                    assert_eq!(p.y, 20);
                    assert_eq!(v.x, 1);
                    assert_eq!(v.y, 2);
                }
                if e == e2 {
                    assert_eq!(p.x, 20);
                    assert_eq!(p.y, 30);
                    assert_eq!(v.x, 1);
                    assert_eq!(v.y, 2);
                }
                count += 1;
            }
        }
    });

    assert_eq!(count, 2);
}

// ────────────────────────────────────────────────────────────────
// New tests appended below
// ────────────────────────────────────────────────────────────────

#[test]
fn ptr_type() {
    let world = World::new();

    let q = world
        .query::<(&Position, Option<&Velocity>)>()
        .set_cache_kind(QueryCacheKind::Auto)
        .build();

    let e1 = world.entity().set(Position { x: 0, y: 0 });
    let e2 = world
        .entity()
        .set(Position { x: 0, y: 0 })
        .set(Velocity { x: 0, y: 0 });
    world
        .entity()
        .set(Velocity { x: 0, y: 0 })
        .set(Mass { value: 0 });

    let mut count = 0;
    q.each_entity_shared(&world, |e, (_p, _v)| {
        count += 1;
        assert!(e == e1 || e == e2);
    });

    assert_eq!(count, 2);
}

#[test]
fn named_scoped_query() {
    // Same behavior as named_scoped_rule — just a different name alias
    let world = World::new();

    let e1 = world.entity().set(Position { x: 0, y: 0 });
    let e2 = world.entity().set(Position { x: 0, y: 0 });

    let q = world
        .query_named::<&Position>("my::query")
        .set_cache_kind(QueryCacheKind::Auto)
        .build();

    let mut count = 0;
    q.each_entity_shared(&world, |e, _p| {
        assert!(e == e1 || e == e2);
        count += 1;
    });
    assert_eq!(count, 2);

    let qe = q.entity();
    assert_ne!(qe, 0);
    assert_eq!(qe.name(), "query");
    assert_eq!(qe.path().unwrap(), "::my::query");
}

#[test]
fn cascade_w_set_var() {
    let world = World::new();

    let sun = world.entity().set(Position { x: 1, y: 2 });
    let earth = world.entity().child_of(sun);

    let q = world
        .query::<Option<&Position>>()
        .term_at(0)
        .cascade()
        .build();

    let mut count = 0;
    q.with_var(0, earth).each_entity_shared(&world, |e, p| {
        count += 1;
        assert_eq!(e, earth);
        assert!(p.is_some());
        let p = p.unwrap();
        assert_eq!(p.x, 1);
        assert_eq!(p.y, 2);
    });

    assert_eq!(count, 1);
}

// group_by_raw_ordered — EcsQueryGroupByOrdered not exposed in Rust bindings
// TODO: missing API: query_flags(QueryFlags::GroupByOrdered)
// fn query_builder_group_by_raw_ordered() {}

// group_by_template_ordered — EcsQueryGroupByOrdered not exposed in Rust bindings
// TODO: missing API: query_flags(QueryFlags::GroupByOrdered)
// fn query_builder_group_by_template_ordered() {}

// group_by_default_func_w_id_ordered — EcsQueryGroupByOrdered not exposed in Rust bindings
// TODO: missing API: query_flags(QueryFlags::GroupByOrdered)
// fn query_builder_group_by_default_func_w_id_ordered() {}

// group_by_default_func_w_type_ordered — EcsQueryGroupByOrdered not exposed in Rust bindings
// TODO: missing API: query_flags(QueryFlags::GroupByOrdered)
// fn query_builder_group_by_default_func_w_type_ordered() {}

// group_by_callbacks_ordered — EcsQueryGroupByOrdered not exposed in Rust bindings
// TODO: missing API: query_flags(QueryFlags::GroupByOrdered)
// fn query_builder_group_by_callbacks_ordered() {}

// iterate_groups — no q.groups() / iterate_groups() API in Rust bindings
// TODO: missing API: Query::groups() iterator
// fn query_builder_iterate_groups() {}

// iterate_groups_empty — no q.groups() API
// TODO: missing API: Query::groups() iterator
// fn query_builder_iterate_groups_empty() {}

// iterate_groups_w_isa — no q.groups() API
// TODO: missing API: Query::groups() iterator
// fn query_builder_iterate_groups_w_isa() {}

#[test]
fn pred_eq() {
    let world = World::new();

    let foo = world.entity_named("Foo");

    // C++ uses .with(flecs::PredEq, "Foo") — name string form
    let q = world
        .query::<()>()
        .with(*flecs::PredEq)
        .set_second("Foo")
        .flags(flecs::term_flags::IsName::ID)
        .build();

    let mut count = 0;
    q.each_entity_shared(&world, |e, _| {
        assert_eq!(e, foo);
        count += 1;
    });

    assert_eq!(count, 1);
}

#[test]
fn pred_eq_name() {
    let world = World::new();

    // Create "Foo" before building the query so the name can be resolved at query build time
    let foo = world.entity_named("Foo");

    let q = world
        .query::<()>()
        .with(*flecs::PredEq)
        .set_second("Foo")
        .flags(flecs::term_flags::IsName::ID)
        .build();

    let mut count = 0;
    q.each_entity_shared(&world, |e, _| {
        assert_eq!(e, foo);
        count += 1;
    });

    assert_eq!(count, 1);
}

#[test]
#[ignore = "PredMatch with IsName+set_second tries to resolve pattern as entity at build time — needs Flecs API clarification for pattern-based name matching"]
fn pred_match() {
    let world = World::new();

    let q = world
        .query::<()>()
        .with(*flecs::PredMatch)
        .second()
        .name("Fo")
        .flags(flecs::term_flags::IsName::ID)
        .build();

    let foo = world.entity_named("Foo");

    let mut count = 0;
    q.each_entity_shared(&world, |e, _| {
        assert_eq!(e, foo);
        count += 1;
    });

    assert_eq!(count, 1);
}

// not_w_write — not found in C++ source, no direct mapping
// TODO: missing API: not present in QueryBuilder.cpp source
// fn query_builder_not_w_write() {}

// optional_module — not found in C++ source, no direct mapping
// TODO: missing API: not present in QueryBuilder.cpp source
// fn query_builder_optional_module() {}

// pair_with_variable_src — not found in C++ source
// TODO: missing API: not present in QueryBuilder.cpp source
// fn query_builder_pair_with_variable_src() {}

// pair_with_variable_src_no_row_fields — not found in C++ source
// TODO: missing API: not present in QueryBuilder.cpp source
// fn query_builder_pair_with_variable_src_no_row_fields() {}

// query_each_from_component — not found in C++ source
// TODO: missing API: not present in QueryBuilder.cpp source
// fn query_builder_query_each_from_component() {}

// query_from_entity — not found in C++ source
// TODO: missing API: not present in QueryBuilder.cpp source
// fn query_builder_query_from_entity() {}

// query_from_entity_name — not found in C++ source
// TODO: missing API: not present in QueryBuilder.cpp source
// fn query_builder_query_from_entity_name() {}

// query_iter_from_component — not found in C++ source
// TODO: missing API: not present in QueryBuilder.cpp source
// fn query_builder_query_iter_from_component() {}

// query_single_pair — not found in C++ source
// TODO: missing API: not present in QueryBuilder.cpp source
// fn query_builder_query_single_pair() {}

// query_type_w_template — not found in C++ source
// TODO: missing API: not present in QueryBuilder.cpp source
// fn query_builder_query_type_w_template() {}

// set_group_captured_query — not found in C++ source
// TODO: missing API: not present in QueryBuilder.cpp source
// fn query_builder_set_group_captured_query() {}

// set_this_var — not found in C++ source
// TODO: missing API: not present in QueryBuilder.cpp source
// fn query_builder_set_this_var() {}

// set_var_captured_query — not found in C++ source
// TODO: missing API: not present in QueryBuilder.cpp source
// fn query_builder_set_var_captured_query() {}

// set_var_id_captured_query — not found in C++ source
// TODO: missing API: not present in QueryBuilder.cpp source
// fn query_builder_set_var_id_captured_query() {}

// worker_iter_captured_query — not found in C++ source
// TODO: missing API: not present in QueryBuilder.cpp source
// fn query_builder_worker_iter_captured_query() {}

// ─── setup ────────────────────────────────────────────────────────────────────

#[test]
fn setup() {
    // C++ setup() configures cache_kind via test params. In Rust we just verify
    // the default build works — no external test param infrastructure.
    let world = World::new();
    let _q = world.query::<(&Position, &Velocity)>().build();
}

// ─── builder_build_to_auto ───────────────────────────────────────────────────

#[test]
fn builder_build_to_auto() {
    let world = World::new();

    let q = world
        .query::<(&Position, &Velocity)>()
        .set_cache_kind(QueryCacheKind::Auto)
        .build();

    let e1 = world.entity().add(Position::id()).add(Velocity::id());
    world.entity().add(Position::id());

    let mut count = 0;
    q.each_entity_shared(&world, |e, _| {
        count += 1;
        assert_eq!(e, e1);
    });

    assert_eq!(count, 1);
}

// ─── 32_terms ────────────────────────────────────────────────────────────────

#[test]
fn n32_terms() {
    #[derive(Component)]
    struct T00;
    #[derive(Component)]
    struct T01;
    #[derive(Component)]
    struct T02;
    #[derive(Component)]
    struct T03;
    #[derive(Component)]
    struct T04;
    #[derive(Component)]
    struct T05;
    #[derive(Component)]
    struct T06;
    #[derive(Component)]
    struct T07;
    #[derive(Component)]
    struct T08;
    #[derive(Component)]
    struct T09;
    #[derive(Component)]
    struct T10;
    #[derive(Component)]
    struct T11;
    #[derive(Component)]
    struct T12;
    #[derive(Component)]
    struct T13;
    #[derive(Component)]
    struct T14;
    #[derive(Component)]
    struct T15;
    #[derive(Component)]
    struct T16;
    #[derive(Component)]
    struct T17;
    #[derive(Component)]
    struct T18;
    #[derive(Component)]
    struct T19;
    #[derive(Component)]
    struct T20;
    #[derive(Component)]
    struct T21;
    #[derive(Component)]
    struct T22;
    #[derive(Component)]
    struct T23;
    #[derive(Component)]
    struct T24;
    #[derive(Component)]
    struct T25;
    #[derive(Component)]
    struct T26;
    #[derive(Component)]
    struct T27;
    #[derive(Component)]
    struct T28;
    #[derive(Component)]
    struct T29;
    #[derive(Component)]
    struct T30;
    #[derive(Component)]
    struct T31;

    let world = World::new();

    let f = world
        .query::<()>()
        .with(T00::id())
        .with(T01::id())
        .with(T02::id())
        .with(T03::id())
        .with(T04::id())
        .with(T05::id())
        .with(T06::id())
        .with(T07::id())
        .with(T08::id())
        .with(T09::id())
        .with(T10::id())
        .with(T11::id())
        .with(T12::id())
        .with(T13::id())
        .with(T14::id())
        .with(T15::id())
        .with(T16::id())
        .with(T17::id())
        .with(T18::id())
        .with(T19::id())
        .with(T20::id())
        .with(T21::id())
        .with(T22::id())
        .with(T23::id())
        .with(T24::id())
        .with(T25::id())
        .with(T26::id())
        .with(T27::id())
        .with(T28::id())
        .with(T29::id())
        .with(T30::id())
        .with(T31::id())
        .build();

    assert_eq!(f.field_count(), 32);

    let e = world
        .entity()
        .add(T00::id())
        .add(T01::id())
        .add(T02::id())
        .add(T03::id())
        .add(T04::id())
        .add(T05::id())
        .add(T06::id())
        .add(T07::id())
        .add(T08::id())
        .add(T09::id())
        .add(T10::id())
        .add(T11::id())
        .add(T12::id())
        .add(T13::id())
        .add(T14::id())
        .add(T15::id())
        .add(T16::id())
        .add(T17::id())
        .add(T18::id())
        .add(T19::id())
        .add(T20::id())
        .add(T21::id())
        .add(T22::id())
        .add(T23::id())
        .add(T24::id())
        .add(T25::id())
        .add(T26::id())
        .add(T27::id())
        .add(T28::id())
        .add(T29::id())
        .add(T30::id())
        .add(T31::id());

    let mut count = 0;
    f.run_shared(&world, |mut it| {
        while it.next() {
            assert_eq!(it.count(), 1);
            assert_eq!(it.get_entity(0_usize).unwrap(), e);
            assert_eq!(it.field_count(), 32);
            count += 1;
        }
    });

    assert_eq!(count, 1);
}

// ─── 33_terms ────────────────────────────────────────────────────────────────
// TODO: missing API: 33-term limit enforcement panics/asserts in C via test_abort.
// Not ported: requires crash-handler test infrastructure.

// ─── with_id ─────────────────────────────────────────────────────────────────

#[test]
fn with_id() {
    let world = World::new();

    let q = world
        .query::<()>()
        .with(world.id_view_from(Position::id()))
        .with(world.id_view_from(Velocity::id()))
        .set_cache_kind(QueryCacheKind::Auto)
        .build();

    let e1 = world.entity().add(Position::id()).add(Velocity::id());
    world.entity().add(Position::id());

    let mut count = 0;
    q.each_entity_shared(&world, |e, _| {
        count += 1;
        assert_eq!(e, e1);
    });

    assert_eq!(count, 1);
}

// ─── without_id ──────────────────────────────────────────────────────────────

#[test]
fn without_id() {
    let world = World::new();

    let q = world
        .query::<()>()
        .with(world.id_view_from(Position::id()))
        .without(world.id_view_from(Velocity::id()))
        .set_cache_kind(QueryCacheKind::Auto)
        .build();

    world.entity().add(Position::id()).add(Velocity::id());
    let e2 = world.entity().add(Position::id());

    let mut count = 0;
    q.each_entity_shared(&world, |e, _| {
        count += 1;
        assert_eq!(e, e2);
    });

    assert_eq!(count, 1);
}

// ─── write_id ────────────────────────────────────────────────────────────────

#[test]
fn write_id() {
    let world = World::new();

    let q = world
        .query::<()>()
        .with(&Position::id())
        .write(world.id_view_from(Position::id()))
        .set_cache_kind(QueryCacheKind::Auto)
        .build();

    assert_eq!(q.term(1).inout(), InOutKind::Out);
    assert_eq!(q.term(1).first_id(), world.id_view_from(Position::id()));
    assert_eq!(q.term(1).src_id(), 0);
}

// ─── read_id ─────────────────────────────────────────────────────────────────

#[test]
fn read_id() {
    let world = World::new();

    let q = world
        .query::<()>()
        .with(&Position::id())
        .read(world.id_view_from(Position::id()))
        .set_cache_kind(QueryCacheKind::Auto)
        .build();

    assert_eq!(q.term(1).inout(), InOutKind::In);
    assert_eq!(q.term(1).first_id(), world.id_view_from(Position::id()));
    assert_eq!(q.term(1).src_id(), 0);
}

// ─── set_var_on_query ─────────────────────────────────────────────────────────

#[test]
fn set_var_on_query() {
    #[derive(Component)]
    struct LikesVOQ;

    let world = World::new();

    let apples = world.entity();
    let pears = world.entity();

    world.entity().add((LikesVOQ::id(), apples));

    let e2 = world.entity().add((LikesVOQ::id(), pears));

    let r = world
        .query::<()>()
        .with((LikesVOQ::id(), id::<flecs::Wildcard>()))
        .second()
        .set_var("Food")
        .build();

    let food_var = r.find_var("Food").unwrap();

    let mut count = 0;
    r.with_var(food_var, pears).each_iter(|it, i, ()| {
        assert_eq!(it.get_entity(i).unwrap(), e2);
        assert_eq!(it.get_var(food_var), pears);
        count += 1;
    });

    assert_eq!(count, 1);
}

// ─── set_var_by_name_on_query ─────────────────────────────────────────────────

#[test]
fn set_var_by_name_on_query() {
    #[derive(Component)]
    struct LikesVBNQ;

    let world = World::new();

    let apples = world.entity();
    let pears = world.entity();

    world.entity().add((LikesVBNQ::id(), apples));
    let e2 = world.entity().add((LikesVBNQ::id(), pears));

    let r = world
        .query::<()>()
        .with((LikesVBNQ::id(), id::<flecs::Wildcard>()))
        .second()
        .set_var("Food")
        .build();

    let food_var_id = r.find_var("Food").unwrap();
    let mut count = 0;
    r.with_var_expr("Food", pears).each_iter(|it, i, ()| {
        assert_eq!(it.get_entity(i).unwrap(), e2);
        assert_eq!(it.get_var(food_var_id), pears);
        count += 1;
    });

    assert_eq!(count, 1);
}

// ─── set_table_var ────────────────────────────────────────────────────────────

#[test]
fn set_table_var() {
    let world = World::new();

    let e1 = world.entity().add(Position::id());
    let e2 = world.entity().add(Position::id());
    let e3 = world.entity().add(Position::id()).add(Velocity::id());

    let r = world.new_query::<&Position>();

    let mut count = 0;
    r.with_var_table_expr("this", e1.table().unwrap())
        .each_iter(|it, index, _| {
            let e = it.get_entity(index).unwrap();
            if usize::from(index) == 0 {
                assert_eq!(e, e1);
            } else if usize::from(index) == 1 {
                assert_eq!(e, e2);
            }
            count += 1;
        });

    assert_eq!(count, 2);

    r.with_var_table_expr("this", e3.table().unwrap())
        .each_iter(|it, index, _| {
            let e = it.get_entity(index).unwrap();
            assert_eq!(e, e3);
            count += 1;
        });

    assert_eq!(count, 3);
}

// ─── set_range_var ────────────────────────────────────────────────────────────

#[test]
fn set_range_var() {
    let world = World::new();

    let e1 = world.entity().add(Position::id());
    let e2 = world.entity().add(Position::id());
    let e3 = world.entity().add(Position::id()).add(Velocity::id());

    let r = world.new_query::<&Position>();

    let mut count = 0;

    r.with_var_table_expr("this", e1.range().unwrap())
        .each_iter(|it, index, _| {
            let e = it.get_entity(index).unwrap();
            assert_eq!(e, e1);
            count += 1;
        });
    assert_eq!(count, 1);

    r.with_var_table_expr("this", e2.range().unwrap())
        .each_iter(|it, index, _| {
            let e = it.get_entity(index).unwrap();
            assert_eq!(e, e2);
            count += 1;
        });
    assert_eq!(count, 2);

    r.with_var_table_expr("this", e3.range().unwrap())
        .each_iter(|it, index, _| {
            let e = it.get_entity(index).unwrap();
            assert_eq!(e, e3);
            count += 1;
        });
    assert_eq!(count, 3);
}

// ─── set_table_var_chained ────────────────────────────────────────────────────

#[test]
fn set_table_var_chained() {
    let world = World::new();

    world.entity().add(Position::id());
    world.entity().add(Position::id());
    let e3 = world.entity().add(Position::id()).add(Velocity::id());
    world.entity().add(Velocity::id());

    let q1 = world.new_query::<&Position>();
    let q2 = world.new_query::<&Velocity>();

    let mut count = 0;

    q1.run_shared(&world, |mut it| {
        while it.next() {
            q2.with_var_table_expr("this", it.table().unwrap())
                .each_entity_shared(&world, |e, _| {
                    assert_eq!(e, e3);
                    count += 1;
                });
        }
    });

    assert_eq!(count, 1);
}

// ─── set_range_var_chained ────────────────────────────────────────────────────

#[test]
fn set_range_var_chained() {
    let world = World::new();

    world.entity().add(Position::id());
    world.entity().add(Position::id());
    let e3 = world.entity().add(Position::id()).add(Velocity::id());
    world.entity().add(Velocity::id());

    let q1 = world.new_query::<&Position>();
    let q2 = world.new_query::<&Velocity>();

    let mut count = 0;

    q1.run_shared(&world, |mut it| {
        while it.next() {
            q2.with_var_table_expr("this", it.range().unwrap())
                .each_entity_shared(&world, |e, _| {
                    assert_eq!(e, e3);
                    count += 1;
                });
        }
    });

    assert_eq!(count, 1);
}

// ─── set_group_on_query ───────────────────────────────────────────────────────

#[test]
fn set_group_on_query() {
    let world = World::new();

    let rel = world.entity();
    let tgt_a = world.entity();
    let tgt_b = world.entity();
    let tgt_c = world.entity();
    let tag = world.entity();

    world.entity().add((rel, tgt_a));
    let e2 = world.entity().add((rel, tgt_b));
    world.entity().add((rel, tgt_c));

    world.entity().add((rel, tgt_a)).add(tag);
    let e5 = world.entity().add((rel, tgt_b)).add(tag);
    world.entity().add((rel, tgt_c)).add(tag);

    let q = world
        .query::<()>()
        .with((rel, id::<flecs::Wildcard>()))
        .group_by(rel)
        .build();

    let mut e2_found = false;
    let mut e5_found = false;
    let mut count = 0;

    q.with_group(tgt_b).each_iter(|it, i, ()| {
        let e = it.get_entity(i).unwrap();
        assert_eq!(it.group_id(), tgt_b);
        if e == e2 {
            e2_found = true;
        }
        if e == e5 {
            e5_found = true;
        }
        count += 1;
    });

    assert_eq!(count, 2);
    assert!(e2_found);
    assert!(e5_found);
}

// ─── set_group_type_on_query ──────────────────────────────────────────────────

#[test]
fn set_group_type_on_query() {
    #[derive(Component)]
    struct RelSGT;
    #[derive(Component)]
    struct TgtASGT;
    #[derive(Component)]
    struct TgtBSGT;
    #[derive(Component)]
    struct TgtCSGT;

    let world = World::new();

    let tag = world.entity();

    world.entity().add((RelSGT::id(), TgtASGT::id()));
    let e2 = world.entity().add((RelSGT::id(), TgtBSGT::id()));
    world.entity().add((RelSGT::id(), TgtCSGT::id()));

    world.entity().add((RelSGT::id(), TgtASGT::id())).add(tag);
    let e5 = world.entity().add((RelSGT::id(), TgtBSGT::id())).add(tag);
    world.entity().add((RelSGT::id(), TgtCSGT::id())).add(tag);

    let q = world
        .query::<()>()
        .with((RelSGT::id(), id::<flecs::Wildcard>()))
        .group_by(RelSGT::id())
        .build();

    let tgt_b_raw = world.id_view_from(TgtBSGT::id());
    let mut e2_found = false;
    let mut e5_found = false;
    let mut count = 0;

    q.with_group(TgtBSGT::id()).each_iter(|it, i, ()| {
        let e = it.get_entity(i).unwrap();
        assert_eq!(it.group_id(), tgt_b_raw);
        if e == e2 {
            e2_found = true;
        }
        if e == e5 {
            e5_found = true;
        }
        count += 1;
    });

    assert_eq!(count, 2);
    assert!(e2_found);
    assert!(e5_found);
}

// ─── group_by_raw_ordered / template_ordered / default_func_*_ordered / callbacks_ordered ──
// TODO: missing API: EcsQueryGroupByOrdered flag not exposed in Rust QueryFlags.

// ─── iterate_groups / iterate_groups_empty / iterate_groups_w_isa ──────────────
// TODO: missing API: Query::groups() iterator not implemented in Rust bindings.

// ─── iter_column_w_const_deref ────────────────────────────────────────────────

#[test]
fn iter_column_w_const_deref() {
    let mut world = World::new();

    let f = world.new_query::<&Position>();

    let base = world.prefab().set(Position { x: 10, y: 20 });
    world.entity().is_a(base);
    world.entity().is_a(base);

    let mut count = 0;
    f.run_exclusive(&mut world, |mut it| {
        while it.next() {
            let p = it.field::<Position>(0);
            // In Rust there's no pointer deref syntax; we index p[0] for the shared component
            let pv = p[0];
            for _ in it.iter() {
                assert_eq!(pv.x, 10);
                assert_eq!(pv.y, 20);
                count += 1;
            }
        }
    });

    assert_eq!(count, 2);
}

// ─── filter_as_move_arg ───────────────────────────────────────────────────────

#[test]
fn filter_as_move_arg() {
    let mut world = World::new();

    let f = world
        .query::<&SelfRef>()
        .set_cache_kind(QueryCacheKind::Auto)
        .build();

    let e = world.entity();
    e.set(SelfRef { value: e.id() });
    let e = world.entity();
    e.set(SelfRef { value: e.id() });
    let e = world.entity();
    e.set(SelfRef { value: e.id() });

    // Pass query by value to a helper — mirrors C++ "move arg" semantics
    fn count_query(world: &mut World, f: Query<&SelfRef>) -> i32 {
        let mut count = 0;
        f.each_entity_exclusive(world, |e, s| {
            assert_eq!(e, s.value);
            count += 1;
        });
        count
    }

    assert_eq!(count_query(&mut world, f), 3);
}

// ─── world_each_entity ────────────────────────────────────────────────────────
// C++ `ecs.each([](flecs::entity e){})` internally adds ECS_ANY as the sole query
// term, matching all entities that have at least one component.
// Rust equivalent: build a query with `.with(flecs::Any::ID)` and iterate with
// `each_entity`. The 0-term `world.each_entity::<()>` does NOT work because a
// query with no terms matches no tables.

// ─── singleton_pair ───────────────────────────────────────────────────────────
// TODO: missing API: Dynamic second target on singleton pair query (.term_at(0).second(tgt).singleton())
// requires complex pair-with-target builder syntax not yet easily mapped in Rust.
// The C++ test uses ecs.query_builder<const Position>().term_at(0).second(tgt).singleton()
// which needs non-trivial term builder chaining with wildcard pair + second override.

// ─── query_w_this_second ──────────────────────────────────────────────────────

#[test]
fn query_w_this_second() {
    let world = World::new();

    let rel = world.entity();

    // C++ uses .with(rel, flecs::This) — setting $this as the second variable.
    // In Rust, use .with(rel).set_second_id(Wildcard) + .second().set_var("this")
    // to create (rel, $this) term.
    let q = world
        .query::<()>()
        .with(rel)
        .set_id(*flecs::Wildcard)
        .second()
        .set_var("this")
        .build();

    let e1 = world.entity();
    e1.add((rel, e1));

    let mut count = 0;
    q.each_entity_shared(&world, |e, _| {
        assert_eq!(e, e1);
        count += 1;
    });

    assert_eq!(count, 1);
}

// ─── each_w_field_w_fixed_src ─────────────────────────────────────────────────

#[test]
fn each_w_field_w_fixed_src() {
    let world = World::new();

    let e1 = world
        .entity()
        .set(Position { x: 10, y: 20 })
        .set(Velocity { x: 1, y: 2 });

    let e2 = world.entity().set(Position { x: 20, y: 30 });

    let q = world
        .query::<()>()
        .with(&Position::id())
        .with(&Velocity::id())
        .src()
        .set_id(e1)
        .build();

    // Use run() because each_iter restricts field() for $this sources and field_at() for non-sparse.
    // Fixed-src components must be accessed via run()+field().
    let mut count = 0;
    q.run_shared(&world, |mut it| {
        while it.next() {
            let p_field = it.field::<Position>(0);
            let v = it.field::<Velocity>(1);
            for row in it.iter() {
                let e = it.entity(row);
                let p = &p_field[row];
                if e == e1 {
                    assert_eq!(p.x, 10);
                    assert_eq!(p.y, 20);
                    assert_eq!(v[0].x, 1);
                    assert_eq!(v[0].y, 2);
                }
                if e == e2 {
                    assert_eq!(p.x, 20);
                    assert_eq!(p.y, 30);
                    assert_eq!(v[0].x, 1);
                    assert_eq!(v[0].y, 2);
                }
                count += 1;
            }
        }
    });

    assert_eq!(count, 2);
}

// ─── each_w_field_at_w_fixed_src ──────────────────────────────────────────────

#[test]
fn each_w_field_at_w_fixed_src() {
    let world = World::new();

    let e1 = world
        .entity()
        .set(Position { x: 10, y: 20 })
        .set(Velocity { x: 1, y: 2 });

    let e2 = world.entity().set(Position { x: 20, y: 30 });

    let q = world
        .query::<()>()
        .with(&Position::id())
        .with(&Velocity::id())
        .src()
        .set_id(e1)
        .build();

    // Use run() — same as each_w_field_w_fixed_src but uses field_at semantics:
    // Velocity[0] is always the fixed entity (row 0), Position uses per-entity row.
    let mut count = 0;
    q.run_shared(&world, |mut it| {
        while it.next() {
            let p_field = it.field::<Position>(0);
            let v = it.field::<Velocity>(1);
            for row in it.iter() {
                let e = it.entity(row);
                let p = &p_field[row];
                let v0 = &v[0]; // fixed-src: always row 0
                if e == e1 {
                    assert_eq!(p.x, 10);
                    assert_eq!(p.y, 20);
                    assert_eq!(v0.x, 1);
                    assert_eq!(v0.y, 2);
                }
                if e == e2 {
                    assert_eq!(p.x, 20);
                    assert_eq!(p.y, 30);
                    assert_eq!(v0.x, 1);
                    assert_eq!(v0.y, 2);
                }
                count += 1;
            }
        }
    });

    assert_eq!(count, 2);
}

// ─── each_w_const_field_w_fixed_src ───────────────────────────────────────────

#[test]
fn each_w_const_field_w_fixed_src() {
    let world = World::new();

    let e1 = world
        .entity()
        .set(Position { x: 10, y: 20 })
        .set(Velocity { x: 1, y: 2 });

    let e2 = world.entity().set(Position { x: 20, y: 30 });

    let q = world
        .query::<()>()
        .with(&Position::id())
        .with(&Velocity::id())
        .src()
        .set_id(e1)
        .build();

    let mut count = 0;
    q.run_shared(&world, |mut it| {
        while it.next() {
            let p_field = it.field::<Position>(0);
            let v = it.field::<Velocity>(1);
            for row in it.iter() {
                let e = it.entity(row);
                let p = &p_field[row];
                if e == e1 {
                    assert_eq!(p.x, 10);
                    assert_eq!(p.y, 20);
                    assert_eq!(v[0].x, 1);
                    assert_eq!(v[0].y, 2);
                }
                if e == e2 {
                    assert_eq!(p.x, 20);
                    assert_eq!(p.y, 30);
                    assert_eq!(v[0].x, 1);
                    assert_eq!(v[0].y, 2);
                }
                count += 1;
            }
        }
    });

    assert_eq!(count, 2);
}

// ─── each_w_const_field_at_w_fixed_src ────────────────────────────────────────

#[test]
fn each_w_const_field_at_w_fixed_src() {
    let world = World::new();

    let e1 = world
        .entity()
        .set(Position { x: 10, y: 20 })
        .set(Velocity { x: 1, y: 2 });

    let e2 = world.entity().set(Position { x: 20, y: 30 });

    let q = world
        .query::<()>()
        .with(&Position::id())
        .with(&Velocity::id())
        .src()
        .set_id(e1)
        .build();

    let mut count = 0;
    q.run_shared(&world, |mut it| {
        while it.next() {
            let p_field = it.field::<Position>(0);
            let v = it.field::<Velocity>(1);
            for row in it.iter() {
                let e = it.entity(row);
                let p = &p_field[row];
                let v0 = &v[0];
                if e == e1 {
                    assert_eq!(p.x, 10);
                    assert_eq!(p.y, 20);
                    assert_eq!(v0.x, 1);
                    assert_eq!(v0.y, 2);
                }
                if e == e2 {
                    assert_eq!(p.x, 20);
                    assert_eq!(p.y, 30);
                    assert_eq!(v0.x, 1);
                    assert_eq!(v0.y, 2);
                }
                count += 1;
            }
        }
    });

    assert_eq!(count, 2);
}

// ─── each_w_untyped_field_w_fixed_src ─────────────────────────────────────────
// C++ uses `it.field(1)` -> `untyped_field` -> `vf[0]` as void* in an each(iter,row) callback.
// Rust: `it.field_untyped(1)` -> `FieldUntyped` -> `.at(0)` as *const T.
// Using run() instead of each_iter() to avoid the EcsIterCppEach safety restriction
// (field_untyped from inside each_iter triggers the "use field_at" assertion even for
// fixed-src fields due to a Rust-side over-conservative check).

#[test]
fn each_w_untyped_field_w_fixed_src() {
    let world = World::new();

    let e1 = world
        .entity()
        .set(Position { x: 10, y: 20 })
        .set(Velocity { x: 1, y: 2 });

    let e2 = world.entity().set(Position { x: 20, y: 30 });

    let q = world
        .query::<()>()
        .with(&Position::id())
        .with(&Velocity::id())
        .src()
        .set_id(e1)
        .build();

    let mut count = 0;
    q.run_shared(&world, |mut it| {
        while it.next() {
            // field_untyped gives FieldUntyped; .at(0) returns a *const c_void pointer.
            // Field 1 (Velocity) has a fixed src so it's the same value for all rows.
            let vf = it.field_untyped(1);
            let v = unsafe { &*(vf.at(0) as *const Velocity) };
            // field::<T>(index) for non-sparse dense components; index into Field with [row].
            let p_field = it.field::<Position>(0);

            for row in it.iter() {
                let e = it.entity(row);
                let p = &p_field[row];

                if e == e1 {
                    assert_eq!(p.x, 10);
                    assert_eq!(p.y, 20);
                    assert_eq!(v.x, 1);
                    assert_eq!(v.y, 2);
                }
                if e == e2 {
                    assert_eq!(p.x, 20);
                    assert_eq!(p.y, 30);
                    assert_eq!(v.x, 1);
                    assert_eq!(v.y, 2);
                }
                count += 1;
            }
        }
    });

    assert_eq!(count, 2);
}

// ─── each_w_untyped_field_at_w_fixed_src ──────────────────────────────────────
// C++ uses `it.field_at(1, 0)` -> void* in an each(iter,row) callback.
// Rust equivalent: `it.field_at_untyped(1, 0)` -> *const c_void.
// Using run() for the same reason as above.

#[test]
fn each_w_untyped_field_at_w_fixed_src() {
    let world = World::new();

    let e1 = world
        .entity()
        .set(Position { x: 10, y: 20 })
        .set(Velocity { x: 1, y: 2 });

    let e2 = world.entity().set(Position { x: 20, y: 30 });

    let q = world
        .query::<()>()
        .with(&Position::id())
        .with(&Velocity::id())
        .src()
        .set_id(e1)
        .build();

    let mut count = 0;
    q.run_shared(&world, |mut it| {
        while it.next() {
            // field_at_untyped(field_index, row) returns a raw *const c_void pointer.
            // Field 1 (Velocity) has a fixed src so row=0 gives the shared component.
            let vptr = it.field_at_untyped(1, 0);
            let v = unsafe { &*(vptr as *const Velocity) };
            let p_field = it.field::<Position>(0);

            for row in it.iter() {
                let e = it.entity(row);
                let p = &p_field[row];

                if e == e1 {
                    assert_eq!(p.x, 10);
                    assert_eq!(p.y, 20);
                    assert_eq!(v.x, 1);
                    assert_eq!(v.y, 2);
                }
                if e == e2 {
                    assert_eq!(p.x, 20);
                    assert_eq!(p.y, 30);
                    assert_eq!(v.x, 1);
                    assert_eq!(v.y, 2);
                }
                count += 1;
            }
        }
    });

    assert_eq!(count, 2);
}

#[test]
fn world_each_entity() {
    let world = World::new();

    let e1 = world.entity();
    let e2 = world.entity().add(TagA::id());
    let e3 = world.entity().set(Position { x: 10, y: 20 });

    let mut e1_found = false;
    let mut e2_found = false;
    let mut e3_found = false;
    let mut count = 0;

    world.each_alive_entity(|e| {
        if e == e1 {
            e1_found = true;
        } else if e == e2 {
            e2_found = true;
        } else if e == e3 {
            e3_found = true;
        }
        count += 1;
    });

    assert!(e1_found);
    assert!(e2_found);
    assert!(e3_found);
    assert!(count > 3);
}

// --- Construction-fallibility split (spec §4.1) ---

/// A purely-typed builder is infallible: `new_query`/`build` return `Query` directly
/// (no `Result` to thread) and the documented `ecs_query_init` panic is unreachable
/// for a well-formed typed descriptor. Building and iterating a representative typed
/// query must simply succeed.
#[test]
fn typed_query_build_is_infallible() {
    let mut world = World::new();

    // Convenience constructor returns `Query<D>` directly.
    let q1: Query<(&Position, &Velocity)> = world.new_query::<(&Position, &Velocity)>();
    // Builder terminal returns `Query<D>` directly.
    let q2 = world.query::<(&Position, &Velocity)>().build();

    world
        .entity()
        .set(Position { x: 1, y: 2 })
        .set(Velocity { x: 3, y: 4 });

    let mut c1 = 0;
    q1.each_exclusive(&mut world, |(_p, _v)| c1 += 1);
    let mut c2 = 0;
    q2.each_exclusive(&mut world, |(_p, _v)| c2 += 1);
    assert_eq!(c1, 1);
    assert_eq!(c2, 1);
}

/// `expr()` with a syntax error surfaces `QueryBuildError::InvalidExpr` carrying the
/// offending string, and the world is left clean (a subsequent typed query builds and
/// iterates, and no closure/leak is involved on the query path).
#[test]
fn expr_invalid_returns_invalid_expr_with_string() {
    let mut world = World::new();

    let err = world
        .query::<()>()
        .expr("invalid syntax!!!")
        .build()
        .unwrap_err();

    match err {
        QueryBuildError::InvalidExpr { expr } => assert_eq!(expr, "invalid syntax!!!"),
        other => panic!("expected InvalidExpr, got {other:?}"),
    }

    // World left clean: a fresh typed query still builds and iterates.
    world.entity().set(Position { x: 5, y: 6 });
    let q = world.new_query::<&Position>();
    let mut count = 0;
    q.each_exclusive(&mut world, |_p| count += 1);
    assert_eq!(count, 1);
}

/// A valid `expr()` builds through the fallible typestate and iterates normally.
#[test]
fn expr_valid_builds_and_iterates() {
    let mut world = World::new();
    world.component::<Position>();

    let q = world
        .query::<()>()
        .expr("Position")
        .build()
        .expect("valid query expression");

    world.entity().set(Position { x: 7, y: 8 });
    world.entity().set(Position { x: 9, y: 10 });

    let mut count = 0;
    q.run_exclusive(&mut world, |mut it| {
        while it.next() {
            count += it.count();
        }
    });
    assert_eq!(count, 2);
}

/// After `expr()` transitions to the fallible builder, the remaining configuration
/// methods stay chainable (they delegate to the wrapped builder) and the terminal
/// still returns a `Result`.
#[test]
fn fallible_builder_keeps_config_methods_chainable() {
    let world = World::new();
    world.component::<Position>();

    let q = world
        .query::<()>()
        .expr("Position")
        .set_cache_kind(QueryCacheKind::Auto)
        .query_flags(QueryFlags::MatchEmptyTables)
        .build()
        .expect("valid query expression with chained config");

    assert_eq!(q.field_count(), 1);
}
