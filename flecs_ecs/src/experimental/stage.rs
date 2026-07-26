//! Per-callback stage handle: [`Stage`] (spec §6.1, §7.3).
//!
//! [`Stage<'s>`] is the "commands" surface handed to a system callback by the
//! `_with` terminals ([`each_with`](crate::core::SystemAPI), `each_entity_with`,
//! `par_each_with`). It wraps the **stage world pointer** flecs passes to the
//! callback (`it->world`), through which structural commands are issued.
//!
//! # Why it cannot escape the callback
//!
//! * **No public constructor.** [`Stage::new`] is `pub(crate)`; only the system
//!   trampolines build one, from the iterator's stage world, scoped to the
//!   single callback invocation.
//! * **`!Send` / `!Sync`.** A `PhantomData<*const ()>` marker keeps the handle on
//!   the worker thread that owns the stage, so it can never be moved to another
//!   thread (the per-stage lock maps are single-owner, spec §6.1).
//! * **`!Copy` / `!Clone`.** The handle cannot be duplicated out of the callback.
//! * **Higher-ranked lifetime.** The `_with` terminals bind the callback as
//!   `FnMut(Item<'_>, Stage<'_>)`, i.e. `for<'x> FnMut(.., Stage<'x>)`. The
//!   closure must accept *any* `'x`, so it cannot unify `'x` with an outer
//!   lifetime and stash the handle: a stored `Stage` would name a lifetime the
//!   closure does not get to choose, and fails to borrow-check.
//!
//! # Deferred commands
//!
//! Every op issued through a [`Stage`] targets the **stage world**, not the real
//! world. During staged pipeline execution flecs runs each system with its
//! worker stage in deferred mode (`ecs_run` opens `ecs_defer_begin` on the stage
//! before dispatch; the C `flecs_defer_op` path enqueues structural ops on a
//! stage whenever `stage->defer > 0`), so `set` / `add` / `remove` and
//! [`spawn`](Stage::spawn) enqueue onto the stage's command buffer and are merged
//! into the real world at the next sync point. Reads still resolve against real
//! storage. This is why the deferred-command handle is just an
//! [`EntityView`](crate::core::EntityView) over the stage world: its setters
//! defer automatically because the stage is deferred.

use core::marker::PhantomData;

use crate::core::{EntityView, IntoEntity, WorldProvider, WorldRef};

use super::bundle::Bundle;

/// A per-callback handle to the worker's stage (spec §6.1, §7.3).
///
/// Handed to `_with` system callbacks as the second (or third) argument. Use it
/// to issue **deferred** structural commands — create entities, and `set` /
/// `add` / `remove` on entities — that are merged into the world at the next
/// sync point. It exposes the frame timestep via [`delta_time`](Stage::delta_time).
///
/// The handle is `!Send` / `!Sync` / `!Copy` and lives only for the callback
/// invocation; it has no public constructor and cannot be stored or moved off
/// the worker thread (see the module docs for the escape argument).
///
/// `count()` is intentionally **not** on `Stage`: the batch row count is a
/// per-iteration property, so it lives on
/// [`Iter`](crate::experimental::Iter) (reached through the `each_iter`
/// terminals), not on the per-stage handle.
///
/// # A `Stage` cannot be constructed outside a callback
///
/// The constructor is crate-private and the fields are private, so downstream
/// code has no way to build one; the only source is a `_with` system callback.
/// Both attempts below fail to compile:
///
/// ```compile_fail
/// use flecs_ecs::experimental::Stage;
/// use flecs_ecs::prelude::*;
///
/// let world = World::new();
/// // `Stage::new` is `pub(crate)`: not callable here.
/// let _s = Stage::new(world.world(), 0.0);
/// ```
///
/// ```compile_fail
/// use flecs_ecs::experimental::Stage;
///
/// // Private fields: no struct-literal construction from outside the crate.
/// let _s: Stage<'static> = Stage { world: todo!(), delta_time: 0.0 };
/// ```
pub struct Stage<'s> {
    world: WorldRef<'s>,
    delta_time: f32,
    /// `!Send` / `!Sync`, and (with the absent `Copy`/`Clone` derives) `!Copy`.
    _not_send_sync: PhantomData<*const ()>,
}

impl<'s> Stage<'s> {
    /// Build a stage handle over the callback's stage world.
    ///
    /// Crate-only: the system `_with` trampolines are the sole callers, passing
    /// the iterator's stage world (`it->world`) and its `delta_time`. There is no
    /// public constructor, which is what keeps the handle un-forgeable outside a
    /// callback (module docs).
    #[inline(always)]
    pub(crate) fn new(world: WorldRef<'s>, delta_time: f32) -> Self {
        Stage {
            world,
            delta_time,
            _not_send_sync: PhantomData,
        }
    }

    /// Time elapsed since the last frame (the iterator's `delta_time`).
    ///
    /// Captured from the running iteration when the handle is built, so timestep
    /// driven `_with` systems need no [`Iter`](crate::experimental::Iter).
    #[inline(always)]
    pub fn delta_time(&self) -> f32 {
        self.delta_time
    }

    /// Deferred-create a new entity on the stage, returning a handle for further
    /// deferred commands.
    ///
    /// The id is reserved immediately; component ops on the returned view enqueue
    /// and merge at the next sync point. Only valid from a single-threaded
    /// callback: id allocation from a worker during the multithreaded pipeline
    /// phase races and panics (as elsewhere in the safe API).
    #[inline]
    pub fn entity(&self) -> EntityView<'s> {
        EntityView::new(self.world)
    }

    /// A deferred-command handle to an existing entity `id` on the stage.
    ///
    /// Reads resolve against real storage; `set` / `add` / `remove` on the
    /// returned view enqueue on the stage and merge at the next sync point. This
    /// is the "issue a deferred command on this entity" path
    /// (`stage.entity_view(e).set(..)`).
    #[inline]
    pub fn entity_view(&self, id: impl IntoEntity) -> EntityView<'s> {
        EntityView::new_from(self.world, id)
    }

    /// Deferred twin of `World::spawn`: enqueue a whole [`Bundle`] onto a new
    /// entity (spec §7.3).
    ///
    /// A staged `spawn` cannot use the immediate `ecs_bulk_init` archetype move
    /// (it must create and observe its ids at once, impossible under an open
    /// defer scope), so it reserves the entity and enqueues one deferred `set`
    /// per component; flecs merges the same-entity commands into a single
    /// archetype move at the sync point.
    #[inline]
    pub fn spawn<B: Bundle>(&self, bundle: B) -> EntityView<'s> {
        let entity = self.entity();
        bundle.apply_set(entity);
        entity
    }
}

impl<'s> WorldProvider<'s> for Stage<'s> {
    #[inline(always)]
    fn world(&self) -> WorldRef<'s> {
        self.world
    }
}
