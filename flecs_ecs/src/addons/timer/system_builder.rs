use crate::{addons::system::SystemBuilder, prelude::*};

impl<T: QueryTuple> SystemBuilder<'_, T> {
    /// Set system interval.
    ///
    /// This operation will cause the system to be ran at the specified interval.
    ///
    /// The timer is synchronous, and is incremented each frame by `delta_time`.
    pub fn set_interval(mut self, interval: f32) -> Self {
        self.desc.interval = interval;
        self
    }

    /// Sets a rate filter on the system, causing it to run once every `rate`
    /// ticks. The tick source may be any entity, including another system.
    pub fn set_tick_source_rate(mut self, tick_source: impl Into<Entity>, rate: i32) -> Self {
        self.desc.rate = rate;
        self.desc.tick_source = *tick_source.into();
        self
    }

    /// Sets a rate filter on the system, causing it to run once every `rate`
    /// ticks. If a tick source was provided, this just updates the rate of the
    /// system.
    pub fn set_rate(mut self, rate: i32) -> Self {
        self.desc.rate = rate;
        self
    }

    /// Set tick source.
    /// This operation sets a shared tick source for the system.
    pub fn set_tick_source(mut self, tick_source: impl IntoEntity) -> Self {
        self.desc.tick_source = *tick_source.into_entity(self.world());
        self
    }
}
