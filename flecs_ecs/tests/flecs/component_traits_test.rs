#![allow(clippy::float_cmp)]
//! Ported from test/cpp/src/ComponentTraits.cpp

use crate::common_test::*;
use flecs_ecs::experimental::prelude::EntityGuardExt;

#[derive(Component)]
#[flecs(traits(DontFragment))]
struct TraitDontFragment {
    x: f32,
    y: f32,
}

#[derive(Component)]
#[flecs(traits((OnInstantiate, Override)))]
struct TraitOnInstantiateOverride {
    x: f32,
    y: f32,
}

#[derive(Component)]
#[flecs(traits((OnInstantiate, Inherit)))]
struct TraitOnInstantiateInherit {
    x: f32,
    y: f32,
}

#[derive(Component)]
#[flecs(traits((OnInstantiate, DontInherit)))]
struct TraitOnInstantiateDontInherit {
    x: f32,
    y: f32,
}

#[derive(Component)]
#[flecs(traits(DontFragment, (OnInstantiate, DontInherit)))]
struct TraitDontFragmentDontInherit {
    x: f32,
    y: f32,
}

#[derive(Component)]
struct TraitNone {
    x: f32,
    y: f32,
}

#[derive(Component)]
#[flecs(traits(Sparse))]
struct TraitSparse {
    x: f32,
    y: f32,
}

#[derive(Component)]
#[flecs(traits(Sparse, (OnInstantiate, Inherit)))]
struct TraitSparseInherit {
    x: f32,
    y: f32,
}

#[derive(Component)]
#[flecs(traits(DontFragment, (OnInstantiate, Inherit)))]
struct TraitDontFragmentInherit {
    x: f32,
    y: f32,
}

#[test]
fn dont_fragment_explicit() {
    let world = World::new();

    let c = world.component::<TraitDontFragment>();
    let c = unsafe { c.entity_view(&world) };
    assert!(c.has(flecs::DontFragment::ID));
    assert!(!c.has((flecs::OnInstantiate::ID, flecs::Wildcard::ID)));
}

#[test]
fn dont_fragment_implicit() {
    let world = World::new();

    let e = world.entity().set(TraitDontFragment { x: 10.0, y: 20.0 });
    assert!(e.has(TraitDontFragment::id()));

    let c = world.entity_from_id(TraitDontFragment::entity_id(&world));
    assert!(c.has(flecs::DontFragment::ID));
    assert!(!c.has((flecs::OnInstantiate::ID, flecs::Wildcard::ID)));
}

#[test]
fn on_instantiate_override_explicit() {
    let world = World::new();

    let c = world.component::<TraitOnInstantiateOverride>();
    let c = unsafe { c.entity_view(&world) };
    assert!(c.has((flecs::OnInstantiate::ID, flecs::Override::ID)));
    assert!(!c.has(flecs::DontFragment::ID));
}

#[test]
fn on_instantiate_override_implicit() {
    let world = World::new();

    let e = world
        .entity()
        .set(TraitOnInstantiateOverride { x: 10.0, y: 20.0 });
    assert!(e.has(TraitOnInstantiateOverride::id()));

    let c = world.entity_from_id(TraitOnInstantiateOverride::entity_id(&world));
    assert!(c.has((flecs::OnInstantiate::ID, flecs::Override::ID)));
    assert!(!c.has(flecs::DontFragment::ID));
}

#[test]
fn on_instantiate_inherit_explicit() {
    let world = World::new();

    let c = world.component::<TraitOnInstantiateInherit>();
    let c = unsafe { c.entity_view(&world) };
    assert!(c.has((flecs::OnInstantiate::ID, flecs::Inherit::ID)));
    assert!(!c.has(flecs::DontFragment::ID));
}

#[test]
fn on_instantiate_inherit_implicit() {
    let world = World::new();

    let e = world
        .entity()
        .set(TraitOnInstantiateInherit { x: 10.0, y: 20.0 });
    assert!(e.has(TraitOnInstantiateInherit::id()));

    let c = world.entity_from_id(TraitOnInstantiateInherit::entity_id(&world));
    assert!(c.has((flecs::OnInstantiate::ID, flecs::Inherit::ID)));
    assert!(!c.has(flecs::DontFragment::ID));
}

#[test]
fn on_instantiate_dont_inherit_explicit() {
    let world = World::new();

    let c = world.component::<TraitOnInstantiateDontInherit>();
    let c = unsafe { c.entity_view(&world) };
    assert!(c.has((flecs::OnInstantiate::ID, flecs::DontInherit::ID)));
    assert!(!c.has(flecs::DontFragment::ID));
}

#[test]
fn on_instantiate_dont_inherit_implicit() {
    let world = World::new();

    let e = world
        .entity()
        .set(TraitOnInstantiateDontInherit { x: 10.0, y: 20.0 });
    assert!(e.has(TraitOnInstantiateDontInherit::id()));

    let c = world.entity_from_id(TraitOnInstantiateDontInherit::entity_id(&world));
    assert!(c.has((flecs::OnInstantiate::ID, flecs::DontInherit::ID)));
    assert!(!c.has(flecs::DontFragment::ID));
}

// C++ on_instantiate_specialized_explicit/implicit use a template
// specialization of flecs::on_instantiate_trait; Rust expresses the same
// through the derive attribute, which is covered by the tests above.

#[test]
fn dont_fragment_dont_inherit_explicit() {
    let world = World::new();

    let c = world.component::<TraitDontFragmentDontInherit>();
    let c = unsafe { c.entity_view(&world) };
    assert!(c.has(flecs::DontFragment::ID));
    assert!(c.has((flecs::OnInstantiate::ID, flecs::DontInherit::ID)));
}

#[test]
fn dont_fragment_dont_inherit_implicit() {
    let world = World::new();

    let e = world
        .entity()
        .set(TraitDontFragmentDontInherit { x: 10.0, y: 20.0 });
    assert!(e.has(TraitDontFragmentDontInherit::id()));

    let c = world.entity_from_id(TraitDontFragmentDontInherit::entity_id(&world));
    assert!(c.has(flecs::DontFragment::ID));
    assert!(c.has((flecs::OnInstantiate::ID, flecs::DontInherit::ID)));
}

#[test]
fn no_traits_explicit() {
    let world = World::new();

    let c = world.component::<TraitNone>();
    let c = unsafe { c.entity_view(&world) };
    assert!(!c.has(flecs::DontFragment::ID));
    assert!(!c.has((flecs::OnInstantiate::ID, flecs::Wildcard::ID)));
}

#[test]
fn no_traits_implicit() {
    let world = World::new();

    let e = world.entity().set(TraitNone { x: 10.0, y: 20.0 });
    assert!(e.has(TraitNone::id()));

    let c = world.entity_from_id(TraitNone::entity_id(&world));
    assert!(!c.has(flecs::DontFragment::ID));
    assert!(!c.has((flecs::OnInstantiate::ID, flecs::Wildcard::ID)));
}

#[test]
fn sparse_explicit() {
    let world = World::new();

    let c = world.component::<TraitSparse>();
    let c = unsafe { c.entity_view(&world) };
    assert!(c.has(flecs::Sparse::ID));
    assert!(!c.has(flecs::DontFragment::ID));
    assert!(!c.has((flecs::OnInstantiate::ID, flecs::Wildcard::ID)));
}

#[test]
fn sparse_implicit() {
    let world = World::new();

    let e = world.entity().set(TraitSparse { x: 10.0, y: 20.0 });
    assert!(e.has(TraitSparse::id()));

    let c = world.entity_from_id(TraitSparse::entity_id(&world));
    assert!(c.has(flecs::Sparse::ID));
    assert!(!c.has(flecs::DontFragment::ID));
}

// C++ sparse_specialized uses a template specialization of flecs::sparse;
// covered by the derive attribute tests above.

#[test]
fn sparse_get_get_mut() {
    let world = World::new();

    let e = world.entity().set(TraitSparse { x: 10.0, y: 20.0 });

    {
        let v = e.get_ref::<&TraitSparse>().unwrap();
        assert_eq!(v.x, 10.0);
        assert_eq!(v.y, 20.0);
    };

    {
        let mut mv = e.get_ref::<&mut TraitSparse>().unwrap();
        mv.x = 30.0;
        mv.y = 40.0;
    };

    let found = e.get_ref::<&TraitSparse>().map(|p| {
        assert_eq!(p.x, 30.0);
        assert_eq!(p.y, 40.0);
    });
    assert!(found.is_some());
}

#[test]
fn sparse_try_get_not_found() {
    let world = World::new();

    world.component::<TraitSparse>();

    let e = world.entity();
    assert!(e.get_ref::<&TraitSparse>().is_none());
    assert!(e.get_ref::<&mut TraitSparse>().is_none());
}

#[test]
fn dont_fragment_get_get_mut() {
    let world = World::new();

    let e = world.entity().set(TraitDontFragment { x: 10.0, y: 20.0 });

    {
        let v = e.get_ref::<&TraitDontFragment>().unwrap();
        assert_eq!(v.x, 10.0);
        assert_eq!(v.y, 20.0);
    };

    {
        let mut mv = e.get_ref::<&mut TraitDontFragment>().unwrap();
        mv.x = 30.0;
        mv.y = 40.0;
    };

    let found = e.get_ref::<&TraitDontFragment>().map(|p| {
        assert_eq!(p.x, 30.0);
        assert_eq!(p.y, 40.0);
    });
    assert!(found.is_some());

    assert!(
        world
            .entity()
            .get_ref::<&TraitDontFragment>()
            .is_none()
    );
}

#[test]
fn on_instantiate_override_get_get_mut() {
    let world = World::new();

    let e = world
        .entity()
        .set(TraitOnInstantiateOverride { x: 10.0, y: 20.0 });

    {
        let v = e.get_ref::<&TraitOnInstantiateOverride>().unwrap();
        assert_eq!(v.x, 10.0);
        assert_eq!(v.y, 20.0);
    };

    {
        let mut mv = e.get_ref::<&mut TraitOnInstantiateOverride>().unwrap();
        mv.x = 30.0;
        mv.y = 40.0;
    };

    let found = e.get_ref::<&TraitOnInstantiateOverride>().map(|p| {
        assert_eq!(p.x, 30.0);
        assert_eq!(p.y, 40.0);
    });
    assert!(found.is_some());

    assert!(
        world
            .entity()
            .get_ref::<&TraitOnInstantiateOverride>()
            .is_none()
    );
}

#[test]
fn on_instantiate_dont_inherit_get_get_mut() {
    let world = World::new();

    let e = world
        .entity()
        .set(TraitOnInstantiateDontInherit { x: 10.0, y: 20.0 });

    {
        let v = e.get_ref::<&TraitOnInstantiateDontInherit>().unwrap();
        assert_eq!(v.x, 10.0);
        assert_eq!(v.y, 20.0);
    };

    {
        let mut mv = e.get_ref::<&mut TraitOnInstantiateDontInherit>().unwrap();
        mv.x = 30.0;
        mv.y = 40.0;
    };

    let found = e.get_ref::<&TraitOnInstantiateDontInherit>().map(|p| {
        assert_eq!(p.x, 30.0);
        assert_eq!(p.y, 40.0);
    });
    assert!(found.is_some());
}

#[test]
fn on_instantiate_inherit_get_inherited() {
    let world = World::new();

    let base = world
        .prefab()
        .set(TraitOnInstantiateInherit { x: 10.0, y: 20.0 });
    let inst = world.entity().is_a(base);

    let found = inst.get_ref::<&TraitOnInstantiateInherit>().map(|p| {
        assert_eq!(p.x, 10.0);
        assert_eq!(p.y, 20.0);
    });
    assert!(found.is_some());

    assert!(inst.get_ref::<&mut TraitOnInstantiateInherit>().is_none());
}

#[test]
fn no_traits_get_inherited() {
    let world = World::new();

    world
        .component::<TraitNone>()
        .add_trait::<(flecs::OnInstantiate, flecs::Inherit)>();

    let base = world.prefab().set(TraitNone { x: 10.0, y: 20.0 });
    let inst = world.entity().is_a(base);

    let found = inst.get_ref::<&TraitNone>().map(|p| {
        assert_eq!(p.x, 10.0);
        assert_eq!(p.y, 20.0);
    });
    assert!(found.is_some());

    assert!(inst.get_ref::<&mut TraitNone>().is_none());
}

// C++ get_dispatch checks compile-time type traits used for get dispatch;
// Rust dispatches at runtime, covered by the get/get_mut tests above.

#[test]
fn dynamic_inherit_dense_owned() {
    let world = World::new();

    world
        .component::<TraitNone>()
        .add_trait::<(flecs::OnInstantiate, flecs::Inherit)>();

    let e = world.entity().set(TraitNone { x: 10.0, y: 20.0 });

    let found = e.get_ref::<&TraitNone>().map(|p| {
        assert_eq!(p.x, 10.0);
        assert_eq!(p.y, 20.0);
    });
    assert!(found.is_some());

    let found = e.get_ref::<&mut TraitNone>().map(|mut mp| {
        mp.x = 30.0;
    });
    assert!(found.is_some());
    {
        let p = e.get_ref::<&TraitNone>().unwrap();
        assert_eq!(p.x, 30.0);
    };
}

#[test]
fn dynamic_inherit_dense_inherited() {
    let world = World::new();

    world
        .component::<TraitNone>()
        .add_trait::<(flecs::OnInstantiate, flecs::Inherit)>();

    let base = world.prefab().set(TraitNone { x: 10.0, y: 20.0 });
    let inst = world.entity().is_a(base);

    let found = inst.get_ref::<&TraitNone>().map(|p| {
        assert_eq!(p.x, 10.0);
        assert_eq!(p.y, 20.0);
    });
    assert!(found.is_some());

    assert!(inst.get_ref::<&mut TraitNone>().is_none());
}

#[test]
fn static_inherit_dense_owned() {
    let world = World::new();

    let c = world.component::<TraitOnInstantiateInherit>();
    assert!(
        unsafe { c.entity_view(&world) }
            .has((flecs::OnInstantiate::ID, flecs::Inherit::ID))
    );

    let e = world
        .entity()
        .set(TraitOnInstantiateInherit { x: 10.0, y: 20.0 });

    let found = e.get_ref::<&TraitOnInstantiateInherit>().map(|p| {
        assert_eq!(p.x, 10.0);
        assert_eq!(p.y, 20.0);
    });
    assert!(found.is_some());

    let found = e.get_ref::<&mut TraitOnInstantiateInherit>().map(|mut mp| {
        mp.x = 30.0;
    });
    assert!(found.is_some());
    {
        let p = e.get_ref::<&TraitOnInstantiateInherit>().unwrap();
        assert_eq!(p.x, 30.0);
    };
}

#[test]
fn static_inherit_dense_inherited() {
    let world = World::new();

    let base = world
        .prefab()
        .set(TraitOnInstantiateInherit { x: 10.0, y: 20.0 });
    let inst = world.entity().is_a(base);

    let found = inst.get_ref::<&TraitOnInstantiateInherit>().map(|p| {
        assert_eq!(p.x, 10.0);
        assert_eq!(p.y, 20.0);
    });
    assert!(found.is_some());

    assert!(inst.get_ref::<&mut TraitOnInstantiateInherit>().is_none());
}

#[test]
fn static_inherit_sparse_owned() {
    let world = World::new();

    let c = world.component::<TraitSparseInherit>();
    let cv = unsafe { c.entity_view(&world) };
    assert!(cv.has(flecs::Sparse::ID));
    assert!(cv.has((flecs::OnInstantiate::ID, flecs::Inherit::ID)));

    let e = world.entity().set(TraitSparseInherit { x: 10.0, y: 20.0 });

    let found = e.get_ref::<&TraitSparseInherit>().map(|p| {
        assert_eq!(p.x, 10.0);
        assert_eq!(p.y, 20.0);
    });
    assert!(found.is_some());

    let found = e.get_ref::<&mut TraitSparseInherit>().map(|mut mp| {
        mp.x = 30.0;
    });
    assert!(found.is_some());
    {
        let p = e.get_ref::<&TraitSparseInherit>().unwrap();
        assert_eq!(p.x, 30.0);
    };
}

#[test]
fn static_inherit_sparse_inherited() {
    let world = World::new();

    let base = world.prefab().set(TraitSparseInherit { x: 10.0, y: 20.0 });
    let inst = world.entity().is_a(base);

    let found = inst.get_ref::<&TraitSparseInherit>().map(|p| {
        assert_eq!(p.x, 10.0);
        assert_eq!(p.y, 20.0);
    });
    assert!(found.is_some());

    assert!(inst.get_ref::<&mut TraitSparseInherit>().is_none());
}

#[test]
fn static_inherit_dont_fragment_owned() {
    let world = World::new();

    let c = world.component::<TraitDontFragmentInherit>();
    let cv = unsafe { c.entity_view(&world) };
    assert!(cv.has(flecs::DontFragment::ID));
    assert!(cv.has((flecs::OnInstantiate::ID, flecs::Inherit::ID)));

    let e = world
        .entity()
        .set(TraitDontFragmentInherit { x: 10.0, y: 20.0 });

    let found = e.get_ref::<&TraitDontFragmentInherit>().map(|p| {
        assert_eq!(p.x, 10.0);
        assert_eq!(p.y, 20.0);
    });
    assert!(found.is_some());

    let found = e.get_ref::<&mut TraitDontFragmentInherit>().map(|mut mp| {
        mp.x = 30.0;
    });
    assert!(found.is_some());
    {
        let p = e.get_ref::<&TraitDontFragmentInherit>().unwrap();
        assert_eq!(p.x, 30.0);
    };
}

#[test]
fn static_inherit_dont_fragment_inherited() {
    let world = World::new();

    let base = world
        .prefab()
        .set(TraitDontFragmentInherit { x: 10.0, y: 20.0 });
    let inst = world.entity().is_a(base);

    let found = inst.get_ref::<&TraitDontFragmentInherit>().map(|p| {
        assert_eq!(p.x, 10.0);
        assert_eq!(p.y, 20.0);
    });
    assert!(found.is_some());

    assert!(inst.get_ref::<&mut TraitDontFragmentInherit>().is_none());
}

// Like C++, the compile-time sparse get fast path cannot handle dynamically
// added (OnInstantiate, Inherit); these tests expect the resulting abort.
// The abort comes from an ecs_check, which compiles out under NDEBUG, so
// these only run against a debug C build.
#[test]
#[should_panic]
#[cfg(debug_assertions)]
fn dynamic_inherit_sparse_owned() {
    let _guard = FlecsPanicAbortGuard::install();
    let world = World::new();

    world
        .component::<TraitSparse>()
        .add_trait::<(flecs::OnInstantiate, flecs::Inherit)>();

    let e = world.entity().set(TraitSparse { x: 10.0, y: 20.0 });

    {
        let _ = e.get_ref::<&TraitSparse>().unwrap();
    };
}

#[test]
#[should_panic]
#[cfg(debug_assertions)]
fn dynamic_inherit_sparse_owned_get_mut() {
    let _guard = FlecsPanicAbortGuard::install();
    let world = World::new();

    world
        .component::<TraitSparse>()
        .add_trait::<(flecs::OnInstantiate, flecs::Inherit)>();

    let e = world.entity().set(TraitSparse { x: 10.0, y: 20.0 });

    {
        let _ = e.get_ref::<&mut TraitSparse>().unwrap();
    };
}

#[test]
#[should_panic]
#[cfg(debug_assertions)]
fn dynamic_inherit_sparse_inherited() {
    let _guard = FlecsPanicAbortGuard::install();
    let world = World::new();

    world
        .component::<TraitSparse>()
        .add_trait::<(flecs::OnInstantiate, flecs::Inherit)>();

    let base = world.prefab().set(TraitSparse { x: 10.0, y: 20.0 });
    let inst = world.entity().is_a(base);

    {
        let _ = inst.get_ref::<&TraitSparse>().unwrap();
    };
}

#[test]
#[should_panic]
#[cfg(debug_assertions)]
fn dynamic_inherit_dont_fragment_owned() {
    let _guard = FlecsPanicAbortGuard::install();
    let world = World::new();

    world
        .component::<TraitDontFragment>()
        .add_trait::<(flecs::OnInstantiate, flecs::Inherit)>();

    let e = world.entity().set(TraitDontFragment { x: 10.0, y: 20.0 });

    {
        let _ = e.get_ref::<&TraitDontFragment>().unwrap();
    };
}

#[test]
#[should_panic]
#[cfg(debug_assertions)]
fn dynamic_inherit_dont_fragment_owned_get_mut() {
    let _guard = FlecsPanicAbortGuard::install();
    let world = World::new();

    world
        .component::<TraitDontFragment>()
        .add_trait::<(flecs::OnInstantiate, flecs::Inherit)>();

    let e = world.entity().set(TraitDontFragment { x: 10.0, y: 20.0 });

    {
        let _ = e.get_ref::<&mut TraitDontFragment>().unwrap();
    };
}

#[test]
#[should_panic]
#[cfg(debug_assertions)]
fn dynamic_inherit_dont_fragment_inherited() {
    let _guard = FlecsPanicAbortGuard::install();
    let world = World::new();

    world
        .component::<TraitDontFragment>()
        .add_trait::<(flecs::OnInstantiate, flecs::Inherit)>();

    let base = world.prefab().set(TraitDontFragment { x: 10.0, y: 20.0 });
    let inst = world.entity().is_a(base);

    {
        let _ = inst.get_ref::<&TraitDontFragment>().unwrap();
    };
}
