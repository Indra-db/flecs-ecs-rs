//! Arity smoke tests for the `tuples!`-generated experimental tuple impls
//! (spec §4.8): guard tuples (`get`), chunk tuples (`chunks`), and
//! `EntityMut::get_many` at arities the old hand-capped impls (max 5) never
//! covered.

use flecs_ecs::core::*;
use flecs_ecs::experimental::is_proven_disjoint;
use flecs_ecs::experimental::prelude::*;
use flecs_ecs::macros::*;

#[derive(Component, Default)]
struct C1(i32);
#[derive(Component, Default)]
struct C2(i32);
#[derive(Component, Default)]
struct C3(i32);
#[derive(Component, Default)]
struct C4(i32);
#[derive(Component, Default)]
struct C5(i32);
#[derive(Component, Default)]
struct C6(i32);
#[derive(Component, Default)]
struct C7(i32);
#[derive(Component, Default)]
struct C8(i32);
#[derive(Component, Default)]
struct C9(i32);
#[derive(Component, Default)]
struct C10(i32);
#[derive(Component, Default)]
struct C11(i32);
#[derive(Component, Default)]
struct C12(i32);

fn seed_12(world: &World, n: i32) {
    for i in 0..n {
        world
            .entity()
            .set(C1(i))
            .set(C2(2))
            .set(C3(3))
            .set(C4(4))
            .set(C5(5))
            .set(C6(6))
            .set(C7(7))
            .set(C8(8))
            .set(C9(9))
            .set(C10(10))
            .set(C11(11))
            .set(C12(12));
    }
}

#[test]
fn get_tuple_arity_8() {
    let world = World::new();
    seed_12(&world, 1);
    let e = world.new_query::<&C1>().first_entity();

    let (mut a, b, c, d, e2, f, g, h) = e
        .get::<(&mut C1, &C2, &C3, &C4, &C5, &C6, &C7, &C8)>()
        .unwrap();
    a.0 = b.0 + c.0 + d.0 + e2.0 + f.0 + g.0 + h.0;
    drop((a, b, c, d, e2, f, g, h));

    let a = e.get::<&C1>().unwrap();
    assert_eq!(a.0, 2 + 3 + 4 + 5 + 6 + 7 + 8);
}

#[test]
fn get_tuple_arity_12() {
    let world = World::new();
    seed_12(&world, 1);
    let e = world.new_query::<&C1>().first_entity();

    let guards = e
        .get::<(
            &mut C1,
            &C2,
            &C3,
            &C4,
            &C5,
            &C6,
            &C7,
            &C8,
            &C9,
            &C10,
            &C11,
            &C12,
        )>()
        .unwrap();
    let sum = guards.1.0
        + guards.2.0
        + guards.3.0
        + guards.4.0
        + guards.5.0
        + guards.6.0
        + guards.7.0
        + guards.8.0
        + guards.9.0
        + guards.10.0
        + guards.11.0;
    let mut a = guards.0;
    a.0 = sum;
    drop(a);

    let a = e.get::<&C1>().unwrap();
    assert_eq!(a.0, (2..=12).sum::<i32>());
}

#[test]
fn get_tuple_arity_12_conflict_still_detected() {
    let world = World::new();
    seed_12(&world, 1);
    let e = world.new_query::<&C1>().first_entity();

    let _held = e.get::<&mut C12>().unwrap();
    let r = e.try_get::<(
        &C1,
        &C2,
        &C3,
        &C4,
        &C5,
        &C6,
        &C7,
        &C8,
        &C9,
        &C10,
        &C11,
        &mut C12,
    )>();
    assert!(matches!(r, Err(AccessError::Conflict { write: true, .. })));
}

#[test]
fn chunks_arity_8() {
    let mut world = World::new();
    seed_12(&world, 4);
    let q = world.new_query::<(&mut C1, &C2, &C3, &C4, &C5, &C6, &C7, &C8)>();
    assert!(is_proven_disjoint(&world, q.query_ptr()));

    let mut rows = 0;
    for (c1, c2, c3, c4, c5, c6, c7, c8) in q.chunks(&mut world) {
        rows += c1.len();
        for i in 0..c1.len() {
            c1[i].0 = c2[i].0 + c3[i].0 + c4[i].0 + c5[i].0 + c6[i].0 + c7[i].0 + c8[i].0;
        }
    }
    assert_eq!(rows, 4);

    q.each_shared(&world, |(c1, ..)| {
        assert_eq!(c1.0, 2 + 3 + 4 + 5 + 6 + 7 + 8);
    });
}

#[test]
fn chunks_arity_12() {
    let mut world = World::new();
    seed_12(&world, 3);
    let q = world.new_query::<(
        &mut C1,
        &C2,
        &C3,
        &C4,
        &C5,
        &C6,
        &C7,
        &C8,
        &C9,
        &C10,
        &C11,
        &C12,
    )>();
    assert!(is_proven_disjoint(&world, q.query_ptr()));

    let mut rows = 0;
    for chunk in q.chunks(&mut world) {
        rows += chunk.0.len();
        for i in 0..chunk.0.len() {
            chunk.0[i].0 = chunk.11[i].0;
        }
    }
    assert_eq!(rows, 3);

    q.each_shared(&world, |t| assert_eq!(t.0.0, 12));
}

#[test]
fn get_many_arity_8() {
    let mut world = World::new();
    seed_12(&world, 1);
    let id = world.new_query::<&C1>().first_entity().id();

    let mut e = world.entity_mut(id).unwrap();
    let (c1, c2, c3, c4, c5, c6, c7, c8) = e
        .get_many::<(&mut C1, &C2, &C3, &C4, &C5, &C6, &C7, &C8)>()
        .unwrap();
    c1.0 = c2.0 + c3.0 + c4.0 + c5.0 + c6.0 + c7.0 + c8.0;
    assert_eq!(e.get::<C1>().unwrap().0, 2 + 3 + 4 + 5 + 6 + 7 + 8);
}

#[test]
fn get_many_arity_12() {
    let mut world = World::new();
    seed_12(&world, 1);
    let id = world.new_query::<&C1>().first_entity().id();

    let mut e = world.entity_mut(id).unwrap();
    let t = e
        .get_many::<(
            &mut C1,
            &C2,
            &C3,
            &C4,
            &C5,
            &C6,
            &C7,
            &C8,
            &C9,
            &C10,
            &C11,
            &C12,
        )>()
        .unwrap();
    t.0.0 = t.1.0
        + t.2.0
        + t.3.0
        + t.4.0
        + t.5.0
        + t.6.0
        + t.7.0
        + t.8.0
        + t.9.0
        + t.10.0
        + t.11.0;
    assert_eq!(e.get::<C1>().unwrap().0, (2..=12).sum::<i32>());
}
