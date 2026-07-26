use core::panic::AssertUnwindSafe;
use core::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::panic::catch_unwind;
use alloc::sync::Arc;

use flecs_ecs::core::*;
use flecs_ecs::macros::*;
use flecs_ecs::experimental::{QueryExclusiveExt, QuerySharedExt};
use flecs_ecs::experimental::prelude::{EntityGuardExt, WorldEntityRefExt};

#[derive(Component)]
struct Foo(i32);

#[derive(Component)]
struct Bar(i32);

/// Component whose `Drop` bumps a shared counter, so a test can assert a
/// panicking iteration dropped each instance exactly once (no double-drop, no
/// leak). Holds an `Arc` so the counter is per-test, avoiding cross-test races
/// under the parallel test runner.
#[derive(Component)]
struct Tracked {
    counter: Arc<AtomicUsize>,
}

impl Drop for Tracked {
    fn drop(&mut self) {
        self.counter.fetch_add(1, Ordering::Relaxed);
    }
}

/// Extract the `&str` message from a caught panic payload.
fn panic_message(payload: &(dyn core::any::Any + Send)) -> &str {
    payload
        .downcast_ref::<&str>()
        .copied()
        .or_else(|| payload.downcast_ref::<String>().map(String::as_str))
        .unwrap_or("<non-string panic payload>")
}

const TRACKED_COUNT: usize = 6;

/// Populate `count` entities carrying a `Tracked` (and a `Foo` for querying),
/// spread over a few tables so a per-batch trampoline sees more than one batch.
fn spawn_tracked(world: &World, counter: &Arc<AtomicUsize>, count: usize) {
    for i in 0..count {
        let e = world.entity().set(Foo(i as i32)).set(Tracked {
            counter: Arc::clone(counter),
        });
        if i % 2 == 0 {
            e.set(Bar(0));
        }
    }
}

#[test]
fn entity_get_panic_releases_safety_scope() {
    let world = World::new();
    let entity = world.entity().set(Foo(1));

    let result = catch_unwind(AssertUnwindSafe(|| {
        let _foo = entity.get_ref::<&mut Foo>().unwrap();
        panic!("expected");
    }));

    assert!(result.is_err());
    {
        let mut foo = entity.get_ref::<&mut Foo>().unwrap();
        foo.0 += 1;
    }
    {
        let foo = entity.get_ref::<&Foo>().unwrap();
        assert_eq!(foo.0, 2);
    }
}

#[test]
fn cached_ref_panic_releases_safety_scope() {
    let world = World::new();
    let entity = world.entity().set(Foo(1));
    let mut cached = world.entity_ref::<Foo>(entity.id()).unwrap();

    let result = catch_unwind(AssertUnwindSafe(|| {
        let _g = cached.get_mut(&world).unwrap();
        panic!("expected");
    }));

    assert!(result.is_err());
    {
        let mut foo = cached.get_mut(&world).unwrap();
        foo.0 += 1;
    }
    {
        let foo = cached.get(&world).unwrap();
        assert_eq!(foo.0, 2);
    }
}

#[test]
fn query_panic_releases_safety_scope() {
    let world = World::new();
    world.entity().set(Foo(1));
    let query = world.new_query::<&mut Foo>();

    let result = catch_unwind(AssertUnwindSafe(|| {
        query.each_shared(&world, |_| panic!("expected"));
    }));

    assert!(result.is_err());
    query.each_shared(&world, |foo| foo.0 += 1);
    query.each_shared(&world, |foo| assert_eq!(foo.0, 2));
}

#[test]
fn partial_tuple_acquisition_rolls_back_prior_keys() {
    let world = World::new();
    let entity = world.entity().set(Foo(1)).set(Bar(1));

    {
        let _foo = entity.get_ref::<&mut Foo>().unwrap();
        let result = catch_unwind(AssertUnwindSafe(|| {
            let _ = entity.get_ref::<(&mut Bar, &mut Foo)>();
        }));

        assert!(result.is_err());
        {
            let mut bar = entity.get_ref::<&mut Bar>().unwrap();
            bar.0 += 1;
        }
    }

    {
        let bar = entity.get_ref::<&Bar>().unwrap();
        assert_eq!(bar.0, 2);
    }
}

/// Assert the world is still usable after a caught trampoline panic: the stage
/// lock map is balanced (a fresh mutable query does not spuriously conflict),
/// entity creation works, and `progress` runs again cleanly.
fn assert_world_usable(world: &mut World) {
    let mut seen = 0u32;
    world.new_query::<&mut Foo>().each_exclusive(world, |f| {
        f.0 += 1;
        seen += 1;
    });
    assert!(seen > 0, "query over surviving entities produced no rows");
    world.entity().set(Foo(123));
    world.progress();
}

/// Panic in a `system().each` callback: caught at the trampoline, rethrown from
/// `progress`, world usable afterwards, no double-drop.
#[test]
fn system_each_panic_surfaces_from_progress() {
    let counter = Arc::new(AtomicUsize::new(0));
    {
        let mut world = World::new();
        spawn_tracked(&world, &counter, TRACKED_COUNT);

        let armed = Arc::new(AtomicBool::new(true));
        let a = Arc::clone(&armed);
        world.system::<&Foo>().each(move |_| {
            if a.swap(false, Ordering::Relaxed) {
                panic!("boom each");
            }
        });

        let result = catch_unwind(AssertUnwindSafe(|| {
            world.progress();
        }));
        let payload = result.expect_err("progress should rethrow the callback panic");
        assert_eq!(panic_message(&*payload), "boom each");

        assert_world_usable(&mut world);
    }
    assert_eq!(counter.load(Ordering::Relaxed), TRACKED_COUNT);
}

/// Panic in a `system().each_entity` callback.
#[test]
fn system_each_entity_panic_surfaces_from_progress() {
    let counter = Arc::new(AtomicUsize::new(0));
    {
        let mut world = World::new();
        spawn_tracked(&world, &counter, TRACKED_COUNT);

        let armed = Arc::new(AtomicBool::new(true));
        let a = Arc::clone(&armed);
        world.system::<&Foo>().each_entity(move |_, _| {
            if a.swap(false, Ordering::Relaxed) {
                panic!("boom each_entity");
            }
        });

        let result = catch_unwind(AssertUnwindSafe(|| {
            world.progress();
        }));
        let payload = result.expect_err("progress should rethrow the callback panic");
        assert_eq!(panic_message(&*payload), "boom each_entity");

        assert_world_usable(&mut world);
    }
    assert_eq!(counter.load(Ordering::Relaxed), TRACKED_COUNT);
}

/// Panic in a `system().each_iter` callback.
#[test]
fn system_each_iter_panic_surfaces_from_progress() {
    let counter = Arc::new(AtomicUsize::new(0));
    {
        let mut world = World::new();
        spawn_tracked(&world, &counter, TRACKED_COUNT);

        let armed = Arc::new(AtomicBool::new(true));
        let a = Arc::clone(&armed);
        world.system::<&Foo>().each_iter(move |_, _, _| {
            if a.swap(false, Ordering::Relaxed) {
                panic!("boom each_iter");
            }
        });

        let result = catch_unwind(AssertUnwindSafe(|| {
            world.progress();
        }));
        let payload = result.expect_err("progress should rethrow the callback panic");
        assert_eq!(panic_message(&*payload), "boom each_iter");

        assert_world_usable(&mut world);
    }
    assert_eq!(counter.load(Ordering::Relaxed), TRACKED_COUNT);
}

/// Panic in a `system().each_with` callback (`Stage`-carrying terminal).
#[test]
fn system_each_with_panic_surfaces_from_progress() {
    let counter = Arc::new(AtomicUsize::new(0));
    {
        let mut world = World::new();
        spawn_tracked(&world, &counter, TRACKED_COUNT);

        let armed = Arc::new(AtomicBool::new(true));
        let a = Arc::clone(&armed);
        world.system::<&Foo>().each_with(move |_, _stage| {
            if a.swap(false, Ordering::Relaxed) {
                panic!("boom each_with");
            }
        });

        let result = catch_unwind(AssertUnwindSafe(|| {
            world.progress();
        }));
        let payload = result.expect_err("progress should rethrow the callback panic");
        assert_eq!(panic_message(&*payload), "boom each_with");

        assert_world_usable(&mut world);
    }
    assert_eq!(counter.load(Ordering::Relaxed), TRACKED_COUNT);
}

/// Panic in a `system().run` callback.
#[test]
fn system_run_panic_surfaces_from_progress() {
    let counter = Arc::new(AtomicUsize::new(0));
    {
        let mut world = World::new();
        spawn_tracked(&world, &counter, TRACKED_COUNT);

        let armed = Arc::new(AtomicBool::new(true));
        let a = Arc::clone(&armed);
        world.system::<&Foo>().run(move |mut it| {
            while it.next() {
                it.each();
            }
            if a.swap(false, Ordering::Relaxed) {
                panic!("boom run");
            }
        });

        let result = catch_unwind(AssertUnwindSafe(|| {
            world.progress();
        }));
        let payload = result.expect_err("progress should rethrow the callback panic");
        assert_eq!(panic_message(&*payload), "boom run");

        assert_world_usable(&mut world);
    }
    assert_eq!(counter.load(Ordering::Relaxed), TRACKED_COUNT);
}

/// Panic in a `par_each` callback on a worker thread: it must not cross into C
/// on that worker; the payload is stashed under the world-context mutex and
/// rethrown by `progress` on the main thread.
#[test]
fn system_par_each_panic_surfaces_from_progress_multithreaded() {
    let counter = Arc::new(AtomicUsize::new(0));
    {
        let mut world = World::new();
        // Enough entities across tables that several workers get partitions.
        spawn_tracked(&world, &counter, 400);
        world.set_threads(4);

        let armed = Arc::new(AtomicBool::new(true));
        let a = Arc::clone(&armed);
        world.system::<&Foo>().par_each(move |_| {
            if a.swap(false, Ordering::Relaxed) {
                panic!("boom par_each");
            }
        });

        let result = catch_unwind(AssertUnwindSafe(|| {
            world.progress();
        }));
        let payload = result.expect_err("progress should rethrow the worker panic");
        assert_eq!(panic_message(&*payload), "boom par_each");

        // World still usable after a worker panic: run the (now-disarmed)
        // parallel system again and a fresh query.
        world.progress();
        let mut seen = 0u32;
        world.new_query::<&Foo>().each_shared(&world, |_| seen += 1);
        assert!(seen > 0);
    }
    assert_eq!(counter.load(Ordering::Relaxed), 400);
}

/// Panic in an observer callback: caught at the observer trampoline, rethrown
/// from the `emit` entry point.
#[test]
fn observer_panic_surfaces_from_emit() {
    #[derive(Component)]
    struct Ping;

    let counter = Arc::new(AtomicUsize::new(0));
    {
        let world = World::new();
        let e = world.entity().set(Foo(1)).set(Tracked {
            counter: Arc::clone(&counter),
        });

        let armed = Arc::new(AtomicBool::new(true));
        let a = Arc::clone(&armed);
        world.observer::<Ping, &Foo>().each(move |_| {
            if a.swap(false, Ordering::Relaxed) {
                panic!("boom observer");
            }
        });

        let result = catch_unwind(AssertUnwindSafe(|| {
            world.event::<Ping>().add(Foo::id()).entity(e).emit(&Ping);
        }));
        let payload = result.expect_err("emit should rethrow the observer panic");
        assert_eq!(panic_message(&*payload), "boom observer");

        // World usable: a fresh mutable query does not spuriously conflict.
        let mut seen = 0u32;
        world.new_query::<&mut Foo>().each_shared(&world, |foo| {
            foo.0 += 1;
            seen += 1;
        });
        assert_eq!(seen, 1);
        world.entity().set(Foo(9));
    }
    assert_eq!(counter.load(Ordering::Relaxed), 1);
}

/// Panic in an `order_by` comparator (a ZST Rust closure invoked from flecs'
/// sort). The comparator holds no stage borrow, so its panic needs no lock
/// rollback; it unwinds through the retained `C-unwind` sort path and surfaces
/// at the query iteration entry point, leaving the world usable.
#[test]
fn order_by_comparator_panic_surfaces_from_iteration() {
    let counter = Arc::new(AtomicUsize::new(0));
    {
        let world = World::new();
        spawn_tracked(&world, &counter, TRACKED_COUNT);

        // Sorting (and thus the comparator) runs when the ordered query is
        // materialised, so build and iterate inside the same catch.
        let result = catch_unwind(AssertUnwindSafe(|| {
            let query = world
                .query::<&Foo>()
                .order_by::<Foo>(|_e1, _a: &Foo, _e2, _b: &Foo| panic!("boom order_by"))
                .build();
            query.each_shared(&world, |_| {});
        }));
        let payload = result.expect_err("iteration should surface the comparator panic");
        assert_eq!(panic_message(&*payload), "boom order_by");

        // World usable afterwards: an unsorted query iterates fine.
        let mut seen = 0u32;
        world.new_query::<&mut Foo>().each_shared(&world, |foo| {
            foo.0 += 1;
            seen += 1;
        });
        assert_eq!(seen, TRACKED_COUNT as u32);
    }
    assert_eq!(counter.load(Ordering::Relaxed), TRACKED_COUNT);
}
