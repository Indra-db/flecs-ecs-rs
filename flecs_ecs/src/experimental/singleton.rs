//! Direct world-level singleton access under the register split (spec §3.5).
//!
//! A component `T`'s singleton is stored on `T`'s own component entity (the
//! flecs convention), so singleton access is single-entity access on that
//! entity, split by register exactly like [`EntityView`](crate::core::EntityView)
//! access:
//!
//! * [`singleton`](WorldSingletonExt::singleton) is the **shared register**: it
//!   returns a [`Ref`] guard (pin + lock, zero defer FFI on the read path), so
//!   it composes with the episode model — a `set` issued while the guard is live
//!   is deferred and applied when the last guard drops.
//! * [`singleton_mut`](WorldSingletonExt::singleton_mut) is the **exclusive
//!   register**: the `&mut World` proves no outstanding borrow, so it hands back
//!   a plain `&mut T` with no lock.
//!
//! The `Singleton<T>` query wrapper and the trait-based singleton *term* (§4.9)
//! are out of scope for this surface.

use crate::core::{ComponentId, ComponentOrPairId, DataComponent, World};

use super::entity_access::EntityGuardExt;
use super::exclusive::WorldExclusiveExt;
use super::guard::Ref;

/// World-level singleton access split by register (spec §3.5).
pub trait WorldSingletonExt {
    /// Shared read of a singleton: a [`Ref`] guard (pin + lock, no defer FFI on
    /// the read path). `None` if the singleton component is not set.
    ///
    /// # Panics
    /// If a conflicting borrow of the singleton storage is live (e.g. a write
    /// guard on the same component), like [`RefCell::borrow`](core::cell::RefCell::borrow).
    fn singleton<T>(&self) -> Option<Ref<'_, <T as ComponentOrPairId>::CastType>>
    where
        T: ComponentId + ComponentOrPairId + DataComponent;

    /// Exclusive singleton access: a plain `&mut T`, no lock (the `&mut World`
    /// is the proof). `None` if the singleton component is not set.
    fn singleton_mut<T>(&mut self) -> Option<&mut <T as ComponentOrPairId>::CastType>
    where
        T: ComponentId + ComponentOrPairId + DataComponent;
}

impl WorldSingletonExt for World {
    #[inline]
    fn singleton<T>(&self) -> Option<Ref<'_, <T as ComponentOrPairId>::CastType>>
    where
        T: ComponentId + ComponentOrPairId + DataComponent,
    {
        // The singleton entity is T's own component entity.
        let component = T::entity_id(self);
        self.entity_from_id(component).get_ref::<&T>()
    }

    #[inline]
    fn singleton_mut<T>(&mut self) -> Option<&mut <T as ComponentOrPairId>::CastType>
    where
        T: ComponentId + ComponentOrPairId + DataComponent,
    {
        let component = T::entity_id(&*self);
        self.get_exclusive::<T>(component)
    }
}
