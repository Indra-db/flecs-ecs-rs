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
- Query build-time caches (spec §4.7): the disjointness verdict (a `bool`) and the owning-world identity (`*const ecs_world_t`) are computed once at `build()` and read per iteration as one branch plus one pointer compare. Stable by construction: flecs rejects adding any trait except `With` to a component already queried for (`flecs_trait_can_add_after_query`, `flecs.c:4298`, enforced at `flecs.c:4331`), so no iteration-time assert exists; a CI test pins the C behaviour and is re-verified on vendored upgrades.
- Guard pin counter (spec §7.1): guards touch only the stage lock map and a per-stage pin counter (single-owner per thread, no atomics); the flecs defer level is opened lazily once per write episode, never per guard, so the read path issues zero defer FFI.
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

Built in `flecs_ecs/src/experimental/`: guard-based `get_ref`/`try_get_ref`/`cloned_owned` with fused tuple acquire and rollback, exclusive-register `get_exclusive`/`each_exclusive`, lending `ChunkCursor` + `each!` macro, and the tier-0 disjointness proof. 44 tests green, clippy clean. Verified benches (quiet machine, powermode 0, controls consistent): guard `get` 12.68 ns vs closure `get` 14.36 ns (-11.7%); `each_exclusive` 2.767 µs vs locked `each` 2.989 µs (-7.4%), +4.7% over raw C 2.642 µs; `each!` write 2.751 µs vs locked write 3.224 µs (-14.7%); chunk read cursor 3.57 µs, slower than `each` per the bounds-check friction below.

Validated design facts:

- Tier-0 on the shared register buys nothing beyond batch acquire: a held entity guard is invisible to a build-time proof, so shared-path terms must always register. Tier-0's full skip belongs to the exclusive register only, and even there requires the intra-query disjointness proof (`each_exclusive` falls back to the locked path when unproven; `chunks` refuses, since it hands out `&mut` slices).
- Guards must hold a defer level for their lifetime (storage-pin): structural ops through the shared world are queued until the last guard drops, and observers run at release. This generalises the CPS-`get` semantics to arbitrary lexical scopes and is part of decision 2.
- Fused tuple acquire with rollback and `PendingDefer` unwind cover is panic-safe without any per-batch RAII.

Review findings (orchestrator review of the prototype):

- Cross-world hole found and fixed (`4dc66a81`): `each_exclusive(&mut world)`/`chunks(&mut world)` accepted any world's `&mut`, so exclusivity over world B "proved" access to world A's storage. Both now assert world identity; regression tests added.
- **Spec requirement (hard):** every iteration and access entry point in the final surface must thread a world borrow (`&World`/`&mut World` parameter). A `Query` handle that can iterate without borrowing the world defeats the exclusive register: legacy `q.each(...)` compiles while `get_exclusive`'s `&mut T` is live and takes locks the exclusive path never registered. The prototype coexists with this hazard because it is experimental; the final API removes world-argument-free iteration entirely.

Frictions for the spec (from the prototype):

- ~~Chunk read path pays slice bounds-checks per row~~ Resolved (`a2e3ef7d`): `RowSlice::rows()` exposes native slice iterators and `each!` zips them; measured chunk read 2.85 µs vs locked `each` 3.09 µs, macro read at parity with `each`.
- `Ref`/`Mut` need `Debug where T: Debug` (and likely `Display`) for test ergonomics.
- Guard tuples do not model `Option<&T>`; owned/optional access stays on `cloned`. Decide whether that is final or whether an optional guard element ships.
- Tuple impls are hand-capped at arity 5; move to the crate's `tuples!` macro for the final surface.
- `is_proven_disjoint` should take a safe query handle, not a raw pointer.

## Open items for the spec

- Worker-thread stage acquisition: how a worker legally obtains a stage handle; promote the scheduler invariant (non-MT systems only run on the `progress()` thread) into a CI-tested contract.
- Defer/staging surface: persistent `Commands` buffer vs per-call token; `mem::forget` on a guard must not wedge defer depth.
- Unsafe-twin catalogue and final naming pass.

## Review round, 2026-07-26

Three-reviewer evaluation of the spec draft:

- **Adversarial review** — soundness and contradiction hunt over the draft surface.
- **Design-pattern exploration** — alternative shapes weighed against the register split.
- **Examples migration evaluation** (`design/2026-07-26-examples-migration-evaluation.md`) — 11 real examples rewritten under the spec, scored **BETTER 2 / MIXED 6 / WORSE 3**.

**Defer decomposition measurement.** Guard `get` was timed at **12.68 ns** with a per-guard defer bracket versus **9.75 ns** without it: the bracket is ~23% of the op, paid on every guard. This motivated the pin-counter redesign (defer level opened lazily once per write episode, read path zero defer FFI).

**Adopted amendments** (one line each):

1. Guard pin counter + lazy per-write-episode defer level (replaces the per-guard defer level); read path zero defer FFI. (spec §3.2, §3.4, §3.6, §7.1, §7.2)
2. Disjointness verdict + world identity cached at `build()`; per-call cost is one branch + one pointer compare. (§4.7, invariants)
3. Bundles: `spawn` / `spawn_batch` / `EntityMut::insert` / `Stage::spawn`, one table move via `ecs_bulk_init`. (§4.11, §3.5)
4. `chunks` becomes a true `Iterator` (slices borrow table storage with `'w`); unlocks zip/enumerate/collect and rayon `par_bridge`. (§4.6)
5. `CachedRef<T>` integrated: resolve-once, revalidate-by-table-id repeated single-entity access. (§3.7)
6. Thin opt-in change detection (`detect_changes` / `is_changed` / `skip` / `chunks().changed()`); per-entity tick storage rejected. (§4.12)
7. Typed-tuple query and system construction infallible; `Result` only on the `expr()`/`term_dyn()` typestate. (§4.1, §5.1)
8. `each` names `each_shared` in its doc; §4.4 guidance block; `World::each` / `each_entity` one-liners. (§4.4)
9. `Query::batches` for unproven dense queries; explicit `chunks`/`each!` routing table. (§4.4, §4.5, §4.9)
10. `Iter<'_>` iteration context (delta time, count, entity, observer event metadata); `each_iter` / `each_entity_with`; `Stage` timestep. (§5.6, §5.2, §5.4)
11. Two singleton forms: mutable trait-based term (Tier-1 locked) vs read-only `Singleton<T>` wrapper; `World::singleton` / `singleton_mut`. (§3.5, §4.9)
12. Register-split completions: `World::entity` / `entity_new`, `EntityMut::get_many`, single-optional `get`, pair setters, `progress(&mut self)`, relationship-iteration helpers, id helpers. (§3.4, §3.5, §3.8, §5.7)
13. Diagnostics: `#[track_caller]` + `#[cold]`, `#[diagnostic::on_unimplemented]`, sealed kernel traits, named `each!` error. (§9.5)
14. Migration-table corrections and the `par_each` example requirement; `MAX_TRACKED_TERMS = 64`. (§12, §13)

**Recorded rejections** (considered and declined):

- **WorldScope sessions** — a scoped session object over the world; the register split already scopes access by borrow.
- **Type-level filter typestate** — encoding term filters in the type; too much type machinery for no soundness gain.
- **Generativity-branded worlds** — `GhostCell`-style invariant lifetime brands; ergonomically hostile and unnecessary given `!Clone` + the register split.
- **`SystemParam` / `ParamSet`** (bevy-style) — the item tuple + `Stage` cover the need without a param-injection framework.
- **Per-entity change ticks** (bevy `Changed`/`Added`) — per-row write cost against priority 1; observers/monitors cover it.
- **`IntoIterator` on `&Query`** — would iterate without a world argument, defeating world-threading (§2.3).

**Decision 5 shape upgraded.** From a lending `while let` cursor to a true `Iterator` (§4.6); the substance is unchanged (no `LendingIterator` dependency, chunk is the primitive, row is sugar).

## Implementation waves 1-2, measured (2026-07-26)

Wave 1a (pin counter, commits b09f8108..f67995dd): guard `get` 12.68 -> 9.80 ns (-23%), zero defer FFI on reads; write episodes lazy, one begin/end per episode; world teardown drains leaked episodes (flecs aborts on unbalanced defer at readonly_begin, verified); 22 wrapper call sites of `ensure_write_episode` across 4 files; residual C-internal bypass paths tracked as an audit item. Wave 1b (bundles, merged 29fd5e96): spawn 115.3 vs set*3 169.8 us (-32%), insert -34%, spawn_batch(1000) 17.8 vs 169.5 us (9.5x); ecs_bulk_init verified move-semantics with null desc.table required for OnAdd (flecs.c:7322 added_flags gate); spawn/spawn_batch refuse under live guards or defer (cannot defer without breaking return-ids), insert routes through the write episode. Wave 2 (commits 8912e750..00c6153d): disjointness verdict + world identity cached per query (each_exclusive now +3.0% over raw C); chunks is a true Iterator, single-visit verified in C for plain/sorted/grouped/change-skip (flecs.c:84540-84588 sort merge partitions rows); Query::batches locked lending cursor for unproven dense queries (3.16 us vs 2.96 locked each), lock protocol plain calls in next(), no per-batch Drop, unwind via StageLocksScope. Suite: 2452 passed, 0 failed, clippy clean.
