mod id_operations;
mod inout_oper;
mod into_component_id;
mod into_entity;
mod into_id;
mod into_table;
mod query_api;
mod system_api;
mod world_provider;

pub use id_operations::*;
pub use inout_oper::*;
pub use into_component_id::*;
pub use into_entity::*;
pub use into_id::*;
pub use into_table::*;
pub use query_api::*;
pub use system_api::*;
pub use world_provider::*;

use crate::core::{
    ComponentId, ComponentPointers, EntityView, FieldIndex, ImplementsClone, ImplementsDefault,
    ImplementsPartialEq, ImplementsPartialOrd, ImplementsSend, ImplementsSync, QueryTuple,
    TableIter, ecs_assert,
};
#[cfg(feature = "flecs_safety_locks")]
use crate::core::{acquire_batch_locks, release_batch_locks};
#[cfg(any(debug_assertions, feature = "flecs_force_enable_ecs_asserts"))]
use crate::core::{FlecsErrorCode, IterTableLock};
use crate::sys;

#[doc(hidden)]
pub mod private {

    use crate::core::*;
    use crate::sys;
    use core::ffi::c_void;

    #[cfg(feature = "std")]
    extern crate std;

    extern crate alloc;
    use alloc::boxed::Box;
    use flecs_ecs_derive::extern_abi;

    /// Run one callback-trampoline body under the catch-stash-rethrow policy
    /// (spec §5.5, Decision 8).
    ///
    /// A panic in the user callback is caught here rather than allowed to unwind
    /// through the flecs C frame that called this trampoline. On catch the stage
    /// borrow map is rolled back to its pre-invocation snapshot (query batches
    /// take and release their term borrows with plain calls, so a panicking
    /// callback would otherwise leak them, spec §5.2), the payload is stashed in
    /// the world context, and control returns to C normally; the enclosing safe
    /// entry point rethrows it once flecs has returned.
    ///
    /// Once a panic is stashed the frame is poisoned: later trampoline
    /// invocations in the same frame (remaining tables of a per-table callback,
    /// or later systems/observers in the pipeline) short-circuit here without
    /// running any user code, leaving flecs to drive its iterator/pipeline to
    /// the end so its own frames unwind cleanly.
    ///
    /// `AssertUnwindSafe` is justified once for the whole trampoline family:
    /// every shared invariant the body can touch is restored before returning
    /// to C. The stage locks are restored by the snapshot below; the debug
    /// `ecs_table_lock` a run-loop trampoline holds on the in-flight table and
    /// the iterator itself are balanced by the finalization below; any lazily
    /// opened defer level is balanced by its guard's `Drop`, which runs during
    /// the unwind before this catch. No caller observes a torn state, so
    /// treating the borrowed iterator/world state the body captures as
    /// unwind-safe is sound.
    ///
    /// The finalization obligations differ across the trampoline family because
    /// flecs does not finalize the iterator of a `run` callback after it returns
    /// (it expects the callback to iterate to completion, which auto-finalizes
    /// via `ecs_query_next`). Any early exit from a `run` trampoline must call
    /// `ecs_iter_fini` itself or the stage's stack-allocator cursor leaks and
    /// aborts at teardown. Two flags capture the two early exits, because the
    /// iterator state that makes finalization safe differs between them:
    ///
    /// * `fini_on_shortcircuit` — a poisoned-frame short-circuit runs no body,
    ///   so the iterator is exactly as flecs created it (fresh, live) and is
    ///   finalized unconditionally. True for every `run` trampoline
    ///   (`execute_run`, `execute_run_each*`); this is what releases the sibling
    ///   `par_*` workers whose `run` starts after one worker has stashed a panic.
    ///
    /// * `fini_on_panic` — a caught panic finalizes the in-flight iterator
    ///   (releasing the flecs per-table debug lock first, mirroring
    ///   `TableIter::fini`). Only the trampolines that drive their own
    ///   `while table_iter.internal_next()` loop (`execute_run_each*`) set this:
    ///   there a caught panic is always mid-batch, so the iterator is live and
    ///   `ecs_iter_fini` runs exactly once. The raw `execute_run` leaves it
    ///   `false`: its user closure owns iteration and may already have run the
    ///   loop to completion (which finalized the iterator while
    ///   `TableIter::internal_next` left `EcsIterIsValid` set) or called `fini`
    ///   itself, so finalizing again would double-free.
    ///
    /// The per-table callback trampolines (`execute_each*`) set both `false`:
    /// they are driven by `it.each()` or flecs' default-run loop, both of which
    /// own and finalize the iterator.
    #[inline]
    pub(crate) fn run_callback_trampoline(
        world: &WorldRef<'_>,
        iter_ptr: *mut sys::ecs_iter_t,
        fini_on_shortcircuit: bool,
        fini_on_panic: bool,
        body: impl FnOnce(),
    ) {
        let ctx = world.world_ctx();
        if ctx.frame_panicked() {
            if fini_on_shortcircuit {
                unsafe { sys::ecs_iter_fini(iter_ptr) };
            }
            return;
        }
        #[cfg(feature = "flecs_safety_locks")]
        let scope = crate::core::StageLocksScope::new(world);
        match std::panic::catch_unwind(core::panic::AssertUnwindSafe(body)) {
            Ok(()) => {}
            Err(payload) => {
                #[cfg(feature = "flecs_safety_locks")]
                scope.restore();
                if fini_on_panic {
                    let iter = unsafe { &*iter_ptr };
                    if iter.flags & sys::EcsIterIsValid != 0 {
                        #[cfg(any(
                            debug_assertions,
                            feature = "flecs_force_enable_ecs_asserts"
                        ))]
                        if !iter.table.is_null() {
                            unsafe { sys::ecs_table_unlock(iter.world, iter.table) };
                        }
                        unsafe { sys::ecs_iter_fini(iter_ptr) };
                    }
                }
                ctx.stash_panic(payload);
            }
        }
    }

    #[allow(non_camel_case_types)]
    #[doc(hidden)]
    pub trait internal_SystemAPI<'a, P, T>
    where
        T: QueryTuple,
        P: ComponentId,
    {
        fn set_callback_binding_context(&mut self, binding_ctx: *mut c_void) -> &mut Self;

        fn set_callback_binding_context_free(
            &mut self,
            binding_ctx_free: sys::ecs_ctx_free_t,
        ) -> &mut Self;

        fn set_run_binding_context(&mut self, binding_ctx: *mut c_void) -> &mut Self;

        fn set_run_binding_context_free(
            &mut self,
            run_ctx_free: flecs_ecs_sys::ecs_ctx_free_t,
        ) -> &mut Self;

        fn desc_binding_context(&self) -> *mut c_void;

        fn set_desc_callback(&mut self, callback: Option<ExternIterFn>);

        fn set_desc_run(&mut self, callback: Option<ExternIterFn>);

        /// Callback of the each functionality
        ///
        /// # Arguments
        ///
        /// * `iter` - The iterator which gets passed in from `C`
        #[allow(clippy::not_unsafe_ptr_arg_deref, reason = "iter will always be valid")]
        #[extern_abi]
        fn execute_each<const CALLED_FROM_RUN: bool, Func>(iter: *mut sys::ecs_iter_t)
        where
            Func: FnMut(T::TupleType<'_>),
        {
            let iter_ptr = iter;
            let iter = unsafe { &mut *iter_ptr };
            let world = unsafe { WorldRef::from_ptr(iter.world) };
            run_callback_trampoline(&world, iter_ptr, false, false, || unsafe {
                let each = &mut *(iter.callback_ctx as *mut Func);
                #[cfg(feature = "flecs_safety_locks")]
                if iter.row_fields == 0 {
                    internal_each_iter_next::<T, CALLED_FROM_RUN, false>(iter, &world, each);
                } else {
                    internal_each_iter_next::<T, CALLED_FROM_RUN, true>(iter, &world, each);
                }

                #[cfg(not(feature = "flecs_safety_locks"))]
                {
                    internal_each_iter_next::<T, CALLED_FROM_RUN, false>(iter, &world, each);
                }
            });
        }

        /// Callback of the `each_entity` functionality
        ///
        /// # Arguments
        ///
        /// * `iter` - The iterator which gets passed in from `C`
        #[allow(clippy::not_unsafe_ptr_arg_deref, reason = "iter will always be valid")]
        #[extern_abi]
        fn execute_each_entity<const CALLED_FROM_RUN: bool, Func>(iter: *mut sys::ecs_iter_t)
        where
            Func: FnMut(EntityView, T::TupleType<'_>),
        {
            let iter_ptr = iter;
            let iter = unsafe { &mut *iter_ptr };
            let world = unsafe { WorldRef::from_ptr(iter.world) };
            run_callback_trampoline(&world, iter_ptr, false, false, || unsafe {
                let each_entity = &mut *(iter.callback_ctx as *mut Func);
                #[cfg(feature = "flecs_safety_locks")]
                if iter.row_fields == 0 {
                    internal_each_entity_iter_next::<T, CALLED_FROM_RUN, false>(
                        iter,
                        &world,
                        each_entity,
                    );
                } else {
                    internal_each_entity_iter_next::<T, CALLED_FROM_RUN, true>(
                        iter,
                        &world,
                        each_entity,
                    );
                }

                #[cfg(not(feature = "flecs_safety_locks"))]
                {
                    internal_each_entity_iter_next::<T, CALLED_FROM_RUN, false>(
                        iter,
                        &world,
                        each_entity,
                    );
                }
            });
        }

        #[allow(clippy::not_unsafe_ptr_arg_deref, reason = "iter will always be valid")]
        #[extern_abi]
        fn execute_each_iter<const CALLED_FROM_RUN: bool, Func>(iter: *mut sys::ecs_iter_t)
        where
            Func: FnMut(TableIter<CALLED_FROM_RUN, P>, FieldIndex, T::TupleType<'_>),
        {
            let iter_ptr = iter;
            let iter = unsafe { &mut *iter_ptr };
            let world = unsafe { WorldRef::from_ptr(iter.world) };
            run_callback_trampoline(&world, iter_ptr, false, false, || unsafe {
                let each_iter = &mut *(iter.callback_ctx as *mut Func);
                #[cfg(feature = "flecs_safety_locks")]
                if iter.row_fields == 0 {
                    internal_each_iter::<T, P, CALLED_FROM_RUN, false>(iter, &world, each_iter);
                } else {
                    internal_each_iter::<T, P, CALLED_FROM_RUN, true>(iter, &world, each_iter);
                }

                #[cfg(not(feature = "flecs_safety_locks"))]
                {
                    internal_each_iter::<T, P, CALLED_FROM_RUN, false>(iter, &world, each_iter);
                }
            });
        }

        /// Callback of the `iter_only` functionality
        ///
        /// # Arguments
        ///
        /// * `iter` - The iterator which gets passed in from `C`
        #[allow(clippy::not_unsafe_ptr_arg_deref, reason = "iter will always be valid")]
        #[extern_abi]
        fn execute_run<Func>(iter: *mut sys::ecs_iter_t)
        where
            Func: FnMut(TableIter<true, P>),
        {
            let iter_ptr = iter;
            let iter = unsafe { &mut *iter_ptr };
            let world = unsafe { WorldRef::from_ptr(iter.world) };
            run_callback_trampoline(&world, iter_ptr, true, false, || unsafe {
                let run = &mut *iter.run_ctx.cast::<Func>();
                internal_run::<P>(iter, run, world);
            });
        }

        #[extern_abi]
        fn free_callback<Func>(ptr: *mut c_void) {
            unsafe {
                let ptr = ptr.cast::<Func>();
                drop(Box::from_raw(ptr));
            };
        }

        #[allow(clippy::not_unsafe_ptr_arg_deref, reason = "iter will always be valid")]
        #[extern_abi]
        fn execute_run_each<Func, const CHECKED: bool>(iter: *mut sys::ecs_iter_t)
        where
            Func: FnMut(T::TupleType<'_>),
        {
            let iter_ptr = iter;
            let iter = unsafe { &mut *iter_ptr };
            iter.flags &= !sys::EcsIterIsValid;
            let world = unsafe { WorldRef::from_ptr(iter.world) };
            run_callback_trampoline(&world, iter_ptr, true, true, || unsafe {
                let each = &mut *iter.run_ctx.cast::<Func>();
                let mut table_iter = TableIter::<true, ()>::new(iter, world);

                #[cfg(feature = "flecs_safety_locks")]
                if CHECKED {
                    if table_iter.iter.row_fields == 0 {
                        while table_iter.internal_next() {
                            internal_each_iter_next::<T, true, false>(
                                table_iter.iter,
                                &world,
                                each,
                            );
                        }
                    } else {
                        while table_iter.internal_next() {
                            internal_each_iter_next::<T, true, true>(table_iter.iter, &world, each);
                        }
                    }
                } else {
                    // Unchecked: always use false for sparse terms check
                    while table_iter.internal_next() {
                        internal_each_iter_next::<T, true, false>(table_iter.iter, &world, each);
                    }
                }

                #[cfg(not(feature = "flecs_safety_locks"))]
                {
                    while table_iter.internal_next() {
                        internal_each_iter_next::<T, true, false>(table_iter.iter, &world, each);
                    }
                }
            });
        }

        #[allow(clippy::not_unsafe_ptr_arg_deref, reason = "iter will always be valid")]
        #[extern_abi]
        fn execute_run_each_entity<Func, const CHECKED: bool>(iter: *mut sys::ecs_iter_t)
        where
            Func: FnMut(EntityView, T::TupleType<'_>),
        {
            let iter_ptr = iter;
            let iter = unsafe { &mut *iter_ptr };
            iter.flags &= !sys::EcsIterIsValid;
            let world = unsafe { WorldRef::from_ptr(iter.world) };
            run_callback_trampoline(&world, iter_ptr, true, true, || unsafe {
                let each_entity = &mut *iter.run_ctx.cast::<Func>();
                let mut table_iter = TableIter::<true, ()>::new(iter, world);

                #[cfg(feature = "flecs_safety_locks")]
                if CHECKED {
                    if table_iter.iter.row_fields == 0 {
                        while table_iter.internal_next() {
                            internal_each_entity_iter_next::<T, true, false>(
                                table_iter.iter,
                                &world,
                                each_entity,
                            );
                        }
                    } else {
                        while table_iter.internal_next() {
                            internal_each_entity_iter_next::<T, true, true>(
                                table_iter.iter,
                                &world,
                                each_entity,
                            );
                        }
                    }
                } else {
                    // Unchecked: always use false for sparse terms check
                    while table_iter.internal_next() {
                        internal_each_entity_iter_next::<T, true, false>(
                            table_iter.iter,
                            &world,
                            each_entity,
                        );
                    }
                }

                #[cfg(not(feature = "flecs_safety_locks"))]
                {
                    while table_iter.internal_next() {
                        internal_each_entity_iter_next::<T, true, false>(
                            table_iter.iter,
                            &world,
                            each_entity,
                        );
                    }
                }
            });
        }

        #[allow(
            clippy::not_unsafe_ptr_arg_deref,
            reason = "this doesn't actually deref the pointer"
        )]
        #[extern_abi]
        fn execute_run_each_iter<Func, const CHECKED: bool>(iter: *mut sys::ecs_iter_t)
        where
            Func: FnMut(TableIter<false, P>, FieldIndex, T::TupleType<'_>) + 'static,
        {
            let iter_ptr = iter;
            let iter = unsafe { &mut *iter_ptr };
            iter.flags &= !sys::EcsIterIsValid;
            let world = unsafe { WorldRef::from_ptr(iter.world) };
            run_callback_trampoline(&world, iter_ptr, true, true, || unsafe {
                let each_iter = &mut *iter.run_ctx.cast::<Func>();
                let mut table_iter = TableIter::<true, ()>::new(iter, world);

                #[cfg(feature = "flecs_safety_locks")]
                if CHECKED {
                    if table_iter.iter.row_fields == 0 {
                        while table_iter.internal_next() {
                            internal_each_iter::<T, P, false, false>(
                                table_iter.iter,
                                &world,
                                each_iter,
                            );
                        }
                    } else {
                        while table_iter.internal_next() {
                            internal_each_iter::<T, P, false, true>(
                                table_iter.iter,
                                &world,
                                each_iter,
                            );
                        }
                    }
                } else {
                    // Unchecked: always use false for sparse terms check
                    while table_iter.internal_next() {
                        internal_each_iter::<T, P, false, false>(
                            table_iter.iter,
                            &world,
                            each_iter,
                        );
                    }
                }

                #[cfg(not(feature = "flecs_safety_locks"))]
                {
                    while table_iter.internal_next() {
                        internal_each_iter::<T, P, false, false>(
                            table_iter.iter,
                            &world,
                            each_iter,
                        );
                    }
                }
            });
        }

        /// Trampoline for `each_with`: iterate every matched row, handing the
        /// callback the row's component tuple and a per-callback
        /// [`Stage`](crate::experimental::Stage) built from the iterator's stage
        /// world. Always routes through the checked batch-lock path
        /// ([`internal_each_iter_next`]); the stage handle can reach arbitrary
        /// storage, so a `_with` system never takes the lock-free path (spec
        /// §5.2).
        #[cfg(feature = "flecs_experimental")]
        #[allow(clippy::not_unsafe_ptr_arg_deref, reason = "iter will always be valid")]
        #[extern_abi]
        fn execute_run_each_with<Func>(iter: *mut sys::ecs_iter_t)
        where
            Func: FnMut(T::TupleType<'_>, crate::experimental::Stage<'_>),
        {
            let iter_ptr = iter;
            let iter = unsafe { &mut *iter_ptr };
            iter.flags &= !sys::EcsIterIsValid;
            let world = unsafe { WorldRef::from_ptr(iter.world) };
            run_callback_trampoline(&world, iter_ptr, true, true, || unsafe {
                let each = &mut *iter.run_ctx.cast::<Func>();
                let mut table_iter = TableIter::<true, ()>::new(iter, world);

                #[cfg(feature = "flecs_safety_locks")]
                if table_iter.iter.row_fields == 0 {
                    while table_iter.internal_next() {
                        let dt = table_iter.iter.delta_time;
                        internal_each_iter_next::<T, true, false>(
                            table_iter.iter,
                            &world,
                            &mut |t| each(t, crate::experimental::Stage::new(world, dt)),
                        );
                    }
                } else {
                    while table_iter.internal_next() {
                        let dt = table_iter.iter.delta_time;
                        internal_each_iter_next::<T, true, true>(
                            table_iter.iter,
                            &world,
                            &mut |t| each(t, crate::experimental::Stage::new(world, dt)),
                        );
                    }
                }

                #[cfg(not(feature = "flecs_safety_locks"))]
                while table_iter.internal_next() {
                    let dt = table_iter.iter.delta_time;
                    internal_each_iter_next::<T, true, false>(table_iter.iter, &world, &mut |t| {
                        each(t, crate::experimental::Stage::new(world, dt))
                    });
                }
            });
        }

        /// Trampoline for `each_entity_with`: as [`execute_run_each_with`] but the
        /// callback also receives the row's [`EntityView`], for the "visit each
        /// matched entity and issue a deferred command on it" shape (spec §5.2,
        /// GAP-6).
        #[cfg(feature = "flecs_experimental")]
        #[allow(clippy::not_unsafe_ptr_arg_deref, reason = "iter will always be valid")]
        #[extern_abi]
        fn execute_run_each_entity_with<Func>(iter: *mut sys::ecs_iter_t)
        where
            Func: FnMut(EntityView, T::TupleType<'_>, crate::experimental::Stage<'_>),
        {
            let iter_ptr = iter;
            let iter = unsafe { &mut *iter_ptr };
            iter.flags &= !sys::EcsIterIsValid;
            let world = unsafe { WorldRef::from_ptr(iter.world) };
            run_callback_trampoline(&world, iter_ptr, true, true, || unsafe {
                let each = &mut *iter.run_ctx.cast::<Func>();
                let mut table_iter = TableIter::<true, ()>::new(iter, world);

                #[cfg(feature = "flecs_safety_locks")]
                if table_iter.iter.row_fields == 0 {
                    while table_iter.internal_next() {
                        let dt = table_iter.iter.delta_time;
                        internal_each_entity_iter_next::<T, true, false>(
                            table_iter.iter,
                            &world,
                            &mut |e, t| each(e, t, crate::experimental::Stage::new(world, dt)),
                        );
                    }
                } else {
                    while table_iter.internal_next() {
                        let dt = table_iter.iter.delta_time;
                        internal_each_entity_iter_next::<T, true, true>(
                            table_iter.iter,
                            &world,
                            &mut |e, t| each(e, t, crate::experimental::Stage::new(world, dt)),
                        );
                    }
                }

                #[cfg(not(feature = "flecs_safety_locks"))]
                while table_iter.internal_next() {
                    let dt = table_iter.iter.delta_time;
                    internal_each_entity_iter_next::<T, true, false>(
                        table_iter.iter,
                        &world,
                        &mut |e, t| each(e, t, crate::experimental::Stage::new(world, dt)),
                    );
                }
            });
        }

        // /// Get the binding context
        // fn get_binding_context(&mut self, is_run: bool) -> &mut ReactorBindingType {
        //     let mut binding_ctx: *mut ReactorBindingType = self.desc_binding_context() as *mut _;

        //     if binding_ctx.is_null() {
        //         let new_binding_ctx = Box::<ReactorBindingType>::default();
        //         let static_ref = Box::leak(new_binding_ctx);
        //         binding_ctx = static_ref;
        //         if is_run {
        //             self.set_run_binding_context(binding_ctx as *mut c_void);
        //             self.set_run_binding_context_free(Some(Self::binding_ctx_drop));
        //         } else {
        //             self.set_callback_binding_context(binding_ctx as *mut c_void);
        //             self.set_callback_binding_context_free(Some(Self::binding_ctx_drop));
        //         }
        //     }
        //     unsafe { &mut *binding_ctx }
        // }

        // /// drop the binding context
        // extern "C-unwind" fn binding_ctx_drop(ptr: *mut c_void) {
        //     let ptr_struct: *mut ReactorBindingType = ptr as *mut ReactorBindingType;
        //     unsafe {
        //         ptr::drop_in_place(ptr_struct);
        //     }
        // }
    }

    #[allow(non_camel_case_types)]
    #[doc(hidden)]
    pub trait internal_ParSystemAPI<'a, P, T>: internal_SystemAPI<'a, P, T>
    where
        T: QueryTuple,
        P: ComponentId,
    {
        fn set_multi_threaded(&mut self, multi_threaded: bool);
    }
}

pub trait DoesNotImpl {
    const IMPLS: bool = false;
}

impl<T> DoesNotImpl for T {}

impl<T: Clone> ImplementsClone<T> {
    pub const IMPLS: bool = true;
}

impl<T: Default> ImplementsDefault<T> {
    pub const IMPLS: bool = true;
}

impl<T: PartialEq> ImplementsPartialEq<T> {
    pub const IMPLS: bool = true;
}

impl<T: PartialOrd> ImplementsPartialOrd<T> {
    pub const IMPLS: bool = true;
}

impl<T: Send> ImplementsSend<T> {
    pub const IMPLS: bool = true;
}

impl<T: Sync> ImplementsSync<T> {
    pub const IMPLS: bool = true;
}

pub trait FlecsConstantId {
    const ID: u64;
}
/// A little “extractor” trait: given the iterator and an index,
/// produce whatever you want to pass as the first argument to the user‑closure.
pub(crate) trait EntityExtractor {
    const CONTAINS_ENTITY: bool;
    /// The type of the “extra” first parameter to the callback.
    type Output;

    /// Called inside the per‑element loop to build that argument.
    unsafe fn extract(&self, iter: &sys::ecs_iter_t, idx: usize) -> Self::Output;
}

/// When you don’t want to pass an entity, use this ZST.
pub(crate) struct NoEntity;
impl EntityExtractor for NoEntity {
    const CONTAINS_ENTITY: bool = false;
    type Output = ();
    #[inline(always)]
    unsafe fn extract(&self, _iter: &sys::ecs_iter_t, _idx: usize) {}
}

/// When you do want an `EntityView`, carry your `&WorldRef` here.
pub(crate) struct WithEntity<'w>(pub &'w WorldRef<'w>);
impl<'w> EntityExtractor for WithEntity<'w> {
    const CONTAINS_ENTITY: bool = true;
    type Output = EntityView<'w>;
    #[inline(always)]
    unsafe fn extract(&self, iter: &sys::ecs_iter_t, idx: usize) -> EntityView<'w> {
        // SAFETY: same as your original
        let raw = unsafe { *iter.entities.add(idx) };
        EntityView::new_from_raw(self.0, raw)
    }
}

#[inline(always)]
pub(crate) fn internal_run<P: ComponentId>(
    iter: &mut sys::ecs_iter_t,
    func: &mut impl FnMut(TableIter<true, P>),
    world: WorldRef<'_>,
) {
    iter.flags &= !sys::EcsIterIsValid;
    let iter_t = unsafe { TableIter::new(iter, world) };
    func(iter_t);
}

#[inline(always)]
fn each_plain<T: QueryTuple, E: EntityExtractor, F: FnMut(E::Output, T::TupleType<'_>)>(
    extractor: &E,
    components_data: &mut <T as QueryTuple>::Pointers,
    iter: &mut sys::ecs_iter_t,
    count: usize,
    func: &mut F,
) {
    // No “ref” or “row” – plain case
    for i in 0..count {
        let extra = unsafe { extractor.extract(iter, i) };
        let tuple = components_data.get_tuple(i);
        func(extra, tuple);
    }
}

#[inline(always)]
fn each_row<T: QueryTuple, E: EntityExtractor, F: FnMut(E::Output, T::TupleType<'_>)>(
    extractor: &E,
    components_data: &mut <T as QueryTuple>::Pointers,
    iter: &mut sys::ecs_iter_t,
    count: usize,
    func: &mut F,
) {
    // “row” case: sparse components
    for i in 0..count {
        let extra = unsafe { extractor.extract(iter, i) };
        let tuple = components_data.get_tuple_with_row(iter, i);
        func(extra, tuple);
    }
}

#[inline(always)]
fn each_ref<T: QueryTuple, E: EntityExtractor, F: FnMut(E::Output, T::TupleType<'_>)>(
    extractor: &E,
    components_data: &mut <T as QueryTuple>::Pointers,
    iter: &mut sys::ecs_iter_t,
    count: usize,
    func: &mut F,
) {
    // “ref” case: singleton and inherited components
    for i in 0..count {
        let extra = unsafe { extractor.extract(iter, i) };
        let tuple = components_data.get_tuple_with_ref(i);
        func(extra, tuple);
    }
}

#[inline(always)]
pub(crate) fn internal_each_generic<
    T: QueryTuple,
    E: EntityExtractor,
    const CALLED_FROM_RUN: bool,
    const ANY_SPARSE_TERMS: bool,
    F: FnMut(E::Output, T::TupleType<'_>),
>(
    iter: &mut sys::ecs_iter_t,
    extractor: E,
    mut func: F,
    _world: &WorldRef<'_>,
) {
    const {
        assert!(
            !T::CONTAINS_ANY_TAG_TERM,
            "a type provided in the query signature is a Tag and cannot be used with \
             `.each`. use `.run` instead or provide the tag with `.with()`"
        );
    }

    // #[cfg(feature = "flecs_safety_locks")]
    // let world = unsafe { WorldRef::from_ptr((iter).world) };
    #[cfg(any(debug_assertions, feature = "flecs_force_enable_ecs_asserts"))]
    let world_ptr = iter.world;
    iter.flags |= sys::EcsIterCppEach;
    let (is_any_array, mut components_data) = T::create_ptrs(iter);
    let count = if iter.count == 0 && iter.table.is_null() {
        1_usize
    } else {
        iter.count as usize
    };

    ecs_assert!(
        !(E::CONTAINS_ENTITY && iter.entities.is_null()),
        FlecsErrorCode::InvalidOperation,
        "query/system does not return entities ($this variable is not populated).\nQuery/System: {:?}",
        {
            if iter.system != 0 {
                let e = _world.entity_from_id(iter.system);
                (e.id(), e.get_name())
            } else {
                let e = unsafe { (*iter.query).entity };
                if e == 0 {
                    (crate::core::Entity(0), Some("<unnamed>".to_string()))
                } else {
                    let e = _world.entity_from_id(e);
                    (e.id(), e.get_name())
                }
            }
        }
    );

    #[cfg(feature = "flecs_safety_locks")]
    let (__locks, __lock_token) = acquire_batch_locks::<ANY_SPARSE_TERMS, T>(
        _world,
        components_data.safety_table_records(),
    );

    // only lock/unlock in debug or forced‑assert builds, and only
    // if we’re not in the “called from run” path:
    #[cfg(any(debug_assertions, feature = "flecs_force_enable_ecs_asserts"))]
    let _table_lock = (!CALLED_FROM_RUN).then(|| IterTableLock::new(world_ptr, iter.table));

    if !is_any_array.a_ref && !is_any_array.a_row {
        each_plain::<T, E, F>(&extractor, &mut components_data, iter, count, &mut func);
    } else if is_any_array.a_row {
        each_row::<T, E, F>(&extractor, &mut components_data, iter, count, &mut func);
    } else {
        each_ref::<T, E, F>(&extractor, &mut components_data, iter, count, &mut func);
    }

    #[cfg(feature = "flecs_safety_locks")]
    release_batch_locks::<ANY_SPARSE_TERMS, T>(
        _world,
        __locks,
        __lock_token,
        components_data.safety_table_records(),
    );
}

#[inline(always)]
pub(crate) fn internal_each_iter_next<
    T: QueryTuple,
    const CALLED_FROM_RUN: bool,
    const ANY_SPARSE_TERMS: bool,
>(
    iter: &mut sys::ecs_iter_t,
    world: &WorldRef<'_>,
    func: &mut impl FnMut(T::TupleType<'_>),
) {
    // drop the `()` from the call
    internal_each_generic::<T, NoEntity, CALLED_FROM_RUN, ANY_SPARSE_TERMS, _>(
        iter,
        NoEntity,
        move |(), t| func(t),
        world,
    );
}

#[inline(always)]
pub(crate) fn internal_each_entity_iter_next<
    T: QueryTuple,
    const CALLED_FROM_RUN: bool,
    const ANY_SPARSE_TERMS: bool,
>(
    iter: &mut sys::ecs_iter_t,
    world: &WorldRef<'_>,
    func: &mut impl FnMut(EntityView, T::TupleType<'_>),
) {
    // pass a WithEntity extractor so it builds the EntityView
    let extractor = WithEntity(world);
    internal_each_generic::<T, WithEntity<'_>, CALLED_FROM_RUN, ANY_SPARSE_TERMS, _>(
        iter, extractor, func, world,
    );
}

#[inline(always)]
pub(crate) fn internal_each_iter<
    T: QueryTuple,
    P: ComponentId,
    const CALLED_FROM_RUN: bool,
    const ANY_SPARSE_TERMS: bool,
>(
    iter: &mut sys::ecs_iter_t,
    world: &WorldRef<'_>,
    func: &mut impl FnMut(TableIter<CALLED_FROM_RUN, P>, FieldIndex, T::TupleType<'_>),
) {
    const {
        assert!(
            !T::CONTAINS_ANY_TAG_TERM,
            "a type provided in the query signature is a Tag and cannot be used with `.each`. use `.run` instead or provide the tag with `.with()`"
        );
    }

    unsafe {
        #[cfg(any(debug_assertions, feature = "flecs_force_enable_ecs_asserts"))]
        let world_ptr = iter.world;
        iter.flags |= sys::EcsIterCppEach;
        let (is_any_array, mut components_data) = T::create_ptrs(iter);
        let count = if iter.count == 0 && iter.table.is_null() {
            1_usize
        } else {
            iter.count as usize
        };

        #[cfg(feature = "flecs_safety_locks")]
        let (__locks, __lock_token) = acquire_batch_locks::<ANY_SPARSE_TERMS, T>(
            world,
            components_data.safety_table_records(),
        );

        // only lock/unlock in debug or forced‑assert builds, and only
        // if we’re not in the “called from run” path:
        #[cfg(any(debug_assertions, feature = "flecs_force_enable_ecs_asserts"))]
        let _table_lock = (!CALLED_FROM_RUN).then(|| IterTableLock::new(world_ptr, iter.table));

        if !is_any_array.a_ref && !is_any_array.a_row {
            for i in 0..count {
                let tuple = components_data.get_tuple(i);
                let iter_t = TableIter::new(iter, *world);
                func(iter_t, FieldIndex(i), tuple);
            }
        } else if is_any_array.a_row {
            for i in 0..count {
                let tuple = components_data.get_tuple_with_row(iter, i);
                let iter_t = TableIter::new(iter, *world);
                func(iter_t, FieldIndex(i), tuple);
            }
        } else {
            for i in 0..count {
                let tuple = components_data.get_tuple_with_ref(i);
                let iter_t = TableIter::new(iter, *world);
                func(iter_t, FieldIndex(i), tuple);
            }
        }

        #[cfg(feature = "flecs_safety_locks")]
        release_batch_locks::<ANY_SPARSE_TERMS, T>(
            world,
            __locks,
            __lock_token,
            components_data.safety_table_records(),
        );
    }
}
