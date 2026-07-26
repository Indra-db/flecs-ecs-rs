//! Chunk cursor: per-table-batch column slices for vectorised user code.
//!
//! [`QueryChunksExt::chunks`] takes `&mut World` (exclusive register) and returns
//! a true [`Iterator`] whose `Item` is one table batch's whole-column slice tuple
//! such as `(&mut [A], &[B])`, so the body can iterate or SIMD over a contiguous
//! run. The yielded slices borrow the table storage with the cursor's `'w` (the
//! `&'w mut World`), not the cursor, so an item outlives a `next()` call and the
//! std combinators (`zip` / `enumerate` / `sum` / `collect`) and rayon's
//! `par_bridge` compose directly on the chunk stream. [`Iterator::for_each`]
//! remains the by-value terminal.
//!
//! Non-overlap of the yielded `&mut` slices rests on two facts: `chunks` only
//! accepts a provably-disjoint query (see [`disjoint`](super::disjoint); it
//! panics otherwise), and flecs visits each `(table, row-range)` at most once per
//! iteration, so no two chunks alias — this holds for plain, sorted, grouped, and
//! change-skipped iteration (spec §4.6).
//!
//! The [`each!`](crate::each) macro drives the cursor through [`EachCursor`], a
//! lending shape that also covers the shared-register locked-batches cursor.

use core::marker::PhantomData;

use crate::core::{
    ComponentOrPairId, ComponentPointers, ExternIterNextFn, IterGuard, QueryAPI, QueryTuple, World,
    WorldProvider,
};
use crate::sys;
use flecs_ecs_derive::tuples;

use super::sealed::Sealed;

impl<T: ComponentOrPairId> Sealed for &T {}
impl<T: ComponentOrPairId> Sealed for &mut T {}
impl<T: ComponentOrPairId> Sealed for Option<&T> {}
impl<T: ComponentOrPairId> Sealed for Option<&mut T> {}
impl<T> Sealed for &[T] {}
impl<T> Sealed for &mut [T] {}

/// A column-count marker so [`each!`](crate::each) can turn a binding-count vs
/// column-count mismatch into a **named** compile error at type-check time (spec
/// §9.5) instead of a raw tuple-destructure mismatch. A cursor whose query has
/// `M` columns reports [`Cols<M>`] as its
/// [`ColArity`](EachCursor::ColArity); the bound [`Cols<M>: EachArity<N>`] holds
/// only when `M == N`, so a mismatch fails the bound with this message. The
/// check is lifetime-free and reads no cursor value, so it stays zero-cost.
#[doc(hidden)]
#[diagnostic::on_unimplemented(
    message = "each!: the number of bound names does not equal the query's column count",
    label = "each! binding-count vs column-count mismatch"
)]
pub trait EachArity<const N: usize> {}

/// Type-level column count (see [`EachArity`]).
#[doc(hidden)]
pub struct Cols<const N: usize>;

impl<const N: usize> EachArity<N> for Cols<N> {}

/// Binds a cursor's column count `N` at the type level so [`each!`](crate::each)
/// reports a **named** binding-count vs column-count mismatch (spec §9.5). Takes
/// the cursor by shared reference and does nothing: it inlines away to nothing,
/// so the check is zero-cost and never moves or spills the cursor.
#[doc(hidden)]
#[inline(always)]
pub fn arity_assert<C, const N: usize>(_cursor: &C)
where
    C: EachCursor,
    <C as EachCursor>::ColArity: EachArity<N>,
{
}

/// One column of a chunk: `&T` yields `&[T]`, `&mut T` yields `&mut [T]`.
#[doc(hidden)]
#[diagnostic::on_unimplemented(
    message = "`{Self}` is not a valid chunk column",
    label = "not a chunk column",
    note = "a chunk column must be `&T`, `&mut T`, `Option<&T>`, or `Option<&mut T>` for a component `T`"
)]
pub trait ChunkElement: Sealed {
    type Slice<'c>;
    /// # Safety
    /// `ptr` is the base of a dense column with at least `count` valid elements,
    /// exclusively owned for `'c` (guaranteed by the `&mut World` register and
    /// the disjointness proof).
    unsafe fn make<'c>(ptr: *mut u8, count: usize) -> Self::Slice<'c>;
}

impl<T: ComponentOrPairId> ChunkElement for &T {
    type Slice<'c> = &'c [<T as ComponentOrPairId>::CastType];
    #[inline(always)]
    unsafe fn make<'c>(ptr: *mut u8, count: usize) -> Self::Slice<'c> {
        unsafe { core::slice::from_raw_parts(ptr as *const _, count) }
    }
}

impl<T: ComponentOrPairId> ChunkElement for &mut T {
    type Slice<'c> = &'c mut [<T as ComponentOrPairId>::CastType];
    #[inline(always)]
    unsafe fn make<'c>(ptr: *mut u8, count: usize) -> Self::Slice<'c> {
        unsafe { core::slice::from_raw_parts_mut(ptr as *mut _, count) }
    }
}

impl<T: ComponentOrPairId> ChunkElement for Option<&T> {
    type Slice<'c> = Option<&'c [<T as ComponentOrPairId>::CastType]>;
    #[inline(always)]
    unsafe fn make<'c>(ptr: *mut u8, count: usize) -> Self::Slice<'c> {
        if ptr.is_null() {
            None
        } else {
            Some(unsafe { core::slice::from_raw_parts(ptr as *const _, count) })
        }
    }
}

impl<T: ComponentOrPairId> ChunkElement for Option<&mut T> {
    type Slice<'c> = Option<&'c mut [<T as ComponentOrPairId>::CastType]>;
    #[inline(always)]
    unsafe fn make<'c>(ptr: *mut u8, count: usize) -> Self::Slice<'c> {
        if ptr.is_null() {
            None
        } else {
            Some(unsafe { core::slice::from_raw_parts_mut(ptr as *mut _, count) })
        }
    }
}

/// Per-row access to a chunk column, preserving mutability: `&mut [T]` yields
/// `&mut T`, `&[T]` yields `&T`. Used by the [`each!`](crate::each) macro to
/// build the inner row loop.
///
/// [`rows`](RowSlice::rows) is the pre-checked path: it returns the column's
/// native slice iterator (`slice::Iter` / `slice::IterMut`), so the macro's row
/// loop advances by pointer without a per-row bounds check. [`row`](RowSlice::row)
/// / [`row_len`](RowSlice::row_len) are the indexed fallback.
#[doc(hidden)]
pub trait RowSlice: Sealed {
    type Row<'r>
    where
        Self: 'r;
    type RowIter<'r>: Iterator<Item = Self::Row<'r>>
    where
        Self: 'r;
    fn row_len(&self) -> usize;
    fn row(&mut self, index: usize) -> Self::Row<'_>;
    /// The column's native slice iterator, yielding one `Row` per element with
    /// the bounds check hoisted out of the row loop.
    fn rows(&mut self) -> Self::RowIter<'_>;
}

impl<T> RowSlice for &mut [T] {
    type Row<'r>
        = &'r mut T
    where
        Self: 'r;
    type RowIter<'r>
        = core::slice::IterMut<'r, T>
    where
        Self: 'r;
    #[inline(always)]
    fn row_len(&self) -> usize {
        (**self).len()
    }
    #[inline(always)]
    fn row(&mut self, index: usize) -> &mut T {
        &mut (**self)[index]
    }
    #[inline(always)]
    fn rows(&mut self) -> core::slice::IterMut<'_, T> {
        (**self).iter_mut()
    }
}

impl<T> RowSlice for &[T] {
    type Row<'r>
        = &'r T
    where
        Self: 'r;
    type RowIter<'r>
        = core::slice::Iter<'r, T>
    where
        Self: 'r;
    #[inline(always)]
    fn row_len(&self) -> usize {
        (**self).len()
    }
    #[inline(always)]
    fn row(&mut self, index: usize) -> &T {
        &(**self)[index]
    }
    #[inline(always)]
    fn rows(&mut self) -> core::slice::Iter<'_, T> {
        (**self).iter()
    }
}

/// Maps a query tuple to its per-batch slice tuple.
#[doc(hidden)]
#[diagnostic::on_unimplemented(
    message = "`{Self}` cannot be iterated as chunk columns",
    label = "not a chunk-column tuple",
    note = "`chunks` / `batches` need a query tuple of dense columns (`&T`, `&mut T`, `Option<&T>`, `Option<&mut T>`); ref/inherited/sparse terms route to `each_shared` or `run`"
)]
pub trait ChunkColumns: Sealed {
    type Chunk<'c>;
    /// The lifetime-free column-count marker for this chunk shape (see
    /// [`EachArity`]).
    type Arity;
    /// # Safety
    /// `ptrs` are this batch's dense column bases in term order, each valid for
    /// `count` elements and exclusively owned for `'c`.
    unsafe fn columns<'c>(ptrs: &[*mut u8], count: usize) -> Self::Chunk<'c>;
}

impl<A: ChunkElement> ChunkColumns for A {
    type Chunk<'c> = A::Slice<'c>;
    type Arity = Cols<1>;
    #[inline(always)]
    unsafe fn columns<'c>(ptrs: &[*mut u8], count: usize) -> Self::Chunk<'c> {
        unsafe { A::make(ptrs[0], count) }
    }
}

macro_rules! chunk_column_count {
    () => { 0usize };
    ($head:ident $(, $tail:ident)*) => { 1usize + chunk_column_count!($($tail),*) };
}

macro_rules! impl_chunk_columns_tuple {
    ($( $t:ident ),+) => {
        impl<$($t: ChunkElement),+> Sealed for ($($t,)+) {}
        impl<$($t: ChunkElement),+> ChunkColumns for ($($t,)+) {
            type Chunk<'c> = ($($t::Slice<'c>,)+);
            type Arity = Cols<{ chunk_column_count!($($t),+) }>;
            #[inline(always)]
            unsafe fn columns<'c>(ptrs: &[*mut u8], count: usize) -> Self::Chunk<'c> {
                let mut column: isize = -1;
                ($( {
                    column += 1;
                    unsafe { $t::make(ptrs[column as usize], count) }
                }, )+)
            }
        }
    };
}

tuples!(impl_chunk_columns_tuple, 1, 32);

/// A true [`Iterator`] over a query's table batches, yielding whole-column
/// slices.
///
/// Borrows `&'w mut World` for its whole life (exclusive register): no locks are
/// taken. The yielded slices borrow the **table storage** with the cursor's
/// `'w`, not the cursor, so successive chunks do not alias and an item outlives a
/// [`next`](Iterator::next) call. This is sound because `chunks` only accepts a
/// proven-disjoint query (spec §4.7) and flecs visits each `(table, row-range)`
/// at most once per iteration (spec §4.6), so no two yielded slices ever overlap
/// — even for sorted or grouped queries (the sort merge partitions each table's
/// rows into disjoint ranges).
type ChunkMarker<'w, P, T> = PhantomData<(fn() -> (P, T), &'w mut World)>;

pub struct ChunkCursor<'w, P, T> {
    iter: IterGuard,
    iter_next: ExternIterNextFn,
    _marker: ChunkMarker<'w, P, T>,
}

impl<'w, P, T> ChunkCursor<'w, P, T>
where
    T: QueryTuple + ChunkColumns,
{
    #[doc(hidden)]
    pub(crate) fn new(iter: IterGuard, iter_next: ExternIterNextFn) -> Self {
        ChunkCursor {
            iter,
            iter_next,
            _marker: PhantomData,
        }
    }

}

impl<'w, P, T> Iterator for ChunkCursor<'w, P, T>
where
    T: QueryTuple + ChunkColumns,
{
    type Item = T::Chunk<'w>;

    /// Yield the next dense table batch's whole-column slice tuple, bound to the
    /// cursor's `'w` (the `&'w mut World`), not to `&mut self`. Singleton-only
    /// batches (`count == 0 && table.is_null()`) carry no dense self columns and
    /// are skipped (spec §4.6).
    #[inline]
    fn next(&mut self) -> Option<T::Chunk<'w>> {
        loop {
            let next_fn = self.iter_next;
            if !self.iter.next(|i| unsafe { next_fn(i) }) {
                return None;
            }
            let it: &mut sys::ecs_iter_t = &mut self.iter;
            it.flags |= sys::EcsIterCppEach;
            assert!(
                (it.ref_fields | it.up_fields | it.row_fields) == 0,
                "chunks() requires plain dense self columns; ref/inherited/sparse terms are not \
                 supported by the chunk cursor"
            );
            if it.count == 0 && it.table.is_null() {
                continue;
            }
            let count = it.count as usize;
            let (_is_any, data) = T::create_ptrs(it);
            // SAFETY: exclusive register + proven-disjoint query + single-visit
            // iteration, so each batch's column bases are exclusively owned and
            // pairwise non-aliasing across the whole `'w` iteration. `T::columns`
            // reads `data`'s pointer array to build slices over the table storage;
            // the returned slices borrow that storage, not `data`, so binding them
            // to `'w` is sound.
            return Some(unsafe { T::columns(data.column_ptrs(), count) });
        }
    }
}

/// Cursor shape consumed by the [`each!`](crate::each) macro: a lending
/// `each_next` that yields one chunk borrowed for the duration of a single loop
/// body. Both the exclusive [`ChunkCursor`] (a true `Iterator`) and the shared
/// [`LockedBatches`] (a lending, Tier-1-locked cursor) implement it, so `each!`
/// drives either with one expansion.
#[doc(hidden)]
#[diagnostic::on_unimplemented(
    message = "`{Self}` is not an `each!` cursor",
    label = "not an `each!` cursor",
    note = "`each!` drives a `query.chunks(&mut world)` or `query.batches(&world)` cursor; pass one of those as the `in` expression"
)]
pub trait EachCursor: Sealed {
    /// The lifetime-free column-count marker for the cursor's query (see
    /// [`EachArity`]); [`each!`](crate::each) bounds it to emit a named
    /// binding-count vs column-count error (spec §9.5).
    type ColArity;
    type Chunk<'c>
    where
        Self: 'c;
    /// Advance and return the next chunk, borrowed until the returned value is
    /// dropped. Returns `None` at end of iteration.
    fn each_next(&mut self) -> Option<Self::Chunk<'_>>;
}

impl<'w, P, T> Sealed for ChunkCursor<'w, P, T> where T: QueryTuple + ChunkColumns {}

impl<'w, P, T> EachCursor for ChunkCursor<'w, P, T>
where
    T: QueryTuple + ChunkColumns,
{
    type ColArity = <T as ChunkColumns>::Arity;
    // The exclusive cursor yields slices bound to the world's `'w`, not to the
    // per-call `&mut self` borrow, so the macro's chunk (which the body drops
    // each turn) is simply the `Iterator` item.
    type Chunk<'c>
        = T::Chunk<'w>
    where
        Self: 'c;

    #[inline]
    fn each_next(&mut self) -> Option<T::Chunk<'w>> {
        Iterator::next(self)
    }
}

/// Chunk-cursor access on a query. Provisional name.
pub trait QueryChunksExt<'a, P, T>: QueryAPI<'a, P, T>
where
    T: QueryTuple,
{
    /// Returns a lending [`ChunkCursor`] over this query's table batches on the
    /// exclusive register.
    ///
    /// # Panics
    /// If the query is not provably disjoint (distinct dense self component ids,
    /// no wildcards / sparse / traversal terms), since the cursor hands out
    /// `&mut` column slices with no runtime locks.
    fn chunks<'w>(&self, world: &'w mut World) -> ChunkCursor<'w, P, T>;
}

impl<'a, P, T, Q> QueryChunksExt<'a, P, T> for Q
where
    Q: QueryAPI<'a, P, T>,
    T: QueryTuple + ChunkColumns,
{
    #[track_caller]
    fn chunks<'w>(&self, world: &'w mut World) -> ChunkCursor<'w, P, T> {
        let world_ref = self.world();
        // Cached world identity (spec §4.7): read the query's real world and
        // pointer-compare against the &mut World (always a real world). No FFI.
        assert!(
            core::ptr::eq(
                unsafe { (*self.query_ptr()).real_world },
                (&*world).world_ptr()
            ),
            "chunks() requires the query's own world: the &mut World passed in belongs to a \
             different world and cannot prove exclusive access to this query's storage"
        );
        assert!(
            super::disjoint::is_proven_disjoint_cached(
                self.disjoint_cache(),
                world_ref,
                self.query_ptr(),
            ),
            "chunks() requires a provably-disjoint query: distinct dense self component ids, no \
             wildcards, sparse, or traversal terms. This query could not be proven disjoint."
        );

        #[cfg(feature = "flecs_safety_locks")]
        {
            let locks = crate::core::stage_locks_dyn(&world_ref);
            // SAFETY: the stage map is owned by this thread.
            debug_assert!(
                unsafe { (*locks.as_ptr()).is_empty() },
                "chunks() entered with a live borrow in the stage lock map; the &mut World \
                 exclusivity invariant was violated"
            );
        }

        ChunkCursor::new(IterGuard::new(self.retrieve_iter()), self.iter_next_func())
    }
}
