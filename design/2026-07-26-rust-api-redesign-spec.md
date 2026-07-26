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
point (Decision 2); the guard holds a defer level open for its lifetime so
storage cannot move under a live borrow (storage-pin). `World` stops being
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
/// Shared immutable access to one component; releases its read borrow and one
/// defer level on drop. `Deref<Target = T>`.
pub struct Ref<'w, T> { /* ptr, world, lock key */ }

/// Shared mutable access to one component; releases its write borrow and one
/// defer level on drop. `Deref` + `DerefMut<Target = T>`.
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

### 3.4 Guard tuples do **not** model `Option<&T>` (decided)

**Decision:** the guard forms (`get` / `try_get`) require every requested
component to be present; a null pointer for any element makes the whole acquire
`None` / `AccessError::MissingComponent`. Optional / maybe-absent access is
served by:

- `cloned` for an owned copy-out (`Option` per element), and
- the chunk cursor's `Option<&[T]>` / `Option<&mut [T]>` columns on the
  exclusive register (§4.6), which already model absence.

Rationale: an `Option<Ref<T>>` element would complicate the fused-acquire
rollback (a `None` element holds no borrow, so the release loop must branch per
element) and the row-at-a-time "maybe present" case is rare and fully covered by
`cloned`. No optional guard element ships. This is final; revisit only if a
concrete need for a *borrowed* optional row read appears (tracked in Open
Questions as a non-blocker).

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

### 3.6 Deferred-write semantics under `&World` (shared register)

Under `&World` / `EntityView`, `set` and structural ops are **deferred** to the
next sync point (Decision 2). Each live guard holds one defer level open
(`ecs_rust_get_scope_begin` / `ecs_defer_begin`), so the flush is unreachable
while any guard lives — storage cannot move and the borrowed pointer cannot
dangle. **Observer timing:** deferred ops issued under `&World` run, and their
`OnSet` / `OnAdd` observers fire, when the **last guard drops** (the outermost
defer level closes). This generalises the old CPS-`get` "observers run after the
callback" semantics to arbitrary lexical scopes. A `set` issued while two guards
are live does not fire its observer until both have dropped.

```rust
impl<'w> EntityView<'w> {
    /// Deferred under &World: queued now, applied (and observers fired) when the
    /// last live guard on this stage drops.
    pub fn set<T>(self, value: T) -> Self;
    pub fn add(self, id: impl IntoId) -> Self;
    pub fn remove(self, id: impl IntoId) -> Self;
}
```

Immediate read-back of a shared-register write requires `&mut World` /
`EntityMut` (Decision 2): there is no way to observe a still-deferred write
through `&World` without first dropping to a sync point.

---

## 4. Queries

### 4.1 Building

The builder's terminal is the build, taken **by value**, returning a `Result`:

```rust
impl<'w> QueryBuilder<'w, D> {
    /// Terminal. Consumes the builder (so double-build and build-never are
    /// compile errors) and returns the query or the construction error.
    pub fn build(self) -> Result<Query<D>, QueryBuildError>;
}

#[derive(Debug)]
#[non_exhaustive]
pub enum QueryBuildError {
    /// `expr()` string failed to parse.
    InvalidExpr { expr: String },
    /// A term was malformed (bad id, conflicting modifiers).
    InvalidTerm { index: usize },
    /// The C `ecs_query_init` rejected the descriptor for another reason.
    Init,
}
impl core::error::Error for QueryBuildError {}
```

This removes today's two problems at once: `build(&mut self) -> Self::BuiltType`
(`builder.rs:6`) returns `&mut self`-tied and can be called twice or never, and
it *panics* on a bad descriptor while `try_build` returns `Option` and loses the
reason. The clean-break surface has exactly one terminal, consuming, and
`Result`-returning. There is no `try_build`; `build` is already fallible.

The intermediate configuration methods (`with`, `without`, `term`, `expr`,
`set_cached`, ...) keep `&mut self` for chaining but are *not* terminal, so the
"build twice / never" hazard exists only at the single by-value `build`.

### 4.2 The query handle

```rust
pub struct Query<D: QueryTuple> { /* owns ecs_query_t refcount */ }
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

### 4.4 Iteration surface

```rust
impl<D: QueryTuple> Query<D> {
    /// Exclusive register. Tier 0 when proven-disjoint, else Tier 1 over &mut
    /// World. The common fast path.
    pub fn each(&self, world: &mut World, f: impl FnMut(D::Item<'_>));

    /// Shared register. Tier 1 (always registers batch locks). Legal while other
    /// shared borrows exist; conflicts panic.
    pub fn each_shared(&self, world: &World, f: impl FnMut(D::Item<'_>));

    /// Exclusive register, lending chunk cursor (§4.6). Panics if not
    /// proven-disjoint.
    pub fn chunks<'w>(&self, world: &'w mut World) -> ChunkCursor<'w, D>;

    /// Row form carrying the entity.
    pub fn each_entity(&self, world: &mut World, f: impl FnMut(EntityView, D::Item<'_>));
    pub fn each_entity_shared(&self, world: &World, f: impl FnMut(EntityView, D::Item<'_>));

    /// Escape hatch: manual table-batch iteration (replaces today's `run`).
    /// Still world-threaded; yields a `TableIter` bound to the borrow.
    pub fn run(&self, world: &mut World, f: impl FnMut(TableIter<D>));
    pub fn run_shared(&self, world: &World, f: impl FnMut(TableIter<D>));
}
```

`each` taking `&mut World` is the fast, common case; `each_shared` is the
opt-in for iterating while other shared borrows are alive. Rust cannot overload
one name on argument mutability, hence the two names (this is the naming the
design task fixed).

### 4.5 The `each!` macro

Adopts the prototype macro (`experimental/mod.rs`), driven by the chunk cursor,
with native `break` / `continue` / `?`:

```rust
each!((pos, vel) in query.chunks(&mut world) {
    pos.x += vel.x;      // pos: &mut Position, vel: &Velocity, per row
});
```

The macro expands to a `while let Some(chunk) = cursor.next()` over batches and
an inner `for row in 0..len` that binds each column's row (`RowSlice::row`).
Because the body is inlined textually, `break`/`continue`/`?` refer to the
caller's control flow. Bound-name count must equal column count (else a
compile error from the tuple destructure).

### 4.6 Chunk cursor: pre-checked slice iterators (friction fix)

```rust
pub struct ChunkCursor<'w, D> { /* lending: next() borrows &mut self */ }

impl<'w, D: QueryTuple> ChunkCursor<'w, D> {
    /// Advance to the next table batch, yielding whole-column slices
    /// (`&mut [A]`, `&[B]`, `Option<&[C]>`...). Lending: the chunk borrows
    /// `&mut self`, so it is dropped before the next pull (hence `while let`,
    /// not `Iterator`). Decision 5: chunk is the primitive, row is sugar.
    pub fn next(&mut self) -> Option<D::Chunk<'_>>;

    /// Terminal, by value: cannot be reused (double-consume is a compile error).
    pub fn for_each(self, f: impl FnMut(D::Chunk<'_>));
}
```

**Friction fix (prototype):** the read path must vectorise. The cursor yields
real `&[T]` / `&mut [T]` slices (not an index-and-bounds-check accessor), so
user code that does `chunk.0.iter_mut().zip(chunk.1)` gets pointer-add codegen
with the bounds check hoisted out of the row loop, matching `each`'s pointer
arithmetic. The `each!` macro's inner loop indexes `0..len` where `len` is read
once per chunk, so LLVM elides the per-row bounds check. This closes the "chunk
read path pays per-row bounds checks" friction. The cursor rejects ref /
inherited / sparse columns (`ref_fields | up_fields | row_fields != 0` → panic):
those are not plain dense self columns and cannot be handed out as slices.

### 4.7 Disjointness proof behind a safe handle (friction fix)

```rust
impl<D: QueryTuple> Query<D> {
    /// True when the query's data terms provably address pairwise-distinct,
    /// dense, self-sourced storage (so a lock-free exclusive run cannot alias).
    /// Conservative: `false` means "not proven", never "known aliasing".
    pub fn is_proven_disjoint(&self, world: &World) -> bool;
}
```

Takes a safe `&Query` + `&World`, not a raw `*const ecs_query_t` (prototype
friction). The analysis is `experimental/disjoint.rs` unchanged: rejects
wildcards, non-`$this`/`Self`-traversal sources, non-`And` operators, sparse /
`DontFragment` storage, and any two data terms sharing a concrete id; ignores
tag terms. `each`/`chunks` call it internally; it is also public so callers can
branch before choosing `chunks` vs `each_shared`.

### 4.8 Tuple arities via `tuples!` (friction fix)

The prototype hand-capped guard, chunk, and disjoint tuple impls at arity 5. The
final surface generates them with the crate's existing `tuples!` macro (same
macro that drives `QueryTuple` / `GetTuple`), so `GuardTuple`, `ChunkColumns`,
and `GuardElement` cover the full supported arity uniformly. No behavioural
change, only coverage.

### 4.9 Multi-source terms are read-only by construction

Traversal / singleton terms name storage the current entity does not own, so
they are **immutable-only in the type system**:

```rust
Up<T>        // -> &T only; `Up<&mut T>` does not implement the query-term trait
Cascade<T>   // -> &T only
Singleton<T> // -> &T only
```

Requesting `&mut` through `Up` / `Cascade` / `Singleton` is a **compile error**
(the mutable term impl is simply not provided for these wrappers), not a runtime
guard. This removes an entire class of aliasing (two entities sharing an
inherited component and both writing it).

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

---

## 5. Systems and observers

### 5.1 Terminal build is two-phase (kept by construction)

The system/observer builder's terminal is the closure-taking build, by value,
returning `Result`:

```rust
impl<'w> SystemBuilder<'w, D> {
    pub fn each(self, f: impl FnMut(D::Item<'_>) + 'static)
        -> Result<System, SystemBuildError>;
    pub fn each_entity(self, f: impl FnMut(EntityView, D::Item<'_>) + 'static)
        -> Result<System, SystemBuildError>;
    pub fn run(self, f: impl FnMut(TableIter<D>) + 'static)
        -> Result<System, SystemBuildError>;
}
```

**Two-phase init (kept):** the closure is boxed and its context installed
*only after* `ecs_system_init` returns a valid system entity. A failed init must
not leak the closure. This is already correct on the current surface
(`system_builder.rs` builds the entity first, checks `id() != 0`); the spec
locks it in as "closure ownership transfers on success, is dropped on failure",
enforced by construction (the boxed closure lives in a local that is only handed
to the C context ptr after the id check).

### 5.2 Closures are `'static`; world access is opt-in

System/observer closures are `'static` (they outlive the build call and run
later). They get world access **only** through:

- the iteration **item** (the `D::Item` handed per row/batch), or
- an explicit `.each_with(|item, stage|)` opt-in:

```rust
impl<'w> SystemBuilder<'w, D> {
    /// Opt-in world access. `stage` is the callback's stage handle (§6), through
    /// which deferred commands are issued. Enabling this makes the system
    /// register its term locks (the item alone can be served lock-free on the
    /// tiered path when disjoint; adding arbitrary stage access cannot).
    pub fn each_with(self, f: impl FnMut(D::Item<'_>, Stage<'_>) + 'static)
        -> Result<System, SystemBuildError>;
}
```

`.each` (no stage) can qualify for the lock-free tiered path; `.each_with`
always registers this system's term locks, since the stage handle can reach
arbitrary storage the disjointness proof does not cover.

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
trampoline"); the spec requires every new trampoline (each/each_with/run/observer)
to route through the same catch-stash-rethrow path, tested by the panic-cleanup
safety suite (§13).

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
    pub fn par_each(self, f: impl Fn(D::Item<'_>) + Send + Sync + 'static)
        -> Result<System, SystemBuildError>
        where for<'x> D::Item<'x>: Send;
    pub fn par_each_with(self, f: impl Fn(D::Item<'_>, Stage<'_>) + Send + Sync + 'static)
        -> Result<System, SystemBuildError>
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

### 7.1 Guard-held defer levels (prototype-validated)

Each shared-register guard holds exactly one flecs defer level for its lifetime
(§3.6). Nesting is by count: *k* live guards → *k* open levels; the queue flushes
when the count returns to zero (last guard drops). This is `guard.rs` +
`entity_access.rs`, validated. Structural ops through `&World` while a guard is
live are queued; the borrowed pointer stays valid because the flush is
unreachable until every guard drops (storage-pin).

### 7.2 `mem::forget` on a guard must not wedge the world (specified)

If a caller `mem::forget`s a guard, its `Drop` never runs, so its read/write
borrow is **never released** and its defer level is **never closed**. The
specified, sound behaviour:

- **The borrow leaks, not corrupts.** The forgotten borrow stays registered in
  the stage lock map. Subsequent conflicting access to that same storage will
  *panic* (or `try_*`-error) forever — a leak that fails safe, never a
  use-after-free. The stage map is a plain counter; a stuck counter denies access,
  it does not alias.
- **The defer level leaks, not corrupts.** The forgotten level stays open, so
  writes issued after it remain queued past the point they would otherwise flush.
  A leaked level can only *delay* the flush and the observers behind it; it can
  never alias or grant access, so no UB is reachable. Whether `progress()`'s
  pipeline sync points drain the world queue despite an unbalanced user-held
  level is deliberately not asserted here: the implementation must pin the
  actual behaviour with a test (leak a guard, run `progress()`, assert either
  drain-at-sync or delay-until-close, and assert no abort at world destruction),
  and the guard-type docs must state whichever behaviour is pinned.

This is sound because both leaked resources are *monotonic denials* (a stuck
lock denies; a stuck defer delays), never grants. The prototype's `PendingDefer`
(`entity_access.rs:30`) guarantees the *acquire* path is balanced on panic; the
`mem::forget` case is the caller's explicit leak and the spec commits to
"fails-safe, does not wedge" as the contract. No `mem::forget`-detection code is
added (it would cost the hot path); the guarantee is structural.

### 7.3 Commands: per-call token, not persistent buffer (decided)

**Decision:** systems get a **per-call stage token** (`Stage<'s>`, §6.1),
obtained through `.each_with` / `.par_each_with`, not a persistent `Commands`
buffer stored on the system.

Rationale:

- A persistent `Commands` buffer would have to outlive sync points and reconcile
  with the world's own defer stack and with the guard-held defer levels (§7.1),
  duplicating bookkeeping the flecs defer queue already owns.
- The per-call token scopes command issuance to the callback invocation, so it
  composes cleanly with guard defer levels and with the panic-cleanup path (the
  token cannot outlive the `catch_unwind`).
- Deferred ops through `Stage` are just entries on the world's existing defer
  queue; there is no separate replay step to own.

`Stage<'s>` therefore *is* the "commands" surface: `stage.entity()`,
`stage.entity(e).set(..)`, `stage.defer(|| ..)` all enqueue onto the stage's
defer queue and flush at the next sync point.

### 7.4 Sync points

A sync point is where the deferred queue drains and observers fire:

- end of a `progress()` pipeline phase (between systems, per flecs merge points);
- last-guard-drop on a stage (§3.6), which closes the outermost Rust-held level;
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
- `Result<T, E>` — malformed construction: `build() -> Result<_, QueryBuildError>`,
  `SystemBuilder::each() -> Result<_, SystemBuildError>`, and
  `try_get() -> Result<_, AccessError>` (the fallible borrow).
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
| Fallible construction | `build`, `each`, ... `-> Result` | terminal, by value |
| Shared-register iteration | `each_shared`, `each_entity_shared`, `run_shared` | `&World` |
| Exclusive-register iteration | `each`, `each_entity`, `run`, `chunks` | `&mut World` |

### 9.4 Unsafe-twin catalogue

Every runtime-checked op has an `unsafe *_unchecked` twin with an exact contract.
Safety is never a feature; these are the Tier-2 escape hatches.

| Checked op | Unsafe twin | Safety contract |
|---|---|---|
| `EntityView::get` / `try_get` | `get_unchecked` | Caller guarantees no other live borrow of any requested component's storage on this stage for the returned guards' lifetimes. Skips `read_begin`/`write_begin`; still holds the defer level (storage-pin is a soundness requirement, not a check). |
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

Table of old → new for the 15 most common operations. (Full guide is a separate
document; this is its skeleton.)

| # | Old pattern | New pattern |
|---|---|---|
| 1 | `let w2 = world.clone();` | Removed. Pass `&World` / `&mut World`; store an `EntityView`/`QueryHandle` if you need a `Copy` handle. |
| 2 | `e.get::<&Position>(\|p\| { ... use p ... })` (CPS) | `let p = e.get::<&Position>().unwrap(); /* use p */` (guard, NLL scope) |
| 3 | `e.get::<(&A, &mut B)>(\|(a, b)\| ...)` | `let (a, mut b) = e.get::<(&A, &mut B)>().unwrap();` |
| 4 | `e.try_get::<&A>(\|a\| ...)` | `let a = e.try_get::<&A>()?;` |
| 5 | `let p = e.cloned::<&Position>();` | `let p = e.cloned::<&Position>();` (now `Option`) |
| 6 | `world.get::<&mut Position>(e, \|p\| ...)` | `let p = world.get_mut::<Position>(e).unwrap();` (exclusive, `&mut World`) |
| 7 | `let q = world.query::<&Position>().build();` (panics on error) | `let q = world.query::<&Position>().build()?;` (`Result`) |
| 8 | `q.each(\|p\| ...)` (no world arg) | `q.each(&mut world, \|p\| ...)` (exclusive) or `q.each_shared(&world, \|p\| ...)` |
| 9 | `q.each_entity(\|e, p\| ...)` | `q.each_entity(&mut world, \|e, p\| ...)` |
| 10 | `q.run(\|mut it\| ...)` | `q.run(&mut world, \|it\| ...)` |
| 11 | manual `while it.next()` chunk loop | `each!((a, b) in q.chunks(&mut world) { ... })` or `q.chunks(&mut world)` `while let` |
| 12 | `world.system::<...>().set_context(ptr).each(...)` | capture in the closure, or `.ctx(value)` then `System::ctx::<T>()` |
| 13 | `world.observer::<OnAdd, &Position>()` | Compile error now; use `world.observer::<OnAdd, ()>()` (data-free) or a different event |
| 14 | `it.field::<Position>(0)` (feature-gated check) | `it.field::<Position>(0)` (always type-checked) / `it.get_field::<Position>(0)` for `Option` |
| 15 | `*entity_view = Entity::new(x)` (via `DerefMut`) | Removed; obtain a fresh `world.entity_view(x)` |

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
  (each/each_with/run/par_each/observer) leaves the stage lock map balanced (the
  `StageLocksScope` restore) and no dangling `ecs_table_lock`; the world is
  usable afterward and the panic is rethrown from the safe entry point.
- **MT stage isolation** — two workers accessing the same component on disjoint
  tables/entities do not false-conflict; a real cross-worker conflict errors
  rather than aliases; `Stage` cannot be constructed outside a callback (compile
  test).
- **Compile-fail** (`trybuild`) — the register violations that must not compile:
  holding a `&mut T` from `World::get_mut` across another world access;
  `Up<&mut T>` / `Singleton<&mut T>`; `observer::<OnAdd, &Position>()`;
  double-`build`; iterating a consumed `ChunkCursor`; sending a non-`Send`
  `QueryHandle`; `*entity_view = ...`.

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

---

## 14. C++-isms designed away

Each verified against the tree; replacement stated.

1. **Closure-CPS `get`** (`entity_view_const.rs:1452`, `fn get(self, callback:
   impl FnOnce(...) -> R) -> R`) → guard-returning `get`/`try_get` (§3.1); NLL
   scoping, composes with `?`.
2. **`Copy`-based fluent chaining on views** — `EntityView: Copy` returning
   `Self` from setters let a stale copy be reused after a structural change. New:
   `EntityView` setters are deferred and return `Self` for chaining only within
   one deferred scope; `EntityMut` (`!Copy`) is the mutable-fluent path, so a
   moved-out mutable view cannot be reused.
3. **`&mut self`-returning builders that build twice or never**
   (`builder.rs:6` `fn build(&mut self)`) → terminal `build(self) -> Result`
   (§4.1): consuming, single, fallible.
4. **Panic-first error handling in core/** — `build()` panicking on a bad
   descriptor with `try_build` bolted on → single `build() -> Result<_,
   QueryBuildError>` (§4.1); panic reserved for aliasing bugs (§9.1).
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

## 15. Open questions

Only genuinely undecidable-without-the-user items remain; each has a
recommendation.

1. **`Rest` public constructor shape.** The raw-pointer hazard is settled (wrap
   it, §11), but the exact safe surface is a judgment call: (a)
   `Rest::new(port: u16)` only, `ipaddr` defaulted; (b) `Rest::new(port,
   ipaddr: &str)`; (c) a typed `RestConfig { port, ip: Option<IpAddr> }`.
   **Recommendation: (c)** — a typed config struct with `impl_` kept private and
   zero-initialised, converted to the `#[repr(C)]` FFI `Rest` internally. It is
   the only option that never lets a user construct a `Rest` with a dangling
   `impl_`.

2. **Keep the manual `run` / `TableIter` escape hatch, or cut it in the clean
   break?** `chunks` + `each` cover the common cases; `run`/`TableIter` is the
   low-level table-batch surface (`each_iter` today). **Recommendation: keep it**,
   world-threaded (§4.4), because reflection/serialization addons and advanced
   users need raw field access, and `chunks` deliberately refuses non-dense
   columns (ref/sparse/inherited) that only `TableIter` can reach. Cutting it
   would force those users to `unsafe`.

3. **Borrowed optional row read (`Option<Ref<T>>` guard element).** §3.4 defers
   it to `cloned` (owned) and the chunk cursor (exclusive). The only gap is a
   *borrowed, shared-register, maybe-absent* single-row read. **Recommendation:
   do not ship it initially** — no prototype user hit the gap, and adding it
   complicates the fused-acquire rollback. Revisit if a concrete caller appears;
   it is an additive, non-breaking change if added later.

4. **`Stage` vs `&World` as the `.each_with` second parameter.** The spec uses a
   distinct `Stage<'s>` newtype (§6.1) rather than reusing `&World`.
   **Recommendation: keep the distinct `Stage`** — it prevents a callback from
   accidentally treating its stage as the owning world (e.g. calling immediate
   `&mut World` ops), makes the deferred-command surface explicit, and carries the
   `!Send`/`!Sync` and per-callback lifetime cleanly. The cost is one more type in
   the surface, which the migration guide covers.
