# Handoff: flecs Rust API redesign, wave-5 continuation

Date: 2026-07-27. Branch: `safety-perf` @ `3a00182d`. Tree clean. Nothing pushed.
Audience: a fresh agent on a new device with no context beyond the committed files and this folder.

## This folder

- `HANDOFF.md` (this file) — state, next actions, discipline, working agreements.
- `2026-07-26-rust-api-redesign-design.md` — decision record: locked user rulings, binding invariants, all measured results, review-round history.
- `2026-07-26-rust-api-redesign-spec.md` — the full API contract (15 sections), amended in place as reality diverged; current.
- `2026-07-27-wave5-execution-plan.md` — the 18-sub-wave surface-swap plan, 8 orchestrator rulings, ordering law, known debt, execution log.
- `2026-07-26-examples-migration-evaluation.md` — the honest examples scoring that drove the ergonomics amendments.

The same four design docs are also committed in the repo under `design/` at the branch tip, so the repo alone is sufficient if this folder is lost; this folder just adds HANDOFF.md and convenience.

## State at `3a00182d`

- Waves 1-4 (perf foundation, ten soundness fixes, prototype, spec) and wave-5 sub-waves SW-1..SW-13 are complete and merged. The legacy entity-access surface is DELETED and the guard API owns the final names: `entity.get::<&T>() -> Option<Ref<T>>`, `try_get`, `cloned`, `World::get_mut`, `World::entity_mut/entity_new` (EntityMut), `world.spawn((..))` bundles, `entity_ref` CachedRef, singletons; all promoted to `src/core/access/` and exported from the prelude.
- Query iteration still has BOTH surfaces: legacy world-argument-free `q.each(|..|)` etc. in `core/utility/traits/query_api.rs`, and the new world-threaded forms (`each_exclusive`, `each_entity_exclusive`, `run_exclusive`, the `each_shared` family, `chunks`, `batches`, `each!`, the `each_iter` ctx form) still under `src/experimental/`. Removing the legacy surface and claiming final names is SW-14, which had NOT started when this handoff was cut (its agent was killed before any change; the tree is clean).
- Suite: debug `cargo +1.97.0 test -p flecs_ecs --test flecs` = 2539 passed / 0 failed / 27 ignored; release = 2506 / 0 / 45; doctests 348. `clippy --all-targets --workspace -- -D warnings` clean. These are the baseline gates for every future commit.
- Toolchain: cargo +1.97.0 required (workspace floor). The 4 old trybuild `.stderr` drifts were regenerated under 1.97 during SW-5/6; if CI pins a different rustc, revisit commit `64d3c2e7`.
- Machine-local things that do NOT transfer: criterion baselines (re-baseline first, see below), the old device's session memory, local branches other than safety-perf (all merged; sw8/sw10/sw11/sw12/sw5-6 and worktree-agent-* branches are safe to delete), one stale stash from a concurrent-edit race (safe to drop).

## Next action: SW-14 (Flip 2, iteration). Full task spec

Execute exactly this scope, alone in the tree (tree-wide renames do not parallelize):

1. Migrate the deferred `each_iter` call sites (~120+ across tests/examples/flecs_docs_test, deliberately left legacy by ruling 3): legacy `q.each_iter(|it, index, item|)` to the world-threaded ctx form; establish one idiomatic mapping (sites needing `(it, index)` together use `run_exclusive`/`run_shared` with a TableIter loop) and apply it uniformly.
2. Delete the legacy world-argument-free `each`/`each_entity`/`each_iter`/`run` from `query_api.rs`. `QueryIter`/`ChainedIter` stay but must only be iterable with a world argument. Compilation enumerates remaining call sites; migrate them all.
3. Rename: `each_exclusive -> each`, `each_entity_exclusive -> each_entity`, `run_exclusive -> run`; the `_shared` names stay. SparseQuery keeps legacy names (flag for SW-17).
4. World one-liners per ruling 5: `World::each::<D>`/`each_entity::<D>` take `&mut self` with a per-world TypeId-keyed cached query (same machinery as the bundle id cache in `world_ctx.rs`).
5. Promote experimental chunks/batches/disjoint/iter_ctx/exclusive+shared ext traits/`each!` to core (follow SW-13's promotion pattern: `git show 3a00182d`); `is_proven_disjoint` becomes a `Query` method. `stage.rs`/`bundle.rs` stay experimental until SW-15/17.
6. Add the prelude allowlist test (spec section 14.11).
7. MANDATORY bench gate: rewrite the `query_each_*` benches to the new surface (same workloads, same names); numbers must match or beat the fresh baseline with raw-C controls flat. Run the full `^flecs/query_each` + `^flecs/exp_` suites.

Then, in order: SW-15 (SystemRunnerFluent + set_context deletion, OnAdd type bound, order_by transmute; converts the tests flagged below), SW-16 (WorldRef demotion per ruling 7; plus SW-13's feature-combo debt), SW-17 (field-accessor grid, RestConfig, meta sweep incl. the enum-target read and the ScriptBuilder pin-bypass residual, module identity, SparseQuery surface, App::frame_action gap), SW-18 (migration guide, full gate battery, release prep). Per-sub-wave detail is in the plan document. Post-SW-18 deferred debt: the IntoId const-bool -> trait dispatch refactor (ruling 8; own bench-gated session).

## Flagged kept-legacy sites (routing list)

- `each_iter` sites everywhere -> SW-14 (item 1 above).
- `system_test`/`observer_test::lookup_and_update_ctx` (raw ctx pointer identity), `system_test` 4x `register_twice_*` (need System handle decoupling from the world borrow: see the SW-3 note in the plan), `observer_rust_test::observer_panic_on_add_1..4` (the runtime guard SW-15's type bound replaces), SystemRunnerFluent call sites in system_builder tests -> SW-15.
- `meta_trait_test::test_enum` (enum-as-relationship-target read the guard surface rejects: it caught a latent use-after-free; needs a designed meta read path), SparseQuery names, ScriptBuilder caller-supplied-entity pin bypass, App::frame_action WorldRef gap -> SW-17.
- SW-13 feature-combo debt: guard surface requires `flecs_safety_locks`; `EntityMut::insert` gated `flecs_experimental`; some `core::access` doc links still point at old experimental paths -> SW-16.

## Benchmark discipline (non-negotiable; this project retracted five wrong perf claims before adopting it)

- criterion only; `pmset -g | grep powermode` must print 0 before any run (macOS; use the platform equivalent elsewhere); the `*_raw_c` control benches must stay within ~3% or the run is layout/thermal drift: re-run, do not claim.
- Criterion baselines do NOT transfer between machines. FIRST ACTION on the new device, before any code change: `cargo +1.97.0 bench --bench main -- --save-baseline pre '^flecs/query_each'` and the same for `'^flecs/exp_'` at `3a00182d`. All gates are deltas against that fresh baseline; the old machine's absolute numbers below are reference only.
- Old-machine reference medians: query_each_1_term 8.19-8.27us, 4_terms 3.04-3.12us, 1_write 3.24-3.26us, each_entity 3.57-3.60us, raw-C controls 2.61-2.71us and 7.88-7.92us; exp_entity_get_1 9.87ns, exp_cached_ref_get 4.40ns, exp_each_exclusive_4 2.63-2.79us, chunks 2.94-3.06us, batches 3.16-3.39us, each! write 2.75-2.84us; bundle spawn_3 ~115-120us vs set*3 ~170-178us, spawn_batch_1000 ~17.8us vs ~169us. Historical context: session start had closure get at 14.4ns and each 1-term at 10.8us; the redesign's gains must never be given back.
- Hard perf invariants (design record, binding): no per-batch RAII Drop on iteration paths, no atomics in lock paths, 16-byte `ecs_rust_get_ptr_t` (const-asserted in `core/get_tuple.rs`), catch_unwind per-invocation never per-row, disjointness verdict cached per query.

## Orchestration pattern that worked (50+ agent runs, zero regressions shipped)

- One writer per git tree. Parallel agents get separate worktrees; on the old harness worktrees spawned from a STALE base, so every worktree agent's first mandatory step was `git checkout -B <branch> safety-perf`. Verify whether the new harness has the same behavior before trusting a worktree's base.
- Every sub-wave: full suite + clippy `-D warnings` per commit, every test-count delta accounted by name, bench gates where hot paths are touched, and an orchestrator hand-review of the one soundness-critical claim before merge. This caught, over the project: a cross-world exclusivity hole, a spawn-under-guard UB composition, a consumed-iterator abort, an enum revalidation false-panic, a spec-level Sync unsoundness (ReadOnlyTerms alone was insufficient; item-Send clause is load-bearing), and a vacuous-green lock-tier misroute.
- Migration rules: preserve test INTENT (safety tests state their lock tier in the report), never accept "pre-existing failure" claims without re-verifying on the clean tree, fix defects found along the way permanently in the same motion, never bless flaky evidence.

## Working agreements

Never `git push`. Never post to GitHub (draft text for the user instead). Commit freely on this branch without asking. No code comments except public API docs. No em dashes in prose. cargo +1.97.0 for everything. Vendored `flecs.c`/`flecs.h` untouched; the shim `flecs_rust.c/h` is editable. Fix every failure encountered regardless of origin. `synk/` at the repo root is an unrelated user project: never touch it.
