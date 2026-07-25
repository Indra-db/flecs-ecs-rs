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
/// Dense: `(table ptr << 16) | column`. Sparse: `(1 << 63) | component
/// record ptr`. Pointer-based keys need no FFI to derive and never collide
/// across the two shapes thanks to the tag bit (user-space pointers stay
/// below 2^47 on all tier-1 targets, so the shift cannot reach bit 63); both
/// pointers are stable for the world's lifetime and the maps are world-owned.
/// Entries are balanced begin/end pairs, so no key outlives the storage it
/// names. The same encodings are produced C-side by `ECS_RUST_DENSE_KEY` /
/// `ECS_RUST_SPARSE_KEY` in `flecs_rust.c` for `ecs_rust_get_ptr_t::lock_key`.
#[cfg(feature = "flecs_safety_locks")]
pub(crate) type LockKey = u64;

#[cfg(feature = "flecs_safety_locks")]
#[inline(always)]
pub(crate) fn dense_lock_key(table: *mut sys::ecs_table_t, column: i16) -> LockKey {
    ((table as usize as u64) << 16) | (column as u16 as u64)
}

#[cfg(feature = "flecs_safety_locks")]
#[inline(always)]
pub(crate) fn sparse_lock_key(cr: *mut sys::ecs_component_record_t) -> LockKey {
    (1u64 << 63) | (cr as usize as u64)
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
    pub(crate) fn read_begin(&mut self, key: LockKey) -> bool {
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
    pub(crate) fn read_end(&mut self, key: LockKey) {
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
    pub(crate) fn write_begin(&mut self, key: LockKey) -> bool {
        if self.find(key).is_some() {
            return true;
        }
        self.entries.push((key, WRITE_FLAG));
        false
    }

    /// Probe for a live write on `key` without registering a read.
    #[inline(always)]
    pub(crate) fn check_read(&self, key: LockKey) -> bool {
        self.entries
            .iter()
            .any(|entry| entry.0 == key && entry.1 & WRITE_FLAG != 0)
    }

    #[inline(always)]
    pub(crate) fn write_end(&mut self, key: LockKey) {
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
pub(crate) fn alias_violation_panic(world: &WorldRef, component_id: u64, write: bool) -> ! {
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

/// Take or release the borrow for one query term. Returns `true` when an
/// acquire conflicts; releases never fail.
#[cfg(feature = "flecs_safety_locks")]
#[inline(always)]
fn term_lock<const ACQUIRE: bool, const READONLY: bool>(
    world: &WorldRef<'_>,
    locks: NonNull<StageLocks>,
    info: &super::TableColumnSafety,
    any_sparse_terms: bool,
) -> bool {
    // if component_id is set, this term is a row (sparse) term
    let key = if any_sparse_terms && info.component_id != 0 {
        // batch-level resolve; keys must match the cr-keyed FieldAt /
        // rw_locking sparse paths, which never pay this lookup per row.
        // flecs_components_get requires the real world, not a stage.
        let cr = unsafe {
            let real_world = sys::ecs_get_world(world.raw_world.as_ptr() as *const _);
            sys::flecs_components_get(real_world, info.component_id)
        };
        sparse_lock_key(cr)
    } else if info.table.is_null() {
        return false;
    } else {
        dense_lock_key(info.table, info.column)
    };

    // SAFETY: stage map is owned by this thread.
    let map = unsafe { &mut *locks.as_ptr() };
    if READONLY {
        if ACQUIRE {
            return map.read_begin(key);
        }
        map.read_end(key);
    } else if ACQUIRE {
        return map.write_begin(key);
    } else {
        map.write_end(key);
    }
    false
}

/// Roll back the terms acquired before `index`, then report the violation.
#[cfg(feature = "flecs_safety_locks")]
#[cold]
#[inline(never)]
fn acquire_violation<const ANY_SPARSE_TERMS: bool, T: QueryTuple>(
    world: &WorldRef<'_>,
    locks: NonNull<StageLocks>,
    table_records: &[super::TableColumnSafety],
    index: usize,
    write: bool,
) -> ! {
    release_terms::<ANY_SPARSE_TERMS, T>(world, locks, table_records, index);
    // SAFETY: index is a term index the caller just visited.
    let info = unsafe { table_records.get_unchecked(index) };
    let id = if ANY_SPARSE_TERMS && info.component_id != 0 {
        info.component_id
    } else {
        dense_component_id(info.table, info.column)
    };
    alias_violation_panic(world, id, write);
}

/// Terms are laid out as four contiguous runs: immutable, mutable, optional
/// immutable, optional mutable. `upto` clamps every run so a partially
/// acquired batch releases exactly what it took.
#[cfg(feature = "flecs_safety_locks")]
#[inline(always)]
fn release_terms<const ANY_SPARSE_TERMS: bool, T: QueryTuple>(
    world: &WorldRef<'_>,
    locks: NonNull<StageLocks>,
    table_records: &[super::TableColumnSafety],
    upto: usize,
) {
    let end_immutable: usize = const { T::COUNT_IMMUTABLE };
    let end_mutable: usize = const { T::COUNT_IMMUTABLE + T::COUNT_MUTABLE };
    let end_optional_immutable: usize =
        const { T::COUNT_IMMUTABLE + T::COUNT_MUTABLE + T::COUNT_OPTIONAL_IMMUTABLE };
    let end_optional_mutable: usize = const {
        T::COUNT_IMMUTABLE
            + T::COUNT_MUTABLE
            + T::COUNT_OPTIONAL_IMMUTABLE
            + T::COUNT_OPTIONAL_MUTABLE
    };

    unsafe {
        for i in 0..end_immutable.min(upto) {
            let info = table_records.get_unchecked(i);
            term_lock::<false, true>(world, locks, info, ANY_SPARSE_TERMS);
        }
        for i in end_immutable..end_mutable.min(upto) {
            let info = table_records.get_unchecked(i);
            term_lock::<false, false>(world, locks, info, ANY_SPARSE_TERMS);
        }
        for i in end_mutable..end_optional_immutable.min(upto) {
            let info = table_records.get_unchecked(i);
            term_lock::<false, true>(world, locks, info, ANY_SPARSE_TERMS);
        }
        for i in end_optional_immutable..end_optional_mutable.min(upto) {
            let info = table_records.get_unchecked(i);
            term_lock::<false, false>(world, locks, info, ANY_SPARSE_TERMS);
        }
    }
}

#[cfg(feature = "flecs_safety_locks")]
#[inline(always)]
fn acquire_terms<const ANY_SPARSE_TERMS: bool, T: QueryTuple>(
    world: &WorldRef<'_>,
    locks: NonNull<StageLocks>,
    table_records: &[super::TableColumnSafety],
) {
    let end_immutable: usize = const { T::COUNT_IMMUTABLE };
    let end_mutable: usize = const { T::COUNT_IMMUTABLE + T::COUNT_MUTABLE };
    let end_optional_immutable: usize =
        const { T::COUNT_IMMUTABLE + T::COUNT_MUTABLE + T::COUNT_OPTIONAL_IMMUTABLE };
    let end_optional_mutable: usize = const {
        T::COUNT_IMMUTABLE
            + T::COUNT_MUTABLE
            + T::COUNT_OPTIONAL_IMMUTABLE
            + T::COUNT_OPTIONAL_MUTABLE
    };

    unsafe {
        for i in 0..end_immutable {
            let info = table_records.get_unchecked(i);
            if term_lock::<true, true>(world, locks, info, ANY_SPARSE_TERMS) {
                acquire_violation::<ANY_SPARSE_TERMS, T>(world, locks, table_records, i, false);
            }
        }
        for i in end_immutable..end_mutable {
            let info = table_records.get_unchecked(i);
            if term_lock::<true, false>(world, locks, info, ANY_SPARSE_TERMS) {
                acquire_violation::<ANY_SPARSE_TERMS, T>(world, locks, table_records, i, true);
            }
        }
        for i in end_mutable..end_optional_immutable {
            let info = table_records.get_unchecked(i);
            if term_lock::<true, true>(world, locks, info, ANY_SPARSE_TERMS) {
                acquire_violation::<ANY_SPARSE_TERMS, T>(world, locks, table_records, i, false);
            }
        }
        for i in end_optional_immutable..end_optional_mutable {
            let info = table_records.get_unchecked(i);
            if term_lock::<true, false>(world, locks, info, ANY_SPARSE_TERMS) {
                acquire_violation::<ANY_SPARSE_TERMS, T>(world, locks, table_records, i, true);
            }
        }
    }
}

/// Releases a batch's term borrows when the scope ends, including when the
/// iteration callback unwinds.
#[cfg(feature = "flecs_safety_locks")]
pub(crate) struct QueryLockGuard<'a, T: QueryTuple, const ANY_SPARSE_TERMS: bool> {
    world: WorldRef<'a>,
    locks: NonNull<StageLocks>,
    table_records: *const super::TableColumnSafety,
    len: usize,
    _marker: core::marker::PhantomData<fn() -> T>,
}

#[cfg(feature = "flecs_safety_locks")]
impl<T: QueryTuple, const ANY_SPARSE_TERMS: bool> Drop for QueryLockGuard<'_, T, ANY_SPARSE_TERMS> {
    #[inline(always)]
    fn drop(&mut self) {
        // SAFETY: the guard never outlives the `ComponentsData` the records
        // live in, and the stage map is owned by this thread.
        let table_records = unsafe { core::slice::from_raw_parts(self.table_records, self.len) };
        release_terms::<ANY_SPARSE_TERMS, T>(&self.world, self.locks, table_records, usize::MAX);
    }
}

/// Take the borrows for one table batch. The returned guard releases them,
/// so a panic in the iteration callback cannot leak a borrow.
///
/// The guard borrows `table_records` as a raw pointer: callers must keep the
/// owning `ComponentsData` alive for the guard's whole lifetime, which drop
/// order gives them for free when the guard is declared after it.
#[cfg(feature = "flecs_safety_locks")]
#[inline]
pub(crate) fn acquire_read_write_locks<'a, T: QueryTuple, const ANY_SPARSE_TERMS: bool>(
    world: &WorldRef<'a>,
    table_records: &[super::TableColumnSafety],
) -> QueryLockGuard<'a, T, ANY_SPARSE_TERMS> {
    let locks = if world.is_currently_multithreaded() {
        stage_locks::<true>(world)
    } else {
        stage_locks::<false>(world)
    };
    acquire_terms::<ANY_SPARSE_TERMS, T>(world, locks, table_records);
    QueryLockGuard {
        world: *world,
        locks,
        table_records: table_records.as_ptr(),
        len: table_records.len(),
        _marker: core::marker::PhantomData,
    }
}

/// Releases a single write borrow when the scope ends, including on unwind.
#[cfg(feature = "flecs_safety_locks")]
pub(crate) struct WriteLockGuard {
    locks: NonNull<StageLocks>,
    key: LockKey,
}

#[cfg(feature = "flecs_safety_locks")]
impl WriteLockGuard {
    /// Panics when the borrow conflicts, in which case nothing is registered
    /// and no guard is produced.
    #[inline(always)]
    pub(crate) fn acquire(
        world: &WorldRef<'_>,
        locks: NonNull<StageLocks>,
        key: LockKey,
        id: u64,
    ) -> Self {
        // SAFETY: stage map is owned by this thread.
        if unsafe { (*locks.as_ptr()).write_begin(key) } {
            alias_violation_panic(world, id, true);
        }
        Self { locks, key }
    }

    /// Release inline rather than on unwind.
    #[inline(always)]
    pub(crate) fn release(self) {
        // SAFETY: as in `acquire`.
        unsafe { (*self.locks.as_ptr()).write_end(self.key) };
        core::mem::forget(self);
    }
}

#[cfg(feature = "flecs_safety_locks")]
impl Drop for WriteLockGuard {
    #[inline(always)]
    fn drop(&mut self) {
        // SAFETY: as in `acquire`.
        unsafe { (*self.locks.as_ptr()).write_end(self.key) };
    }
}

#[cfg(all(test, feature = "flecs_safety_locks"))]
mod tests {
    use super::*;

    #[test]
    fn stage_locks_read_write_protocol() {
        let mut locks = StageLocks::default();
        let key = 42u64;
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
        let sparse = sparse_lock_key(0x7000_0000_0000 as *mut sys::ecs_component_record_t);
        assert!(sparse >> 63 == 1);
        // highest realistic user-space pointer (2^47 - 1): the column shift
        // must not reach the sparse tag bit
        let dense = dense_lock_key(0x7FFF_FFFF_FFFF as *mut sys::ecs_table_t, i16::MAX);
        assert!(dense >> 63 == 0);
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
