#![allow(dead_code)]
use crate::common_test::*;

#[test]
fn multi_world_empty() {
    let _w1 = World::new();
    let _w2 = World::new();
}

#[test]
fn builtin_components() {
    let world = World::new();

    // Component ids must match the C-level builtin entity IDs.
    assert_eq!(
        world.component::<flecs::Component>().id(),
        Entity::from(unsafe { flecs_ecs::sys::FLECS_IDEcsComponentID_ })
    );
    assert_eq!(
        world.component::<flecs::Identifier>().id(),
        Entity::from(unsafe { flecs_ecs::sys::FLECS_IDEcsIdentifierID_ })
    );
    assert_eq!(
        world.component::<flecs::Poly>().id(),
        Entity::from(unsafe { flecs_ecs::sys::FLECS_IDEcsPolyID_ })
    );
    assert_eq!(
        world.component::<flecs::RateFilter>().id(),
        Entity::from(unsafe { flecs_ecs::sys::FLECS_IDEcsRateFilterID_ })
    );
    assert_eq!(
        world.component::<flecs::TickSource>().id(),
        Entity::from(unsafe { flecs_ecs::sys::FLECS_IDEcsTickSourceID_ })
    );
    assert_eq!(
        world.component::<flecs::System>().id(),
        Entity::from(unsafe { flecs_ecs::sys::EcsSystem })
    );
    assert_eq!(
        world.component::<flecs::Observer>().id(),
        Entity::from(unsafe { flecs_ecs::sys::EcsObserver })
    );
    assert_eq!(
        world.component::<flecs::Query>().id(),
        Entity::from(unsafe { flecs_ecs::sys::EcsQuery })
    );
    assert_eq!(
        world.component::<flecs::Name>().id(),
        Entity::from(unsafe { flecs_ecs::sys::EcsName })
    );
    assert_eq!(
        world.component::<flecs::Symbol>().id(),
        Entity::from(unsafe { flecs_ecs::sys::EcsSymbol })
    );
}

#[test]
fn multi_world_component() {
    let w1 = World::new();
    let w2 = World::new();

    let p_1 = w1.component::<Position>();
    let v_1 = w1.component::<Velocity>();
    let v_2 = w2.component::<Velocity>();
    let m_2 = w2.component::<Mass>();
    w2.component::<Mass>();

    assert_eq!(p_1.id(), v_2.id());
    assert_eq!(v_1.id(), m_2.id());

    let w1_e = w1
        .entity()
        .set(Position { x: 10, y: 20 })
        .set(Velocity { x: 1, y: 2 })
        .set(Mass { value: 100 });

    let w2_e = w2
        .entity()
        .set(Position { x: 10, y: 20 })
        .set(Velocity { x: 1, y: 2 })
        .set(Mass { value: 100 });

    w1_e.get::<&Position>(|p| {
        assert_eq!(p.x, 10);
        assert_eq!(p.y, 20);
    });

    w1_e.get::<&Velocity>(|v| {
        assert_eq!(v.x, 1);
        assert_eq!(v.y, 2);
    });

    w1_e.get::<&Mass>(|m| {
        assert_eq!(m.value, 100);
    });

    w2_e.get::<&Position>(|p| {
        assert_eq!(p.x, 10);
        assert_eq!(p.y, 20);
    });

    w2_e.get::<&Velocity>(|v| {
        assert_eq!(v.x, 1);
        assert_eq!(v.y, 2);
    });

    w2_e.get::<&Mass>(|m| {
        assert_eq!(m.value, 100);
    });
}

#[test]
fn multi_world_component_namespace() {
    let w1 = World::new();
    let c1 = w1.component::<a::Comp>();
    let id_1 = c1.id();
    drop(w1);

    let w2 = World::new();
    let c2 = w2.component::<a::Comp>();
    let id_2 = c2.id();

    assert_eq!(id_1, id_2);
}

#[test]
fn multi_world_module() {
    ns::reset_invoke_count();

    let mut world1 = World::new();
    world1.import::<ns::NamespaceModule>();

    let mut world2 = World::new();
    world2.import::<ns::NamespaceModule>();

    world1.entity().add(ns::FooComp::id());
    world2.entity().add(ns::FooComp::id());

    world1.progress();
    assert_eq!(ns::get_invoke_count(), 1);

    world2.progress();
    assert_eq!(ns::get_invoke_count(), 2);
}

#[test]
fn multi_world_recycled_component_different_generation() {
    use flecs_ecs::sys;
    let id_from_w1: Entity;
    {
        let ecs = World::new();
        for _ in 0..256 {
            unsafe { sys::ecs_new_low_id(ecs.ptr_mut()) };
        }
        ecs.entity().destruct();
        id_from_w1 = ecs.component::<Position>().id();
    }

    let ecs = World::new();
    for _ in 0..256 {
        unsafe { sys::ecs_new_low_id(ecs.ptr_mut()) };
    }
    ecs.entity().destruct();
    let id_from_w2 = ecs.component::<Position>().id();

    assert_eq!(id_from_w1, id_from_w2);
}

#[test]
fn type_id() {
    let world = World::new();

    let p = world.component::<Position>();
    assert_eq!(p.id(), world.component_id::<Position>());
}

#[test]
#[should_panic]
#[cfg_attr(not(debug_assertions), ignore)]
fn different_comp_same_name() {
    let world = World::new();
    let _guard = FlecsPanicAbortGuard::install();
    world.component::<Position>();
    world.component_named::<Velocity>("Position");
}

#[test]
fn reregister_namespace() {
    let world = World::new();

    world.component::<ns::FooComp>();
    let p_id_1 = world.component_id::<ns::FooComp>();

    world.component::<ns::FooComp>();
    let p_id_2 = world.component_id::<ns::FooComp>();

    assert_eq!(p_id_1, p_id_2);
}

#[test]
fn reregister_after_delete() {
    let world = World::new();

    let c = world.component::<Position>();
    assert_eq!(c.name(), "Position");
    assert_eq!(c.path(), Some("::Position".to_string()));
    assert_eq!(c.symbol(), "flecs::common_test::Position");

    c.destruct();

    assert!(!c.is_alive());

    let d = world.component::<Position>();
    assert!(!c.is_alive());
    assert!(d.is_alive());
    assert_eq!(d.name(), "Position");
    assert_eq!(d.path(), Some("::Position".to_string()));
    assert_eq!(d.symbol(), "flecs::common_test::Position");
}

#[test]
fn register_component_w_core_name() {
    #[derive(Component)]
    struct Module;

    let world = World::new();

    let c = world.component::<Module>();
    assert_ne!(c.id(), 0);
    assert_eq!(c.path(), Some("::Module".to_string()));
}

#[test]
fn register_short_template() {
    #[derive(Component)]
    struct Test;

    #[derive(Component)]
    struct Tmp<T: ComponentId> {
        pub v: i32,
        _marker: core::marker::PhantomData<T>,
    }

    let world = World::new();

    let c = world.component::<Tmp<Test>>();
    assert_ne!(c.id(), 0);
    assert_eq!(c.name(), "Tmp<Test>");

    c.try_get::<&EcsComponent>(|ptr| {
        assert_eq!(ptr.size, core::mem::size_of::<Tmp<Test>>() as i32);
        assert_eq!(ptr.alignment, core::mem::align_of::<Tmp<Test>>() as i32);
    });
}

#[test]
fn reimport() {
    #[derive(Component)]
    struct FooModule;

    impl Module for FooModule {
        fn module(world: &World) {
            world.module::<FooModule>("FooModule");
        }
    }

    let world = World::new();

    let m1 = world.import::<FooModule>();
    let m2 = world.import::<FooModule>();

    assert_eq!(m1, m2);
}

#[test]
fn reimport_module_new_world() {
    #[derive(Component)]
    struct FooModule;

    impl Module for FooModule {
        fn module(world: &World) {
            world.module::<FooModule>("FooModule");
        }
    }

    let e1_id;
    {
        let world = World::new();
        e1_id = world.import::<FooModule>().id();
    }

    {
        let world = World::new();
        let e2_id = world.import::<FooModule>().id();
        assert_eq!(e1_id, e2_id);
    }
}

#[test]
fn reimport_namespaced_module() {
    ns::reset_import_count();
    assert_eq!(ns::get_import_count(), 0);

    let world = World::new();

    world.import::<ns::NamespaceModule>();
    assert_eq!(ns::get_import_count(), 1);

    world.import::<ns::NamespaceModule>();
    assert_eq!(ns::get_import_count(), 1);
}

#[test]
fn implicit_register_w_new_world() {
    {
        let world = World::new();

        let e = world.entity().set(Position { x: 10, y: 20 });
        assert!(e.has(Position::id()));
        e.get::<&Position>(|p| {
            assert_eq!(p.x, 10);
            assert_eq!(p.y, 20);
        });
    }

    {
        let world = World::new();

        let e = world.entity().set(Position { x: 10, y: 20 });
        assert!(e.has(Position::id()));
        e.get::<&Position>(|p| {
            assert_eq!(p.x, 10);
            assert_eq!(p.y, 20);
        });
    }
}

#[test]
fn count() {
    let world = World::new();

    assert_eq!(world.count(Position::id()), 0);

    world.entity().add(Position::id());
    world.entity().add(Position::id());
    world.entity().add(Position::id());
    world.entity().add(Position::id()).add(Mass::id());
    world.entity().add(Position::id()).add(Mass::id());
    world.entity().add(Position::id()).add(Velocity::id());

    assert_eq!(world.count(Position::id()), 6);
}

#[test]
fn count_id() {
    let world = World::new();

    let ent = world.entity();

    assert_eq!(world.count(ent.id()), 0);

    world.entity().add(ent.id());
    world.entity().add(ent.id());
    world.entity().add(ent.id());
    world.entity().add(ent.id()).add(Mass::id());
    world.entity().add(ent.id()).add(Mass::id());
    world.entity().add(ent.id()).add(Velocity::id());

    assert_eq!(world.count(ent.id()), 6);
}

#[test]
fn count_pair() {
    let world = World::new();

    let parent = world.entity();

    assert_eq!(world.count((flecs::ChildOf::ID, parent.id())), 0);

    world.entity().add((flecs::ChildOf::ID, parent.id()));
    world.entity().add((flecs::ChildOf::ID, parent.id()));
    world.entity().add((flecs::ChildOf::ID, parent.id()));
    world
        .entity()
        .add((flecs::ChildOf::ID, parent.id()))
        .add(Mass::id());
    world
        .entity()
        .add((flecs::ChildOf::ID, parent.id()))
        .add(Mass::id());
    world
        .entity()
        .add((flecs::ChildOf::ID, parent.id()))
        .add(Velocity::id());

    assert_eq!(world.count((flecs::ChildOf::ID, parent.id())), 6);
}

#[test]
fn count_pair_type_id() {
    let world = World::new();

    let target = world.entity();

    assert_eq!(world.count((Rel::id(), target.id())), 0);

    world.entity().add((Rel::id(), target.id()));
    world.entity().add((Rel::id(), target.id()));
    world.entity().add((Rel::id(), target.id()));
    world.entity().add((Rel::id(), target.id())).add(Mass::id());
    world.entity().add((Rel::id(), target.id())).add(Mass::id());
    world
        .entity()
        .add((Rel::id(), target.id()))
        .add(Velocity::id());

    assert_eq!(world.count((Rel::id(), target.id())), 6);
}

#[test]
fn count_pair_id() {
    let world = World::new();

    let rel = world.entity();
    let target = world.entity();

    assert_eq!(world.count((rel.id(), target.id())), 0);

    world.entity().add((rel.id(), target.id()));
    world.entity().add((rel.id(), target.id()));
    world.entity().add((rel.id(), target.id()));
    world.entity().add((rel.id(), target.id())).add(Mass::id());
    world.entity().add((rel.id(), target.id())).add(Mass::id());
    world
        .entity()
        .add((rel.id(), target.id()))
        .add(Velocity::id());

    assert_eq!(world.count((rel.id(), target.id())), 6);
}

#[test]
fn staged_count() {
    let world = World::new();

    let stage = world.stage(0);

    world.readonly_begin(false);
    assert_eq!(stage.count(Position::id()), 0);
    world.readonly_end();

    world.readonly_begin(false);

    stage.entity().add(Position::id());
    stage.entity().add(Position::id());
    stage.entity().add(Position::id());
    stage.entity().add(Position::id()).add(Mass::id());
    stage.entity().add(Position::id()).add(Mass::id());
    stage.entity().add(Position::id()).add(Velocity::id());

    assert_eq!(stage.count(Position::id()), 0);

    world.readonly_end();

    assert_eq!(stage.count(Position::id()), 6);
}

#[test]
fn async_stage_add() {
    let world = World::new();
    world.component::<Position>();

    let e = world.entity();
    let async_stage = world.create_async_stage();

    e.mut_current_stage(async_stage).add(Position::id());

    assert!(!e.has(Position::id()));

    async_stage.merge();

    assert!(e.has(Position::id()));
}

#[test]
fn async_stage_owning_handle() {
    let world = World::new();
    world.component::<Position>();

    let e = world.entity();
    let stage = world.async_stage();

    e.mut_current_stage(stage.stage()).add(Position::id());

    assert!(!e.has(Position::id()));

    stage.merge();

    assert!(e.has(Position::id()));

    drop(stage);

    assert!(e.has(Position::id()));

    let e2 = world.entity();
    assert_ne!(e2.id(), 0u64);
}

#[test]
fn reset_cached_component_ids_reregisters() {
    let world = World::new();

    let id_before = world.component::<Position>().id();
    world.reset_cached_component_ids();
    let id_after = world.component::<Position>().id();

    assert_eq!(id_before, id_after);
}

#[test]
fn with_tag() {
    let world = World::new();

    let tag = world.entity();

    world.with(tag.id(), || {
        let e1 = world.entity();
        e1.set(SelfRef { value: e1.id() });
        let e2 = world.entity();
        e2.set(SelfRef { value: e2.id() });
        let e3 = world.entity();
        e3.set(SelfRef { value: e3.id() });
    });

    let self_comp = world.component::<SelfRef>();
    assert!(!self_comp.has(tag.id()));

    let mut count = 0;
    world
        .query::<()>()
        .with(tag.id())
        .build()
        .each_entity(|e, _| {
            assert!(e.has(tag.id()));
            e.get::<&SelfRef>(|s| {
                assert_eq!(s.value, e.id());
            });
            count += 1;
        });
    assert_eq!(count, 3);
}

#[test]
fn with_tag_type() {
    let world = World::new();

    world.with(Tag::id(), || {
        let e1 = world.entity();
        e1.set(SelfRef { value: e1.id() });
        let e2 = world.entity();
        e2.set(SelfRef { value: e2.id() });
        let e3 = world.entity();
        e3.set(SelfRef { value: e3.id() });
    });

    let self_comp = world.component::<SelfRef>();
    assert!(!self_comp.has(Tag::id()));

    let mut count = 0;
    world
        .query::<()>()
        .with(Tag::id())
        .build()
        .each_entity(|e, _| {
            assert!(e.has(Tag::id()));
            e.get::<&SelfRef>(|s| {
                assert_eq!(s.value, e.id());
            });
            count += 1;
        });
    assert_eq!(count, 3);
}

#[test]
fn with_relation() {
    let world = World::new();

    let likes = world.entity();
    let bob = world.entity();

    world.with((likes.id(), bob.id()), || {
        let e1 = world.entity();
        e1.set(SelfRef { value: e1.id() });
        let e2 = world.entity();
        e2.set(SelfRef { value: e2.id() });
        let e3 = world.entity();
        e3.set(SelfRef { value: e3.id() });
    });

    let self_comp = world.component::<SelfRef>();
    assert!(!self_comp.has((likes.id(), bob.id())));

    let mut count = 0;
    world
        .query::<()>()
        .with((likes.id(), bob.id()))
        .build()
        .each_entity(|e, _| {
            assert!(e.has((likes.id(), bob.id())));
            e.get::<&SelfRef>(|s| {
                assert_eq!(s.value, e.id());
            });
            count += 1;
        });
    assert_eq!(count, 3);
}

#[test]
fn with_relation_type() {
    let world = World::new();

    let bob = world.entity();

    world.with((Likes::id(), bob.id()), || {
        let e1 = world.entity();
        e1.set(SelfRef { value: e1.id() });
        let e2 = world.entity();
        e2.set(SelfRef { value: e2.id() });
        let e3 = world.entity();
        e3.set(SelfRef { value: e3.id() });
    });

    let self_comp = world.component::<SelfRef>();
    assert!(!self_comp.has((Likes::id(), bob.id())));

    let mut count = 0;
    world
        .query::<()>()
        .with((Likes::id(), bob.id()))
        .build()
        .each_entity(|e, _| {
            assert!(e.has((Likes::id(), bob.id())));
            e.get::<&SelfRef>(|s| {
                assert_eq!(s.value, e.id());
            });
            count += 1;
        });
    assert_eq!(count, 3);
}

#[test]
fn with_relation_object_type() {
    let world = World::new();

    world.with((Likes::id(), Bob::id()), || {
        let e1 = world.entity();
        e1.set(SelfRef { value: e1.id() });
        let e2 = world.entity();
        e2.set(SelfRef { value: e2.id() });
        let e3 = world.entity();
        e3.set(SelfRef { value: e3.id() });
    });

    let self_comp = world.component::<SelfRef>();
    assert!(!self_comp.has((Likes::id(), Bob::id())));

    let mut count = 0;
    world
        .query::<()>()
        .with((Likes::id(), Bob::id()))
        .build()
        .each_entity(|e, _| {
            assert!(e.has((Likes::id(), Bob::id())));
            e.get::<&SelfRef>(|s| {
                assert_eq!(s.value, e.id());
            });
            count += 1;
        });
    assert_eq!(count, 3);
}

#[test]
fn with_scope() {
    let world = World::new();

    let parent = world.entity_named("P");

    world.scope(parent.id(), |world| {
        let e1 = world.entity_named("C1");
        e1.set(SelfRef { value: e1.id() });
        let e2 = world.entity_named("C2");
        e2.set(SelfRef { value: e2.id() });
        let e3 = world.entity_named("C3");
        e3.set(SelfRef { value: e3.id() });

        assert_eq!(world.lookup("C1"), e1.id());
        assert_eq!(world.lookup("C2"), e2.id());
        assert_eq!(world.lookup("C3"), e3.id());

        assert_eq!(parent.lookup("C1"), e1.id());
        assert_eq!(parent.lookup("C2"), e2.id());
        assert_eq!(parent.lookup("C3"), e3.id());

        assert_eq!(world.lookup("::P::C1"), e1.id());
        assert_eq!(world.lookup("::P::C2"), e2.id());
        assert_eq!(world.lookup("::P::C3"), e3.id());
    });

    assert_ne!(parent.lookup("C1"), 0u64);
    assert_ne!(parent.lookup("C2"), 0u64);
    assert_ne!(parent.lookup("C3"), 0u64);

    assert_eq!(world.lookup("P::C1"), parent.lookup("C1"));
    assert_eq!(world.lookup("P::C2"), parent.lookup("C2"));
    assert_eq!(world.lookup("P::C3"), parent.lookup("C3"));

    let self_comp = world.component::<SelfRef>();
    assert!(!self_comp.has((flecs::ChildOf::ID, parent.id())));

    let mut count = 0;
    world
        .query::<()>()
        .with((flecs::ChildOf::ID, parent.id()))
        .build()
        .each_entity(|e, _| {
            assert!(e.has((flecs::ChildOf::ID, parent.id())));
            e.get::<&SelfRef>(|s| {
                assert_eq!(s.value, e.id());
            });
            count += 1;
        });
    assert_eq!(count, 3);
}

#[test]
fn with_scope_type() {
    #[derive(Component)]
    struct ParentScope;

    let world = World::new();

    world.scope(ParentScope::id(), |_| {
        world.entity_named("Child");
    });

    let parent = world.lookup("ParentScope");
    assert_ne!(parent.id(), 0u64);

    let child = world.lookup("ParentScope::Child");
    assert_ne!(child.id(), 0u64);
    assert_eq!(child.id(), parent.lookup("Child").id());
}

#[test]
fn with_scope_type_staged() {
    #[derive(Component)]
    struct ParentScope;

    let world = World::new();
    let stage = world.stage(0);

    world.readonly_begin(false);
    stage.set_scope(ParentScope::id());
    let e = stage.entity_named("Child");
    stage.set_scope(0);
    world.readonly_end();

    assert!(e.has((flecs::ChildOf::ID, ParentScope::id())));

    let parent = world.lookup("ParentScope");
    assert_ne!(parent.id(), 0u64);

    let child = world.lookup("ParentScope::Child");
    assert_ne!(child.id(), 0u64);
    assert_eq!(child.id(), parent.lookup("Child").id());
}

#[test]
fn with_scope_no_lambda() {
    let world = World::new();

    let parent = world.entity_named("Parent");
    let child = {
        let _scope = world.set_scope(parent.id());
        let c = world.entity_named("Child");
        world.set_scope(0);
        c
    };
    assert!(child.has((flecs::ChildOf::ID, parent.id())));
    assert!(world.get_scope().is_none());
}

#[test]
fn with_scope_type_no_lambda() {
    #[derive(Component)]
    struct ParentScope;

    let world = World::new();

    world.set_scope(ParentScope::id());
    let child = world.entity_named("Child");
    world.set_scope(0);

    assert!(child.has((flecs::ChildOf::ID, ParentScope::id())));
    assert!(world.get_scope().is_none());
}

#[test]
fn with_tag_nested() {
    let world = World::new();

    let tier1 = world.entity();

    world.with(tier1.id(), || {
        let tier2 = world.entity_named("Tier2");
        tier2.with(|| {
            world.entity_named("Tier3");
        });
    });

    let tier2 = world.lookup("Tier2");
    assert_ne!(tier2.id(), 0u64);

    let tier3 = world.lookup("Tier3");
    assert_ne!(tier3.id(), 0u64);

    assert!(tier2.has(tier1.id()));
    assert!(tier3.has(tier2.id()));
}

#[test]
fn with_scope_nested() {
    let world = World::new();

    let parent = world.entity_named("P");

    world.scope(parent.id(), |world| {
        let child = world.entity_named("C");
        child.scope(|world| {
            let gchild = world.entity_named("GC");
            assert_eq!(world.lookup("GC"), gchild.id());
            assert_eq!(world.lookup("::P::C::GC"), gchild.id());
        });

        assert_eq!(world.lookup("C"), child.id());
        assert_eq!(world.lookup("::P::C"), child.id());
        assert_ne!(world.lookup("::P::C::GC"), 0u64);
    });

    assert!(world.try_lookup("C").is_none());
    assert!(world.try_lookup("GC").is_none());
    assert!(world.try_lookup("C::GC").is_none());

    let child = world.lookup("P::C");
    assert_ne!(child.id(), 0u64);
    assert!(child.has((flecs::ChildOf::ID, parent.id())));

    let gchild = world.lookup("P::C::GC");
    assert_ne!(gchild.id(), 0u64);
    assert!(gchild.has((flecs::ChildOf::ID, child.id())));
}

#[test]
fn recursive_lookup() {
    let world = World::new();

    let a = world.entity_named("A");
    let b = world.entity_named("B");

    let p = world.entity_named("P");
    p.scope(|world| {
        let ca = world.entity_named("A");
        assert_ne!(ca.id(), a.id());

        assert_eq!(world.lookup("A"), ca.id());
        assert_eq!(world.lookup("P::A"), ca.id());
        assert_eq!(world.lookup("::P::A"), ca.id());
        assert_eq!(world.lookup("::A"), a.id());

        assert_eq!(world.lookup("B"), b.id());
        assert_eq!(world.lookup("::B"), b.id());
    });
}

#[test]
fn lookup_is_recursive_try_lookup_is_scope_only() {
    let world = World::new();

    let root = world.entity_named("LookupRoot");

    let parent = world.entity_named("LookupParent");
    parent.scope(|world| {
        assert_eq!(world.lookup("LookupRoot"), root.id());
        assert_eq!(world.lookup_recursive("LookupRoot"), root.id());
        assert!(world.try_lookup("LookupRoot").is_none());
        assert!(world.try_lookup_recursive("LookupRoot").is_some());
    });
}

#[test]
fn type_w_tag_name() {
    let world = World::new();

    let c = world.component::<Tag>();
    assert_ne!(c.id(), 0);
    assert_eq!(c.path(), Some("::Tag".to_string()));
    assert_ne!(c.id().0, flecs::PairIsTag::ID);
}

#[test]
fn entity_w_tag_name() {
    let world = World::new();

    let c = world.entity_named("Tag");
    assert_ne!(c.id(), 0);
    assert_eq!(c.path(), Some("::Tag".to_string()));
    assert_ne!(c.id().0, flecs::PairIsTag::ID);
}

#[test]
fn template_component_name() {
    #[derive(Component)]
    struct TemplateType<T: ComponentId>(core::marker::PhantomData<T>);

    let world = World::new();

    let c = world.component::<TemplateType<Position>>();
    assert_ne!(c.id(), 0);
    assert_eq!(c.name(), "TemplateType<Position>");
    assert_eq!(c.path(), Some("::TemplateType<Position>".to_string()));
}

#[test]
fn template_component_w_namespace_name() {
    #[derive(Component)]
    struct TemplateWrapper<T: ComponentId>(core::marker::PhantomData<T>);

    let world = World::new();
    let c = world.component::<TemplateWrapper<Position>>();
    assert_ne!(c.id(), 0);
    assert_eq!(c.name(), "TemplateWrapper<Position>");
    assert_eq!(c.path(), Some("::TemplateWrapper<Position>".to_string()));
}

#[test]
fn template_component_w_namespace_name_and_namespaced_arg() {
    #[derive(Component)]
    struct Foo;

    #[derive(Component)]
    struct TemplateWrapper<T: ComponentId>(core::marker::PhantomData<T>);

    let world = World::new();
    let c = world.component::<TemplateWrapper<Foo>>();
    assert_ne!(c.id(), 0);
    assert_eq!(c.name(), "TemplateWrapper<Foo>");
    assert_eq!(c.path(), Some("::TemplateWrapper<Foo>".to_string()));
}

#[test]
fn template_component_w_same_namespace_name() {
    #[derive(Component)]
    struct Foo<T: ComponentId>(core::marker::PhantomData<T>);

    let world = World::new();
    let c = world.component::<Foo<Position>>();
    assert_ne!(c.id(), 0);
    assert_eq!(c.name(), "Foo<Position>");
    assert_eq!(c.path(), Some("::Foo<Position>".to_string()));
}

#[test]
fn template_component_w_same_namespace_name_and_namespaced_arg() {
    #[derive(Component)]
    struct Bar;

    #[derive(Component)]
    struct Foo<T: ComponentId>(core::marker::PhantomData<T>);

    let world = World::new();
    let c = world.component::<Foo<Bar>>();
    assert_ne!(c.id(), 0);
    assert_eq!(c.name(), "Foo<Bar>");
    assert_eq!(c.path(), Some("::Foo<Bar>".to_string()));
}

#[test]
fn template_component_from_module_2_args() {
    #[derive(Component)]
    struct Foo;

    #[derive(Component)]
    struct Bar;

    #[derive(Component)]
    struct TypeWithArgs<T: ComponentId, U: ComponentId>(
        core::marker::PhantomData<T>,
        core::marker::PhantomData<U>,
    );

    #[derive(Component)]
    struct ModuleWTemplateComponent;

    impl Module for ModuleWTemplateComponent {
        fn module(world: &World) {
            world.module::<ModuleWTemplateComponent>("ModuleWTemplateComponent");
            world.component::<Foo>();
            world.component::<Bar>();
            world.component::<TypeWithArgs<Foo, Bar>>();
        }
    }

    let world = World::new();

    let m = world.import::<ModuleWTemplateComponent>();
    assert_eq!(m, world.lookup("ModuleWTemplateComponent"));

    let tid = world.component_id::<TypeWithArgs<Foo, Bar>>();
    assert_ne!(tid, 0u64);

    let mid = m
        .try_lookup("TypeWithArgs<Foo,Bar>")
        .or_else(|| m.try_lookup("TypeWithArgs<Foo, Bar>"));
    assert!(mid.is_some());
    assert_eq!(mid.unwrap().id(), tid);
}

#[test]
fn entity_as_tag() {
    let world = World::new();

    let e = world.component_id::<Tag>();
    assert_ne!(e, 0u64);

    let t = world.component::<Tag>();
    assert_ne!(t.id(), 0u64);
    assert_eq!(e, t.id());

    let e2 = world.entity().add(Tag::id());

    assert!(e2.has(Tag::id()));
    assert!(e2.has(e));

    assert_eq!(t.name(), "Tag");
}

#[test]
fn entity_w_name_as_tag() {
    let world = World::new();

    let e = world.component_named::<Tag>("Foo");
    assert_ne!(e.id(), 0u64);

    let t = world.component::<Tag>();
    assert_ne!(t.id(), 0u64);
    assert_eq!(e.id(), t.id());

    let e2 = world.entity().add(Tag::id());

    assert!(e2.has(Tag::id()));
    assert!(e2.has(e.id()));

    assert_eq!(e.name(), "Foo");
}

#[test]
fn entity_as_component() {
    let world = World::new();

    let e = world.component_id::<Position>();
    assert_ne!(e, 0u64);

    let t = world.component::<Position>();
    assert_ne!(t.id(), 0u64);
    assert_eq!(e, t.id());

    let e2 = world.entity().set(Position { x: 10, y: 20 });

    assert!(e2.has(Position::id()));
    assert!(e2.has(e));

    assert_eq!(t.name(), "Position");
}

#[test]
fn entity_w_name_as_component() {
    let world = World::new();

    let e = world.component_named::<Position>("Foo");
    assert_ne!(e.id(), 0u64);

    let t = world.component::<Position>();
    assert_ne!(t.id(), 0u64);
    assert_eq!(e.id(), t.id());

    let e2 = world.entity().set(Position { x: 10, y: 20 });

    assert!(e2.has(Position::id()));
    assert!(e2.has(e.id()));

    assert_eq!(e.name(), "Foo");
}

#[test]
fn entity_as_component_2_worlds() {
    let ecs1 = World::new();
    let e1 = ecs1.component::<Position>();
    assert_ne!(e1.id(), 0u64);

    let ecs2 = World::new();
    let e2 = ecs2.component::<Position>();
    assert_ne!(e2.id(), 0u64);

    assert_eq!(e1.id(), e2.id());
    assert_eq!(e1.id(), ecs1.component::<Position>().id());
    assert_eq!(e2.id(), ecs2.component::<Position>().id());
}

#[test]
fn entity_as_namespaced_component_2_worlds() {
    mod parent_ns {
        use flecs_ecs::prelude::*;
        #[derive(Component, Default)]
        pub struct Parent;
        #[derive(Component, Default)]
        pub struct Child;
    }
    use parent_ns::*;

    let ecs1 = World::new();
    let e1 = ecs1.component::<Parent>();
    assert_ne!(e1.id(), 0u64);

    let e1_1 = ecs1.component::<Child>();
    assert_ne!(e1_1.id(), 0u64);

    let ecs2 = World::new();
    let e2 = ecs2.component::<Parent>();
    assert_ne!(e2.id(), 0u64);

    let e2_1 = ecs2.component::<Child>();
    assert_ne!(e2_1.id(), 0u64);

    assert_eq!(e1.id(), e2.id());
    assert_eq!(e1.id(), ecs1.component::<Parent>().id());
    assert_eq!(e2.id(), ecs2.component::<Parent>().id());

    assert_eq!(e1_1.id(), e2_1.id());
    assert_eq!(e1_1.id(), ecs1.component::<Child>().id());
    assert_eq!(e2_1.id(), ecs2.component::<Child>().id());
}

#[test]
fn entity_as_component_2_worlds_implicit_namespaced() {
    mod parent_ns2 {
        use flecs_ecs::prelude::*;
        #[derive(Component, Default)]
        pub struct Parent;
        #[derive(Component, Default)]
        pub struct Child;
    }
    use parent_ns2::*;

    let ecs1 = World::new();
    let e1 = ecs1.component::<Parent>();
    assert_ne!(e1.id(), 0u64);
    ecs1.entity().add(Child::id());

    let ecs2 = World::new();
    let e2 = ecs2.component::<Parent>();
    assert_ne!(e2.id(), 0u64);
    ecs2.entity().add(Child::id());

    assert_eq!(e1.id(), e2.id());
    assert_eq!(e1.id(), ecs1.component::<Parent>().id());
    assert_eq!(e2.id(), ecs2.component::<Parent>().id());

    assert_eq!(
        ecs1.component::<Child>().id(),
        ecs2.component::<Child>().id()
    );
}

#[test]
fn delete_with_id() {
    let world = World::new();

    let tag = world.entity();
    let e1 = world.entity().add(tag.id());
    let e2 = world.entity().add(tag.id());
    let e3 = world.entity().add(tag.id());

    world.delete_entities_with(tag.id());

    assert!(!e1.is_alive());
    assert!(!e2.is_alive());
    assert!(!e3.is_alive());
}

#[test]
fn delete_with_type() {
    let world = World::new();

    let e1 = world.entity().add(Tag::id());
    let e2 = world.entity().add(Tag::id());
    let e3 = world.entity().add(Tag::id());

    world.delete_entities_with(Tag::id());

    assert!(!e1.is_alive());
    assert!(!e2.is_alive());
    assert!(!e3.is_alive());
}

#[test]
fn delete_with_pair() {
    let world = World::new();

    let rel = world.entity();
    let obj = world.entity();

    let e1 = world.entity().add((rel.id(), obj.id()));
    let e2 = world.entity().add((rel.id(), obj.id()));
    let e3 = world.entity().add((rel.id(), obj.id()));

    world.delete_entities_with((rel.id(), obj.id()));

    assert!(!e1.is_alive());
    assert!(!e2.is_alive());
    assert!(!e3.is_alive());
}

#[test]
fn delete_with_pair_type() {
    let world = World::new();

    let e1 = world.entity().add((Rel::id(), Obj::id()));
    let e2 = world.entity().add((Rel::id(), Obj::id()));
    let e3 = world.entity().add((Rel::id(), Obj::id()));

    world.delete_entities_with((Rel::id(), Obj::id()));

    assert!(!e1.is_alive());
    assert!(!e2.is_alive());
    assert!(!e3.is_alive());
}

#[test]
fn delete_with_implicit() {
    // No entities have Tag — must not crash
    let world = World::new();
    world.delete_entities_with(Tag::id());
}

#[test]
fn delete_with_pair_implicit() {
    // No entities have (Rel, Obj) — must not crash
    let world = World::new();
    world.delete_entities_with((Rel::id(), Obj::id()));
}

#[test]
fn remove_all_id() {
    let world = World::new();

    let tag_a = world.entity();
    let tag_b = world.entity();
    let e1 = world.entity().add(tag_a.id());
    let e2 = world.entity().add(tag_a.id());
    let e3 = world.entity().add(tag_a.id()).add(tag_b.id());

    world.remove_all(tag_a.id());

    assert!(e1.is_alive());
    assert!(e2.is_alive());
    assert!(e3.is_alive());

    assert!(!e1.has(tag_a.id()));
    assert!(!e2.has(tag_a.id()));
    assert!(!e3.has(tag_a.id()));
    assert!(e3.has(tag_b.id()));
}

#[test]
fn remove_all_type() {
    let world = World::new();

    let e1 = world.entity().add(Position::id());
    let e2 = world.entity().add(Position::id());
    let e3 = world.entity().add(Position::id()).add(Velocity::id());

    world.remove_all(Position::id());

    assert!(e1.is_alive());
    assert!(e2.is_alive());
    assert!(e3.is_alive());

    assert!(!e1.has(Position::id()));
    assert!(!e2.has(Position::id()));
    assert!(!e3.has(Position::id()));
    assert!(e3.has(Velocity::id()));
}

#[test]
fn remove_all_pair() {
    let world = World::new();

    let rel = world.entity();
    let obj_a = world.entity();
    let obj_b = world.entity();

    let e1 = world.entity().add((rel.id(), obj_a.id()));
    let e2 = world.entity().add((rel.id(), obj_a.id()));
    let e3 = world
        .entity()
        .add((rel.id(), obj_a.id()))
        .add((rel.id(), obj_b.id()));

    world.remove_all((rel.id(), obj_a.id()));

    assert!(e1.is_alive());
    assert!(e2.is_alive());
    assert!(e3.is_alive());

    assert!(!e1.has((rel.id(), obj_a.id())));
    assert!(!e2.has((rel.id(), obj_a.id())));
    assert!(!e3.has((rel.id(), obj_a.id())));
    assert!(e3.has((rel.id(), obj_b.id())));
}

#[test]
fn remove_all_pair_type() {
    #[derive(Component, Default)]
    struct Rel;
    #[derive(Component, Default)]
    struct ObjA;
    #[derive(Component, Default)]
    struct ObjB;

    let world = World::new();

    let e1 = world.entity().add((Rel::id(), ObjA::id()));
    let e2 = world.entity().add((Rel::id(), ObjA::id()));
    let e3 = world
        .entity()
        .add((Rel::id(), ObjA::id()))
        .add((Rel::id(), ObjB::id()));

    world.remove_all((Rel::id(), ObjA::id()));

    assert!(e1.is_alive());
    assert!(e2.is_alive());
    assert!(e3.is_alive());

    assert!(!e1.has((Rel::id(), ObjA::id())));
    assert!(!e2.has((Rel::id(), ObjA::id())));
    assert!(!e3.has((Rel::id(), ObjA::id())));

    assert!(!e1.has((Rel::id(), ObjB::id())));
    assert!(!e2.has((Rel::id(), ObjB::id())));
    assert!(e3.has((Rel::id(), ObjB::id())));
}

#[test]
fn remove_all_implicit() {
    let world = World::new();
    world.remove_all(Tag::id());
}

#[test]
fn remove_all_pair_implicit() {
    #[derive(Component, Default)]
    struct Rel;
    #[derive(Component, Default)]
    struct Obj;

    let world = World::new();
    world.remove_all((Rel::id(), Obj::id()));
}

#[test]
fn get_scope() {
    let world = World::new();

    let e = world.entity_named("scope");
    world.set_scope(e.id());

    let s = world.get_scope();
    assert!(s.is_some());
    if let Some(scope_entity) = s {
        assert_eq!(scope_entity.id(), e.id());
        assert_eq!(scope_entity.name(), "scope");
    }
}

#[test]
fn get_scope_type() {
    #[derive(Component, Default)]
    struct ParentScope;

    let world = World::new();

    world.set_scope(ParentScope::id());

    let s = world.get_scope();
    assert!(s.is_some());
    let scope_entity = s.unwrap();
    assert_eq!(scope_entity.id(), world.component_id::<ParentScope>());
    assert_eq!(scope_entity.name(), "ParentScope");
}

#[test]
fn register_namespace_after_component() {
    mod outer_mod {
        use flecs_ecs::prelude::*;
        #[derive(Component, Default)]
        pub struct Outer;
        #[derive(Component, Default)]
        pub struct Inner;
    }
    use outer_mod::*;

    let world = World::new();
    // Register Inner before Outer (like CPP's Outer::Inner before Outer)
    let inn = world.component::<Inner>();
    let out = world.component::<Outer>();

    assert_ne!(inn.id(), 0u64);
    assert_ne!(out.id(), 0u64);
    assert_eq!(inn.path(), Some("::Inner".to_string()));
    assert_eq!(out.path(), Some("::Outer".to_string()));
}

#[test]
fn is_alive() {
    let world = World::new();

    let e = world.entity();

    assert!(world.is_alive(e.id()));
    assert!(!world.is_alive(1000u64));

    e.destruct();

    assert!(!world.is_alive(e.id()));
}

#[test]
fn is_valid() {
    let world = World::new();

    let e = world.entity();

    assert!(world.is_valid(e.id()));
    assert!(!world.is_valid(1000u64));
    assert!(!world.is_valid(0u64));

    e.destruct();

    assert!(!world.is_valid(e.id()));

    world.make_alive(1000u64);

    assert!(world.is_valid(1000u64));
}

#[test]
fn exists() {
    let world = World::new();

    let e = world.entity();

    assert!(world.exists(e.id()));
    assert!(!world.exists(1000));
}

#[test]
fn get_alive() {
    let world = World::new();

    let e1 = world.entity();
    let e_no_gen = unsafe { flecs_ecs::sys::ecs_strip_generation(*e1.id()) };
    assert_eq!(*e1.id(), e_no_gen);
    e1.destruct();

    let e2 = world.entity();
    assert_ne!(*e1.id(), *e2.id());
    assert_eq!(e_no_gen, unsafe {
        flecs_ecs::sys::ecs_strip_generation(*e2.id())
    });

    let alive = world.get_alive(e_no_gen);
    assert_eq!(alive, e2.id());
}

#[test]
fn make_alive() {
    let world = World::new();

    let e1 = world.entity();
    e1.destruct();
    assert!(!e1.is_alive());

    let e2 = world.entity();
    assert_ne!(e1.id(), e2.id());
    assert_eq!(*e1.id(), unsafe {
        flecs_ecs::sys::ecs_strip_generation(*e2.id())
    });
    e2.destruct();
    assert!(!e2.is_alive());

    let e3 = world.make_alive(e2.id());
    assert_eq!(e2.id(), e3);
    assert!(world.is_alive(e3));
}

#[test]
fn reset_all() {}

#[test]
fn get_tick() {
    let mut world = World::new();

    assert_eq!(world.info().frame_count_total, 0);

    world.progress_time(1.0);
    assert_eq!(world.info().frame_count_total, 1);

    world.progress_time(1.0);
    assert_eq!(world.info().frame_count_total, 2);
}

#[test]
fn register_from_scope() {
    #[derive(Component, Default)]
    struct Scope;
    #[derive(Component, Default)]
    struct FromScope;

    let world = World::new();

    world.set_scope(Scope::id());
    let c = world.component::<FromScope>();
    world.set_scope(0u64);

    assert!(c.has((flecs::ChildOf::ID, world.component_id::<Scope>())));
}

#[test]
fn register_nested_from_scope() {
    #[derive(Component, Default)]
    struct Scope;
    mod nested {
        use flecs_ecs::prelude::*;
        #[derive(Component, Default)]
        pub struct FromScope;
    }

    let world = World::new();

    world.set_scope(Scope::id());
    let c = world.component::<nested::FromScope>();
    world.set_scope(0u64);

    assert!(c.has((flecs::ChildOf::ID, world.component_id::<Scope>())));
}

#[test]
fn register_w_root_name() {
    #[derive(Component, Default)]
    struct Scope;

    let world = World::new();

    let c = world.component_named::<Scope>("::Root");

    assert!(!c.has((flecs::ChildOf::ID, flecs::Wildcard::ID)));
    assert_eq!(c.path(), Some("::Root".to_string()));
}

#[test]
fn register_nested_w_root_name() {
    mod nested {
        use flecs_ecs::prelude::*;
        #[derive(Component, Default)]
        pub struct FromScope;
    }

    let world = World::new();

    let c = world.component_named::<nested::FromScope>("::Root");

    assert!(!c.has((flecs::ChildOf::ID, flecs::Wildcard::ID)));
    assert_eq!(c.path(), Some("::Root".to_string()));
}

#[test]
fn set_lookup_path() {
    let world = World::new();

    let parent = world.entity_named("Parent");
    let mut child_id = flecs_ecs::core::Entity::from(0u64);
    world.scope(parent.id(), |w| {
        child_id = w.entity_named("Child").id();
    });

    assert_eq!(world.lookup("Parent").id(), parent.id());
    assert!(world.try_lookup("Child").is_none());
    assert_eq!(world.lookup("Parent::Child").id(), child_id);

    let old_path = world.set_lookup_path(parent.id());
    assert_eq!(world.lookup("Parent").id(), parent.id());
    assert_eq!(world.lookup("Child").id(), child_id);
    assert_eq!(world.lookup("Parent::Child").id(), child_id);

    unsafe { flecs_ecs::sys::ecs_set_lookup_path(world.ptr_mut(), old_path) };
}

#[test]
fn run_post_frame() {
    use core::cell::Cell;

    thread_local! {
        static CTX: Cell<i32> = const { Cell::new(10) };
    }

    let mut world = World::new();

    world.system::<()>().run(|mut it| {
        while it.next() {
            it.world().run_post_frame(|_w| {
                CTX.with(|c| c.set(c.get() + 1));
            });
        }
    });

    CTX.with(|c| assert_eq!(c.get(), 10));
    world.progress_time(1.0);
    CTX.with(|c| assert_eq!(c.get(), 11));
}

#[test]
fn component_w_low_id() {
    let world = World::new();

    let p = world.component::<Position>();
    assert_ne!(p.id(), 0);
    // FLECS_HI_COMPONENT_ID is 256; user components registered at startup
    // get IDs in the low range.
    assert!(p.id() < 256, "expected low component id, got {}", p.id());
}

#[test]
fn get_ref() {
    #[derive(Component)]
    struct Space {
        v: i32,
    }

    let world = World::new();
    world.set(Space { v: 12 });

    world.try_get::<&Space>(|s| {
        assert_eq!(s.v, 12);
    });
}

#[test]
fn get_set_log_level() {
    let original = get_log_level();
    set_log_level(-1);
    assert_eq!(get_log_level(), -1);
    set_log_level(original);
}

#[test]
fn reset_world() {
    let world = World::new();
    let e = world.entity().id();

    assert!(world.exists(e));
    let world = world.reset();
    assert!(!world.exists(e));
}

#[test]
#[cfg(feature = "flecs_rest")]
fn reset_set_rest_after_reset() {
    let world = World::new();
    let world = world.reset();
    world.set(flecs::rest::Rest::default());
    assert!(world.try_get::<&flecs::rest::Rest>(|_| ()).is_some());
}

#[test]
fn id_from_pair_type() {
    let world = World::new();

    let id = world.id_view_from((Position::id(), Velocity::id()));
    assert!(id.is_pair());
    assert_eq!(id.first_id().id(), world.component_id::<Position>());
    assert_eq!(id.second_id().id(), world.component_id::<Velocity>());
}

#[test]
fn scope_w_name() {
    let world = World::new();

    let parent = world.entity_named("parent");
    let _scope = world.set_scope(parent.id());
    let child = world.entity();

    assert!(child.has((flecs::ChildOf::ID, parent.id())));
    world.set_scope(0); // Reset scope
}

#[test]
fn set_get_context() {
    let world = World::new();

    let mut ctx: i32 = 0;
    world.set_context(&mut ctx as *mut i32 as *mut core::ffi::c_void, None);
    assert_eq!(world.context() as *const i32, &ctx as *const i32);
}

#[test]
fn set_get_binding_context() {
    let world = World::new();

    let mut ctx: i32 = 0;
    world.set_context(&mut ctx as *mut i32 as *mut core::ffi::c_void, None);
    let retrieved = world.context() as *const i32;
    assert_eq!(retrieved, &ctx as *const i32);
}

#[test]
fn set_get_context_w_free() {
    unsafe extern "C-unwind" fn ctx_free_cb(ctx: *mut core::ffi::c_void) {
        unsafe { *(ctx as *mut i32) = 10 };
    }

    let mut ctx_val: i32 = 0;
    {
        let world = World::new();
        world.set_context(
            &mut ctx_val as *mut i32 as *mut core::ffi::c_void,
            Some(ctx_free_cb),
        );
        assert_eq!(world.context() as *const i32, &ctx_val as *const i32);
        assert_eq!(ctx_val, 0);
    }
    assert_eq!(ctx_val, 10);
}

#[test]
fn make_pair() {
    let world = World::new();

    let r = world.entity();
    let t = world.entity();
    let id = world.id_view_from((r, t));

    assert!(id.is_pair());
    assert_eq!(id.first_id().id(), r.id());
    assert_eq!(id.second_id().id(), t.id());
}

#[test]
fn make_pair_of_pair_type() {
    let world = World::new();

    let t = world.entity();
    let id = world.id_view_from((world.component_id::<Position>(), t.id()));

    assert!(id.is_pair());
    assert_eq!(id.first_id().id(), world.component_id::<Position>());
    assert_eq!(id.second_id().id(), t.id());
}

#[test]
fn delta_time() {
    thread_local! {
        static DT: core::cell::Cell<f32> = const { core::cell::Cell::new(0.0) };
    }

    let mut world = World::new();
    world.entity().add(Tag::id());

    world.system::<()>().with(Tag::id()).run(|mut it| {
        while it.next() {
            DT.with(|dt| dt.set(it.delta_time()));
        }
    });

    world.progress_time(2.0);

    DT.with(|dt| assert!((dt.get() - 2.0_f32).abs() < f32::EPSILON));
}

#[test]
fn register_nested_component_in_module() {
    mod nested_component_module {
        use flecs_ecs::prelude::*;

        #[derive(Component, Default)]
        pub struct Foo;
        #[derive(Component, Default)]
        pub struct Bar;

        pub struct Module;
        impl Module {
            pub fn import(world: &World) {
                world.component::<Foo>();
                world.component::<Bar>();
            }
        }
    }
    use nested_component_module::*;

    let world = World::new();
    Module::import(&world);

    assert_ne!(world.component_id::<Foo>(), 0u64);
    assert_ne!(world.component_id::<Bar>(), 0u64);
}

#[test]
fn atfini() {
    thread_local! {
        static ATFINI_INVOKED: core::cell::Cell<i32> = const { core::cell::Cell::new(0) };
    }
    ATFINI_INVOKED.with(|c| c.set(0));

    fn on_destroyed(_w: WorldRef) {
        ATFINI_INVOKED.with(|c| c.set(c.get() + 1));
    }

    {
        let world = World::new();
        world.on_destroyed(on_destroyed);
    }

    ATFINI_INVOKED.with(|c| assert_eq!(c.get(), 1));
}

#[test]
fn atfini_w_ctx() {
    thread_local! {
        static CALLED: core::cell::Cell<bool> = const { core::cell::Cell::new(false) };
    }
    CALLED.with(|c| c.set(false));

    fn fini_cb(_world: WorldRef) {
        CALLED.with(|c| c.set(true));
    }

    {
        let world = World::new();
        world.on_destroyed(fini_cb);
    }

    assert!(CALLED.with(core::cell::Cell::get));
}

#[test]
fn get_mut_type() {
    let world = World::new();

    let has = world.try_get::<&mut Position>(|_| ()).is_some();
    assert!(!has);

    world.set(Position { x: 10, y: 20 });
    world.try_get::<&mut Position>(|pos| {
        assert_eq!(pos.x, 10);
        assert_eq!(pos.y, 20);
    });
}

#[test]
fn get_mut_rel_type() {
    #[derive(Component)]
    struct Tgt;

    let world = World::new();

    let has = world.try_get::<&mut (Position, Tgt)>(|_| ()).is_some();
    assert!(!has);

    world.set_pair::<Position, Tgt>(Position { x: 10, y: 20 });
    world.try_get::<&mut (Position, Tgt)>(|pos| {
        assert_eq!(pos.x, 10);
        assert_eq!(pos.y, 20);
    });
}

#[test]
fn world_mini() {
    thread_local! {
        static COUNT: core::cell::Cell<i32> = const { core::cell::Cell::new(0) };
    }
    COUNT.with(|c| c.set(0));

    {
        let world = World::new_mini();

        world.on_destroyed(|_| {
            COUNT.with(|c| c.set(c.get() + 1));
        });

        assert!(world.try_lookup("flecs.system").is_none());
        assert!(world.try_lookup("flecs.pipeline").is_none());
        assert!(world.try_lookup("flecs.timer").is_none());
        assert!(world.try_lookup("flecs.meta").is_none());
    }

    assert_eq!(COUNT.with(core::cell::Cell::get), 1);
}

#[test]
fn copy_world() {
    let world1 = World::new();
    let world2 = world1.clone();

    assert_eq!(world1.ptr_mut(), world2.ptr_mut());
}

#[test]
fn fini_reentrancy() {
    #[derive(Component, Default)]
    struct A {
        a: i32,
    }

    let world = World::new();

    world
        .component::<A>()
        .on_remove(|e: EntityView, _a: &mut A| {
            let _w = e.world();
        });

    world.entity().add(A::id());
    // world drops here; on_remove fires with world reference - should not abort
}

#[test]
fn fini_copy_move_assign() {
    let world1 = World::new();
    let world2 = world1.clone();
    // Both world1 and world2 share the same underlying world pointer.
    assert_eq!(world1.ptr_mut(), world2.ptr_mut());
}

#[test]
fn world_init_fini_log_all() {
    let _world = World::new();
    // World creation and destruction with default addons should not crash.
}

#[test]
fn exclusive_access_self_mutate() {
    let world = World::new();

    world.exclusive_access_begin(None);

    let e = world.entity();
    e.add(Position::id());
    assert!(e.has(Position::id()));

    world.exclusive_access_end(false);
}

#[test]
#[cfg_attr(not(debug_assertions), ignore)]
fn exclusive_access_other_mutate() {
    struct SendPtr(*mut flecs_ecs::sys::ecs_world_t);
    // SAFETY: the pointer is only used to reconstruct a world handle on the
    // other thread; the test asserts that flecs' exclusive access guard
    // aborts that access at runtime.
    unsafe impl Send for SendPtr {}

    let world = World::new();
    let _guard = FlecsPanicAbortGuard::install();

    world.exclusive_access_begin(None);

    let world_ptr = SendPtr(world.ptr_mut());
    let thread = std::thread::spawn(move || {
        let world_ptr = world_ptr;
        let world_ref = unsafe { WorldRef::from_ptr(world_ptr.0) };
        world_ref.entity();
    });

    assert!(thread.join().is_err());

    world.exclusive_access_end(false);
}

#[test]
fn id_if_registered() {
    {
        let world = World::new();

        assert!(world.get_component_id::<Position>().is_none());
        assert!(world.get_component_id::<Position>().is_none());

        let c = world.component::<Position>();

        assert_eq!(world.get_component_id::<Position>(), Some(c.id()));
    }

    {
        let world = World::new();

        assert!(world.get_component_id::<Position>().is_none());
        assert!(world.get_component_id::<Position>().is_none());

        let c = world.component::<Position>();

        assert_eq!(world.get_component_id::<Position>(), Some(c.id()));
    }
}

#[test]
fn get_type_info_t() {
    let world = World::new();
    let c = world.component::<Position>();
    let ti = world.type_info_from(c.id());
    assert!(ti.is_some());
    let ti = unsafe { &*ti.unwrap() };
    assert_eq!(ti.size, core::mem::size_of::<Position>() as i32);
    assert_eq!(ti.alignment, core::mem::align_of::<Position>() as i32);
    assert_eq!(ti.component, *world.component_id::<Position>());
}

#[test]
fn get_type_info_by_type() {
    let world = World::new();
    let ti = world.type_info_from(world.component_id::<Position>());
    assert!(ti.is_some());
    let ti = unsafe { &*ti.unwrap() };
    assert_eq!(ti.size, core::mem::size_of::<Position>() as i32);
    assert_eq!(ti.alignment, core::mem::align_of::<Position>() as i32);
    assert_eq!(ti.component, *world.component_id::<Position>());
}

#[test]
fn get_type_info_r_t() {
    let world = World::new();
    let c = world.component::<Position>();
    let tgt = world.entity();
    let ti = world.type_info_from((c.id(), tgt.id()));
    assert!(ti.is_some());
    let ti = unsafe { &*ti.unwrap() };
    assert_eq!(ti.size, core::mem::size_of::<Position>() as i32);
    assert_eq!(ti.alignment, core::mem::align_of::<Position>() as i32);
    assert_eq!(ti.component, *world.component_id::<Position>());
}

#[test]
fn get_type_info_rel_type_tgt_id() {
    let world = World::new();
    let tgt = world.entity();
    let ti = world.type_info_from((world.component_id::<Position>(), tgt.id()));
    assert!(ti.is_some());
    let ti = unsafe { &*ti.unwrap() };
    assert_eq!(ti.size, core::mem::size_of::<Position>() as i32);
    assert_eq!(ti.alignment, core::mem::align_of::<Position>() as i32);
    assert_eq!(ti.component, *world.component_id::<Position>());
}

#[test]
fn get_type_info_rel_type_tgt_type() {
    #[derive(Component)]
    struct Tgt;
    let world = World::new();
    let ti = world.type_info_from((
        world.component_id::<Position>(),
        world.component_id::<Tgt>(),
    ));
    assert!(ti.is_some());
    let ti = unsafe { &*ti.unwrap() };
    assert_eq!(ti.size, core::mem::size_of::<Position>() as i32);
    assert_eq!(ti.alignment, core::mem::align_of::<Position>() as i32);
    assert_eq!(ti.component, *world.component_id::<Position>());
}

#[test]
fn get_type_info_t_tag() {
    #[derive(Component)]
    struct LocalTag;
    let world = World::new();
    let c = world.component::<LocalTag>();
    let ti = world.type_info_from(c.id());
    assert!(ti.is_none());
}

#[test]
fn get_type_info_by_type_tag() {
    #[derive(Component)]
    struct LocalTag;
    let world = World::new();
    let ti = world.type_info_from(world.component_id::<LocalTag>());
    assert!(ti.is_none());
}

#[test]
fn get_type_info_r_t_tag() {
    #[derive(Component)]
    struct LocalTag;
    let world = World::new();
    let c = world.component::<LocalTag>();
    let tgt = world.entity();
    let ti = world.type_info_from((c.id(), tgt.id()));
    assert!(ti.is_none());
}

#[test]
fn get_type_info_rel_type_tgt_id_tag() {
    #[derive(Component)]
    struct LocalTag;
    let world = World::new();
    let tgt = world.entity();
    let ti = world.type_info_from((world.component_id::<LocalTag>(), tgt.id()));
    assert!(ti.is_none());
}

#[test]
fn get_type_info_rel_type_tgt_type_tag() {
    #[derive(Component)]
    struct LocalTag;
    #[derive(Component)]
    struct LocalTgt;
    let world = World::new();
    let ti = world.type_info_from((
        world.component_id::<LocalTag>(),
        world.component_id::<LocalTgt>(),
    ));
    assert!(ti.is_none());
}

#[test]
fn builtin_after_reset() {
    let world = World::new();

    let c1 = world.component::<flecs::Component>().id();
    assert_ne!(c1, 0);

    let world = world.reset();

    let c2 = world.component::<flecs::Component>();
    assert_ne!(c2.id(), 0);
    assert_eq!(c1, c2.id());
}

#[test]
fn readonly_begin_end() {
    let world = World::new();

    world.entity().set(Position { x: 0, y: 0 });

    world.readonly_begin(false);
    let count = world.count(Position::id());
    world.readonly_end();

    assert_eq!(count, 1);
}

#[test]
fn defer_begin_end() {
    let world = World::new();

    world.defer_begin();
    world.entity().add(Position::id());
    world.defer_end();

    assert_eq!(world.count(Position::id()), 1);
}

#[test]
fn frame_begin_end() {
    let world = World::new();

    let dt = world.frame_begin(1.0);
    assert!(dt >= 0.0);
    world.frame_end();
}

#[test]
fn on_destroyed() {
    static mut CALLED: bool = false;

    fn on_destroyed(_w: WorldRef) {
        unsafe {
            CALLED = true;
        }
    }

    {
        let world = World::new();
        world.on_destroyed(on_destroyed);
    }

    unsafe { assert!(CALLED) };
}

#[test]
fn each_child() {
    let world = World::new();

    let parent = world.entity_named("Parent");
    {
        let _scope = world.set_scope(parent.id());
        let _c1 = world.entity_named("C1");
        let _c2 = world.entity_named("C2");
    }
    world.set_scope(0); // Reset scope

    // C1 and C2 are children of Parent (created via scope), not the world entity.
    let mut count = 0;
    parent.each_child(|_| {
        count += 1;
    });

    assert_eq!(count, 2);
}

#[test]
fn set_entity_range() {
    let world = World::new();

    // Use a large min value that is safely above all built-in entity IDs.
    let min = 500000u32;
    let max = 1000000u32;
    let range = world.entity_range_new(min, max);
    world.entity_range_set(range);

    let e = world.entity();
    assert!(e.id() >= min as u64 && e.id() < max as u64);
}

#[test]
fn stage_count() {
    let world = World::new();

    world.set_stage_count(4);
    assert_eq!(world.get_stage_count(), 4);
}

#[test]
fn preallocate_entity_count() {
    let world = World::new();

    world.preallocate_entity_count(100);

    for _ in 0..100 {
        world.entity().add(Position::id());
    }

    assert_eq!(world.count(Position::id()), 100);
}

#[test]
fn get_info() {
    let world = World::new();

    let info = world.info();

    assert!(info.table_count >= 0);
}

#[test]
fn should_quit() {
    let world = World::new();

    assert!(!world.should_quit());
}

#[test]
fn is_deferred() {
    let world = World::new();

    assert!(!world.is_deferred());
    world.defer_begin();
    assert!(world.is_deferred());
    world.defer_end();
    assert!(!world.is_deferred());
}

#[test]
fn is_readonly() {
    let world = World::new();

    assert!(!world.is_readonly());
}

#[test]
fn check_components_consistency() {
    let world = World::new();

    let pos1 = world.component::<Position>();
    let pos2 = world.component::<Position>();

    assert_eq!(pos1.id(), pos2.id());
}

#[test]
fn register_after_delete() {
    let world = World::new();

    let e1 = world.component::<Position>();
    assert!(e1.is_alive());
    e1.destruct();

    let e2 = world.component::<Position>();
    assert!(e2.is_alive());
}

#[test]
fn entity_with_component() {
    let world = World::new();

    let e = world
        .entity()
        .set(Position { x: 5, y: 10 })
        .set(Velocity { x: 1, y: 2 });

    assert!(e.has(Position::id()));
    assert!(e.has(Velocity::id()));

    e.get::<&Position>(|p| {
        assert_eq!(p.x, 5);
        assert_eq!(p.y, 10);
    });

    e.get::<&Velocity>(|v| {
        assert_eq!(v.x, 1);
        assert_eq!(v.y, 2);
    });
}

#[test]
fn query_count_consistency() {
    let world = World::new();

    for _ in 0..5 {
        world.entity().add(Position::id());
    }

    assert_eq!(world.count(Position::id()), 5);
}

#[test]
fn entity_view_operations() {
    let world = World::new();

    let e = world.entity();
    assert!(e.is_alive());

    let id = e.id();
    assert_ne!(id, 0);

    let e2 = world.lookup(world.entity_named("test").name().as_str());
    assert!(e2.is_alive());
}

#[test]
fn context_operations() {
    let world = World::new();

    let mut ctx_val: i32 = 42;
    world.set_context(&mut ctx_val as *mut i32 as *mut core::ffi::c_void, None);

    let ctx = world.context();
    assert_eq!(ctx as *const i32, &ctx_val as *const i32);
}
