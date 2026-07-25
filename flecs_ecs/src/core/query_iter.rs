//! Class that extends the capabilities of a [`Query`] by providing additional operations on the query's iterator.
use core::ffi::c_void;

use crate::core::*;
use crate::sys;

/// Owns a raw iterator for the duration of one iteration and releases it on
/// drop unless the iteration ran to completion, in which case the C side
/// already released it. Without this a panicking user callback would leak the
/// iterator's cursor into the stage stack allocator, which flecs reports as a
/// leak when the world is destroyed.
pub(crate) struct IterGuard {
    iter: sys::ecs_iter_t,
    live: bool,
}

impl IterGuard {
    #[inline(always)]
    pub(crate) fn new(iter: sys::ecs_iter_t) -> IterGuard {
        IterGuard { iter, live: true }
    }

    /// Advance the iterator with `next`. Returning `false` means the C side
    /// finalized the iterator, so the guard stops owning it; leaving the loop
    /// any other way keeps the guard armed.
    #[inline(always)]
    pub(crate) fn next(&mut self, next: impl FnOnce(&mut sys::ecs_iter_t) -> bool) -> bool {
        let more = next(&mut self.iter);
        self.live &= more;
        more
    }

    /// Finalize now instead of on drop.
    #[inline(always)]
    pub(crate) fn fini(&mut self) {
        if self.live {
            self.live = false;
            // SAFETY: the iterator came from `retrieve_iter` and has not been
            // finalized yet.
            unsafe { sys::ecs_iter_fini(&mut self.iter) };
        }
    }
}

impl core::ops::Deref for IterGuard {
    type Target = sys::ecs_iter_t;

    #[inline(always)]
    fn deref(&self) -> &sys::ecs_iter_t {
        &self.iter
    }
}

impl core::ops::DerefMut for IterGuard {
    #[inline(always)]
    fn deref_mut(&mut self) -> &mut sys::ecs_iter_t {
        &mut self.iter
    }
}

impl Drop for IterGuard {
    #[inline(always)]
    fn drop(&mut self) {
        self.fini();
    }
}

/// An iterator over a query, bound to the world or stage it was created with.
///
/// `QueryIter` holds raw world/iterator pointers and is therefore
/// `!Send`/`!Sync` — it cannot cross threads, even scoped ones:
///
/// ```compile_fail,E0277
/// use flecs_ecs::prelude::*;
///
/// #[derive(Component)]
/// struct Position {
///     x: f32,
/// }
///
/// let world = World::new();
/// world.entity().set(Position { x: 0.0 });
///
/// let query = world.new_query::<&Position>();
/// let iter = query.iterable();
/// std::thread::scope(|s| {
///     s.spawn(move || {
///         drop(iter);
///     });
/// });
/// ```
/// A `QueryIter` is single-shot: the underlying C iterator owns resources
/// (a cursor into the stage's stack allocator) that are released exactly once,
/// either when iteration runs to completion or on drop. Consuming operations
/// ([`QueryAPI::each`], [`QueryAPI::run`],
/// [`QueryAPI::count`], ...) hand those resources to the C side;
/// using the same `QueryIter` for a second consuming operation panics.
/// Create a fresh one with `query.iterable()` instead.
pub struct QueryIter<'a, P, T>
where
    T: QueryTuple,
{
    iter: sys::ecs_iter_t,
    iter_next: ExternIterNextFn,
    consumed: core::cell::Cell<bool>,
    _phantom: core::marker::PhantomData<&'a (P, T)>,
}

impl<P, T> QueryIter<'_, P, T>
where
    T: QueryTuple,
{
    pub fn new(iter: sys::ecs_iter_t, iter_next: ExternIterNextFn) -> Self {
        Self {
            iter,
            iter_next,
            consumed: core::cell::Cell::new(false),
            _phantom: core::marker::PhantomData,
        }
    }

    /// Marks the iterator as consumed and returns the raw iterator, transferring
    /// responsibility for releasing its resources to the caller.
    fn take_iter(&self) -> sys::ecs_iter_t {
        assert!(
            !self.consumed.get(),
            "QueryIter already consumed: each/run/count and other consuming operations \
             release the underlying flecs iterator. Create a new one with `query.iterable()`."
        );
        self.consumed.set(true);
        self.iter
    }

    /// Limit results to tables with specified group id (grouped queries only)
    ///
    /// # Arguments
    ///
    /// * `group_id`: the group id to set
    pub fn set_group(&mut self, group_id: impl IntoEntity) -> &mut Self {
        unsafe { sys::ecs_iter_set_group(&mut self.iter, *group_id.into_entity(self.world())) }
        self
    }

    /// set variable of iter
    ///
    /// # Arguments
    ///
    /// * `var_id`: the variable id to set
    ///
    /// * `value`: the value to set
    pub fn set_var(&mut self, var_id: i32, value: impl Into<Entity>) -> &mut Self {
        ecs_assert!(var_id != -1, FlecsErrorCode::InvalidParameter, 0);
        unsafe { sys::ecs_iter_set_var(&mut self.iter, var_id, *value.into()) };
        self
    }

    /// set variable of iter as table
    ///
    /// # Arguments
    ///
    /// * `var_id`: the variable id to set
    ///
    /// * `range`: the range to set
    pub fn set_var_table(&mut self, var_id: i32, table: impl IntoTableRange) -> &mut Self {
        ecs_assert!(var_id != -1, FlecsErrorCode::InvalidParameter, 0);
        unsafe { sys::ecs_iter_set_var_as_range(&mut self.iter, var_id, &table.range_raw()) };
        self
    }

    /// set variable for rule iter
    ///
    /// # Arguments
    ///
    /// * `name`: the name of the variable to set
    /// * `value`: the value to set
    pub fn set_var_expr(&mut self, name: &str, value: impl Into<Entity>) -> &mut Self {
        let name = compact_str::format_compact!("{}\0", name);

        let query = self.iter.query;
        let var_id = unsafe { sys::ecs_query_find_var(query, name.as_ptr() as *const _) };
        ecs_assert!(
            var_id != -1,
            FlecsErrorCode::InvalidParameter,
            name.as_str()
        );
        unsafe { sys::ecs_iter_set_var(&mut self.iter, var_id, *value.into()) };
        self
    }

    /// set variable for rule iter as table
    ///
    /// # Arguments
    ///
    /// * `name`: the name of the variable to set
    /// * `range`: the range to set
    pub fn set_var_table_expr(&mut self, name: &str, table: impl IntoTableRange) -> &mut Self {
        let name = compact_str::format_compact!("{}\0", name);

        let query = self.iter.query;
        let var_id = unsafe { sys::ecs_query_find_var(query, name.as_ptr() as *const _) };
        ecs_assert!(
            var_id != -1,
            FlecsErrorCode::InvalidParameter,
            name.as_str()
        );
        unsafe { sys::ecs_iter_set_var_as_range(&mut self.iter, var_id, &table.range_raw()) };
        self
    }
}

impl<P, T> Drop for QueryIter<'_, P, T>
where
    T: QueryTuple,
{
    fn drop(&mut self) {
        if !self.consumed.get() {
            // SAFETY: the iterator was created by ecs_query_iter (or a chained
            // variant) and has not been handed to a consuming operation, so its
            // resources are still live and owned by this struct. ecs_iter_fini
            // is the documented release path for iterators that were not
            // iterated until completion.
            unsafe { sys::ecs_iter_fini(&mut self.iter) };
        }
    }
}

#[doc(hidden)]
impl<P, T> IterOperations for QueryIter<'_, P, T>
where
    T: QueryTuple,
{
    #[inline(always)]
    fn retrieve_iter(&self) -> sys::ecs_iter_t {
        self.take_iter()
    }

    fn retrieve_iter_stage<'w>(&self, _stage: impl WorldProvider<'w>) -> sys::ecs_iter_t {
        panic!(
            "Cannot change the stage of an iterator that already exists. Use retrieve_iter_stage on the underlying query instead."
        );
    }

    #[inline(always)]
    fn iter_next(&self, iter: &mut sys::ecs_iter_t) -> bool {
        unsafe { (self.iter_next)(iter) }
    }

    #[inline(always)]
    fn query_ptr(&self) -> *const sys::ecs_query_t {
        self.iter.query
    }

    #[inline(always)]
    fn iter_next_func(&self) -> ExternIterNextFn {
        self.iter_next
    }
}

impl<'a, P, T> QueryAPI<'a, P, T> for QueryIter<'a, P, T>
where
    T: QueryTuple,
    Self: WorldProvider<'a>,
{
    fn entity(&self) -> EntityView<'_> {
        let world = unsafe { WorldRef::from_ptr(self.iter.real_world) };
        EntityView::new_from(world, unsafe {
            sys::ecs_get_entity(self.iter.query as *const c_void)
        })
    }
}

/// Formats the query as a string expression using `ecs_query_str`.
/// The resulting expression can be parsed to create the same query.
impl<P, T> core::fmt::Display for QueryIter<'_, P, T>
where
    T: QueryTuple,
{
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{}", query_expr_string(IterOperations::query_ptr(self)))
    }
}

impl<'a, P, T> WorldProvider<'a> for QueryIter<'a, P, T>
where
    T: QueryTuple,
{
    fn world(&self) -> WorldRef<'a> {
        unsafe { WorldRef::from_ptr(self.iter.world) }
    }
}

impl<'a, P, T> QueryIter<'a, P, T>
where
    T: QueryTuple,
{
    /// Return a paged iterator.
    ///
    /// Paged iterators limit the results to those starting at `offset`, returning at most `limit` results.
    ///
    /// # Arguments
    ///
    /// * `offset` - Number of entities to skip.
    /// * `limit`  - Maximum number of entities to return.
    pub fn page(self, offset: i32, limit: i32) -> ChainedIter<'a, P, T> {
        ChainedIter::new_page(self, offset, limit)
    }

    /// Return a worker iterator.
    ///
    /// Worker iterators evenly distribute matched entities across N workers.
    /// Useful for multi-threaded processing of query results.
    ///
    /// # Arguments
    ///
    /// * `index` - Zero-based index of this worker (0..count-1).
    /// * `count` - Total number of workers.
    pub fn worker(self, index: i32, count: i32) -> ChainedIter<'a, P, T> {
        ChainedIter::new_worker(self, index, count)
    }
}

/// An iterator that wraps a [`QueryIter`] with page or worker constraints.
///
/// Zero-allocation design: stores the parent `ecs_iter_t` inline (no heap).
///
/// The parent is wrapped in [`core::cell::UnsafeCell`] because `ecs_page_next`/`ecs_worker_next`
/// mutate the parent through the `chain_it` raw pointer stored in the child iter.
/// `UnsafeCell` is the correct primitive for this: it grants interior mutability
/// through a raw pointer while the struct is borrowed via `&self`.
///
/// Safety invariant: `retrieve_iter` must not be called while a previously returned
/// child iter is still alive (i.e. after `run()`/`each()` returns is fine).
/// The `QueryAPI` methods all call `retrieve_iter` exactly once per invocation,
/// so this invariant is upheld by construction.
pub struct ChainedIter<'a, P, T>
where
    T: QueryTuple,
{
    /// Parent iter — wrapped in `UnsafeCell` so C can mutate it through `chain_it`.
    /// Must not be moved after `retrieve_iter` is called (enforced by `&self` borrow).
    parent: core::cell::UnsafeCell<sys::ecs_iter_t>,
    param1: i32,
    param2: i32,
    make_iter: unsafe extern "C-unwind" fn(*const sys::ecs_iter_t, i32, i32) -> sys::ecs_iter_t,
    iter_next: ExternIterNextFn,
    consumed: core::cell::Cell<bool>,
    _phantom: core::marker::PhantomData<&'a (P, T)>,
}

impl<'a, P, T> ChainedIter<'a, P, T>
where
    T: QueryTuple,
{
    fn new_page(parent: QueryIter<'a, P, T>, offset: i32, limit: i32) -> Self {
        Self {
            parent: core::cell::UnsafeCell::new(parent.take_iter()),
            param1: offset,
            param2: limit,
            make_iter: sys::ecs_page_iter,
            iter_next: sys::ecs_page_next,
            consumed: core::cell::Cell::new(false),
            _phantom: core::marker::PhantomData,
        }
    }

    fn new_worker(parent: QueryIter<'a, P, T>, index: i32, count: i32) -> Self {
        Self {
            parent: core::cell::UnsafeCell::new(parent.take_iter()),
            param1: index,
            param2: count,
            make_iter: sys::ecs_worker_iter,
            iter_next: sys::ecs_worker_next,
            consumed: core::cell::Cell::new(false),
            _phantom: core::marker::PhantomData,
        }
    }
}

impl<P, T> Drop for ChainedIter<'_, P, T>
where
    T: QueryTuple,
{
    fn drop(&mut self) {
        if !self.consumed.get() {
            // SAFETY: the parent iterator's resources were transferred into this
            // struct on construction and no child iterator was created, so the
            // parent is still pristine and must be released here.
            unsafe { sys::ecs_iter_fini(self.parent.get_mut()) };
        }
    }
}

#[doc(hidden)]
impl<P, T> IterOperations for ChainedIter<'_, P, T>
where
    T: QueryTuple,
{
    #[inline(always)]
    fn retrieve_iter(&self) -> sys::ecs_iter_t {
        assert!(
            !self.consumed.get(),
            "ChainedIter already consumed: iterating a page/worker iterator releases the \
             underlying flecs iterator. Create a new one with `query.iterable().page()/.worker()`."
        );
        self.consumed.set(true);
        // SAFETY: &self is borrowed for the entire run()/each() call, so self.parent
        // cannot move. The child iter stores chain_it = self.parent.get(), which
        // remains valid and exclusively owned (no other &mut exists) for the duration.
        // The consumed flag guarantees a single child per parent, so the parent's
        // resources are released exactly once (by the child's fini/completion).
        unsafe { (self.make_iter)(self.parent.get() as *const _, self.param1, self.param2) }
    }

    fn retrieve_iter_stage<'w>(&self, _stage: impl WorldProvider<'w>) -> sys::ecs_iter_t {
        panic!("Cannot change stage on a chained (page/worker) iterator.");
    }

    #[inline(always)]
    fn iter_next(&self, iter: &mut sys::ecs_iter_t) -> bool {
        unsafe { (self.iter_next)(iter) }
    }

    #[inline(always)]
    fn query_ptr(&self) -> *const sys::ecs_query_t {
        unsafe { (*self.parent.get()).query }
    }

    #[inline(always)]
    fn iter_next_func(&self) -> ExternIterNextFn {
        self.iter_next
    }
}

impl<'a, P, T> QueryAPI<'a, P, T> for ChainedIter<'a, P, T>
where
    T: QueryTuple,
    Self: WorldProvider<'a>,
{
    fn entity(&self) -> EntityView<'_> {
        let world = unsafe { WorldRef::from_ptr((*self.parent.get()).real_world) };
        EntityView::new_from(world, unsafe {
            sys::ecs_get_entity((*self.parent.get()).query as *const core::ffi::c_void)
        })
    }
}

/// Formats the query as a string expression using `ecs_query_str`.
/// The resulting expression can be parsed to create the same query.
impl<P, T> core::fmt::Display for ChainedIter<'_, P, T>
where
    T: QueryTuple,
{
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{}", query_expr_string(IterOperations::query_ptr(self)))
    }
}

impl<'a, P, T> WorldProvider<'a> for ChainedIter<'a, P, T>
where
    T: QueryTuple,
{
    fn world(&self) -> WorldRef<'a> {
        unsafe { WorldRef::from_ptr((*self.parent.get()).world) }
    }
}
