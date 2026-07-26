//! Deliverable 4: build-time disjointness proof (tier 0).

use super::{Position, Velocity};
use flecs_ecs::core::*;
use flecs_ecs::experimental::is_proven_disjoint;
use flecs_ecs::macros::{query, Component};

#[derive(Component, Default)]
struct Tag;

#[derive(Component, Default)]
struct Sparse(i32);

#[test]
fn distinct_dense_terms_are_proven() {
    let world = World::new();
    let q = query!(world, &mut Position, &Velocity).build();
    assert!(is_proven_disjoint(&world, q.query_ptr()));
}

#[test]
fn single_term_is_proven() {
    let world = World::new();
    let q = query!(world, &mut Position).build();
    assert!(is_proven_disjoint(&world, q.query_ptr()));
}

#[test]
fn tag_term_does_not_block_proof() {
    let world = World::new();
    // Tag has no data, so it cannot alias; the data terms are still distinct.
    let q = query!(world, &mut Position, Tag).build();
    assert!(is_proven_disjoint(&world, q.query_ptr()));
}

#[test]
fn duplicate_component_is_not_proven() {
    let world = World::new();
    // Same concrete id, one written: classic mutable-alias hazard.
    let q = query!(world, &mut Position, &Position).build();
    assert!(!is_proven_disjoint(&world, q.query_ptr()));
}

#[test]
fn wildcard_pair_is_not_proven() {
    let world = World::new();
    let q = query!(world, &mut Position)
        .with((id::<Velocity>(), id::<flecs::Wildcard>()))
        .build();
    assert!(!is_proven_disjoint(&world, q.query_ptr()));
}

#[test]
fn sparse_term_is_not_proven() {
    let world = World::new();
    world.component::<Sparse>().add_trait::<flecs::Sparse>();
    let q = query!(world, &mut Sparse).build();
    assert!(!is_proven_disjoint(&world, q.query_ptr()));
}

#[test]
fn optional_term_is_not_proven() {
    let world = World::new();
    let q = world.new_query::<(&mut Position, Option<&Velocity>)>();
    assert!(!is_proven_disjoint(&world, q.query_ptr()));
}

// Adversarial: a query that must NOT be proven keeps its locks and still
// panics on a genuine conflict when iterated on the shared register.
#[test]
#[should_panic]
fn non_disjoint_query_still_conflicts_on_shared_path() {
    let world = World::new();
    world.entity().set(Position::default());
    // Not proven disjoint (duplicate id); the shared-path each keeps locking,
    // so a nested write inside the read still panics.
    let q_read = query!(world, &Position).build();
    let q_write = query!(world, &mut Position).build();
    q_read.each(|_| {
        q_write.each(|_| {});
    });
}
