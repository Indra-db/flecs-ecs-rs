//! Runtime mutable-alias detection ("safety locks").
//!
//! All bookkeeping lives on the Rust side: every world owns one [`StageLocks`]
//! counter map per stage. A stage's map is only ever touched by the thread
//! that owns that stage (flecs pipeline invariant: a multithreaded system's
//! workers each run on their own stage, single-threaded systems run on stage
//! 0), so the maps need no atomics and worker threads never contend or
//! false-share.
//!
//! Lock keys identify the storage region a component pointer originates from:
//! a `(table, column)` pair for dense components, the component id for
//! sparse / non-fragmenting components. Sparse tracking is per stage like
//! dense tracking, so two workers accessing the same sparse component on
//! disjoint entities do not report a spurious conflict.

#[cfg(feature = "flecs_safety_locks")]
use super::WorldRef;
#[cfg(feature = "flecs_safety_locks")]
use crate::core::QueryTuple;
#[cfg(feature = "flecs_safety_locks")]
use crate::core::{IdOperations, IdView};
#[cfg(feature = "flecs_safety_locks")]
use flecs_ecs::sys;

#[cfg(feature = "flecs_safety_locks")]
use core::cell::UnsafeCell;
#[cfg(feature = "flecs_safety_locks")]
use core::ptr::NonNull;

#[cfg(feature = "flecs_safety_locks")]
extern crate alloc;
#[cfg(feature = "flecs_safety_locks")]
use alloc::vec;
#[cfg(feature = "flecs_safety_locks")]
use alloc::vec::Vec;

/// Reserve the highest bit as the write flag.
#[cfg(feature = "flecs_safety_locks")]
const WRITE_FLAG: u16 = 1 << 15;
/// The remaining bits hold the read count.
#[cfg(feature = "flecs_safety_locks")]
const READ_MASK: u16 = WRITE_FLAG - 1;

/// Identifies the storage region a component pointer originates from.
/// Dense: `(table ptr << 16) | column`. Sparse: `(1 << 127) | component
/// record ptr`. Pointer-based keys need no FFI to derive and never collide
/// across the two shapes thanks to the tag bit; both pointers are stable for
/// the world's lifetime and the maps are world-owned. Entries are balanced
/// begin/end pairs, so no key outlives the storage it names.
#[cfg(feature = "flecs_safety_locks")]
pub(crate) type LockKey = u128;

#[cfg(feature = "flecs_safety_locks")]
#[inline(always)]
pub(crate) fn dense_lock_key(table: *mut sys::ecs_table_t, column: i16) -> LockKey {
    ((table as usize as u128) << 16) | (column as u16 as u128)
}

#[cfg(feature = "flecs_safety_locks")]
#[inline(always)]
pub(crate) fn sparse_lock_key(cr: *mut sys::ecs_component_record_t) -> LockKey {
    (1u128 << 127) | (cr as usize as u128)
}

/// Read/write counters for one stage. Not thread-safe by design: each stage
/// is owned by exactly one thread while systems run.
///
/// Entries are held in a linear-scanned vec rather than a hash map: the live
/// set is bounded by actual borrow nesting depth (query terms plus nested
/// accesses), which stays in the single digits in practice, and a cache-hot
/// scan of a few 24-byte entries beats hashing. Capacity is retained across
/// uses, so steady state performs no allocation.
#[cfg(feature = "flecs_safety_locks")]
#[derive(Default)]
pub(crate) struct StageLocks {
    entries: Vec<(LockKey, u16)>,
}

#[cfg(feature = "flecs_safety_locks")]
impl StageLocks {
    #[inline(always)]
    fn find(&mut self, key: LockKey) -> Option<&mut (LockKey, u16)> {
        self.entries.iter_mut().find(|entry| entry.0 == key)
    }

    /// Returns `true` on violation (a write is already held). On violation no
    /// read is registered.
    #[inline(always)]
    fn read_begin(&mut self, key: LockKey) -> bool {
        if let Some(entry) = self.find(key) {
            if entry.1 & WRITE_FLAG != 0 {
                return true;
            }
            entry.1 += 1;
        } else {
            self.entries.push((key, 1));
        }
        false
    }

    #[inline(always)]
    fn read_end(&mut self, key: LockKey) {
        if let Some(index) = self.entries.iter().position(|entry| entry.0 == key) {
            let entry = &mut self.entries[index];
            debug_assert!(entry.1 & READ_MASK != 0, "unbalanced read_end");
            entry.1 -= 1;
            if entry.1 == 0 {
                self.entries.swap_remove(index);
            }
        }
    }

    /// Returns `true` on violation (a read or write is already held). On
    /// violation no write is registered.
    #[inline(always)]
    fn write_begin(&mut self, key: LockKey) -> bool {
        if self.find(key).is_some() {
            return true;
        }
        self.entries.push((key, WRITE_FLAG));
        false
    }

    #[inline(always)]
    fn write_end(&mut self, key: LockKey) {
        if let Some(index) = self.entries.iter().position(|entry| entry.0 == key) {
            let entry = &mut self.entries[index];
            debug_assert!(entry.1 & WRITE_FLAG != 0, "unbalanced write_end");
            entry.1 &= !WRITE_FLAG;
            if entry.1 == 0 {
                self.entries.swap_remove(index);
            }
        }
    }
}

/// Per-world container of one [`StageLocks`] per stage, stored in
/// [`WorldCtx`](crate::core::world_ctx::WorldCtx).
///
/// Safety model: `stage()` hands out a raw pointer to a stage's map. A stage
/// map is only mutated from the thread owning that stage, and the stage vec
/// is only resized while the world is single-threaded (no stage thread
/// running, no lock guard alive), mirroring the C-side contract of
/// `ecs_set_stage_count`.
#[cfg(feature = "flecs_safety_locks")]
pub(crate) struct SafetyLocks {
    stages: UnsafeCell<Vec<UnsafeCell<StageLocks>>>,
}

#[cfg(feature = "flecs_safety_locks")]
impl SafetyLocks {
    pub(crate) fn new() -> Self {
        Self {
            stages: UnsafeCell::new(vec![UnsafeCell::default()]),
        }
    }

    /// Must only be called while the world is single-threaded and no lock
    /// guard is alive (the C `ecs_set_stage_count` / `ecs_set_threads`
    /// contract): resizing may move the stage maps.
    pub(crate) fn set_stage_count(&self, count: i32) {
        invalidate_stage_locks_cache();
        let count = count.max(1) as usize;
        // SAFETY: single-threaded per the function contract; no `stage()`
        // pointer is alive because no lock guard exists outside iteration.
        let stages = unsafe { &mut *self.stages.get() };
        stages.resize_with(count, UnsafeCell::default);
    }

    #[inline(always)]
    pub(crate) fn stage(&self, index: i32) -> NonNull<StageLocks> {
        // SAFETY: the vec is only resized single-threaded; concurrent readers
        // (worker threads resolving their own stage) see a stable vec.
        let stages = unsafe { &*self.stages.get() };
        let cell: &UnsafeCell<StageLocks> = stages.get(index as usize).expect(
            "stage count changed without going through World::set_threads/set_stage_count/set_task_threads",
        );
        // SAFETY: never null, points into the live vec buffer.
        unsafe { NonNull::new_unchecked(cell.get()) }
    }
}

// One-slot cache for the single-threaded stage-map resolve: (world ptr,
// StageLocks ptr). Sound because the safe API confines component access to
// the world-owning thread plus flecs workers (joined before world teardown),
// and the slot is cleared whenever a world is dropped on this thread or a
// stage-map vec is resized (both of which can invalidate the cached pointer).
#[cfg(feature = "flecs_safety_locks")]
std::thread_local! {
    static STAGE_LOCKS_CACHE: core::cell::Cell<(usize, usize)> =
        const { core::cell::Cell::new((0, 0)) };
}

#[cfg(feature = "flecs_safety_locks")]
pub(crate) fn invalidate_stage_locks_cache() {
    STAGE_LOCKS_CACHE.with(|cache| cache.set((0, 0)));
}

#[cfg(feature = "flecs_safety_locks")]
#[inline(never)]
#[cold]
fn stage_locks_resolve(world: &WorldRef, stage_index: i32) -> NonNull<StageLocks> {
    // ecs_get_binding_ctx resolves stage pointers to the world itself, so no
    // separate real_world() round-trip is needed.
    let ctx = unsafe {
        &*(sys::ecs_get_binding_ctx(world.raw_world.as_ptr())
            as *const crate::core::world_ctx::WorldCtx)
    };
    ctx.safety_locks.stage(stage_index)
}

/// Resolve the current calling context's stage lock map.
///
/// `world` must be the world pointer of the calling context: inside a
/// multithreaded system callback that is the iterator's stage world, which
/// maps each worker to its own stage map.
#[cfg(feature = "flecs_safety_locks")]
#[inline(always)]
pub(crate) fn stage_locks<const MULTITHREADED: bool>(world: &WorldRef) -> NonNull<StageLocks> {
    if MULTITHREADED {
        // resolved per batch, amortized; workers each hit their own stage
        return stage_locks_resolve(world, world.stage_id());
    }
    let key = world.raw_world.as_ptr() as usize;
    STAGE_LOCKS_CACHE.with(|cache| {
        let (cached_key, cached_locks) = cache.get();
        if cached_key == key {
            // SAFETY: slot is cleared on world drop and stage resize, so a
            // hit means the pointer is still live.
            return unsafe { NonNull::new_unchecked(cached_locks as *mut StageLocks) };
        }
        let locks = stage_locks_resolve(world, 0);
        cache.set((key, locks.as_ptr() as usize));
        locks
    })
}

/// [`stage_locks`] with the multithreaded branch resolved at runtime.
#[cfg(feature = "flecs_safety_locks")]
#[inline(always)]
pub(crate) fn stage_locks_dyn(world: &WorldRef) -> NonNull<StageLocks> {
    if world.is_currently_multithreaded() {
        stage_locks::<true>(world)
    } else {
        stage_locks::<false>(world)
    }
}

#[cfg(feature = "flecs_safety_locks")]
#[cold]
#[inline(never)]
fn alias_violation_panic(world: &WorldRef, component_id: u64, write: bool) -> ! {
    let component = {
        let id = IdView::new_from_id(world, component_id);
        if id.is_pair() {
            format!(
                "({}, {})",
                world.entity_from_id(id.first_id()),
                world.entity_from_id(id.second_id())
            )
        } else {
            format!("{}", id.entity_view())
        }
    };
    if write {
        panic!(
            "Cannot set write: reads already present or write already set for component: {component}"
        );
    } else {
        panic!("Cannot increment read: write already set for component: {component}");
    }
}

#[cfg(feature = "flecs_safety_locks")]
#[inline(always)]
fn dense_component_id(table: *mut sys::ecs_table_t, column: i16) -> u64 {
    unsafe {
        *(*sys::ecs_table_get_type(table))
            .array
            .add(sys::ecs_table_column_to_type_index(table, column as i32) as usize)
    }
}

#[cfg(feature = "flecs_safety_locks")]
#[inline(always)]
fn sparse_component_id(cr: *mut sys::ecs_component_record_t) -> u64 {
    unsafe { sys::flecs_component_get_id(cr) }
}

#[cfg(feature = "flecs_safety_locks")]
pub(super) const INCREMENT: bool = true;
#[cfg(feature = "flecs_safety_locks")]
pub(super) const DECREMENT: bool = false;

#[cfg(feature = "flecs_safety_locks")]
#[inline(always)]
pub(crate) fn sparse_id_record_lock_read_begin(
    world: &WorldRef,
    locks: NonNull<StageLocks>,
    cr: *mut sys::ecs_component_record_t,
) {
    if unsafe { (*locks.as_ptr()).read_begin(sparse_lock_key(cr)) } {
        alias_violation_panic(world, sparse_component_id(cr), false);
    }
}

#[cfg(feature = "flecs_safety_locks")]
#[inline(always)]
pub(crate) fn sparse_id_record_lock_read_end(
    locks: NonNull<StageLocks>,
    cr: *mut sys::ecs_component_record_t,
) {
    unsafe { (*locks.as_ptr()).read_end(sparse_lock_key(cr)) }
}

#[cfg(feature = "flecs_safety_locks")]
#[inline(always)]
pub(crate) fn sparse_id_record_lock_write_begin(
    world: &WorldRef,
    locks: NonNull<StageLocks>,
    cr: *mut sys::ecs_component_record_t,
) {
    if unsafe { (*locks.as_ptr()).write_begin(sparse_lock_key(cr)) } {
        alias_violation_panic(world, sparse_component_id(cr), true);
    }
}

#[cfg(feature = "flecs_safety_locks")]
#[inline(always)]
pub(crate) fn sparse_id_record_lock_write_end(
    locks: NonNull<StageLocks>,
    cr: *mut sys::ecs_component_record_t,
) {
    unsafe { (*locks.as_ptr()).write_end(sparse_lock_key(cr)) }
}

/// Panicking read lock on a dense table column.
#[cfg(feature = "flecs_safety_locks")]
#[inline(always)]
pub(crate) fn get_table_column_lock_read_begin(
    world: &WorldRef,
    locks: NonNull<StageLocks>,
    table: *mut sys::ecs_table_t,
    column: i16,
) {
    if unsafe { (*locks.as_ptr()).read_begin(dense_lock_key(table, column)) } {
        alias_violation_panic(world, dense_component_id(table, column), false);
    }
}

/// Non-panicking read lock attempt. Returns `true` when a write is already
/// held, in which case no read lock was registered.
#[cfg(feature = "flecs_safety_locks")]
#[inline(always)]
pub(crate) fn table_column_lock_read_begin(
    locks: NonNull<StageLocks>,
    table: *mut sys::ecs_table_t,
    column: i16,
) -> bool {
    unsafe { (*locks.as_ptr()).read_begin(dense_lock_key(table, column)) }
}

#[cfg(feature = "flecs_safety_locks")]
#[inline(always)]
pub(crate) fn table_column_lock_read_end(
    locks: NonNull<StageLocks>,
    table: *mut sys::ecs_table_t,
    column: i16,
) {
    unsafe { (*locks.as_ptr()).read_end(dense_lock_key(table, column)) }
}

/// Panicking write lock on a dense table column.
#[cfg(feature = "flecs_safety_locks")]
#[inline(always)]
pub(crate) fn get_table_column_lock_write_begin(
    world: &WorldRef,
    locks: NonNull<StageLocks>,
    table: *mut sys::ecs_table_t,
    column: i16,
) {
    if unsafe { (*locks.as_ptr()).write_begin(dense_lock_key(table, column)) } {
        alias_violation_panic(world, dense_component_id(table, column), true);
    }
}

/// Non-panicking write lock attempt. Returns `true` when a read or write is
/// already held, in which case no write lock was registered.
#[cfg(feature = "flecs_safety_locks")]
#[inline(always)]
pub(crate) fn table_column_lock_write_begin(
    locks: NonNull<StageLocks>,
    table: *mut sys::ecs_table_t,
    column: i16,
) -> bool {
    unsafe { (*locks.as_ptr()).write_begin(dense_lock_key(table, column)) }
}

#[cfg(feature = "flecs_safety_locks")]
#[inline(always)]
pub(crate) fn table_column_lock_write_end(
    locks: NonNull<StageLocks>,
    table: *mut sys::ecs_table_t,
    column: i16,
) {
    unsafe { (*locks.as_ptr()).write_end(dense_lock_key(table, column)) }
}

#[inline]
#[cfg(feature = "flecs_safety_locks")]
pub(crate) fn do_read_write_locks<
    const INCREMENT: bool,
    const ANY_SPARSE_TERMS: bool,
    T: QueryTuple,
>(
    world: &WorldRef,
    table_records: &[super::TableColumnSafety],
) {
    if world.is_currently_multithreaded() {
        __internal_do_read_write_locks::<INCREMENT, true, ANY_SPARSE_TERMS, T>(
            world,
            table_records,
        );
    } else {
        __internal_do_read_write_locks::<INCREMENT, false, ANY_SPARSE_TERMS, T>(
            world,
            table_records,
        );
    }
}

#[cfg(feature = "flecs_safety_locks")]
#[inline(always)]
fn __internal_do_read_write_locks<
    const INCREMENT: bool,
    const MULTITHREADED: bool,
    const ANY_SPARSE_TERMS: bool,
    T: QueryTuple,
>(
    world: &WorldRef<'_>,
    table_records: &[super::TableColumnSafety],
) {
    let count_immutable: usize = const { T::COUNT_IMMUTABLE };
    let start_index_mutable: usize = const { T::COUNT_IMMUTABLE };
    let start_index_optional_immutable: usize = const { T::COUNT_IMMUTABLE + T::COUNT_MUTABLE };
    let start_index_optional_mutable: usize =
        const { T::COUNT_IMMUTABLE + T::COUNT_MUTABLE + T::COUNT_OPTIONAL_IMMUTABLE };
    let end_index_mutable: usize = const { T::COUNT_IMMUTABLE + T::COUNT_MUTABLE };
    let end_index_optional_immutable: usize =
        const { T::COUNT_IMMUTABLE + T::COUNT_MUTABLE + T::COUNT_OPTIONAL_IMMUTABLE };
    let end_index_optional_mutable: usize = const {
        T::COUNT_IMMUTABLE
            + T::COUNT_MUTABLE
            + T::COUNT_OPTIONAL_IMMUTABLE
            + T::COUNT_OPTIONAL_MUTABLE
    };

    let locks = stage_locks::<MULTITHREADED>(world);

    #[inline(always)]
    fn term_lock<const INCREMENT: bool, const READONLY: bool>(
        world: &WorldRef<'_>,
        locks: NonNull<StageLocks>,
        info: &super::TableColumnSafety,
        any_sparse_terms: bool,
    ) {
        // if component_id is set, this term is a row (sparse) term
        if any_sparse_terms && info.component_id != 0 {
            // batch-level resolve; keys must match the cr-keyed FieldAt /
            // rw_locking sparse paths, which never pay this lookup per row.
            // flecs_components_get requires the real world, not a stage.
            let cr = unsafe {
                let real_world = sys::ecs_get_world(world.raw_world.as_ptr() as *const _);
                sys::flecs_components_get(real_world, info.component_id)
            };
            let key = sparse_lock_key(cr);
            let map = unsafe { &mut *locks.as_ptr() };
            if READONLY {
                if INCREMENT {
                    if map.read_begin(key) {
                        alias_violation_panic(world, info.component_id, false);
                    }
                } else {
                    map.read_end(key);
                }
            } else if INCREMENT {
                if map.write_begin(key) {
                    alias_violation_panic(world, info.component_id, true);
                }
            } else {
                map.write_end(key);
            }
            return;
        }

        if info.table.is_null() {
            return;
        }

        if READONLY {
            if INCREMENT {
                get_table_column_lock_read_begin(world, locks, info.table, info.column);
            } else {
                table_column_lock_read_end(locks, info.table, info.column);
            }
        } else if INCREMENT {
            get_table_column_lock_write_begin(world, locks, info.table, info.column);
        } else {
            table_column_lock_write_end(locks, info.table, info.column);
        }
    }

    unsafe {
        for i in 0..count_immutable {
            let info = table_records.get_unchecked(i);
            term_lock::<INCREMENT, true>(world, locks, info, ANY_SPARSE_TERMS);
        }
        for i in start_index_mutable..end_index_mutable {
            let info = table_records.get_unchecked(i);
            term_lock::<INCREMENT, false>(world, locks, info, ANY_SPARSE_TERMS);
        }
        for i in start_index_optional_immutable..end_index_optional_immutable {
            let info = table_records.get_unchecked(i);
            term_lock::<INCREMENT, true>(world, locks, info, ANY_SPARSE_TERMS);
        }
        for i in start_index_optional_mutable..end_index_optional_mutable {
            let info = table_records.get_unchecked(i);
            term_lock::<INCREMENT, false>(world, locks, info, ANY_SPARSE_TERMS);
        }
    }
}

#[cfg(all(test, feature = "flecs_safety_locks"))]
mod tests {
    use super::*;

    #[test]
    fn stage_locks_read_write_protocol() {
        let mut locks = StageLocks::default();
        let key = 42u128;
        assert!(!locks.read_begin(key));
        assert!(!locks.read_begin(key));
        assert!(locks.write_begin(key), "write while reads held must fail");
        locks.read_end(key);
        locks.read_end(key);
        assert!(!locks.write_begin(key));
        assert!(locks.read_begin(key), "read while write held must fail");
        assert!(locks.write_begin(key), "second write must fail");
        locks.write_end(key);
        assert!(!locks.read_begin(key));
        locks.read_end(key);
        assert!(locks.entries.is_empty(), "balanced end must clean entries");
    }

    #[test]
    fn stage_locks_disjoint_keys_do_not_conflict() {
        let mut locks = StageLocks::default();
        assert!(!locks.write_begin(1));
        assert!(!locks.write_begin(2));
        assert!(!locks.read_begin(3));
        locks.write_end(1);
        locks.write_end(2);
        locks.read_end(3);
    }

    #[test]
    fn dense_and_sparse_keys_never_collide() {
        // sparse keys carry the tag bit, dense keys never can (pointers stay
        // far below 2^111 even after the column shift)
        let sparse = sparse_lock_key(usize::MAX as *mut sys::ecs_component_record_t);
        assert!(sparse >> 127 == 1);
        let dense = dense_lock_key(usize::MAX as *mut sys::ecs_table_t, i16::MAX);
        assert!(dense >> 127 == 0);
    }

    #[test]
    fn safety_locks_resize() {
        let locks = SafetyLocks::new();
        locks.set_stage_count(4);
        for i in 0..4 {
            let stage = locks.stage(i);
            assert!(!unsafe { (*stage.as_ptr()).write_begin(7) });
            unsafe { (*stage.as_ptr()).write_end(7) };
        }
        locks.set_stage_count(0); // clamps to 1
        let _ = locks.stage(0);
    }
}
