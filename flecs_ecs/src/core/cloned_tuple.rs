#![allow(unused)]

use core::ffi::c_void;
use core::marker::PhantomData;

use crate::core::*;
use crate::sys;
use flecs_ecs_derive::tuples;
use sys::ecs_record_t;

pub struct ComponentsData<T: ClonedTuple, const LEN: usize> {
    pub array_components: [*mut c_void; LEN],
    pub has_all_components: bool,
    #[cfg(feature = "flecs_safety_locks")]
    pub(crate) safety_info: [LockInfo; LEN],
    _marker: PhantomData<T>,
}

pub trait ClonedComponentPointers<T: ClonedTuple> {
    /// # Safety
    ///
    /// The caller must ensure that `record` is the entity record for `entity`
    /// as returned by `ecs_record_find` on the same world.
    unsafe fn new<'a, const SHOULD_PANIC: bool>(
        world: impl WorldProvider<'a>,
        entity: Entity,
        record: *const ecs_record_t,
    ) -> Self;

    fn new_singleton<'a, const SHOULD_PANIC: bool>(world: impl WorldProvider<'a>) -> Self;

    fn get_tuple<'a>(&self) -> T::TupleType<'a>;

    fn has_all_components(&self) -> bool;

    fn component_ptrs(&self) -> &[*mut c_void];

    #[cfg(feature = "flecs_safety_locks")]
    fn safety_info(&self) -> &[LockInfo];
}

impl<T: ClonedTuple, const LEN: usize> ClonedComponentPointers<T> for ComponentsData<T, LEN> {
    unsafe fn new<'a, const SHOULD_PANIC: bool>(
        world: impl WorldProvider<'a>,
        entity: Entity,
        record: *const ecs_record_t,
    ) -> Self {
        let mut array_components = [core::ptr::null::<c_void>() as *mut c_void; LEN];

        #[cfg(feature = "flecs_safety_locks")]
        let mut safety_info = [LockInfo::default(); LEN];

        // SAFETY: same contract as this function — record is the entity's
        // record from the same world, guaranteed by the caller.
        let has_all_components = unsafe {
            T::populate_array_ptrs::<SHOULD_PANIC>(
                world,
                entity,
                record,
                &mut array_components[..],
                #[cfg(feature = "flecs_safety_locks")]
                &mut safety_info,
            )
        };

        Self {
            array_components,
            has_all_components,
            #[cfg(feature = "flecs_safety_locks")]
            safety_info,
            _marker: PhantomData::<T>,
        }
    }

    fn new_singleton<'a, const SHOULD_PANIC: bool>(world: impl WorldProvider<'a>) -> Self {
        let mut array_components = [core::ptr::null::<c_void>() as *mut c_void; LEN];

        #[cfg(feature = "flecs_safety_locks")]
        let mut safety_info = [LockInfo::default(); LEN];

        let has_all_components = T::populate_array_ptrs_singleton::<SHOULD_PANIC>(
            world,
            &mut array_components[..],
            #[cfg(feature = "flecs_safety_locks")]
            &mut safety_info,
        );

        Self {
            array_components,
            has_all_components,
            #[cfg(feature = "flecs_safety_locks")]
            safety_info,
            _marker: PhantomData::<T>,
        }
    }

    fn get_tuple<'a>(&self) -> T::TupleType<'a> {
        T::create_tuple(&self.array_components[..])
    }

    fn has_all_components(&self) -> bool {
        self.has_all_components
    }

    fn component_ptrs(&self) -> &[*mut c_void] {
        &self.array_components
    }

    #[cfg(feature = "flecs_safety_locks")]
    fn safety_info(&self) -> &[LockInfo] {
        &self.safety_info
    }
}

pub trait ClonedTupleTypeOperation {
    type ActualType: Clone;
    type OnlyType: ComponentOrPairId;
    const IS_OPTION: bool;

    fn create_tuple_data(array_components_data: *mut c_void) -> Self::ActualType;
}

impl<T> ClonedTupleTypeOperation for &T
where
    T: ComponentOrPairId,
    <T as ComponentOrPairId>::CastType: Clone,
{
    type ActualType = <T as ComponentOrPairId>::CastType;
    type OnlyType = T;
    const IS_OPTION: bool = false;

    fn create_tuple_data(array_components_data: *mut c_void) -> Self::ActualType {
        let data_ptr = array_components_data as *const <T as ComponentOrPairId>::CastType;
        // SAFETY: up to this point we have checked that the data is not null
        unsafe { (*data_ptr).clone() }
    }
}

impl<T> ClonedTupleTypeOperation for Option<&T>
where
    T: ComponentOrPairId,
    <T as ComponentOrPairId>::CastType: Clone,
{
    type ActualType = Option<<T as ComponentOrPairId>::CastType>;
    type OnlyType = T;
    const IS_OPTION: bool = true;

    fn create_tuple_data(array_components_data: *mut c_void) -> Self::ActualType {
        if array_components_data.is_null() {
            None
        } else {
            let data_ptr = array_components_data as *const <T as ComponentOrPairId>::CastType;
            Some(unsafe { (*data_ptr).clone() })
        }
    }
}

#[diagnostic::on_unimplemented(
    message = "`there is a problem with {Self}`. Please double check the signature and if the types implement clone and are not empty components (tags) or the relationship are not made of 2 tags",
    label = "Failure in cloned signature",
    note = "Valid syntax: `.cloned::<(Position,)>()` -- single component",
    note = "Valid syntax: `.cloned::<(Position, Velocity)>()` -- multiple components",
    note = "Valid syntax: `.cloned::<((Position, Velocity))>()` -- relationship",
    note = "Valid syntax: `.cloned::<((Position, Velocity),Mass, etc)>()` -- multiple components: relationship and component"
)]
pub trait ClonedTuple: Sized {
    type Pointers: ClonedComponentPointers<Self>;
    type TupleType<'a>;

    /// # Safety
    ///
    /// The caller must ensure that `record` is the entity record for `entity`
    /// as returned by `ecs_record_find` on the same world.
    unsafe fn create_ptrs<'a, const SHOULD_PANIC: bool>(
        world: impl WorldProvider<'a>,
        entity: Entity,
        record: *const ecs_record_t,
    ) -> Self::Pointers {
        // SAFETY: same contract as this function, guaranteed by the caller.
        unsafe { Self::Pointers::new::<'a, SHOULD_PANIC>(world, entity, record) }
    }

    fn create_ptrs_singleton<'a, const SHOULD_PANIC: bool>(
        world: impl WorldProvider<'a>,
    ) -> Self::Pointers {
        Self::Pointers::new_singleton::<'a, SHOULD_PANIC>(world)
    }

    /// # Safety
    ///
    /// The caller must ensure that `world_ptr` points to a valid world and
    /// that `record` is the (non-null) entity record for `entity` obtained
    /// from that same world.
    #[inline(always)]
    unsafe fn internal_populate_array_ptrs<
        'a,
        const SHOULD_PANIC: bool,
        T: ClonedTupleTypeOperation,
    >(
        world: &WorldRef<'a>,
        world_ptr: *mut sys::ecs_world_t,
        entity: u64,
        record: *const ecs_record_t,
        id: u64,
        components: &mut [*mut c_void],
        has_all_components: &mut bool,
        index: usize,
        #[cfg(feature = "flecs_safety_locks")] safety_info: &mut [LockInfo],
    ) {
        world.check_thread_affinity_shared::<<T::OnlyType as ComponentOrPairId>::CastType>();

        let get_ptr = if const {
            // mirrors the C++ `is_get_sparse_component` fast path, see
            // `get_tuple.rs` for details.
            (<T::OnlyType as ComponentOrPairId>::CastType::IS_SPARSE
                || <T::OnlyType as ComponentOrPairId>::CastType::IS_DONT_FRAGMENT)
                && !matches!(
                    <T::OnlyType as ComponentOrPairId>::CastType::ON_INSTANTIATE,
                    OnInstantiatePolicy::Inherit
                )
                && !<T::OnlyType as ComponentOrPairId>::IS_PAIR
        } {
            unsafe {
                sys::ecs_rust_get_sparse_id(
                    world_ptr,
                    entity,
                    id,
                    core::mem::size_of::<<T::OnlyType as ComponentOrPairId>::CastType>(),
                )
            }
        } else {
            unsafe { sys::ecs_rust_record_get_id(world_ptr, entity, record, id) }
        };
        let component_ptr = get_ptr_raw(&get_ptr);

        if <T::OnlyType as ComponentOrPairId>::IS_PAIR {
            assert!(
                {
                    let first = ecs_first(id, world);
                    first != flecs::Wildcard::ID && first != flecs::Any::ID
                },
                "Pair with flecs::Wildcard or flecs::Any as first terms are not supported"
            );

            assert!(
                {
                    let id = unsafe { sys::ecs_get_typeid(world_ptr, id) };
                    let cast_id =
                        world.component_id::<<T::OnlyType as ComponentOrPairId>::CastType>();
                    id != 0 && id == cast_id
                },
                "Pair is not a (data) component. Possible cause: PairIsTag trait or cast type is not the same as the pair due to flecs::Wildcard or flecs::Any"
            );
        }

        if component_ptr.is_null() {
            components[index] = core::ptr::null_mut();
            if !T::IS_OPTION {
                if SHOULD_PANIC {
                    ecs_assert!(false, FlecsErrorCode::OperationFailed,
                    "Component `{}` not found on `EntityView::cloned` operation
                    with parameters: `{}`.
                    Use `try_cloned` variant to avoid assert/panicking if you want to handle the error
                    or use `Option<{}> instead to handle individual cases.",
                    core::any::type_name::<T::OnlyType>(), core::any::type_name::<Self>(), core::any::type_name::<T::ActualType>());
                    panic!(
                        "Component `{}` not found on `EntityView::cloned` operation
                with parameters: `{}`.
                Use `try_cloned` variant to avoid assert/panicking if
                you want to handle the error or use `Option<{}>
                instead to handle individual cases.",
                        core::any::type_name::<T::OnlyType>(),
                        core::any::type_name::<Self>(),
                        core::any::type_name::<T::ActualType>()
                    );
                }
                *has_all_components = false;
            }
        } else {
            components[index] = component_ptr;
            #[cfg(feature = "flecs_safety_locks")]
            {
                safety_info[index] = LockInfo {
                    key: get_ptr.lock_key,
                    id,
                };
            }
        }
    }

    /// # Safety
    ///
    /// The caller must ensure that `record` is the entity record for `entity`
    /// as returned by `ecs_record_find` on the same world.
    unsafe fn populate_array_ptrs<'a, const SHOULD_PANIC: bool>(
        world: impl WorldProvider<'a>,
        entity: Entity,
        record: *const ecs_record_t,
        components: &mut [*mut c_void],
        #[cfg(feature = "flecs_safety_locks")] safety_info: &mut [LockInfo],
    ) -> bool;

    fn populate_array_ptrs_singleton<'a, const SHOULD_PANIC: bool>(
        world: impl WorldProvider<'a>,
        components: &mut [*mut c_void],
        #[cfg(feature = "flecs_safety_locks")] safety_info: &mut [LockInfo],
    ) -> bool;

    fn create_tuple<'a>(array_components: &[*mut c_void]) -> Self::TupleType<'a>;
}

/////////////////////
// The higher sized tuples are done by a macro towards the bottom of this file.
/////////////////////

#[rustfmt::skip]
impl<A> ClonedTuple for A
where
    A: ClonedTupleTypeOperation,
{
    type Pointers = ComponentsData<A, 1>;
    type TupleType<'e> = A::ActualType;

    unsafe fn populate_array_ptrs<'a, const SHOULD_PANIC: bool>(
        world: impl WorldProvider<'a>,
        entity: Entity,
        record: *const ecs_record_t,
        components: &mut [*mut c_void],
        #[cfg(feature = "flecs_safety_locks")] safety_info: &mut [LockInfo],
    ) -> bool {
        let world_ref = world.world();
        let world_ptr = unsafe {
            sys::ecs_get_world(world_ref.ptr_mut() as *const c_void) as *mut sys::ecs_world_t
        };
        let entity = *entity;
        let mut has_all_components = true;

        let id = <A::OnlyType as ComponentOrPairId>::get_id(world_ref);

        // SAFETY: world_ptr comes from the live world above; record is
        // the caller-provided record for entity from that world.
        unsafe {
            Self::internal_populate_array_ptrs::<SHOULD_PANIC, A>(
                &world_ref,
                world_ptr,
                entity,
                record,
                id,
                components,
                &mut has_all_components,
                0,
                #[cfg(feature = "flecs_safety_locks")]
                safety_info,
            );
        }

        has_all_components
    }

    fn populate_array_ptrs_singleton<'a, const SHOULD_PANIC: bool>(
    world: impl WorldProvider<'a>,
    components: &mut [*mut c_void],
    #[cfg(feature = "flecs_safety_locks")] safety_info: &mut [LockInfo],
    ) -> bool {
        let world_ref = world.world();
        let world_ptr = unsafe {
            sys::ecs_get_world(world_ref.ptr_mut() as *const c_void) as *mut sys::ecs_world_t
        };
        let entity = <<A::OnlyType as ComponentOrPairId>::First>::entity_id(world);
        let record = unsafe { sys::ecs_record_find(world_ptr, entity) };
        let id = <A::OnlyType as ComponentOrPairId>::get_id(world_ref);
        let mut has_all_components = true;


        // SAFETY: world_ptr comes from the live world above; record was
        // just looked up for entity in that same world.
        unsafe {
            Self::internal_populate_array_ptrs::<SHOULD_PANIC, A>(
                &world_ref,
                world_ptr,
                entity,
                record,
                id,
                components,
                &mut has_all_components,
                0,
                #[cfg(feature = "flecs_safety_locks")]
                safety_info,
            );
        }

        has_all_components
    }

    fn create_tuple<'a>(array_components: &[*mut c_void]) -> Self::TupleType<'a> {
        A::create_tuple_data(array_components[0])
    }
}

pub struct Wrapper<T>(T);

pub trait TupleForm<'a, T, U> {
    type Tuple;
    const IS_OPTION: bool;

    fn return_type_for_tuple(array: *mut U, index: usize) -> Self::Tuple;
}

impl<'a, T: 'a> TupleForm<'a, T, T> for Wrapper<T> {
    type Tuple = &'a mut T;
    const IS_OPTION: bool = false;

    #[inline(always)]
    fn return_type_for_tuple(array: *mut T, index: usize) -> Self::Tuple {
        unsafe { &mut (*array.add(index)) }
    }
}

impl<'a, T: 'a> TupleForm<'a, Option<T>, T> for Wrapper<T> {
    type Tuple = Option<&'a mut T>;
    const IS_OPTION: bool = true;

    #[inline(always)]
    fn return_type_for_tuple(array: *mut T, index: usize) -> Self::Tuple {
        unsafe {
            if array.is_null() {
                None
            } else {
                Some(&mut (*array.add(index)))
            }
        }
    }
}

macro_rules! tuple_count {
    () => { 0 };
    ($head:ident) => { 1 };
    ($head:ident, $($tail:ident),*) => { 1 + tuple_count!($($tail),*) };
}

macro_rules! impl_cloned_tuple {
    ($($t:ident),*) => {
        impl<$($t: ClonedTupleTypeOperation),*> ClonedTuple for ($($t,)*) {
            type TupleType<'e> = ($(
                $t::ActualType,
            )*);

            type Pointers = ComponentsData<Self, { tuple_count!($($t),*) }>;

            #[allow(unused)]
            unsafe fn populate_array_ptrs<'a, const SHOULD_PANIC: bool>(
                world: impl WorldProvider<'a>, entity: Entity, record: *const ecs_record_t, components: &mut [*mut c_void],
                #[cfg(feature = "flecs_safety_locks")] safety_info: &mut [LockInfo]
            ) -> bool {

                let world_ref = world.world();
                let world_ptr = unsafe { sys::ecs_get_world(world_ref.ptr_mut() as *const c_void) as *mut sys::ecs_world_t };
                let entity = *entity;
                let mut index : usize = 0;
                let mut has_all_components = true;

                $(
                    let id = <$t::OnlyType as ComponentOrPairId>::get_id(world_ref);

                    // SAFETY: world_ptr comes from the live world above; record is
                    // the caller-provided record for entity from that world.
                    unsafe {
                        Self::internal_populate_array_ptrs::<SHOULD_PANIC, $t>(&world_ref, world_ptr, entity, record, id, components, &mut has_all_components, index,
                            #[cfg(feature = "flecs_safety_locks")]
                            safety_info);
                    }

                    index += 1;
                )*

                has_all_components
            }

            #[allow(unused)]
            fn populate_array_ptrs_singleton<'a, const SHOULD_PANIC: bool>(
                world: impl WorldProvider<'a>, components: &mut [*mut c_void],
                #[cfg(feature = "flecs_safety_locks")] safety_info: &mut [LockInfo]
            ) -> bool {
                let world_ref = world.world();
                let world_ptr = unsafe { sys::ecs_get_world(world_ref.ptr_mut() as *const c_void) as *mut sys::ecs_world_t };
                let mut index : usize = 0;
                let mut has_all_components = true;

                $(
                    let entity = <<$t::OnlyType as ComponentOrPairId>::First>::entity_id(world_ref);
                    let record = unsafe { sys::ecs_record_find(world_ptr, entity) };
                    let id = <$t::OnlyType as ComponentOrPairId>::get_id(world_ref);

                    // SAFETY: world_ptr comes from the live world above; record was
                    // just looked up for entity in that same world.
                    unsafe {
                        Self::internal_populate_array_ptrs::<SHOULD_PANIC, $t>(&world_ref, world_ptr, entity, record, id, components, &mut has_all_components, index,
                            #[cfg(feature = "flecs_safety_locks")]
                            safety_info);
                    }

                    index += 1;
                )*

                has_all_components

            }

            #[allow(unused, clippy::unused_unit)]
            fn create_tuple<'a>(array_components: &[*mut c_void]) -> Self::TupleType<'a> {
                let mut column: isize = -1;
                ($({
                    column += 1;
                    $t::create_tuple_data(array_components[column as usize])
                },)*)
            }

        }
    }
}

tuples!(impl_cloned_tuple, 0, 32);
