use core::{
    ffi::CStr,
    sync::atomic::{AtomicU32, Ordering},
};

use crate::core::*;
use crate::sys;

use hashbrown::hash_map::Entry;

/// A marker trait to indicate that a component represents a trait
#[cfg(feature = "flecs_query_rust_traits")]
pub trait RustTrait {}

/// Indicates that the type is a tag component. A tag component is a component that does not have any data. Is a zero-sized type.
pub trait TagComponent {}

#[doc(hidden)]
impl<T, U> TagComponent for (T, U)
where
    T: TagComponent,
    U: TagComponent,
{
}

#[diagnostic::on_unimplemented(
    message = "the size of type `{Self}` should not be zero, should not be a tag.",
    label = "Supports only non-empty components"
)]
/// Indicates that the type is a non-tag component. A non-tag component contains data, is not a zero-sized type.
pub trait DataComponent {}

#[doc(hidden)]
impl<T> DataComponent for &T where T: DataComponent {}
#[doc(hidden)]
impl<T> DataComponent for &mut T where T: DataComponent {}
#[doc(hidden)]
impl<T> DataComponent for Option<&T> where T: DataComponent {}
#[doc(hidden)]
impl<T> DataComponent for Option<&mut T> where T: DataComponent {}
#[doc(hidden)]
impl<T, U> DataComponent for (T, U)
where
    T: ComponentId,
    U: ComponentId,
    (T, U): ComponentOrPairId,
    <(T, U) as ComponentOrPairId>::CastType: DataComponent,
    registration_types::ConditionalTypePairSelector<
        <<(T, U) as ComponentOrPairId>::First as ComponentInfo>::TagType,
        T,
        U,
    >: FlecsPairType,
{
}

/// Indicates what component types are supported by the ECS.
pub trait ECSComponentType {}

impl ECSComponentType for Enum {}
impl ECSComponentType for Struct {}

/// Indicates what the type of the component is.
pub trait ComponentType<T: ECSComponentType> {}

#[doc(hidden)]
impl<T> ComponentType<Enum> for &T where T: ComponentType<Enum> {}
#[doc(hidden)]
impl<T> ComponentType<Enum> for &mut T where T: ComponentType<Enum> {}

fn try_register_component<'a, const MANUAL_REGISTRATION_CHECK: bool, T>(
    world: impl WorldProvider<'a>,
) -> sys::ecs_entity_t
where
    T: ComponentId,
{
    use crate::core::component_registration::registration;
    // when a component is registered through [`Component`], manual registration check is set to false
    // thus for `try_register_component` we set the flag that it uses const component_registration to true
    if MANUAL_REGISTRATION_CHECK {
        registration::try_register_component::<false, T>(world)
    } else {
        registration::try_register_component::<true, T>(world)
    }
}

fn try_register_component_named<'a, const MANUAL_REGISTRATION_CHECK: bool, T>(
    world: impl WorldProvider<'a>,
    name: &str,
) -> sys::ecs_entity_t
where
    T: ComponentId,
{
    use crate::core::component_registration::registration;
    // when a component is registered through [`Component`], manual registration check is set to false
    // thus for `try_register_component` we set the flag that it uses const component_registration to true
    if MANUAL_REGISTRATION_CHECK {
        registration::try_register_component_named::<false, T>(world, name)
    } else {
        registration::try_register_component_named::<true, T>(world, name)
    }
}

#[doc(hidden)]
pub trait InternalComponentHooks {
    fn internal_pre_registration_name() -> Option<&'static str> {
        None
    }

    fn internal_on_component_registration(_world: WorldRef, _component_id: Entity) {}
}

/// A trait that holds a fn `on_component_registration`. This automatically gets implemented
/// on types with the `Component` derive macro, unless the attribute `flecs(on_registration)` is used.
/// When it is used, the user can specify what should happen when a component is registered.
///
/// # See also
/// * [`component_macro`](crate::component_macro) documentation for examples and more info
pub trait OnComponentRegistration: InternalComponentHooks {
    /// This function is called when a component is first registered.
    /// Implementing it allows for custom injectable code to be executed on component registration
    fn on_component_registration(_world: WorldRef, _component_id: Entity) {}
}

/// Trait that manages component IDs across multiple worlds & binaries.
///
/// proc macro Component should be used to implement this trait automatically
///
/// ```
/// # use flecs_ecs::prelude::Component;
/// #[derive(Component)] //this will implement the trait for the type
/// struct Position {
///     vec: Vec<i32>,
/// }
/// ```
///
/// The `ComponentId` trait is designed to maintain component IDs for a Rust type
/// in a manner that is consistent across different worlds (or instances).
/// When a component is utilized, this trait will determine whether it has already been registered.
/// If it hasn't, it registers the component with the current world.
///
/// If the ID has been previously established, the trait ensures the world recognizes it.
/// If the world doesn't, this implies the component was registered by a different world.
/// In such a case, the component is registered with the present world using the pre-existing ID.
/// If the ID is already known, the trait takes care of the component registration and checks for consistency in the input.
#[diagnostic::on_unimplemented(
    message = "`{Self}` is not a flecs component.",
    label = "use `#[derive(Component)]` on `{Self}` to mark it as one."
)]
pub trait ComponentId: Sized + ComponentInfo + 'static + OnComponentRegistration {
    #[doc(hidden)]
    type UnderlyingType: ComponentId;
    #[doc(hidden)]
    type UnderlyingEnumType: ComponentId + EnumComponentInfo;
    #[doc(hidden)]
    type UnderlyingTypeOfEnum: ComponentId + Copy;

    /// attempts to register the component with the world. If it's already registered, it does nothing.
    #[doc(hidden)]
    #[inline(always)]
    fn __register_or_get_id<'a, const MANUAL_REGISTRATION_CHECK: bool>(
        world: impl WorldProvider<'a>,
    ) -> sys::ecs_entity_t {
        if !Self::IS_GENERIC {
            let index = Self::index() as usize;
            let world = world.world();
            let len = world.components_array().len();

            if len > index {
                let component_id = world.components_array()[index];
                if component_id == 0 || !world.is_alive(component_id) {
                    if MANUAL_REGISTRATION_CHECK {
                        #[cfg(feature = "flecs_manual_registration")]
                        {
                            ecs_assert!(
                                false,
                                FlecsErrorCode::InvalidOperation,
                                "Component {} is not registered with the world before usage",
                                Self::name()
                            );
                        }
                    }

                    // Register first: for enums this recurses to register the
                    // underlying type, which can grow (and reallocate) the
                    // components array. Re-borrow the array afterwards so no
                    // `&mut` to it is ever held across the recursive call.
                    let id = if let Some(name) = Self::internal_pre_registration_name() {
                        try_register_component_named::<MANUAL_REGISTRATION_CHECK, Self>(world, name)
                    } else {
                        try_register_component::<MANUAL_REGISTRATION_CHECK, Self>(world)
                    };

                    world.components_array()[index] = id;
                    #[cfg(feature = "flecs_meta")]
                    {
                        world
                            .components_map()
                            .insert(core::any::TypeId::of::<Self>(), id);
                    }

                    Self::internal_on_component_registration(world, Entity::new(id));

                    return id;
                }
                world.components_array()[index]
            } else {
                {
                    let components_array = world.components_array();
                    components_array.reserve(index + 1 - len);
                    let capacity = components_array.capacity();
                    unsafe {
                        core::ptr::write_bytes(
                            components_array.as_mut_ptr().add(len),
                            0,
                            capacity - len,
                        );
                        components_array.set_len(capacity);
                    }
                }

                let id = if let Some(name) = Self::internal_pre_registration_name() {
                    try_register_component_named::<MANUAL_REGISTRATION_CHECK, Self>(world, name)
                } else {
                    try_register_component::<MANUAL_REGISTRATION_CHECK, Self>(world)
                };

                world.components_array()[index] = id;
                #[cfg(feature = "flecs_meta")]
                {
                    world
                        .components_map()
                        .insert(core::any::TypeId::of::<Self>(), id);
                }

                Self::internal_on_component_registration(world, Entity::new(id));

                id
            }
        } else {
            let world = world.world();
            let components_map = world.components_map();

            match components_map.entry(core::any::TypeId::of::<Self>()) {
                Entry::Occupied(mut entry) => {
                    if !world.is_alive(*entry.get()) {
                        if MANUAL_REGISTRATION_CHECK {
                            #[cfg(feature = "flecs_manual_registration")]
                            {
                                ecs_assert!(
                                    false,
                                    FlecsErrorCode::InvalidOperation,
                                    "Component {} is not registered with the world before usage",
                                    Self::name()
                                );
                            }
                        }

                        let id = if let Some(name) = Self::internal_pre_registration_name() {
                            try_register_component_named::<MANUAL_REGISTRATION_CHECK, Self>(
                                world, name,
                            )
                        } else {
                            try_register_component::<MANUAL_REGISTRATION_CHECK, Self>(world)
                        };

                        let id = entry.insert(id);

                        Self::internal_on_component_registration(world, Entity::new(id));
                        id
                    } else {
                        *entry.into_mut()
                    }
                }
                Entry::Vacant(entry) => {
                    if MANUAL_REGISTRATION_CHECK {
                        #[cfg(feature = "flecs_manual_registration")]
                        {
                            ecs_assert!(
                                false,
                                FlecsErrorCode::InvalidOperation,
                                "Component {} is not registered with the world before usage",
                                Self::name()
                            );
                        }
                    }

                    let id = if let Some(name) = Self::internal_pre_registration_name() {
                        try_register_component_named::<MANUAL_REGISTRATION_CHECK, Self>(world, name)
                    } else {
                        try_register_component::<MANUAL_REGISTRATION_CHECK, Self>(world)
                    };

                    let id = entry.insert(id);

                    Self::internal_on_component_registration(world, Entity::new(*id));
                    *id
                }
            }
        }
    }

    /// attempts to register the component with name with the world. If it's already registered, it does nothing.
    #[inline(always)]
    #[doc(hidden)]
    fn __register_or_get_id_named<'a, const MANUAL_REGISTRATION_CHECK: bool>(
        world: impl WorldProvider<'a>,
        name: &str,
    ) -> sys::ecs_entity_t {
        if !Self::IS_GENERIC {
            let index = Self::index() as usize;
            let world = world.world();
            let len = world.components_array().len();

            if len > index {
                let component_id = world.components_array()[index];
                if component_id == 0 || !world.is_alive(component_id) {
                    if MANUAL_REGISTRATION_CHECK {
                        #[cfg(feature = "flecs_manual_registration")]
                        {
                            ecs_assert!(
                                false,
                                FlecsErrorCode::InvalidOperation,
                                "Component {} is not registered with the world before usage",
                                Self::name()
                            );
                        }
                    }

                    // Register first, then re-borrow: registration can recurse
                    // (enum underlying types) and reallocate the array, so no
                    // `&mut` to it may be held across the recursive call.
                    let id = try_register_component_named::<MANUAL_REGISTRATION_CHECK, Self>(
                        world, name,
                    );

                    world.components_array()[index] = id;

                    #[cfg(feature = "flecs_meta")]
                    {
                        world
                            .components_map()
                            .insert(core::any::TypeId::of::<Self>(), id);
                    }

                    Self::internal_on_component_registration(world, Entity::new(id));

                    return id;
                }
                world.components_array()[index]
            } else {
                {
                    let components_array = world.components_array();
                    components_array.reserve(index + 1 - len);
                    let capacity = components_array.capacity();
                    unsafe {
                        core::ptr::write_bytes(
                            components_array.as_mut_ptr().add(len),
                            0,
                            capacity - len,
                        );
                        components_array.set_len(capacity);
                    }
                }
                let id =
                    try_register_component_named::<MANUAL_REGISTRATION_CHECK, Self>(world, name);

                world.components_array()[index] = id;
                #[cfg(feature = "flecs_meta")]
                {
                    world
                        .components_map()
                        .insert(core::any::TypeId::of::<Self>(), id);
                }

                Self::internal_on_component_registration(world, Entity::new(id));

                id
            }
        } else {
            let world = world.world();
            let components_map = world.components_map();

            match components_map.entry(core::any::TypeId::of::<Self>()) {
                Entry::Occupied(mut entry) => {
                    if !world.is_alive(*entry.get()) {
                        if MANUAL_REGISTRATION_CHECK {
                            #[cfg(feature = "flecs_manual_registration")]
                            {
                                ecs_assert!(
                                    false,
                                    FlecsErrorCode::InvalidOperation,
                                    "Component {} is not registered with the world before usage",
                                    Self::name()
                                );
                            }
                        }

                        let id = try_register_component_named::<MANUAL_REGISTRATION_CHECK, Self>(
                            world, name,
                        );

                        let id = entry.insert(id);

                        Self::internal_on_component_registration(world, Entity::new(id));
                        id
                    } else {
                        *entry.into_mut()
                    }
                }
                Entry::Vacant(entry) => {
                    if MANUAL_REGISTRATION_CHECK {
                        #[cfg(feature = "flecs_manual_registration")]
                        {
                            ecs_assert!(
                                false,
                                FlecsErrorCode::InvalidOperation,
                                "Component {} is not registered with the world before usage",
                                Self::name()
                            );
                        }
                    }

                    let id = try_register_component_named::<MANUAL_REGISTRATION_CHECK, Self>(
                        world, name,
                    );

                    let id = entry.insert(id);

                    Self::internal_on_component_registration(world, Entity::new(*id));
                    *id
                }
            }
        }
    }

    /// checks if the component is registered with a world.
    #[inline(always)]
    fn is_registered_with_world<'a>(world: impl WorldProvider<'a>) -> bool {
        if !Self::IS_GENERIC {
            let index = Self::index();
            let world = world.world();
            let components_array = world.components_array();
            let len = components_array.len();

            if len > index as usize {
                components_array[index as usize] != 0
            } else {
                false
            }
        } else {
            let world = world.world();
            let components_map = world.components_map();
            components_map.contains_key(&core::any::TypeId::of::<Self>())
        }
    }

    /// utility function to returning an Id object which is used for fn's that take `IntoEntity` or `IntoId` variable
    #[inline(always)]
    fn id() -> utility::id::Id<Self> {
        utility::id::Id::<Self>::new()
    }

    /// returns the component id registered with a particular world. If the component is not registered, it will register it.
    /// # Note
    /// Each world has it's own unique id for the component.
    #[inline(always)]
    fn entity_id<'a>(world: impl WorldProvider<'a>) -> sys::ecs_entity_t {
        Self::UnderlyingType::__register_or_get_id::<true>(world)
    }

    // Not public API.
    #[doc(hidden)]
    fn __register_lifecycle_hooks(_type_hooks: &mut sys::ecs_type_hooks_t) {}

    // Not public API.
    #[doc(hidden)]
    fn __register_default_hooks(_type_hooks: &mut sys::ecs_type_hooks_t) {}

    // Not public API.
    #[doc(hidden)]
    fn __register_clone_hooks(_type_hooks: &mut sys::ecs_type_hooks_t) {}

    // Not public API.
    #[doc(hidden)]
    fn __register_compare_hooks(_type_hooks: &mut sys::ecs_type_hooks_t) {}

    // Not public API.
    #[doc(hidden)]
    fn __register_equals_hooks(_type_hooks: &mut sys::ecs_type_hooks_t) {}

    fn register_ctor_hook<'a>(world: impl WorldProvider<'a>)
    where
        Self: Default,
    {
        ecs_assert!(
            Self::IS_GENERIC,
            FlecsErrorCode::InvalidOperation,
            "There is no need to register default hooks for non generic components. This is done automatically if a type has Default implemented"
        );
        let world_ptr = world.world_ptr_mut();
        let id = Self::entity_id(world);
        let hooks = unsafe { flecs_ecs_sys::ecs_get_hooks_id(world_ptr, id) };
        if hooks.is_null() {
            let mut hooks = Default::default();
            register_ctor_lifecycle_actions::<Self>(&mut hooks);
            unsafe { flecs_ecs_sys::ecs_set_hooks_id(world_ptr, id, &hooks) }
        } else {
            let hooks = &mut unsafe { *hooks };
            register_ctor_lifecycle_actions::<Self>(hooks);
            unsafe { flecs_ecs_sys::ecs_set_hooks_id(world_ptr, id, hooks) }
        }
    }

    fn register_clone_hook<'a>(world: impl WorldProvider<'a>)
    where
        Self: Clone,
    {
        ecs_assert!(
            Self::IS_GENERIC,
            FlecsErrorCode::InvalidOperation,
            "There is no need to register clone hooks for non generic components. This is done automatically if a type has Clone implemented"
        );
        let world_ptr = world.world_ptr_mut();
        let id = Self::entity_id(world);
        let hooks = unsafe { flecs_ecs_sys::ecs_get_hooks_id(world_ptr, id) };
        if hooks.is_null() {
            let mut hooks = Default::default();
            register_copy_lifecycle_action::<Self>(&mut hooks);
            unsafe { flecs_ecs_sys::ecs_set_hooks_id(world_ptr, id, &hooks) }
        } else {
            let hooks = &mut unsafe { *hooks };
            register_copy_lifecycle_action::<Self>(hooks);
            unsafe { flecs_ecs_sys::ecs_set_hooks_id(world_ptr, id, hooks) }
        }
    }

    fn register_compare_hook<'a>(world: impl WorldProvider<'a>)
    where
        Self: PartialOrd,
    {
        ecs_assert!(
            Self::IS_GENERIC,
            FlecsErrorCode::InvalidOperation,
            "There is no need to register compare hooks for non generic components. This is done automatically if a type has PartialEq implemented"
        );
        let world_ptr = world.world_ptr_mut();
        let id = Self::entity_id(world);
        let hooks = unsafe { flecs_ecs_sys::ecs_get_hooks_id(world_ptr, id) };
        if hooks.is_null() {
            let mut hooks = Default::default();
            register_partial_ord_lifecycle_action::<Self>(&mut hooks);
            unsafe { flecs_ecs_sys::ecs_set_hooks_id(world_ptr, id, &hooks) }
        } else {
            let hooks = &mut unsafe { *hooks };
            register_partial_ord_lifecycle_action::<Self>(hooks);
            unsafe { flecs_ecs_sys::ecs_set_hooks_id(world_ptr, id, hooks) }
        }
    }

    fn register_equals_hook<'a>(world: impl WorldProvider<'a>)
    where
        Self: PartialEq,
    {
        ecs_assert!(
            Self::IS_GENERIC,
            FlecsErrorCode::InvalidOperation,
            "There is no need to register equals hooks for non generic components. This is done automatically if a type has PartialEq implemented"
        );
        let world_ptr = world.world_ptr_mut();
        let id = Self::entity_id(world);
        let hooks = unsafe { flecs_ecs_sys::ecs_get_hooks_id(world_ptr, id) };
        if hooks.is_null() {
            let mut hooks = Default::default();
            register_partial_eq_lifecycle_action::<Self>(&mut hooks);
            unsafe { flecs_ecs_sys::ecs_set_hooks_id(world_ptr, id, &hooks) }
        } else {
            let hooks = &mut unsafe { *hooks };
            register_partial_eq_lifecycle_action::<Self>(hooks);
            unsafe { flecs_ecs_sys::ecs_set_hooks_id(world_ptr, id, hooks) }
        }
    }

    #[doc(hidden)]
    #[inline(always)]
    fn fetch_new_index() -> u32 {
        static INDEX_POOL: AtomicU32 = AtomicU32::new(1);
        INDEX_POOL.fetch_add(1, Ordering::Relaxed)
    }

    #[doc(hidden)]
    #[inline(always)]
    fn get_or_init_index(id: &AtomicU32) -> u32 {
        match id.fetch_update(Ordering::Acquire, Ordering::Relaxed, |v| {
            if v != u32::MAX {
                None
            } else {
                Some(Self::fetch_new_index())
            }
        }) {
            Ok(_) => id.load(Ordering::Acquire),
            Err(old) => old,
        }
    }

    #[doc(hidden)]
    fn index() -> u32;
}

/// `OnInstantiate` policy a component can declare at compile time via
/// `#[flecs(traits((OnInstantiate, ...)))]`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OnInstantiatePolicy {
    Override,
    Inherit,
    DontInherit,
}

/// Component information trait.
pub trait ComponentInfo: Sized {
    const IS_GENERIC: bool;
    const IS_ENUM: bool;
    const IS_TAG: bool;
    /// Whether the component declares the `Sparse` trait at compile time via
    /// `#[flecs(traits(Sparse))]`. Traits added at runtime are not reflected here.
    const IS_SPARSE: bool = false;
    /// Whether the component declares the `DontFragment` trait at compile time
    /// via `#[flecs(traits(DontFragment))]`. Traits added at runtime are not
    /// reflected here.
    const IS_DONT_FRAGMENT: bool = false;
    /// The `OnInstantiate` policy the component declares at compile time via
    /// `#[flecs(traits((OnInstantiate, Inherit)))]`. Defaults to `Override`,
    /// matching flecs. Policies added at runtime are not reflected here.
    const ON_INSTANTIATE: OnInstantiatePolicy = OnInstantiatePolicy::Override;
    const NEEDS_DROP: bool = core::mem::needs_drop::<Self>();
    const IMPLS_CLONE: bool;
    const IMPLS_DEFAULT: bool;
    const IMPLS_PARTIAL_ORD: bool;
    const IMPLS_PARTIAL_EQ: bool;
    /// Whether the component type is [`Send`]. Components that are not `Send`
    /// are thread-bound: they may only be accessed from the thread that owns
    /// the [`World`][crate::core::World] and cannot be used in multithreaded systems.
    const IMPLS_SEND: bool;
    /// Whether the component type is [`Sync`].
    const IMPLS_SYNC: bool;
    #[doc(hidden)]
    const IS_REF: bool;
    #[doc(hidden)]
    const IS_MUT: bool;
    #[doc(hidden)]
    type TagType;
}

/// Caches the ids, index and name of the enum variants.
pub trait EnumComponentInfo: ComponentType<Enum> + ComponentId {
    const SIZE_ENUM_FIELDS: u32;
    type VariantIterator: Iterator<Item = Self>;

    /// # Note
    /// this function is used to pass the name to the C API.
    fn name_cstr(&self) -> &CStr;

    fn enum_index(&self) -> usize;

    fn iter() -> Self::VariantIterator;

    /// Looks up the cached entity ID for `variant_index` in this specific world's `components_array`.
    /// Returns 0 if not registered with this world.
    #[doc(hidden)]
    fn __get_variant_id_from_world<'a>(variant_index: usize, world: impl WorldProvider<'a>) -> u64 {
        let world = world.world();
        let idx = Self::__enum_variant_index(variant_index) as usize;
        let arr = world.components_array();
        if idx < arr.len() { arr[idx] } else { 0 }
    }

    #[doc(hidden)]
    #[doc(hidden)]
    fn are_fields_registered_as_entities<'a>(world: impl WorldProvider<'a>) -> bool {
        let world = world.world();
        let arr = world.components_array();
        for i in 0..Self::SIZE_ENUM_FIELDS as usize {
            let idx = Self::__enum_variant_index(i) as usize;
            if idx >= arr.len() || arr[idx] == 0 {
                return false;
            }
        }
        true
    }

    #[doc(hidden)]
    #[doc(hidden)]
    fn is_field_registered_as_entity<'a>(&self, world: impl WorldProvider<'a>) -> bool {
        Self::__get_variant_id_from_world(self.enum_index(), world) != 0
    }

    #[doc(hidden)]
    #[doc(hidden)]
    fn is_index_registered_as_entity<'a>(index: usize, world: impl WorldProvider<'a>) -> bool {
        Self::__get_variant_id_from_world(index, world) != 0
    }

    /// get the entity id of the variant of the enum. This function will register the enum with the world if it's not registered.
    fn id_variant<'a>(&self, world: impl WorldProvider<'a>) -> EntityView<'a> {
        use crate::core::component_registration::registration;
        let world = world.world();
        if !<Self as ComponentId>::is_registered_with_world(world) {
            const COMPONENT_REGISTRATION: bool = false;
            registration::try_register_component::<COMPONENT_REGISTRATION, Self>(world);
        }
        let index = self.enum_index();
        EntityView::new_from(world, Self::__get_variant_id_from_world(index, world))
    }

    /// # Safety
    ///
    /// This function is unsafe because it assumes the enum has been registered as a component with the world.
    /// if uncertain, use `try_register_component::<T>` to try and register it
    unsafe fn id_variant_unchecked<'a>(&self, world: impl WorldProvider<'a>) -> EntityView<'a> {
        let index = self.enum_index();
        EntityView::new_from(
            world.world(),
            Self::__get_variant_id_from_world(index, world),
        )
    }

    #[doc(hidden)]
    #[doc(hidden)]
    fn id_variant_of_index<'a>(index: usize, world: impl WorldProvider<'a>) -> Option<u64> {
        if index < Self::SIZE_ENUM_FIELDS as usize {
            Some(Self::__get_variant_id_from_world(index, world))
        } else {
            None
        }
    }

    /// # Safety
    /// This function is unsafe because you must ensure that the index is within the bounds of
    /// the number of variants in the enum. If uncertain, use `SIZE_ENUM_FIELDS` to check.
    #[doc(hidden)]
    unsafe fn id_variant_of_index_unchecked<'a>(
        index: usize,
        world: impl WorldProvider<'a>,
    ) -> u64 {
        Self::__get_variant_id_from_world(index, world)
    }

    #[doc(hidden)]
    fn __enum_variant_index(variant_index: usize) -> u32;
}

#[doc(hidden)]
impl<T: ComponentInfo> ComponentInfo for &T {
    const IS_GENERIC: bool = T::IS_GENERIC;
    const IS_ENUM: bool = T::IS_ENUM;
    const IS_TAG: bool = T::IS_TAG;
    const IMPLS_CLONE: bool = T::IMPLS_CLONE;
    const IMPLS_DEFAULT: bool = T::IMPLS_DEFAULT;
    const IMPLS_PARTIAL_ORD: bool = T::IMPLS_PARTIAL_ORD;
    const IMPLS_PARTIAL_EQ: bool = T::IMPLS_PARTIAL_EQ;
    const IMPLS_SEND: bool = T::IMPLS_SEND;
    const IMPLS_SYNC: bool = T::IMPLS_SYNC;
    const IS_SPARSE: bool = T::IS_SPARSE;
    const IS_DONT_FRAGMENT: bool = T::IS_DONT_FRAGMENT;
    const ON_INSTANTIATE: OnInstantiatePolicy = T::ON_INSTANTIATE;
    const IS_REF: bool = true;
    const IS_MUT: bool = false;
    type TagType = T::TagType;
}

#[doc(hidden)]
impl<T: ComponentInfo> ComponentInfo for &mut T {
    const IS_GENERIC: bool = T::IS_GENERIC;
    const IS_ENUM: bool = T::IS_ENUM;
    const IS_TAG: bool = T::IS_TAG;
    const IMPLS_CLONE: bool = T::IMPLS_CLONE;
    const IMPLS_DEFAULT: bool = T::IMPLS_DEFAULT;
    const IMPLS_PARTIAL_ORD: bool = T::IMPLS_PARTIAL_ORD;
    const IMPLS_PARTIAL_EQ: bool = T::IMPLS_PARTIAL_EQ;
    const IMPLS_SEND: bool = T::IMPLS_SEND;
    const IMPLS_SYNC: bool = T::IMPLS_SYNC;
    const IS_SPARSE: bool = T::IS_SPARSE;
    const IS_DONT_FRAGMENT: bool = T::IS_DONT_FRAGMENT;
    const ON_INSTANTIATE: OnInstantiatePolicy = T::ON_INSTANTIATE;
    const IS_REF: bool = false;
    const IS_MUT: bool = true;
    type TagType = T::TagType;
}

#[doc(hidden)]
impl<T: InternalComponentHooks> InternalComponentHooks for &T {
    fn internal_on_component_registration(world: WorldRef, component_id: Entity) {
        T::internal_on_component_registration(world, component_id);
    }
}

#[doc(hidden)]
impl<T: OnComponentRegistration> OnComponentRegistration for &T {
    fn on_component_registration(world: WorldRef, component_id: Entity) {
        T::on_component_registration(world, component_id);
    }
}

#[doc(hidden)]
impl<T: InternalComponentHooks> InternalComponentHooks for &mut T {
    fn internal_on_component_registration(world: WorldRef, component_id: Entity) {
        T::internal_on_component_registration(world, component_id);
    }
}

#[doc(hidden)]
impl<T: OnComponentRegistration> OnComponentRegistration for &mut T {
    fn on_component_registration(world: WorldRef, component_id: Entity) {
        T::on_component_registration(world, component_id);
    }
}

#[doc(hidden)]
impl<T: ComponentId> ComponentId for &'static T {
    #[inline(always)]
    fn index() -> u32 {
        T::UnderlyingType::index()
    }

    type UnderlyingType = T::UnderlyingType;
    type UnderlyingEnumType = T::UnderlyingEnumType;
    type UnderlyingTypeOfEnum = T::UnderlyingTypeOfEnum;
}

#[doc(hidden)]
impl<T: ComponentId> ComponentId for &'static mut T {
    #[inline(always)]
    fn index() -> u32 {
        T::UnderlyingType::index()
    }

    type UnderlyingType = T::UnderlyingType;
    type UnderlyingEnumType = T::UnderlyingEnumType;
    type UnderlyingTypeOfEnum = T::UnderlyingTypeOfEnum;
}

#[doc(hidden)]
pub trait FlecsDefaultType {
    #[doc(hidden)]
    type Type: Default;
}

#[doc(hidden)]
pub trait FlecsCloneType {
    type Type: Clone;
}

#[doc(hidden)]
pub trait FlecsPartialEqType {
    type Type: PartialEq;
}

#[doc(hidden)]
pub trait FlecsPartialOrdType {
    type Type: PartialOrd;
}

#[doc(hidden)]
pub trait FlecsPairType {
    type Type: ComponentId;
    const IS_FIRST: bool;
}

#[doc(hidden)]
pub trait FlecsCachedRefPairType {
    type Type;
}

#[doc(hidden)]
impl<T> FlecsDefaultType for ConditionalTypeSelector<false, T> {
    type Type = FlecsNoneDefaultDummy;
}

#[doc(hidden)]
impl<T> FlecsDefaultType for ConditionalTypeSelector<true, T>
where
    T: Default,
{
    type Type = T;
}

#[doc(hidden)]
impl<T> FlecsCloneType for ConditionalTypeSelector<false, T> {
    type Type = FlecsNoneCloneDummy;
}

#[doc(hidden)]
impl<T> FlecsCloneType for ConditionalTypeSelector<true, T>
where
    T: Clone,
{
    type Type = T;
}

#[doc(hidden)]
impl<T> FlecsPartialEqType for ConditionalTypeSelector<false, T> {
    type Type = FlecsNonePartialEqDummy;
}

#[doc(hidden)]
impl<T> FlecsPartialEqType for ConditionalTypeSelector<true, T>
where
    T: PartialEq,
{
    type Type = T;
}

#[doc(hidden)]
impl<T> FlecsPartialOrdType for ConditionalTypeSelector<false, T> {
    type Type = FlecsNonePartialOrdDummy;
}

#[doc(hidden)]
impl<T> FlecsPartialOrdType for ConditionalTypeSelector<true, T>
where
    T: PartialOrd,
{
    type Type = T;
}

#[doc(hidden)]
pub struct FlecsIsATag;
#[doc(hidden)]
pub struct FlecsNotATag;

#[doc(hidden)]
pub struct FlecsIsTyped;

#[doc(hidden)]
pub struct FlecsIsNotTyped;

#[doc(hidden)]
impl<T, U> FlecsPairType for ConditionalTypePairSelector<FlecsNotATag, T, U>
where
    T: ComponentId,
    U: ComponentId,
{
    type Type = T;
    const IS_FIRST: bool = true;
}

#[doc(hidden)]
impl<T, U> FlecsPairType for ConditionalTypePairSelector<FlecsIsATag, T, U>
where
    T: ComponentId,
    U: ComponentId,
{
    type Type = U;
    const IS_FIRST: bool = false;
}

#[doc(hidden)]
impl<IsFirstTyped, IsSecondTyped, T, U> FlecsCachedRefPairType
    for ConditionalCachedRefTypeSelector<
        IsFirstTyped,
        IsSecondTyped,
        FlecsIsATag,
        FlecsIsATag,
        T,
        U,
    >
where
    T: InternalIntoEntity,
    U: InternalIntoEntity,
{
    type Type = core::ffi::c_void;
}

#[doc(hidden)]
impl<IsFirstTyped, IsSecondTyped, IsSecondATag, T, U> FlecsCachedRefPairType
    for ConditionalCachedRefTypeSelector<
        IsFirstTyped,
        IsSecondTyped,
        FlecsNotATag,
        IsSecondATag,
        T,
        U,
    >
where
    T: InternalIntoEntity,
    U: InternalIntoEntity,
{
    type Type = T::CastType;
}

#[doc(hidden)]
impl<IsSecondTyped, T, U> FlecsCachedRefPairType
    for ConditionalCachedRefTypeSelector<
        FlecsIsNotTyped,
        IsSecondTyped,
        FlecsIsATag,
        FlecsNotATag,
        T,
        U,
    >
where
    T: InternalIntoEntity,
    U: InternalIntoEntity,
{
    type Type = core::ffi::c_void;
}

#[doc(hidden)]
impl<IsSecondTyped, T, U> FlecsCachedRefPairType
    for ConditionalCachedRefTypeSelector<
        FlecsIsTyped,
        IsSecondTyped,
        FlecsIsATag,
        FlecsNotATag,
        T,
        U,
    >
where
    T: InternalIntoEntity,
    U: InternalIntoEntity,
{
    type Type = U::CastType;
}

// #[doc(hidden)]
// impl<IsSecondTyped, IsSecondTag, T, U> FlecsCachedRefPairType
//     for ConditionalCachedRefTypeSelector<
//         FlecsIsTyped,
//         IsSecondTyped,
//         FlecsNotATag,
//         IsSecondTag,
//         T,
//         U,
//     >
// where
//     T: InternalIntoEntity,
//     U: InternalIntoEntity,
// {
//     type Type = T::CastType;
// }

// #[doc(hidden)]
// impl<IsFirstTyped, T, U> FlecsCachedRefPairType
//     for ConditionalCachedRefTypeSelector<
//         IsFirstTyped,
//         FlecsIsTyped,
//         FlecsIsATag,
//         FlecsNotATag,
//         T,
//         U,
//     >
// where
//     T: InternalIntoEntity,
//     U: InternalIntoEntity,
// {
//     type Type = U::CastType;
// }
