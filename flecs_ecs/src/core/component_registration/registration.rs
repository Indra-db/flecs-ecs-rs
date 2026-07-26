#![doc(hidden)]
use core::ffi::c_char;
use core::ffi::c_void;

use crate::core::*;
use crate::sys;

struct ScopeWithGuard {
    world: *mut sys::ecs_world_t,
    prev_scope: Option<sys::ecs_entity_t>,
    prev_with: Option<sys::ecs_id_t>,
}

impl ScopeWithGuard {
    #[inline(always)]
    fn new(world: *mut sys::ecs_world_t, disable_component_scoping: bool) -> Self {
        if !disable_component_scoping {
            return Self {
                world,
                prev_scope: None,
                prev_with: None,
            };
        }

        let prev_scope = unsafe {
            let current_scope = sys::ecs_get_scope(world);
            if current_scope != 0 {
                sys::ecs_set_scope(world, 0);
                Some(current_scope)
            } else {
                None
            }
        };

        let prev_with = unsafe {
            let previous_with = sys::ecs_set_with(world, 0);
            if previous_with != 0 {
                Some(previous_with)
            } else {
                None
            }
        };

        Self {
            world,
            prev_scope,
            prev_with,
        }
    }
}

impl Drop for ScopeWithGuard {
    fn drop(&mut self) {
        if let Some(prev_with) = self.prev_with {
            unsafe { sys::ecs_set_with(self.world, prev_with) };
        }

        if let Some(prev_scope) = self.prev_scope {
            unsafe { sys::ecs_set_scope(self.world, prev_scope) };
        }
    }
}

#[doc(hidden)]
pub fn internal_register_component<
    'a,
    const IS_NAMED: bool,
    const COMPONENT_REGISTRATION: bool,
    T,
>(
    world: impl WorldProvider<'a>,
    name: *const c_char,
) -> u64
where
    T: ComponentId,
{
    let world = world.world();

    let id = if IS_NAMED {
        register_component_data_named::<COMPONENT_REGISTRATION, T>(world, name)
    } else {
        register_component_data::<COMPONENT_REGISTRATION, T>(world)
    };

    if T::IS_ENUM {
        let underlying_enum_type_id = world.component_id::<T::UnderlyingTypeOfEnum>();
        register_enum_data::<T>(world, id, *underlying_enum_type_id);
    }
    id
}

#[doc(hidden)]
pub(crate) fn external_register_component<'a, const COMPONENT_REGISTRATION: bool, T>(
    world: impl WorldProvider<'a>,
    name: *const c_char,
) -> u64
where
    T: 'static,
{
    let world = world.world();
    external_register_component_data::<COMPONENT_REGISTRATION, T>(world, name) as u64
}

#[inline(always)]
/// attempts to register the component with the world. If it's already registered, it does nothing.
pub(crate) fn try_register_component<'a, const COMPONENT_REGISTRATION: bool, T>(
    world: impl WorldProvider<'a>,
) -> sys::ecs_entity_t
where
    T: ComponentId,
{
    const NAMED: bool = false;
    internal_register_component::<NAMED, COMPONENT_REGISTRATION, T>(world, core::ptr::null())
}

#[inline(always)]
pub(crate) fn try_register_component_named<'a, const COMPONENT_REGISTRATION: bool, T>(
    world: impl WorldProvider<'a>,
    name: &str,
) -> sys::ecs_entity_t
where
    T: ComponentId,
{
    let name = compact_str::format_compact!("{}\0", name);
    const NAMED: bool = true;
    internal_register_component::<NAMED, COMPONENT_REGISTRATION, T>(
        world,
        name.as_ptr() as *const c_char,
    )
}

/// registers enum fields with the world.
pub(crate) fn register_enum_data<T>(
    world: WorldRef<'_>,
    id: sys::ecs_entity_t,
    underlying_type_id: sys::ecs_entity_t,
) where
    T: ComponentId,
{
    //TODO we should convert this ecs_cpp functions to rust so if it ever changes, our solution won't break
    let world_ptr = world.world_ptr_mut();
    unsafe { sys::ecs_cpp_enum_init(world_ptr, id, underlying_type_id) };
    let enum_size = const { core::mem::size_of::<T::UnderlyingTypeOfEnum>() };
    const {
        assert!(core::mem::size_of::<T::UnderlyingTypeOfEnum>() <= core::mem::size_of::<u64>());
    }
    for enum_item in T::UnderlyingEnumType::iter() {
        let name = enum_item.name_cstr();
        let enum_index = enum_item.enum_index();
        let mut constant_value = 0u64;
        let constant_value_ptr = &mut constant_value as *mut u64;
        // SAFETY: `constant_value` is an 8-byte, 8-aligned local, so writing any
        // integer of `enum_size` (1, 2, 4 or 8) bytes at its address is in bounds
        // and sufficiently aligned.
        unsafe {
            match enum_size {
                1 => (constant_value_ptr as *mut u8).write(enum_index as u8),
                2 => (constant_value_ptr as *mut u16).write(enum_index as u16),
                4 => (constant_value_ptr as *mut u32).write(enum_index as u32),
                _ => constant_value_ptr.write(enum_index as u64),
            }
        }
        // SAFETY: C reads `enum_size` bytes from `constant_value`, which is at
        // least `enum_size` bytes large (const-asserted above).
        let entity_id: sys::ecs_entity_t = unsafe {
            sys::ecs_cpp_enum_constant_register(
                world_ptr,
                id,
                // Pass current world's cached ID (0 = new registration, existing = re-registration)
                T::UnderlyingEnumType::id_variant_of_index_unchecked(enum_index, world),
                name.as_ptr(),
                constant_value_ptr as *mut c_void,
                underlying_type_id,
                enum_size,
            )
        };
        store_enum_entity_if_needed::<T>(world, enum_index, entity_id);
    }
}

#[inline(always)]
fn store_enum_entity_if_needed<T: ComponentId>(
    world: WorldRef<'_>,
    index: usize,
    entity_id: sys::ecs_entity_t,
) {
    if !T::UnderlyingEnumType::is_index_registered_as_entity(index, world) {
        let variant_idx = T::UnderlyingEnumType::__enum_variant_index(index) as usize;
        let arr = world.components_array();
        if variant_idx >= arr.len() {
            arr.reserve(variant_idx + 1 - arr.len());
            let capacity = arr.capacity();
            unsafe {
                core::ptr::write_bytes(arr.as_mut_ptr().add(arr.len()), 0, capacity - arr.len());
                arr.set_len(capacity);
            }
        }
        arr[variant_idx] = entity_id;
    }
}

/// registers the component with the world.
pub(crate) fn register_component_data_named<const COMPONENT_REGISTRATION: bool, T>(
    world: WorldRef<'_>,
    name: *const c_char,
) -> sys::ecs_entity_t
where
    T: ComponentId,
{
    let world_ptr = world.world_ptr_mut();
    let _scope = ScopeWithGuard::new(world_ptr, !COMPONENT_REGISTRATION);
    register_component_data_explicit::<T, false>(world, name)
}

/// registers the component with the world.
pub(crate) fn register_component_data<const COMPONENT_REGISTRATION: bool, T>(
    world: WorldRef<'_>,
) -> sys::ecs_entity_t
where
    T: ComponentId,
{
    let world_ptr = world.world_ptr_mut();
    let _scope = ScopeWithGuard::new(world_ptr, !COMPONENT_REGISTRATION);
    register_component_data_explicit::<T, false>(world, core::ptr::null())
}

pub(crate) fn external_register_component_data<const COMPONENT_REGISTRATION: bool, T>(
    world: WorldRef<'_>,
    name: *const c_char,
) -> sys::ecs_entity_t
where
    T: 'static,
{
    let world_ptr = world.world_ptr_mut();
    let _scope = ScopeWithGuard::new(world_ptr, !COMPONENT_REGISTRATION);

    external_register_component_data_explicit::<T>(world, name)
}

/// registers the component with the world.
pub(crate) fn register_component_data_explicit<T, const ALLOCATE_TAG: bool>(
    world: WorldRef<'_>,
    name: *const c_char,
) -> sys::ecs_entity_t
where
    T: ComponentId,
{
    let worldref = world;
    let world = worldref.world_ptr_mut();

    // The `index()` fast path is only sound for non-generic types: statics
    // declared inside a generic fn are shared across every monomorphization, so
    // `Container<A>` and `Container<B>` collide on the same array slot. Generic
    // types are resolved by their (unique per monomorphization) symbol below.
    let c = if !T::IS_GENERIC {
        let arr = worldref.components_array();
        let index = T::index() as usize;
        if index < arr.len() { arr[index] } else { 0 }
    } else {
        0
    };

    if c != 0 && unsafe { sys::ecs_is_alive(world, c) } {
        return c;
    }

    let type_name_without_scope = if T::IS_GENERIC {
        crate::core::get_type_name_without_scope_generic::<T>()
    } else {
        crate::core::get_type_name_without_scope::<T>()
    };

    let type_name_without_scope =
        compact_str::format_compact!("{}\0", type_name_without_scope.as_str());

    let type_name = crate::core::type_name_cstring::<T>();
    let type_name_ptr = type_name.as_ptr();

    let mut user_name = name;
    let mut implicit_name = false;

    if user_name.is_null() {
        // If no name was provided, use the type name as the name.
        user_name = type_name_ptr;
        /* Keep track of whether name was explicitly set. If not, and
         * the component was already registered, just use the registered
         * name. The registered name may differ from the typename as the
         * registered name includes the flecs scope. This can in theory
         * be different from the C++ namespace though it is good
         * practice to keep them the same */
        implicit_name = true;
    }

    if implicit_name {
        // Use only the short name (last segment of the Rust path).
        // Inside a module scope this registers the component relative to the module.
        // At root scope this matches C++ behavior: ecs_cpp_component_register uses
        // ecs_cpp_trim_module which strips the namespace when no scope is active,
        // effectively registering just "Position" not "myns::Position".
        user_name = type_name_without_scope.as_ptr() as *const c_char;
    }

    //TODO should I do this?
    /* If the component is not yet registered, ensure no other component
     * or entity has been registered with this name. Ensure component is
     * looked up from root. */
    // let prev_scope = unsafe { sys::ecs_set_scope(world, 0) };
    // c = unsafe {
    //     sys::ecs_lookup_path_w_sep(
    //         world,
    //         0,
    //         user_name,
    //         SEPARATOR.as_ptr(),
    //         SEPARATOR.as_ptr(),
    //         false,
    //     )
    // };

    // TODO it feels like this should be investigated with the get only type name approach
    // TODO if it's valid. When two components with the same name, but different module get registered
    // this still gives id 0, so what's the point of this check?

    // Always check if this type was already registered by looking up its symbol.
    // The symbol is the full Rust type_name (unique per type), so this lookup only ever
    // finds the same type's entity — safe to reuse unconditionally.
    // This covers both the implicit-name path AND the explicit-name path
    // (#[flecs(name = "...")] or module-qualified names). Without this, calling
    // component::<T>() a second time from inside a hook/observer would skip the cache
    // (arr[index] == 0 because finalize_component_registration doesn't write it back),
    // attempt to create a duplicate entity with the same name, and SIGABRT.
    let prev_scope = unsafe { sys::ecs_set_scope(world, 0) };
    let id = unsafe { sys::ecs_lookup_symbol(world, type_name_ptr as *const _, false, false) };
    unsafe { sys::ecs_set_scope(world, prev_scope) };
    if id != 0 {
        return id;
    }

    let name = user_name;

    //TODO hack, otherwise importing will have mismatch symbol with the c components
    let entity_desc_name = if type_name_without_scope.starts_with("Ecs") {
        type_name_without_scope.as_ptr() as *const c_char
    } else {
        type_name_ptr
    };
    let type_info = create_type_info::<T, ALLOCATE_TAG>();

    finalize_component_registration(world, name, entity_desc_name, type_info)
}

pub(crate) fn external_register_component_data_explicit<T>(
    world: WorldRef<'_>,
    name: *const c_char,
) -> sys::ecs_entity_t
where
    T: 'static,
{
    let world_ptr = world.world_ptr_mut();

    let type_name_without_scope = crate::core::get_type_name_without_scope_generic::<T>();
    let type_name_without_scope =
        compact_str::format_compact!("{}\0", type_name_without_scope.as_str());

    let id = if name.is_null() {
        // SAFETY: `world_ptr` is the caller-provided valid world pointer; setting
        // the root scope (0) temporarily is a simple FFI call with no further
        // invariants beyond a valid world pointer.
        let prev_scope = unsafe { sys::ecs_set_scope(world_ptr, 0) };
        // SAFETY: `type_name_without_scope` is a `compact_str` built with a
        // trailing '\0' above, so its pointer is a valid null-terminated C string
        // for the duration of this call, and `world_ptr` is valid.
        let id = unsafe {
            sys::ecs_lookup_symbol(
                world_ptr,
                type_name_without_scope.as_ptr() as *const _,
                false,
                false,
            )
        };
        // SAFETY: `world_ptr` is the caller-provided valid world pointer and
        // `prev_scope` is the scope id returned by the `ecs_set_scope` call above,
        // so this restores the caller's original scope via a simple FFI setter.
        unsafe { sys::ecs_set_scope(world_ptr, prev_scope) };
        id
    } else {
        0
    };
    if id != 0 {
        return id;
    }

    let type_name = crate::core::type_name_cstring::<T>();
    let type_name_ptr = type_name.as_ptr();

    let name = if name.is_null() { type_name_ptr } else { name };

    let type_info = external_create_type_info::<T>();

    finalize_component_registration(world_ptr, name, type_name_ptr, type_info)
}

fn finalize_component_registration(
    world: *mut sys::ecs_world_t,
    name: *const c_char,
    entity_desc_name: *const c_char,
    type_info: sys::ecs_type_info_t,
) -> sys::ecs_entity_t {
    let entity_desc = create_entity_desc(name, entity_desc_name);

    // SAFETY: `entity_desc` was built by `create_entity_desc` with valid,
    // null-terminated name pointers, and `world` is the caller-provided valid
    // world pointer; `ecs_entity_init` follows the flecs C API contract for
    // entity creation.
    let entity = unsafe { flecs_ecs_sys::ecs_entity_init(world, &entity_desc) };

    let component_desc = create_component_desc(entity, type_info);

    #[cfg(any(debug_assertions, feature = "flecs_force_enable_ecs_asserts"))]
    {
        // SAFETY: `component_desc` was built by `create_component_desc` with a valid
        // entity id and type_info, and `world` is the caller-provided valid world
        // pointer; `ecs_component_init` follows the flecs C API contract for
        // component registration.
        let entity = unsafe { flecs_ecs_sys::ecs_component_init(world, &component_desc) };
        // SAFETY: the `&&` short-circuits on `entity != 0`, so `ecs_exists` is only
        // called with the non-zero entity id just returned by `ecs_component_init`
        // above on the same valid `world` pointer; this is a read-only existence
        // check.
        ecs_assert!(
            entity != 0 && unsafe { sys::ecs_exists(world, entity) },
            FlecsErrorCode::InternalError
        );

        entity
    }
    #[cfg(not(any(debug_assertions, feature = "flecs_force_enable_ecs_asserts")))]
    {
        unsafe { flecs_ecs_sys::ecs_component_init(world, &component_desc) }
    }
}
