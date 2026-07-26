#![allow(dead_code)]
#![allow(unused_imports)]
use crate::common_test::*;
use flecs_ecs::experimental::prelude::EntityGuardExt;
use flecs_ecs::prelude::*;

#[test]
fn world_factory_entity() {
    let world = World::new();

    let e = world.entity();
    assert_ne!(e, 0u64);
}

#[test]
fn world_factory_entity_w_name() {
    let world = World::new();

    let e = world.entity_named("MyName");
    assert_ne!(e, 0u64);
    assert_eq!(e.name(), "MyName");
}

#[test]
fn world_factory_entity_w_id() {
    let world = World::new();

    let e = world.entity_from_id(100u64);
    assert_eq!(e, 100u64);
}

#[test]
fn world_factory_prefab() {
    let world = World::new();

    let e = world.prefab();
    assert_ne!(e, 0u64);
    assert!(e.has(flecs::Prefab::ID));
}

#[test]
fn world_factory_prefab_w_name() {
    let world = World::new();

    let e = world.prefab_named("MyName");
    assert_ne!(e, 0u64);
    assert!(e.has(flecs::Prefab::ID));
    assert_eq!(e.name(), "MyName");
}

#[test]
fn world_factory_system() {
    let mut world = World::new();

    let s = world
        .system::<(&mut Position, &Velocity)>()
        .each_entity(|_e, (p, v)| {
            p.x += v.x;
            p.y += v.y;
        });

    assert_ne!(*s.id(), 0u64);

    let e = world
        .entity()
        .set(Position { x: 10, y: 20 })
        .set(Velocity { x: 1, y: 2 })
        .id();

    world.progress();

    {
        let p = world.entity_from_id(e).get::<&Position>().unwrap();
        assert_eq!(p.x, 11);
        assert_eq!(p.y, 22);
    };
}

#[test]
fn world_factory_system_w_name() {
    let mut world = World::new();

    let s = world
        .system_named::<(&mut Position, &Velocity)>("MySystem")
        .each_entity(|_e, (p, v)| {
            p.x += v.x;
            p.y += v.y;
        });

    assert_ne!(*s.id(), 0u64);
    assert_eq!(s.name(), "MySystem");

    let e = world
        .entity()
        .set(Position { x: 10, y: 20 })
        .set(Velocity { x: 1, y: 2 })
        .id();

    world.progress();

    {
        let p = world.entity_from_id(e).get::<&Position>().unwrap();
        assert_eq!(p.x, 11);
        assert_eq!(p.y, 22);
    };
}

#[test]
fn world_factory_system_w_expr() {
    let mut world = World::new();

    world.component::<Position>();
    world.component::<Velocity>();

    let s = world
        .system_named::<()>("MySystem")
        .expr("Position, [in] Velocity")
        .run(|mut it| {
            while it.next() {
                let mut p = it.field_mut::<Position>(0);
                let v = it.field::<Velocity>(1);
                for i in it.iter() {
                    p[i].x += v[i].x;
                    p[i].y += v[i].y;
                }
            }
        })
        .expect("valid system query expression");

    assert_ne!(*s.id(), 0u64);
    assert_eq!(s.name(), "MySystem");

    let e = world
        .entity()
        .set(Position { x: 10, y: 20 })
        .set(Velocity { x: 1, y: 2 })
        .id();

    world.progress();

    {
        let p = world.entity_from_id(e).get::<&Position>().unwrap();
        assert_eq!(p.x, 11);
        assert_eq!(p.y, 22);
    };
}

#[test]
fn world_factory_query() {
    let world = World::new();

    let q = world.new_query::<(&mut Position, &Velocity)>();

    let e = world
        .entity()
        .set(Position { x: 10, y: 20 })
        .set(Velocity { x: 1, y: 2 });

    q.each_entity(|_e, (p, v)| {
        p.x += v.x;
        p.y += v.y;
    });

    {
        let p = e.get::<&Position>().unwrap();
        assert_eq!(p.x, 11);
        assert_eq!(p.y, 22);
    };
}

#[test]
fn world_factory_query_w_expr() {
    let world = World::new();

    world.component::<Position>();
    world.component::<Velocity>();

    let q = world
        .query::<()>()
        .expr("Position, [in] Velocity")
        .build()
        .expect("valid query expression");

    let e = world
        .entity()
        .set(Position { x: 10, y: 20 })
        .set(Velocity { x: 1, y: 2 });

    q.run(|mut it| {
        while it.next() {
            let mut p = it.field_mut::<Position>(0);
            let v = it.field::<Velocity>(1);
            for i in it.iter() {
                p[i].x += v[i].x;
                p[i].y += v[i].y;
            }
        }
    });

    {
        let p = e.get::<&Position>().unwrap();
        assert_eq!(p.x, 11);
        assert_eq!(p.y, 22);
    };
}

#[derive(Component)]
struct MyModule;

impl Module for MyModule {
    fn module(world: &World) {
        world.module::<MyModule>("MyModule");
        world.component::<Position>();
    }
}

#[test]
fn world_factory_module() {
    let world = World::new();

    world.import::<MyModule>();

    let p = world.lookup("MyModule::Position");
    assert_ne!(p, 0u64);
}
