# Wave 5 execution plan: the surface swap

Date: 2026-07-27. Base: `232b9514`. Source: planning pass over the spec, design record, and tree; full sub-wave detail lives in the planning transcript, this file records the executable sequence and the orchestrator rulings.

## Ordering law

Enablers first (EntityMut, Stage, builder terminals), semantic migration second (one touch per file region, targeting final names where free and blessed interim names where colliding), mechanical flips last (rename-only commits). Renames must come last: Rust cannot overload the colliding names (`get`, `each`, `each_entity`, `each_iter`, `run`) and inherent methods shadow traits, so coexistence during migration requires the interim spellings that already exist.

## Sub-waves

| id | title | size | isolation |
|---|---|---|---|
| SW-1 | CI contract pack (scheduler ThreadId, get_ptr width) | S | |
| SW-2 | EntityMut + exclusive entity surface (get_many, spawn -> EntityMut) | M/L | |
| SW-3 | Stage<'s>, system `_with` terminals, typed ctx, System::run(&mut World), progress(&mut self), par example | L | review point |
| SW-4 | Panic trampolines: catch-stash-rethrow | M/L | soundness, line-by-line review |
| SW-5 | World: !Clone | S | soundness review |
| SW-6 | QueryHandle Sync narrowing (ReadOnlyTerms) | S | soundness review |
| SW-7 | Builder terminal consolidation (by-value chain) | M | |
| SW-8 | Iteration-surface completion + tuples! arity uplift | M | |
| SW-9 | Region migration: entity-family tests | L | GAP-8 sites reviewed |
| SW-10 | Region migration: query-family + safety tests | L | safety-test intent reviewed |
| SW-11 | Region migration: system/observer/event/addon tests | L | sync-point rewrites reviewed |
| SW-12 | Region migration: examples + docs, big-bang per directory | L | 3 WORSE examples re-scored |
| SW-13 | Flip 1: entity access removal + final renames | M | bench gate: guard get <= 9.8 ns |
| SW-14 | Flip 2: query iteration removal + final renames | M/L | full criterion sweep vs `pre` |
| SW-15 | Flip 3: SystemRunnerFluent, set_context, OnAdd bound, order_by transmute | M | |
| SW-16 | WorldRef demotion | M | feature-combination check |
| SW-17 | Addon + C++-ism sweep (field grid, RestConfig, meta, module identity) | L | |
| SW-18 | Conformance, migration guide, release prep | M | full gate battery |

## Orchestrator rulings (2026-07-27, final)

1. **Builder chaining:** intermediates become `self -> Self`. By-value chaining preserves Decision 7's compile-error guarantee, call-site chains compile unchanged, and the few bind-then-configure loops restructure mechanically (`b = b.with(...)`). Construction is off the hot path.
2. **Trampoline ABI:** keep `extern "C-unwind"` and the build.rs unwind flags after catch-stash-rethrow lands, as defense-in-depth. A bug in the catch layer becomes a defined abort, not UB. The perf baselines already include these flags; revisit only with a measured win.
3. **Interim names:** `each_entity_exclusive` / `run_exclusive` blessed; `each_iter` call sites migrate only at flip SW-14.
4. **GAP-8 rewrites:** immediate-read-back sites move to `EntityMut` / `World::get_mut` unless the test is about deferral. The three WORSE-scored examples must land MIXED or better.
5. **World one-liner cache:** per-world TypeId-keyed cached query (same machinery as the bundle id cache). One-liners are the casual per-frame path; build-per-call would betray pay-for-what-you-use.
6. **OnAdd backstop:** runtime guard stays as backstop behind the type bound. Dynamic `.add_event()` cannot be type-checked; zero hot-path cost.
7. **`.world()` shape:** returns `&'a World`. The Deref transmute becomes a crate-private detail with its layout const_assert. 543 call sites keep compiling; a `&World` is exactly the shared-register handle.
8. **SW-17 scope:** field grid, RestConfig, meta transmutes, module identity are in scope. §14.9 (`IntoId` const-bool -> trait dispatch) is explicitly deferred to a follow-up after SW-18: it sits on every add/set hot path and needs its own bench-gated session. Recorded as debt with a named gate (`has_bench`/`add_remove_bench`/`set_bench`).

## Standing gates for every sub-wave

`cargo +1.97.0 test -p flecs_ecs --test flecs` green, clippy `-D warnings` clean, per-sub-wave bench gates as listed, raw-C controls within drift bounds, no per-batch RAII Drop on iteration paths, no atomics in lock paths, 16-byte `ecs_rust_get_ptr_t` untouched.

## Known debt entering wave 5

- ScriptBuilder::build_from_file/build_from_code on a caller-supplied entity is the one pin-bypass path left open in release (debug net covers it); close or refuse during SW-17's addon sweep.
- §14.9 IntoId dispatch refactor deferred past SW-18 (ruling 8).
