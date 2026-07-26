#![allow(dead_code)]
use crate::common_test::*;

#[test]
fn script_ast_can_be_called_multiple_times() {
    let world = World::new();

    let mut script =
        Script::parse(&world, "test_script", "ent_a {}", None).expect("script should parse");

    let first = script.ast();
    assert!(first.is_some());

    let second = script.ast();
    assert_eq!(first, second);

    let third = script.ast();
    assert_eq!(first, third);
}

#[test]
fn script_builder_reuse_file_then_code() {
    let world = World::new();

    let path = std::env::temp_dir().join(format!(
        "flecs_rust_script_builder_reuse_file_then_code_{}.flecs",
        std::process::id()
    ));
    std::fs::write(&path, "file_ent {}").unwrap();

    let mut builder = world.script();

    let from_file = builder.build_from_file(path.to_str().unwrap());
    assert_ne!(*from_file.id(), 0);
    assert!(world.try_lookup("file_ent").is_some());

    let from_code = builder.build_from_code("code_ent {}");
    assert_ne!(*from_code.id(), 0);
    assert!(world.try_lookup("code_ent").is_some());

    std::fs::remove_file(&path).ok();
}

#[test]
fn script_builder_reuse_code_then_file() {
    let world = World::new();

    let path = std::env::temp_dir().join(format!(
        "flecs_rust_script_builder_reuse_code_then_file_{}.flecs",
        std::process::id()
    ));
    std::fs::write(&path, "file_ent2 {}").unwrap();

    let mut builder = world.script();

    let from_code = builder.build_from_code("code_ent2 {}");
    assert_ne!(*from_code.id(), 0);
    assert!(world.try_lookup("code_ent2").is_some());

    let from_file = builder.build_from_file(path.to_str().unwrap());
    assert_ne!(*from_file.id(), 0);
    assert!(world.try_lookup("file_ent2").is_some());

    std::fs::remove_file(&path).ok();
}

#[test]
fn alert_builder_fallible_build() {
    let world = World::new();
    world.import::<AlertsModule>();
    world.component::<Position>();

    let valid = world.alert::<&Position>().build();
    assert_ne!(*valid.id(), 0);

    let invalid = world.alert::<()>().expr("invalid syntax!!!").build();
    assert!(matches!(invalid, Err(AlertBuildError::InvalidExpr { .. })));
}

#[test]
fn system_builder_fallible_build() {
    let world = World::new();

    extern "C-unwind" fn noop_iter(_it: *mut flecs_ecs::sys::ecs_iter_t) {}

    let desc = flecs_ecs::sys::ecs_system_desc_t {
        callback: Some(noop_iter),
        ..Default::default()
    };
    let valid = world.system_builder_from_desc::<&Position>(desc).build();
    assert_ne!(*valid.id(), 0);

    let desc = flecs_ecs::sys::ecs_system_desc_t {
        callback: Some(noop_iter),
        ..Default::default()
    };
    let invalid = world
        .system_builder_from_desc::<()>(desc)
        .expr("invalid syntax!!!")
        .build();
    assert!(matches!(invalid, Err(SystemBuildError::InvalidExpr { .. })));
}

#[test]
fn observer_builder_fallible_build() {
    use flecs_ecs::core::private::internal_SystemAPI;

    let world = World::new();

    extern "C-unwind" fn noop_iter(_it: *mut flecs_ecs::sys::ecs_iter_t) {}

    let mut valid_builder = world.observer::<flecs::OnSet, &Position>();
    valid_builder.set_desc_callback(Some(noop_iter));
    assert_ne!(*valid_builder.build().id(), 0);

    let mut invalid_builder = world
        .observer::<flecs::OnSet, ()>()
        .expr("invalid syntax!!!");
    invalid_builder.set_desc_callback(Some(noop_iter));
    assert!(matches!(
        invalid_builder.build(),
        Err(ObserverBuildError::InvalidExpr { .. })
    ));
}

#[test]
fn system_observer_debug_display_eq() {
    let world = World::new();

    let sys_a = world.system::<&Position>().each(|_pos| {});
    let sys_a_again = world.system_from(*sys_a);
    let sys_b = world.system::<&Position>().each(|_pos| {});

    assert_eq!(sys_a, sys_a_again);
    assert_ne!(sys_a, sys_b);
    assert!(!format!("{sys_a:?}").is_empty());
    assert!(!format!("{sys_a}").is_empty());

    let obs_a = world.observer::<flecs::OnSet, &Position>().each(|_pos| {});
    let obs_a_again = world.observer_from(*obs_a);
    let obs_b = world.observer::<flecs::OnSet, &Position>().each(|_pos| {});

    assert_eq!(obs_a, obs_a_again);
    assert_ne!(obs_a, obs_b);
    assert!(!format!("{obs_a:?}").is_empty());
    assert!(!format!("{obs_a}").is_empty());
}
