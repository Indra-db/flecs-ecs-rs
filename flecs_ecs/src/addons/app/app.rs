//! addon for running the main application loop.

use crate::core::*;
use crate::sys;
use core::ffi::{c_int, c_void};

type FrameAction = Box<dyn FnMut(WorldRef, &sys::ecs_app_desc_t) -> i32>;
type RunAction = Box<dyn FnMut(WorldRef, &mut sys::ecs_app_desc_t) -> i32>;

/// Per-app action storage. A pointer to this lives in `desc.ctx` (which the
/// flecs app addon reserves for custom run/frame actions), so the process
/// global trampolines below can dispatch to the actions of the app that is
/// actually running — concurrent apps in one process don't interfere.
#[derive(Default)]
struct AppActions {
    frame: Option<FrameAction>,
    run: Option<RunAction>,
}

// XAI: the C app addon copies the descriptor into a process-global
// `static ecs_app_desc_t ecs_app_desc` inside ecs_app_run, so concurrent
// ecs_app_run calls clobber each other's frames/ctx. Serialize all app runs
// in this process.
static APP_RUN_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

// XAI: `ecs_app_run_frame` dispatches to the *installed* frame action
// (flecs.c `return frame_action(world, desc)`), so the fallback paths below
// must replicate the C defaults instead of calling `ecs_app_run_frame`,
// which would recurse into this trampoline.

extern "C-unwind" fn frame_action_trampoline(
    world: *mut sys::ecs_world_t,
    desc: *const sys::ecs_app_desc_t,
) -> c_int {
    // SAFETY: flecs passes a valid world and desc pointer to the frame action.
    let actions = unsafe { ((*desc).ctx as *mut AppActions).as_mut() };
    let Some(action) = actions.and_then(|actions| actions.frame.as_mut()) else {
        // This app has no Rust frame action (e.g. another app run in the same
        // process): replicate flecs_default_frame_action.
        return unsafe { !sys::ecs_progress(world, (*desc).delta_time) as c_int };
    };

    // SAFETY: see above.
    let world_ref = unsafe { WorldRef::from_ptr(world) };
    let desc_ref = unsafe { &*desc };

    let result = std::panic::catch_unwind(core::panic::AssertUnwindSafe(|| {
        action(world_ref, desc_ref)
    }));

    // A panic must not unwind into the C run loop; quit with an error.
    result.unwrap_or(-1)
}

extern "C-unwind" fn run_action_trampoline(
    world: *mut sys::ecs_world_t,
    desc: *mut sys::ecs_app_desc_t,
) -> c_int {
    // SAFETY: flecs passes a valid world and desc pointer to the run action.
    let actions = unsafe { ((*desc).ctx as *mut AppActions).as_mut() };
    let Some(action) = actions.and_then(|actions| actions.run.as_mut()) else {
        // This app has no Rust run action: replicate flecs_default_run_action.
        return unsafe { default_run_action(world, desc) };
    };

    // SAFETY: see above.
    let world_ref = unsafe { WorldRef::from_ptr(world) };
    let desc_ref = unsafe { &mut *desc };

    let result = std::panic::catch_unwind(core::panic::AssertUnwindSafe(|| {
        action(world_ref, desc_ref)
    }));

    result.unwrap_or(-1)
}

/// Replica of the (non-exported) C `flecs_default_run_action`, used when the
/// run trampoline is installed but the running app has no Rust run action.
///
/// # Safety
///
/// `world` and `desc` must be the valid pointers flecs passed to the run
/// action.
unsafe fn default_run_action(
    world: *mut sys::ecs_world_t,
    desc: *mut sys::ecs_app_desc_t,
) -> c_int {
    unsafe {
        if let Some(init) = (*desc).init {
            init(world);
        }

        let mut result = 0;
        let frames = (*desc).frames;
        if frames != 0 {
            for _ in 0..frames {
                result = sys::ecs_app_run_frame(world, desc);
                if result != 0 {
                    break;
                }
            }
        } else {
            loop {
                result = sys::ecs_app_run_frame(world, desc);
                if result != 0 {
                    break;
                }
            }
        }

        sys::ecs_quit(world);

        if result == 1 { 0 } else { result }
    }
}

/// Application interface.
///
/// These are typically constructed via [`World::app()`]
///
/// The app borrows the [`World`] it was created from for the duration of its
/// lifetime; it does not hold an owning world handle. The world is finalized
/// when the borrowed `World` is dropped, whether or not [`App::run()`] was
/// called.
pub struct App<'a> {
    pub(crate) world: Option<WorldRef<'a>>,
    pub(crate) desc: sys::ecs_app_desc_t,
    actions: Option<Box<AppActions>>,
}

impl<'a> App<'a> {
    /// Create a new application.
    ///
    /// # Arguments
    ///
    /// * `world` - The world to run the application on.
    ///
    /// # See also
    ///
    /// * [`World::app()`]
    pub(crate) fn new(world: WorldRef<'a>) -> Self {
        let mut obj = Self {
            world: Some(world),
            desc: sys::ecs_app_desc_t::default(),
            actions: None,
        };

        let stats = unsafe { sys::ecs_get_world_info(obj.world_ptr()) };
        obj.desc.target_fps = unsafe { (*stats).target_fps };
        let zero: FTime = 0.0;
        if obj.desc.target_fps.to_bits() == zero.to_bits() {
            obj.desc.target_fps = 60.0;
        }
        obj
    }

    pub(crate) fn world_ptr(&self) -> *mut sys::ecs_world_t {
        self.world
            .as_ref()
            .expect("App::run consumed the world; the App cannot be reused")
            .world_ptr_mut()
    }

    fn actions_mut(&mut self) -> &mut AppActions {
        assert!(
            self.desc.ctx.is_null() || self.actions.is_some(),
            "App::context() cannot be combined with App::frame_action()/App::run_action(); \
             the ctx field is used to store the Rust actions"
        );
        let actions = self.actions.get_or_insert_with(Box::default);
        self.desc.ctx = (&mut **actions) as *mut AppActions as *mut c_void;
        actions
    }

    /// Set the target frames per second.
    ///
    /// # Arguments
    ///
    /// * `fps` - The target frames per second.
    pub fn set_target_fps(&mut self, fps: FTime) -> &mut Self {
        self.desc.target_fps = fps;
        self
    }

    /// Set the time delta.
    ///
    /// # Arguments
    ///
    /// * `delta_time` - The time delta.
    pub fn set_delta_time(&mut self, delta_time: FTime) -> &mut Self {
        self.desc.delta_time = delta_time;
        self
    }

    /// Set the number of threads.
    ///
    /// # Arguments
    ///
    /// * `threads` - The number of threads.
    pub fn set_threads(&mut self, threads: i32) -> &mut Self {
        self.desc.threads = threads;
        self
    }

    /// Set the number of frames.
    ///
    /// # Arguments
    ///
    /// * `frames` - The number of frames.
    pub fn set_frames(&mut self, frames: i32) -> &mut Self {
        self.desc.frames = frames;
        self
    }

    /// Enable the REST API.
    ///
    /// # Arguments
    ///
    /// * `port` - The port to listen on.
    #[cfg(feature = "flecs_rest")]
    pub fn enable_rest(&mut self, port: u16) -> &mut Self {
        self.desc.enable_rest = true;
        self.desc.port = port;
        self
    }

    /// Enable the stats.
    ///
    /// # Arguments
    ///
    /// * `enable` - Whether to enable the stats.
    ///
    /// # See also
    ///
    /// * [`addons::stats`](crate::addons::stats)
    #[cfg(feature = "flecs_stats")]
    pub fn enable_stats(&mut self, enable: bool) -> &mut Self {
        self.desc.enable_stats = enable;
        self
    }

    // TODO change this to FnMut(&mut World) -> cint
    /// Set the application init action.
    ///
    /// # Arguments
    ///
    /// * `value` - The init action.
    pub fn init(&mut self, value: sys::ecs_app_init_action_t) -> &mut Self {
        self.desc.init = value;
        self
    }

    /// Set the application context.
    ///
    /// The flecs app addon reserves this field for custom run/frame actions.
    /// It cannot be combined with [`App::frame_action()`]/[`App::run_action()`],
    /// which store their closures in it.
    ///
    /// # Arguments
    ///
    /// * `ctx` - The context.
    ///
    /// # Panics
    ///
    /// Panics if a Rust frame or run action was already set on this app.
    pub fn context(&mut self, ctx: *mut c_void) -> &mut Self {
        assert!(
            self.actions.is_none(),
            "App::context() cannot be combined with App::frame_action()/App::run_action(); \
             the ctx field is used to store the Rust actions"
        );
        self.desc.ctx = ctx;
        self
    }

    /// Set a custom frame action, invoked once per frame by the app run loop.
    ///
    /// The action replaces the default frame behavior (which calls
    /// [`World::progress()`]), so it should call `world.progress()` itself.
    /// Return `0` to continue running, non-zero to stop the app.
    ///
    /// The action is stored per app (in the descriptor's `ctx` field, which
    /// the flecs app addon reserves for this purpose) and runs on the thread
    /// that calls [`App::run()`], so it may capture `!Send` data such as
    /// rendering handles. A panic in the action stops the app with exit code
    /// `-1` instead of unwinding into the C run loop.
    ///
    /// # Panics
    ///
    /// Panics if a different frame action was already installed through the C
    /// API directly, or if [`App::context()`] was set on this app.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use flecs_ecs::prelude::*;
    ///
    /// let world = World::new();
    /// world
    ///     .app()
    ///     .set_frames(3)
    ///     .frame_action(|world, _desc| {
    ///         // A custom frame action replaces the app's default, so it drives
    ///         // the world itself. `frame_action` hands a `WorldRef` (a shared
    ///         // handle), so the exclusive `World::progress` is reached through
    ///         // its raw C entry point here.
    ///         if unsafe { flecs_ecs::sys::ecs_progress(world.ptr_mut(), 0.0) } { 0 } else { 1 }
    ///     })
    ///     .run();
    /// ```
    pub fn frame_action(
        &mut self,
        action: impl FnMut(WorldRef, &sys::ecs_app_desc_t) -> i32 + 'static,
    ) -> &mut Self {
        self.actions_mut().frame = Some(Box::new(action));
        let result = unsafe { sys::ecs_app_set_frame_action(Some(frame_action_trampoline)) };
        assert!(
            result == 0,
            "a different frame action is already installed through the flecs C API"
        );
        self
    }

    /// Set a custom run action, replacing the default app run loop.
    ///
    /// The action receives the world and the app descriptor and is expected
    /// to drive the main loop itself (e.g. by calling
    /// [`sys::ecs_app_run_frame`] or [`World::progress()`] until done).
    /// Its return value becomes the return value of [`App::run()`].
    ///
    /// The action is stored per app (in the descriptor's `ctx` field, which
    /// the flecs app addon reserves for this purpose) and runs on the thread
    /// that calls [`App::run()`], so it may capture `!Send` data. A panic in
    /// the action stops the app with exit code `-1` instead of unwinding into
    /// the C run loop.
    ///
    /// # Panics
    ///
    /// Panics if a different run action was already installed through the C
    /// API directly, or if [`App::context()`] was set on this app.
    pub fn run_action(
        &mut self,
        action: impl FnMut(WorldRef, &mut sys::ecs_app_desc_t) -> i32 + 'static,
    ) -> &mut Self {
        self.actions_mut().run = Some(Box::new(action));
        let result = unsafe { sys::ecs_app_set_run_action(Some(run_action_trampoline)) };
        assert!(
            result == 0,
            "a different run action is already installed through the flecs C API"
        );
        self
    }

    /// Run application. This will run the application with the parameters specified in desc.
    /// After the application quits ([`World::quit()`] is called) this will return.
    /// If a custom run action is set, it will be invoked by this operation.
    /// The default run action calls the frame action in a loop until it returns a non-zero value.
    ///
    /// The app only borrows the world, so running the app leaves the caller's
    /// [`World`] handle intact; the world is finalized when that handle is
    /// dropped. If the app is used in an environment that takes over the main
    /// loop (like emscripten), the quit flag is not set and the app's actions
    /// are intentionally leaked, since the C-side descriptor copy keeps
    /// referencing them after this call returns.
    ///
    /// Concurrent app runs in one process are serialized: the underlying C
    /// addon stores the descriptor in a process-global, so only one app can
    /// run at a time.
    ///
    /// # Panics
    ///
    /// Panics when called more than once on the same `App`.
    ///
    /// # Returns
    ///
    /// The exit code of the application.
    pub fn run(&mut self) -> i32 {
        let world = self
            .world
            .take()
            .expect("App::run can only be called once per App");
        let world_ptr = world.world_ptr_mut();
        let _run_guard = APP_RUN_LOCK
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let result = unsafe { sys::ecs_app_run(world_ptr, &mut self.desc) };
        if !unsafe { sys::ecs_should_quit(world_ptr) } {
            // The environment took over the main loop (e.g. emscripten): the
            // actions are still referenced by the C-side descriptor copy and
            // must stay alive for the duration of the takeover. The world is
            // owned by the caller, which keeps it alive.
            core::mem::forget(self.actions.take());
        }
        result
    }
}
