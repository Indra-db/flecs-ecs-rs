use std::marker::PhantomData;
use std::{alloc::Layout, os::raw::c_void};

use crate::core::*;
use crate::sys;

/// A strongly-typed interface wrapper around `EventBuilderUntyped` for constructing events with specific data.
///
/// # Type parameters
///
/// * `T` - The type of the event data to set for the event, which must implement `ComponentId`.
///
/// Ensures the use of appropriate data types for events, enhancing type safety and data integrity.
/// This design aims to prevent the utilization of incompatible components as event data,
/// thereby ensuring greater explicitness and correctness in event handling.
pub struct EventBuilder<'a, T = ()> {
    pub world: WorldRef<'a>,
    pub(crate) desc: sys::ecs_event_desc_t,
    pub(crate) ids: TypeT,
    pub(crate) ids_array: [IdT; sys::FLECS_EVENT_DESC_MAX as usize],
    _phantom: std::marker::PhantomData<T>,
}

impl<'a, T: ComponentId> EventBuilder<'a, T> {
    /// Create a new typed `EventBuilder`
    ///
    /// # Arguments
    ///
    /// * `world` - The world to create the `EventBuilderUntyped` in
    /// * `event` - The event to create the `EventBuilderUntyped` for
    ///
    /// # See also
    ///
    /// * C++ API: `event_builder_typed::event_builder_typed`
    #[doc(alias = "event_builder_typed::event_builder_typed")]
    pub fn new(world: impl IntoWorld<'a>) -> Self {
        let mut obj = Self {
            world: world.world(),
            desc: Default::default(),
            ids: Default::default(),
            ids_array: Default::default(),
            _phantom: PhantomData,
        };
        obj.desc.event = T::UnderlyingType::id(world);
        obj
    }

    /// Create a new (untyped) `EventBuilderUntyped`
    ///
    /// # Safety
    /// Caller must ensure either that `event` represents a ZST
    /// or the event data is set to point to the appropriate type
    ///
    /// # Arguments
    ///
    /// * `world` - The world to create the `EventBuilderUntyped` in
    /// * `event` - The event to create the `EventBuilderUntyped` for
    ///
    /// # See also
    ///
    /// * C++ API: `event_builder_base::event_builder_base`
    #[doc(alias = "event_builder_base::event_builder_base")]
    pub unsafe fn new_untyped(
        world: impl IntoWorld<'a>,
        event: impl Into<Entity>,
    ) -> EventBuilder<'a, ()> {
        let mut obj = EventBuilder::<'a, ()> {
            world: world.world(),
            desc: Default::default(),
            ids: Default::default(),
            ids_array: Default::default(),
            _phantom: PhantomData::<()>,
        };
        obj.desc.event = *event.into();
        obj
    }

    /// Add component id or pair to emit for the event
    ///
    /// # Arguments
    ///
    /// * `id` - The id of the component to add to the event
    ///
    /// # See also
    ///
    /// * C++ API: `event_builder_base::id`
    #[doc(alias = "event_builder_base::id")]
    pub fn add_id(&mut self, id: impl IntoId) -> &mut Self {
        let id = *id.into();
        let ids = &mut self.ids;
        let ids_array = &mut self.ids_array;
        ids.array = ids_array.as_mut_ptr();
        unsafe {
            *ids.array.add(ids.count as usize) = id;
        }
        ids.count += 1;
        self
    }

    /// Add component to emit for the event
    ///
    /// # Type parameters
    ///
    /// * `C` - The component to add to the event
    ///
    /// # See also
    ///
    /// * C++ API: `event_builder_base::id`
    #[doc(alias = "event_builder_base::id")]
    pub fn add<C>(&mut self) -> &mut Self
    where
        C: ComponentOrPairId,
    {
        let world = self.world;
        self.add_id(C::get_id(world))
    }

    /// Add a pair of components to emit for the event
    ///
    /// # Type parameters
    ///
    /// * `First` - The first component to add to the event
    ///
    /// # Arguments
    ///
    /// * `second` - The id of the second component to add to the event
    ///
    /// # See also
    ///
    /// * C++ API: `event_builder_base::id`
    #[doc(alias = "event_builder_base::id")]
    fn add_first<First>(&mut self, second: impl Into<Entity>) -> &mut Self
    where
        First: ComponentId,
    {
        let world = self.world;
        self.add_id(ecs_pair(First::id(world), *second.into()))
    }

    #[doc(alias = "event_builder_base::id")]
    fn add_second<Second>(&mut self, first: impl Into<Entity>) -> &mut Self
    where
        Second: ComponentId,
    {
        let world = self.world;
        self.add_id(ecs_pair(*first.into(), Second::id(world)))
    }

    /// Set the entity to emit for the event
    ///
    /// # Arguments
    ///
    /// * `entity` - The entity to emit for the event
    ///
    /// # See also
    ///
    /// * C++ API: `event_builder_base::entity`
    #[doc(alias = "event_builder_base::entity")]
    pub fn target(&mut self, entity: impl Into<Entity>) -> &mut Self {
        let desc = &mut self.desc;
        desc.entity = *entity.into();
        self
    }

    /// Set the table to emit for the event
    ///
    /// # Arguments
    ///
    /// * `table` - The table to emit for the event
    /// * `offset` - The offset tof the table to emit for the event
    /// * `count` - The count of the table to emit for the event
    ///
    /// # See also
    ///
    /// * C++ API: `event_builder_base::table`
    #[doc(alias = "event_builder_base::table")]
    pub fn table(&mut self, table: impl IntoTable, offset: i32, count: i32) -> &mut Self {
        let desc = &mut self.desc;
        desc.table = table.table_ptr_mut();
        desc.offset = offset;
        desc.count = count;
        self
    }

    pub fn emit(&mut self, data: &mut T) {
        let ids = &mut self.ids;
        let ids_array = &mut self.ids_array;
        let desc = &mut self.desc;
        let world = self.world;
        ids.array = ids_array.as_mut_ptr();

        if !T::IS_TAG {
            desc.const_param = data as *const T as *const c_void;
        }

        desc.ids = ids;
        desc.observable = world.real_world().world_ptr_mut() as *mut c_void;
        unsafe { sys::ecs_emit(world.world_ptr_mut(), desc) };
    }

    pub fn enqueue(&mut self, data: T) {
        let ids = &mut self.ids;
        let ids_array = &mut self.ids_array;
        let desc = &mut self.desc;
        let world = self.world;
        ids.array = ids_array.as_mut_ptr();

        if !T::IS_TAG {
            desc.param = Box::leak(Box::new(data)) as *mut T as *mut c_void;
        }

        desc.ids = ids;
        desc.observable = world.real_world().world_ptr_mut() as *mut c_void;
        unsafe {
            sys::ecs_enqueue(world.world_ptr_mut(), desc);
            if !T::IS_TAG {
                std::alloc::dealloc(desc.param as *mut u8, Layout::new::<T>());
            }
        };
    }
}
