//! System module implementation
use core::ffi::c_void;

use crate::core::*;
use crate::sys;

pub struct SystemRunnerFluent<'a> {
    stage: WorldRef<'a>,
    id: sys::ecs_entity_t,
    stage_current: i32,
    stage_count: i32,
    offset: i32,
    limit: i32,
    delta_time: FTime,
    param: *mut c_void,
}

impl<'a> SystemRunnerFluent<'a> {
    /// Create a new system runner fluent interface
    pub fn new(
        world: impl WorldProvider<'a>,
        id: impl Into<Entity>,
        stage_current: i32,
        stage_count: i32,
        delta_time: FTime,
        param: *mut c_void,
    ) -> Self {
        Self {
            stage: world.world(),
            id: *id.into(),
            stage_current,
            stage_count,
            offset: 0,
            limit: 0,
            delta_time,
            param,
        }
    }

    /// Set the offset
    pub fn set_offset(&mut self, offset: i32) -> &mut Self {
        self.offset = offset;
        self
    }

    /// Set the limit
    pub fn set_limit(&mut self, limit: i32) -> &mut Self {
        self.limit = limit;
        self
    }

    /// Set the stage
    pub fn set_stage(&mut self, stage: impl WorldProvider<'a>) -> &mut Self {
        self.stage = stage.world();
        self
    }
}

impl Drop for SystemRunnerFluent<'_> {
    fn drop(&mut self) {
        if self.stage_count != 0 {
            unsafe {
                sys::ecs_run_worker(
                    self.stage.world_ptr_mut(),
                    self.id,
                    self.stage_current,
                    self.stage_count,
                    self.delta_time,
                    self.param,
                );
            }
        } else {
            unsafe {
                sys::ecs_run(
                    self.stage.world_ptr_mut(),
                    self.id,
                    self.delta_time,
                    self.param,
                );
            }
        }
        // Rethrow a panic the system callback stashed during this run (spec
        // §5.5). This runs on drop, so it must not resume a payload while a
        // different panic is already unwinding through it (that would abort);
        // when already unwinding, the stashed panic is left for the enclosing
        // entry point to surface.
        if !std::thread::panicking() {
            self.stage.rethrow_stashed_panic();
        }
    }
}
