# Examples migration evaluation: does the redesign make real code better?

Date: 2026-07-26. Branch: `safety-perf`. Scope: read-only. No code changed, nothing committed.

Method: 11 representative examples from `flecs_ecs/examples/flecs/` (today's API in
real use) rewritten faithfully under `design/2026-07-26-rust-api-redesign-spec.md`,
using the exact spec signatures and the real semantics of the working prototype in
`flecs_ecs/src/experimental/` (`entity_access.rs`, `guard.rs`, `exclusive.rs`,
`chunks.rs`, `disjoint.rs`, `mod.rs`). 11, not 10: prefabs and relationships are both
explicitly in scope and neither folds cleanly into another. Multithreaded systems are
in scope "if present" — **no `par_each` / `multi_threaded` example exists in the tree**
(GAP-12), so that ergonomic surface could not be evaluated against real usage.

Judgements are adversarial toward the redesign. Where the old closure API reads better,
it is marked WORSE with the concrete reason.

---

## Verdict table

| # | Example | Verdict | One-line reason |
|---|---|---|---|
| 1 | `entities/entity_basics` | MIXED | Chained setup unchanged; guard read is cleaner, but single-`Option` get needs a 1-tuple workaround and `world.each_entity::<T>` one-liner has no spec equivalent. |
| 2 | `entities/entity_hierarchy` | BETTER | The get-returns-a-value CPS pattern collapses to a plain guard + expression; composes and reads like ordinary Rust. |
| 3 | `entities/entity_get_multiple` | BETTER | Tuple get goes from nested closure to destructured guards with direct mutation; `Option`-in-tuple maps 1:1 to `Option<Ref>`. |
| 4 | `queries/query_basics` | MIXED | `run`'s manual `while it.next()` loop gets the `each!` / chunks win with native control flow, but every `each`/`each_entity` now carries a `&mut world` arg and `new_query` gains a `?`. |
| 5 | `queries/query_wildcard` | WORSE | Relies on `each_iter(\|it, index, item\|)` to inspect the matched pair; the spec's iteration surface (§4.4) has no `each_iter`, forcing a verbose manual `run`. |
| 6 | `queries/query_hierarchy` | MIXED | `cascade` parent term becomes `&mut`-impossible at the type level (real safety win), paid for with `?` on build and `&mut world` on both iterations. |
| 7 | `systems/system_basics` | MIXED | Hello-world-sized: the `Result`-returning `.each_entity(...)?` and `s.run(&mut world)` add pure ceremony with no expressiveness gain at this size. |
| 8 | `systems/system_sync_point` | WORSE | Deferred `e.set(...)` from inside a system has no clean home: there is no `each_entity_with` (entity + `Stage`) terminal, so command-issuing systems fall between `each_entity` and `each_with`. |
| 9 | `observers/observer_basics` | MIXED | `OnAdd` data-free becomes a compile error (footgun removed, real win); but event metadata (`event()`, `event_id()`, `pair()`) lives only on `TableIter`, so both observers must drop to `run` since `each_iter` is gone. |
| 10 | `prefabs/prefab_basics` | WORSE | Read-after-write inside a live guard scope silently changes behavior: the prefab `set` is now deferred until the guard drops, so the in-scope re-print observes the old value, not the new. |
| 11 | `relationships/relationships_basics` | MIXED | Essentially untouched (structural adds are top-level-immediate, read helpers unchanged); neither improved nor regressed. |

---

## Side-by-side comparisons

### 1. entity_basics — MIXED

**Old (as-is):**
```rust
let bob = world
    .entity_named("Bob")
    .set(Position { x: 10.0, y: 20.0 })
    .add(Walking);

bob.get::<Option<&Position>>(|pos| {
    if let Some(pos) = pos {
        println!("Bob's position: {pos:?}");
    }
});

bob.set(Position { x: 20.0, y: 30.0 });

let alice = world.entity_named("Alice").set(Position { x: 10.0, y: 20.0 });
alice.add(Walking);
println!("[{}]", alice.archetype());
alice.remove(Walking);

world.each_entity::<&Position>(|entity, pos| {
    println!("{} has {:?}", entity.name(), pos);
});
```

**New (spec):**
```rust
// Chained setup is unchanged: EntityView is still Copy, setters still return Self,
// and at top level (no open defer scope) the ops apply immediately.
let bob = world
    .entity_named("Bob")
    .set(Position { x: 10.0, y: 20.0 })
    .add(Walking);

// Single Option is not a GuardTuple (only &T / &mut T / tuples are, per the
// prototype). Wrap in a 1-tuple; the all-optional acquire always returns Some. (GAP-3)
let (pos,) = bob.get::<(Option<&Position>,)>().unwrap();
if let Some(pos) = pos {
    println!("Bob's position: {pos:?}");
}
drop(pos); // release the read borrow + defer level before the deferred set below

bob.set(Position { x: 20.0, y: 30.0 }); // deferred under &World

let alice = world.entity_named("Alice").set(Position { x: 10.0, y: 20.0 });
alice.add(Walking);
println!("[{}]", alice.archetype());
alice.remove(Walking);

// No world-level `each_entity::<T>` in the spec (GAP-1): build a query, pass &mut world.
world.query::<&Position>().build().unwrap()
    .each_entity(&mut world, |entity, pos| {
        println!("{} has {:?}", entity.name(), pos);
    });
```
Judgement: the fluent setup surviving verbatim is the pleasant surprise — the "Copy
chaining" concern is smaller than feared because top-level ops are immediate. The plain
read is cleaner as a guard. But the one-liner `world.each_entity` collapses into a
build-a-query-and-thread-`&mut world` two-liner, and the single-`Option` get needs an
ugly 1-tuple. Net wash.

### 2. entity_hierarchy — BETTER

**Old (as-is):**
```rust
fn iterate_tree(entity: EntityView, position_parent: &Position) {
    println!("{} [{:?}]", entity.path().unwrap(), entity.archetype());

    let pos_actual = entity.get::<&Position>(|pos| {
        Position { x: pos.x + position_parent.x, y: pos.y + position_parent.y }
    });

    println!("{pos_actual:?}");
    entity.each_child(|child| {
        iterate_tree(child, &pos_actual);
    });
}
```

**New (spec):**
```rust
fn iterate_tree(entity: EntityView, position_parent: &Position) {
    println!("{} [{:?}]", entity.path().unwrap(), entity.archetype());

    // CPS "get returns a value" becomes a guard + a plain expression. The guard
    // drops at the end of the let, so it does not pin storage across recursion.
    let pos = entity.get::<&Position>().unwrap();
    let pos_actual = Position { x: pos.x + position_parent.x, y: pos.y + position_parent.y };
    drop(pos);

    println!("{pos_actual:?}");
    entity.each_child(|child| {
        iterate_tree(child, &pos_actual);
    });
}
```
Judgement: this is the CPS-`get`-that-returns-a-value case the redesign targets. The
closure-that-only-exists-to-return-a-value disappears; the read is an ordinary borrow
and the computation is an ordinary expression. `each_child` stays a closure (it is
EntityView iteration, allowed under §2.3). Genuinely nicer, lower cognitive load.

### 3. entity_get_multiple — BETTER

**Old (as-is):**
```rust
e.get::<(&mut Position, &mut Mass)>(|(pos, mass)| {
    pos.x += 5.0;
    mass.value += 3.0;
    println!("Position: {{{}, {}}}", pos.x, pos.y);
});

e.get::<(&Position, Option<&Velocity>, &Mass)>(|(pos, velocity, mass)| {
    println!("Position: {{{}, {}}}", pos.x, pos.y);
    if let Some(velocity) = velocity {
        println!("Velocity: {{{}, {}}}", velocity.x, velocity.y);
    }
    println!("Mass: {{{}}}", mass.value);
});
```

**New (spec):**
```rust
let (mut pos, mut mass) = e.get::<(&mut Position, &mut Mass)>().unwrap();
pos.x += 5.0;
mass.value += 3.0;
println!("Position: {{{}, {}}}", pos.x, pos.y);
drop((pos, mass)); // deferred set below wants the write borrows released

// Option in a tuple maps straight to Option<Ref>.
let (pos, velocity, mass) = e.get::<(&Position, Option<&Velocity>, &Mass)>().unwrap();
println!("Position: {{{}, {}}}", pos.x, pos.y);
if let Some(velocity) = velocity {
    println!("Velocity: {{{}, {}}}", velocity.x, velocity.y);
}
println!("Mass: {{{}}}", mass.value);
```
Judgement: the fused tuple acquire is the redesign at its best — one call, destructured
guards, direct field mutation, and NLL scoping. No closure indirection, no borrow of the
whole entity for the closure body. `Option<&Velocity>` -> `Option<Ref>` is exactly the
prototype's `GuardElement` for `Option<&T>`. Cost is one `.unwrap()` and `mut` bindings.
Clear win.

### 4. query_basics — MIXED

**Old (as-is):**
```rust
let query = world.new_query::<(&mut Position, &Velocity)>();

query.each_entity(|e, (pos, vel)| { pos.x += vel.x; pos.y += vel.y; });
query.each(|(pos, vel)| { pos.x += vel.x; pos.y += vel.y; });

query.run(|mut it| {
    while it.next() {
        let mut p = it.field_mut::<Position>(0);
        let v = it.field::<Velocity>(1);
        for i in it.iter() {
            p[i].x += v[i].x;
            p[i].y += v[i].y;
        }
    }
});
```

**New (spec):**
```rust
let query = world.query::<(&mut Position, &Velocity)>().build()?;

query.each_entity(&mut world, |e, (pos, vel)| { pos.x += vel.x; pos.y += vel.y; });
query.each(&mut world, |(pos, vel)| { pos.x += vel.x; pos.y += vel.y; });

// The manual chunk loop becomes the each! macro over the disjoint chunk cursor,
// with real &mut [Position] / &[Velocity] slices and native control flow.
each!((p, v) in query.chunks(&mut world) {
    p.x += v.x;
    p.y += v.y;
});
```
Judgement: the `run` path is a real improvement — the hand-rolled `while it.next()` +
`field_mut`/`field` + index loop becomes the `each!` macro with vectorisable slices and
usable `break`/`continue`/`?`. But the two common paths (`each`, `each_entity`) each
grow a `&mut world` argument, and `new_query` (infallible) becomes `query().build()?`.
For the 90% case (`each`) this is pure argument noise; the win is concentrated in the
`run` escape hatch that most code never touches. Net mixed.

### 5. query_wildcard — WORSE

**Old (as-is):**
```rust
let query = world.new_query::<&(EatsAmount, flecs::Wildcard)>();
// ...
query.each_iter(|it, index, eats| {
    let entity = it.entity(index);
    let pair = it.pair(0);
    let food = pair.second_id();
    println!("{} eats {} {}", entity, eats.amount, food);
});
```

**New (spec):**
```rust
let query = world.query::<&(EatsAmount, flecs::Wildcard)>().build()?;
// ...
// No each_iter in §4.4 (GAP-4). Pair inspection lives on TableIter, so drop to run
// and rebuild the per-row loop by hand.
query.run(&mut world, |mut it| {
    while it.next() {
        let eats = it.field::<EatsAmount>(0);
        let pair = it.pair(0).unwrap();
        let food = pair.second_id();
        for i in it.iter() {
            let entity = it.entity(i);
            println!("{} eats {} {}", entity, eats[i].amount, food);
        }
    }
});
```
Judgement: the whole point of this example — "inspect the pair I am currently matched
with" — depended on `each_iter` handing `(it, index, item)`. The spec's iteration
surface offers `each` (item only) and `each_entity` (entity + item), neither of which
exposes the matched pair, so the only path left is the manual `run` loop. A three-line
readable snippet becomes an eight-line manual iteration. Regression driven by a missing
terminal.

### 6. query_hierarchy — MIXED

**Old (as-is):**
```rust
let query = world
    .query::<(&LocalTransform, Option<&WorldTransform>, &mut WorldTransform)>()
    .term_at(1)
    .parent()
    .cascade()
    .build();

query.each(|(t_local, t_parent, t_world)| {
    t_world.x = t_local.x;
    t_world.y = t_local.y;
    if let Some(t_parent) = t_parent {
        t_world.x += t_parent.x;
        t_world.y += t_parent.y;
    }
});

world.new_query::<&WorldTransform>()
    .each_entity(|entity, p| { println!("{}: {{{}, {}}}", entity.name(), p.x, p.y); });
```

**New (spec):**
```rust
// The parent (cascade) term is Option<&WorldTransform> — read-only. Under §4.9 an Up /
// Cascade term that asked for &mut would not compile at all, which is the safety win.
let query = world
    .query::<(&LocalTransform, Option<&WorldTransform>, &mut WorldTransform)>()
    .term_at(1)
    .parent()
    .cascade()
    .build()?;

query.each(&mut world, |(t_local, t_parent, t_world)| {
    t_world.x = t_local.x;
    t_world.y = t_local.y;
    if let Some(t_parent) = t_parent {
        t_world.x += t_parent.x;
        t_world.y += t_parent.y;
    }
});

world.query::<&WorldTransform>().build()?
    .each_entity(&mut world, |entity, p| {
        println!("{}: {{{}, {}}}", entity.name(), p.x, p.y);
    });
```
Judgement: the builder config (`term_at(1).parent().cascade()`) is unchanged; the closure
bodies are unchanged. The changes are mechanical (`build()?`, `&mut world`). The genuine
gain is invisible in this file but real: a future edit that tried `&mut` on the cascade
parent term is now a compile error, not a runtime aliasing hazard. Slight ceremony,
real structural safety.

### 7. system_basics — MIXED

**Old (as-is):**
```rust
let s = world
    .system::<(&mut Position, &Velocity)>()
    .each_entity(|e, (p, v)| {
        p.x += v.x;
        p.y += v.y;
        println!("{}: {{ {}, {} }}", e.name(), p.x, p.y);
    });
// ...
s.run();
```

**New (spec):**
```rust
let s = world
    .system::<(&mut Position, &Velocity)>()
    .each_entity(|e, (p, v)| {
        p.x += v.x;
        p.y += v.y;
        println!("{}: {{ {}, {} }}", e.name(), p.x, p.y);
    })?; // terminal build now returns Result
// ...
s.run(&mut world); // SystemRunnerFluent deleted; run takes &mut world (§14.7)
```
Judgement: the closure is identical. The costs are two bits of ceremony on a snippet
that cannot meaningfully act on either: `?`/`unwrap` on the `each_entity` build (a bad
system descriptor is a program bug here, not a recoverable condition in a 30-line demo),
and the `&mut world` on `run`. For a hello-world-sized registration this is friction
with no local payoff. The `Result` build earns its keep only in code that builds systems
from dynamic input, which examples are not.

### 8. system_sync_point — WORSE

**Old (as-is):**
```rust
world
    .system_named::<()>("SetVelocitySP")
    .with(&PositionSP::id())
    .set_inout_none()
    .with(&mut VelocitySP::id())
    .each_entity(|e, ()| {
        e.set(VelocitySP { x: 1.0, y: 2.0 });
    });
```

**New (spec):**
```rust
// The callback issues a deferred command (e.set) on the entity it is visiting. The spec
// gives systems world access via the item OR via .each_with(|item, stage|) — but there
// is no each_entity_with: no terminal that hands BOTH the visited entity AND a Stage
// (GAP-6). Two unsatisfying options:

// (a) keep each_entity and issue through the EntityView (its set is deferred to the
//     system's stage). Whether this is allowed without the explicit Stage opt-in, and
//     whether it forces this system onto the lock-registering path, is unspecified.
world
    .system_named::<()>("SetVelocitySP")
    .with(&PositionSP::id())
    .set_inout_none()
    .with(&mut VelocitySP::id())
    .each_entity(|e, ()| {
        e.set(VelocitySP { x: 1.0, y: 2.0 }); // deferred; stage is implicit
    })?;

// (b) use each_with to get an explicit Stage, but then lose the entity ergonomics and
//     re-fetch it from the stage/iter — more machinery than the original one-liner.
```
Judgement: the old code is a tidy one-liner: visit each matched entity, set a component.
The redesign splits "get the entity" (`each_entity`) from "get a command channel"
(`each_with` + `Stage`) into two terminals that do not combine, and routes deferred
commands through the new `Stage` concept. The single most common command-issuing system
shape (visit entity, mutate a *different* component on it) has no first-class terminal.
This is added surface and added concepts for a pattern that was trivial.

### 9. observer_basics — MIXED

**Old (as-is):**
```rust
world
    .observer::<flecs::OnAdd, ()>()
    .with(Position::id())
    .each_iter(|it, index, _| {
        // hand-written warning: don't access the component value on OnAdd, it is
        // uninitialised, which is UB in Rust.
        println!(" - OnAdd: {}: {}", it.event_id().to_str(), it.entity(index));
    });

world
    .observer::<flecs::OnSet, &Position>()
    .add_event(flecs::OnRemove)
    .each_iter(|it, index, pos| {
        println!(" - {}: {}: {}: with {:?}",
            it.event().name(), it.event_id().to_str(), it.entity(index), pos);
    });
```

**New (spec):**
```rust
// OnAdd + a data term is now a COMPILE ERROR (§5.4, D: DataFreeTerms), so the runtime
// "don't touch the value" comment is replaced by the type system. Good.
world
    .observer::<flecs::OnAdd, ()>()
    .with(Position::id())
    // each_iter is gone; event_id()/entity() live on TableIter, so use run. (GAP-4/5)
    .run(|mut it| {
        while it.next() {
            for i in it.iter() {
                println!(" - OnAdd: {}: {}", it.event_id().to_str(), it.entity(i));
            }
        }
    })?;

world
    .observer::<flecs::OnSet, &Position>()
    .add_event(flecs::OnRemove)
    .run(|mut it| {
        while it.next() {
            let pos = it.field::<Position>(0);
            for i in it.iter() {
                println!(" - {}: {}: {}: with {:?}",
                    it.event().name(), it.event_id().to_str(), it.entity(i), &pos[i]);
            }
        }
    })?;
```
Judgement: the `OnAdd`-can't-see-data footgun turning into a compile error (§5.4) is a
real, load-bearing improvement — the example literally carries a paragraph of comment
warning about the exact UB the type now forbids. But every observer here inspects event
metadata (`event()`, `event_id()`), which only `TableIter` exposes, and `each_iter` is
gone, so both observers drop to a manual `run` loop. One win (type safety), one loss
(event-inspecting observers get verbose). Mixed.

### 10. prefab_basics — WORSE (behavior change)

**Old (as-is):**
```rust
let spaceship = world.prefab_named("Prefab").set(Defence { value: 50.0 });
let inst = world.entity_named("my_spaceship").is_a(spaceship);

inst.try_get::<&Defence>(|d_inst| {
    println!("{d_inst:?}");                 // -> Defence { value: 50.0 }
    spaceship.set(Defence { value: 100.0 }); // modify the shared/inherited value
    println!("after set: {d_inst:?}");       // -> Defence { value: 100.0 }  (visible!)
});
```

**New (spec):**
```rust
let spaceship = world.prefab_named("Prefab").set(Defence { value: 50.0 });
let inst = world.entity_named("my_spaceship").is_a(spaceship);

let d_inst = inst.try_get::<&Defence>()?;   // Ref guard: read borrow + one defer level
println!("{d_inst:?}");                      // -> Defence { value: 50.0 }
spaceship.set(Defence { value: 100.0 });     // DEFERRED (§3.6): a guard is live
println!("after set: {d_inst:?}");           // -> Defence { value: 50.0 }  (STALE!)
drop(d_inst);                                // now the deferred set flushes
// The new value is only observable after the guard drops, and only via &mut World.
```
Judgement: this is the sharpest regression, because it is silent. Under §3.6 a shared
`set` issued while any guard is live is deferred until the last guard drops, so the
in-scope re-read observes the pre-write value. The old example's demonstrated output
("after set: 100.0") is no longer reproducible without ending the borrow first. The
migration table (§12) does not flag read-after-write-within-a-guard as a behavior
change, and nothing at the call site signals that the write "did not take" yet. Correct
by the model, surprising in practice.

### 11. relationships_basics — MIXED (neutral)

**Old (as-is):**
```rust
let bob = world
    .entity_named("Bob")
    .add((Eats, apples))
    .add((Eats, pears))
    .add((grows, pears));

println!("Bob eats apples? {}", bob.has((Eats, apples)));
println!("Bob grows food? {}", bob.has((grows, flecs::Wildcard::ID)));

bob.each_target(Eats, |second| { println!("Bob eats {}", second.name()); });
bob.each_pair(flecs::Wildcard::ID, pears, |id| {
    println!("Bob {} pears", id.first_id().name());
});
println!("Bob eats {}", bob.target(Eats, 0).unwrap().name());
```

**New (spec):**
```rust
// Unchanged. Structural adds at top level are immediate; has/target are reads; the
// each_target / each_pair helpers are EntityView iteration (allowed under §2.3, though
// the spec never restates them — GAP-7).
let bob = world
    .entity_named("Bob")
    .add((Eats, apples))
    .add((Eats, pears))
    .add((grows, pears));

println!("Bob eats apples? {}", bob.has((Eats, apples)));
println!("Bob grows food? {}", bob.has((grows, flecs::Wildcard::ID)));

bob.each_target(Eats, |second| { println!("Bob eats {}", second.name()); });
bob.each_pair(flecs::Wildcard::ID, pears, |id| {
    println!("Bob {} pears", id.first_id().name());
});
println!("Bob eats {}", bob.target(Eats, 0).unwrap().name());
```
Judgement: the redesign barely touches this surface. Pair construction, `has`, `target`,
and the relationship-iteration helpers are unchanged, and at top level the chained `add`s
apply immediately. Neither better nor worse — included to confirm the register split does
not spill into the relationship API, which it does not. The only open point is that the
spec's §4 iteration surface never enumerates `each_target`/`each_pair`/`each_child`, so
their post-redesign lock/defer contract is asserted by §2.3 in principle but never spelled
out (GAP-7).

---

## GAP list (spec silences hit while migrating)

- **GAP-1 — world-level one-liner iteration.** `World::each_entity::<T>()` and
  `World::each::<T>()` exist today (`src/core/world/query.rs:166,191`) and are used by
  `entity_basics`, `prefab_basics`, and the print pass of `query_hierarchy`. The spec
  defines iteration only on `Query` (§4.4); it never says whether the ergonomic
  world-level form survives or must expand to `world.query::<T>().build()?.each_entity(&mut world, ..)`.
- **GAP-2 — infallible query constructor.** `world.new_query::<T>()`
  (`src/core/world/query.rs:17`) builds a query without a `Result`. The spec shows only
  `world.query::<T>()...build()?` (§4.1, migration row 7). Whether an infallible
  convenience constructor is retained (and, if so, whether it panics like the old
  `build()` it was meant to replace) is unspecified.
- **GAP-3 — single-element optional get.** The prototype implements `GuardTuple` only for
  `&T`, `&mut T`, and tuples; `Option<&T>` is only a *tuple element* (`GuardElement`), not
  a standalone `GuardTuple` (`entity_access.rs:200-338`). `get::<Option<&Position>>()`
  (used by `entity_basics`, `hello_world`) therefore does not type-check and must be
  written `get::<(Option<&Position>,)>()`. The spec's §3.4 shows only tuple forms and does
  not address the bare-single-optional ergonomic.
- **GAP-4 — `each_iter` has no successor.** `Query::each_iter(|it, index, item|)` (index +
  `TableIter` + item) is the only terminal that exposes the matched pair / iter metadata
  per row. It is absent from the spec's iteration surface (§4.4). `query_wildcard` and both
  observers depend on it; migration forces a manual `run` loop.
- **GAP-5 — observer terminals and event metadata.** §5.1 enumerates system terminals
  (`each`/`each_entity`/`run`/`each_with`) and §5.4 the `OnAdd` bound, but the observer
  builder's terminals are never listed, and event accessors (`event()`, `event_id()`,
  `pair()`) live only on `TableIter`. So any observer that inspects the event must use
  `run`; there is no ergonomic `each`-shaped path to the event, and the spec does not say
  there should be.
- **GAP-6 — no entity + Stage terminal.** Systems can have `each_entity` (entity, no
  stage) or `each_with` (item + `Stage`), but not both. The extremely common "visit each
  matched entity and issue a deferred command on it" shape (`system_sync_point`) has no
  first-class terminal. An `each_entity_with(|entity, item, stage|)` is the obvious missing
  piece.
- **GAP-7 — EntityView relationship/hierarchy iteration helpers.** `each_child`,
  `each_target`, `each_pair` (used by `entity_hierarchy`, `relationships_basics`) are
  closure-based iteration on `EntityView`. §2.3 permits them in principle (EntityView
  carries the borrow), but the spec never restates their lock-registration or defer
  behavior, nor whether their callbacks may themselves take guards.
- **GAP-8 — deferred-write visibility is a behavior change, unflagged.** §3.6 makes a
  shared `set` deferred whenever any guard is live, so read-after-write within a guard
  scope observes the *old* value (`prefab_basics`). This changes observable behavior of
  existing code, but the migration guide (§12) does not list it, and nothing at the call
  site signals the write is still pending.
- **GAP-9 — stored System run vs progress borrow shape.** §14.7 makes `system.run(&mut world)`,
  while `world.progress()` remains a `&World`-style call. Mixed usage (manually running a
  stored `System` and also calling `progress()`) crosses `&mut World` / `&World` borrows of
  the same world; the spec does not state whether `progress` moves to `&mut World`, nor how
  a stored `System`/`Query` handle coexists with a `&mut world` iteration borrow across a
  frame.
- **GAP-10 — typed pair set.** `set_pair::<A, B>(value)` (`query_wildcard`) and
  `set_first`/`set_second` are unmentioned. Assumed retained, but the register split's
  effect on pair setters (deferred under `&World`, immediate on `EntityMut`) is not stated.
- **GAP-11 — id-expression helpers under the new surface.** `Component::id()`,
  `id::<T>()`, `flecs::Wildcard::ID` (used across relationships/observers/wildcard) feed
  `.with(...)` on builders. The DSL is retained (§14.14), but the spec does not restate
  that `.with(Position::id())` on the now-`Result`-returning builders is unaffected.
- **GAP-12 — no multithreaded example exists.** There is no `par_each` / `multi_threaded`
  example anywhere under `examples/flecs/`. The `Stage<'s>`-in-worker / `par_each_with`
  ergonomics (§6) — arguably the redesign's highest-stakes surface — cannot be evaluated
  against real usage, only against the spec. The examples suite itself should grow one.

---

## Aggregate verdict

Counts over 11 examples: **BETTER 2, MIXED 6, WORSE 3.**

The redesign is a clear win exactly where the design record aimed it — multi-component
tuple reads and CPS-`get`-returning-a-value (examples 2, 3) shed a layer of closure
indirection and read like ordinary Rust with NLL scoping. It is neutral-to-slightly-worse
on the broad middle: nearly every query and system snippet grows a `&mut world` argument
and a `?`/`unwrap` on build, which is pure ceremony on the hello-world-sized examples that
dominate a "getting started" tour (examples 1, 4, 6, 7). And it regresses in three places:
one silent behavior change (deferred write under a live guard, example 10) and two missing
terminals (`each_iter` for pair/event inspection, and an entity+`Stage` system terminal —
examples 5, 8, 9).

The chained-fluent-setup fear was overblown: because `EntityView` stays `Copy` with
`Self`-returning setters and top-level ops are immediate, entity construction migrates
almost verbatim. The real ergonomic tax lands on iteration (the `&mut world` thread-through)
and on the two-way split of "give me the entity" vs "give me a command channel" in systems.

### 3 worst regressions
1. **prefab_basics (example 10) — silent read-after-write behavior change.** A shared
   `set` while a guard is live is deferred (§3.6), so the in-scope re-read sees the stale
   value. Existing code that reads back its own write within a `get`/`try_get` scope
   changes output with no compile error and no migration-guide note (GAP-8).
2. **query_wildcard / observers (examples 5, 9) — `each_iter` deleted with no successor.**
   Pair- and event-inspecting iteration is the natural shape for wildcard queries and
   observers, and it only existed through `each_iter(|it, index, item|)`. Its removal
   (GAP-4/5) forces every such snippet down to a manual `run` loop, roughly tripling the
   line count of the readable form.
3. **system_sync_point (example 8) — no entity+Stage terminal.** The commonest
   command-issuing system ("visit each matched entity, mutate another component") falls
   between `each_entity` (no stage) and `each_with` (no entity). GAP-6 leaves it with an
   unspecified implicit-stage `each_entity` path or a clumsy `each_with` rewrite.

### 3 best improvements
1. **entity_get_multiple (example 3) — fused tuple acquire.** `get::<(&mut A, &mut B)>()`
   -> destructured `(Mut<A>, Mut<B>)` with direct mutation and left-to-right rollback
   semantics; the closure and its whole-entity borrow vanish. The prototype's
   `resolve_and_lock` makes this real, including `Option<&T>` -> `Option<Ref>`.
2. **entity_hierarchy (example 2) — CPS-that-returns-a-value collapses.** The closure whose
   only job was to compute and return a `Position` becomes an ordinary borrow plus an
   ordinary expression; it composes with the surrounding recursion instead of nesting.
3. **observer / query type-level safety (examples 9, 6).** `OnAdd` + a data term is now a
   compile error (§5.4) instead of a runtime "don't touch the value" comment guarding real
   UB, and `Up`/`Cascade` parent terms are `&mut`-impossible in the type system (§4.9).
   Both convert a documented footgun into a rejection the compiler makes for you.
