# Flecs Rust API redesign: design record

Date: 2026-07-26. Branch: `safety-perf`. Status: decisions locked, spec in progress, prototype underway.

## Goals

Faster than the current API, more ergonomic, sound from 100% safe code. Clean break: no shims, single semver bump, migration guide. Where a safety check costs too much, an `unsafe fn` twin ships instead of dropping the check. Staged MT (flecs model): `World` is `!Send`/`!Sync`, MT via staging and worker threads.

## Locked decisions

| # | Decision | Ruling | Source |
|---|---|---|---|
| 1 | Core model | Two registers. `&mut World` = exclusive: no runtime locks, plain `&T`/`&mut T`, borrow checker is the lock. `&World` and `Copy` views = shared: RAII guards (`Ref`/`Mut`, `!Copy`) over per-stage lock maps; conflict panics like `RefCell`, `try_get` returns `Result`. `World` stops being `Clone`. Shared-register reads take read locks. | User, 2026-07-26 |
| 2 | Shared-register writes | `set` and structural ops under `&World` are deferred to the next sync point. Storage cannot move while a guard lives because the flush is unreachable (storage-pin). Immediate read-back via `&mut World` / `EntityMut`. | User, 2026-07-26 |
| 3 | `QueryHandle` auto-traits | Narrowed: `Send` where items are `Send`, `Sync` only when all terms are read-only. Cross-thread mutation via partitioned par systems or `unsafe`. Reverts part of `aa282cef`. | User, 2026-07-26 |
| 4 | Query lock granularity | Tiered, pay-for-what-you-use. Tier 0: exclusive-register iteration and build-time-proven-disjoint queries skip lock traffic. Tier 1: unprovable shared-register queries keep per-table-batch locking with today's semantics (disjoint-table read+write of one component stays legal). Tier 2: `unsafe *_unchecked` twins. No semantic narrowing anywhere. | Orchestrator, delegated by user 2026-07-26 |
| 5 | Iteration surface | Closure `each` + chunk cursor (`while let` over per-table column slices) + `each!` macro with native `break`/`continue`/`?`. No `LendingIterator` dependency: std has none and no timeline (settled research). Chunk is the primitive; row-at-a-time is sugar. | Prior brainstorm, user-approved |
| 6 | Error model | `Option` for absence, `Result` for malformed construction (`Result<Query<D>, QueryBuildError>`), panic only for genuine aliasing bugs. Safety is never a Cargo feature. | Prior brainstorm |
| 7 | Builders | Terminal method is the build (`.each(...) -> Result<System, _>`); terminal ops take `self` by value, so double-build and double-consume are compile errors. | Prior brainstorm |
| 8 | Panics vs C | Panics are caught at the trampoline, stashed, rethrown after flecs returns; never unwound through C frames. | Prior brainstorm |

## Invariants the implementation must preserve

- The 16-byte `ecs_rust_get_ptr_t` return. Never widen it: it is why `get` beats upstream by 19-30%.
- Per-stage sparse tracking (global tracking false-positives on disjoint entities across stages).
- No atomics in lock paths; stage maps are single-owner per thread.
- No per-batch RAII type with a `Drop` impl on the iteration path (measured +32% `par_each`, +87% `each` 1-write historically). Unwind recovery lives in `IterGuard`, once per iteration, via `StageLocksScope` snapshot/restore.
- Panic safety: a panicking callback leaves no unbalanced lock counters or `ecs_table_lock`.

## Measured state (Phase 1 complete)

Commits `f11dfc5d` (all-dense `each` fast path skips dead per-batch setup) and `49a81967` (scan-free batch lock push + range release, gated on multi-term query and empty stage map; single-term keeps per-term path).

Shipping config (locks on), criterion medians vs `b092f16b`, raw-C controls flat within 0.4%:

| bench | before (µs) | after (µs) | change |
|---|---|---|---|
| query_each_1_term | 10.798 | 8.248 | -23.3% |
| query_each_4_terms | 3.636 | 3.024 | -16.8% |
| query_each_4_terms_1_write | 3.652 | 3.238 | -11.3% |
| query_each_entity_4_terms | 4.070 | 3.496 | -14.2% |

Lock cost (on vs off, same tree): 1_term +4.9% (was +37.6%), 4_terms +11.6%, 1_write +18.2%, each_entity +9.1%. Soundness of the scan-free push rests on: empty-map precondition, populate-time intra-tuple alias rejection (`tuple_alias`), and `StageLocksScope` unwind restore. Reviewed and approved. 9 new safety tests in `tests/flecs/safety/batch_locks.rs`.

## Phases

1. Perf foundation (done): wrapper specialisation + lock batching, bench-gated.
2. Soundness fixes on the current surface (in progress): audit items that survive any redesign, one commit per fix, one regression test per fix.
3. Prototype (in progress, worktree): guard-based `get`, exclusive register, chunk cursor + `each!`, build-time disjointness, benches vs current API and raw C.
4. Spec finalisation from prototype findings, then full implementation fan-out.

## Open items for the spec

- Tier-0 scope on the shared path: a build-time proof cannot see concurrently held entity guards, so it may only remove intra-query work there; the exclusive path gets the full skip. Prototype must report what is actually sound.
- Worker-thread stage acquisition: how a worker legally obtains a stage handle; promote the scheduler invariant (non-MT systems only run on the `progress()` thread) into a CI-tested contract.
- Defer/staging surface: persistent `Commands` buffer vs per-call token; `mem::forget` on a guard must not wedge defer depth.
- Unsafe-twin catalogue and final naming pass.
