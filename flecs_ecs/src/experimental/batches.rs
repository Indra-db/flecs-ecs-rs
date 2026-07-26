//! Locked batch cursor: dense column slices for an UNPROVEN query on the shared
//! register (spec §4.4/§4.5).
//!
//! [`QueryBatchesExt::batches`] takes `&World` (shared register) and yields, one
//! table batch at a time, the same whole-column slice tuples as
//! [`chunks`](super::chunks::QueryChunksExt::chunks), but for a query that could
//! *not* be proven disjoint. Because a concurrently held entity guard is
//! invisible to a build-time proof, the cursor registers **Tier-1 batch locks**
//! for every batch, so a conflict with a live guard (or another batch borrow)
//! panics like a `RefCell` — the shared-register contract.
//!
//! It is a **lending** cursor, and deliberately so: the lock protocol releases
//! the previous batch's borrows on the next [`next`](LockedBatches::next) call,
//! so a previous chunk must be dead by then. The borrow checker enforces exactly
//! that — [`next`](LockedBatches::next) borrows `&mut self`, so the yielded chunk
//! (which borrows the cursor) must be dropped before the next pull. Drive it with
//! `while let Some(chunk) = cursor.next()` or through the
//! [`each!`](crate::each) macro; never hold two chunks live.
//!
//! The lock protocol uses plain calls, never a per-batch RAII type (a per-batch
//! `Drop` measurably slowed the hot path): [`next`](LockedBatches::next) first
//! releases the previous batch's locks, then advances, then acquires the new
//! batch's locks. The final release happens when the cursor is fully consumed
//! (the terminal `next` that returns `None`) or when the cursor is dropped early.
//! Unwind recovery — a panic while a chunk is borrowed — is handled once per
//! iteration by the [`IterGuard`]'s stage-lock scope, which restores the borrow
//! map if a callback unwinds.

use core::marker::PhantomData;

use crate::core::{
    ComponentPointers, ExternIterNextFn, IterGuard, QueryAPI, QueryTuple, World, WorldProvider,
};
use crate::sys;

#[cfg(feature = "flecs_safety_locks")]
use crate::core::{StageLocks, WorldRef, acquire_batch_locks, release_batch_locks};
#[cfg(feature = "flecs_safety_locks")]
use core::ptr::NonNull;

use super::chunks::{ChunkColumns, EachCursor};

type BatchMarker<'w, P, T> = PhantomData<(fn() -> (P, T), &'w World)>;

/// A lending cursor over an unproven query's table batches, yielding dense
/// column slices under Tier-1 batch locks. See the [module docs](self).
pub struct LockedBatches<'w, P, T>
where
    T: QueryTuple,
{
    iter: IterGuard,
    iter_next: ExternIterNextFn,
    /// Column pointers + safety records for the currently-yielded batch. Held
    /// across calls so the next `next` can release this batch's locks by the same
    /// records it acquired them with.
    pointers: Option<T::Pointers>,
    #[cfg(feature = "flecs_safety_locks")]
    world: WorldRef<'w>,
    /// The stage map and release token for the batch whose locks are currently
    /// held, or `None` when no batch is locked (before the first `next`, after
    /// the last, or after an early drop released them).
    #[cfg(feature = "flecs_safety_locks")]
    held: Option<(NonNull<StageLocks>, usize)>,
    _marker: BatchMarker<'w, P, T>,
}

impl<'w, P, T> LockedBatches<'w, P, T>
where
    T: QueryTuple + ChunkColumns,
{
    #[doc(hidden)]
    #[cfg(feature = "flecs_safety_locks")]
    pub(crate) fn new(iter: IterGuard, iter_next: ExternIterNextFn, world: WorldRef<'w>) -> Self {
        LockedBatches {
            iter,
            iter_next,
            pointers: None,
            world,
            held: None,
            _marker: PhantomData,
        }
    }

    #[doc(hidden)]
    #[cfg(not(feature = "flecs_safety_locks"))]
    pub(crate) fn new(iter: IterGuard, iter_next: ExternIterNextFn) -> Self {
        LockedBatches {
            iter,
            iter_next,
            pointers: None,
            _marker: PhantomData,
        }
    }

    /// Release the locks held for the current batch, if any. Uses the records the
    /// acquire stored in `self.pointers`.
    #[cfg(feature = "flecs_safety_locks")]
    #[inline(always)]
    fn release_current(&mut self) {
        if let Some((locks, token)) = self.held.take() {
            let records = self
                .pointers
                .as_ref()
                .expect("held locks imply a populated batch")
                .safety_table_records();
            // Dense-only cursor, so no sparse terms: `ANY_SPARSE_TERMS = false`.
            release_batch_locks::<false, T>(&self.world, locks, token, records);
        }
    }

    /// Advance to the next dense table batch, releasing the previous batch's
    /// locks and acquiring the new batch's locks (spec §4.4). Returns the batch's
    /// column slices, borrowed until the returned value is dropped (lending), or
    /// `None` when iteration is exhausted.
    ///
    /// # Panics
    /// If the query has ref / inherited / sparse columns (not plain dense self
    /// terms), or if a batch borrow conflicts with a live guard on the shared
    /// register.
    #[inline]
    #[allow(clippy::should_implement_trait)]
    pub fn next(&mut self) -> Option<T::Chunk<'_>> {
        #[cfg(feature = "flecs_safety_locks")]
        self.release_current();

        loop {
            let next_fn = self.iter_next;
            if !self.iter.next(|i| unsafe { next_fn(i) }) {
                self.pointers = None;
                return None;
            }
            let it: &mut sys::ecs_iter_t = &mut self.iter;
            it.flags |= sys::EcsIterCppEach;
            assert!(
                (it.ref_fields | it.up_fields | it.row_fields) == 0,
                "batches() requires plain dense self columns; ref/inherited/sparse terms route to \
                 each_shared/run instead"
            );
            if it.count == 0 && it.table.is_null() {
                continue;
            }
            let count = it.count as usize;
            let (_is_any, pointers) = T::create_ptrs(it);
            self.pointers = Some(pointers);

            #[cfg(feature = "flecs_safety_locks")]
            {
                let records = self
                    .pointers
                    .as_ref()
                    .expect("just populated")
                    .safety_table_records();
                // Registers this batch's term borrows in the stage lock map;
                // panics on a conflict with a live guard. Dense-only: no sparse.
                let held = acquire_batch_locks::<false, T>(&self.world, records);
                self.held = Some(held);
            }

            let ptrs = self.pointers.as_ref().expect("just populated").column_ptrs();
            // SAFETY: the column bases are valid for `count` elements of this
            // batch's storage; the returned slices are tied to `&mut self` by the
            // signature (lending), so they cannot outlive the next release.
            return Some(unsafe { T::columns(ptrs, count) });
        }
    }
}

impl<'w, P, T> super::sealed::Sealed for LockedBatches<'w, P, T> where T: QueryTuple {}

impl<'w, P, T> EachCursor for LockedBatches<'w, P, T>
where
    T: QueryTuple + ChunkColumns,
{
    type ColArity = <T as ChunkColumns>::Arity;
    type Chunk<'c>
        = T::Chunk<'c>
    where
        Self: 'c;

    #[inline]
    fn each_next(&mut self) -> Option<T::Chunk<'_>> {
        self.next()
    }
}

#[cfg(feature = "flecs_safety_locks")]
impl<P, T> Drop for LockedBatches<'_, P, T>
where
    T: QueryTuple,
{
    fn drop(&mut self) {
        // Early-exit release: a `while let` broken out of leaves the last batch's
        // locks held. Release them here (one per iteration, not per batch). A
        // full run already released the last batch in the terminal `next`. The
        // `IterGuard`'s stage-lock scope, dropped right after this, is the unwind
        // backstop for a panic mid-yield or a partial acquire.
        if let (Some((locks, token)), Some(pointers)) = (self.held.take(), self.pointers.as_ref()) {
            release_batch_locks::<false, T>(
                &self.world,
                locks,
                token,
                pointers.safety_table_records(),
            );
        }
        // A batch body that fired a synchronous observer whose Rust hook panicked
        // has that panic stashed (spec §5.5). Resume it when the cursor is
        // dropped, unless a different panic is already unwinding through this
        // drop (resuming then would abort); that panic reaches its own entry
        // point instead.
        if !std::thread::panicking() {
            self.world.rethrow_stashed_panic();
        }
    }
}

/// Locked-batch cursor access on a query (shared register, unproven queries).
pub trait QueryBatchesExt<'a, P, T>: QueryAPI<'a, P, T>
where
    T: QueryTuple,
{
    /// Returns a lending [`LockedBatches`] cursor over this query's table batches
    /// on the shared register. Unlike [`chunks`](super::chunks::QueryChunksExt::chunks),
    /// the query need **not** be proven disjoint: every batch registers Tier-1
    /// locks, so a conflict with a live borrow panics.
    ///
    /// # Panics
    /// If the `&World` belongs to a different world than the query. Per-batch:
    /// if the query has ref / inherited / sparse columns, or a batch borrow
    /// conflicts with a live guard.
    fn batches<'w>(&self, world: &'w World) -> LockedBatches<'w, P, T>;
}

impl<'a, P, T, Q> QueryBatchesExt<'a, P, T> for Q
where
    Q: QueryAPI<'a, P, T>,
    T: QueryTuple + ChunkColumns,
{
    #[track_caller]
    fn batches<'w>(&self, world: &'w World) -> LockedBatches<'w, P, T> {
        // Cached world identity (spec §4.7): read the query's real world and
        // pointer-compare against the &World. No FFI.
        assert!(
            core::ptr::eq(unsafe { (*self.query_ptr()).real_world }, world.world_ptr()),
            "batches() requires the query's own world: the &World passed in belongs to a \
             different world than this query's storage"
        );

        #[cfg(feature = "flecs_safety_locks")]
        {
            LockedBatches::new(
                IterGuard::new(self.retrieve_iter()),
                self.iter_next_func(),
                world.world(),
            )
        }
        #[cfg(not(feature = "flecs_safety_locks"))]
        {
            let _ = world;
            LockedBatches::new(IterGuard::new(self.retrieve_iter()), self.iter_next_func())
        }
    }
}
