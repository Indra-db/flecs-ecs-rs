use crate::prelude::*;
use crate::sys;

#[cfg(feature = "std")]
extern crate std;

extern crate alloc;
use alloc::vec::Vec;

// TODO: Would be great to have a set that sets all the data to the same, no need for multiple entries
/// A builder for creating multiple entities in bulk, optionally adding components and data.
///
/// The `BulkEntityBuilder` allows you to efficiently create multiple entities at once,
/// optionally adding components and setting data for them.
pub struct BulkEntityBuilder<'a> {
    desc: sys::ecs_bulk_desc_t,
    data: [*mut core::ffi::c_void; sys::FLECS_ID_DESC_MAX as usize],
    entity_ids: Vec<u64>,
    world: WorldRef<'a>,
    current_id_index: u8,
}

impl<'a> BulkEntityBuilder<'a> {
    /// Creates a new bulk entity builder with a specified count of entities to insert.
    ///
    /// # Parameters
    ///
    /// - `world`: The world in which to create the entities.
    /// - `count`: The number of entities to create.
    ///
    /// # Panics
    ///
    /// This function will panic if `count` is greater than `i32::MAX`.
    pub(crate) fn new(world: impl WorldProvider<'a>, count: u32) -> Self {
        ecs_assert!(
            count <= i32::MAX as u32,
            FlecsErrorCode::InvalidParameter,
            "count must be less than i32::MAX"
        );

        Self {
            world: world.world(),
            desc: sys::ecs_bulk_desc_t {
                _canary: 0,
                entities: core::ptr::null_mut(),
                count: count as i32,
                ids: [0; sys::FLECS_ID_DESC_MAX as usize],
                data: core::ptr::null_mut(),
                table: core::ptr::null_mut(),
            },
            data: [core::ptr::null_mut(); sys::FLECS_ID_DESC_MAX as usize],
            entity_ids: Vec::new(),
            current_id_index: 0,
        }
    }

    /// Creates a new bulk entity builder with a list of existing empty entities to insert.
    ///
    /// # Parameters
    ///
    /// - `world`: The world in which to create the entities.
    /// - `entities`: A slice of existing entity IDs to be used for the new entities, must be entities without components.
    ///
    /// # Panics
    ///
    /// * This function will panic if the length of `entities` is greater than `i32::MAX`.
    /// * This function will panic if any of the provided entities already have components.
    ///
    /// # Notes
    ///
    /// The provided entities must not already have components.
    pub(crate) fn new_with_entity_ids(
        world: impl WorldProvider<'a>,
        entities: &[impl Into<Entity> + Copy],
    ) -> Self {
        ecs_assert!(
            entities.len() <= i32::MAX as usize,
            FlecsErrorCode::InvalidParameter,
            "count must be less than i32::MAX"
        );

        let entity_ids: Vec<u64> = entities.iter().map(|e| *Into::<Entity>::into(*e)).collect();

        Self {
            world: world.world(),
            desc: sys::ecs_bulk_desc_t {
                _canary: 0,
                entities: core::ptr::null_mut(),
                count: entity_ids.len() as i32,
                ids: [0; sys::FLECS_ID_DESC_MAX as usize],
                data: core::ptr::null_mut(),
                table: core::ptr::null_mut(),
            },
            data: [core::ptr::null_mut(); sys::FLECS_ID_DESC_MAX as usize],
            entity_ids,
            current_id_index: 0,
        }
    }

    /// Adds a component or tag or pair of type `T` to the entities to be created.
    ///
    /// # Panics
    ///
    /// This function will panic if `T` is a generic type or if `T` is not a tag and does not implement `Default`.
    ///
    /// # Notes
    ///
    /// If `T` is a non-tag component and does not implement `Default`, you must use [`Self::set`] to provide data for it.
    ///
    /// # Examples
    ///
    /// ```
    /// use flecs_ecs::prelude::*;
    ///
    /// #[derive(Component, Default)]
    /// struct Position {
    ///     x: i32,
    ///     y: i32,
    /// }
    ///
    /// let world = World::new();
    ///
    /// let entities_created = world.entity_bulk(10).add::<Position>().build();
    /// ```
    pub fn add<T>(&mut self) -> &mut Self
    where
        T: ComponentOrPairId,
    {
        const {
            if T::CastType::IS_GENERIC {
                panic!(
                    "Adding a generic type requires to use the set function. This is due to Rust type system limitations."
                );
            } else if !T::CastType::IS_TAG && !T::CastType::IMPLS_DEFAULT {
                panic!(
                    "Adding an element that is not a Tag / Zero sized type requires to implement Default"
                );
            }
        }
        self.add_id_unchecked(T::get_id(self.world))
    }

    /// Adds a component or tag or pair identified by `id` to the entities to be created.
    ///
    /// # Parameters
    ///
    /// - `id`: The ID of the component or tag or pair to add.
    ///
    /// # Panics
    ///
    /// * This function will panic if `id` is invalid in the current world.
    /// * If the ID is not a tag and does not implement `Default`.
    ///   // TODO: `set_pair`
    ///
    /// # Examples
    ///
    /// ```
    /// use flecs_ecs::prelude::*;
    ///
    /// let world = World::new();
    ///
    /// let entity = world.entity();
    ///
    /// let entities_created = world.entity_bulk(10).add_id(entity).build();
    /// ```
    pub fn add_id(&mut self, id: impl IntoId) -> &mut Self {
        let id = *id.into_id(self.world);
        let world = self.world.world_ptr_mut();

        check_add_id_validity(world, id);

        self.add_id_unchecked(id)
    }

    fn add_id_unchecked(&mut self, id: u64) -> &mut Self {
        self.desc.ids[self.current_id_index as usize] = id;
        self.current_id_index += 1;
        self
    }

    /// Sets component data for a component of type `T` for the entities to be created.
    ///
    /// # Parameters
    ///
    /// - `component_data`: A slice of component data to assign to the entities.
    ///
    /// # Panics
    ///
    /// This function will panic if the length of `component_data` is not equal to the count of entities to be created.
    ///
    /// # Examples
    ///
    /// ```
    /// use flecs_ecs::prelude::*;
    ///
    /// #[derive(Component, Default)]
    /// struct Position {
    ///     x: i32,
    ///     y: i32,
    /// }
    ///
    /// let world = World::new();
    ///
    /// let positions: Vec<Position> = (0..10).map(|i| Position { x: i, y: i }).collect();
    ///
    /// let entities_created = world.entity_bulk(10).set(&positions).build();
    /// ```
    pub fn set<T: ComponentId + DataComponent>(&mut self, component_data: &'a [T]) -> &mut Self {
        assert!(
            component_data.len() == self.desc.count as usize,
            "component_data length must be equal to count of entities"
        );
        let id = T::entity_id(self.world);

        self.desc.ids[self.current_id_index as usize] = id;
        self.data[self.current_id_index as usize] =
            component_data.as_ptr() as *mut core::ffi::c_void;
        self.current_id_index += 1;
        self
    }

    /// build & bulk create the entities and returns a vector of their IDs.
    ///
    /// # Returns
    ///
    /// A vector containing the IDs of the newly created entities.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use flecs_ecs::prelude::*;
    ///
    /// #[derive(Component, Default)]
    /// struct Position {
    ///     x: i32,
    ///     y: i32,
    /// }
    ///
    /// let world = World::new();
    ///
    /// let positions: Vec<Position> = (0..10).map(|i| Position { x: i, y: i }).collect();
    ///
    /// let entities_created = world.entity_bulk(10).set(&positions).build();
    /// ```
    pub fn build(&mut self) -> Vec<Entity> {
        assert_not_in_multithreaded_phase(self.world.world_ptr());
        crate::core::assert_no_live_pin(&self.world, "World::entity_bulk(..).build()");
        self.desc.data = self.data.as_ptr() as *mut _;
        if !self.entity_ids.is_empty() {
            self.desc.entities = self.entity_ids.as_mut_ptr();
        }
        // SAFETY: the world pointer is valid for 'a. `desc.entities` either is null or points
        // into `self.entity_ids`, an owned buffer of `desc.count` raw entity ids. `desc.data`
        // slices are borrowed for 'a via `set`, so they outlive this call.
        let entities = unsafe { sys::ecs_bulk_init(self.world.world_ptr_mut(), &self.desc) };
        // SAFETY: `ecs_bulk_init` returns a pointer to `desc.count` valid entity ids: either
        // `desc.entities` (owned by self) or a live internal buffer of the world.
        unsafe { core::slice::from_raw_parts(entities, self.desc.count as usize) }
            .iter()
            .map(|&e| Entity::from(e))
            .collect::<Vec<_>>()
    }

    /// Builds & bulk create the entities into the specified table and returns their IDs.
    ///
    /// Any components or tags added with [`Self::set`] or [`Self::add`] that are not part of the table will be ignored.
    ///
    /// # Parameters
    ///
    /// - `table`: The table into which to build the entities.
    ///
    /// # Panics
    ///
    /// This function will panic if components in the table do not have a default hook (do not implement `Default`)
    /// and no data is set for them using [`Self::set`].
    ///
    /// # Returns
    ///
    /// A vector containing the IDs of the newly created entities.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use flecs_ecs::prelude::*;
    ///
    /// #[derive(Component)]
    /// struct Position {
    ///     x: i32,
    ///     y: i32,
    /// }
    ///
    /// #[derive(Component, Default)]
    /// struct Velocity {
    ///     x: i32,
    ///     y: i32,
    /// }
    ///
    /// let world = World::new();
    ///
    /// let ent_id = world.entity();
    ///
    /// let ent = world
    ///     .entity()
    ///     .set(Position { x: 0, y: 0 })
    ///     .add(Velocity::id())
    ///     .add(ent_id);
    ///
    /// let mut table = ent.table().unwrap();
    ///
    /// let positions: [Position; 10] = core::array::from_fn(|i| Position {
    ///     x: i as i32,
    ///     y: i as i32,
    /// });
    ///
    /// let entities = world
    ///     .entity_bulk(10)
    ///     .add::<Velocity>() // defaulted
    ///     .set(&positions) // since position has no default, it must be set
    ///     .build_to_table(&mut table);
    /// ```
    pub fn build_to_table(&mut self, table: &mut Table) -> Vec<Entity> {
        assert_not_in_multithreaded_phase(self.world.world_ptr());
        crate::core::assert_no_live_pin(&self.world, "World::entity_bulk(..).build_to_table()");
        if self.data.is_empty() {
            let mut all_default = true;

            let world = self.world.world_ptr_mut();
            let arch = table.archetype();
            let arch_iter = arch.as_slice().iter();
            arch_iter.for_each(|id| {
                let id = *(*id);
                // SAFETY: the world pointer is valid for 'a and `id` comes from a live archetype.
                let is_not_tag = unsafe { sys::ecs_get_typeid(world, id) != 0 };

                if is_not_tag && !has_default_hook(world, id) {
                    all_default = false;
                }
            });
            assert!(
                all_default,
                "Data not set, set the data or ensure all components in the table must have a default constructor"
            );
        } else {
            let world = self.world.world_ptr_mut();
            let arch = table.archetype();
            let arch_slice = arch.as_slice();
            let mut index = 0;
            arch_slice.iter().for_each(|id| {
                let id = *(*id);
                // SAFETY: the world pointer is valid for 'a and `id` comes from a live archetype.
                let is_a_tag = unsafe { sys::ecs_get_typeid(world, id) == 0 };

                if is_a_tag {
                    if !self.data[index].is_null() {
                        let null_pos = self.data.iter().position(|&x| x.is_null());
                        self.data.swap(index, null_pos.unwrap());
                        self.desc.ids.swap(index, null_pos.unwrap());
                    }
                } else if self.desc.ids[index] == id {
                    if self.data[index].is_null() {
                        assert!(
                            has_default_hook(world, id),
                            "Data not set, set the data or ensure the component has a default constructor"
                        );
                    }
                } else {
                    let id_pos = self.desc.ids.iter().position(|&x| x == id);
                    if let Some(id_pos) = id_pos {
                        if self.data[id_pos].is_null() {
                            assert!(
                                has_default_hook(world, id),
                                "Data not set, set the data or ensure the component has a default constructor"
                            );
                        }
                        let is_same_index = id_pos == index;
                        if !is_same_index {
                            self.data.swap(index, id_pos);
                            self.desc.ids.swap(index, id_pos);
                        }
                    } else {
                        assert!(
                            has_default_hook(world, id),
                            "Data not set, set the data or ensure the component has a default constructor"
                        );
                        // Guaranteed to be null in self.data
                    }
                }

                index += 1;
            });
        }

        // SAFETY: `table.table` is a `NonNull` to a live table owned by the same world.
        self.desc.table = unsafe { table.table.as_mut() };
        let entities = self.build();
        self.desc.table = core::ptr::null_mut();
        entities
    }

    /// Returns the number of entities to be created.
    pub fn count(&self) -> u32 {
        self.desc.count as u32
    }
}

impl World {
    /// Creates a new bulk entity builder to create `count` entities.
    ///
    /// # Parameters
    ///
    /// - `count`: The number of entities to create.
    ///
    /// # Returns
    ///
    /// A `BulkEntityBuilder` for configuring and creating the entities.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use flecs_ecs::prelude::*;
    ///
    /// #[derive(Component, Default)]
    /// struct Position {
    ///     x: i32,
    ///     y: i32,
    /// }
    ///
    /// let world = World::new();
    ///
    /// let positions: Vec<Position> = (0..10).map(|i| Position { x: i, y: i }).collect();
    ///
    /// let entities_created = world.entity_bulk(10).set(&positions).build();
    /// ```
    pub fn entity_bulk(&self, count: u32) -> BulkEntityBuilder<'_> {
        BulkEntityBuilder::new(self, count)
    }

    /// Creates a new bulk entity builder with the specified entity IDs.
    ///
    /// # Parameters
    ///
    /// - `entities`: A slice of entity IDs to use for the new entities.
    ///
    /// # Returns
    ///
    /// A `BulkEntityBuilder` for configuring and creating the entities.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// use flecs_ecs::prelude::*;
    ///
    /// #[derive(Component, Default)]
    /// struct Position {
    ///     x: i32,
    ///     y: i32,
    /// }
    ///
    /// let world = World::new();
    ///
    /// let positions: Vec<Position> = (0..10).map(|i| Position { x: i, y: i }).collect();
    ///
    /// let entities: Vec<Entity> = (0..10).map(|_| world.entity().id()).collect();
    /// let new_entities = world
    ///     .entity_bulk_with_entity_ids(&entities)
    ///     .set(&positions)
    ///     .build();
    /// ```
    pub fn entity_bulk_with_entity_ids(
        &self,
        entities: &[impl Into<Entity> + Copy],
    ) -> BulkEntityBuilder<'_> {
        BulkEntityBuilder::new_with_entity_ids(self, entities)
    }
}
