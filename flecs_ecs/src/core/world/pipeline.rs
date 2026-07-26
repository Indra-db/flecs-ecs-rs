use core::ffi::CStr;

use crate::addons::pipeline::PipelineBuilder;

use super::*;

/// Pipeline mixin implementation
impl World {
    /// Create a new [`Pipeline`](crate::addons::pipeline::Pipeline).
    ///
    /// # See also
    ///
    /// * [`World::pipeline_named()`]
    /// * [`World::pipeline_type()`]
    #[inline(always)]
    pub fn pipeline(&self) -> PipelineBuilder<'_, ()> {
        PipelineBuilder::<()>::new(self)
    }

    /// Create a new named [`Pipeline`](crate::addons::pipeline::Pipeline).
    ///
    /// # Arguments
    ///
    /// * `name` - The name of the pipeline.
    ///
    /// # See also
    ///
    /// * [`World::pipeline()`]
    /// * [`World::pipeline_type()`]
    #[inline(always)]
    pub fn pipeline_named<'a>(&'a self, name: &str) -> PipelineBuilder<'a, ()> {
        PipelineBuilder::<()>::new_named(self, name)
    }

    /// Create a new [`Pipeline`](crate::addons::pipeline::Pipeline) with the provided associated type.
    ///
    /// # Type Parameters
    ///
    /// * `Pipeline` - The associated type to use for the pipeline.
    ///
    /// # See also
    ///
    /// * [`World::pipeline()`]
    /// * [`World::pipeline_named()`]
    #[inline(always)]
    pub fn pipeline_type<Pipeline>(&self) -> PipelineBuilder<'_, ()>
    where
        Pipeline: ComponentType<Struct> + ComponentId,
    {
        PipelineBuilder::<()>::new_with_entity(self, Pipeline::entity_id(self))
    }

    /// Set a custom pipeline. This operation sets the pipeline to run when [`World::progress()`] is invoked.
    ///
    /// # Arguments
    ///
    /// * `pipeline` - The pipeline to set.
    ///
    /// # See also
    ///
    /// * [`World::get_pipeline()`]
    /// * [`World::set_pipeline()`]
    #[inline(always)]
    pub fn set_pipeline(&self, pipeline: impl IntoEntity) {
        let world = self.world();
        unsafe {
            sys::ecs_set_pipeline(self.raw_world.as_ptr(), *pipeline.into_entity(world));
        }
    }

    /// Get the current pipeline.
    ///
    /// # Returns
    ///
    /// The current pipeline as an entity.
    ///
    /// # See also
    ///
    /// * [`World::set_pipeline()`]
    /// * [`World::set_pipeline()`]
    #[inline(always)]
    pub fn get_pipeline(&self) -> EntityView<'_> {
        EntityView::new_from(self, unsafe {
            sys::ecs_get_pipeline(self.raw_world.as_ptr())
        })
    }

    /// Progress world one tick.
    ///
    /// Progresses the world by running all enabled and periodic systems
    /// on their matching entities.
    ///
    /// This is a wrapper around [`World::progress_time()`]. It passes `0.0` as
    /// the `delta_time` to automatically measure the time passed since the last
    /// frame. This mode is useful for applications that do not manage time
    /// explicitly and want the system to measure the time automatically.
    ///
    /// # Returns
    ///
    /// True if the world has been progressed, false if [`World::quit()`] has been called.
    ///
    /// # A live guard cannot span a frame
    ///
    /// `progress` takes `&mut self`. A shared-register guard
    /// ([`Ref`](crate::experimental::Ref) / [`Mut`](crate::experimental::Mut) /
    /// `get_ref`) borrows the world `&World`, so the borrow checker forbids it
    /// from being live across a frame step. This replaces the old runtime refusal
    /// (which could only fire after the fact); it is now a compile error:
    ///
    /// ```compile_fail
    /// use flecs_ecs::prelude::*;
    ///
    /// #[derive(Component)]
    /// struct Position { x: i32, y: i32 }
    ///
    /// let mut world = World::new();
    /// let e = world.entity().set(Position { x: 1, y: 2 });
    /// let guard = e.get_ref::<&Position>().unwrap(); // borrows the world
    /// world.progress();                              // needs &mut world -> conflict
    /// let _ = guard;
    /// ```
    ///
    /// # See also
    ///
    /// * [`World::progress_time()`]
    /// * C API: `ecs_progress`
    #[inline(always)]
    pub fn progress(&mut self) -> bool {
        self.progress_time(0.0)
    }

    /// Progress world by delta time.
    ///
    /// Progresses the world by running all enabled and periodic systems
    /// on their matching entities for the specified time since the last frame.
    ///
    /// When `delta_time` is 0, `World::progress_time()` will automatically measure the time passed
    /// since the last frame. For applications not using time management, passing a
    /// non-zero `delta_time` (1.0 recommended) skips automatic time measurement to avoid overhead.
    ///
    /// # Arguments
    ///
    /// * `delta_time` - The time to progress the world by. Pass 0.0 for automatic time measurement.
    ///
    /// # Returns
    ///
    /// True if the world has been progressed, false if [`World::quit()`] has been called.
    ///
    /// # See also
    ///
    /// * [`World::progress()`]
    /// * C API: `ecs_progress`
    #[inline(always)]
    pub fn progress_time(&mut self, delta_time: f32) -> bool {
        // `progress` takes `&mut self` (spec §5.7): it is an exclusive-register
        // frame step. A shared-register guard (`Ref` / `Mut`) borrows the world
        // `&World`, which the borrow checker cannot let coexist with this
        // `&mut self`, so the runtime refusal below is now unreachable from safe
        // code (see the compile_fail doctest on `progress`). It is demoted to a
        // debug-only backstop against `unsafe` code that manufactures a live pin
        // without an outstanding borrow; it costs the release hot path nothing.
        #[cfg(all(debug_assertions, feature = "flecs_safety_locks"))]
        crate::core::assert_no_live_pin(&(&*self).world(), "World::progress()");
        unsafe { sys::ecs_progress(self.raw_world.as_ptr(), delta_time) }
    }

    /// Run pipeline.
    /// Runs all systems in the specified pipeline. Can be invoked from multiple
    /// threads if staging is disabled, managing staging and, if needed, thread
    /// synchronization.
    ///
    /// Providing 0 for pipeline id runs the default pipeline (builtin or set via
    /// `set_pipeline()`). Using [`World::progress()`] auto-invokes this for the
    /// default pipeline. Additional pipelines may be run explicitly.
    ///
    /// # Note
    ///
    /// Only supports single-threaded applications with a single stage when called from an application.
    ///
    /// # Arguments
    ///
    /// * `pipeline` - Pipeline to run.
    ///
    /// # See also
    ///
    /// * [`World::run_pipeline()`]
    /// * [`World::run_pipeline_time()`]
    #[inline(always)]
    pub fn run_pipeline(&mut self, pipeline: impl IntoEntity) {
        Self::run_pipeline_time(self, pipeline, 0.0);
    }

    /// Run pipeline.
    /// Runs all systems in the specified pipeline. Can be invoked from multiple
    /// threads if staging is disabled, managing staging and, if needed, thread
    /// synchronization.
    ///
    /// Providing 0 for pipeline id runs the default pipeline (builtin or set via
    /// `set_pipeline()`). Using [`World::progress()`] auto-invokes this for the
    /// default pipeline. Additional pipelines may be run explicitly.
    ///
    /// # Note
    ///
    /// Only supports single-threaded applications with a single stage when called from an application.
    ///
    /// # Arguments
    ///
    /// * `pipeline` - Pipeline to run.
    /// * `delta_time` - Time to advance the world.
    ///
    /// # See also
    ///
    /// * [`World::run_pipeline()`]
    /// * [`World::run_pipeline_time()`]
    #[inline(always)]
    pub fn run_pipeline_time(&mut self, pipeline: impl IntoEntity, delta_time: FTime) {
        let world = (&*self).world();
        // As `progress` (spec §5.7): `&mut self` makes a live shared-register
        // guard impossible to hold across this call, so the runtime refusal is
        // unreachable from safe code and demoted to a debug-only backstop.
        #[cfg(all(debug_assertions, feature = "flecs_safety_locks"))]
        crate::core::assert_no_live_pin(&world, "World::run_pipeline()");
        unsafe {
            sys::ecs_run_pipeline(
                self.raw_world.as_ptr(),
                *pipeline.into_entity(world),
                delta_time,
            );
        }
    }

    /// Set time scale. Increase or decrease simulation speed by the provided multiplier.
    ///
    /// # Arguments
    ///
    /// * `mul` - The multiplier to set the time scale to.
    ///
    /// # See also
    ///
    /// * [`World::get_time_scale()`]
    #[inline(always)]
    pub fn set_time_scale(&self, mul: FTime) {
        unsafe {
            sys::ecs_set_time_scale(self.raw_world.as_ptr(), mul);
        }
    }

    /// Get time scale.
    ///
    /// Retrieves the current time scale of the world, which affects the speed
    /// at which time passes within the simulation. A time scale of 1.0 means
    /// real-time, values greater than 1.0 speed up the simulation, and values
    /// less than 1.0 slow it down.
    ///
    /// # Returns
    ///
    /// The current time scale as a floating point number.
    ///
    /// # See also
    ///
    /// * [`World::set_time_scale()`]
    #[inline(always)]
    pub fn get_time_scale(&self) -> FTime {
        self.info().time_scale
    }

    /// Get target frames per second (FPS).
    ///
    /// Retrieves the target FPS for the world. This value is used to calculate
    /// the time step for each simulation tick when the automatic time step is
    /// enabled. Adjusting the target FPS can be used to control simulation
    /// speed.
    ///
    /// # Returns
    ///
    /// The target FPS as a floating point number.
    ///
    /// # See also
    ///
    /// * [`World::set_target_fps()`]
    #[inline(always)]
    pub fn get_target_fps(&self) -> FTime {
        self.info().target_fps
    }

    /// Set target frames per second (FPS).
    ///
    /// Configures the world to run at the specified target FPS, ensuring that
    /// [`World::progress()`] is not called more frequently than this rate. This mechanism
    /// enables tracking the elapsed time since the last [`World::progress()`] call and
    /// sleeping for any remaining time in the frame, if applicable.
    ///
    /// Utilizing this feature promotes consistent system execution intervals and
    /// conserves CPU resources by avoiding more frequent system runs than necessary.
    ///
    /// It's important to note that [`World::progress()`] will only introduce sleep periods
    /// when there is surplus time within a frame. This accounts for time consumed both
    /// within Flecs and in external operations.
    ///
    /// # Arguments
    ///
    /// * `world` - The world context.
    /// * `fps` - The desired target FPS as a floating-point number.
    ///
    /// # See also
    ///
    /// * [`World::get_target_fps()`]
    #[inline(always)]
    pub fn set_target_fps(&self, target_fps: FTime) {
        unsafe {
            sys::ecs_set_target_fps(self.raw_world.as_ptr(), target_fps);
        }
    }

    /// Reset world clock. Reset the clock that keeps track of the total time passed in the simulation.
    #[inline(always)]
    pub fn reset_clock(&self) {
        unsafe {
            sys::ecs_reset_clock(self.raw_world.as_ptr());
        }
    }

    /// Set number of worker threads.
    ///
    /// Setting this value to a value higher than 1 will start as many threads and
    /// will cause systems to evenly distribute matched entities across threads.
    /// The operation may be called multiple times to reconfigure the number of threads used,
    /// but never while running a system / pipeline. Calling [`World::set_threads()`] will also end the use
    /// of task threads setup with [`World::set_task_threads()`] and vice-versa
    ///
    /// # Arguments
    ///
    /// * `threads` - The number of threads to use.
    ///
    /// # See also
    ///
    /// * [`World::set_stage_count()`]
    /// * [`World::set_task_threads()`]
    #[inline(always)]
    pub fn set_threads(&self, threads: i32) {
        unsafe {
            sys::ecs_set_threads(self.raw_world.as_ptr(), threads);
        }
        #[cfg(feature = "flecs_safety_locks")]
        self.world_ctx()
            .safety_locks
            .set_stage_count(unsafe { sys::ecs_get_stage_count(self.raw_world.as_ptr()) });
    }

    /// Get number of configured stages. Return number of stages set by [`World::set_stage_count()`].
    ///
    /// # Returns
    ///
    /// The number of stages as an integer.
    ///
    /// # See also
    ///
    /// * [`World::set_stage_count()`]
    /// * [`World::set_threads()`]
    #[inline(always)]
    pub fn get_threads(&self) -> i32 {
        unsafe { sys::ecs_get_stage_count(self.raw_world.as_ptr()) }
    }

    /// Set number of worker task threads.
    ///
    /// Configures the world to use a specified number of short-lived task threads,
    /// distinct from [`World::set_threads()`] where threads persist. Here, threads are
    /// created and joined for each world update, leveraging the `os_api_t` tasks
    /// APIs for task management instead of traditional thread APIs. This approach
    /// is advantageous for integrating with external asynchronous job systems,
    /// allowing for the dynamic creation and synchronization of tasks specific to
    /// each world update.
    ///
    /// This function can be invoked multiple times to adjust the count of task threads,
    /// but must not be called concurrently with system or pipeline execution. Switching
    /// to [`World::set_task_threads()`] from [`World::set_threads()`] (or vice versa) will
    /// terminate the use of the previously configured threading model.
    ///
    /// # Arguments
    ///
    /// * `task_threads` - The number of task threads to use.
    ///
    /// # See also
    ///
    /// * [`World::using_task_threads()`]
    #[inline(always)]
    pub fn set_task_threads(&self, task_threads: i32) {
        unsafe {
            sys::ecs_set_task_threads(self.raw_world.as_ptr(), task_threads);
        }
        #[cfg(feature = "flecs_safety_locks")]
        self.world_ctx()
            .safety_locks
            .set_stage_count(unsafe { sys::ecs_get_stage_count(self.raw_world.as_ptr()) });
    }

    /// Returns true if task thread use have been requested.
    ///
    /// # Returns
    ///
    /// True if task threads are being used, false otherwise.
    ///
    /// # See also
    ///
    /// * [`World::set_task_threads()`]
    #[inline(always)]
    pub fn using_task_threads(&self) -> bool {
        unsafe { sys::ecs_using_task_threads(self.raw_world.as_ptr()) }
    }

    /// Delete empty tables within the world
    ///
    /// # See also
    ///
    /// * C API: `ecs_delete_empty_tables`
    #[inline(always)]
    pub fn delete_empty_tables(&self, desc: sys::ecs_delete_empty_tables_desc_t) -> i32 {
        // SAFETY: `self.raw_world` is a valid, non-null pointer to a live flecs world;
        // `&desc` points to a properly initialized `ecs_delete_empty_tables_desc_t`
        // owned by this stack frame and valid for the duration of the call.
        unsafe { sys::ecs_delete_empty_tables(self.raw_world.as_ptr(), &desc) }
    }

    /// Begin exclusive thread access to the world.
    ///
    /// # Panics
    ///
    /// This operation ensures that only the thread from which this operation is
    /// called can access the world. Attempts to access the world from other threads
    /// will panic.
    ///
    /// `exclusive_access_begin()` must be called in pairs with
    /// `exclusive_access_end()`. Calling `exclusive_access_begin()` from another
    /// thread without first calling `exclusive_access_end()` will panic.
    ///
    /// This operation should only be called once per thread. Calling it multiple
    /// times for the same thread will cause a panic.
    ///
    /// # Note
    ///
    /// This feature only works in builds where asserts are enabled. The
    /// feature requires the OS API `thread_self_` callback to be set.
    ///
    /// # Arguments
    ///
    /// * `thread_name` - Name of the thread obtaining exclusive access. Use `c"thread_name"` to pass a C-style string.
    ///   Required to be a static string for safety reasons.
    pub fn exclusive_access_begin(&self, thread_name: Option<&'static CStr>) {
        let name_ptr = thread_name.map_or(core::ptr::null(), CStr::as_ptr);

        // SAFETY: `self.raw_world` is a valid, non-null pointer to a live flecs world;
        // `name_ptr` is either null or the pointer of a `'static` CStr, so it remains
        // valid for as long as flecs may retain it.
        unsafe {
            sys::ecs_exclusive_access_begin(self.raw_world.as_ptr(), name_ptr);
        }
    }

    /// End exclusive thread access to the world.
    ///
    /// # Panics
    ///
    /// This operation must be called from the same thread that called
    /// `exclusive_access_begin()`. Calling it from a different thread will cause
    /// a panic.
    ///
    /// This operation should be called after `exclusive_access_begin()`. After
    /// calling this operation, other threads are no longer prevented from mutating
    /// the world.
    ///
    /// When `lock_world` is set to true, no thread will be able to mutate the world
    /// until `exclusive_access_begin()` is called again. While the world is locked,
    /// only read-only operations are allowed. For example, `get` without mutable access is allowed,
    /// but `get` with mutable access is not allowed.
    ///
    /// A locked world can be unlocked by calling `exclusive_access_end()` again with
    /// `lock_world` set to false. Note that this only works for locked worlds; if
    /// `exclusive_access_end()` is called on a world that has exclusive thread
    /// access from a different thread, a panic will occur.
    ///
    /// # Arguments
    ///
    /// * `lock_world` - When true, any mutations on the world will be blocked.
    ///
    /// # Note
    ///
    /// This feature only works in builds where asserts are enabled. The
    /// feature requires the OS API `thread_self_` callback to be set.
    pub fn exclusive_access_end(&self, lock_world: bool) {
        // SAFETY: `self.raw_world` is a valid, non-null pointer to a live flecs world
        // for the duration of `&self`; `ecs_exclusive_access_end` only mutates internal
        // world lock/thread-ownership state and requires no other preconditions.
        unsafe {
            sys::ecs_exclusive_access_end(self.raw_world.as_ptr(), lock_world);
        }
    }
}
