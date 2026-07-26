//! Tests for `!Send`/`!Sync` component support.
//!
//! Components are no longer required to be `Send + Sync`. Thread-bound data
//! (e.g. raylib handles, `Rc`) can be stored in the world as long as it is
//! only accessed from the thread that owns the world.

#![allow(clippy::float_cmp)]

use alloc::rc::Rc;
use core::cell::RefCell;

use flecs_ecs::prelude::*;

#[derive(Component)]
struct NonSendHandle {
    value: Rc<RefCell<i32>>,
}

#[test]
fn non_send_component_send_sync_consts() {
    #[derive(Component)]
    struct PlainComponent {
        _value: i32,
    }

    const {
        assert!(!NonSendHandle::IMPLS_SEND);
        assert!(!NonSendHandle::IMPLS_SYNC);
        assert!(PlainComponent::IMPLS_SEND);
        assert!(PlainComponent::IMPLS_SYNC);
    }
}

#[test]
fn non_send_component_access_from_worker_thread_panics() {
    use alloc::sync::Arc;
    use core::panic::AssertUnwindSafe;
    use core::sync::atomic::{AtomicUsize, Ordering};

    #[derive(Component)]
    struct PlainComponent {
        _value: i32,
    }

    #[derive(Component)]
    struct NonSendSingleton {
        value: Rc<i32>,
    }

    let mut world = World::new();
    world.set(NonSendSingleton { value: Rc::new(1) });

    for _ in 0..1024 {
        world.entity().set(PlainComponent { _value: 0 });
    }

    world.set_threads(4);

    let owner_thread = std::thread::current().id();
    let violations = Arc::new(AtomicUsize::new(0));
    let worker_hits = Arc::new(AtomicUsize::new(0));
    let violations_clone = violations.clone();
    let worker_hits_clone = worker_hits.clone();

    world
        .system::<&mut PlainComponent>()
        .par_each_entity(move |entity, _plain| {
            let on_worker = std::thread::current().id() != owner_thread;
            if on_worker {
                worker_hits_clone.fetch_add(1, Ordering::Relaxed);
            }

            let world = entity.world();
            let result = std::panic::catch_unwind(AssertUnwindSafe(|| {
                world.get::<&NonSendSingleton>(|singleton| {
                    core::hint::black_box(&singleton.value);
                });
            }));

            if result.is_err() {
                assert!(
                    on_worker,
                    "thread affinity check must not fire on the owning thread"
                );
                violations_clone.fetch_add(1, Ordering::Relaxed);
            } else {
                assert!(
                    !on_worker,
                    "accessing a !Send component from a worker thread must panic"
                );
            }
        });

    world.progress();

    assert!(
        worker_hits.load(Ordering::Relaxed) > 0,
        "test needs entities processed on worker threads"
    );
    assert_eq!(
        violations.load(Ordering::Relaxed),
        worker_hits.load(Ordering::Relaxed),
        "every worker-thread access must panic"
    );
}

#[test]
fn non_send_component_set_from_worker_thread_panics() {
    use alloc::sync::Arc;
    use core::panic::AssertUnwindSafe;
    use core::sync::atomic::{AtomicUsize, Ordering};

    #[derive(Component)]
    struct PlainComponent {
        _value: i32,
    }

    #[derive(Component)]
    struct NonSendPayload {
        _value: Rc<i32>,
    }

    let mut world = World::new();
    world.component::<NonSendPayload>();

    for _ in 0..1024 {
        world.entity().set(PlainComponent { _value: 0 });
    }

    world.set_threads(4);

    let owner_thread = std::thread::current().id();
    let violations = Arc::new(AtomicUsize::new(0));
    let worker_hits = Arc::new(AtomicUsize::new(0));
    let violations_clone = violations.clone();
    let worker_hits_clone = worker_hits.clone();

    world
        .system::<&mut PlainComponent>()
        .par_each_entity(move |entity, _plain| {
            let on_worker = std::thread::current().id() != owner_thread;
            if on_worker {
                worker_hits_clone.fetch_add(1, Ordering::Relaxed);
            }

            let result = std::panic::catch_unwind(AssertUnwindSafe(|| {
                entity.set(NonSendPayload { _value: Rc::new(1) });
            }));

            assert_eq!(
                result.is_err(),
                on_worker,
                "setting a !Send component must panic on worker threads and only there"
            );
            if result.is_err() {
                violations_clone.fetch_add(1, Ordering::Relaxed);
            }
        });

    world.progress();

    assert!(worker_hits.load(Ordering::Relaxed) > 0);
    assert_eq!(
        violations.load(Ordering::Relaxed),
        worker_hits.load(Ordering::Relaxed)
    );
}

#[test]
fn non_send_component_set_and_get() {
    let world = World::new();

    let shared = Rc::new(RefCell::new(42));
    let entity = world.entity().set(NonSendHandle {
        value: shared.clone(),
    });

    entity.get::<&NonSendHandle>(|handle| {
        assert_eq!(*handle.value.borrow(), 42);
    });

    entity.get::<&mut NonSendHandle>(|handle| {
        *handle.value.borrow_mut() = 7;
    });

    assert_eq!(*shared.borrow(), 7);
}
