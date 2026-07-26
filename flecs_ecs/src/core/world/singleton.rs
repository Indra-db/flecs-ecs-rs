use super::*;

impl World {
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
