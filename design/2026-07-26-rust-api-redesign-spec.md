# Flecs Rust API redesign: specification

Date: 2026-07-26. Branch: `safety-perf`. Status: spec draft for implementation fan-out.

This document specifies the **clean-break public surface** for the next major version:
one semver bump, no compatibility shims, migration guide instead. It is the
contract the implementers build to and the maintainer reviews against. It builds
on and does not re-open the decision record in
[`2026-07-26-rust-api-redesign-design.md`](./2026-07-26-rust-api-redesign-design.md);
every "locked decision" there is final and is referenced here by number
("Decision N").

Conventions used throughout:

- **Compile error** — the surface is typed so the mistake cannot be written.
- **`Result`** — a *malformed construction* the caller can recover from (bad
  query expression, entity not alive as a constructor input). Absence of a
  component is `Option`, never `Result`.
- **`Option`** — *absence* (entity dead, component not present), never an error.
- **Panic** — a *genuine aliasing bug*: a shared-register borrow conflict, or an
  invariant the borrow checker cannot see being violated. Panics are the last
  resort and are always accompanied by a fallible twin (`try_*`) or an
  `unsafe *_unchecked` twin.

Signatures in this document elide obvious bounds (`T: ComponentId`) except where
a bound is load-bearing for the point being made.

---

## 1. Core model recap

Two registers, split by how uniqueness is proven (Decision 1):

- **Exclusive register** — an entry point that takes `&mut World` proves unique
  access to the whole world at the type level. No runtime lock is taken; plain
  `&T` / `&mut T` are returned and the borrow checker is the lock. Validated by
  `experimental/exclusive.rs`.
- **Shared register** — `&World` and the `Copy` views (`EntityView`) prove only
  shared access, so returns are RAII guards (`Ref` / `Mut`, `!Copy`) that
  register a borrow in the calling stage's lock map
  (`core::safety_map::StageLocks`) and release it on `Drop`. A conflicting
  borrow panics like `RefCell`; `try_*` returns the conflict as a `Result`.
  Validated by `experimental/guard.rs` + `entity_access.rs`.

Shared-register *writes* (`set`, structural ops) are deferred to the next sync
point (Decision 2). Live guards do not each open a flecs defer level; instead a
Rust-side per-stage **pin counter** (next to `StageLocks`, single-owner per
thread, no atomics) counts live guards, and the first shared-register write issued
while the pin is nonzero lazily opens **one** `ecs_defer_begin` that closes when
the pin returns to zero (§7.1). Storage therefore cannot move under a live borrow
(storage-pin), and read-only access pays no defer FFI at all. `World` stops being
`Clone`. Full rationale and measured state are in the design record; this spec
does not restate them.

---

## 2. World and views

### 2.1 `World`

```rust
pub struct World { /* three NonNull fields, #[repr(C)] */ }

// Removed: `impl Clone for World` (today at world/world.rs:40). Cloning a world
// handle aliased the same raw world and defeated the exclusive register.
```

`World` is `!Clone`, `!Send`, `!Sync` (the last two already hold via its raw
pointer fields; they are now part of the contract, asserted by a
`static_assertions`-style compile test). A world is created, borrowed, and
dropped on one thread. Worker threads never receive a `World`; they receive a
stage handle (§6).

`World: !Clone` is the load-bearing change: it is what makes `&mut World` a
genuine proof of exclusivity. Any API that handed out a second owning handle
(the old `Clone`, `WorldRef` used as an owner) is removed or narrowed to a
borrow.

### 2.2 Views

```rust
/// Shared-register handle to a live entity. `Copy`, borrows the world shared.
#[derive(Clone, Copy)]
pub struct EntityView<'w> { /* world ptr + id */ }

/// Exclusive-register handle to a live entity. `!Copy`, borrows `&mut World`.
pub struct EntityMut<'w> { /* &mut-world-derived */ }
```

`EntityView<'w>` is the shared view: obtained from `&World`, it is `Copy` and
its component access returns guards (§3). `EntityMut<'w>` is new: obtained from
`&mut World`, it is `!Copy` (it carries the exclusive borrow) and its component
access returns plain `&T` / `&mut T` and performs structural ops *immediately*
(§3.5).

`DerefMut<Target = Entity>` is **removed** from both views (today at
`entity_view_const.rs:110`). It let `*view = Entity::new(x)` silently repoint a
view at another id, which is a footgun with no legitimate use. `Deref<Target =
Entity>` (read-only id access) is retained; the id is also available via
`view.id()`.

**Cached query facts (invariant).** A built `Query` carries two facts fixed at
`build()`: its disjointness verdict (a `bool`, §4.7) and its owning-world identity
(a world pointer). Per-iteration tier selection and the cross-world guard are then
a single branch and a single pointer compare; these two cached facts join the
invariants list beside the 16-byte `get_ptr` rule (design record).

### 2.3 The hard rule: every entry point threads a world borrow

**Every access and iteration entry point in the public surface takes a
`&World` or `&mut World` parameter (or is a method on `EntityView`/`EntityMut`,
which carry that borrow).** This is the hard spec requirement from the
prototype review: a world-argument-free iterator (`q.each(|..|)` on a stored
`Query`) can run while an exclusive-register `&mut T` is live and take locks the
exclusive path never registered, defeating the register. There is therefore **no
`Query::each(&self, ...)`**. Iteration goes through `each(&mut World)` /
`each_shared(&World)` / `chunks(&mut World)` (§4), and stored query handles
iterate only through `iter_stage(stage)` inside staged execution (§6).

### 2.4 Fate of today's free-floating handle types

| Today | Fate |
|---|---|
| `WorldRef<'w>` (`Deref<Target = World>` via transmute, `world_provider.rs:120`) | Demoted to a crate-private implementation detail. Not in the prelude, not a public return type. The `Deref`-via-`transmute` impl is deleted; call sites take `&World` / `&mut World`. |
| `World: Clone` | Deleted. |
| `Query<T>` used both as owner and iterator | Split: `Query<D>` is the owned, single-thread query; iteration requires a world argument (§4). `QueryHandle<D>` is the sendable narrowing (§4.9, §6). |
| `QueryIter` / `ChainedIter` (public, `!Send`/`!Sync`, single-shot) | Retained as the return of `iter_stage` and of `page`/`worker`, but never obtainable without a world/stage. |

---

## 3. Entity access (shared register)

Replaces the closure-CPS `EntityView::get` (today
`fn get<T>(self, callback: impl FnOnce(T::TupleType) -> R) -> R`,
`entity_view_const.rs:1452`). The CPS form forced every read into a closure and
could not compose with `?` or outlive the callback. The redesign returns guards
whose lifetimes follow ordinary NLL scoping.

### 3.1 Names

The prototype's provisional `get_ref` / `try_get_ref` / `cloned_owned` become
the final `get` / `try_get` / `cloned`:

```rust
impl<'w> EntityView<'w> {
    /// Borrow one or more components, returning guards. Panics on a borrow
    /// conflict (like `RefCell::borrow`); returns `None` if the entity is not
    /// alive or a requested component is absent.
    pub fn get<G: GuardTuple<'w>>(self) -> Option<G::Guards>;

    /// Fallible `get`: the conflict (or missing / not-alive) is returned as an
    /// `AccessError` instead of panicking.
    pub fn try_get<G: GuardTuple<'w>>(self) -> Result<G::Guards, AccessError>;

    /// Owned copy-out; no guard is held after return. The whole call is
    /// `Option` (absent components make it `None`), unlike a live `get`.
    pub fn cloned<T: ClonedTuple>(self) -> Option<T::TupleType<'w>>;
}
```

`G` is `&T`, `&mut T`, or a tuple thereof. `get` panics only on
`AccessError::Conflict`; `NotAlive` / `MissingComponent` map to `None`.

### 3.2 Guard types

```rust
/// Shared immutable access to one component; on drop releases its read borrow and
/// decrements the stage pin (§7.1). Holds no defer level of its own.
/// `Deref<Target = T>`.
pub struct Ref<'w, T> { /* ptr, world, lock key */ }

/// Shared mutable access to one component; on drop releases its write borrow and
/// decrements the stage pin (§7.1). Holds no defer level of its own.
/// `Deref` + `DerefMut<Target = T>`.
pub struct Mut<'w, T> { /* ptr, world, lock key */ }
```

Both are `!Copy`, `!Clone`, `!Send`, `!Sync`. Trait additions over the
prototype (frictions list):

```rust
impl<T: Debug> Debug for Ref<'_, T> { /* forwards to T */ }
impl<T: Debug> Debug for Mut<'_, T> { /* forwards to T */ }
impl<T: Display> Display for Ref<'_, T> { /* forwards to T */ }
impl<T: Display> Display for Mut<'_, T> { /* forwards to T */ }
impl<T: PartialEq> PartialEq for Ref<'_, T> { /* compares *self == *other */ }
// (and for Mut; Eq/PartialOrd/Ord/Hash likewise, all `where T: ...`)
```

No `AsRef`/`Borrow` into `T` beyond `Deref`, to keep the guard from being
accidentally stored in a `&T`-keyed container past its borrow.

### 3.3 Fused tuple acquire

A tuple request acquires all elements in one call with **rollback**: pointers
are resolved, then borrows are taken left-to-right; a conflict on element *i*
releases elements `0..i` already taken this call and returns the conflict. This
is the prototype's `resolve_and_lock` and is retained verbatim in semantics: no
per-batch RAII type, panic cover via `PendingDefer` (§7.2). Acquire order is
left-to-right term order; release order is guard-drop order (reverse of
declaration by Rust's drop rules), which is always a subset of the acquired set
so it cannot underflow.

### 3.4 Guard tuples model `Option<&T>` (decided by user, 2026-07-26)

**Decision:** the guard forms support optional elements:
`get::<(&A, Option<&B>)>()` yields `(Ref<A>, Option<Ref<B>>)` and
`Option<&mut B>` yields `Option<Mut<B>>`. A `None` element means the component
was absent at acquire; non-optional elements still gate the whole acquire
(`None` / `AccessError::MissingComponent`).

Semantics an implementer must honour:

- An absent optional element takes **no lock and does not touch the pin**; only
  present elements register borrows and increment the stage pin (§7.1). The
  fused-acquire rollback and the range release therefore branch per element on
  presence (the acquire records which elements locked and pinned).
- Absence is stable for the guard's lifetime: the component cannot appear while
  any guard from the same acquire lives, because shared-register `add`/`set` is
  deferred and the flush is unreachable until the last guard drops (§3.6). An
  all-absent, all-optional acquire holds nothing and pins nothing, which is
  correct: it borrowed nothing.
- `try_get` reports a conflict only for elements that actually attempt a borrow;
  absent optionals cannot conflict.

A **single optional** needs no tuple: `get::<Option<&T>>()` is itself a valid
`GuardTuple`, yielding `Option<Ref<T>>` (and `Option<&mut T>` -> `Option<Mut<T>>`),
so the one-element `(Option<&T>,)` workaround is unnecessary (GAP-3).

Owned copy-out of optionals stays on `cloned`; the chunk cursor's
`Option<&[T]>` columns (§4.6) are unchanged.

### 3.5 `EntityMut` immediate ops (exclusive register)

```rust
impl<'w> EntityMut<'w> {
    /// Plain reference; the `&mut World` behind this view excludes all other
    /// access, so no lock and no guard. `None` if the component is absent.
    pub fn get<T>(&self) -> Option<&T>;
    pub fn get_mut<T>(&mut self) -> Option<&mut T>;

    /// Structural ops run IMMEDIATELY (not deferred): the exclusive borrow means
    /// no live guard can be pinning storage, so a table move is sound here.
    pub fn set<T>(&mut self, value: T) -> &mut Self;
    pub fn add<T>(&mut self, id: impl IntoId) -> &mut Self;
    pub fn remove<T>(&mut self, id: impl IntoId) -> &mut Self;
    // ...
}
```

The `&mut self`-returning fluent setters on `EntityMut` are safe here (unlike on
builders, §4.1) because they mutate live storage each call and there is no
"build" that can be triggered twice or never. `World::get_mut(&mut self, e)`
(the prototype's `get_exclusive`) is the single-component shortcut:

```rust
impl World {
    pub fn get_mut<T>(&mut self, e: impl Into<Entity>) -> Option<&mut T>;
    pub fn entity_mut(&mut self, e: impl Into<Entity>) -> Option<EntityMut<'_>>;
    pub fn entity_view(&self, e: impl Into<Entity>) -> Option<EntityView<'_>>;
}
```

`World::entity` and `World::entity_new` split entity construction by register:

```rust
impl World {
    /// Shared-register handle. Structural ops on the returned view are deferred
    /// ONLY while a defer scope / guard-pin episode (§7.1) or staging is active;
    /// with no open level flecs applies them IMMEDIATELY. This preserves today's
    /// startup ergonomics: `world.entity().set(..).set(..)` at top level (no guard
    /// live) commits each op at once.
    pub fn entity(&self) -> EntityView<'_>;
    /// Exclusive-register handle for immediate construction (the immediate ops above).
    pub fn entity_new(&mut self) -> EntityMut<'_>;
}
```

**Bundles vs fluent chaining.** The `entity().set().set().set()` startup pattern
performs one archetype move per `set`. `World::spawn(bundle)` / `EntityMut::insert(bundle)`
(§4.11) collapse those into a single table move; the fluent form remains valid and
its top-level ops are immediate, but the docs steer bulk construction toward
`spawn` / `insert`.

**Fused exclusive tuple get.** `EntityMut::get_many` mirrors the shared-register
fused acquire but with **zero locks** (the exclusive borrow is the proof):

```rust
impl<'w> EntityMut<'w> {
    /// Fused multi-component exclusive borrow: `get_many::<(&mut A, &B)>()` ->
    /// `Option<(&mut A, &B)>`. A duplicate mutable request (`(&mut A, &mut A)`) or a
    /// mutable+shared alias of one component is a **compile error** (static
    /// duplicate-mutable check); absence of any requested component is `None`.
    pub fn get_many<G: ExclusiveTuple<'w>>(&'w mut self) -> Option<G::Refs>;
}
```

**Pair access under the register split (GAP-10).** `set_pair`, `set_first`,
`set_second`, and the pair getters follow the same split as single-component
access: on the shared register (`EntityView`) a pair `set` is deferred and checks
the pin (§7.1); on the exclusive register (`EntityMut`) it is immediate. Pair
*reads* return `Ref` / `Mut` guards under `&World` and plain `&`/`&mut` under
`&mut World`, exactly as single-component reads.

**Singletons.** Direct world-level singleton access mirrors the register split:

```rust
impl World {
    /// Shared read of a singleton: `Ref` guard, pin + lock, no defer FFI on the
    /// read path (§3.6). `None` if the singleton component is not set.
    pub fn singleton<T>(&self) -> Option<Ref<T>>;
    /// Exclusive singleton access: plain `&mut T`, no lock (the `&mut World` is the
    /// proof). `None` if not set.
    pub fn singleton_mut<T>(&mut self) -> Option<&mut T>;
}
```

The `Singleton<T>` query wrapper (read-only) and the trait-based singleton *term*
(mutable, Tier-1 locked) are covered in §4.9.

### 3.6 Deferred-write semantics under `&World` (shared register)

Under `&World` / `EntityView`, `set` and structural ops are **deferred** to the
next sync point (Decision 2). The mechanism is a Rust-side **per-stage pin
counter** (§7.1), not a defer level per guard:

- Each live guard increments the stage pin on acquire and decrements it on drop.
  The pin lives next to `StageLocks` in `core::safety_map`, is single-owner per
  thread, and uses no atomics. The **read path touches only the pin counter and the
  lock map: zero defer FFI** on guard acquire and drop.
- A shared-register **write** wrapper checks the pin. If the pin is nonzero and no
  defer level is currently open for the stage, it opens **one** lazy
  `ecs_defer_begin` and marks the level open; subsequent writes in the same episode
  reuse it. The level is closed with `ecs_defer_end` when the pin returns to zero
  (the last guard drops). This is **one begin/end per write episode**, not one per
  guard. If the pin is zero, a shared-register write executes with today's immediate
  semantics (no defer bracket).

The **observable deferred-write semantics are unchanged** from a per-guard-defer
design: while any guard is live the flush is unreachable, so storage cannot move and
the borrowed pointer cannot dangle; queued writes apply, and their `OnSet` / `OnAdd`
observers fire, when the **last guard drops**. A `set` issued while two guards are
live does not fire its observer until both have dropped. This generalises the old
CPS-`get` "observers run after the callback" semantics to arbitrary lexical scopes.

**Read-after-write within a live guard (behavior change, GAP-8).** Because the
write is queued until the pin returns to zero, a read of the same component
*through a still-live guard* observes the **pre-write** value. Code that set a
component and re-read it inside one `get`/`try_get` scope must drop its guards
(reaching a sync point) before the new value is visible; immediate read-back still
requires `&mut World` / `EntityMut` (Decision 2). This is a genuine behavior change
from the CPS API and is listed in the migration guide (§12).

```rust
impl<'w> EntityView<'w> {
    /// Deferred under &World: queued now, applied (and observers fired) when the
    /// last live guard on this stage drops. Opens at most one shared defer level
    /// per write episode (§7.1); a write with no guard live is immediate.
    pub fn set<T>(self, value: T) -> Self;
    pub fn add(self, id: impl IntoId) -> Self;
    pub fn remove(self, id: impl IntoId) -> Self;
}
```

**Soundness obligations (implementation checklist).** Every shared-register
mutation wrapper MUST check the pin before mutating. The required wrapper set:

- `EntityView::set`, `add`, `remove`, `set_pair` / `set_first` / `set_second` and
  the other pair setters (§3.5), pair `add` / `remove`, `enable` / `disable`,
  `is_a` / `child_of` and the relationship-add helpers, `EntityView::destruct`, and
  every other `&World`-register structural or set op;
- the trait-based singleton *term* write on the Tier-1 query path (§4.9) is a
  shared-register mutation and MUST check the pin. (`World::singleton_mut` does not:
  it is exclusive, §3.5.)

Two required safeguards back the checklist:

- a **debug-mode C-shim assert** at op entry: `stage pin > 0` implies
  `stage defer > 0` (a pinned stage with an unopened level is a missed wrapper);
- **one safety test per wrapper** asserting the write is queued under a live guard
  and flushes at last-guard-drop.

### 3.7 `CachedRef`: resolved-once repeated single-entity access

`CachedRef<T>` replaces today's `CachedRef` and is the sub-10 ns story for repeated
reads of one entity's component across frames:

```rust
impl World {
    /// Resolve `e`'s `T` once, caching table + column and the lock key. Not Copy.
    pub fn entity_ref<T>(&self, e: impl Into<Entity>) -> Option<CachedRef<T>>;
}

pub struct CachedRef<T> { /* entity, cached table id + column, lock key; !Copy */ }

impl<T> CachedRef<T> {
    /// Fast path: table-version check (revalidate by table id), then a direct
    /// pointer plus pin + lock. `None` if the entity moved tables and no longer has
    /// `T`, or is dead.
    pub fn get(&self, world: &World) -> Option<Ref<T>>;
    pub fn get_mut(&mut self, world: &World) -> Option<Mut<T>>;
}
```

Resolution and revalidation reuse the existing C shim
`ecs_rust_ref_get_scope_begin` (`flecs_ecs_sys/src/flecs_rust.c:620`), which already
revalidates by table id. On the fast path (entity has not changed tables) `get`
skips re-resolution and does only the table-version check, then hands back a standard
`Ref` / `Mut` guard (pin + lock as in §3.1, no defer FFI on the read path per §3.6);
`get_mut`'s `&mut self` lets it refresh the cached table / column on revalidation.
`CachedRef<T>` is **never `Copy`**: a copied cache could outlive the table
revalidation contract. It is the specified replacement for the prototype's caching
path and for the repeated-single-entity benchmark.

### 3.8 Relationship and hierarchy iteration helpers (GAP-7)

`EntityView::each_child`, `each_target`, and `each_pair` remain closure-based
iteration on the shared view (permitted by §2.3: `EntityView` carries the world
borrow). Their callbacks run on the **shared register**: a guard taken inside a
callback registers in the stage lock map and increments the pin exactly as a
top-level `get` would, and a `set` issued inside is deferred per §3.6. The callbacks
may themselves take guards; conflicts panic (or `try_*`-error) as elsewhere.

---

## 4. Queries

### 4.1 Building

Typed-tuple construction is **infallible**. A statically-typed data tuple `D`
cannot produce `InvalidExpr` or `InvalidTerm` — those failure modes only exist for a
runtime `expr()` string or dynamically-added terms. So the pure-typed path returns a
`Query<D>` directly, and `Result` appears only on the builders that actually took a
runtime expression or a dynamic term:

```rust
impl World {
    /// Infallible typed-tuple query. Convenience constructor: builds and returns the
    /// query. The only residual failure is a C `ecs_query_init` rejection, which for
    /// a well-formed static descriptor is exceptional and **panics** (documented as a
    /// program/environment bug, not a recoverable condition).
    pub fn new_query<D: QueryTuple>(&self) -> Query<D>;
    /// Builder entry for configuration (with/without/term/cached/...).
    pub fn query<D: QueryTuple>(&self) -> QueryBuilder<'_, D>;
}

impl<'w, D: QueryTuple> QueryBuilder<'w, D> {
    /// Terminal for a purely-typed builder. Consumes the builder (double-build and
    /// build-never are compile errors) and returns the query directly. Panics only on
    /// the exceptional `ecs_query_init` rejection.
    pub fn build(self) -> Query<D>;

    /// A runtime query expression. Introduces parse/validation failure modes, so it
    /// transitions the builder to the fallible-build typestate.
    pub fn expr(self, expr: &str) -> FallibleQueryBuilder<'w, D>;
    /// A dynamically-constructed term (id known only at runtime). Same transition.
    pub fn term_dyn(self, term: TermRef<'_>) -> FallibleQueryBuilder<'w, D>;
}

impl<'w, D: QueryTuple> FallibleQueryBuilder<'w, D> {
    /// Terminal for a builder that took `expr()` or a dynamic term. Consuming,
    /// single, and `Result`-returning because the descriptor can be malformed.
    /// (Carries the same configuration methods as `QueryBuilder`.)
    pub fn build(self) -> Result<Query<D>, QueryBuildError>;
}

#[derive(Debug)]
#[non_exhaustive]
pub enum QueryBuildError {
    /// `expr()` string failed to parse.
    InvalidExpr { expr: String },
    /// A dynamic term was malformed (bad id, conflicting modifiers).
    InvalidTerm { index: usize },
    /// The C `ecs_query_init` rejected the descriptor for another reason.
    Init,
}
impl core::error::Error for QueryBuildError {}
```

The typestate makes fallibility **track the actual failure surface**: a query built
entirely from typed terms has no `Result` to thread, while one that took a runtime
`expr()` or `term_dyn()` must handle `QueryBuildError`. This removes today's two
problems — `build(&mut self) -> Self::BuiltType` (`builder.rs:6`) that can be called
twice or never, and a `build` that *panics* on a bad descriptor while `try_build`
returns `Option` and loses the reason — and replaces panic-on-bad-descriptor with a
typed `Result` exactly where a descriptor can be bad. There is no `try_build`.

The intermediate configuration methods (`with`, `without`, `term`, `set_cached`,
...) keep `&mut self` for chaining but are *not* terminal, so the "build twice /
never" hazard exists only at the single by-value `build`. **Id-expression helpers
are unaffected (GAP-11):** `Component::id()`, `id::<T>()`, and `flecs::Wildcard` feed
`.with(...)` / `.term(...)` on both builder typestates exactly as before; the
`Result`/infallible split concerns only the terminal `build`.

### 4.2 The query handle

```rust
pub struct Query<D: QueryTuple> {
    /* owns ecs_query_t refcount; caches the §4.7 disjointness verdict (bool) and
       the owning-world identity (*const ecs_world_t), both set once at build */
}
```

`Query<D>` is the owned query. It is `!Send`/`!Sync` in general (§4.9). It
cannot iterate itself: every iteration takes a world argument (§2.3). `D` is the
query's data tuple (`QueryTuple`); `D::Item<'x>` is the per-row bound tuple
handed to callbacks (today's `QueryTuple::TupleType<'x>`, renamed for clarity),
and `D::Chunk<'x>` is the per-batch column-slice tuple (§4.6).

### 4.3 Lock tiers, and exactly when each applies (Decision 4)

Three tiers, pay-for-what-you-use. The tier is chosen per iteration call, not
per query:

- **Tier 0 — full skip (no lock traffic).** Applies **only** on the exclusive
  register (`each(&mut World)`, `chunks(&mut World)`) **and only** when the
  intra-query disjointness proof (`is_proven_disjoint`, §4.7) passes. The
  `&mut World` proves no outside borrow exists; the proof rules out intra-query
  aliasing. `each(&mut World)` on an *unproven* query falls back to the shared
  batch path (Tier 1) over the same `&mut World`; `chunks` on an unproven query
  is a **panic** (it hands out `&mut` slices with no fallback that could make
  them sound). This matches `experimental/exclusive.rs` and `chunks.rs`.
- **Tier 1 — batch locks (shared path, always registers).** Applies on the
  shared register (`each_shared(&World)`, iteration inside systems/observers) and
  as the exclusive-register fallback. Per-table-batch term borrows are registered
  in the stage lock map with today's semantics (`acquire_batch_locks` /
  `release_batch_locks`): disjoint-table read+write of one component stays legal;
  a held entity guard is detected as a conflict. Tier 0 buys nothing here because
  a concurrently held guard is invisible to a build-time proof (design record,
  prototype finding), so the shared path **always** registers batch locks.
- **Tier 2 — `unsafe` twins (no checks).** `*_unchecked` iteration skips all lock
  bookkeeping; the caller asserts non-aliasing. Catalogued in §9.

No semantic narrowing at any tier: a query legal today is legal in every tier it
qualifies for.

**`MAX_TRACKED_TERMS = 64`** (spec constant). The stage lock map and the
disjointness proof are sized to 64 data terms per query; a query with more data
terms than that falls back to **Tier 1** (batch locks) unconditionally, since the
build-time proof cannot be sized past the constant.

### 4.4 Iteration surface

**Guidance (read first).** Bind the world mutably — `let mut world` — and prefer
`each(&mut world, ..)`: it is the fast, common path and reaches Tier 0 when the
query is proven-disjoint (§4.7). Reach for `each_shared(&world, ..)` **only** while
another shared borrow of the world is live (it always registers Tier-1 batch locks).
`each` on an **unproven** query silently falls back to Tier 1 over the same
`&mut world` (no panic, no Tier 0) — correctness is preserved, only the lock-free
fast path is lost. `chunks` / `each!` are the disjoint-only vectorised path (§4.6);
unproven-but-dense queries use `batches`; non-dense queries route to `each_shared` /
`run` (routing table, §4.5).

```rust
impl<D: QueryTuple> Query<D> {
    /// Exclusive register. Tier 0 when proven-disjoint (cached at build, §4.7), else
    /// Tier 1 over &mut World. The common fast path. Prefer this; reach for
    /// `each_shared` only while another shared borrow of the world is live.
    pub fn each(&self, world: &mut World, f: impl FnMut(D::Item<'_>));

    /// Shared register. Tier 1 (always registers batch locks). Legal while other
    /// shared borrows exist; conflicts panic.
    pub fn each_shared(&self, world: &World, f: impl FnMut(D::Item<'_>));

    /// Row form carrying a lightweight iteration context (`Iter`, §5.6): delta time,
    /// row count, per-row entity, and (for observers) event metadata.
    pub fn each_iter(&self, world: &mut World, f: impl FnMut(Iter<'_>, D::Item<'_>));

    /// Row form carrying the entity.
    pub fn each_entity(&self, world: &mut World, f: impl FnMut(EntityView, D::Item<'_>));
    pub fn each_entity_shared(&self, world: &World, f: impl FnMut(EntityView, D::Item<'_>));

    /// Exclusive register, true `Iterator` of column-slice chunks (§4.6). Panics if
    /// not proven-disjoint (routing, §4.5).
    pub fn chunks<'w>(&self, world: &'w mut World) -> Chunks<'w, D>;

    /// Dense-columns cursor for an UNPROVEN query: registers Tier-1 batch locks per
    /// batch and yields the same slice tuples as `chunks` (§4.5 routing). Conflicts
    /// panic per the shared register.
    pub fn batches<'w>(&self, world: &'w World) -> LockedBatches<'w, D>;

    /// Escape hatch: manual table-batch iteration (replaces today's `run`). Still
    /// world-threaded; the closure is called ONCE with a `TableIter` the body
    /// advances (`while it.next()`), matching today.
    pub fn run(&self, world: &mut World, f: impl FnMut(TableIter<D>));
    pub fn run_shared(&self, world: &World, f: impl FnMut(TableIter<D>));
}
```

`each` taking `&mut World` is the fast, common case; `each_shared` is the opt-in for
iterating while other shared borrows are alive. Rust cannot overload one name on
argument mutability, hence the two names (this is the naming the design task fixed).

**World-level one-liners (GAP-1).** The ergonomic no-builder forms are retained,
world-threaded by construction:

```rust
impl World {
    /// Build-and-iterate in one call:
    /// `world.each::<(&mut Position, &Velocity)>(|(p, v)| ..)`.
    pub fn each<D: QueryTuple>(&mut self, f: impl FnMut(D::Item<'_>));
    pub fn each_entity<D: QueryTuple>(&mut self, f: impl FnMut(EntityView, D::Item<'_>));
}
```

They take `&mut self` (exclusive register) and internally build a cached query and
run `each`, so `world.each::<&Position>(..)` replaces the old world-level one-liner
without a visible builder or a `&mut world` argument thread-through.

### 4.5 The `each!` macro

Adopts the prototype macro (`experimental/mod.rs`), driven by the chunk iterator,
with native `break` / `continue` / `?`:

```rust
each!((pos, vel) in query.chunks(&mut world) {
    pos.x += vel.x;      // pos: &mut Position, vel: &Velocity, per row
});
```

The macro expands to a `for chunk in cursor` over batches (the cursor is now a true
`Iterator`, §4.6) and an inner `for row in 0..len` that binds each column's row.
Because the body is inlined textually, `break`/`continue`/`?` refer to the caller's
control flow. `each!` accepts **either** cursor: `query.chunks(..)` (proven-disjoint,
exclusive) or `query.batches(..)` (unproven, dense, Tier-1 locked, §4.4).

**Named compile error on arity mismatch (§9.5 diagnostics).** A bound-name count
that does not equal the column count emits a **named** macro error ("`each!`: N bound
names but query has M data columns"), not a raw tuple-destructure mismatch.

**Routing (loud caveat).** `chunks` / `each!`-over-`chunks` require a
**proven-disjoint, pure dense, self-sourced** query (§4.7). Anything else does not
qualify:

| Query shape | `chunks` / `each!` | Use instead |
|---|---|---|
| proven-disjoint dense self columns | yes: Tier 0, lock-free slices | — |
| unproven but all-dense self columns | panic | `batches(&world)` (Tier-1 locked slices), or `each_shared` |
| singleton / traversal (`Up`/`Cascade`) data term | panic | `each_shared` / `run` |
| wildcard / sparse / `DontFragment` column | panic | `each_shared` / `run` |

`chunks` on a non-qualifying query **panics** (it would hand out `&mut` slices with
no sound basis); `batches` covers the dense-but-unproven case with per-batch locks;
non-dense terms route to `each_shared` or the manual `run` loop.

**IDE note.** Inside `each!` the body is macro-expanded, so IDE assistance
(completion, inline types) is limited. The closure forms (`each` / `each_entity` /
`each_iter`) remain first-class and are the recommended path for IDE-heavy
workflows.

### 4.6 Chunk iterator: pre-checked slice iterators, true `Iterator`

`chunks` returns a **true `Iterator`** (not a lending cursor). This upgrades the
*shape* of Decision 5 (recorded explicitly: still **no `LendingIterator`
dependency**, chunk still the primitive, row still sugar) from a `while let` lending
cursor to `impl Iterator`:

```rust
pub struct Chunks<'w, D> { /* borrows &'w mut World */ }

impl<'w, D: QueryTuple> Iterator for Chunks<'w, D> {
    /// Whole-column slice tuple: `(&'w mut [A], &'w [B], Option<&'w [C]>, ..)`. The
    /// yielded slices borrow the TABLE STORAGE with the cursor's `'w` (from
    /// `&'w mut World`), NOT the cursor, so successive chunks do not alias and the
    /// item outlives a `next()` call.
    type Item = D::Chunk<'w>;
    fn next(&mut self) -> Option<Self::Item>;
}

impl<'w, D: QueryTuple> Chunks<'w, D> {
    /// Terminal, by value: cannot be reused (double-consume is a compile error).
    pub fn for_each(self, f: impl FnMut(D::Chunk<'w>));
    /// Change-detection adapter (§4.12): skips clean batches.
    pub fn changed(self) -> ChangedChunks<'w, D>;
}
```

Non-overlap of the yielded `&'w mut [_]` slices is guaranteed by two facts together:
the **proven-disjoint gate** (`chunks` panics unless the query passed the cached §4.7
proof), and the pinned invariant **"one query iteration visits each `(table,
row-range)` at most once"**. That invariant is promoted to a **CI-tested contract**
listed beside the §6.3 scheduler contract: it is re-verified on every vendored-C bump
and must cover sorted, grouped, and change-skipped iteration. If it cannot be
established for sorted / grouped queries on a given C version, the `Iterator` impl is
**scoped to unsorted / ungrouped queries** and sorted / grouped `chunks` fall back to
the lending `while let` form.

Being a real `Iterator` (with `Send` slices when `T: Send`) unlocks the std
combinators and rayon:

- `zip` / `enumerate` / `sum` / `collect` compose directly on the chunk stream;
- `chunks(..).par_bridge()` (rayon) parallelises the chunk stream, sound because each
  `Item` is a distinct non-overlapping slice set and slices are `Send` when `T: Send`.

`for_each` stays the **by-value terminal** (cannot be reused); `each!` is unchanged in
spelling and now expands to a plain `for chunk in query.chunks(..)`.

**Friction fix (prototype), retained:** the yielded columns are real `&[T]` /
`&mut [T]` slices (not an index-and-bounds-check accessor), so
`chunk.0.iter_mut().zip(chunk.1)` gets pointer-add codegen with the bounds check
hoisted out of the row loop, matching `each`'s pointer arithmetic. The `each!` macro's
inner loop indexes `0..len` where `len` is read once per chunk, so LLVM elides the
per-row bounds check. The cursor rejects ref / inherited / sparse columns
(`ref_fields | up_fields | row_fields != 0` → panic): those are not plain dense self
columns and cannot be handed out as slices (routing, §4.5).

**Singleton-iteration edge case.** When a batch has `count == 0 && table.is_null()`
(the singleton-only iteration flecs emits for a data-free / singleton match), `next()`
yields **nothing** from chunks: there are no dense self columns to slice.

### 4.7 Disjointness proof, computed once at `build()` (friction fix)

The disjointness verdict **and** the query's owning-world identity are computed
**once at `build()`** and cached on `Query` — a `bool` plus a world pointer — so
per-iteration work is one branch (read the cached bool) plus one pointer compare
(cached world ptr vs the passed `&World`):

```rust
impl<D: QueryTuple> Query<D> {
    /// The cached build-time verdict: true when the query's data terms provably
    /// address pairwise-distinct, dense, self-sourced storage (so a lock-free
    /// exclusive run cannot alias). Conservative: `false` means "not proven", never
    /// "known aliasing". O(1): returns the cached bool.
    pub fn is_proven_disjoint(&self) -> bool;
}
```

The analysis itself is `experimental/disjoint.rs` unchanged: rejects wildcards,
non-`$this`/`Self`-traversal sources, non-`And` operators, sparse / `DontFragment`
storage, and any two data terms sharing a concrete id; ignores tag terms. It now runs
at build against a safe `&World`, not per call over a raw `*const ecs_query_t`
(prototype friction). `each` / `chunks` read the cached bool and pointer-compare the
world; that is their entire tier-selection cost. `is_proven_disjoint` stays public so
callers can branch before choosing `chunks` vs `each_shared`.

**Caveat pinned.** A component that gains a `Sparse` or `DontFragment` trait *after* a
query is built against it would invalidate the cached proof. The implementation must
**debug-assert storage traits are unchanged at iteration time**, and the spec
documents the contract: **storage traits are fixed before the first query is built
against them.** Both cached facts join the invariants list beside the 16-byte
`get_ptr` rule (design record).

### 4.8 Tuple arities via `tuples!` (friction fix)

The prototype hand-capped guard, chunk, and disjoint tuple impls at arity 5. The
final surface generates them with the crate's existing `tuples!` macro (same
macro that drives `QueryTuple` / `GetTuple`), so `GuardTuple`, `ChunkColumns`,
and `GuardElement` cover the full supported arity uniformly. No behavioural
change, only coverage.

### 4.9 Multi-source terms, and the two singleton forms

**Traversal terms are read-only by construction.** `Up<T>` / `Cascade<T>` name
storage the current entity does not own, so they are immutable-only in the type
system:

```rust
Up<T>        // -> &T only; `Up<&mut T>` does not implement the query-term trait
Cascade<T>   // -> &T only
```

Requesting `&mut` through `Up` / `Cascade` is a **compile error** (the mutable term
impl is simply not provided for these wrappers), not a runtime guard. This removes an
entire class of aliasing (two entities sharing an inherited component and both writing
it).

**Singletons have two forms (reconciliation).**

- **Trait-based singleton term** — a plain `&Gravity` / `&mut Gravity` where `Gravity`
  carries the `Singleton` trait. This **remains a legal query term, including
  mutable**, resolved with **runtime Tier-1 locking** (the write is registered in the
  stage lock map like any shared-register term, and its wrapper checks the pin, §3.6).
- **`Singleton<T>` wrapper** — the explicit-source **read-only** form. `Singleton<T>`
  yields `&T` only; `Singleton<&mut T>` (mutable through the wrapper) **does not
  compile**. Use it when the source is explicit and the access is a read.

Either way, a singleton-bearing query **never passes the disjointness proof** (§4.7
rejects it), so it always runs **Tier 1** and **panics under `chunks`** (it is not a
pure dense self query). Under `each!` it routes to `each_shared` / `run` per §4.5; the
trait-based mutable form takes its Tier-1 write lock on the batch path.

### 4.10 `Query` / `QueryHandle` auto-traits (Decision 3)

```rust
// Query<D>: owned, single-thread. Neither Send nor Sync in general.

// QueryHandle<D>: the cross-thread narrowing (staged execution only).
unsafe impl<D> Send for QueryHandle<D>
where for<'w> D::Item<'w>: Send {}

unsafe impl<D> Sync for QueryHandle<D>
where D: ReadOnlyTerms {}   // NARROWED: Sync only when ALL terms are read-only
```

This narrows today's impl (`query.rs:555`), which grants `Sync` under the same
`Item: Send` bound as `Send`. Per Decision 3, `Sync` now additionally requires
every term read-only (`ReadOnlyTerms`, a marker auto-derived for tuples of `&T`
/ `Up<T>` / tags, not satisfied when any `&mut T` is present). Cross-thread
*mutation* goes through partitioned `par_*` systems (§6) or `unsafe`. This
reverts part of `aa282cef` as the design record notes.

`QueryHandle::iter_stage(stage)` (retained) is the only way to iterate a handle
off the owning thread, and only inside flecs staged execution where the world is
read-only for the duration.

### 4.11 Bundles

A `Bundle` is a set of components inserted as one archetype move. `trait Bundle` is
implemented for component tuples via the crate's `tuples!` macro (the same macro that
drives `QueryTuple`), so `(A, B, C, ..)` is a `Bundle` up to the supported arity.

```rust
pub trait Bundle { /* sealed; component-set → one table */ }

impl World {
    /// Exclusive, IMMEDIATE construction of one entity from a bundle: ONE table move
    /// via the vendored `ecs_bulk_init` path (count = 1). Returns the new entity as an
    /// `EntityMut` for further immediate ops.
    pub fn spawn<B: Bundle>(&mut self, bundle: B) -> EntityMut<'_>;
    /// Bulk construction of `n` entities from one bundle via `ecs_bulk_init`.
    pub fn spawn_batch<B: Bundle>(&mut self, bundle: B, n: usize)
        -> impl Iterator<Item = Entity> + '_;
}

impl<'w> EntityMut<'w> {
    /// Batch add/set of a whole bundle in ONE table move (not N).
    pub fn insert<B: Bundle>(&mut self, bundle: B) -> &mut Self;
}

impl<'s> Stage<'s> {
    /// Deferred twin of `World::spawn`: enqueues the bundle's commands on the stage;
    /// flecs defer-merge batches same-entity commands into one move at the sync point.
    pub fn spawn<B: Bundle>(&self, bundle: B) -> EntityView<'_>;
}
```

A per-world **bundle → table cache** (keyed through the §8.2 per-world component index
machinery) resolves the target table once per bundle type per world. This replaces **N
archetype moves with one** for the ubiquitous `entity().set().set().set()` startup
pattern; the fluent form remains valid but the docs steer bulk construction toward
`spawn` / `insert`. **Hook / observer ordering parity** with the per-component `set`
sequence (the order and set of `OnAdd` / `OnSet` observers fired) must be pinned by
tests.

### 4.12 Change detection (thin, opt-in)

Change detection is an **opt-in** builder flag over flecs' existing per-query /
per-table change tracking, not per-entity tick storage:

```rust
impl<'w, D: QueryTuple> QueryBuilder<'w, D> {
    /// Opt in to change detection for this query (`ecs_query_changed` machinery).
    pub fn detect_changes(self) -> Self;
}

impl<D: QueryTuple> Query<D> {
    /// Whether the query's matched tables changed since last iteration
    /// (`ecs_query_changed`). Requires `detect_changes()`.
    pub fn is_changed(&self, world: &World) -> bool;
}

impl<'a, D> TableIter<'a, D> {
    /// Whether the current batch changed since last iteration.
    pub fn is_changed(&self) -> bool;
    /// Skip the current batch. `skip` MUST NOT mark write columns dirty (it declares
    /// "I did not write this batch").
    pub fn skip(&mut self);
}
```

`Chunks::changed()` (§4.6) is the iterator adapter that skips clean batches and
composes with the true-`Iterator` chunk stream.

**Rejected: bevy-style per-entity `Changed` / `Added` tick storage.** Per-row change
ticks impose a per-row write cost that violates priority 1 (the hot path), and flecs
observers / monitors already cover per-entity change reaction. The rejection is
recorded so the decision is durable: change detection is per-query / per-table only.

---

## 5. Systems and observers

### 5.1 Terminal build: infallible on the typed path, two-phase init

The pure-typed system path is **infallible**, mirroring §4.1: a statically-typed `D`
cannot produce an invalid descriptor, so the terminal returns `System` directly. The
residual `ecs_system_init` failure is exceptional and **panics** (documented), not a
`Result`.

```rust
impl<'w> SystemBuilder<'w, D> {
    pub fn each(self, f: impl FnMut(D::Item<'_>) + 'static) -> System;
    pub fn each_entity(self, f: impl FnMut(EntityView, D::Item<'_>) + 'static) -> System;
    pub fn each_iter(self, f: impl FnMut(Iter<'_>, D::Item<'_>) + 'static) -> System;
    pub fn run(self, f: impl FnMut(TableIter<D>) + 'static) -> System;
}
```

Only a builder that took a runtime `expr()` or a dynamic term transitions to the
fallible typestate (`FallibleSystemBuilder`), whose terminals return
`Result<System, SystemBuildError>`. This matches the query split in §4.1: the `Result`
appears exactly where a descriptor can be malformed and nowhere else.

**Two-phase init (kept):** the closure is boxed and its context installed *only after*
`ecs_system_init` returns a valid system entity. A failed init must not leak the
closure. This is already correct on the current surface (`system_builder.rs` builds the
entity first, checks `id() != 0`); the spec locks it in as "closure ownership transfers
on success, is dropped on failure", enforced by construction (the boxed closure lives
in a local that is only handed to the C context ptr after the id check).

### 5.2 Closures are `'static`; world access is opt-in

System/observer closures are `'static` (they outlive the build call and run
later). They get world access **only** through:

- the iteration **item** (the `D::Item` handed per row/batch), or
- an explicit `.each_with(|item, stage|)` opt-in:

```rust
impl<'w> SystemBuilder<'w, D> {
    /// Opt-in world access. `stage` is the callback's stage handle (§6), through
    /// which deferred commands are issued. Enabling this makes the system register
    /// its term locks (the item alone can be served lock-free on the tiered path when
    /// disjoint; adding arbitrary stage access cannot). Infallible on the typed path.
    pub fn each_with(self, f: impl FnMut(D::Item<'_>, Stage<'_>) + 'static) -> System;

    /// Entity + item + stage (GAP-6): the "visit each matched entity and issue a
    /// deferred command on it" shape, which `each_entity` (no stage) and `each_with`
    /// (no entity) could not express together.
    pub fn each_entity_with(self, f: impl FnMut(EntityView, D::Item<'_>, Stage<'_>) + 'static)
        -> System;
}
```

`.each` (no stage) can qualify for the lock-free tiered path; `.each_with` /
`.each_entity_with` always register this system's term locks, since the stage handle
can reach arbitrary storage the disjointness proof does not cover.

### 5.3 Context: capture, not `*mut c_void`

`set_context(*mut c_void)` is **deleted** from the system/observer surface
(today `system_api.rs:42`, `observer.rs:133`, `system/mod.rs:124`). The
`*mut c_void` context and its `ecs_ctx_free_t` were an FFI shim for what Rust
closures do natively. Two replacements:

- **Capture** — the closure captures whatever it needs; the box is owned by the
  system and dropped with it. This is the common case and needs no API.
- **Typed context** — for state a caller wants to read back or share, a typed
  setter:

```rust
impl<'w> SystemBuilder<'w, D> {
    /// Store a typed value owned by the system; retrievable via `System::ctx`.
    pub fn ctx<C: 'static>(self, value: C) -> Self;
}
impl System {
    pub fn ctx<C: 'static>(&self) -> Option<&C>;      // downcast, None on type mismatch
    pub fn ctx_mut<C: 'static>(&mut self) -> Option<&mut C>;
}
```

`World::set_context` (the world-level `*mut c_void`, `operations.rs:1031`) is
likewise replaced by a typed `World::set_ctx<C>(C)` / `World::ctx<C>()`. No
`*mut c_void` survives in the public surface.

### 5.4 `OnAdd` observers accept only data-free tuples (type error)

```rust
impl World {
    /// `E` bounds the event; for `OnAdd`, `D` is bounded `D: DataFreeTerms`,
    /// so `observer::<OnAdd, &Position>()` DOES NOT COMPILE.
    pub fn observer<E: ObserverEvent, D: QueryTuple>(&self) -> ObserverBuilder<'_, E, D>;
}
```

For `OnAdd`, component data is not yet initialised, so an observer that fetches
`&T` reads uninitialised memory. Today this is a runtime guard
(`observer_builder.rs:220`: "don't fetch components for OnAdd"). The clean-break
surface makes it a **type error**: `OnAdd`'s builder bounds `D: DataFreeTerms`
(tuples of tags / `EntityView` only, no `&T` / `&mut T`), so a data term with
`OnAdd` fails to compile rather than being silently skipped.

**Observer terminals.** The observer builder's terminals are `each`, `each_entity`,
`each_iter`, and `run` (same shapes as systems, §5.1). Event metadata is reached
through `Iter` (`event()`, `event_id()`, `pair()`, §5.6) on `each_iter`, or through
`TableIter` on `run`; an observer that only reacts (no metadata) uses `each` /
`each_entity`. This gives event-inspecting observers an `each`-shaped path and removes
the forced drop to a manual `run` loop.

### 5.5 Panic policy across the C boundary (Decision 8)

A panicking system/observer/query callback is:

1. **caught at the trampoline** (`catch_unwind` around every Rust callback
   invoked from C),
2. **stashed** in the world context (`WorldCtx::is_panicking` + the payload),
   the stage lock map restored (`StageLocksScope`) and any `ecs_table_lock`
   balanced,
3. **rethrown from `progress()`** (or the enclosing safe entry point) after
   flecs has returned and unwound its own frames normally.

A panic never unwinds through a C frame. This is the invariant from the design
record ("no unbalanced lock counters or `ecs_table_lock`", "Panics caught at the
trampoline"); the spec requires every new trampoline (each / each_iter /
each_entity / each_with / each_entity_with / run / par_each / observer) to route
through the same catch-stash-rethrow path, tested by the panic-cleanup safety
suite (§13).

### 5.6 Iteration context: `Iter<'_>`

`Iter<'_>` is the lightweight per-invocation context handed by the `*_iter` terminals
(`Query::each_iter`, `SystemBuilder::each_iter`, observer `each_iter`), replacing the
deleted `each_iter(|it, index, item|)` triple with a typed context object:

```rust
pub struct Iter<'a> { /* borrows the running iteration, !Send/!Sync */ }

impl<'a> Iter<'a> {
    pub fn delta_time(&self) -> f32;
    pub fn delta_system_time(&self) -> f32;
    pub fn count(&self) -> usize;
    /// The entity for the row currently bound by the callback.
    pub fn entity(&self) -> EntityView<'a>;

    // Observer-only event metadata (present when iterating an observer):
    pub fn event(&self) -> EntityView<'a>;
    pub fn event_id(&self) -> Id;
    pub fn pair(&self, index: i8) -> Option<Id>;   // matched pair metadata
}
```

`Iter` carries the row's entity and (for observers) the event metadata that previously
only `TableIter` exposed, so pair- and event-inspecting iteration (wildcard queries,
observers) no longer has to drop to a manual `run` loop.

**Timestep on the stage.** `Stage<'s>` also exposes `delta_time()` and `count()` so
`each_with` / `each_entity_with` cover timestep-driven systems without an `Iter`:

```rust
impl<'s> Stage<'s> {
    pub fn delta_time(&self) -> f32;
    pub fn count(&self) -> usize;
}
```

### 5.7 Running systems and `progress` (GAP-9)

`World::progress` takes **`&mut self`**: `world.progress(delta)` is an exclusive frame
step. A stored `System` runs via `System::run(&mut world)` (the `SystemRunnerFluent` is
deleted, §14.7). Both are exclusive-register calls, so **mixing manual `run` and
`progress` in one frame is legal** — they serialise on the `&mut World` borrow, and a
stored `System` / `Query` handle coexists with the `&mut world` iteration borrow
because each call borrows the world for its own duration and releases it. There is no
`&World` / `&mut World` split across the two: both are `&mut World`.

---

## 6. Multithreading and staging

### 6.1 How a worker thread legally obtains a stage handle

Workers never hold a `World`. A worker obtains a **stage handle** only from
inside a `par_*` system callback, from the view flecs hands it:

```rust
pub struct Stage<'s> { /* raw stage world ptr, !Send/!Sync, tied to 's */ }

// Inside a par callback, `item`/`entity`/`table_iter` expose `.stage()`:
world.system::<&mut Position>()
    .multi_threaded()
    .par_each_with(|pos, stage: Stage<'_>| { /* stage-scoped deferred ops */ })?;
```

`Stage<'s>` is `!Send`/`!Sync` and lives only for the callback invocation. Its
lock map is that worker's own stage map (`stage_locks::<true>` resolves per
worker; `safety_map.rs` guarantees each stage map is touched by exactly one
thread, so no atomics, no false sharing). A worker cannot manufacture a stage
outside a callback: `Stage` has no public constructor.

### 6.2 `par_each` partitioning

```rust
impl<'w> SystemBuilder<'w, D> {
    // Infallible on the typed path (§5.1); the fallible typestate returns Result.
    pub fn par_each(self, f: impl Fn(D::Item<'_>) + Send + Sync + 'static) -> System
        where for<'x> D::Item<'x>: Send;
    pub fn par_each_with(self, f: impl Fn(D::Item<'_>, Stage<'_>) + Send + Sync + 'static)
        -> System
        where for<'x> D::Item<'x>: Send;
}
```

flecs partitions matched tables across workers so each row is visited by exactly
one worker (the `ecs_worker_iter` split). The `D::Item: Send` bound is what makes
handing a `&mut T` to another thread sound; `&T` needs `T: Sync`, `&mut T` needs
`T: Send`, exactly as today. Cross-partition aliasing is impossible by the
partition; intra-worker aliasing is caught by that worker's stage lock map.

### 6.3 Scheduler invariant promoted to a CI-tested contract

The soundness of per-stage (non-atomic) lock maps rests on the flecs C invariant
that **non-multithreaded systems only run on the `progress()` thread**
(`world_ctx.rs:153`: `flecs_run_pipeline_ops: assert(!stage_index ||
op->multi_threaded)`). Today this is a code comment ("Re-verify on every vendored
flecs C upgrade"). The spec promotes it to a **CI contract**:

- a test registers a mix of single- and multi-threaded systems, records the
  `ThreadId` each callback runs on, and asserts every non-`multi_threaded` system
  ran on the `progress()` thread and every worker ran off it;
- the test is part of the required suite (§13) and gates the vendored-C bump.

This makes the invariant a checked contract rather than a comment, so a flecs
upgrade that changes scheduling fails CI instead of silently unsoundening the
lock maps.

### 6.4 `QueryHandle` narrowing for cross-thread patterns

`QueryHandle<D>` (§4.10) is the only query type that crosses threads, and only
`Send` when `Item: Send`, `Sync` only when read-only. Off the owning thread it
iterates solely via `iter_stage(stage)` inside staged execution. A stored query
shared read-only across a `par` scope is `Sync` and needs no lock beyond the
per-stage batch locks; a stored query used for writes off-thread requires either
partitioned `par_*` or an `unsafe` twin — the type system refuses the naive
shared-mutable pattern.

---

## 7. Defer, staging, Commands

### 7.1 Guard pin counter and lazy defer level

Guards do **not** each hold a flecs defer level. A shared-register guard touches only
two Rust-side structures on acquire and drop: the stage lock map and a per-stage **pin
counter** that lives next to `StageLocks` in `core::safety_map`, single-owner per
thread, no atomics. *k* live guards → pin = *k*. The **read path issues zero defer
FFI**: `get` / `try_get` acquire and guard drop increment and decrement the pin and
touch the lock map, nothing else.

The flecs defer level is opened **lazily, once per write episode**, by the
shared-register write wrappers (§3.6), not by the guards:

- a write wrapper checks the pin; if pin > 0 and no level is open for the stage, it
  opens exactly one `ecs_defer_begin` and marks the level open;
- the level is closed with `ecs_defer_end` when the pin returns to zero (the last
  guard drops); intervening writes reuse the open level;
- if pin == 0 the write executes immediately (today's semantics), with no bracket.

Structural ops through `&World` while a guard is live are therefore queued behind this
single episode-scoped level; the borrowed pointer stays valid because the flush is
unreachable until every guard drops (storage-pin). The queue flushes, and the observers
behind it fire, when the pin returns to zero.

**Why the pin, not per-guard levels.** The defer decomposition measurement of
2026-07-26 timed guard `get` at **12.68 ns** with a per-guard defer bracket versus
**9.75 ns** without it: the bracket is ~23% of the op, paid on every guard whether or
not it writes. The pin design pays the bracket only on write episodes and recovers the
full ~23% for all read-only accesses (the dominant case), while preserving the
observable deferred-write semantics of §3.6 exactly.

**Soundness obligations.** Enumerated as an implementation checklist in §3.6: every
shared-register mutation wrapper checks the pin; a debug-mode C-shim assert enforces
`stage pin > 0` ⇒ `stage defer > 0` at op entry; one safety test per wrapper asserts
queue-under-guard / flush-at-last-drop.

### 7.2 `mem::forget` on a guard must not wedge the world (specified)

If a caller `mem::forget`s a guard, its `Drop` never runs, so its read/write
borrow is **never released** and the stage pin it incremented is **never
decremented**. The specified, sound behaviour:

- **The borrow leaks, not corrupts.** The forgotten borrow stays registered in
  the stage lock map. Subsequent conflicting access to that same storage will
  *panic* (or `try_*`-error) forever — a leak that fails safe, never a
  use-after-free. The stage map is a plain counter; a stuck counter denies access,
  it does not alias.
- **The pin leaks, not corrupts.** The forgotten guard leaves the stage pin
  permanently above zero (a monotonic Rust counter, exactly as the borrow counter
  is). A pin stuck above zero can only keep write episodes on the deferred path; it
  can never alias or grant access.
- **A defer level leaks only if a write episode was open.** Because levels are now
  opened lazily per write episode (§7.1), a forgotten *read* guard with no write
  issued leaks **no** C defer level at all — it leaks only the Rust pin. A guard
  forgotten while a write episode's level is open leaks that one open level, which
  can only *delay* the flush and the observers behind it, never alias or grant
  access, so no UB is reachable. Whether `progress()`'s pipeline sync points drain
  the world queue despite an unbalanced level is deliberately not asserted here: the
  implementation must pin the actual behaviour with a test (leak a guard, run
  `progress()`, assert either drain-at-sync or delay-until-close, and assert no abort
  at world destruction), and the guard-type docs must state whichever behaviour is
  pinned.

This is sound because every leaked resource is a *monotonic denial* (a stuck lock
denies; a stuck pin keeps writes deferred; a stuck level delays), never a grant. The
prototype's `PendingDefer` (`entity_access.rs:30`) guarantees the *acquire* path is
balanced on panic; the `mem::forget` case is the caller's explicit leak and the spec
commits to "fails-safe, does not wedge" as the contract. No `mem::forget`-detection
code is added (it would cost the hot path); the guarantee is structural.

### 7.3 Commands: per-call token, not persistent buffer (decided)

**Decision:** systems get a **per-call stage token** (`Stage<'s>`, §6.1),
obtained through `.each_with` / `.par_each_with`, not a persistent `Commands`
buffer stored on the system.

Rationale:

- A persistent `Commands` buffer would have to outlive sync points and reconcile
  with the world's own defer stack and with the guard pin's episode-scoped defer
  level (§7.1), duplicating bookkeeping the flecs defer queue already owns.
- The per-call token scopes command issuance to the callback invocation, so it
  composes cleanly with the guard pin / write-episode level and with the
  panic-cleanup path (the token cannot outlive the `catch_unwind`).
- Deferred ops through `Stage` are just entries on the world's existing defer
  queue; there is no separate replay step to own.

`Stage<'s>` therefore *is* the "commands" surface: `stage.entity()`,
`stage.entity(e).set(..)`, `stage.defer(|| ..)` all enqueue onto the stage's
defer queue and flush at the next sync point.

### 7.4 Sync points

A sync point is where the deferred queue drains and observers fire:

- end of a `progress()` pipeline phase (between systems, per flecs merge points);
- the stage pin returning to zero at last-guard-drop (§3.6, §7.1), which closes the
  episode-scoped defer level opened by the write wrappers;
- an explicit `world.defer_end()` / scope exit at the top level.

The spec does not change flecs merge semantics; it only pins the Rust-visible
rule that a shared-register write is observable no earlier than the enclosing
sync point.

---

## 8. Registration

### 8.1 No fabricated `&'static mut`

The `impl ComponentId for &'static mut T`
(`registration_traits.rs:830`) that let a `&'static mut T` masquerade as a
component identity is **removed** from the public surface. Component access
never fabricates a `'static` lifetime; the replacement is the register-split
access pattern:

- read/write access is `Ref`/`Mut` (shared) or `&T`/`&mut T` bound to the world
  borrow (exclusive), never a `&'static mut`;
- where the query DSL needs to name "mutable term of `T`", it uses the term
  wrapper types (`&mut T` as a *query term type*, resolved through `QueryTuple`),
  which never materialise a `'static` reference — the lifetime is always the
  iteration borrow.

### 8.2 Generic component identity: per-monomorphization, TypeId-keyed (locked)

Component identity is **per-world, per-monomorphization**:

- each concrete `T` gets a per-world numeric id, cached at a per-monomorphization
  index into the world's `components_array` (`registration_traits.rs`), and
  mirrored in a `TypeId`-keyed `components_map`
  (`insert(TypeId::of::<Self>(), id)`);
- the index is **per-world**, not a shared process-global static. The earlier
  "shared-static index" bug (two worlds colliding on one static slot) is already
  fixed; the spec **locks it in**: identity is resolved through the world's own
  array/map, so generic components (`Wrapper<A>` vs `Wrapper<B>`) and multiple
  worlds never alias ids.

### 8.3 Module identity without `type_name`

Module identity must not use `core::any::type_name::<T>()` as a key
(`type_name` is explicitly documented as not-for-identity; it is unstable across
compilers and can collide). Modules are identified the same way components are:
a `TypeId`-keyed per-world registry entry, with the human-readable name derived
from the `Component` derive's declared name, not from `type_name`. (The
`type_name` uses that remain, in `get_tuple.rs` / `query_tuple.rs` /
`cloned_tuple.rs`, are **diagnostic strings in panic messages only**, never
identity keys; those are allowed.)

### 8.4 Enum registration recursion

Enum component registration recurses to register the underlying integer type,
which can grow and reallocate the world's `components_array`. The invariant is:
**no `&mut` into the components array is held across the recursive registration
call** (`registration_traits.rs:186`, `:331`). The array is re-borrowed after the
recursion returns. The spec locks this in: enum registration is
`register self id → recurse to register underlying → re-borrow array → write id`,
so a reallocation during recursion cannot dangle an outer `&mut`.

---

## 9. Error model, naming, and the unsafe-twin catalogue

### 9.1 Error model (Decision 6)

- `Option<T>` — absence: entity not alive, component not present. Returned by
  `get`, `get_mut`, `cloned`, `entity_view`, `entity_mut`.
- `Result<T, E>` — malformed construction, only where a descriptor can actually be
  malformed: the `expr()`/`term_dyn()` fallible typestate's
  `build() -> Result<_, QueryBuildError>` and `FallibleSystemBuilder`'s terminals
  `-> Result<_, SystemBuildError>` (§4.1, §5.1), and `try_get() -> Result<_,
  AccessError>` (the fallible borrow). The purely-typed query/system build is
  infallible.
- **Panic** — genuine aliasing bug only: a shared-register conflict through the
  panicking entry points (`get`, `each_shared`), or an exclusivity invariant the
  borrow checker cannot see (world-identity mismatch in `each`/`chunks`,
  `is_proven_disjoint` violated for `chunks`). Every panicking op has a `try_*`
  and/or `_unchecked` twin.

Safety is **never a Cargo feature**. `flecs_safety_locks` stays a build-time
*performance* knob for the always-on checks' bookkeeping, but the public
*surface* (which ops are checked, which are `unsafe`) is identical regardless of
features. An op is either safe-and-checked or `unsafe`-and-documented; there is
no "checks compiled out" safe op.

### 9.2 `AccessError`

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum AccessError {
    NotAlive,
    MissingComponent,
    Conflict { component: u64, write: bool },
}
impl Display for AccessError { /* ... */ }
impl core::error::Error for AccessError {}
```

Unchanged from `guard.rs` except `#[non_exhaustive]` for forward compatibility.

### 9.3 Naming conventions

| Concept | Name | Notes |
|---|---|---|
| Panicking shared borrow | `get` | `None` on absence, panic on conflict |
| Fallible shared borrow | `try_get` | `Result<_, AccessError>` |
| Owned copy-out | `cloned` | `Option` per element |
| Exclusive borrow | `get` / `get_mut` on `EntityMut`; `World::get_mut` | plain `&`/`&mut` |
| Unchecked twin | `<op>_unchecked` | always `unsafe fn`, always documented contract |
| Construction terminal | `build`, `each`, ... | terminal, by value; **infallible** on the typed path (returns `Query`/`System`), `-> Result` only on the `expr()`/`term_dyn()` fallible typestate (§4.1, §5.1) |
| Shared-register iteration | `each_shared`, `each_entity_shared`, `run_shared` | `&World` |
| Exclusive-register iteration | `each`, `each_entity`, `run`, `chunks` | `&mut World` |

### 9.4 Unsafe-twin catalogue

Every runtime-checked op has an `unsafe *_unchecked` twin with an exact contract.
Safety is never a feature; these are the Tier-2 escape hatches.

| Checked op | Unsafe twin | Safety contract |
|---|---|---|
| `EntityView::get` / `try_get` | `get_unchecked` | Caller guarantees no other live borrow of any requested component's storage on this stage for the returned guards' lifetimes. Skips `read_begin`/`write_begin`; still increments the stage pin (storage-pin is a soundness requirement, not a check). |
| `EntityMut::get_mut` | — | No twin: already lock-free (exclusive register); the borrow checker is the proof. |
| `Query::each` (unproven → Tier 1) | `each_unchecked` | Caller guarantees the batch's term accesses do not alias any live borrow. Skips `acquire_batch_locks`. Mirrors today's `each_unchecked` (`query_api.rs:147`). |
| `Query::each_entity` | `each_entity_unchecked` | As `each_unchecked`, plus the entity view is not used to take a conflicting borrow. |
| `Query::each_shared` | `each_shared_unchecked` | As `each_unchecked` on the shared register. |
| `Query::chunks` | `chunks_unchecked` | Caller guarantees the query is disjoint (skips `is_proven_disjoint`) *and* no outside borrow exists (skips the empty-map debug assert). Hands out `&mut` slices with zero checks. |
| `par_each` | `par_each_unchecked` | Caller guarantees the partition is non-aliasing and `Item: Send` is upheld manually. Mirrors today's `par_each_unchecked` (`system_api.rs:754`). |
| `TableIter::field<T>` | `field_unchecked<T>` | Caller guarantees index in range and `T` matches the field's component type; skips the type check and bounds check. Mirrors today's `field_unchecked` (`table/iter.rs:682`). |
| `TableIter::field_mut<T>` | `field_mut_unchecked<T>` | As `field_unchecked`, mutable. |

The `_unchecked` twins are the *only* way to reach Tier 2. There is no
feature-gated silent removal of a check.

### 9.5 Diagnostics requirements

The surface must be legible when misused:

- **`#[track_caller]` on every panicking public entry point**, with the actual
  panic-raising fn marked `#[cold]`, so a borrow-conflict panic points at the user's
  call site, not into the guard internals.
- **`#[diagnostic::on_unimplemented]` on every public bound-bearing trait** —
  `GuardTuple`, `QueryTuple`, `ChunkColumns`, `Bundle`, `DataFreeTerms`,
  `ReadOnlyTerms` — with an actionable message (e.g. for `DataFreeTerms`: "an `OnAdd`
  observer cannot request component data; use a tag term or `()`").
- **Every `#[doc(hidden)]` kernel trait is sealed** (`GuardElement`, `GuardParts`,
  `ChunkElement`, `RowSlice`, `ChunkColumns`, `Builder`, ...), so downstream crates
  cannot implement them and the plumbing stays an implementation detail.
- **`each!` emits a NAMED compile error** on bound-count vs column-count mismatch
  (§4.5), not raw tuple-destructure "soup".

§4.5 sets the expectation that IDE assistance inside `each!` is limited; the closure
forms (`each` / `each_entity` / `each_iter`) remain first-class for IDE users.

---

## 10. Table / field layer

### 10.1 Bounds-checked row access (locked)

`TableIter` row access is bounds-checked (already fixed): `field_at<T>(index,
row)` validates `row < count` and returns the element, `get_field_at` returns
`Option`. The unchecked path is the explicit `*_unchecked` twin. The spec locks
in "safe row access is always bounds-checked".

### 10.2 Always-on field type checks (locked)

`field<T>` / `field_mut<T>` always verify `T` matches the field's registered
component type (already fixed). A type mismatch is a panic (a program bug: the
query term and the requested type disagree), with `get_field<T>` as the
`Option`-returning fallible form and `field_unchecked<T>` as the `unsafe` twin.
Type checks are **not** feature-gated.

### 10.3 Field accessor consolidation

Today's overlapping families (`table/iter.rs`) are consolidated. Current
surface has, per direction, four spellings each: `field` (panic),
`get_field` (Option), `field_at` (panic, per-row), `get_field_at` (Option,
per-row), plus untyped variants and `_unchecked`. The clean-break surface keeps
one checked accessor per shape with a consistent `get_`/plain split:

```rust
impl<'a, D> TableIter<'a, D> {
    // Whole-column, checked (panic on type/absence bug):
    pub fn field<T>(&self, index: i8) -> Field<'_, T>;
    pub fn field_mut<T>(&self, index: i8) -> FieldMut<'_, T>;
    // Whole-column, fallible:
    pub fn get_field<T>(&self, index: i8) -> Option<Field<'_, T>>;
    pub fn get_field_mut<T>(&self, index: i8) -> Option<FieldMut<'_, T>>;
    // Per-row, checked and fallible (bounds + type):
    pub fn field_at<T>(&self, index: i8, row: usize) -> &T;           // panic OOB/type
    pub fn get_field_at<T>(&self, index: i8, row: usize) -> Option<&T>;
    pub fn field_at_mut<T>(&self, index: i8, row: usize) -> &mut T;
    pub fn get_field_at_mut<T>(&self, index: i8, row: usize) -> Option<&mut T>;
    // Untyped (raw, for reflection/serialization addons only):
    pub fn field_untyped(&self, index: i8) -> FieldUntyped;
    pub fn field_untyped_mut(&self, index: i8) -> FieldUntypedMut;
    // Unsafe twins:
    pub unsafe fn field_unchecked<T>(&self, index: i8) -> FieldMut<'_, T>;
    pub unsafe fn field_mut_unchecked<T>(&self, index: i8) -> FieldMut<'_, T>;
}
```

The redundancy removed: no separate panicking-vs-`get_` *and* `at`-vs-not matrix
that multiplied into 14 methods; the grid is exactly (whole-column | per-row) ×
(checked | fallible) × (shared | mut), plus untyped and unchecked. Naming is
uniform: `get_` prefix ⇒ `Option`, no prefix ⇒ panic-on-bug, `_unchecked` ⇒
`unsafe`.

### 10.4 What stays `pub`

`TableIter`, `Field`/`FieldMut`/`FieldUntyped`, `TableRange`, and the accessors
above stay `pub` (the manual-iteration and reflection escape hatch). `IterGuard`,
`StageLocks`, `StageLocksScope`, `SafetyLocks`, `LockKey`, and the batch-lock
functions stay **crate-private** (`pub(crate)`), as today. `is_proven_disjoint`
becomes a safe method on `Query` (§4.7) rather than a free function over a raw
pointer.

---

## 11. Addons

One policy statement per addon (which get the new access model vs which stay
thin FFI mirrors):

- **system** — full new access model. Terminal `Result`-build, typed `ctx`,
  `each` / `each_with` / `par_each` / `par_each_with`, no `*mut c_void`.
- **observer** — full new access model. `OnAdd` data-free by type (§5.4), typed
  `ctx`, `Result`-build.
- **pipeline** — new model at the edges (phase entities are ordinary entities,
  accessed through the register split); the schedule itself is a thin mirror of
  `ecs_pipeline_*`.
- **timer** — thin FFI mirror; timers are configuration, not component access.
  Setters take values, no raw pointers.
- **app** — thin builder over `ecs_app_run`; `enable_rest` etc. Owns the
  run-loop, so it is one of the few places a terminal `run()` consumes the app.
- **meta** — new model for reads (reflection returns typed cursors / `Ref`), thin
  mirror for the raw `ecs_meta_*` cursor. The ZST-closure serializer transmutes
  (§14) are removed here.
- **json / doc / stats / metrics / alerts / units** — thin FFI mirrors; they
  serialize or annotate, they do not hand out component references. Stay close to
  the C API with safe wrappers.
- **script** — thin mirror of `ecs_script_*`; string in, entities out.
- **rest** — thin mirror, **but flag the raw-pointer hazard.** Today
  `flecs::rest::Rest` is a `#[repr(C)] Copy` component with
  `pub ipaddr: *mut c_char` and `pub impl_: *mut c_void`
  (`core/flecs/addons/rest.rs:58-59`) that users construct and `world.set(...)`.
  Exposing raw pointers on a safe, user-constructed, `Copy` component is unsound
  (a user can set a bogus `impl_` that C dereferences). **Spec requirement:**
  wrap `Rest` so the public constructor takes only `port` / `ipaddr: &str` (or
  `Option<IpAddr>`) and keeps `impl_` private and zero-initialised; the raw
  layout stays crate-private for the FFI marshalling. See Open Questions for the
  exact public shape.

---

## 12. Migration guide outline

Table of old → new for the most common operations. (Full guide is a separate
document; this is its skeleton.)

| # | Old pattern | New pattern |
|---|---|---|
| 1 | `let w2 = world.clone();` | Removed. Pass `&World` / `&mut World`; store an `EntityView`/`QueryHandle` if you need a `Copy` handle. |
| 2 | `e.get::<&Position>(\|p\| { ... use p ... })` (CPS) | `let p = e.get::<&Position>().unwrap(); /* use p */` (guard, NLL scope) |
| 3 | `e.get::<(&A, &mut B)>(\|(a, b)\| ...)` | `let (a, mut b) = e.get::<(&A, &mut B)>().unwrap();` |
| 4 | `e.try_get::<&A>(\|a\| ...)` | `let a = e.try_get::<&A>()?;` |
| 5 | `let p = e.cloned::<&Position>();` (panics if absent) | `let p = e.cloned::<&Position>();` now returns `Option` (`None` on absence); today's `try_cloned` is folded in. **Behavior change:** absence stops panicking. |
| 6 | `world.get::<&mut Position>(e, \|p\| ...)` | `let p = world.get_mut::<Position>(e).unwrap();` (exclusive, `&mut World`) |
| 7 | `let q = world.query::<&Position>().build();` (panics on error) | `let q = world.query::<&Position>().build();` typed build is now **infallible** (returns `Query<D>`); only a builder that took `expr()`/`term_dyn()` returns `Result` and needs `?`. |
| 8 | `q.each(\|p\| ...)` (no world arg) | `q.each(&mut world, \|p\| ...)` (exclusive) or `q.each_shared(&world, \|p\| ...)`. Requires `let mut world`; the closure cannot hold another borrow of `world` across the call. |
| 9 | `q.each_entity(\|e, p\| ...)` | `q.each_entity(&mut world, \|e, p\| ...)` |
| 10 | `q.run(\|mut it\| ...)` | `q.run(&mut world, \|it\| ...)`. `f` is called **once** with a `TableIter` the body advances (`while it.next()`), matching today. |
| 11 | manual `while it.next()` chunk loop | `each!((a, b) in q.chunks(&mut world) { ... })` (proven-disjoint) or `for chunk in q.chunks(&mut world)` (true `Iterator`, §4.6). Unproven-but-dense: `q.batches(&world)`; non-dense: `each_shared` / `run` (routing §4.5). |
| 12 | `world.system::<...>().set_context(ptr).each(...)` | capture in the closure, or `.ctx(value)` then `System::ctx::<T>()` |
| 13 | `world.observer::<OnAdd, &Position>()` | Compile error now; use `world.observer::<OnAdd, ()>()` (data-free) or a different event |
| 14 | `it.field::<Position>(0)` (feature-gated check) | `it.field::<Position>(0)` (always type-checked) / `it.get_field::<Position>(0)` for `Option` |
| 15 | `*entity_view = Entity::new(x)` (via `DerefMut`) | Removed; obtain a fresh `world.entity_view(x)` |
| 16 | `let s = world.system::<...>().each(...);` (infallible) | `let s = world.system::<...>().each(...);` still infallible on the typed path (returns `System`); `Result` only if the builder took `expr()`/`term_dyn()`. `s.run(&mut world)` (the `SystemRunnerFluent` is deleted). |
| 17 | `let q = world.new_query::<&Position>();` | `let q = world.new_query::<&Position>();` retained and **infallible** (§4.1); it does not panic on a typed descriptor. |
| 18 | `e.set(A).set(B).set(C)` (N table moves) | `world.spawn((A, B, C))` / `em.insert((A, B, C))` — one table move (§4.11); the fluent form still works. |
| 19 | read-back of own `set` inside a `get`/`try_get` scope | **Behavior change (GAP-8):** the `set` is deferred while the guard is live, so the in-scope re-read sees the pre-write value. Drop the guard (reach a sync point) or use `&mut World` / `EntityMut` for immediate read-back (§3.6). |
| 20 | singleton access / `Singleton<T>` term | `world.singleton::<T>()` (`Ref`) / `world.singleton_mut::<T>()` (`&mut`) for direct access; the trait-based `&mut Gravity` term stays legal and Tier-1 locked, while the `Singleton<T>` query wrapper is read-only (`&mut` does not compile). Singleton queries run Tier 1 and panic under `chunks` (§4.9). |

---

## 13. Testing and benchmark strategy

### 13.1 Existing gates (kept)

- **Full test suite** green (`cargo +1.97.0 test`, all features).
- **Clippy** clean with `-D warnings`.
- **Criterion vs the `pre` baseline** with raw-C controls: medians must not
  regress beyond noise, and the raw-C control benches must stay flat (the design
  record's harness holds controls within 0.4%). New surface benches
  (`query_each_*`, guard `get`, `each_exclusive`, `each!` write, chunk read) land
  in the same harness and must stay within **3%** of the raw-C control for the
  equivalent access pattern.

### 13.2 Safety-test taxonomy

Four required categories (extends today's `tests/flecs/safety/`):

- **Guard conflicts** — every panicking borrow has a test asserting the panic and
  a `try_*` test asserting the `AccessError`: read+write, write+write,
  guard-vs-query, disjoint-table read+write stays legal, sparse per-stage
  disjointness.
- **Panic cleanup** — a panicking callback in each trampoline
  (each / each_iter / each_entity / each_with / each_entity_with / run / par_each /
  observer) leaves the stage lock map balanced (the
  `StageLocksScope` restore) and no dangling `ecs_table_lock`; the world is
  usable afterward and the panic is rethrown from the safe entry point.
- **MT stage isolation** — two workers accessing the same component on disjoint
  tables/entities do not false-conflict; a real cross-worker conflict errors
  rather than aliases; `Stage` cannot be constructed outside a callback (compile
  test).
- **Compile-fail** (`trybuild`) — the register violations that must not compile:
  holding a `&mut T` from `World::get_mut` across another world access;
  `Up<&mut T>` / `Singleton<&mut T>`; `observer::<OnAdd, &Position>()`;
  double-`build`; iterating a consumed `Chunks` iterator; `get_many::<(&mut A, &mut A)>()`
  (duplicate mutable); sending a non-`Send` `QueryHandle`; `*entity_view = ...`.

### 13.3 New CI contracts introduced by this spec

- **Scheduler-invariant contract** (§6.3) — the `ThreadId` test that pins
  "non-multithreaded systems run only on the `progress()` thread"; gates the
  vendored-flecs-C bump.
- **`get_ptr` width guard** — a `const` assertion / test that
  `ecs_rust_get_ptr_t` stays 16 bytes (design record invariant; widening it
  loses the 19-30% `get` win).
- **No-`*mut c_void`-in-surface lint** — a test/grep gate asserting the public
  API exposes no `*mut c_void` context parameter and no `pub` raw pointer on a
  safe component (catches a `Rest`-style regression).
- **World-identity assertions** — the `each`/`chunks` cross-world guards
  (`4dc66a81`) get regression tests in the required suite.
- **Chunk single-visit contract** (§4.6) — the "one query iteration visits each
  `(table, row-range)` at most once" invariant that makes the true-`Iterator` chunk
  slices non-aliasing; re-verified on the vendored-C bump; must cover sorted,
  grouped, and change-skipped iteration.

### 13.4 New examples required by this spec

- **A `par_each` / `multi_threaded` example** must be added to the examples suite
  (GAP-12): none exists today under `examples/flecs/`, so the `Stage<'s>`-in-worker /
  `par_each_with` ergonomics (§6) — the redesign's highest-stakes surface — are
  currently unexercised by real usage. The example exercises a partitioned parallel
  write and a stage-issued deferred command.

---

## 14. C++-isms designed away

Each verified against the tree; replacement stated.

1. **Closure-CPS `get`** (`entity_view_const.rs:1452`, `fn get(self, callback:
   impl FnOnce(...) -> R) -> R`) → guard-returning `get`/`try_get` (§3.1); NLL
   scoping, composes with `?`.
2. **`Copy`-based fluent chaining on views** — `EntityView: Copy` returning
   `Self` from setters let a stale copy be reused after a structural change. New:
   `EntityView` setters return `Self` for chaining but their write is deferred
   while a guard-pin episode / defer scope is live (immediate at top level, §3.5),
   so a chained result observes the pin/defer discipline; `EntityMut` (`!Copy`) is
   the mutable-fluent path, so a moved-out mutable view cannot be reused.
3. **`&mut self`-returning builders that build twice or never**
   (`builder.rs:6` `fn build(&mut self)`) → terminal `build(self)` (§4.1):
   consuming and single, infallible on the typed path and `Result`-returning only
   on the `expr()`/`term_dyn()` fallible typestate.
4. **Panic-first error handling in core/** — `build()` panicking on a bad
   descriptor with `try_build` bolted on → typed `build()` is infallible (a typed
   descriptor cannot be malformed; the exceptional `ecs_query_init` failure
   panics), and a malformed runtime descriptor surfaces as `Result<_,
   QueryBuildError>` on the fallible typestate (§4.1). Panic is otherwise reserved
   for aliasing bugs (§9.1).
5. **Overlapping field accessor families** (`table/iter.rs`: `field` /
   `get_field` / `field_at` / `get_field_at` / untyped / `_unchecked`, 14
   spellings) → the consolidated grid in §10.3.
6. **ZST-closure-to-fn-pointer transmutes** (`query_builder.rs:1259`,`:1285`
   `transmute_copy::<_, F>(&())` for `order_by`; the meta/opaque serializer
   equivalents) → generic monomorphized trampolines that store the closure in the
   typed context (§5.3) instead of transmuting a ZST closure to a bare `fn`. No
   `transmute_copy` of a closure in the surface.
7. **Run-as-side-effect-of-`Drop`** (`SystemRunnerFluent`,
   `addons/system/system_runner_fluent.rs`) → an explicit terminal `run()` /
   `run_with(...)`. The fluent runner's `set_offset`/`set_limit` write fields that
   `Drop` never reads (dead API) — the whole `SystemRunnerFluent` type is
   **deleted**; running a system is `system.run(&mut world)` with offset/limit as
   explicit args or via a `page`/`worker` chained iter.
8. **`*mut c_void` contexts** (`system_api.rs:42`, `observer.rs:133`,
   `world/operations.rs:1031`) → capture + typed `ctx<C>` (§5.3). No `*mut
   c_void` in the surface.
9. **Associated-const-bool SFINAE emulation in `IntoId`/`IntoEntity`**
   (`into_id.rs:14-18`, `const IS_PAIR`/`IS_ENUM`/`IS_TYPE_TAG`/... driving
   branch selection) → real trait dispatch: distinct traits/impls
   (`IntoComponentId`, `IntoTag`, `IntoPair`) selected by the type, not a runtime
   `if T::IS_PAIR` on an associated const. The const-bool matrix that emulated
   C++ `if constexpr` is replaced by monomorphized impls, so the compiler
   dead-strips the wrong branch structurally.
10. **`type_name` as identity** — no identity keying on
    `core::any::type_name::<T>()` (§8.3); `TypeId`-keyed registries for component
    and module identity. Remaining `type_name` calls
    (`get_tuple.rs`/`query_tuple.rs`/`cloned_tuple.rs`) are panic-message
    diagnostics only, which is allowed.
11. **Prelude globs exporting `#[doc(hidden)]` internals** → the prelude
    re-exports only the intended public surface; `#[doc(hidden)]` trait plumbing
    (`GuardElement`, `GuardParts`, `ChunkElement`, `RowSlice`, `ChunkColumns`,
    `Builder`) is reachable by path for macro expansion but **not** glob-exported
    from `prelude`. A test asserts the prelude's exported set matches an allowlist.
12. **`WorldRef: Deref<World>` via transmute** (`world_provider.rs:120-136`,
    `transmute::<&WorldRef, &World>`) → removed from the public surface; `WorldRef`
    is crate-private (§2.4). The transmute (guarded today by a layout
    `const_assert`) is an implementation detail, not an API.
13. **`DerefMut<Target = Entity>` on views** (`entity_view_const.rs:110`) →
    removed (§2.2). Read-only `Deref<Target = Entity>` kept; `*view = Entity::new`
    no longer compiles.
14. **The `query!` / `system!` / `observer!` / `id!` DSL macros** — **retained**
    (decided). They are proc macros (declared for the crate, expanded via
    `flecs_ecs_derive`), heavily used: **265 `query!`, 46 `observer!`, 29
    `system!`, 72 `id!`** call sites across `tests/` and `src/`, including the new
    safety/experimental tests (`tests/flecs/safety/*`,
    `tests/flecs/experimental/*`). They are orthogonal to the access model: the
    DSL only builds *query descriptors*, and expands to the new builder surface
    (`world.query::<...>()...build()`), so it inherits the `Result`-build and the
    world-threaded iteration automatically. **Rationale:** deleting them would
    break 400+ call sites for no soundness gain; they never touch the register
    split. They stay, retargeted to the new builders. (The one required change:
    the macros' generated `build()` must thread through the `Result`-returning
    terminal; a macro that discards the `Result` is a bug caught by the build.)

---

## 15. Resolved questions (user rulings, 2026-07-26)

All four open questions were ruled on by the user; none remain open.

1. **`Rest` public constructor shape → typed `RestConfig`.** A
   `RestConfig { port, ip: Option<IpAddr> }` struct, converted internally to the
   `#[repr(C)]` FFI `Rest` with `impl_` kept private and zero-initialised. A
   `Rest` with a dangling `impl_` is unconstructible from safe code.

2. **Manual `run` / `TableIter` escape hatch → kept, world-threaded** (§4.4).
   Reflection/serialization addons and advanced users keep a safe path to
   non-dense columns (ref/sparse/inherited) that `chunks` deliberately refuses.

3. **Borrowed optional guard element → ships in the initial release.** §3.4 now
   specifies `Option<Ref<T>>` / `Option<Mut<T>>` tuple elements with
   per-element presence branching in the fused acquire, rollback, and release.

4. **`.each_with` second parameter → distinct `Stage<'s>` newtype** (§6.1), not
   a reused `&World`: the deferred-command surface stays explicit and a callback
   cannot mistake its stage for the owning world.
