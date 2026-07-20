use core::ffi::c_void;
use core::ptr::NonNull;
use flecs_ecs_sys as sys;

use crate::core::{
    FlecsArray, FlecsIdMap, QueryBuilderImpl, SystemAPI, WorldCtx, WorldRef, ecs_os_api, flecs,
    has_default_hook,
};

/// The `World` is the container for all ECS data. It stores the entities and
/// their components, does queries and runs systems.
///
/// Typically there is only a single world, but there is no limit on the number
/// of worlds an application can create.
///
/// If the world is deleted, all data in the world will be deleted as well.
///
/// # Examples
///
/// ```
/// # use flecs_ecs::core::World;
/// let world = World::new();
/// ```
///
/// # See also
///
/// * [`addons::app`](crate::addons::app)
// XAI: repr(C) so the field order matches `WorldRef` (also repr(C)). The
// `WorldRef: Deref<Target = World>` impl transmutes `&WorldRef -> &World`; that
// is only sound if both types share layout. A static assertion in the `Deref`
// impl pins size/align.
#[repr(C)]
#[derive(Debug, Eq, PartialEq)]
pub struct World {
    pub(crate) raw_world: NonNull<sys::ecs_world_t>,
    pub(crate) components: NonNull<FlecsIdMap>,
    pub(crate) components_array: NonNull<FlecsArray>,
}

impl Clone for World {
    fn clone(&self) -> Self {
        unsafe { sys::flecs_poly_claim_(self.raw_world.as_ptr() as *mut c_void) };
        Self {
            raw_world: self.raw_world,
            components: self.components,
            components_array: self.components_array,
        }
    }
}

impl Default for World {
    fn default() -> Self {
        ecs_os_api::ensure_initialized();

        let raw_world = NonNull::new(unsafe { sys::ecs_init() }).unwrap();
        let ctx = Box::leak(Box::new(WorldCtx::new()));
        let components = unsafe { NonNull::new_unchecked(&mut ctx.components) };
        let components_array = unsafe { NonNull::new_unchecked(&mut ctx.components_array) };
        let world = Self {
            raw_world,
            components,
            components_array,
        };
        unsafe {
            sys::ecs_set_binding_ctx(
                world.raw_world.as_ptr(),
                ctx as *mut WorldCtx as *mut c_void,
                None, //we manually destroy it in world drop for ref count check
            );
        }

        world.init_builtin_components();

        world
            .observer::<flecs::OnAdd, ()>()
            .with((flecs::With, flecs::Wildcard))
            .run(|mut it| {
                let world = it.world().ptr_mut();
                while it.next() {
                    for _ in it.iter() {
                        let pair_id = it.pair(0);
                        let second_id = pair_id.second_id();
                        let raw_second_id = **second_id;
                        let is_not_tag = unsafe { sys::ecs_get_typeid(world, raw_second_id) != 0 };

                         if is_not_tag {
                             assert!(
                                 has_default_hook(world, raw_second_id),
                                 "flecs::with requires a default-constructible target or be a ZST. Implement Default for {second_id}."
                             );
                         }
                    }
                }
            });

        world
    }
}

impl Drop for World {
    fn drop(&mut self) {
        if std::thread::panicking() {
            return;
        }

        #[cfg(feature = "flecs_safety_locks")]
        crate::core::invalidate_stage_locks_cache();

        let world_ptr = self.raw_world.as_ptr();
        if unsafe { sys::flecs_poly_release_(world_ptr as *mut c_void) } == 0 {
            if unsafe { sys::ecs_stage_get_id(world_ptr) } == -1 {
                unsafe { sys::ecs_stage_free(world_ptr) };
            } else {
                let ctx = self.world_ctx();

                // Blocks until no `QueryHandle` is mid-drop on another thread,
                // then makes all later handle drops no-ops (see `QueryHandle`).
                ctx.mark_world_dead();

                unsafe {
                    // before we call ecs_fini(), we increment the reference count back to 1
                    // otherwise, copies of this object created during ecs_fini (e.g. a component on_remove hook)
                    // would call again this destructor and ecs_fini().
                    sys::flecs_poly_claim_(world_ptr as *mut c_void);
                    sys::ecs_fini(self.raw_world.as_ptr())
                };
                let is_ref_count_not_zero = !ctx.is_ref_count_zero();
                if is_ref_count_not_zero && !ctx.is_panicking() {
                    ctx.set_is_panicking_true();
                    panic!("The code base still has lingering references to `Query` objects. This is a bug in the user code.
                        Please ensure that all `Query` objects are out of scope before the world is destroyed.");
                }

                let ctx = unsafe { Box::from_raw(ctx as *const WorldCtx as *mut WorldCtx) };
                drop(ctx);
            }
        }
    }
}

impl World {
    /// Creates a new world, same as `default()`
    pub fn new() -> Self {
        Self::default()
    }

    /// Creates a minimal world using `ecs_mini()` — no addons (system, pipeline,
    /// timer, meta) are loaded. Equivalent to C++ `flecs::world(ecs_mini())` +
    /// `world.make_owner()`.
    ///
    /// Use this when you need a lightweight world without any default addon
    /// overhead, e.g. for testing core ECS functionality in isolation.
    pub fn new_mini() -> Self {
        ecs_os_api::ensure_initialized();

        let raw_world = NonNull::new(unsafe { sys::ecs_mini() }).unwrap();
        let ctx = Box::leak(Box::new(WorldCtx::new()));
        let components = unsafe { NonNull::new_unchecked(&mut ctx.components) };
        let components_array = unsafe { NonNull::new_unchecked(&mut ctx.components_array) };
        let world = Self {
            raw_world,
            components,
            components_array,
        };
        unsafe {
            sys::ecs_set_binding_ctx(
                world.raw_world.as_ptr(),
                ctx as *mut WorldCtx as *mut c_void,
                None,
            );
        }
        world
    }

    /// Explicitly release this world handle, decrementing the ref count.
    /// If this is the last handle, the world is finalized (`ecs_fini`).
    ///
    /// Equivalent to C++ `world.release()`. Taking `self` by value ensures the
    /// caller cannot use the handle after release — the Rust equivalent of
    /// setting `world_ = nullptr` after `flecs_poly_release`.
    ///
    /// # Examples
    /// ```
    /// # use flecs_ecs::core::World;
    /// let world_a = World::new();
    /// let world_b = world_a.clone(); // both point to same world
    /// world_a.release();             // decrements ref; world still alive in world_b
    /// // world_a is consumed — cannot be used here
    /// world_b.release();             // last handle → ecs_fini called
    /// ```
    pub fn release(self) {
        drop(self); // Drop impl handles flecs_poly_release / ecs_fini
    }

    fn init_builtin_components(&self) {
        // used for event handling with no data
        self.component_named::<()>("flecs::rust::() - None");

        #[cfg(feature = "flecs_meta")]
        {
            self.component_named::<crate::prelude::meta::EcsTypeKind>("flecs::meta::type_kind");
            self.component_named::<crate::prelude::meta::EcsPrimitiveKind>(
                "flecs::meta::primitive_kind",
            );

            //self.component_named::<crate::prelude::meta::EcsMember>("flecs::meta::member_t");
            //self.component_named::<crate::prelude::meta::EcsEnumConstant>(
            //    "flecs::meta::enum_constant",
            //);
            //self.component_named::<crate::prelude::meta::EcsBitmaskConstant>(
            //    "flecs::meta::bitmask_constant",
            //);

            unsafe {
                use crate::core::FlecsConstantId;

                self.entity_named("::flecs::rust")
                    .add_id_unchecked(flecs::Module::ID)
            };
            crate::addons::meta::meta_init_builtin(self);
            // entity.scope(|world| {
            //     let comp = world.component::<Entity>();
            //     comp.opaque_func(crate::prelude::meta::flecs_entity_support);
            // });
        }
    }
}

/// Owning handle to an asynchronous stage created with [`World::async_stage()`].
///
/// The underlying stage is freed when this handle is dropped. See
/// [`World::create_async_stage()`] for the semantics of asynchronous stages.
#[derive(Debug)]
pub struct AsyncStage<'a> {
    stage: WorldRef<'a>,
}

impl<'a> AsyncStage<'a> {
    pub(crate) fn new(world: &'a World) -> Self {
        let stage_ptr = unsafe { sys::ecs_stage_new(world.raw_world.as_ptr()) };
        Self {
            // SAFETY: `ecs_stage_new` returns a valid stage pointer for a live world.
            stage: unsafe { WorldRef::from_ptr(stage_ptr) },
        }
    }

    /// Returns the stage as a [`WorldRef`] to enqueue commands on.
    ///
    /// The returned [`WorldRef`] cannot outlive a borrow of this handle, so it
    /// cannot be used after the stage is freed.
    pub fn stage(&self) -> WorldRef<'_> {
        self.stage
    }

    /// Merges the commands enqueued on this stage into the world.
    ///
    /// # See also
    ///
    /// * [`World::merge()`]
    pub fn merge(&self) {
        unsafe { sys::ecs_merge(self.stage.raw_world.as_ptr()) };
    }
}

impl Drop for AsyncStage<'_> {
    fn drop(&mut self) {
        if std::thread::panicking() {
            return;
        }
        #[cfg(feature = "flecs_safety_locks")]
        crate::core::invalidate_stage_locks_cache();
        // SAFETY: this handle exclusively owns the stage and the lifetime `'a`
        // guarantees the world the stage belongs to is still alive.
        unsafe { sys::ecs_stage_free(self.stage.raw_world.as_ptr()) };
    }
}
