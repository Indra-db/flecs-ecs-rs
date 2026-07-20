//! Refs are a fast mechanism for referring to a specific entity/component. It caches data to speedup get operations.

use core::{ffi::c_void, marker::PhantomData, ptr::NonNull};

use crate::core::world_ctx::ScopeEndGuard;
use crate::core::*;
use crate::sys;

/// A cached reference for fast access to a component from a specific entity.
///
/// Every access runs inside a defer scope (structural changes made in the
/// callback are queued, so the component pointer stays valid) and, with the
/// `flecs_safety_locks` feature, registers a write borrow in the same
/// mut-alias tracking that `EntityView::get` and query iteration use.
#[derive(Debug, Clone, Copy)]
pub struct CachedRef<'a, T> {
    pub(crate) world: WorldRef<'a>,
    pub(crate) component_ref: sys::ecs_ref_t,
    pub(crate) component_id: sys::ecs_id_t,
    /// Cached storage lock key, valid while the ref stays in
    /// `lock_key_table_id`'s table; recomputed C-side on archetype change.
    #[cfg(feature = "flecs_safety_locks")]
    pub(crate) lock_key: LockKey,
    #[cfg(feature = "flecs_safety_locks")]
    pub(crate) lock_key_table_id: u64,
    pub(crate) _marker: PhantomData<T>,
}

impl<'a, T> CachedRef<'a, T> {
    /// Create a new ref to a component.
    ///
    /// # Arguments
    ///
    /// * `world`: the world.
    /// * `entity`: the entity to reference.
    /// * `id`: the id of the component to reference.
    pub fn new(
        world: impl WorldProvider<'a>,
        entity: impl Into<Entity>,
        component: impl IntoId,
    ) -> CachedRef<'a, T> {
        let stage_world = world.world();
        // the world we were called with may be a stage; convert it to a world
        // here if that is the case — ecs_ref_* operations require the real world
        let world_ptr = unsafe {
            sys::ecs_get_world(stage_world.world_ptr_mut() as *const c_void)
                as *mut sys::ecs_world_t
        };
        let world = unsafe { WorldRef::from_ptr(world_ptr) };

        let id = *component.into_id(stage_world);

        debug_assert!(
            id != 0,
            "Tried to create invalid `CachedRef` type. id is 0."
        );

        const {
            assert!(
                core::mem::size_of::<T>() != 0,
                "Cached Ref cannot be created for zero-sized types / tags."
            );
        }

        // TODO this is done with FLECS_DEBUG flag normally
        debug_assert!(
            {
                let type_ = unsafe { sys::ecs_get_typeid(world_ptr, id) };
                let ti = unsafe { sys::ecs_get_type_info(world_ptr, type_) };
                ti.is_null() || unsafe { (*ti).size } != 0
            },
            "Cannot create ref to empty type"
        );

        let component_ref = unsafe { sys::ecs_ref_init_id(world_ptr, *entity.into(), id) };
        assert_ne!(
            component_ref.entity, 0,
            "Tried to create invalid `CachedRef` type."
        );
        CachedRef::<T> {
            world,
            component_ref,
            component_id: id,
            #[cfg(feature = "flecs_safety_locks")]
            lock_key: 0,
            #[cfg(feature = "flecs_safety_locks")]
            lock_key_table_id: u64::MAX,
            _marker: PhantomData,
        }
    }

    /// Return entity associated with reference.
    pub fn entity(&self) -> EntityView<'a> {
        EntityView::new_from(self.world, self.component_ref.entity)
    }

    /// Return component associated with reference.
    pub fn component(&self) -> IdView<'a> {
        IdView::new_from_id(self.world, self.component_id)
    }

    pub fn has(&mut self) -> bool {
        !unsafe {
            sys::ecs_ref_get_id(
                self.world.world_ptr_mut(),
                &mut self.component_ref,
                self.component_id,
            )
        }
        .is_null()
    }

    pub fn world(&self) -> WorldRef<'a> {
        self.world
    }

    /// Resolve the component pointer and open the access scope
    /// (defer + cached-key refresh). Returns null when the component is gone,
    /// in which case no scope was opened.
    #[inline(always)]
    fn scope_begin(&mut self) -> *mut c_void {
        #[cfg(feature = "flecs_safety_locks")]
        let cached_table_id = self.lock_key_table_id;
        #[cfg(not(feature = "flecs_safety_locks"))]
        let cached_table_id = self.component_ref.table_id;

        let get_ptr = unsafe {
            sys::ecs_rust_ref_get_scope_begin(
                self.world.world_ptr_mut(),
                &mut self.component_ref,
                self.component_id,
                cached_table_id,
            )
        };

        #[cfg(feature = "flecs_safety_locks")]
        if !get_ptr.ptr.is_null() && get_ptr.lock_key != 0 {
            self.lock_key = get_ptr.lock_key;
            self.lock_key_table_id = self.component_ref.table_id;
        }

        get_ptr.ptr
    }

    /// Register the write borrow for the scope opened by [`Self::scope_begin`].
    #[cfg(feature = "flecs_safety_locks")]
    #[inline(always)]
    fn write_begin(&self) -> NonNull<StageLocks> {
        let locks = stage_locks_dyn(&self.world);
        // SAFETY: stage map is owned by this thread.
        if unsafe { (*locks.as_ptr()).write_begin(self.lock_key) } {
            alias_violation_panic(&self.world, self.component_id, true);
        }
        locks
    }

    #[cfg(feature = "flecs_safety_locks")]
    #[inline(always)]
    fn write_end(&self, locks: NonNull<StageLocks>) {
        // SAFETY: stage map is owned by this thread.
        unsafe { (*locks.as_ptr()).write_end(self.lock_key) };
    }
}

macro_rules! ref_access {
    ($self:ident, $ptr:ident, $callback:ident) => {{
        let _scope = ScopeEndGuard {
            world: $self.world,
        };
        #[cfg(feature = "flecs_safety_locks")]
        {
            let locks = $self.write_begin();
            let ret = $callback($ptr);
            $self.write_end(locks);
            ret
        }
        #[cfg(not(feature = "flecs_safety_locks"))]
        {
            $callback($ptr)
        }
    }};
}

impl<'a, T: ComponentId> CachedRef<'a, T> {
    /// Try to get component from ref.
    pub fn try_get<R>(&mut self, callback: impl FnOnce(&mut T) -> R) -> Option<R> {
        self.world.check_thread_affinity_exclusive::<T>();
        let mut ptr = NonNull::new(self.scope_begin() as *mut T)?;
        // SAFETY: scope_begin returned non-null; defer scope keeps the
        // storage stable and the write lock guards aliasing.
        let ptr = unsafe { ptr.as_mut() };
        Some(ref_access!(self, ptr, callback))
    }

    pub fn get<R>(&mut self, callback: impl FnOnce(&mut T) -> R) -> R {
        self.world.check_thread_affinity_exclusive::<T>();
        let mut ptr = NonNull::new(self.scope_begin() as *mut T)
            .expect("Component not found, use try_get if you want to handle this case");
        // SAFETY: as in try_get.
        let ptr = unsafe { ptr.as_mut() };
        ref_access!(self, ptr, callback)
    }

    /// Access the component without opening a defer scope or registering the
    /// borrow in the mut-alias tracking. This is the fastest possible path.
    ///
    /// # Safety
    ///
    /// The callback must not create any other reference to this component
    /// (no nested `get`, no query iteration touching it on this thread) and
    /// must not perform structural changes (add/remove/delete on any entity
    /// of this component's table), which could move or free the storage the
    /// reference points into.
    pub unsafe fn get_unchecked<R>(&mut self, callback: impl FnOnce(&mut T) -> R) -> R {
        self.world.check_thread_affinity_exclusive::<T>();
        let mut ref_comp = NonNull::new(unsafe {
            sys::ecs_ref_get_id(
                self.world.world_ptr_mut(),
                &mut self.component_ref,
                self.component_id,
            ) as *mut T
        })
        .expect("Component not found");

        callback(unsafe { ref_comp.as_mut() })
    }
}

impl<'a> CachedRef<'a, core::ffi::c_void> {
    /// Try to get component from ref.
    pub fn try_get<R>(&mut self, callback: impl FnOnce(*mut core::ffi::c_void) -> R) -> Option<R> {
        let ptr = NonNull::new(self.scope_begin())?.as_ptr();
        Some(ref_access!(self, ptr, callback))
    }

    pub fn get<R>(&mut self, callback: impl FnOnce(*mut core::ffi::c_void) -> R) -> R {
        let ptr = NonNull::new(self.scope_begin())
            .expect("Component not found, use try_get if you want to handle this case")
            .as_ptr();
        ref_access!(self, ptr, callback)
    }
}
