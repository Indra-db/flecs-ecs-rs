use super::{FlecsArray, FlecsIdMap, World};
use crate::sys;

use core::cell::Cell;

#[cfg(feature = "std")]
extern crate std;

extern crate alloc;
use alloc::boxed::Box;
use alloc::sync::Arc;
use alloc::vec;
use core::any::Any;
use std::sync::Mutex;
use std::sync::PoisonError;

/// Per-world cache mapping a [`Bundle`](crate::experimental::Bundle) type to the
/// sorted, de-duplicated component id array used to resolve its target table.
///
/// Keyed by the bundle's `TypeId` (the §8.2 per-world identity machinery). The
/// stored value is the SORTED id array, never a table pointer: tables can be
/// deleted, component ids cannot.
#[cfg(feature = "flecs_experimental")]
pub(crate) type BundleIdCache = core::cell::RefCell<
    hashbrown::HashMap<core::any::TypeId, alloc::boxed::Box<[u64]>, super::NoOpHash>,
>;

/// Payload of a panic caught at a callback trampoline (spec §5.5, Decision 8),
/// held until the enclosing safe entry point rethrows it. `first` keeps the
/// payload of the first panic in the frame (that one is rethrown); `suppressed`
/// counts later panics in the same frame whose payloads are dropped.
struct PanicStash {
    first: Option<Box<dyn Any + Send>>,
    suppressed: u32,
}

pub(crate) struct WorldCtx {
    query_ref_count: Cell<i32>,
    pub(crate) components: FlecsIdMap,
    pub(crate) components_array: FlecsArray,
    // Bundle-type -> sorted component id array (spec §4.11).
    #[cfg(feature = "flecs_experimental")]
    pub(crate) bundle_ids: BundleIdCache,
    // Atomic because `QueryHandle::drop` reads it from other threads.
    is_panicking: core::sync::atomic::AtomicBool,
    // Set once a callback trampoline has caught and stashed a panic for the
    // current frame (spec §5.5). Read at every trampoline entry (short-circuit
    // the rest of the poisoned frame) and at every safe entry point (rethrow).
    // A worker thread in a `par_*` system writes it under `panic_stash`; the
    // trampoline read is one relaxed load on the non-panic hot path.
    frame_panicked: core::sync::atomic::AtomicBool,
    // The caught payload. Locked only on the panic path (a trampoline catching,
    // or a safe entry point rethrowing), never during normal iteration, so the
    // one lock this wave introduces stays off every hot path. The `Mutex`
    // serialises concurrent stashes from `par_*` worker threads.
    panic_stash: Mutex<PanicStash>,
    owning_thread: std::thread::ThreadId,
    // Shared with every `QueryHandle`. `true` once world teardown has begun;
    // a handle dropping on another thread takes the lock so its refcount
    // release can never interleave with `ecs_fini` freeing query memory.
    world_dead: Arc<Mutex<bool>>,
    // Per-stage mut-alias counters; stage count kept in sync by the
    // set_threads/set_stage_count wrappers.
    #[cfg(feature = "flecs_safety_locks")]
    pub(crate) safety_locks: crate::core::SafetyLocks,
    // Typed world context (spec §5.3): the capture-based replacement for the
    // `*mut c_void` `set_context`. Owned by the world and dropped with it.
    pub(crate) typed_ctx: Option<alloc::boxed::Box<dyn core::any::Any>>,
}

impl WorldCtx {
    pub(crate) fn new() -> Self {
        Self {
            query_ref_count: Cell::new(0),
            components: Default::default(),
            components_array: vec![0; 500],
            #[cfg(feature = "flecs_experimental")]
            bundle_ids: core::cell::RefCell::new(hashbrown::HashMap::default()),
            is_panicking: core::sync::atomic::AtomicBool::new(false),
            frame_panicked: core::sync::atomic::AtomicBool::new(false),
            panic_stash: Mutex::new(PanicStash {
                first: None,
                suppressed: 0,
            }),
            owning_thread: std::thread::current().id(),
            world_dead: Arc::new(Mutex::new(false)),
            #[cfg(feature = "flecs_safety_locks")]
            safety_locks: crate::core::SafetyLocks::new(),
            typed_ctx: None,
        }
    }

    pub(crate) fn world_dead_lock(&self) -> &Arc<Mutex<bool>> {
        &self.world_dead
    }

    pub(crate) fn mark_world_dead(&self) {
        let mut dead = self
            .world_dead
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        *dead = true;
    }

    pub(crate) fn owning_thread(&self) -> std::thread::ThreadId {
        self.owning_thread
    }

    pub(crate) fn inc_query_ref_count(&self) {
        unsafe {
            if sys::ecs_os_has_threading() {
                if let Some(ainc) = sys::ecs_os_api.ainc_ {
                    ainc(self.query_ref_count.as_ptr());
                }
            } else {
                self.query_ref_count.set(self.query_ref_count.get() + 1);
            }
        }
    }

    pub(crate) fn dec_query_ref_count(&self) {
        unsafe {
            if sys::ecs_os_has_threading() {
                if let Some(adec) = sys::ecs_os_api.adec_ {
                    adec(self.query_ref_count.as_ptr());
                }
            } else {
                self.query_ref_count.set(self.query_ref_count.get() - 1);
            }
        }
    }

    #[allow(dead_code)] //used in tests
    pub(crate) fn query_ref_count(&self) -> i32 {
        self.query_ref_count.get()
    }

    pub(crate) fn is_ref_count_zero(&self) -> bool {
        self.query_ref_count.get() == 0
    }

    pub(crate) fn set_is_panicking_true(&self) {
        self.is_panicking
            .store(true, core::sync::atomic::Ordering::Relaxed);
    }

    pub(crate) fn is_panicking(&self) -> bool {
        self.is_panicking
            .load(core::sync::atomic::Ordering::Relaxed)
            || std::thread::panicking()
    }

    /// Whether a callback panic has been caught for the current frame and is
    /// waiting to be rethrown. One relaxed load; the trampolines call it at
    /// entry to skip the rest of a poisoned frame and the safe entry points
    /// call it before deciding to rethrow.
    #[inline(always)]
    pub(crate) fn frame_panicked(&self) -> bool {
        self.frame_panicked
            .load(core::sync::atomic::Ordering::Relaxed)
    }

    /// Stash a panic payload caught at a callback trampoline. The first payload
    /// of the frame is kept for rethrow; any later panic in the same frame only
    /// bumps the suppressed count (its payload is dropped here). Safe to call
    /// from a `par_*` worker thread: the `Mutex` serialises concurrent stashes.
    pub(crate) fn stash_panic(&self, payload: Box<dyn Any + Send>) {
        {
            let mut stash = self
                .panic_stash
                .lock()
                .unwrap_or_else(PoisonError::into_inner);
            if stash.first.is_none() {
                stash.first = Some(payload);
            } else {
                stash.suppressed += 1;
            }
        }
        self.frame_panicked
            .store(true, core::sync::atomic::Ordering::Relaxed);
    }

    /// Take the stashed panic, clearing the frame-panicked flag. Returns the
    /// first payload and the number of additional same-frame panics that were
    /// suppressed. Called from a safe entry point after flecs has returned.
    pub(crate) fn take_stashed_panic(&self) -> Option<(Box<dyn Any + Send>, u32)> {
        if !self.frame_panicked() {
            return None;
        }
        self.frame_panicked
            .store(false, core::sync::atomic::Ordering::Relaxed);
        let mut stash = self
            .panic_stash
            .lock()
            .unwrap_or_else(PoisonError::into_inner);
        let suppressed = core::mem::take(&mut stash.suppressed);
        stash.first.take().map(|payload| (payload, suppressed))
    }
}

/// Rethrow a panic stashed by a callback trampoline, if any (spec §5.5).
///
/// Called from every safe entry point that drives flecs (`progress`,
/// `run_pipeline`, `System::run_with`, event emit, query iteration terminals):
/// after flecs has returned and unwound its own frames normally, the original
/// panic resumes here so it surfaces to the caller as an ordinary Rust panic,
/// preserving its payload (and message) for `should_panic` / `catch_unwind`.
/// On the non-panic path this is a single relaxed load and return.
#[inline(always)]
pub(crate) fn rethrow_stashed_panic(ctx: &WorldCtx) {
    if let Some((payload, suppressed)) = ctx.take_stashed_panic() {
        resume_stashed_panic(payload, suppressed);
    }
}

/// Resume a taken panic payload, reporting any suppressed same-frame panics.
#[allow(clippy::print_stderr, reason = "reporting dropped same-frame panics")]
pub(crate) fn resume_stashed_panic(payload: Box<dyn Any + Send>, suppressed: u32) -> ! {
    if suppressed > 0 {
        std::eprintln!(
            "flecs_ecs: {suppressed} further panic(s) in the same frame were suppressed; \
             rethrowing the first"
        );
    }
    std::panic::resume_unwind(payload);
}

/// Calls `defer_begin` on construction and `defer_end` on drop, so the defer
/// block is closed even when the user callback unwinds.
pub(crate) struct DeferGuard<'w> {
    world: super::WorldRef<'w>,
}

impl<'w> DeferGuard<'w> {
    pub(crate) fn new(world: super::WorldRef<'w>) -> Self {
        world.defer_begin();
        Self { world }
    }
}

impl Drop for DeferGuard<'_> {
    fn drop(&mut self) {
        self.world.defer_end();
    }
}

/// Exit half of a component access scope opened by
/// `sys::ecs_rust_get_scope_begin` (which combines the entity-record lookup
/// with `defer_begin`). Dropping ends the defer scope, including on unwind
/// from the user callback or a borrow-violation panic.
pub(crate) struct ScopeEndGuard<'w> {
    pub(crate) world: super::WorldRef<'w>,
}

impl Drop for ScopeEndGuard<'_> {
    fn drop(&mut self) {
        unsafe { sys::ecs_rust_scope_end(self.world.raw_world.as_ptr()) };
    }
}

impl World {
    pub(crate) fn world_ctx(&self) -> &WorldCtx {
        unsafe { &*(sys::ecs_get_binding_ctx(self.raw_world.as_ptr()) as *const WorldCtx) }
    }

    /// Resume a panic stashed by a callback trampoline during the flecs call
    /// that just returned (spec §5.5). No-op when no panic was caught.
    #[inline(always)]
    pub(crate) fn rethrow_stashed_panic(&self) {
        rethrow_stashed_panic(self.world_ctx());
    }

    // XAI: thread-affinity model. `World`/`WorldRef` are !Send, so all safe
    // handles stay on the thread that created the world (`owning_thread`).
    // Component data can still reach worker threads through two doors:
    // 1. `par_*` systems — guarded statically (`TupleType: Send` bounds on
    //    the par registration methods; `Query` itself is `!Send`/`!Sync`).
    // 2. Views handed to par callbacks (`EntityView`, `TableIter`,
    //    `WorldRef::from_ptr` in trampolines) — guarded by the runtime checks
    //    below at every typed materialization/move choke point.
    // This relies on the flecs C scheduler invariant that non-multi_threaded
    // systems only execute on the thread calling progress() (flecs.c
    // flecs_run_pipeline_ops: assert(!stage_index || op->multi_threaded)).
    // Re-verify on every vendored flecs C upgrade.

    /// Asserts that a shared reference (`&T`) to component data may be
    /// materialized on the current thread. Compiles to nothing for `Sync`
    /// component types.
    #[inline(always)]
    pub(crate) fn check_thread_affinity_shared<T: crate::core::ComponentInfo>(&self) {
        if !T::IMPLS_SYNC {
            self.assert_owning_thread::<T>();
        }
    }

    /// Asserts that an exclusive reference (`&mut T`) to component data may be
    /// materialized, or a `T` value moved in/out of storage, on the current
    /// thread. Compiles to nothing for `Send` component types.
    #[inline(always)]
    pub(crate) fn check_thread_affinity_exclusive<T: crate::core::ComponentInfo>(&self) {
        if !T::IMPLS_SEND {
            self.assert_owning_thread::<T>();
        }
    }

    #[inline(always)]
    fn assert_owning_thread<T>(&self) {
        if std::thread::current().id() != self.world_ctx().owning_thread() {
            thread_affinity_violation(core::any::type_name::<T>());
        }
    }
}

#[cold]
#[inline(never)]
fn thread_affinity_violation(type_name: &str) -> ! {
    panic!(
        "component `{type_name}` is thread-bound (!Send or !Sync) and can only be accessed from the thread that owns the world"
    );
}

/// Mirrors the C-side `ecs_assert` in `flecs_new_id`, which is compiled out in
/// release builds (`NDEBUG`): entity id allocation mutates the shared entity
/// index without synchronization, so creating entities during the
/// multithreaded pipeline phase (e.g. from a `par_*` system callback) would
/// race.
#[inline(always)]
pub(crate) fn assert_not_in_multithreaded_phase(world_ptr: *const sys::ecs_world_t) {
    if unsafe { sys::ecs_world_get_flags(world_ptr) & sys::EcsWorldMultiThreaded != 0 } {
        multithreaded_entity_creation_violation();
    }
}

#[cold]
#[inline(never)]
fn multithreaded_entity_creation_violation() -> ! {
    panic!(
        "entities cannot be created while the world is in its multithreaded execution phase; create them before or after progress(), or from a single-threaded system"
    );
}

#[test]
fn query_ref_count() {
    unsafe {
        flecs_ecs::sys::ecs_os_init();
    }
    use flecs_ecs::core::*;
    use flecs_ecs::macros::*;

    #[derive(Component)]
    struct Tag;

    let world = World::new();
    let query = world.query::<()>().with(Tag).build();

    assert_eq!(world.world_ctx().query_ref_count(), 1);
    assert_eq!(query.reference_count(), 1);

    let query2 = query.clone();

    assert_eq!(world.world_ctx().query_ref_count(), 2);
    assert_eq!(query.reference_count(), 2);

    drop(query);

    assert_eq!(world.world_ctx().query_ref_count(), 1);
    assert_eq!(query2.reference_count(), 1);

    drop(query2);

    assert_eq!(world.world_ctx().query_ref_count(), 0);
}
