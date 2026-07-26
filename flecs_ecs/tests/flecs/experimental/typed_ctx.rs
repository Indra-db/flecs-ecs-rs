//! SW-3 deliverable 3: typed context on systems and the world (spec §5.3).
//!
//! Covers round-trip read, wrong-type `None`, `ctx_mut` mutation, and that the
//! stored value is dropped with the owner (no leak).

use core::sync::atomic::{AtomicUsize, Ordering};

use flecs_ecs::core::*;
use flecs_ecs::macros::*;

#[derive(Component)]
struct Tracked(i32);

struct DropCounter(&'static AtomicUsize);

impl Drop for DropCounter {
    fn drop(&mut self) {
        self.0.fetch_add(1, Ordering::Relaxed);
    }
}

#[test]
fn system_ctx_roundtrip_and_wrong_type() {
    let world = World::new();
    let mut system = world.system::<&Tracked>().ctx(7u32).each(|_| {});

    assert_eq!(system.ctx::<u32>(), Some(&7));
    // wrong type -> None, not UB
    assert_eq!(system.ctx::<i8>(), None);

    if let Some(v) = system.ctx_mut::<u32>() {
        *v = 99;
    }
    assert_eq!(system.ctx::<u32>(), Some(&99));
}

#[test]
fn system_without_ctx_is_none() {
    let world = World::new();
    let system = world.system::<&Tracked>().each(|_| {});
    assert_eq!(system.ctx::<u32>(), None);
}

#[test]
fn world_ctx_roundtrip_and_wrong_type() {
    let mut world = World::new();
    world.set_ctx(42u32);

    assert_eq!(world.ctx::<u32>(), Some(&42));
    assert_eq!(world.ctx::<i8>(), None);

    if let Some(v) = world.ctx_mut::<u32>() {
        *v += 1;
    }
    assert_eq!(world.ctx::<u32>(), Some(&43));
}

#[test]
fn system_ctx_dropped_with_world() {
    static DROPS: AtomicUsize = AtomicUsize::new(0);
    DROPS.store(0, Ordering::Relaxed);

    {
        let world = World::new();
        world
            .system::<&Tracked>()
            .ctx(DropCounter(&DROPS))
            .each(|_| {});
        assert_eq!(DROPS.load(Ordering::Relaxed), 0);
    }

    assert_eq!(
        DROPS.load(Ordering::Relaxed),
        1,
        "the system's typed ctx must be dropped exactly once with the world"
    );
}

#[test]
fn system_ctx_overwrite_drops_previous() {
    static DROPS: AtomicUsize = AtomicUsize::new(0);
    DROPS.store(0, Ordering::Relaxed);

    let world = World::new();
    world
        .system::<&Tracked>()
        .ctx(DropCounter(&DROPS))
        .ctx(DropCounter(&DROPS))
        .each(|_| {});

    // The first ctx value was dropped when the second replaced it.
    assert_eq!(DROPS.load(Ordering::Relaxed), 1);
    drop(world);
    assert_eq!(DROPS.load(Ordering::Relaxed), 2);
}

#[test]
fn world_ctx_dropped_with_world() {
    static DROPS: AtomicUsize = AtomicUsize::new(0);
    DROPS.store(0, Ordering::Relaxed);

    {
        let mut world = World::new();
        world.set_ctx(DropCounter(&DROPS));
        assert_eq!(DROPS.load(Ordering::Relaxed), 0);
    }

    assert_eq!(DROPS.load(Ordering::Relaxed), 1);
}
