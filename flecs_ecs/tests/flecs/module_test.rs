#![allow(dead_code)]
#![allow(clippy::std_instead_of_alloc)]
use flecs_ecs::prelude::*;

// ---------------------------------------------------------------------------
// Shared component types used across multiple tests
// ---------------------------------------------------------------------------
#[derive(Component, Default)]
struct Position {
    pub x: f32,
    pub y: f32,
}

#[derive(Component, Default)]
struct Velocity {
    pub x: f32,
    pub y: f32,
}

// ---------------------------------------------------------------------------
// Module definitions mirroring Module.cpp
// ---------------------------------------------------------------------------

mod ns {
    use flecs_ecs::prelude::*;

    #[derive(Component)]
    pub struct NestedModule;

    impl Module for NestedModule {
        fn module(world: &World) {
            world.module::<NestedModule>("ns::NestedModule");
            world.component::<super::Velocity>();
        }
    }

    #[derive(Component)]
    pub struct SimpleModule;

    impl Module for SimpleModule {
        fn module(world: &World) {
            world.module::<SimpleModule>("ns::SimpleModule");
            world.import::<NestedModule>();
            world.component::<super::Position>();
        }
    }

    #[derive(Component)]
    pub struct NestedNameSpaceType;

    #[derive(Component)]
    pub struct NestedTypeModule;

    impl Module for NestedTypeModule {
        fn module(world: &World) {
            world.module::<NestedTypeModule>("ns::NestedTypeModule");
            world.component::<NestedType>();
            world.component::<NestedNameSpaceType>();
        }
    }

    #[derive(Component)]
    pub struct NestedType;

    #[derive(Component)]
    pub struct NamedModule;

    impl Module for NamedModule {
        fn module(world: &World) {
            world.module::<NamedModule>("::my_scope::NamedModule");
            world.component::<super::Position>();
        }
    }

    #[derive(Component)]
    pub struct ImplicitModule;

    impl Module for ImplicitModule {
        fn module(world: &World) {
            world.module::<ImplicitModule>("ns::ImplicitModule");
            world.component::<super::Position>();
        }
    }

    #[derive(Component)]
    pub struct NamedModuleInRoot;

    impl Module for NamedModuleInRoot {
        fn module(world: &World) {
            world.module::<NamedModuleInRoot>("::NamedModuleInRoot");
            world.component::<super::Position>();
        }
    }

    #[derive(Component)]
    pub struct ReparentModule;

    impl Module for ReparentModule {
        fn module(world: &World) {
            let m = world.module::<ReparentModule>("ReparentModule");
            m.child_of(world.entity_named("::parent"));

            let other = world.entity_named("::ns::ReparentModule");
            assert_ne!(other.id(), 0);
            assert_ne!(other.id(), m.id());
        }
    }
}

#[derive(Component)]
struct ReparentRootModule;

impl Module for ReparentRootModule {
    fn module(world: &World) {
        world.module::<ReparentRootModule>("ns::ReparentRootModule");
    }
}

mod renamed_root_module {
    use flecs_ecs::prelude::*;

    #[derive(Component)]
    pub struct Module;

    impl flecs_ecs::addons::module::Module for Module {
        fn module(world: &World) {
            world.module::<Module>("::MyModule");
            for _ in 0..5 {
                let e = world.entity();
                // entity id fits in u32 (low-id range)
                assert_eq!(e.id().0 as u32 as u64, e.id().0);
            }
        }
    }
}

mod ns_parent {
    use flecs_ecs::prelude::*;

    #[derive(Component, Default)]
    pub struct NsType {
        pub x: f32,
    }

    #[derive(Component)]
    pub struct ShorterParent;

    impl Module for ShorterParent {
        fn module(world: &World) {
            world.module::<ShorterParent>("ns::ShorterParent");
            world.component::<NsType>();
        }
    }

    #[derive(Component)]
    pub struct LongerParent;

    impl Module for LongerParent {
        fn module(world: &World) {
            world.module::<LongerParent>("ns_parent_namespace::LongerParent");
            world.component::<NsType>();
        }
    }

    pub mod ns_child {
        use flecs_ecs::prelude::*;

        #[derive(Component)]
        pub struct Nested;

        impl Module for Nested {
            fn module(world: &World) {
                world.module::<Nested>("ns::child::Nested");
                world.component::<super::NsType>();
            }
        }
    }
}

mod module_with_core_name_mod {
    use super::Position;
    use flecs_ecs::prelude::*;

    #[derive(Component)]
    pub struct Module;

    impl flecs_ecs::addons::module::Module for Module {
        fn module(world: &World) {
            world.module::<Module>("::Module");
            world.component::<Position>();
        }
    }
}

thread_local! {
    static MODULE_CTOR_INVOKED: core::cell::RefCell<i32> = const { core::cell::RefCell::new(0) };
    static MODULE_DTOR_INVOKED: core::cell::RefCell<i32> = const { core::cell::RefCell::new(0) };
}

fn reset_dtor_counts() {
    MODULE_CTOR_INVOKED.with(|c| *c.borrow_mut() = 0);
    MODULE_DTOR_INVOKED.with(|c| *c.borrow_mut() = 0);
}

#[derive(Component)]
struct ModuleWDtor {
    _pad: u8,
}

impl Drop for ModuleWDtor {
    fn drop(&mut self) {
        MODULE_DTOR_INVOKED.with(|c| *c.borrow_mut() += 1);
    }
}

impl Module for ModuleWDtor {
    fn module(world: &World) {
        world.module::<ModuleWDtor>("ModuleWDtor");
        MODULE_CTOR_INVOKED.with(|c| *c.borrow_mut() += 1);
        world.system::<()>().run(|_| {});
    }
}

#[derive(Component)]
struct ModuleA;

#[derive(Component)]
struct ModuleAComponent;

impl Module for ModuleA {
    fn module(world: &World) {
        world.component::<ModuleAComponent>();
    }
}

#[derive(Component)]
struct SystemAndImplicitComponent;

impl Module for SystemAndImplicitComponent {
    fn module(world: &World) {
        world
            .system_named::<()>("VelocitySys")
            .with(Velocity::id())
            .run(|_| {});
    }
}

#[derive(Component)]
struct SystemAndExplicitComponent;

impl Module for SystemAndExplicitComponent {
    fn module(world: &World) {
        world.component::<Velocity>();
        world
            .system_named::<()>("VelocitySys")
            .with(Velocity::id())
            .run(|_| {});
    }
}

thread_local! {
    static SINGLETON_TEST_INVOKED: core::cell::RefCell<i32> = const { core::cell::RefCell::new(0) };
}

#[derive(Component)]
struct SingletonTest;

impl Module for SingletonTest {
    fn module(world: &World) {
        assert!(world.component::<SingletonTest>().has(flecs::Singleton::ID));
        SINGLETON_TEST_INVOKED.with(|c| *c.borrow_mut() += 1);
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[test]
fn module_import() {
    let world = World::new();

    let m = world.import::<ns::SimpleModule>();
    assert_ne!(m.id(), 0);
    assert_eq!(m.path(), Some("::ns::SimpleModule".to_string()));
    assert!(m.has(flecs::Module::ID));

    let e = world.entity().add(Position::id());
    assert_ne!(e.id(), 0);
    assert!(e.has(Position::id()));
}

#[test]
fn module_lookup_from_scope() {
    let world = World::new();
    world.import::<ns::SimpleModule>();

    let ns_entity = world.lookup("ns");
    assert_ne!(ns_entity.id(), 0);

    let module_entity = world.lookup("ns::SimpleModule");
    assert_ne!(module_entity.id(), 0);

    let position_entity = world.lookup("ns::SimpleModule::Position");
    assert_ne!(position_entity.id(), 0);

    let nested_module = ns_entity.lookup("SimpleModule");
    assert_eq!(module_entity.id(), nested_module.id());

    let module_position = module_entity.lookup("Position");
    assert_eq!(position_entity.id(), module_position.id());

    let ns_position = ns_entity.lookup("SimpleModule::Position");
    assert_eq!(position_entity.id(), ns_position.id());
}

#[test]
fn module_nested_module() {
    let world = World::new();
    world.import::<ns::SimpleModule>();

    let velocity = world.lookup("ns::NestedModule::Velocity");
    assert_ne!(velocity.id(), 0);

    assert_eq!(
        velocity.path(),
        Some("::ns::NestedModule::Velocity".to_string())
    );
}

#[test]
fn module_nested_type_module() {
    let world = World::new();
    world.import::<ns::NestedTypeModule>();

    let ns_entity = world.lookup("ns");
    assert_ne!(ns_entity.id(), 0);

    let module_entity = world.lookup("ns::NestedTypeModule");
    assert_ne!(module_entity.id(), 0);

    let type_entity = world.lookup("ns::NestedTypeModule::NestedType");
    assert_ne!(type_entity.id(), 0);

    let ns_type_entity = world.lookup("ns::NestedTypeModule::NestedNameSpaceType");
    assert_ne!(ns_type_entity.id(), 0);

    let mut childof_count = 0i32;
    type_entity.each_pair(flecs::ChildOf::ID, flecs::Wildcard::ID, |_| {
        childof_count += 1;
    });
    assert_eq!(childof_count, 1);

    childof_count = 0;
    ns_type_entity.each_pair(flecs::ChildOf::ID, flecs::Wildcard::ID, |_| {
        childof_count += 1;
    });
    assert_eq!(childof_count, 1);
}

#[test]
fn module_component_redefinition_outside_module() {
    let world = World::new();
    world.import::<ns::SimpleModule>();

    let pos_comp = world.lookup("ns::SimpleModule::Position");
    assert_ne!(pos_comp.id(), 0);

    let pos = world.component::<Position>();
    assert_ne!(pos.id(), 0);
    assert_eq!(pos.id(), pos_comp.id());

    let mut childof_count = 0i32;
    pos_comp.each_pair(flecs::ChildOf::ID, flecs::Wildcard::ID, |_| {
        childof_count += 1;
    });
    assert_eq!(childof_count, 1);
}

#[test]
fn module_tag_on_namespace() {
    let world = World::new();

    let mid = world.import::<ns::NestedModule>();
    assert!(mid.has(flecs::Module::ID));

    let nsid = world.lookup("ns");
    assert!(nsid.has(flecs::Module::ID));
}

#[test]
fn module_dtor_on_fini() {
    reset_dtor_counts();

    MODULE_CTOR_INVOKED.with(|c| assert_eq!(*c.borrow(), 0));
    MODULE_DTOR_INVOKED.with(|c| assert_eq!(*c.borrow(), 0));

    {
        let ecs = World::new();

        ecs.import::<ModuleWDtor>();

        MODULE_CTOR_INVOKED.with(|c| assert_eq!(*c.borrow(), 1));
        MODULE_DTOR_INVOKED.with(|c| assert_eq!(*c.borrow(), 0));
    }

    MODULE_DTOR_INVOKED.with(|c| assert_eq!(*c.borrow(), 1));
}

#[test]
fn module_register_w_root_name() {
    let world = World::new();

    let m = world.import::<ns::NamedModule>();
    let m_lookup = world.lookup("::my_scope::NamedModule");
    assert_ne!(m.id(), 0);
    assert_eq!(m.id(), m_lookup.id());

    let ns_lookup = world.try_lookup("::ns::NamedModule");
    assert!(ns_lookup.is_none());
}

#[test]
fn module_implicit_module() {
    let world = World::new();

    let m = world.import::<ns::ImplicitModule>();
    let m_lookup = world.lookup("::ns::ImplicitModule");
    assert_ne!(m.id(), 0);
    assert_eq!(m.id(), m_lookup.id());

    let p = world.component::<Position>();
    let p_lookup = world.lookup("::ns::ImplicitModule::Position");
    assert_ne!(p.id(), 0);
    assert_eq!(p.id(), p_lookup.id());
}

#[test]
fn module_module_in_namespace_w_root_name() {
    let world = World::new();

    let m = world.import::<ns::NamedModuleInRoot>();
    let m_lookup = world.lookup("::NamedModuleInRoot");
    assert_ne!(m.id(), 0);
    assert_eq!(m.id(), m_lookup.id());
    assert_eq!(m.path(), Some("::NamedModuleInRoot".to_string()));

    let p = world.component::<Position>();
    let p_lookup = world.lookup("::NamedModuleInRoot::Position");
    assert_ne!(p.id(), 0);
    assert_eq!(p.id(), p_lookup.id());
}

#[test]
fn module_module_as_entity() {
    let world = World::new();

    let m = world.import::<ns::SimpleModule>();
    assert_ne!(m.id(), 0);

    let e = world.component::<ns::SimpleModule>();
    assert_eq!(m.id(), e.id());
}

#[test]
fn module_module_as_component() {
    let world = World::new();

    let m = world.import::<ns::SimpleModule>();
    assert_ne!(m.id(), 0);

    let e = world.component::<ns::SimpleModule>();
    assert_eq!(m.id(), e.id());
}

#[test]
fn module_module_with_core_name() {
    let world = World::new();

    let m = world.import::<module_with_core_name_mod::Module>();
    assert_ne!(m.id(), 0);
    assert_eq!(m.path(), Some("::Module".to_string()));

    let pos = m.lookup("Position");
    assert_ne!(pos.id(), 0);
    assert_eq!(pos.path(), Some("::Module::Position".to_string()));
    assert_eq!(pos.id(), world.component_id::<Position>());
}

#[test]
fn module_import_addons_two_worlds() {
    let a = World::new();
    let m1 = a.import::<flecs_ecs::addons::stats::Stats>();
    let u1 = a.import::<flecs_ecs::addons::units::Units>();

    let b = World::new();
    let m2 = b.import::<flecs_ecs::addons::stats::Stats>();
    let u2 = b.import::<flecs_ecs::addons::units::Units>();

    assert_eq!(m1.id(), m2.id());
    assert_eq!(u1.id(), u2.id());
}

#[test]
fn module_lookup_module_after_reparent() {
    let world = World::new();

    let m = world.import::<ns::NestedModule>();
    assert_eq!(m.path(), Some("::ns::NestedModule".to_string()));
    assert_eq!(world.lookup("::ns::NestedModule").id(), m.id());

    let p = world.entity_named("p");
    m.child_of(p);
    assert_eq!(m.path(), Some("::p::NestedModule".to_string()));
    assert_eq!(world.lookup("::p::NestedModule").id(), m.id());

    assert_eq!(world.try_lookup("::ns::NestedModule"), None);

    let e = world.entity_named("::ns::NestedModule");
    assert_ne!(e.id(), m.id());

    let count_child_of_p = world
        .query::<()>()
        .expr("(ChildOf, p.NestedModule)")
        .build()
        .count();
    assert!(count_child_of_p > 0);

    let count_child_of_ns = world
        .query::<()>()
        .expr("(ChildOf, ns.NestedModule)")
        .build()
        .count();
    assert_eq!(count_child_of_ns, 0);
}

#[test]
fn module_reparent_module_in_ctor() {
    let world = World::new();

    let m = world.import::<ns::ReparentModule>();
    assert_eq!(m.path(), Some("::parent::ReparentModule".to_string()));

    let other = world.lookup("::ns::ReparentModule");
    assert_ne!(other.id(), 0);
    assert_ne!(other.id(), m.id());
}

mod namespace_lvl1 {
    pub mod namespace_lvl2 {
        pub mod struct_lvl1 {
            #[derive(flecs_ecs::prelude::Component)]
            pub struct StructLvl2_1;
            #[derive(flecs_ecs::prelude::Component)]
            pub struct StructLvl2_2;
        }
    }
}

// Rust Flecs strips the module path and registers only the short type name via
// get_type_name_without_scope. The CPP behavior of auto-creating Module-tagged
// scope entities from C++ namespaces does not exist in the Rust binding.
#[test]
#[ignore]
fn module_implicitly_add_module_to_scopes_component() {
    use namespace_lvl1::namespace_lvl2::struct_lvl1::StructLvl2_1;

    let ecs = World::new();

    let comp = ecs.component::<StructLvl2_1>();
    let mut current = comp.base.entity;
    assert_ne!(current.id(), 0);
    assert!(!current.has(flecs::Module::ID));
    assert!(current.has(flecs::Component::ID));
    assert_eq!(
        current.path(),
        Some("::namespace_lvl1::namespace_lvl2::struct_lvl1::StructLvl2_1".to_string())
    );

    current = current.parent().unwrap();
    assert_ne!(current.id(), 0);
    assert!(current.has(flecs::Module::ID));
    assert_eq!(
        current.path(),
        Some("::namespace_lvl1::namespace_lvl2::struct_lvl1".to_string())
    );

    current = current.parent().unwrap();
    assert_ne!(current.id(), 0);
    assert!(current.has(flecs::Module::ID));
    assert_eq!(
        current.path(),
        Some("::namespace_lvl1::namespace_lvl2".to_string())
    );

    current = current.parent().unwrap();
    assert_ne!(current.id(), 0);
    assert!(current.has(flecs::Module::ID));
    assert_eq!(current.path(), Some("::namespace_lvl1".to_string()));

    let top_parent = current.parent();
    assert!(top_parent.is_none() || top_parent.unwrap().id() == 0);
}

// Same reason as module_implicitly_add_module_to_scopes_component.
#[test]
#[ignore]
fn module_implicitly_add_module_to_scopes_entity() {
    use namespace_lvl1::namespace_lvl2::struct_lvl1::StructLvl2_2;

    let ecs = World::new();

    // Use ecs.component::<StructLvl2_2>() to register and get entity - mirrors
    // CPP ecs.entity<StructLvl2_2>().set<flecs::Component>({})
    let comp2 = ecs.component::<StructLvl2_2>();
    let mut current = comp2.base.entity;
    assert_ne!(current.id(), 0);
    assert!(!current.has(flecs::Module::ID));
    assert!(current.has(flecs::Component::ID));
    assert_eq!(
        current.path(),
        Some("::namespace_lvl1::namespace_lvl2::struct_lvl1::StructLvl2_2".to_string())
    );

    current = current.parent().unwrap();
    assert_ne!(current.id(), 0);
    assert!(current.has(flecs::Module::ID));
    assert_eq!(
        current.path(),
        Some("::namespace_lvl1::namespace_lvl2::struct_lvl1".to_string())
    );

    current = current.parent().unwrap();
    assert_ne!(current.id(), 0);
    assert!(current.has(flecs::Module::ID));
    assert_eq!(
        current.path(),
        Some("::namespace_lvl1::namespace_lvl2".to_string())
    );

    current = current.parent().unwrap();
    assert_ne!(current.id(), 0);
    assert!(current.has(flecs::Module::ID));
    assert_eq!(current.path(), Some("::namespace_lvl1".to_string()));

    let top_parent = current.parent();
    assert!(top_parent.is_none() || top_parent.unwrap().id() == 0);
}

#[test]
fn module_rename_namespace_shorter() {
    let ecs = World::new();

    let m = ecs.import::<ns_parent::ShorterParent>();
    assert!(m.has(flecs::Module::ID));
    assert_eq!(m.path(), Some("::ns::ShorterParent".to_string()));
    assert!(ecs.try_lookup("::ns_parent").is_none());
    assert!(ecs.try_lookup("::ns_parent::ShorterParent").is_none());
    assert!(
        ecs.try_lookup("::ns_parent::ShorterParent::NsType")
            .is_none()
    );
    assert!(ecs.try_lookup("::ns::ShorterParent::NsType").is_some());

    let ns = ecs.lookup("::ns");
    assert_ne!(ns.id(), 0);
    assert!(ns.has(flecs::Module::ID));
}

#[test]
fn module_rename_namespace_longer() {
    let ecs = World::new();

    let m = ecs.import::<ns_parent::LongerParent>();
    assert!(m.has(flecs::Module::ID));
    assert_eq!(
        m.path(),
        Some("::ns_parent_namespace::LongerParent".to_string())
    );
    assert!(ecs.try_lookup("::ns_parent").is_none());
    assert!(ecs.try_lookup("::ns_parent::LongerParent").is_none());
    assert!(
        ecs.try_lookup("::ns_parent::LongerParent::NsType")
            .is_none()
    );
    assert!(
        ecs.try_lookup("::ns_parent_namespace::LongerParent::NsType")
            .is_some()
    );

    let ns = ecs.lookup("::ns_parent_namespace");
    assert_ne!(ns.id(), 0);
    assert!(ns.has(flecs::Module::ID));
}

#[test]
fn module_rename_namespace_nested() {
    let ecs = World::new();

    let m = ecs.import::<ns_parent::ns_child::Nested>();
    assert!(m.has(flecs::Module::ID));

    assert_eq!(m.path(), Some("::ns::child::Nested".to_string()));
    assert!(ecs.try_lookup("::ns::child::Nested::NsType").is_some());
    assert!(
        ecs.try_lookup("::ns_parent::ns_child::Nested::NsType")
            .is_none()
    );
    assert!(ecs.try_lookup("::ns_parent::ns_child::Nested").is_none());
    assert!(ecs.try_lookup("::ns_parent::ns_child").is_none());
    assert!(ecs.try_lookup("::ns_parent").is_none());

    let ns = ecs.lookup("::ns");
    assert_ne!(ns.id(), 0);
    assert!(ns.has(flecs::Module::ID));

    let ns_child = ecs.lookup("::ns::child");
    assert_ne!(ns_child.id(), 0);
    assert!(ns_child.has(flecs::Module::ID));
}

#[test]
fn module_rename_reparent_root_module() {
    let ecs = World::new();

    let m = ecs.import::<ReparentRootModule>();
    let p = m.parent().unwrap();
    assert_ne!(p.id(), 0);
    assert_eq!(p.name(), "ns");
    assert_eq!(m.name(), "ReparentRootModule");
}

#[test]
fn module_no_recycle_after_rename_reparent() {
    let ecs = World::new();

    let m = ecs.import::<renamed_root_module::Module>();
    let p = m.parent();
    assert!(p.is_none() || p.unwrap().id() == 0);
    assert_eq!(m.name(), "MyModule");
}

#[test]
fn module_reimport_after_delete() {
    use module_with_core_name_mod::Module as CoreModule;

    let ecs = World::new();

    {
        let m = ecs.import::<CoreModule>();
        assert_eq!(m.lookup("Position").id(), ecs.component::<Position>().id());
        assert_eq!(m.id(), ecs.component::<CoreModule>().id());
    }

    ecs.component::<CoreModule>().destruct();

    {
        let m = ecs.import::<CoreModule>();
        assert_eq!(m.lookup("Position").id(), ecs.component::<Position>().id());
        assert_eq!(m.id(), ecs.component::<CoreModule>().id());
    }
}

#[test]
fn module_component_name_w_module_name() {
    let world = World::new();

    let m = world.import::<ModuleA>();
    assert_ne!(m.id(), 0);
    let c = world.lookup("ModuleA::ModuleAComponent");
    assert_ne!(c.id(), 0);
    assert_eq!(c.name(), "ModuleAComponent");
    assert_eq!(c.parent().unwrap().name(), "ModuleA");
}

#[test]
fn module_delete_module_w_implicit_component_and_system() {
    let world = World::new();

    let m = world.import::<SystemAndImplicitComponent>();

    assert!(m.try_lookup("Velocity").is_none());
    assert_ne!(world.lookup("Velocity").id(), 0);
    assert_ne!(m.lookup("VelocitySys").id(), 0);

    m.destruct();
    // verify no crash (if we reach this point, no panic occurred)
}

#[test]
fn module_delete_module_w_explicit_component_and_system() {
    let world = World::new();

    let m = world.import::<SystemAndExplicitComponent>();

    assert_ne!(m.lookup("Velocity").id(), 0);
    assert_ne!(m.lookup("VelocitySys").id(), 0);

    m.destruct();
    // verify no crash (if we reach this point, no panic occurred)
}

#[test]
fn module_module_has_singleton() {
    let world = World::new();

    let e = world.import::<SingletonTest>();

    assert!(e.has(flecs::Singleton::ID));
}

#[test]
fn component_scopes_do_not_become_modules() {
    let world = World::new();

    let current = unsafe { world
        .component_named::<namespace_lvl1::namespace_lvl2::struct_lvl1::StructLvl2_1>(
            "NamespaceLvl1::NamespaceLvl2::StructLvl1::StructLvl2_1",
        )
        .entity_view(&world) };
    assert!(current.id() != 0);
    assert!(!current.has(flecs::Module::ID));
    assert!(current.has(id::<flecs::Component>()));
    assert_eq!(
        current.path().unwrap(),
        "::NamespaceLvl1::NamespaceLvl2::StructLvl1::StructLvl2_1"
    );

    let current = current.parent().unwrap();
    assert!(current.id() != 0);
    assert!(!current.has(flecs::Module::ID));
    assert_eq!(
        current.path().unwrap(),
        "::NamespaceLvl1::NamespaceLvl2::StructLvl1"
    );

    let current = current.parent().unwrap();
    assert!(current.id() != 0);
    assert!(!current.has(flecs::Module::ID));
    assert_eq!(current.path().unwrap(), "::NamespaceLvl1::NamespaceLvl2");

    let current = current.parent().unwrap();
    assert!(current.id() != 0);
    assert!(!current.has(flecs::Module::ID));
    assert_eq!(current.path().unwrap(), "::NamespaceLvl1");

    assert!(current.parent().is_none());
}

#[test]
fn entity_scopes_do_not_become_modules() {
    let world = World::new();

    let current = unsafe { world
        .component_named::<namespace_lvl1::namespace_lvl2::struct_lvl1::StructLvl2_2>(
            "NamespaceLvl1::NamespaceLvl2::StructLvl1::StructLvl2_2",
        )
        .entity_view(&world) };
    assert!(current.id() != 0);
    assert!(!current.has(flecs::Module::ID));
    assert!(current.has(id::<flecs::Component>()));
    assert_eq!(
        current.path().unwrap(),
        "::NamespaceLvl1::NamespaceLvl2::StructLvl1::StructLvl2_2"
    );

    let current = current.parent().unwrap();
    assert!(current.id() != 0);
    assert!(!current.has(flecs::Module::ID));
    assert_eq!(
        current.path().unwrap(),
        "::NamespaceLvl1::NamespaceLvl2::StructLvl1"
    );

    let current = current.parent().unwrap();
    assert!(current.id() != 0);
    assert!(!current.has(flecs::Module::ID));
    assert_eq!(current.path().unwrap(), "::NamespaceLvl1::NamespaceLvl2");

    let current = current.parent().unwrap();
    assert!(current.id() != 0);
    assert!(!current.has(flecs::Module::ID));
    assert_eq!(current.path().unwrap(), "::NamespaceLvl1");

    assert!(current.parent().is_none());
}

// The derived `index()` is a per-type static that Rust shares across every
// monomorphization of a generic type, so `GenericModule<MarkerA>` and
// `GenericModule<MarkerB>` resolve to the same index. `World::import` used that
// index to write `components_array` unconditionally, aliasing the two modules
// onto one entity. They must now register as two distinct module entities.
#[test]
fn import_distinct_generic_module_instantiations() {
    #[derive(Component)]
    struct MarkerA;

    #[derive(Component)]
    struct MarkerB;

    #[derive(Component)]
    struct GenericModule<T: ComponentId + Send + Sync>(core::marker::PhantomData<T>);

    impl<T: ComponentId + Send + Sync> Module for GenericModule<T> {
        fn module(_world: &World) {}
    }

    let world = World::new();

    let a = world.import::<GenericModule<MarkerA>>();
    let b = world.import::<GenericModule<MarkerB>>();

    assert_ne!(a.id(), 0);
    assert_ne!(b.id(), 0);
    assert_ne!(
        a.id(),
        b.id(),
        "distinct generic module instantiations must yield distinct module entities"
    );

    // Re-importing each instantiation must return its own stable entity.
    assert_eq!(a.id(), world.import::<GenericModule<MarkerA>>().id());
    assert_eq!(b.id(), world.import::<GenericModule<MarkerB>>().id());
}
