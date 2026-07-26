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

## Prototype findings (Phase 3, worktree branch `worktree-agent-a384a086b27116b5d`)

Built in `flecs_ecs/src/experimental/`: guard-based `get_ref`/`try_get_ref`/`cloned_owned` with fused tuple acquire and rollback, exclusive-register `get_exclusive`/`each_exclusive`, lending `ChunkCursor` + `each!` macro, and the tier-0 disjointness proof. 44 tests green, clippy clean. Provisional benches (shared machine, controls within 3%): guard `get` ~11% faster than the closure `get`; `each_exclusive` ~10% faster than locked `each`, within ~5% of raw C; `each!` write path ~14% faster than locked `each`.

Validated design facts:

- Tier-0 on the shared register buys nothing beyond batch acquire: a held entity guard is invisible to a build-time proof, so shared-path terms must always register. Tier-0's full skip belongs to the exclusive register only, and even there requires the intra-query disjointness proof (`each_exclusive` falls back to the locked path when unproven; `chunks` refuses, since it hands out `&mut` slices).
- Guards must hold a defer level for their lifetime (storage-pin): structural ops through the shared world are queued until the last guard drops, and observers run at release. This generalises the CPS-`get` semantics to arbitrary lexical scopes and is part of decision 2.
- Fused tuple acquire with rollback and `PendingDefer` unwind cover is panic-safe without any per-batch RAII.

Review findings (orchestrator review of the prototype):

- Cross-world hole found and fixed (`4dc66a81`): `each_exclusive(&mut world)`/`chunks(&mut world)` accepted any world's `&mut`, so exclusivity over world B "proved" access to world A's storage. Both now assert world identity; regression tests added.
- **Spec requirement (hard):** every iteration and access entry point in the final surface must thread a world borrow (`&World`/`&mut World` parameter). A `Query` handle that can iterate without borrowing the world defeats the exclusive register: legacy `q.each(...)` compiles while `get_exclusive`'s `&mut T` is live and takes locks the exclusive path never registered. The prototype coexists with this hazard because it is experimental; the final API removes world-argument-free iteration entirely.

Frictions for the spec (from the prototype):

- Chunk read path pays slice bounds-checks per row where `each` uses pointer adds; the cursor should expose pre-checked iterators (or users push through `iter_mut().zip`) so reads vectorise.
- `Ref`/`Mut` need `Debug where T: Debug` (and likely `Display`) for test ergonomics.
- Guard tuples do not model `Option<&T>`; owned/optional access stays on `cloned`. Decide whether that is final or whether an optional guard element ships.
- Tuple impls are hand-capped at arity 5; move to the crate's `tuples!` macro for the final surface.
- `is_proven_disjoint` should take a safe query handle, not a raw pointer.

## Open items for the spec

- Worker-thread stage acquisition: how a worker legally obtains a stage handle; promote the scheduler invariant (non-MT systems only run on the `progress()` thread) into a CI-tested contract.
- Defer/staging surface: persistent `Commands` buffer vs per-call token; `mem::forget` on a guard must not wedge defer depth.
- Unsafe-twin catalogue and final naming pass.
