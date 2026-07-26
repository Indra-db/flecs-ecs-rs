//! The [`World`] type: the container for all entities, components and queries.
//!
//! Start with [`World::new()`] (or [`World::new_mini()`] for a world without any builtin
//! modules registered) to create a world, then use it to spawn entities, register components
//! and build queries.

use crate::core::*;
use crate::sys;

#[cfg(feature = "std")]
extern crate std;

extern crate alloc;

pub(crate) type FlecsArray = Vec<u64>;

mod component;
mod entity_view;
mod event;
mod id;
mod observer;
mod operations;
#[cfg(feature = "flecs_pipeline")]
mod pipeline;
mod query;
mod singleton;
#[cfg(feature = "flecs_system")]
mod system;
mod world;

pub use world::*;

/// An entity id range created with [`World::entity_range_new()`].
///
/// Constrains new entity identifiers to a `[min, max]` interval when activated
/// with [`World::entity_range_set()`]. Ranges are owned by the world and live
/// for as long as the world does; they cannot be deleted once created.
#[derive(Clone, Copy)]
pub struct EntityRange<'w> {
    pub(crate) ptr: core::ptr::NonNull<sys::ecs_entity_range_t>,
    pub(crate) _marker: core::marker::PhantomData<&'w World>,
}

impl EntityRange<'_> {
    /// First id in the range (inclusive).
    pub fn min(&self) -> u32 {
        unsafe { self.ptr.as_ref().min }
    }

    /// Last id in the range (inclusive, 0 = unlimited).
    pub fn max(&self) -> u32 {
        unsafe { self.ptr.as_ref().max }
    }

    /// Last issued id in the range.
    pub fn current(&self) -> u32 {
        unsafe { self.ptr.as_ref().cur }
    }
}

impl core::fmt::Debug for EntityRange<'_> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("EntityRange")
            .field("min", &self.min())
            .field("max", &self.max())
            .field("current", &self.current())
            .finish()
    }
}
