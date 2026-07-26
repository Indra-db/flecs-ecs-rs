//! Regression tests for soundness fixes in the Rust binding.
//!
//! Each test pins a behavior that used to be undefined behavior or a resource
//! leak reachable from safe Rust. Compile-level fixes (lifetime tightening on
//! `World::components_map`/`components_array` and the `ecs_rust_trait!`
//! `cast`/`cast_mut` signatures, removal of the `&*mut ecs_world_t` →
//! `&WorldRef` transmute) are enforced by the type system and have no runtime
//! test. The `on_replace` deferred-set semantics are covered by
//! `entity_test::defer_on_replace_w_set*`, and the `run_post_frame` /
//! `on_destroyed` callback context round-trip by `world_test`.

#![allow(dead_code)]
#![allow(unused_imports)]
use crate::common_test::*;
use flecs_ecs::prelude::*;

extern crate alloc;

use alloc::sync::Arc;
use core::cell::Cell;
use core::sync::atomic::{AtomicUsize, Ordering};

/// A zero-sized component with alignment > 1 and a Drop impl.
/// The dtor used to run `drop_in_place` through the (unaligned) C storage
/// pointer; it now drops at an aligned dangling address. This exercises the
/// register → add → remove → world-drop path.
#[test]
fn zst_with_alignment_and_drop_dtor_path() {
    thread_local! {
        static DROPPED: Cell<u32> = const { Cell::new(0) };
    }

    #[derive(Component)]
    #[repr(align(8))]
    struct AlignedZst;

    impl Drop for AlignedZst {
        fn drop(&mut self) {
            DROPPED.with(|d| d.set(d.get() + 1));
        }
    }

    let world = World::new();
    world.component::<AlignedZst>();

    let e = world.entity();
    e.add(AlignedZst);
    e.remove(AlignedZst);
    drop(world);
}

/// Two fixes pinned here:
/// - Observer free callbacks used to run `drop_in_place` on the wrong type
///   (`*mut fn()`), leaking the entire closure including its captures. They
///   now reconstruct the `Box<Func>` and drop it on observer destruction.
/// - Entity observers never received events: the `run_*` callbacks looped
///   over `iter.count`, which is 0 for observers matched on a non-$this
///   source. They now invoke the callback once per event.
#[test]
fn observer_closure_dropped_on_world_destroy() {
    #[derive(Component)]
    struct EvTag;

    let counter = Arc::new(AtomicUsize::new(0));
    let counter_clone = Arc::clone(&counter);

    {
        let world = World::new();
        let entity = world.entity().add(Tag);

        entity.observe::<EvTag>(move || {
            counter_clone.fetch_add(1, Ordering::SeqCst);
        });

        entity.emit(&EvTag);
        entity.emit(&EvTag);
    }

    assert_eq!(
        counter.load(Ordering::SeqCst),
        2,
        "entity observer must run once per emitted event"
    );
    assert_eq!(
        Arc::strong_count(&counter),
        1,
        "closure must be dropped on observer destruction (Box::from_raw in \
         on_free_empty::<Func>)"
    );
}

/// Same as above for the payload observer variant (`on_free_payload` /
/// `run_payload`).
#[test]
fn observer_payload_closure_dropped_on_world_destroy() {
    let counter = Arc::new(AtomicUsize::new(0));
    let counter_clone = Arc::clone(&counter);

    {
        let world = World::new();
        let entity = world.entity().add(Tag);

        entity.observe_payload::<Position>(move |p| {
            assert_eq!(p.x, 10);
            counter_clone.fetch_add(1, Ordering::SeqCst);
        });

        entity.emit(&Position { x: 10, y: 20 });
    }

    assert_eq!(
        counter.load(Ordering::SeqCst),
        1,
        "payload entity observer must run once per emitted event"
    );
    assert_eq!(
        Arc::strong_count(&counter),
        1,
        "payload closure must be dropped on observer destruction"
    );
}

/// A system whose query fails to parse (invalid `expr`) used to leak the
/// `each`/`run` closure: `ecs_system_init` returns 0 before flecs registers the
/// ctx-free trampoline, so `build()` must reclaim the leaked closure box itself.
#[test]
fn system_query_init_failure_reclaims_closure() {
    let world = World::new();
    let counter = Arc::new(AtomicUsize::new(0));
    let counter_clone = Arc::clone(&counter);

    let result = world
        .system::<&Position>()
        .expr("invalid syntax!!!")
        .each(move |_p| {
            counter_clone.fetch_add(1, Ordering::SeqCst);
        });

    assert!(
        matches!(result, Err(SystemBuildError::InvalidExpr { .. })),
        "system creation must fail on invalid query"
    );
    assert_eq!(
        Arc::strong_count(&counter),
        1,
        "closure must be reclaimed when ecs_system_init fails"
    );
}

/// Same reclaim requirement on the observer path (`ecs_observer_init`).
#[test]
fn observer_query_init_failure_reclaims_closure() {
    let world = World::new();
    let counter = Arc::new(AtomicUsize::new(0));
    let counter_clone = Arc::clone(&counter);

    let result = world
        .observer::<flecs::OnSet, &Position>()
        .expr("invalid syntax!!!")
        .each(move |_p| {
            counter_clone.fetch_add(1, Ordering::SeqCst);
        });

    assert!(
        matches!(result, Err(ObserverBuildError::InvalidExpr { .. })),
        "observer creation must fail on invalid query"
    );
    assert_eq!(
        Arc::strong_count(&counter),
        1,
        "closure must be reclaimed when ecs_observer_init fails"
    );
}

/// Component hook `_drop` trampolines used to run the closure destructor via
/// `ptr::drop_in_place` on a leaked box, running the destructor but never
/// deallocating. They now reconstruct the `Box` so the closure is freed. The
/// destructor must run exactly once (a double free would double-panic here).
#[test]
fn component_hook_closure_dropped_exactly_once() {
    struct DropCounter(Arc<AtomicUsize>);
    impl Drop for DropCounter {
        fn drop(&mut self) {
            self.0.fetch_add(1, Ordering::SeqCst);
        }
    }

    let counter = Arc::new(AtomicUsize::new(0));
    {
        let world = World::new();
        let guard = DropCounter(Arc::clone(&counter));
        world.component::<Position>().on_add(move |_e, _p| {
            let _keep = &guard;
        });
        world.entity().set(Position { x: 1, y: 2 });
    }

    assert_eq!(
        counter.load(Ordering::SeqCst),
        1,
        "hook closure destructor must run exactly once"
    );
    assert_eq!(
        Arc::strong_count(&counter),
        1,
        "hook closure box must be reclaimed (Box::from_raw), not leaked"
    );
}

/// `Entity::entity_view`/`Id::entity_view` are now `unsafe` because binding an
/// id to the wrong world is type confusion. The safe `*_checked` variants
/// validate the id against the target world via `ecs_is_valid` and return
/// `None` for null, stale, or foreign ids.
#[test]
fn entity_view_checked_rejects_invalid_ids() {
    let world = World::new();

    let e = world.entity();
    let id: Entity = e.id();
    assert!(id.entity_view_checked(&world).is_some());

    assert!(Entity::null().entity_view_checked(&world).is_none());

    e.destruct();
    assert!(
        id.entity_view_checked(&world).is_none(),
        "a destroyed id must not validate against the world"
    );

    let other = World::new();
    let live = world.entity();
    let live_id: Entity = live.id();
    assert!(
        live_id.entity_view_checked(&other).is_none(),
        "an id from another world must not validate"
    );
}

#[derive(Component)]
struct ArcPayload {
    tracker: Arc<()>,
}

/// Deferred enqueue: flecs copies the payload into the command queue and runs
/// the registered dtor on that copy at flush; the Rust side frees only the
/// heap slot. The payload's resources must be released exactly once.
#[test]
fn enqueue_deferred_drops_payload_exactly_once() {
    let world = World::new();

    let tracker = Arc::new(());

    let id_a = world.entity();
    let e1 = world.entity().add(id_a);

    world.defer_begin();
    world
        .event::<ArcPayload>()
        .add(id_a)
        .entity(e1)
        .enqueue(ArcPayload {
            tracker: Arc::clone(&tracker),
        });
    assert_eq!(
        Arc::strong_count(&tracker),
        2,
        "queued payload copy keeps the Arc alive until flush"
    );
    world.defer_end();

    assert_eq!(
        Arc::strong_count(&tracker),
        1,
        "payload must be dropped exactly once at command flush"
    );
}

/// Non-deferred enqueue falls through to a synchronous emit; flecs does not
/// take ownership. The Rust side used to free the heap slot without running
/// the destructor, leaking the payload's resources. It now reclaims the Box.
#[test]
fn enqueue_non_deferred_drops_payload() {
    let world = World::new();

    let tracker = Arc::new(());

    let id_a = world.entity();
    let e1 = world.entity().add(id_a);

    world
        .event::<ArcPayload>()
        .add(id_a)
        .entity(e1)
        .enqueue(ArcPayload {
            tracker: Arc::clone(&tracker),
        });

    assert_eq!(
        Arc::strong_count(&tracker),
        1,
        "payload must be dropped after a non-deferred enqueue (synchronous emit)"
    );
}

/// `TableIter::entities()` used `slice::from_raw_parts_mut` on the C-owned
/// const entities array; it is now a plain shared slice.
#[test]
fn iter_entities_readonly_slice() {
    let world = World::new();

    world.entity().set(Position { x: 1, y: 0 });
    world.entity().set(Position { x: 2, y: 0 });

    let mut total = 0;
    world.query::<&Position>().build().run(|mut it| {
        while it.next() {
            total += it.entities().len();
        }
    });
    assert_eq!(total, 2);
}

/// `entity(row)` used to compute `entities.add(row)` before any bounds check,
/// which is out-of-bounds pointer arithmetic (UB) for a row past the count. It
/// now asserts the row is in range.
#[test]
#[should_panic(expected = "out of bounds")]
fn iter_entity_out_of_range_row_panics() {
    let world = World::new();

    world.entity().set(Position { x: 1, y: 0 });

    world.query::<&Position>().build().run(|mut it| {
        while it.next() {
            let _ = it.entity(it.count() + 5);
        }
    });
}

/// `entity_id(row)` had the same out-of-bounds pointer arithmetic.
#[test]
#[should_panic(expected = "out of bounds")]
fn iter_entity_id_out_of_range_row_panics() {
    let world = World::new();

    world.entity().set(Position { x: 1, y: 0 });

    world.query::<&Position>().build().run(|mut it| {
        while it.next() {
            let _ = it.entity_id(it.count() + 5);
        }
    });
}

/// `get_entity(row)` returns `None` for an out-of-range row instead of doing
/// out-of-bounds pointer arithmetic.
#[test]
fn iter_get_entity_out_of_range_row_is_none() {
    let world = World::new();

    world.entity().set(Position { x: 1, y: 0 });

    world.query::<&Position>().build().run(|mut it| {
        while it.next() {
            let oob = it.count() + 5;
            assert!(it.get_entity(oob).is_none());
        }
    });
}

/// `is_self(index)` used to sign-extend a negative `i8` into a huge `usize`
/// offset and read out of bounds. It now asserts the index is in range.
#[test]
#[should_panic(expected = "out of range")]
fn iter_is_self_negative_index_panics() {
    let world = World::new();

    world.entity().set(Position { x: 1, y: 0 });

    world.query::<&Position>().build().run(|mut it| {
        while it.next() {
            it.is_self(-1);
        }
    });
}

/// `is_set(index)` had the same sign-extension problem.
#[test]
#[should_panic(expected = "out of range")]
fn iter_is_set_out_of_range_index_panics() {
    let world = World::new();

    world.entity().set(Position { x: 1, y: 0 });

    world.query::<&Position>().build().run(|mut it| {
        while it.next() {
            it.is_set(100);
        }
    });
}

/// System accessors used to dereference the result of `ecs_system_get`
/// without a null check, which is a null deref for any entity that is not a
/// system. They now panic with a clear message.
#[test]
#[should_panic(expected = "entity is not a system")]
fn system_accessors_panic_on_non_system_entity() {
    let world = World::new();

    let e = world.entity();
    let system = world.system_from(e);
    let _ = system.query();
}
