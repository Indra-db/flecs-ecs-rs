use core::ffi::c_void;
use core::marker::PhantomData;
use core::ptr::NonNull;

use super::*;

use flecs_ecs_derive::extern_abi;

#[extern_abi]
unsafe fn c_run_post_frame(world: *mut sys::ecs_world_t, ctx: *mut ::core::ffi::c_void) {
    // ctx is a Box<fn(WorldRef)> created in run_post_frame. Flecs invokes this
    // callback exactly once per registration, so the Box is reclaimed here.
    let action: fn(WorldRef) = unsafe { *Box::from_raw(ctx as *mut fn(WorldRef)) };
    let world = unsafe { WorldRef::from_ptr(world) };
    (action)(world);
}

#[extern_abi]
unsafe fn c_on_destroyed(world: *mut sys::ecs_world_t, ctx: *mut ::core::ffi::c_void) {
    // ctx is a Box<fn(WorldRef)> created in on_destroyed. Flecs invokes atfini
    // callbacks exactly once when the world is destroyed, so reclaim the Box here.
    let action: fn(WorldRef) = unsafe { *Box::from_raw(ctx as *mut fn(WorldRef)) };
    let world = unsafe { WorldRef::from_ptr(world) };
    (action)(world);
}

impl World {
    /// deletes and recreates the world
    ///
    /// # Panics
    /// This function panics if lingering references to `Query` and `World` objects are still present.
    ///
    /// # Example
    ///
    /// ```
    /// use flecs_ecs::prelude::*;
    ///
    /// let world = World::new();
    ///
    /// let e = world.entity().id();
    ///
    /// assert!(world.exists(e));
    /// let new_world = world.reset();
    /// assert!(!new_world.exists(e));
    /// ```
    pub fn reset(self) -> Self {
        if unsafe { sys::flecs_poly_refcount(self.raw_world.as_ptr() as *mut c_void) } > 1 {
            panic!("Reset would invalidate other world handles that are still lingering in the user's code base. 
            This is a bug in the user code. Please ensure that all world handles are out of scope before calling `reset`.");
        }
        drop(self);
        World::new()
    }

    /// obtain pointer to C world object.
    ///
    /// # Returns
    ///
    /// Returns a pointer to the C world object.
    #[inline(always)]
    #[doc(hidden)]
    pub fn ptr_mut(&self) -> *mut sys::ecs_world_t {
        self.raw_world.as_ptr()
    }

    /// Get the world's info. See [`sys::WorldInfo`] for what information you can retrieve.
    ///
    /// # Example
    /// ```
    /// use flecs_ecs::prelude::*;
    ///
    /// let mut world = World::new();
    ///
    /// world.progress();
    ///
    /// let world_info = world.info();
    ///
    /// assert!(world_info.delta_time > 0.0);
    /// //assert!(world_info.world_time_total_raw > 0.0); //BUG TODO
    /// //assert!(world_info.systems_ran_frame == 0);
    /// ```
    pub fn info(&self) -> sys::WorldInfo {
        // SAFETY: The pointer is valid for the lifetime of the world.
        unsafe { *sys::ecs_get_world_info(self.raw_world.as_ptr()) }
    }

    /// Signals the application to quit.
    ///
    /// After calling this function, the next call to [`World::progress()`] returns false.
    ///
    /// # Example
    /// ```
    /// use flecs_ecs::prelude::*;
    ///
    /// let mut world = World::new();
    ///
    /// let mut count = 0;
    /// while world.progress() {
    ///     count += 1;
    ///     if count == 5 {
    ///         world.quit();
    ///     }
    /// }
    /// assert!(count == 5);
    /// ```
    ///
    /// # See also
    ///
    /// * [`World::should_quit()`]
    pub fn quit(&self) {
        unsafe {
            sys::ecs_quit(self.raw_world.as_ptr());
        }
    }

    /// Tests if [`World::quit()`] has been called.
    ///
    /// # Returns
    ///
    /// True if quit has been called, false otherwise.
    ///
    /// # Example
    ///
    /// ```
    /// use flecs_ecs::prelude::*;
    ///
    /// let world = World::new();
    ///
    /// assert!(!world.should_quit());
    /// world.quit();
    /// assert!(world.should_quit());
    /// ```
    ///
    /// # See also
    ///
    /// * [`World::quit()`]
    pub fn should_quit(&self) -> bool {
        unsafe { sys::ecs_should_quit(self.raw_world.as_ptr()) }
    }

    /// Registers an action to be executed when the world is destroyed.
    ///
    /// This provides a safe, ergonomic way to register cleanup callbacks that will
    /// be invoked when the world is dropped. The callback receives a [`WorldRef`]
    /// that can be used to access world state during cleanup.
    ///
    /// # Arguments
    ///
    /// * `action` - The function to call when the world is destroyed.
    ///
    /// # Example
    ///
    /// ```
    /// use flecs_ecs::prelude::*;
    ///
    /// let world = World::new();
    /// world.on_destroyed(|world| {
    ///     println!("World is being destroyed!");
    /// });
    /// // World will be destroyed when it goes out of scope
    /// ```
    #[doc(alias = "ecs_atfini")]
    pub fn on_destroyed(&self, action: fn(WorldRef)) {
        let ctx = Box::into_raw(Box::new(action)) as *mut ::core::ffi::c_void;
        unsafe {
            sys::ecs_atfini(self.raw_world.as_ptr(), Some(c_on_destroyed), ctx);
        }
    }

    /// Begins a frame.
    ///
    /// When an application does not use [`World::progress()`] to control the main loop, it
    /// can still use Flecs features such as FPS limiting and time measurements processed.
    ///
    /// Calls to [`World::frame_begin`] must always be followed by [`World::frame_end`].
    ///
    /// The function accepts a `delta_time` parameter, which will get passed to
    /// systems. This value is also used to compute the amount of time the
    /// function needs to sleep to ensure it does not exceed the `target_fps`, when
    /// it is set. When 0 is provided for `delta_time`, the time will be measured.
    ///
    /// # Thread Safety
    /// This function should only be run from the main thread.
    ///
    /// # Arguments
    /// * `delta_time`: Time elapsed since the last frame.
    ///
    /// # Returns
    /// The provided `delta_time`, or the measured time if 0 was provided.
    ///
    /// # Example
    ///
    /// ```
    /// use flecs_ecs::prelude::*;
    ///
    /// #[derive(Component)]
    /// struct Position {
    ///     x: f32,
    ///     y: f32,
    /// }
    ///
    /// let world = World::new();
    ///
    /// let entity = world.entity().set(Position { x: 5.0, y: 0.0 });
    ///
    /// let sys = world.system::<&Position>().each(|pos| {});
    ///
    /// let world_info = world.info();
    ///
    /// assert_eq!(world_info.systems_ran_total, 0);
    ///
    /// let dt = world.frame_begin(0.0);
    ///
    /// world.frame_end();
    ///
    /// let world_info = world.info();
    ///
    /// //TODO
    /// //assert_eq!(world_info.systems_ran_frame, 1);
    /// ```
    ///
    /// # See also
    ///
    /// * [`World::frame_end()`]
    pub fn frame_begin(&self, delta_time: f32) -> f32 {
        unsafe { sys::ecs_frame_begin(self.raw_world.as_ptr(), delta_time) }
    }

    /// Ends a frame.
    ///
    /// This operation must be called at the end of the frame, and always after
    /// [`World::frame_begin()`].
    ///
    /// # Thread Safety
    /// The function should only be run from the main thread.
    ///
    /// # See also
    ///
    /// * [`World::frame_begin()`]
    pub fn frame_end(&self) {
        unsafe {
            sys::ecs_frame_end(self.raw_world.as_ptr());
        }
    }

    /// Begin readonly mode.
    ///
    /// When an application does not use [`World::progress()`] to control the main loop,
    /// it can still use Flecs features such as the defer queue. To stage changes, this function
    /// must be called after [`World::frame_begin()`].
    ///
    /// A call to [`World::readonly_begin()`] must be followed by a call to
    /// [`World::readonly_end()`].
    ///
    /// When staging is enabled, modifications to entities are stored to a stage.
    /// This ensures that arrays are not modified while iterating. Modifications are
    /// merged back to the "main stage" when [`World::readonly_end()`] is invoked.
    ///
    /// While the world is in staging mode, no structural changes (add/remove/...) can
    /// be made to the world itself. Operations must be executed on a stage instead
    /// (see [`World::stage()`]).
    ///
    /// Readonly mode is a stronger version of deferred mode. In deferred mode,
    /// ECS operations such as add/remove/set/delete etc. are added to a command
    /// queue to be executed later. In readonly mode, operations that could break
    /// scheduler logic (such as creating systems, queries) are also disallowed.
    ///
    /// Readonly mode itself has a single-threaded and a multi-threaded mode. In
    /// single-threaded mode certain mutations on the world are still allowed, for example:
    /// - Entity liveliness operations (such as new, `make_alive`), so that systems are
    ///   able to create new entities.
    /// - Implicit component registration, so that this works from systems.
    /// - Mutations to supporting data structures for the evaluation of uncached
    ///   queries, so that these can be created on the fly.
    ///
    /// These mutations are safe in single-threaded applications, but for
    /// multi-threaded applications, the world needs to be entirely immutable. For this
    /// purpose, multi-threaded readonly mode exists, which disallows all mutations on
    /// the world.
    ///
    /// While in readonly mode, applications can still enqueue ECS operations on a
    /// stage. Stages are managed automatically when using the pipeline addon and
    /// [`World::progress()`], but they can also be configured manually.
    ///
    /// Number of stages typically corresponds with number of threads
    ///
    /// When an attempt is made to perform an operation on a world in readonly mode,
    /// the code will throw an assert saying that the world is in readonly mode.
    ///
    /// A call to `readonly_begin` must be followed up with `readonly_end()`.
    /// When `readonly_end()` is called, all enqueued commands from configured
    /// stages are merged back into the world. Calls to `readonly_begin()` and
    /// `readonly_end()` should always happen from a context where the code has
    /// exclusive access to the world. The functions themselves are not thread safe.
    ///
    /// # Thread Safety
    /// This function should only be run from the main thread.
    ///
    /// # Returns
    /// Whether the world is currently staged and whether it is in readonly mode.
    ///
    /// # Example
    ///
    /// ```
    /// use flecs_ecs::prelude::*;
    ///
    /// #[derive(Component)]
    /// struct Position {
    ///     x: i32,
    ///     y: i32,
    /// }
    ///
    /// let world = World::new();
    ///
    /// let stage = world.stage(0);
    ///
    /// world.readonly_begin(false);
    ///
    /// assert_eq!(stage.count(Position::id()), 0);
    ///
    /// world.readonly_end();
    ///
    /// world.readonly_begin(false);
    ///
    /// stage.entity().set(Position { x: 10, y: 20 });
    /// stage.entity().set(Position { x: 10, y: 20 });
    ///
    /// assert_eq!(stage.count(Position::id()), 0);
    ///
    /// world.readonly_end();
    ///
    /// assert_eq!(stage.count(Position::id()), 2);
    /// ```
    ///
    /// # See also
    ///
    /// * [`World::is_readonly()`]
    /// * [`World::readonly_end()`]
    pub fn readonly_begin(&self, multi_threaded: bool) -> bool {
        unsafe { sys::ecs_readonly_begin(self.raw_world.as_ptr(), multi_threaded) }
    }

    /// End readonly mode.
    ///
    /// Leaves staging mode. After this operation, the world may be directly mutated again.
    /// By default, this operation also merges data back into the world, unless auto-merging
    /// was disabled explicitly.
    ///
    /// # Thread Safety
    /// This function should only be run from the main thread.
    ///
    /// # Returns
    ///
    /// Whether the world is currently staged.
    ///
    /// # Example
    ///
    /// see [`World::readonly_begin()`].
    ///
    /// # See also
    ///
    /// * [`World::is_readonly()`]
    /// * [`World::readonly_begin()`]
    pub fn readonly_end(&self) {
        unsafe {
            sys::ecs_readonly_end(self.raw_world.as_ptr());
        }
    }

    /// Test whether the current world object is readonly.
    ///
    /// This function allows the code to test whether the currently used world
    /// object is readonly or whether it allows for writing.
    ///
    /// # Returns
    ///
    /// True if the world or stage is readonly.
    ///
    /// # Example
    ///
    /// ```
    /// use flecs_ecs::prelude::*;
    ///
    /// let world = World::new();
    ///
    /// assert!(!world.is_readonly());
    ///
    /// world.readonly_begin(false);
    ///
    /// assert!(world.is_readonly());
    ///
    /// world.readonly_end();
    ///
    /// assert!(!world.is_readonly());
    /// ```
    ///
    /// # See also
    ///
    /// * [`World::readonly_begin()`]
    /// * [`World::readonly_end()`]
    pub fn is_readonly(&self) -> bool {
        unsafe { sys::ecs_stage_is_readonly(self.raw_world.as_ptr()) }
    }

    /// Defers operations until the end of the frame.
    ///
    /// When this operation is invoked while iterating, the operations between
    /// [`World::defer_begin()`] and [`World::defer_end()`] are executed at the
    /// end of the frame.
    ///
    /// # Thread Safety
    /// This operation is thread safe.
    ///
    /// # Returns
    /// Whether the operation was successful.
    ///
    /// # Example
    ///
    /// ```
    /// use flecs_ecs::prelude::*;
    ///
    /// #[derive(Component)]
    /// struct Position {
    ///     x: i32,
    ///     y: i32,
    /// }
    ///
    /// let world = World::new();
    ///
    /// world.defer_begin();
    ///
    /// let e = world.entity().set(Position { x: 10, y: 20 });
    ///
    /// assert!(!e.has(Position::id()));
    ///
    /// world.defer_end();
    ///
    /// assert!(e.has(Position::id()));
    /// ```
    ///
    /// # See also
    ///
    /// * [`World::defer()`]
    /// * [`World::defer_end()`]
    /// * [`World::defer_suspend()`]
    /// * [`World::defer_resume()`]
    /// * [`World::is_deferred()`]
    pub fn defer_begin(&self) -> bool {
        unsafe { sys::ecs_defer_begin(self.raw_world.as_ptr()) }
    }

    /// Ends a block of operations to defer.
    ///
    /// This should follow a [`World::defer_begin()`] call.
    ///
    /// # Thread Safety
    /// This operation is thread safe.
    ///
    /// # Returns
    /// Whether the operation was successful.
    ///
    /// # Example
    ///
    /// see [`World::defer_begin`]
    ///
    /// # See also
    ///
    /// * [`World::defer()`]
    /// * [`World::defer_begin()`]
    /// * [`World::defer_suspend()`]
    /// * [`World::defer_resume()`]
    /// * [`World::is_deferred()`]
    pub fn defer_end(&self) -> bool {
        unsafe { sys::ecs_defer_end(self.raw_world.as_ptr()) }
    }

    /// Test whether deferring is enabled.
    ///
    /// # Returns
    ///
    /// Whether deferring is enabled.
    ///
    /// # Example
    ///
    /// ```
    /// use flecs_ecs::prelude::*;
    ///
    /// let world = World::new();
    ///
    /// assert!(!world.is_deferred());
    ///
    /// world.defer_begin();
    ///
    /// assert!(world.is_deferred());
    ///
    /// world.defer_end();
    ///
    /// assert!(!world.is_deferred());
    /// ```
    ///
    /// # See also
    ///
    /// * [`World::defer()`]
    /// * [`World::defer_begin()`]
    /// * [`World::defer_end()`]
    /// * [`World::defer_suspend()`]
    /// * [`World::defer_resume()`]
    pub fn is_deferred(&self) -> bool {
        unsafe { sys::ecs_is_deferred(self.raw_world.as_ptr()) }
    }

    /// Test if deferring is suspended for the current stage
    /// (see [`World::defer_suspend()`]).
    pub fn is_defer_suspended(&self) -> bool {
        unsafe { sys::ecs_is_defer_suspended(self.raw_world.as_ptr()) }
    }

    /// Defers all operations executed in the passed-in closure.
    ///
    /// # Arguments
    ///
    /// * `func` - The closure to execute.
    ///
    /// # Examples
    /// ```
    /// # use flecs_ecs::core::World;
    /// # let world = World::new();
    /// let return_something_if_wanted = world.defer(|| {
    ///     // deferred operations here
    /// });
    /// ```
    ///
    /// # See also
    ///
    /// * [`World::defer_begin()`]
    /// * [`World::defer_end()`]
    /// * [`World::defer_suspend()`]
    /// * [`World::defer_resume()`]
    /// * [`World::is_deferred()`]
    pub fn defer<T>(&self, func: impl FnOnce() -> T) -> T {
        unsafe {
            sys::ecs_defer_begin(self.raw_world.as_ptr());
        }
        let result = func();
        unsafe {
            sys::ecs_defer_end(self.raw_world.as_ptr());
        }
        result
    }

    /// Suspends deferring of operations but do flush the queue.
    ///
    /// This operation can be used to do an undeferred operation
    /// while not flushing the operations in the queue.
    ///
    /// An application should invoke [`World::defer_resume()`] before
    /// [`World::defer_end()`] is called. The operation may only be called
    /// when deferring is enabled.
    ///
    /// # See also
    ///
    /// * [`World::defer()`]
    /// * [`World::defer_begin()`]
    /// * [`World::defer_end()`]
    /// * [`World::defer_resume()`]
    /// * [`World::is_deferred()`]
    pub fn defer_suspend(&self) {
        unsafe {
            sys::ecs_defer_suspend(self.raw_world.as_ptr());
        }
    }

    /// Resumes deferring of operations.
    ///
    /// # See also
    ///
    /// * [`World::defer()`]
    /// * [`World::defer_begin()`]
    /// * [`World::defer_end()`]
    /// * [`World::defer_suspend()`]
    /// * [`World::is_deferred()`]
    pub fn defer_resume(&self) {
        unsafe {
            sys::ecs_defer_resume(self.raw_world.as_ptr());
        }
    }

    /// Configure world to have N stages.
    ///
    /// This initializes N stages, which allows applications to defer operations to
    /// multiple isolated defer queues. This is typically used for applications with
    /// multiple threads, where each thread gets its own queue, and commands are
    /// merged when threads are synchronized.
    ///
    /// Note that [`World::set_threads()`] already creates the appropriate number of stages.
    /// The [`World::set_stage_count()`] operation is useful for applications that want to manage
    /// their own stages and/or threads.
    ///
    /// # Arguments
    ///
    /// * `stages`: The number of stages.
    ///
    /// # Example
    ///
    /// ```
    /// use flecs_ecs::prelude::*;
    ///
    /// let world = World::new();
    ///
    /// world.set_stage_count(2);
    ///
    /// world.readonly_begin(false);
    ///
    /// let stage1 = world.stage(0);
    ///
    /// let e1 = stage1.entity_named("e1");
    ///
    /// world.readonly_end();
    ///
    /// assert!(e1.id() != 0);
    /// assert_eq!(e1.name(), "e1");
    /// ```
    ///
    /// # See also
    ///
    /// * [`World::get_stage_count()`]
    /// * [`World::is_stage()`]
    /// * [`World::merge()`]
    /// * [`World::stage()`]
    /// * [`World::stage_id()`]
    pub fn set_stage_count(&self, stages: i32) {
        unsafe {
            sys::ecs_set_stage_count(self.raw_world.as_ptr(), stages);
        }
        #[cfg(feature = "flecs_safety_locks")]
        self.world_ctx()
            .safety_locks
            .set_stage_count(unsafe { sys::ecs_get_stage_count(self.raw_world.as_ptr()) });
    }

    /// Get number of configured stages.
    ///
    /// Return number of stages set by [`World::set_stage_count()`].
    ///
    /// # Returns
    ///
    /// The number of stages used for threading.
    ///
    /// # Example
    ///
    /// ```
    /// use flecs_ecs::prelude::*;
    ///
    /// let world = World::new();
    ///
    /// assert_eq!(world.get_stage_count(), 1);
    ///
    /// world.set_stage_count(4);
    ///
    /// assert_eq!(world.get_stage_count(), 4);
    /// ```
    ///
    /// # See also
    ///
    /// * [`World::is_stage()`]
    /// * [`World::merge()`]
    /// * [`World::set_stage_count()`]
    /// * [`World::stage()`]
    /// * [`World::stage_id()`]
    pub fn get_stage_count(&self) -> i32 {
        unsafe { sys::ecs_get_stage_count(self.raw_world.as_ptr()) }
    }

    /// Get current stage id.
    ///
    /// The stage id can be used by an application to learn about which stage it
    /// is using, which typically corresponds with the worker thread id.
    ///
    /// # Returns
    ///
    /// The stage id.
    ///
    /// # Example
    ///
    /// ```
    /// use flecs_ecs::prelude::*;
    ///
    /// let world = World::new();
    ///
    /// assert_eq!(world.stage_id(), 0);
    ///
    /// world.set_stage_count(4);
    ///
    /// assert_eq!(world.stage_id(), 0);
    ///
    /// let stage = world.stage(3);
    ///
    /// assert_eq!(stage.stage_id(), 3);
    /// ```
    ///
    /// # See also
    ///
    /// * [`World::get_stage_count()`]
    /// * [`World::is_stage()`]
    /// * [`World::merge()`]
    /// * [`World::set_stage_count()`]
    /// * [`World::stage()`]
    pub fn stage_id(&self) -> i32 {
        unsafe { sys::ecs_stage_get_id(self.raw_world.as_ptr()) }
    }

    /// Test if is a stage.
    ///
    /// If this function returns `false`, it is guaranteed that this is a valid
    /// world object.
    ///
    /// # Returns
    ///
    /// True if the world is a stage, false if not.
    ///
    /// # Example
    ///
    /// ```
    /// use flecs_ecs::prelude::*;
    ///
    /// let world = World::new();
    ///
    /// assert!(!world.is_stage());
    ///
    /// let stage = world.stage(0);
    ///
    /// assert!(stage.is_stage());
    /// ```
    ///
    /// # See also
    ///
    /// * [`World::get_stage_count()`]
    /// * [`World::merge()`]
    /// * [`World::set_stage_count()`]
    /// * [`World::stage()`]
    /// * [`World::stage_id()`]
    pub fn is_stage(&self) -> bool {
        unsafe {
            ecs_assert!(
                sys::flecs_poly_is_(
                    self.raw_world.as_ptr() as *const c_void,
                    sys::ecs_world_t_magic as i32
                ) || sys::flecs_poly_is_(
                    self.raw_world.as_ptr() as *const c_void,
                    sys::ecs_stage_t_magic as i32
                ),
                FlecsErrorCode::InvalidParameter,
                "flecs::world instance contains invalid reference to world or stage"
            );
            sys::flecs_poly_is_(
                self.raw_world.as_ptr() as *const c_void,
                sys::ecs_stage_t_magic as i32,
            )
        }
    }

    /// Merge world or stage.
    ///
    /// When automatic merging is disabled, an application can call this
    /// operation on either an individual stage, or on the world which will merge
    /// all stages. This operation may only be called when staging is not enabled
    /// (either after [`World::progress()`] or after [`World::readonly_end()`]).
    ///
    /// This operation may be called on an already merged stage or world.
    ///
    /// # Example
    ///
    /// ```
    /// use flecs_ecs::prelude::*;
    ///
    /// #[derive(Component)]
    /// struct Position {
    ///     x: i32,
    ///     y: i32,
    /// }
    ///
    /// let world = World::new();
    ///
    /// let e = world.entity();
    ///
    /// let stage = world.create_async_stage();
    ///
    /// e.mut_current_stage(stage).set(Position { x: 10, y: 20 });
    ///
    /// assert!(!e.has(Position::id()));
    ///
    /// stage.merge();
    ///
    /// assert!(e.has(Position::id()));
    /// ```
    ///
    /// # See also
    ///
    /// * [`World::get_stage_count()`]
    /// * [`World::is_stage()`]
    /// * [`World::set_stage_count()`]
    /// * [`World::stage()`]
    /// * [`World::stage_id()`]
    pub fn merge(&self) {
        unsafe { sys::ecs_merge(self.raw_world.as_ptr()) };
    }

    /// Get stage-specific world pointer.
    ///
    /// Flecs threads can safely invoke the API as long as they have a private
    /// context to write to, also referred to as the stage. This function returns a
    /// pointer to a stage, disguised as a world pointer.
    ///
    /// Note that this function does not(!) create a new world. It simply wraps the
    /// existing world in a thread-specific context, which the API knows how to
    /// unwrap. The reason the stage is returned as an [`sys::ecs_world_t`] is so that
    /// it can be passed transparently to the existing API functions, vs. having to
    /// create a dedicated API for threading.
    ///
    /// # Arguments
    ///
    /// * `stage_id` - The index of the stage to retrieve.
    ///
    /// # Returns
    ///
    /// A thread-specific pointer to the world.
    ///
    /// # Example
    ///
    /// ```
    /// use flecs_ecs::prelude::*;
    ///
    /// let world = World::new();
    ///
    /// assert_eq!(world.stage_id(), 0);
    ///
    /// world.set_stage_count(4);
    ///
    /// assert_eq!(world.stage_id(), 0);
    ///
    /// let stage = world.stage(3);
    ///
    /// assert_eq!(stage.stage_id(), 3);
    /// ```
    ///
    /// # See also
    ///
    /// * [`World::get_stage_count()`]
    /// * [`World::is_stage()`]
    /// * [`World::merge()`]
    /// * [`World::set_stage_count()`]
    /// * [`World::stage_id()`]
    pub fn stage(&self, stage_id: i32) -> WorldRef<'_> {
        unsafe { WorldRef::from_ptr(sys::ecs_get_stage(self.raw_world.as_ptr(), stage_id)) }
    }

    /// Create asynchronous stage.
    ///
    /// An asynchronous stage can be used to asynchronously queue operations for
    /// later merging with the world. An asynchronous stage is similar to a regular
    /// stage, except that it does not allow reading from the world.
    ///
    /// Asynchronous stages are never merged automatically, and must therefore be
    /// manually merged with the [`World::merge()`] function. It is not necessary to call
    /// [`World::defer_begin()`] or [`World::defer_end()`] before and after enqueuing commands,
    /// as an asynchronous stage unconditionally defers operations.
    ///
    /// The application must ensure that no commands are added to the stage while the
    /// stage is being merged.
    ///
    /// Like all stage handles, the returned [`WorldRef`] is `!Send`/`!Sync`:
    /// asynchronous stages are a tool for batching commands on the world's
    /// owning thread and merging them at a chosen point, not for recording
    /// commands from other threads. Cross-thread query access goes through
    /// [`QueryHandle`][crate::core::QueryHandle] instead.
    ///
    /// The returned [`WorldRef`] does not own the stage: the underlying stage is
    /// **not** freed when the [`WorldRef`] is dropped and stays allocated until the
    /// process exits. Use [`World::async_stage()`] instead to get an owning handle
    /// that frees the stage when dropped.
    ///
    /// # Returns
    ///
    /// The stage.
    ///
    /// # Example
    ///
    /// ```
    /// use flecs_ecs::prelude::*;
    ///
    /// #[derive(Component)]
    /// struct Position {
    ///     x: i32,
    ///     y: i32,
    /// }
    ///
    /// let world = World::new();
    ///
    /// let e = world.entity();
    ///
    /// let stage = world.create_async_stage();
    ///
    /// e.mut_current_stage(stage).set(Position { x: 10, y: 20 });
    ///
    /// assert!(!e.has(Position::id()));
    ///
    /// stage.merge();
    ///
    /// assert!(e.has(Position::id()));
    /// ```
    pub fn create_async_stage(&self) -> WorldRef<'_> {
        unsafe { WorldRef::from_ptr(sys::ecs_stage_new(self.raw_world.as_ptr())) }
    }

    /// Create an asynchronous stage that is freed when the returned handle is dropped.
    ///
    /// Same semantics as [`World::create_async_stage()`], except the returned
    /// [`AsyncStage`] owns the stage and frees it on drop, so the stage does not leak.
    /// Use [`AsyncStage::stage()`] to enqueue commands on the stage and
    /// [`AsyncStage::merge()`] to merge them into the world.
    ///
    /// [`AsyncStage`] is `!Send`/`!Sync` and stays on the world's owning
    /// thread; see [`World::create_async_stage()`] for what stages are (and
    /// are not) for.
    ///
    /// # Example
    ///
    /// ```
    /// use flecs_ecs::prelude::*;
    ///
    /// #[derive(Component)]
    /// struct Position {
    ///     x: i32,
    ///     y: i32,
    /// }
    ///
    /// let world = World::new();
    ///
    /// let e = world.entity();
    ///
    /// let stage = world.async_stage();
    ///
    /// e.mut_current_stage(stage.stage()).set(Position { x: 10, y: 20 });
    ///
    /// assert!(!e.has(Position::id()));
    ///
    /// stage.merge();
    ///
    /// assert!(e.has(Position::id()));
    /// ```
    ///
    /// # See also
    ///
    /// * [`World::create_async_stage()`]
    /// * [`World::merge()`]
    pub fn async_stage(&self) -> AsyncStage<'_> {
        AsyncStage::new(self)
    }

    /// Get actual world.
    ///
    /// If the current object points to a stage, this operation will return the
    /// actual world.
    ///
    /// # Returns
    ///
    /// The actual world.
    ///
    /// # Example
    ///
    /// ```
    /// use flecs_ecs::prelude::*;
    ///
    /// let world = World::new();
    ///
    /// let stage = world.stage(0);
    ///
    /// let world_ref = stage.real_world();
    ///
    /// assert!(!world_ref.is_stage());
    /// ```
    ///
    /// # See also
    ///
    /// * [`World::is_stage()`]
    /// * [`World::stage()`]
    /// * [`WorldRef`]
    pub fn real_world(&self) -> WorldRef<'_> {
        self.world().real_world()
    }

    /// Set world context.
    ///
    /// Set a context value that can be accessed by anyone that has a reference
    /// to the world.
    ///
    /// # Arguments
    ///
    /// * `ctx` - The world context.
    /// * `ctx_free` - The free function for the context. Can pass `None` if no free function is needed.
    ///
    /// # Example
    ///
    /// ```
    /// use core::ffi::c_void;
    /// use flecs_ecs::prelude::*;
    ///
    /// #[extern_abi]
    /// fn free_ctx(ctx: *mut c_void) {
    ///     unsafe {
    ///         Box::from_raw(ctx as *mut i32);
    ///     }
    /// }
    ///
    /// let world = World::new();
    ///
    /// let ctx = Box::leak(Box::new(42));
    ///
    /// world.set_context(ctx as *mut i32 as *mut c_void, Some(free_ctx));
    ///
    /// assert_eq!(world.context() as *const i32, ctx);
    /// ```
    ///
    /// # See also
    ///
    /// * [`World::context()`]
    #[expect(
        clippy::not_unsafe_ptr_arg_deref,
        reason = "this doesn't actually deref the pointer and controls lifetime"
    )]
    pub fn set_context(&self, ctx: *mut c_void, ctx_free: sys::ecs_ctx_free_t) {
        unsafe { sys::ecs_set_ctx(self.raw_world.as_ptr(), ctx, ctx_free) }
    }

    /// Get world context.
    ///
    /// # Returns
    ///
    /// The configured world context.
    ///
    /// # Example
    ///
    /// See [`World::set_context`].
    ///
    /// # See also
    ///
    /// * [`World::set_context()`]
    pub fn context(&self) -> *mut c_void {
        unsafe { sys::ecs_get_ctx(self.raw_world.as_ptr()) }
    }

    /// Store a typed world context (spec §5.3).
    ///
    /// The capture-based replacement for the `*mut c_void`
    /// [`set_context`](World::set_context): the value is owned by the world and
    /// dropped when the world is dropped. Read it back with [`World::ctx`]. A
    /// previously stored value is replaced (and dropped).
    ///
    /// ```
    /// use flecs_ecs::prelude::*;
    ///
    /// let mut world = World::new();
    /// world.set_ctx(42u32);
    /// assert_eq!(world.ctx::<u32>(), Some(&42));
    /// assert_eq!(world.ctx::<i8>(), None); // wrong type
    /// ```
    pub fn set_ctx<C: 'static>(&mut self, value: C) {
        // SAFETY: `&mut self` proves exclusive access to this world, so the
        // binding-context struct behind the world pointer can be mutated.
        let ctx = unsafe {
            &mut *(sys::ecs_get_binding_ctx(self.raw_world.as_ptr()) as *mut crate::core::world_ctx::WorldCtx)
        };
        ctx.typed_ctx = Some(alloc::boxed::Box::new(value));
    }

    /// Read the typed world context set by [`World::set_ctx`], or `None` if none
    /// was set or the stored value has a different type.
    pub fn ctx<C: 'static>(&self) -> Option<&C> {
        self.world_ctx()
            .typed_ctx
            .as_ref()
            .and_then(|b| b.downcast_ref::<C>())
    }

    /// Mutably read the typed world context set by [`World::set_ctx`], or `None`.
    pub fn ctx_mut<C: 'static>(&mut self) -> Option<&mut C> {
        // SAFETY: `&mut self` proves exclusive access to this world.
        let ctx = unsafe {
            &mut *(sys::ecs_get_binding_ctx(self.raw_world.as_ptr()) as *mut crate::core::world_ctx::WorldCtx)
        };
        ctx.typed_ctx.as_mut().and_then(|b| b.downcast_mut::<C>())
    }

    #[expect(dead_code, reason = "possibly used in the future")]
    pub(crate) fn get_context(world: *mut sys::ecs_world_t) -> *mut WorldCtx {
        unsafe { sys::ecs_get_binding_ctx(world) as *mut WorldCtx }
    }

    pub(crate) fn get_components_map_ptr(world: *mut sys::ecs_world_t) -> *mut FlecsIdMap {
        unsafe { &mut (*(sys::ecs_get_binding_ctx(world) as *mut WorldCtx)).components }
    }

    pub(crate) fn components_map(&self) -> &'static mut FlecsIdMap {
        unsafe { &mut (*(self.components.as_ptr())) }
    }

    pub(crate) fn get_components_array_ptr(world: *mut sys::ecs_world_t) -> *mut FlecsArray {
        unsafe { &mut (*(sys::ecs_get_binding_ctx(world) as *mut WorldCtx)).components_array }
    }

    pub(crate) fn components_array(&self) -> &'static mut FlecsArray {
        unsafe { &mut (*(self.components_array.as_ptr())) }
    }

    /// Resets all cached component ids for this world to zero.
    ///
    /// Intended for benchmarks and tests that repeatedly measure component
    /// registration on the same world. After this call, component ids are
    /// resolved again (or re-registered) on next use.
    #[doc(hidden)]
    pub fn reset_cached_component_ids(&self) {
        for id in self.components_map().values_mut() {
            *id = 0;
        }
        for id in self.components_array().iter_mut() {
            *id = 0;
        }
        #[cfg(feature = "flecs_experimental")]
        self.world_ctx().bundle_ids.borrow_mut().clear();
    }

    /// Set world binding context
    /// Set a context value that can be accessed by anyone that has a reference to the world.
    ///
    /// # Arguments
    ///
    /// * `ctx` - The world context.
    /// * `ctx_free` - The free function for the context. Can pass `None` if no free function is needed.
    ///
    /// # See also
    ///
    /// * [`World::get_binding_context()`]
    #[expect(dead_code, reason = "possibly used in the future")]
    pub(crate) fn set_binding_context(&self, ctx: *mut c_void, ctx_free: sys::ecs_ctx_free_t) {
        unsafe { sys::ecs_set_binding_ctx(self.raw_world.as_ptr(), ctx, ctx_free) }
    }

    /// Get world binding context.
    ///
    /// # Returns
    ///
    /// The configured world context.
    ///
    /// # See also
    ///
    /// * [`World::set_binding_context()`]
    #[expect(dead_code, reason = "possibly used in the future")]
    pub(crate) fn get_binding_context(&self) -> *mut c_void {
        unsafe { sys::ecs_get_binding_ctx(self.raw_world.as_ptr()) }
    }

    /// Preallocate memory for a number of entities.
    ///
    /// This function preallocates memory for the entity index.
    ///
    /// # Arguments
    ///
    /// * `entity_count` - Number of entities to preallocate memory for.
    pub fn preallocate_entity_count(&self, entity_count: i32) {
        unsafe { sys::ecs_dim(self.raw_world.as_ptr(), entity_count) };
    }

    /// Free unused memory.
    ///
    /// This operation frees allocated memory that is no longer in use by the world.
    /// Examples of allocations that get cleaned up are:
    /// - Unused pages in the entity index
    /// - Component columns
    /// - Empty tables
    ///
    /// Flecs uses allocators internally for speeding up allocations. Allocators are
    /// not evaluated by this function, which means that the memory reported by the
    /// OS may not go down. For this reason, this function is most effective when
    /// combined with `FLECS_USE_OS_ALLOC`, which disables internal allocators.
    pub fn shrink_memory(&self) {
        unsafe { sys::ecs_shrink(self.raw_world.as_ptr()) };
    }

    /// Create a new entity id range.
    ///
    /// This function creates a range that constrains new entity identifiers to
    /// the `[min, max]` interval. Each range maintains its own list of recycled
    /// entity ids, which ensures that recycled ids always respect the
    /// configured range. If `max` is set to 0, the range is unbounded.
    ///
    /// Entity ranges cannot be deleted once created. Use
    /// [`World::entity_range_set()`] to activate a range.
    ///
    /// # Arguments
    ///
    /// * `min` - The first entity id in the range (inclusive).
    /// * `max` - The last entity id in the range (inclusive, 0 = unlimited).
    ///
    /// # Example
    ///
    /// ```
    /// use flecs_ecs::prelude::*;
    ///
    /// let world = World::new();
    ///
    /// let range = world.entity_range_new(5000, 0);
    /// world.entity_range_set(range);
    ///
    /// let e = world.entity();
    ///
    /// assert_eq!(e.id(), 5000);
    ///
    /// let e = world.entity();
    ///
    /// assert_eq!(e.id(), 5001);
    /// ```
    ///
    /// # See also
    ///
    /// * [`World::entity_range_set()`]
    /// * [`World::entity_range_get()`]
    pub fn entity_range_new(&self, min: u32, max: u32) -> EntityRange<'_> {
        let ptr = unsafe { sys::ecs_entity_range_new(self.raw_world.as_ptr(), min, max) };
        EntityRange {
            ptr: NonNull::new(ptr.cast_mut()).expect("ecs_entity_range_new returned null"),
            _marker: PhantomData,
        }
    }

    /// Set the active entity id range.
    ///
    /// This function activates a range created with [`World::entity_range_new()`].
    /// When a range is activated, new entity identifiers will fall within the
    /// specified `[min, max]` interval, including recycled identifiers.
    ///
    /// When the active range is out of available ids, operations that create
    /// new entity ids will assert.
    ///
    /// # See also
    ///
    /// * [`World::entity_range_new()`]
    /// * [`World::entity_range_get()`]
    pub fn entity_range_set(&self, range: EntityRange<'_>) {
        unsafe { sys::ecs_entity_range_set(self.raw_world.as_ptr(), range.ptr.as_ptr()) };
    }

    /// Get the currently active entity id range.
    ///
    /// Returns the range set by [`World::entity_range_set()`], or `None` if no
    /// range is active.
    ///
    /// # See also
    ///
    /// * [`World::entity_range_new()`]
    /// * [`World::entity_range_set()`]
    pub fn entity_range_get(&self) -> Option<EntityRange<'_>> {
        let ptr = unsafe { sys::ecs_entity_range_get(self.raw_world.as_ptr()) };
        NonNull::new(ptr.cast_mut()).map(|ptr| EntityRange {
            ptr,
            _marker: PhantomData,
        })
    }

    /// Get the largest issued entity ID (not counting generation).
    pub fn max_id(&self) -> Entity {
        Entity::new(unsafe { sys::ecs_get_max_id(self.raw_world.as_ptr()) })
    }

    /// Invoke `func` for every alive entity in the world, including builtin
    /// and internal entities.
    pub fn each_alive_entity(&self, mut func: impl FnMut(EntityView)) {
        let entities = unsafe { sys::ecs_get_entities(self.raw_world.as_ptr()) };
        for i in 0..entities.alive_count {
            let id = unsafe { *entities.ids.add(i as usize) };
            func(EntityView::new_from(self, id));
        }
    }

    /// Get the current scope. Get the scope set by `set_scope`.
    /// If no scope is set, this operation will return `None`.
    ///
    /// # Returns
    ///
    /// Returns an `EntityView` representing the current scope.
    /// If no scope is set, this operation will return `None`.
    ///
    /// # Example
    ///
    /// ```
    /// use flecs_ecs::prelude::*;
    ///
    /// let world = World::new();
    ///
    /// let e = world.entity_named("scope");
    ///
    /// world.set_scope(e);
    ///
    /// let s = world.get_scope();
    ///
    /// assert_eq!(s.unwrap(), e);
    /// ```
    ///
    /// # See also
    ///
    /// * [`World::set_scope()`]
    #[inline(always)]
    pub fn get_scope(&self) -> Option<EntityView<'_>> {
        let scope = unsafe { sys::ecs_get_scope(self.raw_world.as_ptr()) };

        if scope == 0 {
            None
        } else {
            Some(EntityView::new_from(self, scope))
        }
    }

    /// Set the current scope. This operation sets the scope of the current stage to the provided entity.
    /// As a result new entities will be created in this scope, and lookups will be relative to the provided scope.
    /// It is considered good practice to restore the scope to the old value.
    ///
    /// This method changes the current scope to the entity represented by the provided `id`.
    ///
    /// # Arguments
    ///
    /// * `id` - The ID of the scope entity to set.
    ///
    /// # Returns
    ///
    /// Returns an `EntityView` representing the previous set scope.
    ///
    /// # Example
    ///
    /// ```
    /// use flecs_ecs::prelude::*;
    ///
    /// let world = World::new();
    ///
    /// let e = world.entity_named("scope");
    ///
    /// // previous scope can be used to set the scope back to the original.
    /// let previous_scope = world.set_scope(e);
    ///
    /// let s = world.get_scope();
    ///
    /// assert_eq!(s.unwrap(), e);
    /// ```
    ///
    /// # See also
    ///
    /// * [`World::get_scope()`]
    /// * [`World::set_scope()`]
    #[inline(always)]
    pub fn set_scope(&self, id: impl IntoId) -> EntityView<'_> {
        EntityView::new_from(self, unsafe {
            sys::ecs_set_scope(self.raw_world.as_ptr(), *id.into_id(self))
        })
    }

    /// Sets the search path for entity lookup operations.
    ///
    /// This function configures the search path used for looking up an entity.
    ///
    /// # Best Practices
    ///
    /// * It's advisable to restore the previous search path after making temporary changes.
    ///
    /// # Default Behavior
    ///
    /// * The default search path includes `flecs.core`.
    ///
    /// # Overwriting
    ///
    /// * Providing a custom search path will overwrite the existing search path.
    ///
    /// # Considerations
    ///
    /// * If the custom search path doesn't include `flecs.core`, operations that rely on looking up names from `flecs.core` may fail.
    ///
    /// # Arguments
    ///
    /// * `search_path` - The entity to set as the search path.
    ///
    /// # Returns
    ///
    /// Returns the current search path after the operation.
    ///
    /// # See also
    ///
    /// * [`World::lookup()`]
    /// * [`World::lookup_recursive()`]
    /// * [`World::try_lookup()`]
    /// * [`World::try_lookup_recursive()`]
    /// * C API: `sys::ecs_set_lookup_path`
    // TODO we need to improve this function somehow, it's not very ergonomic
    pub fn set_lookup_path(&self, search_path: impl IntoEntity) -> *mut sys::ecs_entity_t {
        unsafe {
            sys::ecs_set_lookup_path(self.raw_world.as_ptr(), &*search_path.into_entity(self))
        }
    }

    /// Lookup an entity by name.
    /// The entity is searched recursively recursively traversing
    /// up the tree until found.
    ///
    /// # Panics
    ///
    /// Ensure that the entity exists before using it.
    /// Use the [`World::try_lookup_recursive()`] variant otherwise.
    ///
    /// # Arguments
    ///
    /// * `name` - The name of the entity to lookup.
    ///
    /// # Returns
    ///
    /// The entity
    ///
    /// # Example
    ///
    /// ```
    /// use flecs_ecs::prelude::*;
    ///
    /// #[derive(Component)]
    /// struct Position {
    ///     x: i32,
    ///     y: i32,
    /// }
    ///
    /// let world = World::new();
    /// let a = world.entity().set(Position { x: 10, y: 20 }).with(|| {
    ///     world.entity_named("X");
    /// });
    ///
    /// let x = world.lookup_recursive("X");
    /// assert!(x.has(a));
    /// ```
    ///
    /// # See also
    ///
    /// * [`World::lookup()`]
    /// * [`World::set_lookup_path()`]
    /// * [`World::try_lookup()`]
    /// * [`World::try_lookup_recursive()`]
    #[inline(always)]
    pub fn lookup_recursive(&self, name: &str) -> EntityView<'_> {
        self.try_lookup_recursive(name)
            .unwrap_or_else(|| EntityView::new_from(self, Entity(0)))
    }

    /// Lookup an entity by name.
    /// The entity is searched recursively, traversing up the tree from the
    /// current scope until found. This matches the C++ `world::lookup`, which
    /// searches recursively by default. Use [`World::try_lookup()`] to search
    /// only the current scope.
    ///
    /// Matches C++ semantics: returns entity with id 0 if not found.
    ///
    /// # Arguments
    ///
    /// * `name` - The name of the entity to lookup.
    ///
    /// # Returns
    ///
    /// The entity, or entity with id 0 if not found.
    ///
    /// # See also
    ///
    /// * [`World::lookup_recursive()`]
    /// * [`World::set_lookup_path()`]
    /// * [`World::try_lookup()`]
    /// * [`World::try_lookup_recursive()`]
    #[inline(always)]
    pub fn lookup(&self, name: &str) -> EntityView<'_> {
        self.try_lookup_recursive(name)
            .unwrap_or_else(|| EntityView::new_from(self, Entity(0)))
    }

    /// Helper function for [`World::try_lookup()`] and [`World::try_lookup_recursive()`].
    fn try_lookup_impl(&self, name: &str, recursively: bool) -> Option<EntityView<'_>> {
        let name = compact_str::format_compact!("{}\0", name);

        let entity_id = unsafe {
            sys::ecs_lookup_path_w_sep(
                self.raw_world.as_ptr(),
                0,
                name.as_ptr() as *const _,
                SEPARATOR.as_ptr(),
                SEPARATOR.as_ptr(),
                recursively,
            )
        };
        if entity_id == 0 {
            None
        } else {
            Some(EntityView::new_from(self, entity_id))
        }
    }

    /// Lookup an entity by name.
    /// The entity is searched recursively recursively traversing
    /// up the tree until found.
    ///
    /// # Arguments
    ///
    /// * `name` - The name of the entity to lookup.
    ///
    /// # Returns
    ///
    /// The entity if found, otherwise `None`.
    ///
    /// # See also
    ///
    /// * [`World::lookup()`]
    /// * [`World::lookup_recursive()`]
    /// * [`World::set_lookup_path()`]
    /// * [`World::try_lookup()`]
    #[inline(always)]
    pub fn try_lookup_recursive(&self, name: &str) -> Option<EntityView<'_>> {
        self.try_lookup_impl(name, true)
    }

    /// Lookup entity by name, only the current scope is searched
    ///
    /// # Arguments
    ///
    /// * `name` - The name of the entity to lookup.
    ///
    /// # Returns
    ///
    /// The entity if found, otherwise `None`.
    ///
    /// # See also
    ///
    /// * [`World::lookup()`]
    /// * [`World::lookup_recursive()`]
    /// * [`World::set_lookup_path()`]
    /// * [`World::try_lookup_recursive()`]
    #[inline(always)]
    pub fn try_lookup(&self, name: &str) -> Option<EntityView<'_>> {
        self.try_lookup_impl(name, false)
    }

    /// Sets a singleton component of type `T` on the world.
    ///
    /// # Arguments
    ///
    /// * `component` - The singleton component to set on the world.
    pub fn set<T: ComponentId + DataComponent + ComponentType<Struct>>(&self, component: T) {
        let id = T::entity_id(self);
        set_helper(self.raw_world.as_ptr(), id, component, id);
    }

    /// Set a singleton pair using the first element type and a second id.
    ///
    /// # Panics
    ///
    /// Panics if the data type of the `(First, second)` pair id is not `First`.
    pub fn set_first<First>(&self, second: impl IntoEntity, first: First)
    where
        First: ComponentId + ComponentType<Struct> + DataComponent,
    {
        let entity = EntityView::new_from(self, First::entity_id(self));
        entity.set_first::<First>(first, second);
    }

    /// Set a singleton pair using the second element type and a first id.
    ///
    /// # Panics
    ///
    /// Panics if the data type of the `(first, Second)` pair id is not `Second`.
    pub fn set_second<Second>(&self, first: impl IntoEntity, second: Second)
    where
        Second: ComponentId + ComponentType<Struct> + DataComponent,
    {
        let entity = EntityView::new_from(self, Second::entity_id(self));
        entity.set_second::<Second>(first, second);
    }

    /// Set singleton pair.
    /// This operation sets the pair value, and uses the first non tag / ZST as type. If the
    /// entity did not yet have the pair, it will be added, otherwise overridden.
    pub fn set_pair<First, Second>(&self, data: <(First, Second) as ComponentOrPairId>::CastType)
    where
        First: ComponentId,
        Second: ComponentId,
        (First, Second): ComponentOrPairId,
    {
        const {
            assert!(
                !<(First, Second) as ComponentOrPairId>::IS_TAGS,
                "setting tag relationships is not possible with `set_pair`. use `add_pair` instead."
            );
        };

        let entity = EntityView::new_from(
            self,
            <<(First, Second) as ComponentOrPairId>::First as ComponentId>::entity_id(self),
        );
        entity.set_pair::<First, Second>(data);
    }

    /// assign a component for an entity.
    /// This operation sets the component value. If the entity did not yet have
    /// the component the operation will panic.
    pub fn assign<T: ComponentId + DataComponent>(&self, value: T) {
        let id = T::entity_id(self);
        assign_helper(self.ptr_mut(), id, value, id);
    }

    /// assign a component for an entity.
    /// This operation sets the component value. If the entity did not yet have
    /// the component the operation will panic.
    pub fn assign_id<T: ComponentId + DataComponent>(&self, value: T, id: impl IntoId) {
        let id = *id.into_id(self);
        assign_helper(self.ptr_mut(), id, value, id);
    }

    /// assign a component for an entity.
    /// This operation sets the component value. If the entity did not yet have
    /// the component the operation will panic.
    pub fn assign_pair<First, Second>(
        &self,
        value: <(First, Second) as ComponentOrPairId>::CastType,
    ) where
        First: ComponentId,
        Second: ComponentId,
        (First, Second): ComponentOrPairId,
    {
        let entity = EntityView::new_from(
            self,
            <<(First, Second) as ComponentOrPairId>::CastType as ComponentId>::entity_id(self),
        );

        entity.assign_pair::<First, Second>(value);
    }

    /// assign a component for an entity.
    /// This operation sets the component value. If the entity did not yet have
    /// the component the operation will panic.
    pub fn assign_first<First>(&self, first: First, second: impl IntoEntity)
    where
        First: ComponentId + DataComponent,
    {
        let entity = EntityView::new_from(self, First::entity_id(self));
        entity.assign_first::<First>(first, second);
    }

    /// assign a component for an entity.
    /// This operation sets the component value. If the entity did not yet have
    /// the component the operation will panic.
    pub fn assign_second<Second>(&self, first: impl IntoEntity, second: Second)
    where
        Second: ComponentId + DataComponent,
    {
        let entity = EntityView::new_from(self, Second::entity_id(self));
        entity.assign_second::<Second>(first, second);
    }

    /// signal that singleton component was modified.
    ///
    /// # Arguments
    ///
    /// * `id` - The id of the component that was modified.
    ///
    /// # See also
    ///
    /// * [`EntityView::modified()`]
    /// * [`World::modified()`]
    #[inline(always)]
    pub fn modified(&self, id: impl Into<Entity>) {
        let id = id.into();
        EntityView::new_from(self, id).modified(id);
    }

    /// set the version of the provided entity.
    pub fn set_version(&self, entity: impl Into<Entity>) {
        unsafe { sys::ecs_set_version(self.raw_world.as_ptr(), *entity.into()) };
    }

    /// returns true if the world is currently multithreaded, such as when a system that is multithreaded is running.
    #[inline(always)]
    pub fn is_currently_multithreaded(&self) -> bool {
        unsafe {
            sys::ecs_world_get_flags(self.raw_world.as_ptr()) & sys::EcsWorldMultiThreaded != 0
        }
    }

    /// Return component id if it has been registered.
    ///
    /// This is similar to `component_id::<T>()` but will never register the
    /// component with the world. If `T` is not registered in this world, returns `None`.
    #[inline(always)]
    pub fn get_component_id<T: ComponentId>(&self) -> Option<Entity> {
        if <T as ComponentId>::is_registered_with_world(self) {
            Some(Entity(T::entity_id(self)))
        } else {
            None
        }
    }

    /// Return raw type info for an id (component, tag, or pair).
    ///
    /// Returns `None` when no type info is available for the provided id.
    #[inline(always)]
    pub fn type_info_from(&self, id: impl IntoId) -> Option<*const sys::ecs_type_info_t> {
        let ptr = unsafe { sys::ecs_get_type_info(self.raw_world.as_ptr(), *id.into_entity(self)) };
        if ptr.is_null() { None } else { Some(ptr) }
    }

    /// Iterate entities in root of world
    ///
    /// # Arguments
    ///
    /// * `func` - The function invoked for each child. Must match the signature `FnMut(EntityView)`.
    #[inline(always)]
    pub fn each_child(&self, callback: impl FnMut(EntityView)) {
        EntityView::new(self).each_child(callback);
    }

    /// create alias for component
    ///
    /// # Type Parameters
    ///
    /// * `T` - The component type to create an alias for.
    ///
    /// # Arguments
    ///
    /// * `alias` - The alias to create.
    ///
    /// # Returns
    ///
    /// The entity representing the component.
    #[inline(always)]
    pub fn set_alias_component<T: ComponentId>(&self, alias: &str) -> EntityView<'_> {
        // Shared-register write hook (spec §3.6, §7.1): defer behind a live guard.
        ensure_write_episode(&self.world());
        let alias = compact_str::format_compact!("{}\0", alias);

        let id = T::entity_id(self);
        if alias.is_empty() {
            unsafe {
                sys::ecs_set_alias(
                    self.raw_world.as_ptr(),
                    id,
                    sys::ecs_get_name(self.raw_world.as_ptr(), id),
                );
            };
        } else {
            unsafe { sys::ecs_set_alias(self.raw_world.as_ptr(), id, alias.as_ptr() as *const _) };
        }
        EntityView::new_from(self, id)
    }

    /// create alias for entity by name
    ///
    /// # Arguments
    ///
    /// * `name` - The name of the entity to create an alias for.
    /// * `alias` - The alias to create.
    ///
    /// # Returns
    ///
    /// The entity found by name.
    #[inline(always)]
    pub fn set_alias_entity_by_name(&self, name: &str, alias: &str) -> EntityView<'_> {
        // Shared-register write hook (spec §3.6, §7.1): defer behind a live guard.
        ensure_write_episode(&self.world());
        let name = compact_str::format_compact!("{}\0", name);
        let alias = compact_str::format_compact!("{}\0", alias);

        let id = unsafe {
            sys::ecs_lookup_path_w_sep(
                self.raw_world.as_ptr(),
                0,
                name.as_ptr() as *const _,
                SEPARATOR.as_ptr(),
                SEPARATOR.as_ptr(),
                true,
            )
        };
        ecs_assert!(id != 0, FlecsErrorCode::InvalidParameter);
        unsafe { sys::ecs_set_alias(self.raw_world.as_ptr(), id, alias.as_ptr() as *const _) };
        EntityView::new_from(self, id)
    }

    /// create alias for entity
    ///
    /// # Arguments
    ///
    /// * `entity` - The entity to create an alias for.
    /// * `alias` - The alias to create.
    #[inline(always)]
    pub fn set_alias_entity(&self, entity: impl Into<Entity>, alias: &str) {
        // Shared-register write hook (spec §3.6, §7.1): defer behind a live guard.
        ensure_write_episode(&self.world());
        let entity = *entity.into();
        if alias.is_empty() {
            // Empty alias = use the entity's own short name (matches C++ ecs.use(entity) with no alias)
            unsafe {
                sys::ecs_set_alias(
                    self.raw_world.as_ptr(),
                    entity,
                    sys::ecs_get_name(self.raw_world.as_ptr(), entity),
                );
            };
        } else {
            let alias = compact_str::format_compact!("{}\0", alias);
            unsafe {
                sys::ecs_set_alias(self.raw_world.as_ptr(), entity, alias.as_ptr() as *const _);
            };
        }
    }

    /// Count entities with the provided id.
    ///
    /// # Arguments
    ///
    /// * `id` - The id to count.
    ///
    /// # Returns
    ///
    /// The number of entities with the provided id.
    pub fn count(&self, id: impl IntoId) -> i32 {
        unsafe { sys::ecs_count_id(self.raw_world.as_ptr(), *id.into_id(self)) }
    }

    /// Count entities with the provided enum constant.
    ///
    /// # Type Parameters
    ///
    /// * `T` - The enum type.
    ///
    /// # Arguments
    ///
    /// * `constant` - The enum constant to count.
    ///
    /// # Returns
    ///
    /// The number of entities with the provided enum constant.
    pub fn count_enum<T: ComponentId + ComponentType<Enum> + EnumComponentInfo>(
        &self,
        enum_value: T,
    ) -> i32 {
        unsafe { sys::ecs_count_id(self.raw_world.as_ptr(), *(enum_value.id_variant(self).id)) }
    }

    /// Count entities with the provided pair enum tag.
    ///
    /// # Type Parameters
    ///
    /// * `First` - The first element of the pair.
    /// * `Second` - The second element of the pair.
    ///
    /// # Arguments
    ///
    /// * `enum_value` - The enum value to count.
    ///
    /// # Returns
    ///
    /// The number of entities with the provided pair enum tag.
    pub fn count_enum_tag_pair<First, Second>(&self, enum_value: Second) -> i32
    where
        First: ComponentId,
        Second: ComponentId + ComponentType<Enum> + EnumComponentInfo,
    {
        unsafe {
            sys::ecs_count_id(
                self.raw_world.as_ptr(),
                ecs_pair(First::entity_id(self), *(enum_value.id_variant(self)).id),
            )
        }
    }

    /// All entities created in function are created in scope. All operations
    /// called in function (such as lookup) are relative to scope.
    ///
    /// # Arguments
    ///
    /// * `parent_id` - The id of the scope to use.
    /// * `func` - The function to run.
    pub fn run_in_scope_with(&self, parent_id: impl IntoEntity, mut func: impl FnMut()) {
        let world = self.world();
        let prev: sys::ecs_id_t =
            unsafe { sys::ecs_set_scope(self.raw_world.as_ptr(), *parent_id.into_entity(world)) };
        func();
        unsafe {
            sys::ecs_set_scope(self.raw_world.as_ptr(), prev);
        }
    }

    /// Use provided scope for operations ran on returned world.
    /// Operations need to be ran in a single statement
    ///
    /// # Arguments
    ///
    /// * `parent_id` - The id of the scope to use.
    ///
    /// # Returns
    ///
    /// A scoped world.
    pub fn scope(&self, parent_id: impl IntoId, mut f: impl FnMut(&World)) {
        let previous_scope = self.set_scope(parent_id);
        f(self);
        self.set_scope(previous_scope);
    }

    /// Use provided scope of name for operations ran on returned world.
    /// Operations need to be ran in a single statement
    ///
    /// # Arguments
    ///
    /// * `name` - The name of the scope to use.
    ///
    /// # Returns
    ///
    /// A scoped world.
    pub fn scope_name(&self, name: &str, f: impl FnMut(&World)) {
        self.scope(EntityView::new_named(self, name).id, f);
    }

    /// all entities created in function are created with id
    ///
    /// # Arguments
    ///
    /// * `id`: The id to create entities with.
    /// * `func`: The function to run.
    pub fn with(&self, id: impl IntoId, mut func: impl FnMut()) {
        // SAFETY: raw_world is a valid, live world pointer.
        let prev: sys::ecs_id_t =
            unsafe { sys::ecs_set_with(self.raw_world.as_ptr(), *id.into_id(self)) };
        func();
        // SAFETY: raw_world is still the same valid world pointer; prev is the value
        // ecs_set_with returned above, restoring the previous with-id.
        unsafe {
            sys::ecs_set_with(self.raw_world.as_ptr(), prev);
        }
    }

    /// Entities created in function are created with enum constant
    ///
    /// # Type Parameters
    ///
    /// * `T`: The enum type.
    ///
    /// # Arguments
    ///
    /// * `enum_value`: The enum value to give the entity.
    /// * `func`: The function to run.
    pub fn with_enum<T>(&self, enum_value: T, func: impl FnMut())
    where
        T: ComponentId + ComponentType<Enum> + EnumComponentInfo,
    {
        self.with(enum_value.id_variant(self), func);
    }

    /// Entities created in function are created with enum tag pair
    ///
    /// # Type Parameters
    ///
    /// * `First`: The first element of the pair.
    /// * `Second`: The enum component type.
    ///
    /// # Arguments
    ///
    /// * `enum_value`: The enum value to give the entity.
    /// * `func`: The function to run.
    pub fn with_enum_pair<First, Second>(&self, enum_value: Second, func: impl FnMut())
    where
        First: ComponentId,
        Second: ComponentId + ComponentType<Enum> + EnumComponentInfo,
    {
        self.with(
            ecs_pair(First::entity_id(self), **(enum_value.id_variant(self))),
            func,
        );
    }

    /// Delete all entities with the given id
    ///
    /// # Arguments
    ///
    /// * `id`: The id to delete.
    pub fn delete_entities_with(&self, id: impl IntoId) {
        // SAFETY: raw_world is a valid, live world pointer.
        unsafe {
            sys::ecs_delete_with(self.raw_world.as_ptr(), *id.into_id(self));
        }
    }

    /// Delete all entities with the given enum constant
    ///
    /// # Type Parameters
    ///
    /// * `T`: The enum type.
    ///
    /// # Arguments
    ///
    /// * `enum_value`: The enum value to query against for deletion.
    pub fn delete_with_enum<T: ComponentId + ComponentType<Enum> + EnumComponentInfo>(
        &self,
        enum_value: T,
    ) {
        self.delete_entities_with(enum_value.id_variant(self));
    }

    /// Delete all entities with the given enum tag pair / relationship
    ///
    /// # Type Parameters
    ///
    /// * `First`: The first element of the pair.
    /// * `Second`: The enum component type.
    ///
    /// # Arguments
    ///
    /// * `enum_value`: The enum value to query against for deletion.
    ///
    /// # See also
    ///
    /// * `world::delete_with`
    pub fn delete_with_enum_pair<First, Second>(&self, enum_value: Second)
    where
        First: ComponentId,
        Second: ComponentId + ComponentType<Enum> + EnumComponentInfo,
    {
        self.delete_entities_with(ecs_pair(
            First::entity_id(self),
            **enum_value.id_variant(self),
        ));
    }

    /// Remove all instances of the given id from entities
    ///
    /// # Arguments
    ///
    /// * `id`: The id to remove.
    pub fn remove_all(&self, id: impl IntoId) {
        // Shared-register write hook (spec §3.6, §7.1): defer behind a live guard.
        ensure_write_episode(&self.world());
        // SAFETY: raw_world is a valid, live world pointer.
        unsafe {
            sys::ecs_remove_all(self.raw_world.as_ptr(), *id.into_id(self));
        }
    }

    /// Remove all instances with the given enum constant from entities
    ///
    /// # Type Parameters
    ///
    /// * `T`: The enum type.
    ///
    /// # Arguments
    ///
    /// * `enum_value`: The enum value to query against for removal.
    pub fn remove_all_enum<T: ComponentId + ComponentType<Enum> + EnumComponentInfo>(
        &self,
        enum_value: T,
    ) {
        self.remove_all(enum_value.id_variant(self));
    }

    /// Remove all instances with the given enum tag pair / relationship from entities
    ///
    /// # Type Parameters
    ///
    /// * `First`: The first element of the pair.
    /// * `Second`: The enum component type.
    ///
    /// # Arguments
    ///
    /// * `enum_value`: The enum value to query against for removal.
    pub fn remove_all_enum_pair<First, Second>(&self, enum_value: Second)
    where
        First: ComponentId,
        Second: ComponentId + ComponentType<Enum> + EnumComponentInfo,
    {
        self.remove_all((First::entity_id(self), enum_value.id_variant(self)));
    }

    /// Checks if the given entity ID exists in the world.
    ///
    /// # Arguments
    ///
    /// * `entity` - The entity to check.
    ///
    /// # Returns
    ///
    /// True if the entity exists, false otherwise.
    pub fn exists(&self, entity: impl Into<Entity>) -> bool {
        // SAFETY: raw_world is a valid, live world pointer.
        unsafe { sys::ecs_exists(self.raw_world.as_ptr(), *entity.into()) }
    }

    /// Checks if the given entity ID is alive in the world.
    pub fn is_alive(&self, entity: impl Into<Entity>) -> bool {
        // SAFETY: raw_world is a valid, live world pointer.
        unsafe { sys::ecs_is_alive(self.raw_world.as_ptr(), *entity.into()) }
    }

    /// Checks if the given entity ID is valid.
    /// Invalid entities cannot be used with API functions.
    pub fn is_valid(&self, entity: impl Into<Entity>) -> bool {
        // SAFETY: raw_world is a valid, live world pointer.
        unsafe { sys::ecs_is_valid(self.raw_world.as_ptr(), *entity.into()) }
    }

    /// Get alive entity for id.
    ///
    /// # Arguments
    ///
    /// * `entity` - The entity to check
    ///
    /// # Returns
    ///
    /// The entity with the current generation. If the entity is not alive, this
    /// function will return an Entity of 0. Use `try_get_alive` if you want to
    /// return an `Option<EntityView>`.
    pub fn get_alive(&self, entity: impl Into<Entity>) -> EntityView<'_> {
        // SAFETY: raw_world is a valid, live world pointer.
        let entity = unsafe { sys::ecs_get_alive(self.raw_world.as_ptr(), *entity.into()) };

        EntityView::new_from(self, entity)
    }

    /// Get alive entity for id.
    ///
    /// # Arguments
    ///
    /// * `entity` - The entity to check
    ///
    /// # Returns
    ///
    /// The entity with the current generation.
    /// If the entity is not alive, this function will return `None`.
    pub fn try_get_alive(&self, entity: impl Into<Entity>) -> Option<EntityView<'_>> {
        // SAFETY: raw_world is a valid, live world pointer.
        let entity = unsafe { sys::ecs_get_alive(self.raw_world.as_ptr(), *entity.into()) };
        if entity == 0 {
            None
        } else {
            Some(EntityView::new_from(self, entity))
        }
    }

    /// Ensures that entity with provided generation is alive.
    /// This operation will fail if an entity exists with the same id and a
    /// different, non-zero generation.
    ///
    /// # Arguments
    ///
    /// * `entity` - The entity to ensure is alive.
    ///
    /// # Returns
    ///
    /// The entity with the provided generation.
    pub fn make_alive(&self, entity: impl Into<Entity>) -> EntityView<'_> {
        let entity = *entity.into();
        // SAFETY: raw_world is a valid, live world pointer.
        unsafe { sys::ecs_make_alive(self.raw_world.as_ptr(), entity) };
        EntityView::new_from(self, entity)
    }

    /// Run callback after completing frame
    ///
    /// # Arguments
    ///
    /// * `action` - The action to run.
    /// * `ctx` - The context to pass to the action.
    pub fn run_post_frame(&self, action: fn(WorldRef)) {
        let ctx = Box::into_raw(Box::new(action)) as *mut ::core::ffi::c_void;
        // SAFETY: raw_world is a valid, live world pointer; ctx is a Box<fn(WorldRef)>
        // that c_run_post_frame reclaims exactly once when flecs invokes it.
        unsafe {
            sys::ecs_run_post_frame(self.raw_world.as_ptr(), Some(c_run_post_frame), ctx);
        }
    }
}
