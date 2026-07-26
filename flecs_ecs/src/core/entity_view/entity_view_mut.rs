use core::ffi::c_void;

use flecs_ecs::core::*;
use sys::EcsIsA;

use crate::{core::flecs::FlecsComponentTrait, sys};

// functions in here match most of the functions in the c++ entity and entity_builder class
impl<'a> EntityView<'a> {
    /// Adds an ID to the entity.
    ///
    /// The provided `id` can represent various types, including a component, a pair, a tag, or another entity.
    ///
    /// # Panics
    ///
    /// This function will panic if the `id` does not meet the following constraints:
    /// - The `id` must be either a zero-sized type (ZST), an entity, or a type that implements a constructor hook.
    /// - Types that implement the [`Default`][core::default::Default] trait automatically fulfill the constructor hook requirement.
    ///
    /// This panic occurs because an invalid `id` type may result in reading uninitialized data, leading to undefined behavior.
    ///
    /// # Usage
    ///
    /// For types that are not ZST and do not implement a constructor hook, use the `set_id` method to safely initialize the `id`.
    #[allow(clippy::should_implement_trait)]
    pub fn add<T: IntoId>(self, id: T) -> Self {
        // Shared-register write hook (spec §3.6, §7.1): defer behind a live guard.
        ensure_write_episode(&self.world);
        let id = *id.into_id(self.world);
        let world = self.world.world_ptr_mut();

        //TODO: seems like add(T::id(),e) is not triggering compiler error, but does runtime check
        const {
            if T::IS_PAIR
                && T::IS_TYPED
                && !T::IF_ID_IS_DEFAULT
                && T::IS_TYPED_SECOND
                && !T::IF_ID_IS_DEFAULT_SECOND
                && !<T as IntoId>::IS_TYPE_TAG
            {
                panic!("none implement default, use `set_pair` instead to ensure valid data");
            }
        }

        const {
            if !T::IS_PAIR && T::IS_TYPED && !T::IF_ID_IS_DEFAULT && !<T as IntoId>::IS_TYPE_TAG {
                panic!("Default hook not implemented for non ZST type");
            }
        }

        if !T::IS_PAIR {
            if !T::IS_TYPED {
                check_add_id_validity(world, id);
            }
        } else if T::IS_TYPED {
            if !T::IF_ID_IS_DEFAULT && !T::IS_TYPED_SECOND {
                check_add_id_validity(world, id);
            }
        } else if T::IS_TYPED_SECOND {
            if !T::IF_ID_IS_DEFAULT_SECOND {
                check_add_id_validity(world, id);
            }
        } else {
            check_add_id_validity(world, id);
        }

        // SAFETY: the world pointer is valid for 'a; id validity (tag/entity or ctor hook for sized components) was verified by the const and runtime checks above.
        unsafe { sys::ecs_add_id(world, *self.id, id) }
        self
    }

    /// Adds an ID to the entity unchecked. Useful for run-time components.
    ///
    /// The provided `id` can represent various types, including a component, a pair, a tag, or another entity.
    ///
    /// # Safety
    /// Caller must ensure the `id` is a valid type.
    /// This function is unsafe because it does not check if the `id` is a valid type nor if
    /// the `id` implements a constructor hook if it is not a zero-sized type (ZST).
    /// if the id is a type without a constructor hook, it could cause you to read uninitialized data.
    /// the caller must ensure to initialize the component data before using it.
    ///
    /// # See Also
    ///
    /// * [`set_id`](Self::set_id)
    pub unsafe fn add_id_unchecked(self, id: impl IntoId) -> Self {
        // Shared-register write hook (spec §3.6, §7.1): defer behind a live guard.
        ensure_write_episode(&self.world);
        let id = *id.into_id(self.world);
        let world = self.world.world_ptr_mut();

        // SAFETY: the world pointer is valid for 'a; the caller upholds this function's contract that the id is valid and initialized if sized.
        unsafe { sys::ecs_add_id(world, *self.id, id) }
        self
    }

    /// Adds a flecs trait.
    pub fn add_trait<T>(self) -> Self
    where
        T: ComponentOrPairId,
        T::First: FlecsComponentTrait,
    {
        let world = self.world;
        // SAFETY: flecs component traits are tags, so the id adds no component data.
        unsafe { self.add_id_unchecked(T::get_id(world)) }
    }

    /// Override a component on an entity.
    /// This is useful if you want to override a component that is inherited by a prefab on a per entity basis
    ///
    /// # Panics
    ///
    /// Caller must ensure the entity has the component to override.
    pub fn override_type<T>(self) -> Self
    where
        T: ComponentOrPairId,
    {
        let id = T::get_id(self.world);
        let world_ptr = self.world.world_ptr_mut();

        // SAFETY: the world pointer is valid for 'a; ecs_get_target_for_id accepts any entity/id values.
        if unsafe { sys::ecs_get_target_for_id(world_ptr, *self.id, EcsIsA, id) } == 0 {
            panic!("Entity does not have the component to override");
        }
        // SAFETY: the entity inherits `id` (checked above), so the override copies the base component data.
        unsafe { self.add_id_unchecked(id) }
    }

    /// Adds a pair to the entity composed of a tag and an (C) flecs enum constant.
    pub fn add_pair_enum<First, Second>(self, enum_value: Second) -> Self
    where
        First: ComponentId,
        Second: ComponentId + ComponentType<Enum> + EnumComponentInfo,
    {
        const {
            if !First::IS_TAG && !First::IMPLS_DEFAULT {
                panic!(
                    "Adding an element that is not a Tag / Zero sized type requires to implement Default"
                );
            }
        }
        let world = self.world;
        let enum_id = enum_value.id_variant(world);
        // SAFETY: First is a tag or implements Default (const-checked above) and the enum variant id is a tag.
        unsafe { self.add_id_unchecked((First::entity_id(world), enum_id)) }
    }

    /// Adds a pair to the entity where the first element is the enumeration type,
    /// and the second element is the enumeration constant.
    ///
    /// This function works only with regular (C style) enumerations.
    ///
    /// # Type Parameters
    ///
    /// - `T`: The enumeration type, which derives from `ComponentId`, `ComponentType<Enum>`, and `EnumComponentInfo`.
    ///
    /// # Arguments
    ///
    /// - `enum_value`: The enumeration value.
    pub fn add_enum<T: ComponentId + ComponentType<Enum> + EnumComponentInfo>(
        self,
        enum_value: T,
    ) -> Self {
        let world = self.world;
        let first = T::entity_id(world);
        // SAFETY: we know that the enum_value is a valid because of the T::id call
        let second = unsafe { enum_value.id_variant_unchecked(world) };
        ecs_assert!(
            second != 0,
            FlecsErrorCode::InvalidParameter,
            "Component was not found in reflection data."
        );
        // SAFETY: the variant id comes from flecs' enum reflection data and was asserted valid above.
        unsafe { self.add_id_unchecked((first, second)) }
    }

    /// Conditional add.
    /// This operation adds if condition is true, removes if condition is false.
    ///
    /// # Arguments
    ///
    /// * `condition`: The condition to evaluate.
    /// * `component`: The component to add.
    pub fn add_if<T: IntoId>(self, id: T, condition: bool) -> Self {
        if condition {
            self.add(id)
        } else {
            // the compiler will optimize this branch away since it's known at compile time
            if T::IS_PAIR {
                // If second is 0 or if relationship is exclusive, use wildcard for
                // second which will remove all instances of the relationship.
                // Replacing 0 with Wildcard will make it possible to use the second
                // as the condition.
                let id = id.into_id(self.world);
                let first = id.get_id_first(self.world);
                let mut second = id.get_id_second(self.world);
                if second == 0
                    // SAFETY: the world pointer is valid for 'a; ecs_has_id accepts any entity/id values.
                    || unsafe { sys::ecs_has_id(self.world.world_ptr(), *first, ECS_EXCLUSIVE) }
                {
                    second = ECS_WILDCARD.into();
                }
                self.remove((first, second))
            } else {
                self.remove(id)
            }
        }
    }

    /// Conditional add.
    /// This operation adds if condition is true, removes if condition is false.
    ///
    /// # Type Parameters
    ///
    /// * `T`: enum type
    ///
    /// # Arguments
    ///
    /// * `condition`: The condition to evaluate.
    /// * `enum_value`: The enumeration constant.
    pub fn add_enum_if<T>(self, enum_value: T, condition: bool) -> Self
    where
        T: ComponentId + ComponentType<Enum> + EnumComponentInfo,
    {
        let world = self.world;
        // SAFETY: we know that the enum_value is a valid because of the T::id call
        self.add_if(
            (T::entity_id(world), unsafe {
                enum_value.id_variant_unchecked(world)
            }),
            condition,
        )
    }

    /// Remove an entity from an entity.
    ///
    /// # Arguments
    ///
    /// * `component_id`: The entity to remove.
    pub fn remove<T: IntoId>(self, id: T) -> Self {
        // Shared-register write hook (spec §3.6, §7.1): defer behind a live guard.
        ensure_write_episode(&self.world);
        let id = *id.into_id(self.world);
        let id = if <T as IntoId>::IS_ENUM {
            ecs_pair(id, ECS_WILDCARD)
        } else {
            id
        };

        // SAFETY: the world pointer is valid for 'a; ecs_remove_id accepts any entity/id values.
        unsafe { sys::ecs_remove_id(self.world.world_ptr_mut(), *self.id, id) }

        self
    }

    /// Shortcut for `add((flecs::IsA, id))`.
    ///
    /// # Arguments
    ///
    /// * `second`: The second element of the pair.
    pub fn is_a(self, second: impl IntoEntity) -> Self {
        // SAFETY: IsA is a builtin tag relationship (PairIsTag), so the pair adds no component data.
        unsafe { self.add_id_unchecked((ECS_IS_A, second.into_entity(self.world))) }
    }

    /// Shortcut for `add_id((flecs::ChildOf::ID, entity))`.
    ///
    /// # Arguments
    ///
    /// * `parent`: The parent entity to establish the relationship with.
    #[inline(always)]
    pub fn child_of(self, parent: impl IntoEntity) -> Self {
        // SAFETY: ChildOf is a builtin tag relationship (PairIsTag), so the pair adds no component data.
        unsafe { self.add_id_unchecked((ECS_CHILD_OF, parent.into_entity(self.world))) }
    }

    /// Shortcut for `add_id((flecs::DependsOn::ID, entity))`.
    ///
    /// # Arguments
    ///
    /// * `second`: The second element of the pair.
    pub fn depends_on(self, second: impl IntoEntity) -> Self {
        // SAFETY: DependsOn is a builtin tag relationship (PairIsTag), so the pair adds no component data.
        unsafe { self.add_id_unchecked((ECS_DEPENDS_ON, second.into_entity(self.world))) }
    }

    /// Shortcut for `add_id((flecs::Dependency::ID, entity))`for Enums.
    ///
    /// # Type Parameters
    ///
    /// * `T`: The enum type.
    ///
    /// # Arguments
    ///
    /// * `enum_value`: The enum constant.
    pub fn depends_on_enum<T: ComponentId + ComponentType<Enum> + EnumComponentInfo>(
        self,
        enum_value: T,
    ) -> Self {
        let world = self.world;
        self.depends_on(enum_value.id_variant(world))
    }

    /// Shortcut for `add_id((flecs::SlotOf::ID, entity))`.
    ///
    /// # Arguments
    ///
    /// * `second`: The second element of the pair.
    pub fn slot_of(self, second: impl IntoEntity) -> Self {
        // SAFETY: SlotOf is a builtin tag relationship (PairIsTag), so the pair adds no component data.
        unsafe { self.add_id_unchecked((ECS_SLOT_OF, second.into_entity(self.world))) }
    }

    /// Shortcut for `add_id((flecs::SlotOf::ID, target(ChildOf)))`.
    pub fn slot(self) -> Self {
        ecs_assert!(
            self.target(flecs::ChildOf::ID, 0).is_some(),
            FlecsErrorCode::InvalidParameter,
            "add ChildOf pair before using slot()"
        );
        let id = self.target(ECS_CHILD_OF, 0);
        self.slot_of(id.expect("ChildOf pair not found"))
    }

    /// Mark id for auto-overriding.
    ///
    /// When an entity inherits from a base entity (using the `IsA` relationship)
    /// any ids marked for auto-overriding on the base will be overridden
    /// automatically by the entity.
    ///
    /// # Arguments
    ///
    /// * `id`: The id to mark for overriding.
    pub fn auto_override(self, id: impl IntoId) -> Self {
        // SAFETY: AUTO_OVERRIDE only marks the id for overriding; data is copied from the base on instantiation.
        unsafe { self.add_id_unchecked(ECS_AUTO_OVERRIDE | id.into_id(self.world)) }
    }

    /// Sets a component mark override for the entity and sets the component data.
    pub fn set_auto_override<T: ComponentId + DataComponent + ComponentType<Struct>>(
        self,
        component: T,
    ) -> Self {
        self.auto_override(T::id()).set(component)
    }

    /// Sets a pair, mark component for auto-overriding.
    pub fn set_pair_override<First, Second>(
        self,
        data: <(First, Second) as ComponentOrPairId>::CastType,
    ) -> Self
    where
        First: ComponentId,
        Second: ComponentId,
        (First, Second): ComponentOrPairId,
        <(First, Second) as ComponentOrPairId>::CastType: DataComponent,
    {
        let id_pair = <(First, Second) as ComponentOrPairId>::get_id(self.world);
        self.auto_override(id_pair).set_id(data, id_pair)
    }

    /// Sets a pair, mark component for auto-overriding.
    ///
    /// # Safety
    ///
    /// Caller must ensure that `First` and `second` pair id data type is the one provided.
    pub unsafe fn set_auto_override_first<First>(
        self,
        first: First,
        second: impl IntoEntity,
    ) -> Self
    where
        First: ComponentId + ComponentType<Struct> + DataComponent,
    {
        let second_id = *second.into_entity(self.world);
        let first_id = First::entity_id(self.world);
        let pair_id = ecs_pair(first_id, second_id);
        self.auto_override(pair_id).set_id(first, pair_id)
    }

    /// Sets a pair, mark component for auto-overriding.
    ///
    /// # Safety
    ///
    /// Caller must ensure that `first` and `Second` pair id data type is the one provided.
    pub unsafe fn set_auto_override_second<Second>(
        self,
        second: Second,
        first: impl IntoEntity,
    ) -> Self
    where
        Second: ComponentId + ComponentType<Struct> + DataComponent,
    {
        let first_id = first.into_entity(self.world);
        let second_id = Second::entity_id(self.world);
        let pair_id = ecs_pair(*first_id, second_id);
        self.auto_override(pair_id).set_id(second, pair_id)
    }

    /// Sets a component of type `T` on the entity.
    ///
    /// # Arguments
    ///
    /// * `component` - The component to set on the entity.
    pub fn set<T: ComponentId>(self, component: T) -> Self {
        let id = T::entity_id(self.world);
        if T::IS_TAG {
            // SAFETY: T is a tag (zero-sized), so no component data needs initialization.
            unsafe { self.add_id_unchecked(id) };
        } else {
            set_helper(
                self.world.world_ptr_mut(),
                *self.id,
                component,
                T::entity_id(self.world),
            );
        }
        self
    }

    /// Sets the data of the specified id. Can be a pair or Component.
    ///
    /// # Safety
    ///
    /// Caller must ensure that `data` is a valid data for the id.
    ///
    /// ```no_run
    /// use flecs_ecs::prelude::*;
    /// use flecs_ecs::experimental::prelude::*;
    ///
    /// #[derive(Component)]
    /// struct InWorld;
    ///
    /// #[derive(Component, Debug)]
    /// struct Position {
    ///     x: f32,
    ///     y: f32,
    /// }
    ///
    /// let world = World::new();
    ///
    /// let position = world.component::<Position>();
    /// let in_world = world.component::<InWorld>();
    ///
    /// let entity = world.entity();
    ///
    /// // using a tuple indicates a relationship. It doesn't have to be a relationship.
    /// entity.set_id(Position { x: 10.0, y: 20.0 }, (in_world, position));
    /// // no relationship
    /// entity.set_id(Position { x: 1.0, y: 2.0 }, position);
    ///
    /// {
    ///     let _pos = entity.get_ref::<&(InWorld, Position)>().unwrap();
    ///     // ...
    /// }
    /// ```
    ///
    /// # See also
    ///
    /// * [`EntityView::add`]
    /// * [`EntityView::set`]
    /// * [`EntityView::set_pair`]
    pub fn set_id<T>(self, data: T, id: impl IntoId) -> Self
    where
        T: ComponentId + DataComponent,
    {
        let world = self.world.world_ptr_mut();
        let id = *id.into_id(self.world);
        let data_id = T::entity_id(self.world);
        // SAFETY: the world pointer is valid for 'a; ecs_get_typeid accepts any id value.
        let id_data_id = unsafe { sys::ecs_get_typeid(world, id) };

        if data_id != id_data_id {
            panic!(
                "Data type does not match id type. For pairs this is the first element occurrence that is not a zero-sized type (ZST)."
            );
        }

        set_helper(world, *self.id, data, id);
        self
    }

    /// Set a pair for an entity.
    /// This operation sets the pair value, and uses the first non tag / ZST as type.
    /// If the data is an flecs enum (Repr(C)), it will use the enum variant id.
    ///
    /// If the entity did not yet have the pair, it will be added, otherwise overridden.
    ///
    /// ```no_run
    /// use flecs_ecs::prelude::*;
    /// use flecs_ecs::experimental::prelude::*;
    ///
    /// #[derive(Component)]
    /// struct InWorld;
    ///
    /// #[derive(Component, Debug)]
    /// struct Position {
    ///     x: f32,
    ///     y: f32,
    /// }
    ///
    /// let world = World::new();
    ///
    /// let entity = world.entity();
    ///
    /// entity.set_pair::<InWorld, _>(Position { x: 1.0, y: 2.0 });
    ///
    /// {
    ///     let _pos = entity.get_ref::<&(InWorld, Position)>().unwrap();
    ///     // ...
    /// }
    /// ```
    pub fn set_pair<First, Second>(
        self,
        data: <(First, Second) as ComponentOrPairId>::CastType,
    ) -> Self
    where
        First: ComponentId,
        Second: ComponentId,
        (First, Second): ComponentOrPairId,
    {
        const {
            assert!(
                !<(First, Second) as ComponentOrPairId>::IS_TAGS,
                "setting tag relationships is not possible with `set_pair`. use `add::<(Tag1, Tag2)()` instead."
            );
        };

        let pair_id = ecs_pair(First::entity_id(self.world), Second::entity_id(self.world));

        ecs_assert!(
            // SAFETY: the world pointer is valid for 'a; ecs_get_typeid accepts any id value.
            unsafe { sys::ecs_get_typeid(self.world.ptr_mut(), pair_id) } != 0,
            FlecsErrorCode::InvalidOperation,
            "Pair is not a (data) component. Possible cause: PairIsTag trait"
        );

        set_helper(self.world.world_ptr_mut(), *self.id, data, pair_id);
        self
    }

    /// Set a pair for an entity using the first element type and a second component ID.
    pub fn set_first<First>(self, first: First, second: impl IntoEntity) -> Self
    where
        First: ComponentId + DataComponent,
    {
        let world = self.world;
        let world_ptr = world.world_ptr_mut();
        let first_id = First::entity_id(self.world);
        let second_id = *second.into_entity(world);
        let pair_id = ecs_pair(first_id, second_id);
        // SAFETY: the world pointer is valid for 'a; ecs_get_typeid accepts any id value.
        let data_id = unsafe { sys::ecs_get_typeid(world_ptr, pair_id) };

        if data_id != first_id {
            panic!(
                "First type does not match id data type. For pairs this is the first element occurrence that is not a zero-sized type (ZST)."
            );
        }

        set_helper(world_ptr, *self.id, first, pair_id);
        self
    }

    /// Set a pair for an entity using the second element type and a first id.
    ///
    /// # Panics
    ///
    /// Caller must ensure that first is a zero-sized type (ZST) or entity and not a pair.
    pub fn set_second<Second>(self, first: impl IntoEntity, second: Second) -> Self
    where
        Second: ComponentId + ComponentType<Struct> + DataComponent,
    {
        let world = self.world.world_ptr_mut();
        let first_id = *first.into_entity(self.world);
        let second_id = Second::entity_id(self.world);
        let pair_id = ecs_pair(first_id, second_id);
        // NOTE: we could this safety check optional
        // SAFETY: the world pointer is valid for 'a; ecs_get_typeid accepts any id value.
        let data_id = unsafe { sys::ecs_get_typeid(world, pair_id) };

        if data_id != second_id {
            panic!(
                "Second type does not match id data type. For pairs this is the first element occurrence that is not a zero-sized type (ZST)."
            );
        }

        set_helper(world, *self.id, second, pair_id);
        self
    }

    /// Set a pair for an entity.
    /// This operation sets the pair value, and uses First as type. If the
    /// entity did not yet have the pair, it will be added.
    ///
    /// # Type Parameters
    ///
    /// * `First`: The first element of the pair.
    /// * `Second`: The second element of the pair.
    ///
    /// # Arguments
    ///
    /// * `constant`: The enum constant.
    /// * `value`: The value to set.
    pub fn set_pair_enum<First, Second>(self, enum_variant: Second, first: First) -> Self
    where
        First: ComponentId + ComponentType<Struct> + DataComponent,
        Second: ComponentId + ComponentType<Enum> + EnumComponentInfo,
    {
        set_helper(
            self.world.world_ptr_mut(),
            *self.id,
            first,
            ecs_pair(
                First::entity_id(self.world),
                **enum_variant.id_variant(self.world),
            ),
        );
        self
    }

    /// Sets a pointer to a component of an entity with a given component ID and size.
    ///
    /// # Safety
    /// Caller must ensure that `ptr` points to data that can be accessed as the type associated with `id`
    ///
    /// # Arguments
    ///
    /// * `self` - A mutable reference to the entity.
    /// * `id` - The component or pair id to set the pointer for.
    /// * `size` - The size of the component.
    /// * `ptr` - A pointer to the component.
    pub unsafe fn set_ptr_with_size(
        self,
        id: impl IntoId,
        size: usize,
        ptr: *const c_void,
    ) -> Self {
        let id = *id.into_id(self.world);
        // SAFETY: the world pointer is valid for 'a; the caller guarantees `ptr` points to `size` bytes valid as the type of `id`.
        unsafe {
            sys::ecs_set_id(self.world.world_ptr_mut(), *self.id, id, size, ptr);
            self
        }
    }

    /// Sets a pointer to a component of an entity with a given component or pair id.
    ///
    /// # Safety
    /// Caller must ensure that `id` refers to a registered, sized (non-tag) component or pair
    /// and that `ptr` points to data that can be accessed as the type associated with `id`
    ///
    /// # Panics
    ///
    /// Panics if `id` does not have an associated type (is not a component or data pair).
    ///
    /// # Arguments
    ///
    /// * `self` - A mutable reference to the entity.
    /// * `id` - The component or pair id to set the pointer for.
    /// * `ptr` - A pointer to the component.
    pub unsafe fn set_ptr(self, id: impl IntoId, ptr: *const c_void) -> Self {
        // SAFETY: the world pointer is valid for 'a; `type_info` is checked non-null before reading, and the caller guarantees `ptr` matches the type of `id`.
        unsafe {
            let id = id.into_id(self.world);
            let type_info = sys::ecs_get_type_info(self.world.world_ptr_mut(), *id);

            assert!(!type_info.is_null(), "invalid component id: {id:?}");

            self.set_ptr_with_size(*id, (*type_info).size as usize, ptr)
        }
    }

    /// assign a component for an entity.
    /// This operation sets the component value. If the entity did not yet have
    /// the component the operation will panic.
    pub fn assign<T: ComponentId + DataComponent>(self, value: T) -> Self {
        assign_helper(
            self.world.world_ptr_mut(),
            *self.id,
            value,
            T::entity_id(self.world),
        );
        self
    }

    /// assign a component for an entity.
    /// This operation sets the component value. If the entity did not yet have
    /// the component the operation will panic.
    pub fn assign_id<T: ComponentId + DataComponent>(self, value: T, id: impl IntoId) -> Self {
        let id = *id.into_id(self.world);
        assign_helper(self.world.world_ptr_mut(), *self.id, value, id);
        self
    }

    /// assign a component for an entity.
    /// This operation sets the component value. If the entity did not yet have
    /// the component the operation will panic.
    pub fn assign_pair<First, Second>(
        self,
        value: <(First, Second) as ComponentOrPairId>::CastType,
    ) -> Self
    where
        First: ComponentId,
        Second: ComponentId,
        (First, Second): ComponentOrPairId,
    {
        const {
            assert!(
                !<(First, Second) as ComponentOrPairId>::IS_TAGS,
                "setting tag relationships is not possible with `set_pair`. use `add::<(Tag1, Tag2)()` instead."
            );
        };

        let pair_id = ecs_pair(First::entity_id(self.world), Second::entity_id(self.world));

        ecs_assert!(
            // SAFETY: the world pointer is valid for 'a; ecs_get_typeid accepts any id value.
            unsafe { sys::ecs_get_typeid(self.world.ptr_mut(), pair_id) } != 0,
            FlecsErrorCode::InvalidOperation,
            "Pair is not a (data) component. Possible cause: PairIsTag trait"
        );

        assign_helper(self.world.world_ptr_mut(), *self.id, value, pair_id);
        self
    }

    /// assign a component for an entity.
    /// This operation sets the component value. If the entity did not yet have
    /// the component the operation will panic.
    pub fn assign_first<First>(self, first: First, second: impl IntoEntity) -> Self
    where
        First: ComponentId + DataComponent,
    {
        let world_ptr = self.world.world_ptr_mut();
        let first_id = First::entity_id(self.world);
        let second_id = *second.into_entity(self.world);
        let pair_id = ecs_pair(first_id, second_id);
        // SAFETY: the world pointer is valid for 'a; ecs_get_typeid accepts any id value.
        let data_id = unsafe { sys::ecs_get_typeid(world_ptr, pair_id) };

        if data_id != first_id {
            panic!(
                "First type does not match id data type. For pairs this is the first element occurrence that is not a zero-sized type (ZST)."
            );
        }

        assign_helper(world_ptr, *self.id, first, pair_id);
        self
    }

    /// assign a component for an entity.
    /// This operation sets the component value. If the entity did not yet have
    /// the component the operation will panic.
    pub fn assign_second<Second>(self, first: impl IntoEntity, second: Second) -> Self
    where
        Second: ComponentId + DataComponent,
    {
        let world = self.world.world_ptr_mut();
        let first_id = *first.into_entity(self.world);
        let second_id = Second::entity_id(self.world);
        let pair_id = ecs_pair(first_id, second_id);
        // NOTE: we could this safety check optional
        // SAFETY: the world pointer is valid for 'a; ecs_get_typeid accepts any id value.
        let data_id = unsafe { sys::ecs_get_typeid(world, pair_id) };

        if data_id != second_id {
            panic!(
                "Second type does not match id data type. For pairs this is the first element occurrence that is not a zero-sized type (ZST)."
            );
        }

        assign_helper(world, *self.id, second, pair_id);
        self
    }

    /// assign a component for an entity.
    /// This operation sets the component value. If the entity did not yet have
    /// the component the operation will panic.
    pub fn assign_pair_enum<First, Second>(self, enum_variant: Second, first: First) -> Self
    where
        First: ComponentId + ComponentType<Struct> + DataComponent,
        Second: ComponentId + ComponentType<Enum> + EnumComponentInfo,
    {
        assign_helper(
            self.world.world_ptr_mut(),
            *self.id,
            first,
            ecs_pair(
                First::entity_id(self.world),
                **enum_variant.id_variant(self.world),
            ),
        );
        self
    }

    fn try_assign_helper<T: ComponentId>(self, value: T, id: u64) -> bool {
        let world_ptr = self.world.world_ptr_mut();
        // SAFETY: the world pointer is valid for 'a and `ecs_owns_id` accepts any id value.
        if !unsafe { sys::ecs_owns_id(world_ptr, *self.id, id) } {
            return false;
        }
        assign_helper(world_ptr, *self.id, value, id);
        true
    }

    /// Non-panicking counterpart of [`EntityView::assign()`].
    /// Sets the component value and returns `true` if the entity already has the component,
    /// otherwise does nothing and returns `false`.
    pub fn try_assign<T: ComponentId + DataComponent>(self, value: T) -> bool {
        let id = T::entity_id(self.world);
        self.try_assign_helper(value, id)
    }

    /// Non-panicking counterpart of [`EntityView::assign_id()`].
    /// Sets the component value and returns `true` if the entity already has the component,
    /// otherwise does nothing and returns `false`.
    pub fn try_assign_id<T: ComponentId + DataComponent>(self, value: T, id: impl IntoId) -> bool {
        let id = *id.into_id(self.world);
        self.try_assign_helper(value, id)
    }

    /// Non-panicking counterpart of [`EntityView::assign_pair()`].
    /// Sets the pair value and returns `true` if the entity already has the pair,
    /// otherwise does nothing and returns `false`.
    pub fn try_assign_pair<First, Second>(
        self,
        value: <(First, Second) as ComponentOrPairId>::CastType,
    ) -> bool
    where
        First: ComponentId,
        Second: ComponentId,
        (First, Second): ComponentOrPairId,
    {
        const {
            assert!(
                !<(First, Second) as ComponentOrPairId>::IS_TAGS,
                "assigning tag relationships is not possible with `try_assign_pair`. use `add::<(Tag1, Tag2)>()` instead."
            );
        };

        let pair_id = ecs_pair(First::entity_id(self.world), Second::entity_id(self.world));
        self.try_assign_helper(value, pair_id)
    }

    /// Non-panicking counterpart of [`EntityView::assign_first()`].
    /// Sets the pair value and returns `true` if the entity already has the pair,
    /// otherwise does nothing and returns `false`.
    ///
    /// # Panics
    ///
    /// Panics if `First` does not match the data type of the pair.
    pub fn try_assign_first<First>(self, first: First, second: impl IntoEntity) -> bool
    where
        First: ComponentId + DataComponent,
    {
        let world_ptr = self.world.world_ptr_mut();
        let first_id = First::entity_id(self.world);
        let second_id = *second.into_entity(self.world);
        let pair_id = ecs_pair(first_id, second_id);
        // SAFETY: the world pointer is valid for 'a and `ecs_get_typeid` accepts any id value.
        let data_id = unsafe { sys::ecs_get_typeid(world_ptr, pair_id) };

        if data_id != first_id {
            panic!(
                "First type does not match id data type. For pairs this is the first element occurrence that is not a zero-sized type (ZST)."
            );
        }

        self.try_assign_helper(first, pair_id)
    }

    /// Non-panicking counterpart of [`EntityView::assign_second()`].
    /// Sets the pair value and returns `true` if the entity already has the pair,
    /// otherwise does nothing and returns `false`.
    ///
    /// # Panics
    ///
    /// Panics if `Second` does not match the data type of the pair.
    pub fn try_assign_second<Second>(self, first: impl IntoEntity, second: Second) -> bool
    where
        Second: ComponentId + DataComponent,
    {
        let world_ptr = self.world.world_ptr_mut();
        let first_id = *first.into_entity(self.world);
        let second_id = Second::entity_id(self.world);
        let pair_id = ecs_pair(first_id, second_id);
        // SAFETY: the world pointer is valid for 'a and `ecs_get_typeid` accepts any id value.
        let data_id = unsafe { sys::ecs_get_typeid(world_ptr, pair_id) };

        if data_id != second_id {
            panic!(
                "Second type does not match id data type. For pairs this is the first element occurrence that is not a zero-sized type (ZST)."
            );
        }

        self.try_assign_helper(second, pair_id)
    }

    /// Sets the name of the entity.
    ///
    /// # Arguments
    ///
    /// * `name` - A string slice that holds the name to be set.
    pub fn set_name(self, name: &str) -> Self {
        // Shared-register write hook (spec §3.6, §7.1): defer behind a live guard.
        ensure_write_episode(&self.world);
        let name = compact_str::format_compact!("{}\0", name);

        // SAFETY: the world pointer is valid for 'a and `name` is a NUL-terminated buffer that outlives the call.
        unsafe {
            sys::ecs_set_name(
                self.world.world_ptr_mut(),
                *self.id,
                name.as_ptr() as *const _,
            );
        }
        self
    }

    /// Removes the name of the entity.
    pub fn remove_name(self) -> Self {
        // Shared-register write hook (spec §3.6, §7.1): defer behind a live guard.
        ensure_write_episode(&self.world);
        // SAFETY: the world pointer is valid for 'a; a null name pointer clears the entity's name.
        unsafe {
            sys::ecs_set_name(self.world.world_ptr_mut(), *self.id, core::ptr::null());
        }
        self
    }

    /// Sets the alias name of the entity.
    ///
    /// # Arguments
    ///
    /// * `name` - A string slice that holds the alias name to be set.
    pub fn set_alias(self, name: &str) -> Self {
        // Shared-register write hook (spec §3.6, §7.1): defer behind a live guard.
        ensure_write_episode(&self.world);
        let name = compact_str::format_compact!("{}\0", name);

        // SAFETY: the world pointer is valid for 'a and `name` is a NUL-terminated buffer that outlives the call.
        unsafe {
            sys::ecs_set_alias(
                self.world.world_ptr_mut(),
                *self.id,
                name.as_ptr() as *const _,
            );
        }
        self
    }

    /// Enables itself (the entity).
    ///
    /// Enabled entities are matched with systems and can be searched with queries.
    pub fn enable_self(self) -> Self {
        // Shared-register write hook (spec §3.6, §7.1): defer behind a live guard.
        ensure_write_episode(&self.world);
        // SAFETY: the world pointer is valid for 'a; ecs_enable accepts any entity value.
        unsafe { sys::ecs_enable(self.world.world_ptr_mut(), *self.id, true) }
        self
    }
    /// Enables an ID which represents a component or pair.
    ///
    /// This sets the enabled bit for this component. If this is the first time the component is
    /// enabled or disabled, the bitset is added.
    ///
    /// # Arguments
    ///
    /// - `component_id`: The ID to enable.
    /// - `toggle`: True to enable, false to disable (default = true).
    pub fn enable(self, id: impl IntoId) -> Self {
        // Shared-register write hook (spec §3.6, §7.1): defer behind a live guard.
        ensure_write_episode(&self.world);
        // SAFETY: the world pointer is valid for 'a; ecs_enable_id accepts any entity/id values.
        unsafe {
            sys::ecs_enable_id(
                self.world.world_ptr_mut(),
                *self.id,
                *id.into_id(self.world),
                true,
            );
        }
        self
    }

    /// Disables self (entity).
    ///
    /// Disabled entities are not matched with systems and cannot be searched with queries,
    /// unless explicitly specified in the query expression.
    pub fn disable_self(self) -> Self {
        // Shared-register write hook (spec §3.6, §7.1): defer behind a live guard.
        ensure_write_episode(&self.world);
        // SAFETY: the world pointer is valid for 'a; ecs_enable accepts any entity value.
        unsafe { sys::ecs_enable(self.world.world_ptr_mut(), *self.id, false) }
        self
    }

    /// Disables an ID which represents a component or pair.
    ///
    /// This sets the enabled bit for this ID. If this is the first time the ID is
    /// enabled or disabled, the bitset is added.
    ///
    /// # Arguments
    ///
    /// - `component_id`: The ID to disable.
    pub fn disable(self, id: impl IntoId) -> Self {
        // Shared-register write hook (spec §3.6, §7.1): defer behind a live guard.
        ensure_write_episode(&self.world);
        // SAFETY: the world pointer is valid for 'a; ecs_enable_id accepts any entity/id values.
        unsafe {
            sys::ecs_enable_id(
                self.world.world_ptr_mut(),
                *self.id,
                *id.into_id(self.world),
                false,
            );
        }
        self
    }

    /// Entities created in the function will have the current entity.
    /// This operation is thread safe.
    ///
    /// # Arguments
    ///
    /// - `func`: The function to call.
    pub fn with(self, func: impl FnOnce()) -> Self {
        let world = self.world.world_ptr_mut();
        // SAFETY: the world pointer is valid for 'a; the guard restores the previous
        // with id even if `func` unwinds.
        let _guard = RestoreWithGuard {
            world,
            prev: unsafe { sys::ecs_set_with(world, *self.id) },
        };
        func();
        self
    }

    /// Entities created in the function will have a pair consisting of a specified ID and the current entity.
    /// This operation is thread safe.
    ///
    /// # Arguments
    ///
    /// - `first`: The first element of the pair.
    /// - `func`: The function to call.///
    pub fn with_first(self, first: impl IntoEntity, func: impl FnOnce()) -> Self {
        let world = self.world.world_ptr_mut();
        let pair = ecs_pair(*first.into_entity(self.world), *self.id);
        // SAFETY: the world pointer is valid for 'a; the guard restores the previous
        // with id even if `func` unwinds.
        let _guard = RestoreWithGuard {
            world,
            prev: unsafe { sys::ecs_set_with(world, pair) },
        };
        func();
        self
    }

    /// Entities created in the function will have a pair consisting of the current entity and a specified ID.
    /// This operation is thread safe.
    ///
    /// # Arguments
    ///
    /// - `second`: The second element of the pair.
    /// - `func`: The function to call.
    pub fn with_second(self, second: impl IntoEntity, func: impl FnOnce()) -> Self {
        let world = self.world.world_ptr_mut();
        let pair = ecs_pair(*self.id, *second.into_entity(self.world));
        // SAFETY: the world pointer is valid for 'a; the guard restores the previous
        // with id even if `func` unwinds.
        let _guard = RestoreWithGuard {
            world,
            prev: unsafe { sys::ecs_set_with(world, pair) },
        };
        func();
        self
    }

    /// The function will be ran with the scope set to the current entity.
    ///
    /// # Arguments
    ///
    /// - `func`: The function to call.
    pub fn run_in_scope(self, func: impl FnOnce()) -> Self {
        let world = self.world.world_ptr_mut();
        // SAFETY: the world pointer is valid for 'a; the guard restores the previous
        // scope even if `func` unwinds.
        let _guard = RestoreScopeGuard {
            world,
            prev: unsafe { sys::ecs_set_scope(world, *self.id) },
        };
        func();
        self
    }

    /// Calls the provided function with a world scoped to entity
    pub fn scope(self, f: impl FnMut(&World)) -> Self {
        let world = &*self.world;
        world.scope(self.id, f);
        self
    }

    /// Signal that component or pair was modified.
    ///
    /// # Arguments
    ///
    /// * `comp` - The component that was modified.
    ///
    /// # See also
    ///
    /// * [`EntityView::modified()`]
    /// * [`World::modified()`]
    pub fn modified<T: IntoId>(self, id: T) {
        const {
            if <T as IntoId>::IS_TYPE_TAG {
                panic!("Cannot modify tag component");
            }
        }

        // Shared-register write hook (spec §3.6, §7.1): defer behind a live guard.
        ensure_write_episode(&self.world);
        // SAFETY: the world pointer is valid for 'a; T was const-checked to not be a tag.
        unsafe {
            sys::ecs_modified_id(
                self.world.world_ptr_mut(),
                *self.id,
                *id.into_id(self.world),
            );
        }
    }


    /// Clear an entity.
    ///
    /// This operation removes all components from an entity without recycling
    /// the entity id.
    pub fn clear(&self) {
        // Shared-register write hook (spec §3.6, §7.1): defer behind a live guard.
        ensure_write_episode(&self.world);
        // SAFETY: the world pointer is valid for 'a; ecs_clear accepts any entity value.
        unsafe { sys::ecs_clear(self.world.world_ptr_mut(), *self.id) }
    }

    /// Delete an entity.
    ///
    /// Entities have to be deleted explicitly, and are not deleted when the
    /// entity object goes out of scope.
    pub fn destruct(self) {
        // Shared-register write hook (spec §3.6, §7.1): defer behind a live guard.
        ensure_write_episode(&self.world);
        // SAFETY: the world pointer is valid for 'a; ecs_delete accepts any entity value.
        unsafe { sys::ecs_delete(self.world.world_ptr_mut(), *self.id) }
    }

    /// Set child order.
    /// Changes the order of children as returned by [`EntityView::each_child()`].
    /// Only applicable to entities with the [`flecs::OrderedChildren`] trait.
    ///
    /// # Example
    ///
    /// ```rust
    /// use flecs_ecs::prelude::*;
    ///
    /// #[derive(Component, Default)]
    /// struct Position {
    ///     x: f32,
    ///     y: f32,
    /// }
    ///
    /// let world = World::new();
    /// let parent = world.entity().add(flecs::OrderedChildren);
    /// let child_a = world.entity().child_of(parent).add(Position::id());
    /// let child_b = world.entity().child_of(parent).add(Position::id());
    /// let child_c = world.entity().child_of(parent).add(Position::id());
    ///
    /// let mut vec: Vec<Entity> = vec![];
    /// parent.each_child(|e| {
    ///     vec.push(*e);
    /// });
    ///
    /// assert!(vec[0] == child_a);
    /// assert!(vec[1] == child_b);
    /// assert!(vec[2] == child_c);
    ///
    /// let children = [*child_c, *child_a, *child_b];
    /// parent.set_child_order(&children);
    ///
    /// vec.clear();
    ///
    /// parent.each_child(|e| {
    ///     vec.push(*e);
    /// });
    ///
    /// assert!(vec[0] == child_c);
    /// assert!(vec[1] == child_a);
    /// assert!(vec[2] == child_b);
    /// ```
    pub fn set_child_order(self, children: &[Entity]) -> Self {
        // Shared-register write hook (spec §3.6, §7.1): defer behind a live guard.
        ensure_write_episode(&self.world);
        let world_ptr = self.world.world_ptr_mut();
        let child_count = children.len() as i32;
        let children_ptr = if child_count > 0 {
            children.as_ptr() as *const sys::ecs_entity_t
        } else {
            core::ptr::null()
        };

        // SAFETY: the world pointer is valid for 'a; `children_ptr` points to `child_count` ids (Entity is repr(transparent) over u64) borrowed for this call.
        unsafe {
            sys::ecs_set_child_order(world_ptr, *self.id, children_ptr, child_count);
        }
        self
    }
}

struct RestoreWithGuard {
    world: *mut sys::ecs_world_t,
    prev: sys::ecs_id_t,
}

impl Drop for RestoreWithGuard {
    fn drop(&mut self) {
        // SAFETY: `world` is a live world pointer captured from the `WorldRef` of the
        // function that created this guard, which is still borrowed for 'a.
        unsafe {
            sys::ecs_set_with(self.world, self.prev);
        }
    }
}

struct RestoreScopeGuard {
    world: *mut sys::ecs_world_t,
    prev: sys::ecs_entity_t,
}

impl Drop for RestoreScopeGuard {
    fn drop(&mut self) {
        // SAFETY: `world` is a live world pointer captured from the `WorldRef` of the
        // function that created this guard, which is still borrowed for 'a.
        unsafe {
            sys::ecs_set_scope(self.world, self.prev);
        }
    }
}
