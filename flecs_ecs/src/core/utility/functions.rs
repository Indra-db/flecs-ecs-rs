#![doc(hidden)]
//! (internal) utility functions for dealing with ECS identifiers. This module is mostly used internally by the library.
//! but can be used by the user if needed.
use crate::core::*;
use crate::sys;
use core::ffi::c_char;

#[cfg(feature = "std")]
extern crate std;

extern crate alloc;
use alloc::{
    ffi::CString,
    format,
    string::{String, ToString},
    vec::Vec,
};

const ECS_GENERATION_MASK: u64 = u32::MAX as u64;

/// Combines two 32 bit integers into a 64 bit integer.
///
/// # Arguments
///
/// * `lo`: The lower 32 bit integer.
/// * `hi`: The higher 32 bit integer.
///
/// # Returns
///
/// The combined 64 bit integer.
#[inline(always)]
pub fn ecs_entity_id_combine(lo: u64, hi: u64) -> u64 {
    (hi << 32) | (lo & ECS_GENERATION_MASK)
}

/// Combines two 32 bit integers into a 64 bit integer and adds the `ECS_PAIR` flag.
///
/// # Arguments
///
/// * `rel`: The first 32 bit integer.
/// * `target`: The second 32 bit integer.
///
/// # Returns
///
/// The combined 64 bit integer with the `ECS_PAIR` flag set.
#[inline(always)]
pub fn ecs_pair(rel: u64, target: u64) -> u64 {
    ECS_PAIR | ecs_entity_id_combine(target, rel)
}

/// Checks if given entity is a pair
pub fn ecs_is_pair(entity: impl Into<Id>) -> bool {
    entity.into() & RUST_ecs_id_FLAGS_MASK == ECS_PAIR
}

/// Combine a relationship with a raw 32-bit value into a value pair id.
///
/// Value pairs encode a literal value in the target position of a pair
/// instead of an entity (matches the C `ecs_value_pair` macro).
#[inline(always)]
pub fn ecs_value_pair(rel: u64, value: u64) -> u64 {
    ECS_VALUE_PAIR | ecs_entity_id_combine(value, rel)
}

/// Set the `ECS_DEPENDS_ON` flag for the given entity.
///
/// # Arguments
///
/// * `entity`: The entity to set the `ECS_DEPENDS_ON` flag for.
///
/// # Returns
///
/// The entity with the `ECS_DEPENDS_ON` flag set.
#[inline(always)]
pub fn ecs_dependson(entity: u64) -> u64 {
    ecs_pair(ECS_DEPENDS_ON, entity)
}

/// Returns true if the entity has the given pair.
///
/// # Arguments
///
/// * `world`: The world to check in.
/// * `entity`: The entity to check.
/// * `first`: The first entity of the pair.
/// * `second`: The second entity of the pair.
///
/// # Returns
///
/// True if the entity has the given pair, false otherwise.
#[inline(always)]
#[expect(dead_code, reason = "possibly used in the future")]
pub(crate) fn ecs_has_pair(
    world: *const sys::ecs_world_t,
    entity: impl Into<Entity>,
    first: impl Into<Entity>,
    second: impl Into<Entity>,
) -> bool {
    unsafe {
        sys::ecs_has_id(
            world,
            *entity.into(),
            ecs_pair(*first.into(), *second.into()),
        )
    }
}

#[inline(always)]
pub(crate) fn ecs_add_pair(
    world: *mut sys::ecs_world_t,
    entity: impl Into<Entity>,
    first: impl Into<Entity>,
    second: impl Into<Entity>,
) {
    unsafe {
        sys::ecs_add_id(
            world,
            *entity.into(),
            ecs_pair(*first.into(), *second.into()),
        );
    };
}

/// Get the first entity from a pair.
///
/// # Arguments
///
/// * `e`: The pair to get the first entity from.
///
/// # Returns
///
/// The first entity from the pair.
#[inline(always)]
pub fn ecs_first<'a>(e: impl IntoId, world: impl WorldProvider<'a>) -> Entity {
    let world = world.world();
    let id = (*e.into_id(world)) & RUST_ECS_COMPONENT_MASK;
    Entity(ecs_entity_id_high(id))
}

/// Get the second entity from a pair.
///
/// # Arguments
///
/// * `e`: The pair to get the second entity from.
///
/// # Returns
///
/// The second entity from the pair.
#[inline(always)]
pub fn ecs_second<'a>(e: impl IntoId, world: impl WorldProvider<'a>) -> Entity {
    let world = world.world();
    Entity(ecs_entity_id_low(Entity(*e.into_id(world))))
}

/// Get the lower 32 bits of an entity id.
///
/// # Arguments
///
/// * `value`: The entity id to get the lower 32 bits from.
///
/// # Returns
///
/// The lower 32 bits of the entity id.
#[inline(always)]
pub fn ecs_entity_id_low(value: impl Into<Entity>) -> u64 {
    *value.into() as u32 as u64
}

/// Get the higher 32 bits of an entity id.
///
/// # Arguments
///
/// * `value`: The entity id to get the higher 32 bits from.
///
/// # Returns
///
/// The higher 32 bits of the entity id.
#[inline(always)]
pub fn ecs_entity_id_high(value: impl Into<Entity>) -> u64 {
    *value.into() >> 32
}

pub fn type_name_cstring<T>() -> CString {
    CString::new(core::any::type_name::<T>()).unwrap()
}

#[derive(Debug, Clone)]
pub enum OnlyTypeName {
    NonGeneric(&'static str),
    Generic(String),
}

impl OnlyTypeName {
    /// Get the type name as a string slice.
    pub fn as_str(&self) -> &str {
        match self {
            OnlyTypeName::NonGeneric(name) => name,
            OnlyTypeName::Generic(name) => name,
        }
    }
}

impl PartialEq for OnlyTypeName {
    fn eq(&self, other: &Self) -> bool {
        self.as_str() == other.as_str()
    }
}

impl PartialEq<&str> for OnlyTypeName {
    fn eq(&self, other: &&str) -> bool {
        self.as_str() == *other
    }
}

impl PartialEq<String> for OnlyTypeName {
    fn eq(&self, other: &String) -> bool {
        self.as_str() == other.as_str()
    }
}

/// Get the type name of the given type.
///
/// # Type Parameters
///
/// * `T`: The type to get the name of.
///
/// # Returns
///
/// `[Type]` string slice.
///
/// # Example
///
/// ```
/// use flecs_ecs::prelude::*;
///
/// pub mod Bar {
///     use flecs_ecs::prelude::*;
///     #[derive(Component)]
///     pub struct Foo;
/// }
///
/// let name = get_type_name_without_scope::<Bar::Foo>();
/// assert_eq!(name, "Foo");
/// ```
#[inline(always)]
pub fn get_type_name_without_scope<T: ComponentId>() -> OnlyTypeName {
    ecs_assert!(
        !T::IS_GENERIC,
        FlecsErrorCode::InvalidParameter,
        "get_only_type_name() cannot be used with generic types"
    );
    let name = T::name();
    OnlyTypeName::NonGeneric(name.split("::").last().unwrap_or(name))
}

/// Get the type name of the given type.
///
/// # Type Parameters
///
/// * `T`: The type to get the name of.
///
/// # Returns
///
/// `[Type]` string slice.
///
/// # Example
///
/// ```
/// use flecs_ecs::core::get_type_name_without_scope_generic;
///
/// pub mod Bar {
///     pub struct Foo;
/// }
///
/// let name = get_type_name_without_scope_generic::<Bar::Foo>();
/// assert_eq!(name, "Foo");
/// ```
#[inline(always)]
pub fn get_type_name_without_scope_generic<T>() -> OnlyTypeName {
    fn split_top_level(s: &str) -> Vec<&str> {
        let mut parts = Vec::new();
        let mut depth = 0;
        let mut start = 0;
        for (i, c) in s.char_indices() {
            match c {
                '<' => depth += 1,
                '>' => depth -= 1,
                ',' if depth == 0 => {
                    parts.push(&s[start..i]);
                    start = i + 1;
                }
                _ => {}
            }
        }
        parts.push(&s[start..]);
        parts
    }

    fn strip_paths(name: &str) -> String {
        if let Some(lt) = name.find('<') {
            // has generics
            let base = &name[..lt];
            let args = &name[lt + 1..name.len() - 1]; // skip final '>'
            let base_name = base.rsplit("::").next().unwrap_or(base);
            let args_strs = split_top_level(args);
            let stripped_args: Vec<String> = args_strs
                .into_iter()
                .map(|arg| strip_paths(arg.trim()))
                .collect();
            format!("{}<{}>", base_name, stripped_args.join(", "))
        } else {
            // no generics
            name.rsplit("::").next().unwrap_or(name).to_string()
        }
    }

    let full = core::any::type_name::<T>();
    OnlyTypeName::Generic(strip_paths(full))
}

/// Returns true if the given type is an empty type.
///
/// # Type Parameters
///
/// * `T`: The type to check.
#[inline(always)]
pub const fn is_empty_type<T>() -> bool {
    core::mem::size_of::<T>() == 0
}

/// Extracts a row index from an ECS record identifier.
///
/// Applies a bitwise AND with `ECS_ROW_MASK` to `row`, isolating relevant bits,
/// and returns the result as an `i32`. This function is typically used to decode
/// information about an entity's components or position encoded within the record identifier.
///
/// # Arguments
///
/// * `row`: A `u32` representing an ECS record identifier.
///
/// # Returns
///
/// * `i32`: The decoded row index.
pub fn ecs_record_to_row(row: u32) -> i32 {
    (row & sys::ECS_ROW_MASK) as i32
}

pub(crate) fn set_helper<T: ComponentId>(
    world: *mut sys::ecs_world_t,
    entity: u64,
    value: T,
    id: u64,
) {
    const {
        assert!(
            core::mem::size_of::<T>() != 0,
            "cannot set zero-sized-type / tag components"
        );
    };

    let world_ref = unsafe { WorldRef::from_ptr(world) };
    world_ref.check_thread_affinity_exclusive::<T>();
    // Shared-register write hook (spec §3.6, §7.1): defer behind a live guard.
    ensure_write_episode(&world_ref);

    unsafe {
        let res = sys::ecs_rust_set(
            world,
            entity,
            id,
            &value as *const _ as *const _,
            const { core::mem::size_of::<T>() },
        );
        assert!(
            !res.ptr.is_null(),
            "set failed: entity is not alive or the world is invalid"
        );
        let comp = res.ptr as *mut T;

        if T::NEEDS_DROP && !res.is_new {
            // Existing component: valid T in table (or deferred buffer) → must drop
            // before overwrite. New component (is_new=true): memory is either
            // uninitialized (non-deferred, ctor skipped) or a fresh cmd buffer
            // (deferred) → never drop on Rust side; flush handles the rest.
            core::ptr::drop_in_place(comp);
        }

        core::ptr::write(comp, value);

        if res.call_modified {
            sys::ecs_modified_id(world, entity, id);
        }
    }
}

pub(crate) fn assign_helper<T: ComponentId>(
    world: *mut sys::ecs_world_t,
    entity: sys::ecs_entity_t,
    value: T,
    id: sys::ecs_id_t,
) {
    const {
        assert!(
            core::mem::size_of::<T>() != 0,
            "cannot assign zero-sized-type / tag components"
        );
    };

    let world_ref = unsafe { WorldRef::from_ptr(world) };
    world_ref.check_thread_affinity_exclusive::<T>();
    // Shared-register write hook (spec §3.6, §7.1): defer behind a live guard.
    ensure_write_episode(&world_ref);

    let res = unsafe {
        sys::ecs_cpp_assign(
            world,
            entity,
            id,
            &value as *const _ as *const _,
            core::mem::size_of::<T>(),
        )
    };

    assert!(
        !res.ptr.is_null(),
        "assign failed: entity is not alive, does not have the component, or the world is invalid"
    );
    let dst = unsafe { &mut *(res.ptr as *mut T) };
    unsafe {
        core::ptr::drop_in_place(dst);
        core::ptr::write(dst, value);
    }

    if !res.stage.is_null() {
        unsafe { sys::flecs_defer_end(res.world, res.stage) };
    }

    if res.call_modified {
        unsafe { sys::ecs_modified_id(world, entity, id) };
    }
}

/// Remove generation from entity id.
///
/// # Arguments
///
/// * `entity`: The entity id to strip the generation from.
///
/// # Returns
///
/// * `sys::ecs_id_t`: The entity id with the generation removed.
#[inline(always)]
pub fn strip_generation(entity: impl Into<Entity>) -> u64 {
    unsafe { sys::ecs_strip_generation(*entity.into()) }
}

/// Get the generation from an entity id.
///
/// # Arguments
///
/// * `entity`: The entity id to get the generation from.
///
/// # Returns
///
/// * `u32`: The generation of the entity id.
#[inline(always)]
pub fn get_generation(entity: impl Into<Entity>) -> u32 {
    (*(entity.into() & sys::ECS_GENERATION_MASK) >> 32) as u32
}

#[inline(always)]
pub(crate) unsafe fn flecs_field_at<T>(it: *const sys::ecs_iter_t, index: i8, row: i32) -> *mut T {
    unsafe {
        let size = core::mem::size_of::<T>();
        sys::ecs_field_at_w_size(it, size, index, row) as *mut T
    }
}

/// Get the `OperKind` for the given type.
///
/// # Type Parameters
///
/// * `T`: The type to get the `OperKind` for.
#[expect(dead_code, reason = "possibly used in the future")]
pub(crate) fn type_to_oper<T: OperType>() -> OperKind {
    T::OPER
}

/// Sets the specified bit in the flags.
pub fn ecs_bit_set(flags: &mut u32, bit: u32) {
    *flags |= bit;
}

/// Clears the specified bit in the flags.
pub fn ecs_bit_clear(flags: &mut u32, bit: u32) {
    *flags &= !bit;
}

/// Conditionally sets or clears a bit in the flags based on a condition.
pub fn ecs_bit_cond(flags: &mut u32, bit: u32, cond: bool) {
    if cond {
        ecs_bit_set(flags, bit);
    } else {
        ecs_bit_clear(flags, bit);
    }
}

/// Copies the given Rust &str to a C string and returns a pointer to the C string.
/// this is intended to be used when the C code needs to take ownership of the string.
///
/// # Note
///
/// This function isn't being used anymore and might be removed in the future.
#[expect(dead_code, reason = "possibly used in the future")]
pub(crate) fn copy_and_allocate_c_char_from_rust_str(data: &str) -> *mut c_char {
    ecs_assert!(
        data.is_ascii(),
        FlecsErrorCode::InvalidParameter,
        "string must be ascii"
    );
    let bytes = data.as_bytes();
    let len = bytes.len() + 1; // +1 for the null terminator
    let memory_c_str = unsafe { sys::ecs_os_api.malloc_.unwrap()(len as i32) } as *mut u8;

    for (i, &byte) in bytes.iter().enumerate() {
        unsafe {
            memory_c_str.add(i).write(byte);
        }
    }

    // Write the null terminator to the end of the memory
    unsafe { memory_c_str.add(bytes.len()).write(0) };

    memory_c_str as *mut c_char
}

/// Prints the given C string to the console.
///
/// # Note
///
/// This function is for development purposes. It is not intended to be used in production code.
#[cfg(feature = "std")]
#[expect(dead_code, reason = "possibly used in the future")]
pub(crate) unsafe fn print_c_string(c_string: *const c_char) {
    unsafe {
        // Ensure the pointer is not null
        assert!(!c_string.is_null(), "Null pointer passed to print_c_string");

        // Create a CStr from the raw pointer
        let c_str = core::ffi::CStr::from_ptr(c_string);

        // Convert CStr to a Rust string slice (&str)
        // This can fail if the C string is not valid UTF-8, so handle errors appropriately
        #[allow(clippy::print_stdout)]
        match c_str.to_str() {
            Ok(s) => println!("{s}"),
            Err(_) => println!("Failed to convert C string to Rust string"),
        }
    }
}

/// Strips the given prefix from the given C string, returning a new C string with the prefix removed.
/// If the given C string does not start with the given prefix, returns `None`.
pub(crate) fn strip_prefix_str_raw<'a>(str: &'a str, prefix: &str) -> Option<&'a str> {
    let str_bytes = str.as_bytes();
    let prefix_bytes = prefix.as_bytes();

    if str_bytes.starts_with(prefix_bytes) {
        // SAFETY: We are slicing `cstr_bytes` which is guaranteed to be a valid
        // C string since it comes from a `&CStr`. We also check that it starts
        // with `prefix_bytes`, and we only slice off `prefix_bytes`, so the rest
        // remains a valid C string.
        Some(
            core::str::from_utf8(&str_bytes[prefix_bytes.len()..])
                .expect("error: pass valid utf strings"),
        )
    } else {
        None
    }
}

pub(crate) fn check_add_id_validity(world: *const sys::ecs_world_t, id: u64) {
    let is_valid_id = unsafe { sys::ecs_id_is_valid(world, id) };

    if !is_valid_id {
        panic!("Id is not a valid component, pair or entity.");
    }

    // SAFETY: `world` is a valid, live world pointer and `id` was just
    // validated above via `ecs_id_is_valid`, so `ecs_get_typeid` can be
    // called safely.
    let is_not_tag = unsafe { sys::ecs_get_typeid(world, id) != 0 };

    if is_not_tag {
        assert!(
            has_default_hook(world, id),
            "Id is not a zero-sized type (ZST) such as a Tag or Entity or does not implement the Default hook for a non ZST type. Default hooks are automatically implemented if the type has a Default trait."
        );
    }
}

#[inline(never)]
pub(crate) fn has_default_hook(world: *const sys::ecs_world_t, id: u64) -> bool {
    // SAFETY: `world` is a valid, live world pointer supplied by the caller and
    // `id` is a valid id (validated by callers such as `check_add_id_validity`
    // via `ecs_id_is_valid`), so `ecs_get_hooks_id` returns a valid, non-null
    // pointer to the type's hooks struct.
    let hooks = unsafe { sys::ecs_get_hooks_id(world, id) };
    // SAFETY: `hooks` was just returned by `ecs_get_hooks_id` for a valid world
    // and id, so it points to a live, initialized `ecs_type_hooks_t`.
    let ctor_hooks =
        unsafe { (*hooks).ctor }.expect("ctor hook is always implemented, either in Rust or C");

    /// Type alias for extern function pointers that adapts to target platform
    #[cfg(target_family = "wasm")]
    type ExternDefaultCtorFn =
        unsafe extern "C" fn(*mut core::ffi::c_void, i32, *const sys::ecs_type_info_t);
    #[cfg(not(target_family = "wasm"))]
    type ExternDefaultCtorFn =
        unsafe extern "C-unwind" fn(*mut core::ffi::c_void, i32, *const sys::ecs_type_info_t);

    !core::ptr::fn_addr_eq(ctor_hooks, sys::flecs_default_ctor as ExternDefaultCtorFn)
}

/// Separate the types of an `Archetype` into a `Vec<String>`.
///
/// # Returns
/// A `Vec<String>` where each entry is a component or relationship of the `Archetype`.
pub fn debug_separate_archetype_types_into_strings(archetype: &Archetype) -> Vec<String> {
    let mut result = Vec::with_capacity(archetype.count());
    let mut skip_next = false; // To skip the next part after joining
    let archetype_str = archetype
        .to_string()
        .unwrap_or_else(|| "empty entity | no components".to_string());

    if archetype.count() == 0 {
        return vec![archetype_str];
    }

    let parts: Vec<&str> = archetype_str.split(',').map(str::trim).collect();

    let ids = archetype.as_slice();
    let mut i_ids = 0;

    for i in 0..parts.len() {
        if skip_next {
            skip_next = false;
            continue;
        }

        let part = parts[i];
        let id = ids[i_ids];

        if part.starts_with('(') {
            // Join this part with the next one
            let combined = format!("{part}, {} : {id}", parts[i + 1]);
            result.push(combined);
            skip_next = true; // Skip the next part since it's already used
        } else {
            result.push(format!("{part} : {id}"));
        }
        i_ids += 1;
    }

    result
}

#[cfg(test)]
mod tests {

    use super::get_type_name_without_scope_generic;

    struct MyStruct;
    #[allow(dead_code)] //used in test
    enum MyEnum {
        A,
        B,
    }

    #[test]
    fn simple_type() {
        assert_eq!(get_type_name_without_scope_generic::<i32>(), "i32");
        assert_eq!(get_type_name_without_scope_generic::<bool>(), "bool");
    }

    #[test]
    fn single_generic() {
        assert_eq!(
            get_type_name_without_scope_generic::<Vec<String>>(),
            "Vec<String>"
        );
        assert_eq!(
            get_type_name_without_scope_generic::<Option<u8>>(),
            "Option<u8>"
        );
    }

    #[test]
    fn multi_generic() {
        assert_eq!(
            get_type_name_without_scope_generic::<Result<i32, f64>>(),
            "Result<i32, f64>"
        );
    }

    #[test]
    fn nested_generics() {
        type Deep = Option<Result<Vec<MyStruct>, MyEnum>>;
        assert_eq!(
            get_type_name_without_scope_generic::<Deep>(),
            "Option<Result<Vec<MyStruct>, MyEnum>>"
        );
    }

    #[test]
    fn custom_struct_and_enum() {
        assert_eq!(
            get_type_name_without_scope_generic::<MyStruct>(),
            "MyStruct"
        );
        assert_eq!(get_type_name_without_scope_generic::<MyEnum>(), "MyEnum");
    }

    #[test]
    fn pointer_and_reference() {
        assert_eq!(get_type_name_without_scope_generic::<&str>(), "&str");
        assert_eq!(
            get_type_name_without_scope_generic::<*const i32>(),
            "*const i32"
        );
    }

    // nested modules used to exercise path stripping
    mod outer {
        pub mod inner {
            pub struct Deep;
            pub struct Wrap<T>(pub T);
            #[allow(dead_code)] //used in test
            pub enum E {
                A,
                B,
            }
        }
    }

    mod a {
        pub mod b {
            pub mod c {
                pub struct Z;
            }
        }
    }

    #[test]
    fn nested_modules_simple() {
        assert_eq!(
            get_type_name_without_scope_generic::<outer::inner::Deep>(),
            "Deep"
        );
        assert_eq!(get_type_name_without_scope_generic::<a::b::c::Z>(), "Z");
        assert_eq!(
            get_type_name_without_scope_generic::<outer::inner::E>(),
            "E"
        );
    }

    #[test]
    fn nested_modules_with_generics() {
        type T1 = outer::inner::Wrap<outer::inner::Deep>;
        type T2 = outer::inner::Wrap<outer::inner::Wrap<outer::inner::Deep>>;
        assert_eq!(get_type_name_without_scope_generic::<T1>(), "Wrap<Deep>");
        assert_eq!(
            get_type_name_without_scope_generic::<T2>(),
            "Wrap<Wrap<Deep>>"
        );
    }

    #[test]
    fn long_std_path_nested_generics() {
        type LongNested = ::alloc::collections::BTreeMap<String, Vec<Vec<String>>>;
        assert_eq!(
            get_type_name_without_scope_generic::<LongNested>(),
            "BTreeMap<String, Vec<Vec<String>>>"
        );
    }
}
