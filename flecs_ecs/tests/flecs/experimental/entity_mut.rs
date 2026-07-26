//! Deliverable: `EntityMut` and the exclusive entity surface (spec §3.5).

use core::sync::atomic::{AtomicUsize, Ordering::SeqCst};

use alloc::sync::Arc;

use flecs_ecs::core::*;
use flecs_ecs::experimental::prelude::*;
use flecs_ecs::macros::*;

#[derive(Component, Debug, PartialEq)]
struct Pos {
    x: i32,
    y: i32,
}

#[derive(Component, Debug, PartialEq)]
struct Vel {
    x: i32,
    y: i32,
}

#[derive(Component, Debug, PartialEq, Clone)]
struct Health(i32);

#[derive(Component)]
struct Tag;

#[derive(Component)]
struct InWorld;

#[test]
fn get_and_get_mut_round_trip() {
    let mut world = World::new();
    let mut e = world.entity_new();
    e.set(Pos { x: 1, y: 2 });

    // Immutable read.
    assert_eq!(e.get::<Pos>().unwrap(), &Pos { x: 1, y: 2 });

    // Mutate through a plain &mut (no guard, no lock).
    {
        let p = e.get_mut::<Pos>().unwrap();
        p.x += 40;
        p.y += 40;
    }
    assert_eq!(e.get::<Pos>().unwrap(), &Pos { x: 41, y: 42 });
}

#[test]
fn get_missing_component_is_none() {
    let mut world = World::new();
    let mut e = world.entity_new();
    e.set(Pos { x: 0, y: 0 });
    assert!(e.get::<Vel>().is_none());
    assert!(e.get_mut::<Vel>().is_none());
}

#[test]
fn get_many_mixed_tuple() {
    let mut world = World::new();
    let mut e = world.entity_new();
    e.set(Pos { x: 1, y: 2 }).set(Vel { x: 3, y: 4 });

    // One fused, lock-free borrow: shared Pos, mutable Vel.
    let (p, v) = e.get_many::<(&Pos, &mut Vel)>().unwrap();
    assert_eq!(*p, Pos { x: 1, y: 2 });
    v.x += 10;
    v.y += 10;

    assert_eq!(e.get::<Vel>().unwrap(), &Vel { x: 13, y: 14 });
}

#[test]
fn get_many_missing_required_is_none() {
    let mut world = World::new();
    let mut e = world.entity_new();
    e.set(Pos { x: 1, y: 2 });
    // Vel absent: the whole fused acquire is None.
    assert!(e.get_many::<(&Pos, &mut Vel)>().is_none());
}

/// Duplicate-mutable (`(&mut A, &mut A)`) is rejected by the fused resolver's
/// static duplicate-mutable check (`create_ptrs`), which folds to a constant per
/// monomorphization and panics before any borrow is handed out. A true
/// compile-time rejection is not expressible on the pinned stable toolchain
/// (`TypeId` / `type_name` cannot be compared in a `const` context), so the check
/// is the sound const-folded panic, matching the shared-register `get`
/// convention; see the `should_panic` doctest on `EntityMut::get_many`.
#[test]
#[should_panic(expected = "more than once")]
fn get_many_duplicate_mutable_panics() {
    let mut world = World::new();
    let mut e = world.entity_new();
    e.set(Pos { x: 1, y: 2 });
    let _ = e.get_many::<(&mut Pos, &mut Pos)>();
}

#[test]
fn chained_set_add_remove_are_immediate() {
    let mut world = World::new();
    let mut e = world.entity_new();

    // Chained structural ops each apply immediately (no defer): the read-back
    // below sees every change without reaching a sync point.
    e.set(Pos { x: 1, y: 1 })
        .set(Vel { x: 2, y: 2 })
        .add(Tag::id())
        .set(Health(5))
        .remove(Vel::id());

    assert_eq!(e.get::<Pos>().unwrap(), &Pos { x: 1, y: 1 });
    assert_eq!(e.get::<Health>().unwrap(), &Health(5));
    assert!(e.get::<Vel>().is_none(), "remove is immediate");

    let view = e.entity_view();
    assert!(view.has(Tag::id()), "add is immediate");
}

#[test]
fn insert_bundle_single_move_sees_final_table() {
    let mut world = World::new();

    let fired = Arc::new(AtomicUsize::new(0));
    let all_present = Arc::new(AtomicUsize::new(0));
    {
        let fired = fired.clone();
        let all_present = all_present.clone();
        world
            .observer::<flecs::OnAdd, ()>()
            .with(Pos::id())
            .each_entity(move |e, _| {
                fired.fetch_add(1, SeqCst);
                if e.has(Vel::id()) && e.has(Health::id()) {
                    all_present.fetch_add(1, SeqCst);
                }
            });
    }

    let mut e = world.entity_new();
    e.insert((Pos { x: 1, y: 1 }, Vel { x: 2, y: 2 }, Health(3)));

    assert_eq!(fired.load(SeqCst), 1, "OnAdd(Pos) fired once");
    assert_eq!(
        all_present.load(SeqCst),
        1,
        "entity already in final table when OnAdd(Pos) fired -> single move"
    );
    assert_eq!(e.get::<Pos>().unwrap(), &Pos { x: 1, y: 1 });
    assert_eq!(e.get::<Health>().unwrap(), &Health(3));
}

#[test]
fn entity_new_and_entity_mut_liveness() {
    let mut world = World::new();

    let id = world.entity_new().id();
    assert!(world.entity_mut(id).is_some());

    world.entity_from_id(id).destruct();
    assert!(world.entity_mut(id).is_none(), "dead entity -> None");

    // A never-created id is also None.
    assert!(world.entity_mut(999_999u64).is_none());
}

#[test]
fn entity_mut_of_existing_entity_mutates() {
    let mut world = World::new();
    let id = world.entity_new().id();
    world.entity_mut(id).unwrap().set(Health(7));
    world
        .entity_from_id(id)
        .get::<&Health>(|h| assert_eq!(*h, Health(7)));
}

#[test]
fn spawn_returns_entity_mut_and_chains() {
    let mut world = World::new();

    // spawn (one table move) returns EntityMut; chaining a further immediate set
    // must compile and land all three components.
    let e = world
        .spawn((Pos { x: 1, y: 2 }, Vel { x: 3, y: 4 }))
        .set(Health(9))
        .id();

    let view = world.entity_from_id(e);
    view.get::<(&Pos, &Vel, &Health)>(|(p, v, h)| {
        assert_eq!(*p, Pos { x: 1, y: 2 });
        assert_eq!(*v, Vel { x: 3, y: 4 });
        assert_eq!(*h, Health(9));
    });
}

#[test]
fn pair_set_and_get() {
    let mut world = World::new();

    // set_pair with the non-tag element as data (spec GAP-10, exclusive side).
    // The pair (InWorld, Pos) carries Pos as its data component.
    let e_id = {
        let mut e = world.entity_new();
        e.set_pair::<InWorld, Pos>(Pos { x: 5, y: 6 });
        assert_eq!(
            e.get::<(InWorld, Pos)>().unwrap(),
            &Pos { x: 5, y: 6 },
            "pair read via exclusive get"
        );
        e.id()
    };

    // set_first / set_second land the pair immediately, addressed by a target id.
    let target = world.entity_new().id();
    {
        let mut e = world.entity_mut(e_id).unwrap();
        e.set_first::<Health>(Health(11), target);
        assert!(
            e.entity_view().has((Health::id(), target)),
            "set_first pair present immediately"
        );
    }

    let src = world.entity_new().id();
    {
        let mut e = world.entity_mut(e_id).unwrap();
        e.set_second::<Health>(src, Health(22));
        assert!(
            e.entity_view().has((src, Health::id())),
            "set_second pair present immediately"
        );
    }
}
