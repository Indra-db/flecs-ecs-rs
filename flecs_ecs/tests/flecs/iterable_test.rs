#![allow(dead_code)]
#![allow(unused_imports)]
use crate::common_test::*;
use flecs_ecs::experimental::prelude::*;
use flecs_ecs::prelude::*;

#[test]
fn iterable_page_each() {
    let world = World::new();
    let e1 = world.entity();
    e1.set(SelfRef { value: *e1 });
    let e2 = world.entity();
    e2.set(SelfRef { value: *e2 });
    let e3 = world.entity();
    e3.set(SelfRef { value: *e3 });
    let e4 = world.entity();
    e4.set(SelfRef { value: *e4 });
    let e5 = world.entity();
    e5.set(SelfRef { value: *e5 });

    let q = world.new_query::<&SelfRef>();

    let mut count = 0;
    q.page(1, 3).each_entity_shared(&world, |e, self_| {
        count += 1;
        assert_ne!(e.id(), e1.id());
        assert_ne!(e.id(), e5.id());
        assert_eq!(e.id(), self_.value);
    });
    assert_eq!(count, 3);
}

#[test]
fn iterable_page_iter() {
    let mut world = World::new();
    let e1 = world.entity();
    e1.set(SelfRef { value: *e1 });
    let e2 = world.entity();
    e2.set(SelfRef { value: *e2 });
    let e3 = world.entity();
    e3.set(SelfRef { value: *e3 });
    let e4 = world.entity();
    e4.set(SelfRef { value: *e4 });
    let e5 = world.entity();
    e5.set(SelfRef { value: *e5 });

    let _ = (e1, e2, e3, e4, e5);

    let q = world.new_query::<&SelfRef>();

    let mut count = 0;
    q.page(1, 3).run_exclusive(&mut world, |mut it| {
        while it.next() {
            let self_ = it.field::<SelfRef>(0);
            assert_eq!(it.count(), 3);
            count += it.count() as i32;
            let _ = self_;
        }
    });
    assert_eq!(count, 3);
}

#[test]
fn iterable_worker_each() {
    let world = World::new();
    let e1 = world.entity();
    e1.set(SelfRef { value: *e1 });
    let e2 = world.entity();
    e2.set(SelfRef { value: *e2 });
    let e3 = world.entity();
    e3.set(SelfRef { value: *e3 });
    let e4 = world.entity();
    e4.set(SelfRef { value: *e4 });
    let e5 = world.entity();
    e5.set(SelfRef { value: *e5 });

    let q = world.new_query::<&SelfRef>();

    let mut count = 0;
    q.worker(0, 2).each_entity_shared(&world, |e, self_| {
        count += 1;
        assert_ne!(e.id(), e4.id());
        assert_ne!(e.id(), e5.id());
        assert_eq!(e.id(), self_.value);
    });
    assert_eq!(count, 3);
}

#[test]
fn iterable_worker_iter() {
    let mut world = World::new();
    let e1 = world.entity();
    e1.set(SelfRef { value: *e1 });
    let e2 = world.entity();
    e2.set(SelfRef { value: *e2 });
    let e3 = world.entity();
    e3.set(SelfRef { value: *e3 });
    let e4 = world.entity();
    e4.set(SelfRef { value: *e4 });
    let e5 = world.entity();
    e5.set(SelfRef { value: *e5 });

    let _ = (e1, e2, e3, e4, e5);

    let q = world.new_query::<&SelfRef>();

    let mut count = 0;
    q.worker(0, 2).run_exclusive(&mut world, |mut it| {
        while it.next() {
            count += it.count() as i32;
        }
    });
    assert_eq!(count, 3);

    count = 0;
    q.worker(1, 2).run_exclusive(&mut world, |mut it| {
        while it.next() {
            count += it.count() as i32;
        }
    });
    assert_eq!(count, 2);
}
