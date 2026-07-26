//! Bundles: a set of components inserted as one archetype move (spec §4.11).
//!
//! A [`Bundle`] is implemented for tuples of components via the crate's
//! `tuples!` macro, so `(A, B, C, ..)` is a `Bundle` up to arity 31. The trait
//! is a sealed implementation detail: its kernel resolves the component id list
//! for a world and writes each element's value into a destination column
//! pointer. Downstream crates cannot implement it.

use core::any::TypeId;
use core::ffi::c_void;
use core::mem::ManuallyDrop;

extern crate alloc;
use alloc::vec::Vec;

use crate::core::{ComponentId, ComponentInfo, Entity, EntityView, World, WorldProvider, WorldRef};
use crate::sys;

use crate::core::access::EntityMut;

mod private {
    pub trait Sealed {}
}

/// Largest supported bundle arity. flecs' `ecs_bulk_desc_t::ids` is a fixed
/// `[ecs_id_t; FLECS_ID_DESC_MAX]` (32) array that flecs reads until a null
/// terminator, so at most 31 real ids fit alongside the terminator.
const MAX_BUNDLE_ARITY: usize = 31;
const ID_BUF: usize = 32;

/// A set of components that can be inserted into an entity as a single
/// archetype move.
///
/// `Bundle` is implemented for tuples of `#[derive(Component)]` types up to
/// arity 31 (bounded by flecs' `FLECS_ID_DESC_MAX`, which must reserve one slot
/// for a null terminator). It powers [`World::spawn`](crate::core::World::spawn),
/// [`World::spawn_batch`](crate::core::World::spawn_batch) and
/// [`EntityView::insert`](crate::core::EntityView::insert), each of which
/// replaces N per-component archetype moves with one.
///
/// The trait is **sealed**: its kernel methods are a `#[doc(hidden)]`
/// implementation detail and cannot be implemented outside this crate.
#[diagnostic::on_unimplemented(
    message = "`{Self}` is not a `Bundle`.",
    label = "a `Bundle` is a tuple of `#[derive(Component)]` types, e.g. `(A, B, C)`, up to arity 31",
    note = "each element must implement `ComponentId` (via `#[derive(Component)]`)"
)]
pub trait Bundle: private::Sealed + Sized + 'static {
    /// Number of components in the bundle (its tuple arity).
    #[doc(hidden)]
    const ARITY: usize;

    /// Resolves (registering on first use) each component id for `world` into
    /// `out`, in tuple order. Performs the per-component thread-affinity check
    /// required to move the value into storage.
    ///
    /// `out.len()` must be `>= ARITY`.
    #[doc(hidden)]
    fn resolve_ids(world: WorldRef, out: &mut [u64]);

    /// Writes `size_of::<T>()` per component into `out`, in tuple order.
    #[doc(hidden)]
    fn sizes(out: &mut [usize]);

    /// Writes a data pointer per component into `out`, in tuple order. Zero-sized
    /// (tag) components get a null pointer, matching flecs' tag contract for
    /// `ecs_bulk_init`.
    ///
    /// # Safety
    ///
    /// `this` must point to a live, initialized `Self`. The returned pointers
    /// borrow from `*this`; the caller MUST NOT drop `*this` afterwards, because
    /// the pointed-to bytes are moved (memcpy) into flecs storage.
    #[doc(hidden)]
    unsafe fn data_ptrs(this: *mut Self, out: &mut [*mut c_void]);

    /// Consumes the bundle, issuing one `set` per component on `entity` in tuple
    /// order. Used by `insert`'s deferred fallback (per-component add/set).
    #[doc(hidden)]
    fn apply_set(self, entity: EntityView);

    /// Consumes the bundle, writing each value into its column on `entity`
    /// (already moved to the destination table by a single `ecs_commit`) and
    /// firing `OnSet`. `added_mask[i]` is `true` when component `i` was newly
    /// added by the commit (its slot is default-constructed or uninitialized),
    /// `false` when it pre-existed (its slot holds a live value that must be
    /// dropped before overwrite).
    #[doc(hidden)]
    fn write_after_commit(self, entity: EntityView, added_mask: &[bool]);
}

/// Debug-only backstop against a live-guard bulk construction (spec §4.11).
///
/// `spawn` / `spawn_batch` now take `&mut World`, so from safe code no
/// shared-register guard (which borrows the world *shared*) can be alive across
/// the call: the borrow checker rejects it (see the compile-fail doctests on
/// [`WorldBundleExt::spawn`]). The runtime refusal is therefore unreachable from
/// safe code and is kept only as a `debug_assert` backstop against an `unsafe` /
/// raw path that fabricated a `&mut World` while a guard's pin was still live —
/// where `ecs_bulk_init` could reallocate the pinned column out from under the
/// guard. It costs nothing in release and is compiled out entirely without
/// `flecs_safety_locks`.
#[cfg(feature = "flecs_safety_locks")]
#[inline(always)]
#[track_caller]
fn assert_no_live_guards(world: &WorldRef, op: &str) {
    let locks = crate::core::stage_locks_dyn(world);
    // SAFETY: the stage map is owned by the calling thread.
    debug_assert!(
        !unsafe { (*locks.as_ptr()).has_live_pin() },
        "cannot {op} while component guards are live on this stage: `ecs_bulk_init` \
         appends rows to existing tables and can reallocate a pinned column, which would \
         dangle the guard. Reachable only via an `unsafe`/raw `&mut World`; safe code is \
         refused at compile time"
    );
}

#[cfg(not(feature = "flecs_safety_locks"))]
#[inline(always)]
fn assert_no_live_guards(_world: &WorldRef, _op: &str) {}

#[cold]
#[inline(never)]
#[track_caller]
fn bundle_deferred_panic(op: &str) -> ! {
    panic!(
        "cannot {op} while the world is deferred: the operation must create and observe \
         its entity ids immediately, which `ecs_bulk_init` cannot do under an open defer \
         scope. Call {op} outside `defer()` / deferred callbacks and with no live \
         write-episode guards"
    );
}

/// Adds and sets a single bundle element on `entity`, dispatching on whether the
/// component is a zero-sized tag at runtime.
///
/// This mirrors the crate's `set` path (emplace for a new component, drop then
/// overwrite for an existing one) but, unlike the public `set`, carries no
/// compile-time "not a tag" assertion, so it is safe to instantiate for a tag
/// element inside the generic bundle plumbing.
#[inline]
fn set_bundle_element<T: ComponentId>(entity: EntityView, value: T) {
    let world = entity.world;
    let id = T::entity_id(world);
    let world_ptr = world.world_ptr_mut();

    if core::mem::size_of::<T>() == 0 {
        // Tag: add the id; the zero-sized value carries no data. `drop` honors a
        // (rare) tag `Drop` impl and is otherwise a no-op.
        unsafe { sys::ecs_add_id(world_ptr, *entity.id, id) };
        drop(value);
        return;
    }

    world.check_thread_affinity_exclusive::<T>();

    // SAFETY: `id`/`value`/size are consistent; `ecs_rust_set` skips the ctor for
    // a new component (emplace) and returns storage (or the deferred command
    // buffer). We move `value` into that slot exactly once, dropping the prior
    // value only when overwriting an existing (non-new) component.
    unsafe {
        let res = sys::ecs_rust_set(
            world_ptr,
            *entity.id,
            id,
            (&value as *const T).cast::<c_void>(),
            core::mem::size_of::<T>(),
        );
        assert!(
            !res.ptr.is_null(),
            "insert failed: entity is not alive or the world is invalid"
        );
        let comp = res.ptr.cast::<T>();
        if T::NEEDS_DROP && !res.is_new {
            core::ptr::drop_in_place(comp);
        }
        core::ptr::write(comp, value);
        if res.call_modified {
            sys::ecs_modified_id(world_ptr, *entity.id, id);
        }
    }
}

macro_rules! bundle_count {
    () => { 0usize };
    ($head:ident $(, $tail:ident)*) => { 1usize + bundle_count!($($tail),*) };
}

macro_rules! impl_bundle {
    ($($t:ident),*) => {
        impl<$($t: ComponentId),*> private::Sealed for ($($t,)*) {}

        impl<$($t: ComponentId),*> Bundle for ($($t,)*) {
            const ARITY: usize = bundle_count!($($t),*);

            #[inline]
            fn resolve_ids(_world: WorldRef, _out: &mut [u64]) {
                let mut _i = 0usize;
                $(
                    _world.check_thread_affinity_exclusive::<$t>();
                    _out[_i] = $t::entity_id(_world);
                    _i += 1;
                )*
            }

            #[inline]
            fn sizes(_out: &mut [usize]) {
                let mut _i = 0usize;
                $(
                    _out[_i] = core::mem::size_of::<$t>();
                    _i += 1;
                )*
            }

            #[inline]
            unsafe fn data_ptrs(_this: *mut Self, _out: &mut [*mut c_void]) {
                // SAFETY: caller guarantees `_this` points to a live, initialized
                // `Self`; we only take addresses of its fields, never move them.
                let ($($t,)*) = unsafe { &mut *_this };
                let mut _i = 0usize;
                $(
                    _out[_i] = if core::mem::size_of::<$t>() == 0 {
                        core::ptr::null_mut()
                    } else {
                        ($t as *mut $t).cast::<c_void>()
                    };
                    _i += 1;
                )*
            }

            #[inline]
            #[allow(clippy::unused_unit)]
            fn apply_set(self, _entity: EntityView) {
                let ($($t,)*) = self;
                $(
                    set_bundle_element(_entity, $t);
                )*
            }

            #[inline]
            #[allow(clippy::unused_unit)]
            fn write_after_commit(self, _entity: EntityView, _added_mask: &[bool]) {
                let _world = _entity.world;
                let _world_ptr = _world.world_ptr_mut();
                let _eid = *_entity.id;
                let ($($t,)*) = self;

                // Pass 1: move every value into its column.
                let mut _i = 0usize;
                $(
                    if core::mem::size_of::<$t>() == 0 {
                        // Tag: no data column; the commit already added it.
                        drop($t);
                    } else {
                        _world.check_thread_affinity_exclusive::<$t>();
                        let id = $t::entity_id(_world);
                        // SAFETY: the entity is in a table that contains `id`
                        // (the commit added the whole bundle), so `ecs_get_mut_id`
                        // returns a valid, correctly-typed slot. We overwrite it
                        // exactly once, dropping the prior value only when the
                        // slot already holds a live value.
                        unsafe {
                            let ptr = sys::ecs_get_mut_id(_world_ptr, _eid, id).cast::<$t>();
                            debug_assert!(!ptr.is_null());
                            let has_live = !_added_mask[_i]
                                || <$t as ComponentInfo>::IMPLS_DEFAULT;
                            if has_live && <$t as ComponentInfo>::NEEDS_DROP {
                                core::ptr::drop_in_place(ptr);
                            }
                            core::ptr::write(ptr, $t);
                        }
                    }
                    _i += 1;
                )*

                // Pass 2: fire OnSet once every value is in place (so
                // multi-component OnSet observers see a fully populated entity),
                // matching the bulk path.
                $(
                    if core::mem::size_of::<$t>() != 0 {
                        let id = $t::entity_id(_world);
                        unsafe { sys::ecs_modified_id(_world_ptr, _eid, id) };
                    }
                )*
            }
        }
    };
}

use flecs_ecs_derive::tuples;
tuples!(impl_bundle, 1, 31);

/// Records (once per bundle type per world) the bundle's resolved component
/// id set, keyed by `TypeId<B>` in the §8.2 per-world cache. Stores the
/// SORTED, de-duplicated id array, never a table pointer: tables can be
/// deleted, component ids cannot.
///
/// The first call sorts the ids, rejects any duplicate component type (which
/// would alias mutable storage), and memoizes the result so later spawns
/// skip the sort and the duplicate check.
///
/// # Panics
///
/// Panics if the bundle contains a duplicate component type.
fn record_bundle_ids<B: Bundle>(world: &World, ids: &[u64]) {
    let type_id = TypeId::of::<B>();
    let cache = &world.world_ctx().bundle_ids;
    if cache.borrow().contains_key(&type_id) {
        return;
    }

    let mut sorted: Vec<u64> = ids.to_vec();
    sorted.sort_unstable();
    for w in sorted.windows(2) {
        assert!(
            w[0] != w[1],
            "bundle contains a duplicate component type (id {}); \
             each component may appear in a bundle at most once",
            w[0]
        );
    }
    cache
        .borrow_mut()
        .insert(type_id, sorted.into_boxed_slice());
}

/// Bundle construction on [`World`] (spec §4.11). Provisional name; the
/// intended final surface is inherent `World::spawn` / `World::spawn_batch`.
pub trait WorldBundleExt {
    /// Constructs one entity from a bundle in a single archetype move (spec
    /// §4.11), on the exclusive register.
    ///
    /// Resolves the bundle's component id list once per bundle type per world
    /// (caching the sorted id array), then moves the bundle's values into
    /// storage via one `ecs_bulk_init` call (count = 1). `OnAdd` fires for every
    /// component and `OnSet` fires for every non-tag component, exactly as for
    /// the equivalent `entity().set()...` sequence (same set and counts of
    /// observers; the per-component *interleaving* order differs because this is
    /// one move, not N).
    ///
    /// The bundle's values are **moved** into storage; they are not dropped on
    /// the Rust side. First use of a bundle type registers any unregistered
    /// components.
    ///
    /// Takes `&mut World` and returns an [`EntityMut`] for further immediate ops,
    /// so `world.spawn((A, B)).set(C)` lands all three in one exclusive scope.
    ///
    /// # Live guards are a compile error
    ///
    /// Because `spawn` takes `&mut World`, no shared-register guard (which borrows
    /// the world *shared*) can be live across the call: `ecs_bulk_init` could
    /// reallocate the guard's pinned column, and the borrow checker refuses the
    /// aliasing outright. This is the refusal the previous runtime panic covered,
    /// now moved to compile time:
    ///
    /// ```compile_fail
    /// use flecs_ecs::prelude::*;
    /// use flecs_ecs::experimental::prelude::*;
    ///
    /// #[derive(Component)]
    /// struct Pos { x: i32 }
    ///
    /// let mut world = World::new();
    /// let e = world.spawn((Pos { x: 1 },));
    /// let g = e.get::<&Pos>().unwrap();      // borrows `world` shared
    /// let _ = world.spawn((Pos { x: 2 },));      // needs `&mut world`: conflict
    /// let _ = g.x;
    /// ```
    ///
    /// # Panics
    ///
    /// Panics if the bundle contains a duplicate component type, if the bundle
    /// arity exceeds 31, or if the world is in its multithreaded execution phase.
    ///
    /// Also panics if the world is deferred (for example between
    /// [`World::defer_begin`](crate::core::World::defer_begin) /
    /// [`World::defer_end`](crate::core::World::defer_end)): `ecs_bulk_init` must
    /// observe its entity ids immediately and cannot run under an open defer
    /// scope. Leave any defer scope before spawning.
    ///
    /// The live-guard `debug_assert` backstop is part of the `flecs_safety_locks`
    /// bookkeeping and guards only against an `unsafe`/raw `&mut World`; building
    /// without that feature compiles it out.
    fn spawn<B: Bundle>(&mut self, bundle: B) -> EntityMut<'_>;

    /// Constructs `count` entities from one bundle in a single `ecs_bulk_init`
    /// call (spec §4.11).
    ///
    /// The bundle is cloned once per row except for the last row, which moves
    /// the original in. Component values are laid out column-major into
    /// temporary buffers and moved into storage in one bulk operation; `OnAdd`
    /// and `OnSet` fire for every row exactly as they would for `count`
    /// individual `spawn` calls.
    ///
    /// Returns the ids flecs allocated, in creation order.
    ///
    /// # Panics
    ///
    /// Panics if the bundle contains a duplicate component type, if the bundle
    /// arity exceeds 31, or if the world is in its multithreaded execution
    /// phase.
    ///
    /// Takes `&mut World`, so a live shared-register guard across the call is a
    /// compile error, exactly as for [`spawn`](WorldBundleExt::spawn). Also
    /// panics under an open defer scope, for the same reason as `spawn`.
    fn spawn_batch<B: Bundle + Clone>(&mut self, bundle: B, count: usize) -> Vec<Entity>;
}

impl WorldBundleExt for World {
    #[track_caller]
    fn spawn<B: Bundle>(&mut self, bundle: B) -> EntityMut<'_> {
        const {
            assert!(
                B::ARITY <= MAX_BUNDLE_ARITY,
                "bundle arity exceeds the maximum of 31 components"
            );
        }

        let world_ptr = self.raw_world.as_ptr();
        // SAFETY: `world_ptr` is this world's live pointer; `&mut self` is held
        // for the whole call (and the returned handle for `spawn`).
        let world = unsafe { WorldRef::from_ptr(world_ptr) };
        crate::core::assert_not_in_multithreaded_phase(world_ptr);
        assert_no_live_guards(&world, "World::spawn");
        if self.is_deferred() {
            bundle_deferred_panic("World::spawn");
        }

        let arity = B::ARITY;
        let mut ids = [0u64; ID_BUF];
        B::resolve_ids(world, &mut ids[..arity]);
        record_bundle_ids::<B>(self, &ids[..arity]);

        // The bundle's bytes are moved (memcpy) into storage by flecs; wrap in
        // `ManuallyDrop` so Rust never drops the source and no double-drop can
        // occur.
        let mut bundle = ManuallyDrop::new(bundle);
        let mut data = [core::ptr::null_mut::<c_void>(); ID_BUF];
        // SAFETY: `bundle` is live and initialized; `data_ptrs` only reads field
        // addresses. We forget `bundle` below, so its bytes are owned solely by
        // storage after `ecs_bulk_init`.
        unsafe {
            B::data_ptrs(&mut *bundle as *mut B, &mut data[..arity]);
        }

        // `desc.table` is intentionally left null: with a preset table,
        // `ecs_bulk_init` takes a branch (flecs.c ~9371) that leaves the diff's
        // `added_flags` at zero, which gates OFF all `OnAdd` observers
        // (flecs.c:7323). Passing `desc.ids` with a null table instead drives
        // the table-graph branch, which computes `added_flags` correctly and
        // fires `OnAdd`, matching the per-component `set` path. flecs' table
        // edge graph makes this resolution cheap and never yields a table
        // pointer we would have to cache.
        let mut desc: sys::ecs_bulk_desc_t = unsafe { core::mem::zeroed() };
        desc.count = 1;
        desc.data = data.as_mut_ptr();
        desc.ids[..arity].copy_from_slice(&ids[..arity]);

        let id_ptr = unsafe { sys::ecs_bulk_init(world_ptr, &desc) };
        assert!(
            !id_ptr.is_null(),
            "ecs_bulk_init failed while spawning a bundle"
        );
        // Copy the id out immediately; the returned array aliases internal state.
        let id = unsafe { *id_ptr };

        // SAFETY: `world` names this live world; we hold `&mut self` for the
        // returned handle's lifetime, and `id` was just created live.
        unsafe { EntityMut::new(world, Entity::new(id)) }
    }

    #[track_caller]
    fn spawn_batch<B: Bundle + Clone>(&mut self, bundle: B, count: usize) -> Vec<Entity> {
        const {
            assert!(
                B::ARITY <= MAX_BUNDLE_ARITY,
                "bundle arity exceeds the maximum of 31 components"
            );
        }

        let world_ptr = self.raw_world.as_ptr();
        // SAFETY: `world_ptr` is this world's live pointer; `&mut self` is held
        // for the whole call (and the returned handle for `spawn`).
        let world = unsafe { WorldRef::from_ptr(world_ptr) };
        crate::core::assert_not_in_multithreaded_phase(world_ptr);
        // Checked before the count == 0 early return so misuse fails
        // deterministically instead of depending on the requested count.
        assert_no_live_guards(&world, "World::spawn_batch");
        if self.is_deferred() {
            bundle_deferred_panic("World::spawn_batch");
        }

        if count == 0 {
            // Nothing is stored; drop the bundle normally.
            drop(bundle);
            return Vec::new();
        }

        let arity = B::ARITY;
        let mut ids = [0u64; ID_BUF];
        B::resolve_ids(world, &mut ids[..arity]);
        record_bundle_ids::<B>(self, &ids[..arity]);

        let mut sizes = [0usize; ID_BUF];
        B::sizes(&mut sizes[..arity]);

        // Column-major staging: one contiguous byte buffer of `count` elements
        // per non-tag component. Bytes are memcpy-moved into storage by flecs;
        // dropping these `Vec<u8>` buffers only frees raw bytes and never runs a
        // component destructor, so there is no double drop.
        let mut columns: Vec<Vec<u8>> = (0..arity)
            .map(|i| {
                if sizes[i] == 0 {
                    Vec::new()
                } else {
                    alloc::vec![0u8; sizes[i] * count]
                }
            })
            .collect();

        // Writes row `r`'s field bytes into the column buffers, then forgets the
        // instance so its values are owned solely by the staging buffers.
        //
        // SAFETY: `inst` points to a live, initialized `B`; each field is
        // memcpy-copied out and the instance is never dropped by the caller.
        let write_row = |inst: *mut B, r: usize, columns: &mut [Vec<u8>]| {
            let mut ptrs = [core::ptr::null_mut::<c_void>(); ID_BUF];
            unsafe { B::data_ptrs(inst, &mut ptrs[..arity]) };
            for i in 0..arity {
                let sz = sizes[i];
                if sz != 0 {
                    unsafe {
                        core::ptr::copy_nonoverlapping(
                            ptrs[i].cast::<u8>(),
                            columns[i].as_mut_ptr().add(r * sz),
                            sz,
                        );
                    }
                }
            }
        };

        let mut orig = ManuallyDrop::new(bundle);
        for r in 0..count {
            if r + 1 < count {
                let mut c = ManuallyDrop::new((*orig).clone());
                write_row(&mut *c as *mut B, r, &mut columns);
            } else {
                write_row(&mut *orig as *mut B, r, &mut columns);
            }
        }

        let mut data = [core::ptr::null_mut::<c_void>(); ID_BUF];
        for i in 0..arity {
            data[i] = if sizes[i] == 0 {
                core::ptr::null_mut()
            } else {
                columns[i].as_mut_ptr().cast::<c_void>()
            };
        }

        // `desc.table` left null on purpose; see `spawn` for why (OnAdd gating).
        let mut desc: sys::ecs_bulk_desc_t = unsafe { core::mem::zeroed() };
        desc.count = count as i32;
        desc.data = data.as_mut_ptr();
        desc.ids[..arity].copy_from_slice(&ids[..arity]);

        let id_ptr = unsafe { sys::ecs_bulk_init(world_ptr, &desc) };
        assert!(
            !id_ptr.is_null(),
            "ecs_bulk_init failed while spawning a bundle batch"
        );
        // Copy the ids out immediately; the returned array aliases internal state.
        let mut out = Vec::with_capacity(count);
        for i in 0..count {
            out.push(Entity::new(unsafe { *id_ptr.add(i) }));
        }
        out
    }
}

/// Bundle insertion on [`EntityView`] (spec §4.11). Provisional name; the
/// intended final surface is `EntityMut::insert`.
pub trait EntityBundleExt: Sized {
    /// Adds and sets a whole bundle on an existing entity in a **single**
    /// archetype move (spec §4.11), not N.
    ///
    /// On an immediate (non-deferred) world this computes the destination table
    /// once (adding the bundle's ids to the entity's current table) and performs
    /// one `ecs_commit` structural move, then moves each value into place and
    /// fires `OnSet`. `OnAdd` is emitted after the single move, so an observer
    /// sees the entity already in its final table; `OnSet` fires once per non-tag
    /// component, matching the per-component `set` path's observer counts.
    ///
    /// Deferred context: if the world is already deferred when `insert` is called
    /// (for example from inside a system or
    /// [`World::defer`](crate::core::World::defer)), the bundle is applied as a
    /// per-component add/set on the active stage and merged at the enclosing sync
    /// point. Note this deferred path is **not** a single archetype move: the
    /// Rust `set` fast path emplaces new components (skipping wasted default
    /// construction), and flecs deliberately excludes emplace commands from its
    /// same-entity command batching (flecs.c:6460), so each component is a
    /// separate transition. The result is identical; only the number of internal
    /// moves differs.
    ///
    /// Live guards: `insert` is a shared-register write, so it first runs the
    /// write-episode hook (spec §3.6). If any
    /// [`Ref`](crate::experimental::Ref) / [`Mut`](crate::experimental::Mut)
    /// guard is live on this stage, the hook lazily opens the episode's defer
    /// level and `insert` takes the deferred path above: nothing moves storage
    /// while the guard pins it, the guard's data stays valid across the call,
    /// and the bundle (with its `OnAdd` / `OnSet` events) applies when the last
    /// guard drops.
    ///
    /// The bundle's values are **moved** into storage; they are not dropped on
    /// the Rust side.
    ///
    /// # Panics
    ///
    /// Panics if the bundle contains a duplicate component type, if the bundle
    /// arity exceeds 31, or if the entity is not alive.
    fn insert<B: Bundle>(self, bundle: B) -> Self;
}

impl<'a> EntityBundleExt for EntityView<'a> {
    #[track_caller]
    fn insert<B: Bundle>(self, bundle: B) -> Self {
        const {
            assert!(
                B::ARITY <= MAX_BUNDLE_ARITY,
                "bundle arity exceeds the maximum of 31 components"
            );
        }

        let world = self.world;
        let world_ptr = world.world_ptr_mut();

        let arity = B::ARITY;
        let mut ids = [0u64; ID_BUF];
        B::resolve_ids(world, &mut ids[..arity]);
        record_bundle_ids::<B>(&world, &ids[..arity]);

        // Shared-register write hook (spec §3.6): with a live guard on this
        // stage this lazily opens the episode's defer level, so the branch below
        // routes to the deferred path and no table move can reallocate storage
        // out from under the guard. With no live guard it is a no-op.
        crate::core::ensure_write_episode(&world);

        if world.is_deferred() {
            // `ecs_commit` cannot run while deferred; apply per component and let
            // flecs merge at the enclosing sync point (see doc note above).
            bundle.apply_set(self);
            return self;
        }

        // SAFETY: the entity is alive on an immediate world; `ecs_commit`
        // performs the whole add as one structural move. `added`/`added_mask`
        // describe exactly the newly-added ids so `OnAdd` fires once each and so
        // value writes know which slots are freshly constructed.
        unsafe {
            let record = sys::ecs_record_find(world_ptr, *self.id);
            assert!(!record.is_null(), "insert on an entity that is not alive");
            let src_table = (*record).table;

            let mut dst_table = src_table;
            for &id in &ids[..arity] {
                dst_table = sys::ecs_table_add_id(world_ptr, dst_table, id);
            }

            let src_type = &*sys::ecs_table_get_type(src_table);
            let src_ids: &[u64] = if src_type.count > 0 {
                core::slice::from_raw_parts(src_type.array, src_type.count as usize)
            } else {
                &[]
            };

            let mut added = [0u64; ID_BUF];
            let mut added_mask = [false; ID_BUF];
            let mut added_count = 0usize;
            for i in 0..arity {
                let is_new = !src_ids.contains(&ids[i]);
                added_mask[i] = is_new;
                if is_new {
                    added[added_count] = ids[i];
                    added_count += 1;
                }
            }

            let added_type = sys::ecs_type_t {
                array: added.as_mut_ptr(),
                count: added_count as i32,
            };

            // One structural move; ctors new columns and fires OnAdd.
            sys::ecs_commit(
                world_ptr,
                *self.id,
                record,
                dst_table,
                &added_type,
                core::ptr::null(),
            );

            bundle.write_after_commit(self, &added_mask[..arity]);
        }

        self
    }
}
