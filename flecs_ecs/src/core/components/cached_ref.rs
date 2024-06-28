//! Refs are a fast mechanism for referring to a specific entity/component

use std::{marker::PhantomData, os::raw::c_void, ptr::NonNull};

use crate::core::*;
use crate::sys;

/// A reference to a component from a specific entity.
/// Refs are a fast mechanism for referring to a specific entity/component
#[derive(Debug)]
pub struct CachedRef<'a, T: ComponentId + DataComponent> {
    world: WorldRef<'a>,
    component_ref: RefT,
    _marker: PhantomData<T>,
}

impl<'a, T> Clone for CachedRef<'a, T>
where
    T: ComponentId + DataComponent,
{
    fn clone(&self) -> Self {
        Self {
            world: self.world.clone(),
            component_ref: self.component_ref,
            _marker: PhantomData,
        }
    }
}

impl<'a, T> Copy for CachedRef<'a, T> where T: ComponentId + DataComponent {}

impl<'a, T: ComponentId + DataComponent> CachedRef<'a, T> {
    /// Create a new ref to a component.
    ///
    /// # Arguments
    ///
    /// * `world`: the world.
    /// * `entity`: the entity to reference.
    /// * `id`: the id of the component to reference.
    ///
    /// # See also
    ///
    /// * C++ API: `ref::ref`
    ///
    #[doc(alias = "ref::ref")]
    pub fn new(world: impl IntoWorld<'a>, entity: impl Into<Entity>, mut id: IdT) -> Self {
        // the world we were called with may be a stage; convert it to a world
        // here if that is the case
        let world_ptr =
            unsafe { sys::ecs_get_world(world.world_ptr_mut() as *const c_void) as *mut WorldT };

        if id == 0 {
            id = T::id(world);
        }

        ecs_assert!(
            std::mem::size_of::<T>() != 0,
            FlecsErrorCode::InvalidParameter
        );

        let component_ref = unsafe { sys::ecs_ref_init_id(world_ptr, *entity.into(), id) };
        assert_ne!(
            component_ref.entity, 0,
            "Tried to create invalid `CachedRef` type."
        );
        CachedRef {
            world: unsafe { WorldRef::from_ptr(world_ptr) },
            component_ref,
            _marker: PhantomData,
        }
    }

    /// Try to get component from ref.
    ///
    /// # See also
    ///
    /// * C++ API: `ref::try_get`
    #[doc(alias = "ref::try_get")]
    pub fn try_get(&mut self) -> Option<&mut T> {
        NonNull::new(unsafe {
            sys::ecs_ref_get_id(
                self.world.world_ptr_mut(),
                &mut self.component_ref,
                self.component_ref.id,
            ) as *mut T
        })
        .map(|mut t| unsafe { t.as_mut() })
    }

    pub fn get(&mut self) -> &mut T {
        self.try_get()
            .expect("Called CachedRef::get but the CachedRef was invalid")
    }

    pub fn entity(&self) -> EntityView<'a> {
        EntityView::new_from(self.world, self.component_ref.entity)
    }
}
