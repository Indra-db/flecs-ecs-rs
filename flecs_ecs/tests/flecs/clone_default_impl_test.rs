#![allow(dead_code)]
use crate::common_test::FlecsPanicAbortGuard;
use flecs_ecs::core::{ComponentInfo, World};
use flecs_ecs::experimental::prelude::EntityGuardExt;
use flecs_ecs_derive::Component;

// normal structs
#[derive(Component)]
struct NoneCloneDefault;

#[derive(Default, Clone, Component)]
struct CloneDefault;

#[derive(Clone, Component)]
struct CloneNoDefault;

#[derive(Default, Component)]
struct DefaultNoClone;

// drop structs
#[derive(Component, Default)]
struct DefaultNoCloneDrop {
    _data: String,
}

#[derive(Default, Clone, Component)]
struct CloneDefaultDrop {
    data: String,
}

#[test]
fn compile_time_check_impls_clone_default() {
    // we do it this way to avoid the warning of constant bools getting optimized away from clippy in test cases.
    let none_clone_default = !NoneCloneDefault::IMPLS_CLONE && !NoneCloneDefault::IMPLS_DEFAULT;
    let clone_default = CloneDefault::IMPLS_CLONE && CloneDefault::IMPLS_DEFAULT;
    let clone_no_default = CloneNoDefault::IMPLS_CLONE && !CloneNoDefault::IMPLS_DEFAULT;
    let default_no_clone = !DefaultNoClone::IMPLS_CLONE && DefaultNoClone::IMPLS_DEFAULT;

    assert!(none_clone_default);
    assert!(clone_default);
    assert!(clone_no_default);
    assert!(default_no_clone);
}

#[test]
fn copy_hook_implemented_for_drop_types() {
    let world = World::new();
    let e_orig = world.entity().set(CloneDefaultDrop {
        data: "data".to_string(),
    });

    let entity_cloned = e_orig.duplicate(true);

    let cd = e_orig.get::<&CloneDefaultDrop>().unwrap();
    let cd_cloned = entity_cloned.get::<&CloneDefaultDrop>().unwrap();
    assert!(cd.data == cd_cloned.data);
}

#[test]
#[should_panic(expected = "Clone is not implemented for type")]
fn copy_hook_not_implemented_for_drop_types() {
    let world = World::new();
    // Guard installed AFTER World::new() so it survives the reset of abort_.
    // The Rust panic from duplicate() fires first (caught by #[should_panic]).
    // Any subsequent C abort during world cleanup is suppressed by our abort() override.
    let _guard = FlecsPanicAbortGuard::install();
    let e_orig = world.entity().set(DefaultNoCloneDrop {
        _data: "data".to_string(),
    });

    let _entity_cloned = e_orig.duplicate(true); // PANICS via Rust
}
