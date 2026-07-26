use crate::prelude::*;
use crate::sys;

impl System<'_> {
    /// Assign tick source to system based on an id.
    /// Systems can be their own tick source, which can be any of the tick sources (one shot timers, interval times and rate filters).
    /// However, in some cases it is must be guaranteed that different systems tick on the exact same frame.
    ///
    /// This cannot be guaranteed by giving two systems the same interval/rate filter as it is possible
    /// that one system is (for example) disabled, which would cause the systems to go out of sync.
    /// To provide these guarantees, systems must use the same tick source, which is what this operation enables.
    ///
    /// When two systems share the same tick source, it is guaranteed that they tick in the same frame.
    /// The provided tick source can be any entity that is a tick source, including another system.
    /// If the provided entity is not a tick source the system will not be ran.
    ///
    /// To disassociate a tick source from a system, use [`System::reset_tick_source()`](crate::addons::system::System::reset_tick_source).
    pub fn set_tick_source(&self, id: impl IntoEntity) {
        // Shared-register write hook (spec §3.6, §7.1): adds EcsTickSource to the
        // system entity, so it must defer behind a live guard on its data.
        crate::core::ensure_write_episode(&self.world);
        unsafe {
            sys::ecs_set_tick_source(
                self.entity.world_ptr_mut(),
                *self.id,
                *id.into_entity(self.world),
            );
        }
    }

    /// Reset, disassociate a tick source from a system
    pub fn reset_tick_source(&self) {
        // Shared-register write hook (spec §3.6, §7.1): mutates EcsTickSource on the
        // system entity, so it must defer behind a live guard on its data.
        crate::core::ensure_write_episode(&self.world);
        unsafe { sys::ecs_set_tick_source(self.entity.world_ptr_mut(), *self.id, 0) }
    }
}
