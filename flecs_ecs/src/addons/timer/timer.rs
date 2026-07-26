//! When running a pipeline, systems are ran each time [`World::progress()`](crate::core::World::progress) is called.
//! The `flecs_timer` feature addon makes it possible to run systems at a specific time interval or rate.
use core::ops::{Deref, DerefMut};

use flecs_ecs_sys::{self as sys};

use crate::core::{ComponentId, Entity, EntityView, WorldProvider, WorldRef};

use super::super::system::System;

pub trait TimerAPI: Sized {
    fn world(&self) -> WorldRef<'_>;
    fn world_ptr(&self) -> *const sys::ecs_world_t;
    fn world_ptr_mut(&self) -> *mut sys::ecs_world_t;
    fn id(&self) -> Entity;

    /// Set timer interval.
    /// This operation will continuously invoke systems associated with the timer after the interval period expires.
    /// If the entity contains an existing timer, the interval value will be reset.
    ///
    /// The timer is synchronous, and is incremented each frame by `delta_time`.
    ///
    /// The `tick_source` entity will be a tick source after this operation.
    /// Tick sources can be read by getting the [`flecs::TickSource`](crate::core::flecs::system::TickSource) component.
    /// If the tick source ticked this frame, the 'tick' member will be true.
    /// When the tick source is a system, the system will tick when the timer ticks.
    fn set_interval(self, interval: f32) -> Self {
        // Shared-register write hook (spec §3.6, §7.1): adds EcsTimer to the timer
        // entity, so it must defer behind a live guard on that entity's data.
        crate::core::ensure_write_episode(&self.world());
        unsafe { sys::ecs_set_interval(self.world_ptr_mut(), *self.id(), interval) };
        self
    }

    /// Get current interval value for the specified timer.
    ///
    /// This operation returns the value set by [`set_interval()`](crate::addons::timer::TimerAPI::set_interval).
    ///
    /// # Returns
    ///
    /// The interval. If the entity is not a timer, the operation will return 0.
    fn interval(&self) -> f32 {
        unsafe { sys::ecs_get_interval(self.world_ptr(), *self.id()) }
    }

    /// Set timer timeout.
    /// This operation executes any systems associated with the timer after the specified timeout value.
    /// If the entity contains an existing timer, the timeout value will be reset.
    /// The timer can be started and stopped with [`start()`](crate::addons::timer::TimerAPI::start) and [`stop()`](crate::addons::timer::TimerAPI::stop).
    ///     
    /// The timer is synchronous, and is incremented each frame by `delta_time`.
    ///
    /// The `tick_source` entity will be a tick source after this operation.
    /// Tick sources can be read by getting the [`flecs::TickSource`](crate::core::flecs::system::TickSource) component.
    /// If the tick source ticked this frame, the 'tick' member will be true.
    /// When the tick source is a system, the system will tick when the timer ticks.
    fn set_timeout(self, timeout: f32) -> Self {
        // Shared-register write hook (spec §3.6, §7.1): adds EcsTimer to the timer
        // entity, so it must defer behind a live guard on that entity's data.
        crate::core::ensure_write_episode(&self.world());
        unsafe { sys::ecs_set_timeout(self.world_ptr_mut(), *self.id(), timeout) };
        self
    }

    /// Get current timeout value for the specified timer.
    /// This operation returns the value set by [`set_timeout()`](crate::addons::timer::TimerAPI::set_timeout).
    ///
    /// After the timeout expires the [`flecs::timer::Timer`](crate::core::flecs::timer::Timer) component is removed from the entity.
    /// This means that if [`TimerAPI::timeout`] is invoked after the timer is expired, the operation will return 0.
    ///
    /// The timer is synchronous, and is incremented each frame by `delta_time`.
    ///
    /// The `tick_source` entity will be a tick source after this operation.
    /// Tick sources can be read by getting the [`flecs::TickSource`](crate::core::flecs::system::TickSource) component.
    /// If the tick source ticked this frame, the 'tick' member will be true.
    /// When the tick source is a system, the system will tick when the timer ticks.
    ///
    /// # Returns
    ///
    /// The timeout. If no timer is active for this entity, the operation returns 0.
    fn timeout(&self) -> f32 {
        unsafe { sys::ecs_get_timeout(self.world_ptr(), *self.id()) }
    }

    /// Set rate filter. Will use the frame tick as tick source,
    /// which corresponds with the number of times [`World::progress()`](crate::core::World::progress) is called.
    /// This operation initializes a rate filter.
    /// Rate filters sample tick sources and tick at a configurable multiple.
    /// A rate filter is a tick source itself, which means that rate filters can be chained.
    ///
    /// Rate filters enable deterministic system execution which cannot be achieved with interval timers alone.
    /// For example, if timer A has interval 2.0 and timer B has interval 4.0,
    /// it is not guaranteed that B will tick at exactly twice the multiple of A.
    /// This is partly due to the indeterministic nature of timers, and partly due to floating point rounding errors.
    ///
    /// Rate filters can be combined with timers (or other rate filters)
    /// to ensure that a system ticks at an exact multiple of a tick source (which can be another system).
    /// If a rate filter is created with a rate of 1 it will tick at the exact same time as its source.
    ///
    /// The `tick_source` entity will be a tick source after this operation.
    /// Tick sources can be read by getting the [`flecs::TickSource`](crate::core::flecs::system::TickSource) component.
    /// If the tick source ticked this frame, the 'tick' member will be true.
    /// When the tick source is a system, the system will tick when the timer ticks.
    ///
    /// # See also
    ///
    /// * [`TimerAPI::set_rate_with_tick_source()`]
    fn set_rate(self, rate: i32) -> Self {
        // Shared-register write hook (spec §3.6, §7.1): adds EcsRateFilter to the
        // entity, so it must defer behind a live guard on that entity's data.
        crate::core::ensure_write_episode(&self.world());
        unsafe { sys::ecs_set_rate(self.world_ptr_mut(), *self.id(), rate, 0) };
        self
    }

    /// Set rate filter.
    /// This operation initializes a rate filter.
    /// Rate filters sample tick sources and tick at a configurable multiple.
    /// A rate filter is a tick source itself, which means that rate filters can be chained.
    ///
    /// Rate filters enable deterministic system execution which cannot be achieved with interval timers alone.
    /// For example, if timer A has interval 2.0 and timer B has interval 4.0,
    /// it is not guaranteed that B will tick at exactly twice the multiple of A.
    /// This is partly due to the indeterministic nature of timers, and partly due to floating point rounding errors.
    ///
    /// Rate filters can be combined with timers (or other rate filters)
    /// to ensure that a system ticks at an exact multiple of a tick source (which can be another system).
    /// If a rate filter is created with a rate of 1 it will tick at the exact same time as its source.
    ///
    /// If no tick source is provided (Entity(0)), the rate filter will use the frame tick as source,
    /// which corresponds with the number of times [`World::progress()`](crate::core::World::progress) is called.
    ///
    /// The `tick_source` entity will be a tick source after this operation.
    /// Tick sources can be read by getting the [`flecs::TickSource`](crate::core::flecs::system::TickSource) component.
    /// If the tick source ticked this frame, the 'tick' member will be true.
    /// When the tick source is a system, the system will tick when the timer ticks.
    /// # See also
    ///
    /// * [`TimerAPI::set_rate()`]
    fn set_rate_with_tick_source(self, rate: i32, tick_source: impl Into<Entity>) -> Self {
        // Shared-register write hook (spec §3.6, §7.1): adds EcsRateFilter to the
        // entity, so it must defer behind a live guard on that entity's data.
        crate::core::ensure_write_episode(&self.world());
        unsafe { sys::ecs_set_rate(self.world_ptr_mut(), *self.id(), rate, *tick_source.into()) };
        self
    }

    /// Start timer.
    /// This operation resets the timer and starts it with the specified timeout.
    fn start(&self) {
        unsafe { sys::ecs_start_timer(self.world_ptr_mut(), *self.id()) };
    }

    /// Stop timer.
    /// This operation stops a timer from triggering.
    fn stop(&self) {
        unsafe { sys::ecs_stop_timer(self.world_ptr_mut(), *self.id()) };
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Timer<'a> {
    entity: EntityView<'a>,
}

impl<'a> Deref for Timer<'a> {
    type Target = EntityView<'a>;

    #[inline(always)]
    fn deref(&self) -> &Self::Target {
        &self.entity
    }
}

impl DerefMut for Timer<'_> {
    #[inline(always)]
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.entity
    }
}

impl From<Timer<'_>> for Entity {
    #[inline]
    fn from(timer: Timer) -> Self {
        timer.id
    }
}

impl<'a> Timer<'a> {
    pub(crate) fn new(world: impl WorldProvider<'a>) -> Self {
        Timer {
            entity: EntityView::new(world),
        }
    }

    pub(crate) fn new_from<T: ComponentId>(world: impl WorldProvider<'a>) -> Self {
        Timer {
            entity: EntityView::new_from(world.world(), T::entity_id(world)),
        }
    }
}

impl TimerAPI for Timer<'_> {
    #[inline(always)]
    fn world(&self) -> WorldRef<'_> {
        self.entity.world
    }

    #[inline(always)]
    fn world_ptr(&self) -> *const flecs_ecs_sys::ecs_world_t {
        self.entity.world_ptr()
    }

    #[inline(always)]
    fn world_ptr_mut(&self) -> *mut flecs_ecs_sys::ecs_world_t {
        self.entity.world_ptr_mut()
    }

    #[inline(always)]
    fn id(&self) -> Entity {
        self.id
    }
}

impl TimerAPI for System<'_> {
    #[inline(always)]
    fn world(&self) -> WorldRef<'_> {
        self.entity.world
    }

    #[inline(always)]
    fn world_ptr(&self) -> *const flecs_ecs_sys::ecs_world_t {
        self.entity.world_ptr()
    }

    #[inline(always)]
    fn world_ptr_mut(&self) -> *mut flecs_ecs_sys::ecs_world_t {
        self.entity.world_ptr_mut()
    }

    #[inline(always)]
    fn id(&self) -> Entity {
        self.id
    }
}
