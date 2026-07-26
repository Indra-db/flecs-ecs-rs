use super::{FlecsArray, FlecsIdMap, NoOpHash, World};
use crate::sys;

use core::any::TypeId;
use core::cell::{Cell, RefCell};

#[cfg(feature = "std")]
extern crate std;

extern crate alloc;
use alloc::boxed::Box;
use alloc::sync::Arc;
use alloc::vec;
use std::sync::Mutex;

/// Per-world cache mapping a [`Bundle`](crate::experimental::Bundle) type to the
/// sorted, de-duplicated component id array used to resolve its target table.
///
/// Keyed by the bundle's `TypeId` (the §8.2 per-world identity machinery). The
/// stored value is the SORTED id array, never a table pointer: tables can be
/// deleted, component ids cannot.
pub(crate) type BundleIdCache = RefCell<hashbrown::HashMap<TypeId, Box<[u64]>, NoOpHash>>;

pub(crate) struct WorldCtx {
    query_ref_count: Cell<i32>,
    pub(crate) components: FlecsIdMap,
    pub(crate) components_array: FlecsArray,
    pub(crate) bundle_ids: BundleIdCache,
    // Atomic because `QueryHandle::drop` reads it from other threads.
    is_panicking: core::sync::atomic::AtomicBool,
    owning_thread: std::thread::ThreadId,
    // Shared with every `QueryHandle`. `true` once world teardown has begun;
    // a handle dropping on another thread takes the lock so its refcount
    // release can never interleave with `ecs_fini` freeing query memory.
    world_dead: Arc<Mutex<bool>>,
    // Per-stage mut-alias counters; stage count kept in sync by the
    // set_threads/set_stage_count wrappers.
    #[cfg(feature = "flecs_safety_locks")]
    pub(crate) safety_locks: crate::core::SafetyLocks,
}

impl WorldCtx {
    pub(crate) fn new() -> Self {
        Self {
            query_ref_count: Cell::new(0),
            components: Default::default(),
            components_array: vec![0; 500],
            bundle_ids: RefCell::new(hashbrown::HashMap::default()),
            is_panicking: core::sync::atomic::AtomicBool::new(false),
            owning_thread: std::thread::current().id(),
            world_dead: Arc::new(Mutex::new(false)),
            #[cfg(feature = "flecs_safety_locks")]
            safety_locks: crate::core::SafetyLocks::new(),
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
