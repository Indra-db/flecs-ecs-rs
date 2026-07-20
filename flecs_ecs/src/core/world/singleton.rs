use super::*;

pub trait WorldGet<Return> {
    /// gets a mutable or immutable singleton component and/or relationship(s) from the world and return a value.
    /// Only one singleton component at a time is retrievable, but you can call this function multiple times within the callback.
    /// each component type must be marked `&` or `&mut` to indicate if it is mutable or not.
    /// use `Option` wrapper to indicate if the component is optional.
    ///
    /// - `try_get` assumes when not using `Option` wrapper, that the entity has the component.
    ///   If it does not, it will not run the callback.
    ///   If unsure and you still want to have the callback be ran, use `Option` wrapper instead.
    ///
    /// # Note
    ///
    /// - You cannot get single component tags with this function, use `has` functionality instead.
    /// - You can only get relationships with a payload, so where one is not a tag / not a zst.
    ///   tag relationships, use `has` functionality instead.
    /// - This causes the table to lock where the entity belongs to to prevent invalided references, see #Panics.
    ///   The lock is dropped at the end of the callback.
    ///
    /// # Panics
    ///
    /// - This will panic if within the callback you do any operation that could invalidate the reference.
    ///   This happens when the entity is moved to a different table in memory. Such as adding, removing components or
    ///   creating/deleting entities where the entity belongs to the same table (which could cause a table grow operation).
    ///   In case you need to do such operations, you can either do it after the get operation or defer the world with `world.defer_begin()`.
    ///
    /// # Returns
    ///
    /// - If the callback was run, the return value of the callback wrapped in [`Some`]
    /// - Otherwise, returns [`None`]
    ///
    /// # Example
    ///
    /// ```
    /// use flecs_ecs::prelude::*;
    ///
    /// #[derive(Component)]
    /// struct Tag;
    ///
    /// #[derive(Component)]
    /// pub struct Velocity {
    ///     pub x: f32,
    ///     pub y: f32,
    /// }
    ///
    /// #[derive(Component)]
    /// pub struct Position {
    ///     pub x: f32,
    ///     pub y: f32,
    /// }
    ///
    /// let world = World::new();
    ///
    /// world.set(Position { x: 10.0, y: 20.0 });
    /// world.set_pair::<Tag, Position>(Position { x: 30.0, y: 40.0 });
    ///
    /// let val = world.try_get::<&Position>(|pos| pos.x);
    /// assert_eq!(val, Some(10.0));
    ///
    /// let val = world.try_get::<&Velocity>(|vel| vel.x);
    /// assert_eq!(val, None);
    ///
    /// let has_run = world
    ///     .try_get::<&mut (Tag, Position)>(|pos| {
    ///         assert_eq!(pos.x, 30.0);
    ///     })
    ///     .is_some();
    /// assert!(has_run);
    /// ```
    fn try_get<T: GetTuple>(
        &self,
        callback: impl for<'e> FnOnce(T::TupleType<'e>) -> Return,
    ) -> Option<Return>;

    /// gets a mutable or immutable singleton component and/or relationship(s) from the world and return a value.
    /// Only one singleton component at a time is retrievable, but you can call this function multiple times within the callback.
    /// each component type must be marked `&` or `&mut` to indicate if it is mutable or not.
    /// use `Option` wrapper to indicate if the component is optional.
    ///
    /// # Note
    ///
    /// - You cannot get single component tags with this function, use `has` functionality instead.
    /// - You can only get relationships with a payload, so where one is not a tag / not a zst.
    ///   tag relationships, use `has` functionality instead.
    /// - This causes the table to lock where the entity belongs to to prevent invalided references, see #Panics.
    ///   The lock is dropped at the end of the callback.
    ///
    /// # Panics
    ///
    /// - This will panic if within the callback you do any operation that could invalidate the reference.
    ///   This happens when the entity is moved to a different table in memory. Such as adding, removing components or
    ///   creating/deleting entities where the entity belongs to the same table (which could cause a table grow operation).
    ///   In case you need to do such operations, you can either do it after the get operation or defer the world with `world.defer_begin()`.
    ///
    /// - `get` assumes when not using `Option` wrapper, that the entity has the component.
    ///   This will panic if the entity does not have the component. If unsure, use `Option` wrapper or `try_get` function instead.
    ///   `try_get` does not run the callback if the entity does not have the component that isn't marked `Option`.
    ///
    /// # Example
    ///
    /// ```
    /// use flecs_ecs::prelude::*;
    ///
    /// #[derive(Component)]
    /// struct Tag;
    ///
    /// #[derive(Component)]
    /// pub struct Position {
    ///     pub x: f32,
    ///     pub y: f32,
    /// }
    ///
    /// let world = World::new();
    ///
    /// world.set(Position { x: 10.0, y: 20.0 });
    /// world.set_pair::<Tag, Position>(Position { x: 30.0, y: 40.0 });
    ///
    /// let val = world.get::<&Position>(|pos| pos.x);
    /// assert_eq!(val, 10.0);
    ///
    /// world.get::<&mut (Tag, Position)>(|pos| {
    ///     assert_eq!(pos.x, 30.0);
    /// });
    /// ```
    ///
    /// # See also
    ///
    /// * [`World::cloned()`]
    fn get<T: GetTuple>(&self, callback: impl for<'e> FnOnce(T::TupleType<'e>) -> Return)
    -> Return;
}
impl<Return> WorldGet<Return> for World {
    fn try_get<T: GetTuple>(
        &self,
        callback: impl for<'e> FnOnce(T::TupleType<'e>) -> Return,
    ) -> Option<Return> {
        let tuple_data = T::create_ptrs_singleton::<false>(self);
        let has_all_components = tuple_data.has_all_components();

        if has_all_components {
            let tuple = tuple_data.get_tuple();
            let _defer = crate::core::world_ctx::DeferGuard::new(self.world());

            #[cfg(feature = "flecs_safety_locks")]
            {
                let world_ref = self.world();
                let multithreaded = self.is_currently_multithreaded();

                if multithreaded {
                    return Some(rw_locking::<T, Return, true>(
                        &world_ref, callback, tuple_data, tuple,
                    ));
                }
                return Some(rw_locking::<T, Return, false>(
                    &world_ref, callback, tuple_data, tuple,
                ));
            }

            #[cfg(not(feature = "flecs_safety_locks"))]
            {
                return Some(callback(tuple));
            }
        }
        None
    }

    fn get<T: GetTuple>(
        &self,
        callback: impl for<'e> FnOnce(T::TupleType<'e>) -> Return,
    ) -> Return {
        let tuple_data = T::create_ptrs_singleton::<true>(self);
        let tuple = tuple_data.get_tuple();
        let _defer = crate::core::world_ctx::DeferGuard::new(self.world());

        #[cfg(feature = "flecs_safety_locks")]
        {
            let world_ref = self.world();
            let multithreaded = self.is_currently_multithreaded();
            if multithreaded {
                rw_locking::<T, Return, true>(&world_ref, callback, tuple_data, tuple)
            } else {
                rw_locking::<T, Return, false>(&world_ref, callback, tuple_data, tuple)
            }
        }

        #[cfg(not(feature = "flecs_safety_locks"))]
        {
            callback(tuple)
        }
    }
}

impl World {
    /// Clones a singleton component and/or relationship from the world and returns it.
    /// each component type must be marked `&`. This helps Rust type checker to determine if it's a relationship.
    /// use `Option` wrapper to indicate if the component is optional.
    /// use `()` tuple format when getting multiple components.
    ///
    /// # Note
    ///
    /// - You cannot clone component tags with this function.
    /// - You can only clone relationships with a payload, so where one is not a tag / not a zst.
    ///
    /// # Panics
    ///
    /// - This will panic if the world does not have the singleton component that isn't marked `Option`.
    ///
    /// # Example
    ///
    /// ```
    /// use flecs_ecs::prelude::*;
    ///
    /// #[derive(Component)]
    /// struct Tag;
    ///
    /// #[derive(Component, Clone)]
    /// pub struct Position {
    ///     pub x: f32,
    ///     pub y: f32,
    /// }
    ///
    /// #[derive(Component, Clone)]
    /// pub struct Velocity {
    ///     pub x: f32,
    ///     pub y: f32,
    /// }
    ///
    /// let world = World::new();
    ///
    /// world.set(Position { x: 10.0, y: 20.0 });
    /// world.set_pair::<Tag, Position>(Position { x: 30.0, y: 40.0 });
    ///
    /// let pos = world.cloned::<&Position>();
    /// assert_eq!(pos.x, 10.0);
    ///
    /// let tag_pos = world.cloned::<&(Tag, Position)>();
    /// assert_eq!(tag_pos.x, 30.0);
    ///
    /// let vel = world.cloned::<Option<&Velocity>>();
    /// assert!(vel.is_none());
    /// ```
    ///
    /// # See also
    ///
    /// * [`World::get()`]
    #[must_use]
    pub fn cloned<T: ClonedTuple>(&self) -> <T as cloned_tuple::ClonedTuple>::TupleType<'_> {
        let tuple_data = T::create_ptrs_singleton::<true>(self);

        #[cfg(feature = "flecs_safety_locks")]
        {
            let world = self.real_world();
            let safety_info = tuple_data.safety_info();

            let multithreaded = self.is_currently_multithreaded();

            if multithreaded {
                clone_locking::<true>(world, tuple_data.component_ptrs(), safety_info);
            } else {
                // single-threaded mode
                clone_locking::<false>(world, tuple_data.component_ptrs(), safety_info);
            }
        }
        tuple_data.get_tuple()
    }

    /// Tries to clone a singleton component and/or relationship from the world and returns it.
    /// each component type must be marked `&`. This helps Rust type checker to determine if it's a relationship.
    /// use `Option` wrapper to indicate if the component is optional.
    /// use `()` tuple format when getting multiple components.
    ///
    /// If the world does not have all the requested components, returns [`None`].
    /// If the world has all the requested components, returns the cloned components wrapped in [`Some`].
    ///
    /// # Note
    ///
    /// - You cannot clone component tags with this function.
    /// - You can only clone relationships with a payload, so where one is not a tag / not a zst.
    ///
    /// # Panics
    ///
    /// - This will panic if within the function you do any operation that could invalidate the reference.
    ///   This happens when the entity is moved to a different table in memory. Such as adding
    ///   removing components or creating/deleting entities where the entity belongs to the same table.
    ///   In case you need to do such operations, you can either do it after the get operation or
    ///   use a different approach to avoid invalidating the reference.
    pub fn try_cloned<T: ClonedTuple>(
        &self,
    ) -> Option<<T as cloned_tuple::ClonedTuple>::TupleType<'_>> {
        let tuple_data = T::create_ptrs_singleton::<false>(self);

        //todo we can maybe early return if we don't yet if doesn't have all. Same for try_get
        let has_all_components = tuple_data.has_all_components();

        if has_all_components {
            #[cfg(feature = "flecs_safety_locks")]
            {
                let world = self.real_world();
                let safety_info = tuple_data.safety_info();

                let multithreaded = self.is_currently_multithreaded();

                if multithreaded {
                    clone_locking::<true>(world, tuple_data.component_ptrs(), safety_info);
                } else {
                    clone_locking::<false>(world, tuple_data.component_ptrs(), safety_info);
                }
            }
            Some(tuple_data.get_tuple())
        } else {
            None
        }
    }

    /// Get a reference to a singleton component.
    ///
    /// A reference allows for quick and safe access to a component value, and is
    /// a faster alternative to repeatedly calling `get` for the same component.
    ///
    /// - `T`: Component for which to get a reference.
    ///
    /// Returns: The reference singleton component.
    #[inline(always)]
    pub fn cached_ref<'a, T: IntoEntity + Copy>(
        &'a self,
        component: T,
    ) -> CachedRef<'a, <T as IntoEntity>::CastType> {
        let component_id = component.into_entity(self);
        EntityView::new_from(self, component_id).cached_ref(component)
    }

    /// Get singleton entity for type.
    ///
    /// # Type Parameters
    ///
    /// * `T` - The component type to get the singleton entity for.
    ///
    /// # Returns
    ///
    /// The entity representing the component.
    #[inline(always)]
    pub fn singleton<T: ComponentId>(&self) -> EntityView<'_> {
        EntityView::new_from(self, T::entity_id(self))
    }

    /// Retrieves the target for a given pair from a singleton entity.
    ///
    /// This operation fetches the target associated with a specific pair. An optional
    /// `index` parameter allows iterating through multiple targets if the entity
    /// has more than one instance of the same relationship.
    ///
    /// # Arguments
    ///
    /// * `first` - The first element of the pair for which to retrieve the target.
    /// * `index` - The index (0 for the first instance of the relationship).
    ///
    /// # See also
    ///
    /// * [`World::target()`]
    pub fn target(&self, relationship: impl IntoEntity, index: Option<usize>) -> EntityView<'_> {
        let relationship = *relationship.into_entity(self);
        EntityView::new_from(self, unsafe {
            sys::ecs_get_target(
                self.raw_world.as_ptr(),
                relationship,
                relationship,
                index.unwrap_or(0) as i32,
            )
        })
    }

    /// Check if world has the provided id.
    ///
    /// # Arguments
    ///
    /// * `id`: The id to check of a pair, entity or component.
    ///
    /// # Returns
    ///
    /// True if the world has the provided id, false otherwise.
    ///
    /// # See also
    ///
    /// * [`World::has()`]
    /// * [`World::has_enum()`]
    #[inline(always)]
    pub fn has<T: IntoId>(&self, id: T) -> bool {
        let id = *id.into_id(self);
        if T::IS_PAIR {
            let first_id = id.get_id_first(self);
            EntityView::new_from(self, first_id).has(id)
        } else {
            EntityView::new_from(self, id).has(id)
        }
    }

    /// Check if world has the provided enum constant.
    ///
    /// # Type Parameters
    ///
    /// * `T` - The enum type.
    ///
    /// # Arguments
    ///
    /// * `constant` - The enum constant to check.
    ///
    /// # Returns
    ///
    /// True if the world has the provided constant, false otherwise.
    ///
    /// # See also
    ///
    /// * [`World::has()`]
    #[inline(always)]
    pub fn has_enum<T>(&self, constant: T) -> bool
    where
        T: ComponentId + ComponentType<Enum> + EnumComponentInfo,
    {
        EntityView::new_from(self, T::entity_id(self)).has_enum(constant)
    }

    /// Add a singleton component by id.
    /// id can be a component, entity or pair id.
    ///
    /// # Arguments
    ///
    /// * `id`: The id of the component to add.
    ///
    /// # Returns
    ///
    /// `EntityView` handle to the singleton component.
    #[inline(always)]
    pub fn add<T: IntoId>(&self, id: T) -> EntityView<'_> {
        let id = *id.into_id(self);
        // this branch will compile out in release mode
        if T::IS_PAIR {
            let first_id = id.get_id_first(self);
            EntityView::new_from(self, first_id).add(id)
        } else {
            EntityView::new_from(self, id).add(id)
        }
    }

    /// Add a singleton enum component.
    ///
    /// # Type Parameters
    ///
    /// * `T` - The enum component to add.
    ///
    /// # Returns
    ///
    /// `EntityView` handle to the singleton enum component.
    #[inline(always)]
    pub fn add_enum<T: ComponentId + ComponentType<Enum> + EnumComponentInfo>(
        &self,
        enum_value: T,
    ) -> EntityView<'_> {
        EntityView::new_from(self, T::entity_id(self)).add_enum::<T>(enum_value)
    }

    /// Add a singleton pair with enum tag.
    ///
    /// # Type Parameters
    ///
    /// * `First` - The first element of the pair.
    /// * `Second` - The second element of the pair of type enum.
    ///
    /// # Arguments
    ///
    /// * `enum_value`: The enum value to add.
    ///
    /// # Returns
    ///
    /// `EntityView` handle to the singleton pair.
    #[inline(always)]
    pub fn add_pair_enum<First, Second>(&self, enum_value: Second) -> EntityView<'_>
    where
        First: ComponentId,
        Second: ComponentId + ComponentType<Enum> + EnumComponentInfo,
    {
        EntityView::new_from(self, First::entity_id(self))
            .add_pair_enum::<First, Second>(enum_value)
    }

    /// Remove singleton component by id.
    /// id can be a component, entity or pair id.
    ///
    /// # Arguments
    ///
    /// * `id`: The id of the component to remove.
    pub fn remove<T: IntoId>(&self, id: T) -> EntityView<'_> {
        let id = *id.into_id(self);
        if T::IS_PAIR {
            let first_id = id.get_id_first(self);
            EntityView::new_from(self, first_id).remove(id)
        } else {
            EntityView::new_from(self, id).remove(id)
        }
    }
}
