#![allow(dead_code)]
#![allow(clippy::float_cmp)]
#![allow(unused_imports)]
use crate::common_test::*;
use alloc::rc::Rc;
use core::cell::Cell;
use flecs_ecs::prelude::*;

#[test]
fn system_builder_builder_assign_same_type() {
    let world = World::new();

    let e1 = world
        .entity()
        .set(Position { x: 0, y: 0 })
        .set(Velocity { x: 0, y: 0 })
        .id();
    world.entity().set(Position { x: 0, y: 0 });

    let count = Rc::new(Cell::new(0i32));
    let count2 = count.clone();

    let s = world
        .system::<(&Position, &Velocity)>()
        .each_entity(move |e, _| {
            count2.set(count2.get() + 1);
            assert_eq!(e.id(), e1);
        });

    assert_eq!(count.get(), 0);
    s.run();
    assert_eq!(count.get(), 1);
}

#[test]
fn system_builder_builder_build_to_auto() {
    let world = World::new();

    let e1 = world
        .entity()
        .set(Position { x: 0, y: 0 })
        .set(Velocity { x: 0, y: 0 })
        .id();
    world.entity().set(Position { x: 0, y: 0 });

    let count = Rc::new(Cell::new(0i32));
    let count2 = count.clone();

    let s = world
        .system::<(&Position, &Velocity)>()
        .each_entity(move |e, _| {
            count2.set(count2.get() + 1);
            assert_eq!(e.id(), e1);
        });

    assert_eq!(count.get(), 0);
    s.run();
    assert_eq!(count.get(), 1);
}

#[test]
fn system_builder_builder_build_n_statements() {
    let world = World::new();

    let e1 = world
        .entity()
        .set(Position { x: 0, y: 0 })
        .set(Velocity { x: 0, y: 0 })
        .id();
    world.entity().set(Position { x: 0, y: 0 });

    let count = Rc::new(Cell::new(0i32));
    let count2 = count.clone();

    let mut qb = world.system::<()>();
    qb.with(&Position::id());
    qb.with(&Velocity::id());
    let s = qb.each_entity(move |e, _| {
        count2.set(count2.get() + 1);
        assert_eq!(e.id(), e1);
    });

    s.run();

    assert_eq!(count.get(), 1);
}

#[test]
fn system_builder_1_type() {
    let world = World::new();

    let e1 = world.entity().set(Position { x: 0, y: 0 }).id();
    world.entity().set(Velocity { x: 0, y: 0 });

    let count = Rc::new(Cell::new(0i32));
    let count2 = count.clone();

    let s = world.system::<&Position>().each_entity(move |e, _p| {
        count2.set(count2.get() + 1);
        assert_eq!(e.id(), e1);
    });

    assert_eq!(count.get(), 0);
    s.run();
    assert_eq!(count.get(), 1);
}

#[test]
fn system_builder_add_1_type() {
    let world = World::new();

    let e1 = world.entity().set(Position { x: 0, y: 0 }).id();
    world.entity().set(Velocity { x: 0, y: 0 });

    let count = Rc::new(Cell::new(0i32));
    let count2 = count.clone();

    let s = world
        .system::<()>()
        .with(&Position::id())
        .each_entity(move |e, _| {
            count2.set(count2.get() + 1);
            assert_eq!(e.id(), e1);
        });

    assert_eq!(count.get(), 0);
    s.run();
    assert_eq!(count.get(), 1);
}

#[test]
fn system_builder_add_2_types() {
    let world = World::new();

    let e1 = world
        .entity()
        .set(Position { x: 0, y: 0 })
        .set(Velocity { x: 0, y: 0 })
        .id();
    world.entity().set(Velocity { x: 0, y: 0 });

    let count = Rc::new(Cell::new(0i32));
    let count2 = count.clone();

    let s = world
        .system::<()>()
        .with(&Position::id())
        .with(&Velocity::id())
        .each_entity(move |e, _| {
            count2.set(count2.get() + 1);
            assert_eq!(e.id(), e1);
        });

    assert_eq!(count.get(), 0);
    s.run();
    assert_eq!(count.get(), 1);
}

#[test]
fn system_builder_add_1_type_w_1_type() {
    let world = World::new();

    let e1 = world
        .entity()
        .set(Position { x: 0, y: 0 })
        .set(Velocity { x: 0, y: 0 })
        .id();
    world.entity().set(Velocity { x: 0, y: 0 });

    let count = Rc::new(Cell::new(0i32));
    let count2 = count.clone();

    let s = world
        .system::<&Position>()
        .with(&Velocity::id())
        .each_entity(move |e, _p| {
            count2.set(count2.get() + 1);
            assert_eq!(e.id(), e1);
        });

    assert_eq!(count.get(), 0);
    s.run();
    assert_eq!(count.get(), 1);
}

#[test]
fn system_builder_add_2_types_w_1_type() {
    let world = World::new();

    let e1 = world
        .entity()
        .set(Position { x: 0, y: 0 })
        .set(Velocity { x: 0, y: 0 })
        .set(Mass { value: 0 })
        .id();
    world.entity().set(Velocity { x: 0, y: 0 });

    let count = Rc::new(Cell::new(0i32));
    let count2 = count.clone();

    let s = world
        .system::<&Position>()
        .with(&Velocity::id())
        .with(&Mass::id())
        .each_entity(move |e, _p| {
            count2.set(count2.get() + 1);
            assert_eq!(e.id(), e1);
        });

    assert_eq!(count.get(), 0);
    s.run();
    assert_eq!(count.get(), 1);
}

#[test]
fn system_builder_add_pair() {
    let world = World::new();

    let likes = world.entity();
    let bob = world.entity();
    let alice = world.entity();

    let e1 = world.entity().add((likes, bob)).id();
    world.entity().add((likes, alice));

    let count = Rc::new(Cell::new(0i32));
    let count2 = count.clone();

    let s = world
        .system::<()>()
        .with((likes, bob))
        .each_entity(move |e, _| {
            count2.set(count2.get() + 1);
            assert_eq!(e.id(), e1);
        });

    assert_eq!(count.get(), 0);
    s.run();
    assert_eq!(count.get(), 1);
}

#[test]
fn system_builder_add_not() {
    let world = World::new();

    let e1 = world.entity().set(Position { x: 0, y: 0 }).id();
    world
        .entity()
        .set(Position { x: 0, y: 0 })
        .set(Velocity { x: 0, y: 0 });

    let count = Rc::new(Cell::new(0i32));
    let count2 = count.clone();

    let s = world
        .system::<&Position>()
        .with(&Velocity::id())
        .set_oper(OperKind::Not)
        .each_entity(move |e, _p| {
            count2.set(count2.get() + 1);
            assert_eq!(e.id(), e1);
        });

    assert_eq!(count.get(), 0);
    s.run();
    assert_eq!(count.get(), 1);
}

#[test]
fn system_builder_add_or() {
    let world = World::new();

    let e1 = world.entity().set(Position { x: 0, y: 0 }).id();
    let e2 = world.entity().set(Velocity { x: 0, y: 0 }).id();
    world.entity().set(Mass { value: 0 });

    let count = Rc::new(Cell::new(0i32));
    let count2 = count.clone();

    let s = world
        .system::<()>()
        .with(&Position::id())
        .set_oper(OperKind::Or)
        .with(&Velocity::id())
        .each_entity(move |e, _| {
            count2.set(count2.get() + 1);
            assert!(e.id() == e1 || e.id() == e2);
        });

    assert_eq!(count.get(), 0);
    s.run();
    assert_eq!(count.get(), 2);
}

#[test]
fn system_builder_add_optional() {
    let world = World::new();

    let e1 = world.entity().set(Position { x: 0, y: 0 }).id();
    let e2 = world
        .entity()
        .set(Position { x: 0, y: 0 })
        .set(Velocity { x: 0, y: 0 })
        .id();
    world
        .entity()
        .set(Velocity { x: 0, y: 0 })
        .set(Mass { value: 0 });

    let count = Rc::new(Cell::new(0i32));
    let count2 = count.clone();

    let s = world
        .system::<()>()
        .with(&Position::id())
        .with(&Velocity::id())
        .set_oper(OperKind::Optional)
        .each_entity(move |e, _| {
            count2.set(count2.get() + 1);
            assert!(e.id() == e1 || e.id() == e2);
        });

    assert_eq!(count.get(), 0);
    s.run();
    assert_eq!(count.get(), 2);
}

#[test]
fn system_builder_ptr_type() {
    let world = World::new();

    let e1 = world.entity().set(Position { x: 0, y: 0 }).id();
    let e2 = world
        .entity()
        .set(Position { x: 0, y: 0 })
        .set(Velocity { x: 0, y: 0 })
        .id();
    world
        .entity()
        .set(Velocity { x: 0, y: 0 })
        .set(Mass { value: 0 });

    let count = Rc::new(Cell::new(0i32));
    let count2 = count.clone();

    // Velocity* in C++ = Option<&Velocity> (optional) in Rust
    let s = world
        .system::<(&Position, Option<&Velocity>)>()
        .each_entity(move |e, _| {
            count2.set(count2.get() + 1);
            assert!(e.id() == e1 || e.id() == e2);
        });

    assert_eq!(count.get(), 0);
    s.run();
    assert_eq!(count.get(), 2);
}

#[test]
fn system_builder_const_type() {
    let world = World::new();

    let e1 = world.entity().set(Position { x: 0, y: 0 }).id();
    world.entity().set(Velocity { x: 0, y: 0 });

    let count = Rc::new(Cell::new(0i32));
    let count2 = count.clone();

    let s = world.system::<&Position>().each_entity(move |e, _p| {
        count2.set(count2.get() + 1);
        assert_eq!(e.id(), e1);
    });

    assert_eq!(count.get(), 0);
    s.run();
    assert_eq!(count.get(), 1);
}

#[test]
fn system_builder_string_term() {
    let world = World::new();

    // Register so that short name is accessible
    world.component_named::<Position>("Position");

    let e1 = world.entity().set(Position { x: 0, y: 0 }).id();
    world.entity().set(Velocity { x: 0, y: 0 });

    let count = Rc::new(Cell::new(0i32));
    let count2 = count.clone();

    let s = world
        .system::<()>()
        .expr("Position")
        .each_entity(move |e, _| {
            count2.set(count2.get() + 1);
            assert_eq!(e.id(), e1);
        })
        .expect("valid system query expression");

    s.run();

    assert_eq!(count.get(), 1);
}

#[test]
fn system_builder_singleton_term() {
    #[derive(Component)]
    struct EntityComp {
        value: Entity,
    }

    #[derive(Component)]
    struct Singleton {
        value: i32,
    }

    let world = World::new();

    world
        .component::<Singleton>()
        .add_trait::<flecs::Singleton>();

    world.set(Singleton { value: 10 });

    let count = Rc::new(Cell::new(0i32));
    let count2 = count.clone();

    let s = world
        .system::<&EntityComp>()
        .with(&Singleton::id())
        .set_in()
        .run(move |mut it| {
            while it.next() {
                let e_field = it.field::<EntityComp>(0);
                let s_field = it.field::<Singleton>(1);
                assert!(!it.is_self(1));
                assert_eq!(s_field[0].value, 10);
                for i in it.iter() {
                    assert_eq!(it.entity_id(i), e_field[i].value);
                    count2.set(count2.get() + 1);
                }
            }
        });

    let e = world.entity();
    e.set(EntityComp { value: e.id() });
    let e = world.entity();
    e.set(EntityComp { value: e.id() });
    let e = world.entity();
    e.set(EntityComp { value: e.id() });

    s.run();

    assert_eq!(count.get(), 3);
}

#[test]
fn system_builder_10_terms() {
    let world = World::new();

    let count = Rc::new(Cell::new(0i32));
    let count2 = count.clone();

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
        .id();

    let s = world
        .system::<()>()
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
        .run(move |mut it| {
            while it.next() {
                assert_eq!(it.count(), 1);
                assert_eq!(it.get_entity(0usize).unwrap(), e);
                assert_eq!(it.field_count(), 10);
                count2.set(count2.get() + 1);
            }
        });

    s.run();

    assert_eq!(count.get(), 1);
}

#[test]
fn system_builder_16_terms() {
    let world = World::new();

    let count = Rc::new(Cell::new(0i32));
    let count2 = count.clone();

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
        .id();

    let s = world
        .system::<()>()
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
        .run(move |mut it| {
            while it.next() {
                assert_eq!(it.count(), 1);
                assert_eq!(it.get_entity(0usize).unwrap(), e);
                assert_eq!(it.field_count(), 16);
                count2.set(count2.get() + 1);
            }
        });

    s.run();

    assert_eq!(count.get(), 1);
}

#[test]
fn system_builder_name_arg() {
    let world = World::new();

    let s = world
        .system_named::<&Position>("MySystem")
        .term_at(0)
        .src()
        .name("MySystem")
        .run(|mut it| while it.next() {});

    assert!(s.has(Position::id()));
}

#[test]
fn system_builder_create_w_no_template_args() {
    let world = World::new();

    let e1 = world.entity().set(Position { x: 0, y: 0 }).id();

    let count = Rc::new(Cell::new(0i32));
    let count2 = count.clone();

    let s = world
        .system::<()>()
        .with(&Position::id())
        .each_entity(move |e, _| {
            count2.set(count2.get() + 1);
            assert_eq!(e.id(), e1);
        });

    assert_eq!(count.get(), 0);
    s.run();
    assert_eq!(count.get(), 1);
}

// deduce_terms_from_each_callback, deduce_optional_terms_from_each_callback,
// deduce_pair_term_from_each_callback, deduce_singleton_term_from_each_callback,
// deduce_singleton_and_component_terms_from_each_callback, with_terms_after_deduced_terms:
// not portable — C++ deduces query terms from lambda parameter types; Rust has no equivalent.

#[test]
fn system_builder_write_annotation() {
    #[derive(Component)]
    struct LocalTagA;

    #[derive(Component)]
    struct LocalTagB;

    let mut world = World::new();

    let e1 = world.entity().add(LocalTagA::id()).id();

    let a_count = Rc::new(Cell::new(0i32));
    let a_count2 = a_count.clone();
    let b_count = Rc::new(Cell::new(0i32));
    let b_count2 = b_count.clone();

    // System matches LocalTagA, declares write to LocalTagB (not a filter, just a dependency).
    // Matches C++: ecs.system<Tag0>().with<Tag1>().write().each(...)
    world
        .system::<()>()
        .with(&LocalTagA::id())
        .write(world.id_view_from(LocalTagB::id()))
        .each_entity(move |e, _| {
            a_count2.set(a_count2.get() + 1);
            assert_eq!(e.id(), e1);
            e.add(LocalTagB::id());
        });

    world
        .system::<()>()
        .with(&LocalTagB::id())
        .each_entity(move |e, _| {
            b_count2.set(b_count2.get() + 1);
            assert_eq!(e.id(), e1);
            assert!(e.has(LocalTagB::id()));
        });

    assert_eq!(a_count.get(), 0);
    assert_eq!(b_count.get(), 0);

    world.progress();

    assert_eq!(a_count.get(), 1);
    assert_eq!(b_count.get(), 1);
}

#[test]
fn system_builder_name_from_root() {
    let world = World::new();

    let sys = world
        .system_named::<()>("::ns::MySystem")
        .each_entity(|_e, _| {});

    assert_eq!(sys.name(), "MySystem");

    let ns = world.lookup("::ns");
    assert_eq!(ns, sys.parent().unwrap());
}
