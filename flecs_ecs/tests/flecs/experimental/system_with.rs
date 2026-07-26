//! SW-3 deliverable 2: the system `_with` terminals and their `Stage` handle
//! (spec §5.2, §6.1, §7.3).
//!
//! Covers: `each_with` / `each_entity_with` deliver the tuple and a `Stage`;
//! commands issued through the stage route through the flecs defer queue and
//! apply at the sync point, not mid-frame; and a `_with` system always registers
//! its term locks (a guard taken inside the callback on the system's own write
//! term conflicts).

use core::sync::atomic::{AtomicBool, AtomicI32, Ordering};

use flecs_ecs::core::*;
use flecs_ecs::macros::*;

#[derive(Component)]
struct Value(i32);

#[derive(Component)]
struct Applied;

#[derive(Component)]
struct Spawned(i32);

/// `each_with` hands the component tuple and a `Stage`; a deferred `set` through
/// the stage applies after the frame.
#[test]
fn each_with_delivers_stage_and_defers() {
    let mut world = World::new();
    let e = world.entity().set(Value(0)).id();

    world.system::<&Value>().each_with(move |v, stage| {
        // read term sees the pre-frame value
        assert_eq!(v.0, 0);
        // deferred command on the iterated entity, issued through the stage
        stage.entity_view(e).set(Value(v.0 + 42));
    });

    world.progress();

    world
        .entity_from_id(e)
        .get::<&Value>(|v| assert_eq!(v.0, 42));
}

/// `each_entity_with` also hands the row's `EntityView`; a deferred structural
/// `add` is not visible mid-frame and applies at the sync point.
#[test]
fn each_entity_with_defers_structural_op_to_sync() {
    let mut world = World::new();
    let e = world.entity().set(Value(1)).id();

    world.system::<&Value>().each_entity_with(|entity, _v, stage| {
        // structural add is deferred: not applied while the frame runs
        stage.entity_view(entity.id()).add(Applied::id());
        assert!(
            !entity.has(Applied::id()),
            "deferred add must not be visible mid-frame"
        );
    });

    world.progress();

    assert!(
        world.entity_from_id(e).has(Applied::id()),
        "deferred add must apply at the sync point"
    );
}

/// `Stage::spawn` deferred-creates an entity and enqueues its bundle; it exists
/// after the sync point.
#[test]
fn stage_spawn_defers_new_entity() {
    let mut world = World::new();
    world.entity().set(Value(0));

    world.system::<&Value>().each_with(|_v, stage| {
        stage.entity().set(Spawned(7));
    });

    world.progress();

    let mut seen = 0;
    world.new_query::<&Spawned>().each(|s| {
        assert_eq!(s.0, 7);
        seen += 1;
    });
    assert_eq!(seen, 1, "the stage-spawned entity applied at the sync point");
}

/// `Stage::delta_time` reflects the frame timestep.
#[test]
fn stage_delta_time_visible() {
    let mut world = World::new();
    world.entity().set(Value(0));

    static SEEN: AtomicI32 = AtomicI32::new(0);
    SEEN.store(0, Ordering::Relaxed);

    world.system::<&Value>().each_with(|_v, stage| {
        // 0.25s frame: store as milliseconds to compare without floats crossing
        // the atomic boundary.
        SEEN.store((stage.delta_time() * 1000.0) as i32, Ordering::Relaxed);
    });

    world.progress_time(0.25);
    assert_eq!(SEEN.load(Ordering::Relaxed), 250);
}

/// A `_with` system always registers its term locks (spec §5.2): a read guard
/// taken inside the callback on the system's own `&mut` write term conflicts,
/// proving the write borrow was registered for the batch. The conflict is caught
/// inside the callback so nothing unwinds through the C pipeline frame.
#[test]
fn each_entity_with_registers_write_term_lock() {
    let mut world = World::new();
    world.entity().set(Value(0));

    static CONFLICTED: AtomicBool = AtomicBool::new(false);
    CONFLICTED.store(false, Ordering::Relaxed);

    world
        .system::<&mut Value>()
        .each_entity_with(|entity, _v, _stage| {
            let result = std::panic::catch_unwind(core::panic::AssertUnwindSafe(|| {
                // read guard on Value vs the batch's write term on Value
                entity.get::<&Value>(|_| {});
            }));
            if result.is_err() {
                CONFLICTED.store(true, Ordering::Relaxed);
            }
        });

    world.progress();

    assert!(
        CONFLICTED.load(Ordering::Relaxed),
        "a read guard on the system's own write term must conflict with the registered batch lock"
    );
}

/// A guard on a component the system does not name must NOT false-positive: the
/// `_with` system registers only its own terms.
#[test]
fn each_entity_with_disjoint_guard_ok() {
    let mut world = World::new();
    world.entity().set(Value(0)).set(Spawned(0));

    world
        .system::<&mut Value>()
        .each_entity_with(|entity, _v, _stage| {
            // Spawned is disjoint from the Value write term: no conflict.
            entity.get::<&mut Spawned>(|s| s.0 += 1);
        });

    world.progress();
    world
        .new_query::<&Spawned>()
        .each(|s| assert_eq!(s.0, 1));
}
