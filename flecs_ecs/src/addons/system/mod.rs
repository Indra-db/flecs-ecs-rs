//! Systems are a query + function that can be ran manually or by a pipeline.
//!
//! The system module allows for creating and running systems. A system is a
//! query in combination with a callback function. In addition systems have
//! support for time management, scheduling via pipeline and can be monitored by the stats addon.

mod system_builder;
mod system_runner_fluent;
pub use system_builder::*;
pub use system_runner_fluent::*;

use core::ops::DerefMut;
use core::{ffi::c_void, ops::Deref, ptr::NonNull};

use crate::core::*;
use crate::sys;

/// Systems are a query + function that can be ran manually or by a pipeline.
#[derive(Clone, Copy)]
pub struct System<'a> {
    pub(crate) entity: EntityView<'a>,
}

impl core::fmt::Debug for System<'_> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        core::fmt::Debug::fmt(&self.entity, f)
    }
}

impl core::fmt::Display for System<'_> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        core::fmt::Display::fmt(&self.entity, f)
    }
}

impl PartialEq for System<'_> {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        self.entity == other.entity
    }
}

impl Eq for System<'_> {}

impl<'a> Deref for System<'a> {
    type Target = EntityView<'a>;

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.entity
    }
}

impl DerefMut for System<'_> {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.entity
    }
}

impl<'a> WorldProvider<'a> for System<'a> {
    #[inline(always)]
    fn world(&self) -> WorldRef<'a> {
        self.world
    }
}

impl<'a> System<'a> {
    //todo!() in query etc desc is a pointer, does it need to be?
    /// Create a new system
    ///
    /// # Arguments
    ///
    /// * `world` - The world to create the system in.
    /// * `desc` - The system description.
    pub fn new(world: impl WorldProvider<'a>, desc: sys::ecs_system_desc_t) -> Self {
        let id = unsafe { sys::ecs_system_init(world.world_ptr_mut(), &desc) };
        let entity = EntityView::new_from(world.world(), id);

        Self { entity }
    }

    /// Wrap an existing system entity in a system object
    ///
    /// # Arguments
    ///
    /// * `world` - The world the system is in.
    /// * `system_entity` - The entity of the system.
    pub fn new_from_existing(system_entity: EntityView<'a>) -> Self {
        Self {
            entity: system_entity,
        }
    }

    /// Replace the system's callback or run action.
    ///
    /// Returns an updater on which `.each()`/`.run()` (and variants) can be
    /// called; the change is applied through `ecs_system_update()`.
    pub fn update<T: QueryTuple>(&self) -> SystemUpdater<'a, T> {
        SystemUpdater::new(self.entity)
    }

    /// Limit which groups the system iterates. Only tables in the given
    /// query group will be iterated by the system.
    pub fn set_group(&self, group_id: impl IntoEntity) {
        let group_id = group_id.into_entity(self.world);
        unsafe {
            sys::ecs_system_set_group(self.world.world_ptr_mut(), *self.id(), *group_id);
        }
    }

    /// Reset the system's group filter set by [`System::set_group()`].
    pub fn clear_group(&self) {
        unsafe {
            sys::ecs_system_set_group(self.world.world_ptr_mut(), *self.id(), 0);
        }
    }

    /// Set the context for the system
    ///
    /// # Arguments
    ///
    /// * `context` - The context to set.
    pub fn set_context(&mut self, context: *mut c_void) {
        let desc: sys::ecs_system_desc_t = sys::ecs_system_desc_t {
            ctx: context,
            ..Default::default()
        };

        unsafe {
            sys::ecs_system_update(self.world.world_ptr_mut(), *self.id(), &desc);
        }
    }

    fn system_ptr(&self) -> *const sys::ecs_system_t {
        let system = unsafe { sys::ecs_system_get(self.world.world_ptr(), *self.id()) };
        assert!(!system.is_null(), "entity is not a system");
        system
    }

    /// Get the context for the system
    ///
    /// # Panics
    ///
    /// Panics if the entity is not a system.
    pub fn context(&self) -> *mut c_void {
        unsafe { (*self.system_ptr()).ctx }
    }

    /// Read the typed context stored by
    /// [`SystemBuilder::ctx`](crate::addons::system::SystemBuilder::ctx), or
    /// `None` if none was set or the stored value has a different type (spec
    /// §5.3).
    ///
    /// # Panics
    ///
    /// Panics if the entity is not a system.
    pub fn ctx<C: 'static>(&self) -> Option<&C> {
        let ctx = unsafe { (*self.system_ptr()).ctx };
        if ctx.is_null() {
            return None;
        }
        // SAFETY: a non-null ctx set through `SystemBuilder::ctx` is the thin
        // `*mut Box<dyn Any>` it installed; `downcast_ref` returns `None` on a
        // type mismatch. The borrow is tied to `&self`.
        let boxed = unsafe { &*(ctx as *const alloc::boxed::Box<dyn core::any::Any>) };
        boxed.downcast_ref::<C>()
    }

    /// Mutably read the typed context stored by
    /// [`SystemBuilder::ctx`](crate::addons::system::SystemBuilder::ctx), or
    /// `None` (spec §5.3).
    ///
    /// # Panics
    ///
    /// Panics if the entity is not a system.
    pub fn ctx_mut<C: 'static>(&mut self) -> Option<&mut C> {
        let ctx = unsafe { (*self.system_ptr()).ctx };
        if ctx.is_null() {
            return None;
        }
        // SAFETY: as `ctx`, but `&mut self` proves exclusive access to the
        // system's context.
        let boxed = unsafe { &mut *(ctx as *mut alloc::boxed::Box<dyn core::any::Any>) };
        boxed.downcast_mut::<C>()
    }

    /// Get the underlying query for the system
    ///
    /// # Panics
    ///
    /// Panics if the entity is not a system or the system has no query.
    pub fn query(&self) -> Query<()> {
        let query =
            NonNull::new(unsafe { (*self.system_ptr()).query }).expect("system has no query");
        unsafe { Query::<()>::new_from(query) }
    }

    /// returns whether the system is immediate
    ///
    /// # Panics
    ///
    /// Panics if the entity is not a system.
    pub fn immediate(&self) -> bool {
        unsafe { (*self.system_ptr()).immediate }
    }

    /// returns whether the system is multi-threaded
    ///
    /// # Panics
    ///
    /// Panics if the entity is not a system.
    pub fn multi_threaded(&self) -> bool {
        unsafe { (*self.system_ptr()).multi_threaded }
    }

    /// Run the system
    ///
    /// # Arguments
    ///
    /// * `delta_time` - The time delta.
    /// * `param` - A user-defined parameter to pass to the system
    #[inline]
    pub fn run_dt_param(&self, delta_time: FTime, param: *mut c_void) -> SystemRunnerFluent<'_> {
        SystemRunnerFluent::new(self.world.real_world(), *self.id(), 0, 0, delta_time, param)
    }

    /// Run the system
    ///
    /// # Arguments
    ///
    /// * `delta_time` - The time delta.
    #[inline]
    pub fn run_dt(&self, delta_time: FTime) -> SystemRunnerFluent<'_> {
        self.run_dt_param(delta_time, core::ptr::null_mut())
    }

    /// Run the system
    #[inline]
    pub fn run(&self) -> SystemRunnerFluent<'_> {
        self.run_dt_param(0.0, core::ptr::null_mut())
    }

    /// Run the system worker
    ///
    /// # Arguments
    ///
    /// * `stage_current` - The current stage.
    /// * `stage_count` - The total number of stages.
    /// * `delta_time` - The time delta.
    /// * `param` - An optional parameter to pass to the system.
    pub fn run_worker(
        &self,
        stage_current: i32,
        stage_count: i32,
        delta_time: FTime,
        param: *mut c_void,
    ) -> SystemRunnerFluent<'_> {
        SystemRunnerFluent::new(
            self.world,
            *self.id(),
            stage_current,
            stage_count,
            delta_time,
            param,
        )
    }
}
