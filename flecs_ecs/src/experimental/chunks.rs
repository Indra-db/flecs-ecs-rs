//! Chunk cursor: per-table-batch column slices for vectorised user code.
//!
//! [`QueryChunksExt::chunks`] takes `&mut World` (exclusive register) and yields,
//! one table batch at a time, whole-column slices such as `(&mut [A], &[B])`, so
//! the body can iterate or SIMD over a contiguous run. It is a *lending* cursor:
//! [`ChunkCursor::next`] borrows `&mut self`, so each chunk must be dropped
//! before the next is pulled (this is why it is a `while let`, not an
//! [`Iterator`]). The terminal [`ChunkCursor::for_each`] takes `self` by value,
//! so a consumed cursor cannot be reused (double-consumption is a compile error).
//!
//! Because the cursor hands out `&mut` column slices with no runtime locks, it
//! requires a provably-disjoint query (see [`disjoint`](super::disjoint));
//! `chunks` panics otherwise.

use core::marker::PhantomData;

use crate::core::{
    ComponentOrPairId, ComponentPointers, ExternIterNextFn, IterGuard, QueryAPI, QueryTuple, World,
};
use crate::sys;

/// One column of a chunk: `&T` yields `&[T]`, `&mut T` yields `&mut [T]`.
#[doc(hidden)]
pub trait ChunkElement {
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

/// Per-row indexing of a chunk column, preserving mutability: `&mut [T]` yields
/// `&mut T`, `&[T]` yields `&T`. Used by the [`each!`](crate::each) macro to
/// build the inner row loop.
#[doc(hidden)]
pub trait RowSlice {
    type Row<'r>
    where
        Self: 'r;
    fn row_len(&self) -> usize;
    fn row(&mut self, index: usize) -> Self::Row<'_>;
}

impl<T> RowSlice for &mut [T] {
    type Row<'r>
        = &'r mut T
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
}

impl<T> RowSlice for &[T] {
    type Row<'r>
        = &'r T
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
}

/// Maps a query tuple to its per-batch slice tuple.
#[doc(hidden)]
pub trait ChunkColumns {
    type Chunk<'c>;
    /// # Safety
    /// `ptrs` are this batch's dense column bases in term order, each valid for
    /// `count` elements and exclusively owned for `'c`.
    unsafe fn columns<'c>(ptrs: &[*mut u8], count: usize) -> Self::Chunk<'c>;
}

impl<A: ChunkElement> ChunkColumns for A {
    type Chunk<'c> = A::Slice<'c>;
    #[inline(always)]
    unsafe fn columns<'c>(ptrs: &[*mut u8], count: usize) -> Self::Chunk<'c> {
        unsafe { A::make(ptrs[0], count) }
    }
}

macro_rules! impl_chunk_columns_tuple {
    ($( $t:ident @ $idx:tt ),+ $(,)?) => {
        impl<$($t: ChunkElement),+> ChunkColumns for ($($t,)+) {
            type Chunk<'c> = ($($t::Slice<'c>,)+);
            #[inline(always)]
            unsafe fn columns<'c>(ptrs: &[*mut u8], count: usize) -> Self::Chunk<'c> {
                ($( unsafe { $t::make(ptrs[$idx], count) }, )+)
            }
        }
    };
}

impl_chunk_columns_tuple!(A @ 0, B @ 1);
impl_chunk_columns_tuple!(A @ 0, B @ 1, C @ 2);
impl_chunk_columns_tuple!(A @ 0, B @ 1, C @ 2, D @ 3);
impl_chunk_columns_tuple!(A @ 0, B @ 1, C @ 2, D @ 3, E @ 4);

/// A lending cursor over a query's table batches, yielding column slices.
///
/// Borrows `&mut World` for its whole life (exclusive register): no locks are
/// taken and the yielded `&mut` slices cannot alias anything else.
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

    /// Advance to the next table batch, yielding its column slices. Returns
    /// `None` when iteration is exhausted. Lending: the returned chunk borrows
    /// `&mut self`, so it must be dropped before calling `next` again.
    #[inline]
    #[allow(clippy::should_implement_trait)]
    pub fn next(&mut self) -> Option<T::Chunk<'_>> {
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
        let (_is_any, data) = T::create_ptrs(it);
        let count = it.count as usize;
        // SAFETY: exclusive register + proven-disjoint query, so the column
        // bases are exclusively owned and non-aliasing for this borrow.
        Some(unsafe { T::columns(data.column_ptrs(), count) })
    }

    /// Terminal op: run `f` on every chunk, consuming the cursor. Because it
    /// takes `self` by value, the cursor cannot be used again afterwards.
    #[inline]
    pub fn for_each(mut self, mut f: impl FnMut(T::Chunk<'_>)) {
        while let Some(chunk) = self.next() {
            f(chunk);
        }
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
    fn chunks<'w>(&self, world: &'w mut World) -> ChunkCursor<'w, P, T> {
        let _ = world;
        let world_ref = self.world();
        assert!(
            super::disjoint::is_proven_disjoint(world_ref, self.query_ptr()),
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
