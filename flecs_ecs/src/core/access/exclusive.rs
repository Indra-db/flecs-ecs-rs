//! Exclusive register: single-component access proven unique by `&mut World`.
//!
//! When the caller holds `&mut World`, the borrow checker already excludes every
//! other access to the world for the duration of the returned borrow, so no
//! runtime lock is taken and a plain `&mut T` can be handed back.
//!
//! ```compile_fail
//! use flecs_ecs::prelude::*;
//!
//! #[derive(Component)]
//! struct Position { x: i32 }
//!
//! let mut world = World::new();
//! let e = world.entity().set(Position { x: 0 });
//! let p = world.get_mut::<Position>(e).unwrap();
//! // Holding `p` (which borrows `&mut world`) while touching the world again
//! // must not compile:
//! let _q = world.get_mut::<Position>(e);
//! p.x += 1;
//! ```

use crate::core::{
    ComponentOrPairId, DataComponent, Entity, GetComponentPointers, GetTuple, World, WorldRef,
};
use crate::sys;

/// Exclusive-register component access on [`World`].
pub trait WorldExclusiveExt {
    /// Borrow a component mutably with zero lock traffic. The returned
    /// reference borrows `&mut self`, so the borrow checker forbids every other
    /// access to this world while it is live — which is exactly what makes
    /// skipping the runtime lock sound.
    ///
    /// Returns `None` if the entity is not alive or does not have the component.
    fn get_mut<T>(
        &mut self,
        entity: impl Into<Entity>,
    ) -> Option<&mut <T as ComponentOrPairId>::CastType>
    where
        T: ComponentOrPairId + DataComponent;
}

impl WorldExclusiveExt for World {
    fn get_mut<T>(
        &mut self,
        entity: impl Into<Entity>,
    ) -> Option<&mut <T as ComponentOrPairId>::CastType>
    where
        T: ComponentOrPairId + DataComponent,
    {
        let entity: Entity = entity.into();
        let world_ptr = self.raw_world.as_ptr();
        // SAFETY: world_ptr is this world's live pointer.
        let world = unsafe { WorldRef::from_ptr(world_ptr) };

        // No defer scope and no lock: the &mut World borrow already excludes any
        // aliasing access for the whole lifetime of the returned reference.
        // ecs_record_find asserts on dead entities in debug builds, so gate on
        // liveness first (mirrors ecs_rust_get_scope_begin).
        if !unsafe { sys::ecs_is_alive(world_ptr, *entity) } {
            return None;
        }
        let record = unsafe { sys::ecs_record_find(world_ptr, *entity) };
        if record.is_null() {
            return None;
        }

        // SAFETY: record was just looked up for `entity` on this world.
        let data = unsafe { <&mut T as GetTuple>::create_ptrs::<false>(world, entity, record) };
        let ptr = data.component_ptrs()[0];
        if ptr.is_null() {
            return None;
        }
        // SAFETY: ptr is a valid, uniquely-owned pointer to the component; the
        // returned reference is tied to `&mut self`, so nothing else can alias
        // it for `'w`.
        Some(unsafe { &mut *(ptr as *mut <T as ComponentOrPairId>::CastType) })
    }
}
