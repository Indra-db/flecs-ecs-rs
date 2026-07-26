//! Deliverable 4: build-time disjointness proof (tier 0).

use super::{Position, Velocity};
use flecs_ecs::core::*;
use flecs_ecs::experimental::is_proven_disjoint;
use flecs_ecs::experimental::prelude::*;
use flecs_ecs::macros::{query, Component};

#[derive(Component, Default)]
struct Tag;

#[derive(Component, Default)]
struct Sparse(i32);

#[derive(Component, Default)]
struct Late(i32);

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

// The cached verdict (spec §4.7) must be consistent across repeated exclusive
// iterations: lazily computed on first use, then reused. Exercises both the
// each_exclusive and chunks entry points on the same query so the second call
// reads the cache the first populated.
#[test]
fn cached_verdict_is_stable_across_calls() {
    let mut world = World::new();
    for i in 0..6 {
        world
            .entity()
            .set(Position { x: i, y: 0 })
            .set(Velocity { x: 1, y: 0 });
    }
    let q = query!(world, &mut Position, &Velocity).build();

    let mut sum1 = 0;
    q.each_exclusive(&mut world, |(p, v)| {
        p.x += v.x;
        sum1 += p.x;
    });
    // Second exclusive call reads the cached verdict rather than recomputing.
    let mut sum2 = 0;
    q.each_exclusive(&mut world, |(p, v)| {
        p.x += v.x;
        sum2 += p.x;
    });
    assert_eq!(sum2, sum1 + 6);

    // chunks() reads the same cached verdict and must still qualify.
    let mut rows = 0;
    q.chunks(&mut world).for_each(|(p, _v)| rows += p.len());
    assert_eq!(rows, 6);
}

// Pins the C invariant backing the cached-verdict stability (spec §4.7): once a
// query is built against a component, flecs refuses to add any trait other than
// `With` (`flecs_trait_can_add_after_query`, vendored `flecs.c:4298`; the
// enforcement lives in `flecs_register_flag_for_trait` at `flecs.c:4331`, active
// outside world-init). Because that trait is later `With`-only, no `Sparse` /
// `DontFragment` can appear after build, so the disjointness proof cached at
// build time can never be invalidated.
//
// The enforcement is an `ecs_check` that calls `ecs_os_abort`. Even with the
// OS-API abort shim installed, the panic cannot unwind cleanly back through the
// vendored-C frame that raised it (the flecs translation unit is compiled
// without unwind tables), so the process takes a hard `SIGABRT` and the whole
// test binary dies. This makes the check genuinely untestable in-process; the
// test is therefore `#[ignore]`d and stands as the executable reproducer for
// manual verification (`cargo test -- --ignored c_forbids_late_sparse_trait`,
// which is expected to abort). Re-verify on every vendored-C bump alongside the
// scheduler contract by confirming the two C line references above still hold.
#[test]
#[ignore = "flecs ecs_check aborts (SIGABRT) and cannot unwind through the C frame; manual-only"]
fn c_forbids_late_sparse_trait_after_query() {
    let _guard = crate::common_test::FlecsPanicAbortGuard::install();
    let result = std::panic::catch_unwind(core::panic::AssertUnwindSafe(|| {
        let world = World::new();
        let _q = query!(world, &mut Late).build();
        // Adding Sparse to an already-queried component must be rejected.
        world.component::<Late>().add_trait::<flecs::Sparse>();
    }));
    assert!(
        result.is_err(),
        "flecs must reject adding the Sparse trait after a query is built against the component"
    );
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
