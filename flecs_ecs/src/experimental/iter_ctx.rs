//! Lightweight per-invocation iteration context (`Iter`, spec §5.6).
//!
//! [`Iter`] is the typed context object the `each_iter` terminals hand to the
//! callback in place of the old `each_iter(|it, index, item|)` triple. It
//! exposes the running iteration's timestep, row count, and per-row entity.
//!
//! It is a **stack struct** built once per table batch and handed to the
//! callback **by reference** per row (`FnMut(&Iter, D::Item)`): there is no
//! per-batch RAII drop and no allocation, so the row loop stays at parity with
//! the plain `each`. `Iter` borrows the running iteration for `'a` and is
//! `!Send`/`!Sync`.
//!
//! This wave ships the **query** terminals only (`each_iter` /
//! `each_iter_shared`); the system-builder and observer terminals arrive with
//! the surface swap. [`Iter`] already carries the observer event metadata of
//! spec §5.6 ([`event`](Iter::event), [`event_id`](Iter::event_id),
//! [`pair`](Iter::pair)) so those terminals can hand it out unchanged.

use core::marker::PhantomData;

use crate::core::{
    ComponentPointers, EntityView, Id, IterGuard, QueryAPI, QueryTuple, World, WorldProvider,
    WorldRef,
};
use crate::sys;

#[cfg(feature = "flecs_safety_locks")]
use crate::core::{acquire_batch_locks, release_batch_locks};

/// The lightweight per-invocation iteration context (spec §5.6), built once per
/// table batch and passed to the callback by reference.
///
/// Exposes the timestep (`delta_time` / `delta_system_time`), the batch row
/// count, and the entity for a given row. `!Send`/`!Sync`; borrows the running
/// iteration for `'a`.
pub struct Iter<'a> {
    iter: *const sys::ecs_iter_t,
    world: WorldRef<'a>,
    _marker: PhantomData<&'a sys::ecs_iter_t>,
}

impl<'a> Iter<'a> {
    #[inline(always)]
    fn new(iter: &sys::ecs_iter_t, world: WorldRef<'a>) -> Self {
        Iter {
            iter: iter as *const sys::ecs_iter_t,
            world,
            _marker: PhantomData,
        }
    }

    /// Time elapsed since the last frame (`ecs_iter_t::delta_time`).
    #[inline(always)]
    pub fn delta_time(&self) -> f32 {
        // SAFETY: `iter` points to the live iterator this context borrows.
        unsafe { (*self.iter).delta_time }
    }

    /// Time elapsed since this system last ran (`ecs_iter_t::delta_system_time`).
    #[inline(always)]
    pub fn delta_system_time(&self) -> f32 {
        // SAFETY: as `delta_time`.
        unsafe { (*self.iter).delta_system_time }
    }

    /// Number of rows in the current batch.
    #[inline(always)]
    pub fn count(&self) -> usize {
        // SAFETY: as `delta_time`.
        unsafe { (*self.iter).count as usize }
    }

    /// The entity for row `row` of the current batch.
    ///
    /// # Panics
    /// If `row >= self.count()`, or if this batch carries no entities (a
    /// query/system whose `$this` variable is not populated).
    #[inline]
    #[track_caller]
    pub fn entity(&self, row: usize) -> EntityView<'a> {
        // SAFETY: `iter` points to the live iterator this context borrows.
        let iter = unsafe { &*self.iter };
        assert!(
            row < iter.count as usize,
            "Iter::entity row {row} out of bounds for batch count {}",
            iter.count
        );
        assert!(
            !iter.entities.is_null(),
            "Iter::entity called on an iteration that does not return entities"
        );
        // SAFETY: row < count and entities has count valid entries.
        EntityView::new_from(self.world, unsafe { *iter.entities.add(row) })
    }

    /// The event being dispatched (spec §5.6). **Observer-only**: meaningful
    /// inside an observer callback, where it names the event entity
    /// (`flecs::OnAdd`, `flecs::OnSet`, a custom event, ..). Outside an
    /// observer invocation the iterator carries no event and the returned view
    /// wraps the zero id (`view.id() == 0`).
    #[inline(always)]
    pub fn event(&self) -> EntityView<'a> {
        // SAFETY: `iter` points to the live iterator this context borrows.
        EntityView::new_from(self.world, unsafe { (*self.iter).event })
    }

    /// The (component) id the event was emitted for (spec §5.6).
    /// **Observer-only**: meaningful inside an observer callback. Outside an
    /// observer invocation the iterator carries no event id and this returns
    /// the zero [`Id`].
    #[inline(always)]
    pub fn event_id(&self) -> Id {
        // SAFETY: as `event`.
        Id::new(unsafe { (*self.iter).event_id })
    }

    /// The id matched for field `index` when that id is a pair, else `None`
    /// (spec §5.6). Valid in any iteration, not only observers: for a wildcard
    /// pair term this is the concrete pair matched for the current batch.
    ///
    /// # Panics
    /// In debug builds, if `index` is not a valid field index for this
    /// iteration.
    #[inline]
    pub fn pair(&self, index: i8) -> Option<Id> {
        // SAFETY: `iter` points to the live iterator this context borrows;
        // ecs_field_id validates `index` against the iterator in debug builds.
        let id = unsafe { sys::ecs_field_id(self.iter, index) };
        if unsafe { sys::ecs_id_is_pair(id) } {
            Some(Id::new(id))
        } else {
            None
        }
    }
}

/// Run one table batch: build the [`Iter`] context once, then hand it to the
/// callback by reference for every row. Registers Tier-1 batch locks around the
/// batch (shared-register semantics; the exclusive terminal uses the same path
/// for near-parity — see the trait docs).
#[inline(always)]
fn each_iter_batch<T: QueryTuple, const ANY_SPARSE_TERMS: bool>(
    iter: &mut sys::ecs_iter_t,
    world: &WorldRef<'_>,
    func: &mut impl FnMut(&Iter, T::TupleType<'_>),
) {
    iter.flags |= sys::EcsIterCppEach;
    let (is_any_array, mut components_data) = T::create_ptrs(iter);
    let count = if iter.count == 0 && iter.table.is_null() {
        1_usize
    } else {
        iter.count as usize
    };

    #[cfg(feature = "flecs_safety_locks")]
    let (__locks, __token) =
        acquire_batch_locks::<ANY_SPARSE_TERMS, T>(world, components_data.safety_table_records());

    let ctx = Iter::new(iter, *world);
    if !is_any_array.a_ref && !is_any_array.a_row {
        for i in 0..count {
            let tuple = components_data.get_tuple(i);
            func(&ctx, tuple);
        }
    } else if is_any_array.a_row {
        for i in 0..count {
            let tuple = components_data.get_tuple_with_row(iter, i);
            func(&ctx, tuple);
        }
    } else {
        for i in 0..count {
            let tuple = components_data.get_tuple_with_ref(i);
            func(&ctx, tuple);
        }
    }

    #[cfg(feature = "flecs_safety_locks")]
    release_batch_locks::<ANY_SPARSE_TERMS, T>(
        world,
        __locks,
        __token,
        components_data.safety_table_records(),
    );
}

#[inline(always)]
fn run_each_iter<'a, P, T, Q>(query: &Q, mut func: impl FnMut(&Iter, T::TupleType<'_>))
where
    Q: QueryAPI<'a, P, T>,
    T: QueryTuple,
{
    const {
        assert!(
            !T::CONTAINS_ANY_TAG_TERM,
            "a type provided in the query signature is a Tag and cannot be used with \
             `each_iter`. use `run` instead or provide the tag with `.with()`"
        );
    }

    let world = query.world();
    let mut iter = IterGuard::new(query.retrieve_iter());

    // ecs_query_iter leaves the timestep zero for a standalone query, so seed it
    // from the world's frame info; inside a system flecs sets these itself.
    // SAFETY: world.world_ptr() is a live world pointer.
    let info = unsafe { &*sys::ecs_get_world_info(world.world_ptr()) };
    iter.delta_time = info.delta_time;
    iter.delta_system_time = info.delta_time;

    #[cfg(not(feature = "flecs_safety_locks"))]
    {
        while iter.next(|i| query.iter_next(i)) {
            each_iter_batch::<T, false>(&mut iter, &world, &mut func);
        }
    }
    #[cfg(feature = "flecs_safety_locks")]
    {
        if iter.row_fields == 0 {
            while iter.next(|i| query.iter_next(i)) {
                each_iter_batch::<T, false>(&mut iter, &world, &mut func);
            }
        } else {
            while iter.next(|i| query.iter_next(i)) {
                each_iter_batch::<T, true>(&mut iter, &world, &mut func);
            }
        }
    }
}

/// Query iteration carrying the lightweight [`Iter`] context (spec §5.6).
///
/// Both terminals build the context once per batch and pass it to the callback
/// by reference; neither allocates nor installs a per-batch RAII drop. The two
/// register the same Tier-1 batch locks, so `each_iter` (exclusive register,
/// `&mut World`) and `each_iter_shared` (shared register, `&World`) differ only
/// in which world borrow the caller proves — mirroring `each` / `each_shared`.
pub trait QueryIterCtxExt<'a, P, T>: QueryAPI<'a, P, T>
where
    T: QueryTuple,
{
    /// Exclusive register: iterate with the per-row [`Iter`] context.
    ///
    /// # Panics
    /// If the `&mut World` belongs to a different world than the query.
    fn each_iter(&self, world: &mut World, func: impl FnMut(&Iter, T::TupleType<'_>));

    /// Shared register: iterate with the per-row [`Iter`] context while other
    /// shared borrows of the world are live.
    ///
    /// # Panics
    /// If the `&World` belongs to a different world than the query.
    fn each_iter_shared(&self, world: &World, func: impl FnMut(&Iter, T::TupleType<'_>));
}

impl<'a, P, T, Q> QueryIterCtxExt<'a, P, T> for Q
where
    Q: QueryAPI<'a, P, T>,
    T: QueryTuple,
{
    #[inline]
    #[track_caller]
    fn each_iter(&self, world: &mut World, func: impl FnMut(&Iter, T::TupleType<'_>)) {
        assert!(
            core::ptr::eq(
                unsafe { (*self.query_ptr()).real_world },
                (&*world).world_ptr()
            ),
            "each_iter requires the query's own world: the &mut World passed in belongs to a \
             different world than this query's storage"
        );
        run_each_iter::<P, T, Q>(self, func);
    }

    #[inline]
    #[track_caller]
    fn each_iter_shared(&self, world: &World, func: impl FnMut(&Iter, T::TupleType<'_>)) {
        assert!(
            core::ptr::eq(unsafe { (*self.query_ptr()).real_world }, world.world_ptr()),
            "each_iter_shared requires the query's own world: the &World passed in belongs to a \
             different world than this query's storage"
        );
        run_each_iter::<P, T, Q>(self, func);
    }
}
