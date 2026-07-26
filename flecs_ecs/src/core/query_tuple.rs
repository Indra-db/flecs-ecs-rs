use core::marker::PhantomData;

use crate::core::*;
use crate::sys;
use flecs_ecs_derive::tuples;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[doc(hidden)]
pub struct IsAnyArray {
    pub a_ref: bool, //e.g. singleton, prefab inheritance
    pub a_row: bool, //e.g. sparse, non_fragmenting
}

#[cfg(feature = "flecs_safety_locks")]
#[derive(Debug, Clone, Copy)]
pub struct TableColumnSafety {
    //only set for sparse terms
    pub component_id: u64,
    pub(crate) table: *mut sys::ecs_table_t,
    pub column: i16,
}

#[cfg(feature = "flecs_safety_locks")]
impl Default for TableColumnSafety {
    fn default() -> Self {
        Self {
            component_id: 0,
            table: core::ptr::null_mut(),
            column: -1,
        }
    }
}

pub struct ComponentsData<T: QueryTuple, const LEN: usize> {
    pub(crate) array_components: [*mut u8; LEN],
    pub is_ref_array_components: [bool; LEN],
    pub is_row_array_components: [bool; LEN],
    pub index_array_components: [i8; LEN],
    #[cfg(feature = "flecs_safety_locks")]
    pub safety_table_records: [TableColumnSafety; LEN],
    _marker: PhantomData<T>,
}

pub trait ComponentPointers<T: QueryTuple> {
    fn new(iter: &sys::ecs_iter_t) -> (IsAnyArray, Self);

    fn get_tuple(&mut self, index: usize) -> T::TupleType<'_>;

    fn get_tuple_with_row(
        &mut self,
        iter: &sys::ecs_iter_t,
        index_row_entity: usize,
    ) -> T::TupleType<'_>;

    fn get_tuple_with_ref(&mut self, index: usize) -> T::TupleType<'_>;

    /// Per-term column base pointers for the current table batch, in term order.
    /// Used by the experimental chunk cursor to build whole-column slices.
    fn column_ptrs(&self) -> &[*mut u8];

    #[cfg(feature = "flecs_safety_locks")]
    fn safety_table_records(&self) -> &[TableColumnSafety];
}

impl<T: QueryTuple, const LEN: usize> ComponentPointers<T> for ComponentsData<T, LEN> {
    #[inline(always)]
    fn new(iter: &sys::ecs_iter_t) -> (IsAnyArray, Self) {
        let mut array_components = [core::ptr::null::<u8>() as *mut u8; LEN];
        #[cfg(feature = "flecs_safety_locks")]
        let mut safety_table_records = [TableColumnSafety::default(); LEN];

        // Fast path: every field is a plain dense column of `it.table`. Only
        // `array_components` (and the lock records) are read downstream, so the
        // is_ref/is_row/index arrays are left dead here; inlining lets the
        // caller DCE them once the plain dispatch is selected.
        if (iter.ref_fields | iter.up_fields) == 0 {
            T::populate_self_array_ptrs(
                iter,
                &mut array_components[..],
                #[cfg(feature = "flecs_safety_locks")]
                &mut safety_table_records[..],
            );
            return (
                IsAnyArray {
                    a_ref: false,
                    a_row: false,
                },
                Self {
                    array_components,
                    is_ref_array_components: [false; LEN],
                    is_row_array_components: [false; LEN],
                    index_array_components: [0; LEN],
                    #[cfg(feature = "flecs_safety_locks")]
                    safety_table_records,
                    _marker: PhantomData::<T>,
                },
            );
        }

        let mut is_ref_array_components = [false; LEN];
        let mut is_row_array_components = [false; LEN];
        let mut index_array_components = [0; LEN];
        let is_any_array = T::populate_array_ptrs(
            iter,
            &mut array_components[..],
            &mut is_ref_array_components[..],
            &mut is_row_array_components[..],
            &mut index_array_components[..],
            #[cfg(feature = "flecs_safety_locks")]
            &mut safety_table_records[..],
        );

        (
            is_any_array,
            Self {
                array_components,
                is_ref_array_components,
                is_row_array_components,
                index_array_components,
                #[cfg(feature = "flecs_safety_locks")]
                safety_table_records,
                _marker: PhantomData::<T>,
            },
        )
    }

    #[inline(always)]
    fn get_tuple(&mut self, index: usize) -> T::TupleType<'_> {
        T::create_tuple(&self.array_components[..], index)
    }

    fn get_tuple_with_row(
        &mut self,
        iter: &sys::ecs_iter_t,
        index_row_entity: usize,
    ) -> T::TupleType<'_> {
        T::create_tuple_with_row(
            iter,
            &mut self.array_components[..],
            &self.is_ref_array_components[..],
            &self.is_row_array_components[..],
            &self.index_array_components[..],
            index_row_entity,
        )
    }

    fn get_tuple_with_ref(&mut self, index: usize) -> T::TupleType<'_> {
        T::create_tuple_with_ref(
            &self.array_components[..],
            &self.is_ref_array_components[..],
            index,
        )
    }

    #[inline(always)]
    fn column_ptrs(&self) -> &[*mut u8] {
        &self.array_components[..]
    }

    #[cfg(feature = "flecs_safety_locks")]
    fn safety_table_records(&self) -> &[TableColumnSafety] {
        &self.safety_table_records[..]
    }
}

pub trait IterableTypeOperation {
    type CastType;
    type ActualType<'w>;
    type SliceType<'w>;
    type OnlyType: ComponentOrPairId;
    type OnlyPairType: ComponentId;
    const IS_IMMUTABLE: bool;
    const IS_OPTIONAL: bool;

    fn populate_term(term: &mut sys::ecs_term_t);

    fn create_tuple_data<'a>(array_components_data: *mut u8, index: usize) -> Self::ActualType<'a>;

    fn create_tuple_with_ref_data<'a>(
        array_components_data: *mut u8,
        is_ref: bool,
        index: usize,
    ) -> Self::ActualType<'a>;
}

impl<T> IterableTypeOperation for &T
where
    T: ComponentOrPairId,
{
    type CastType = *const <T as ComponentOrPairId>::CastType;
    type ActualType<'w> = &'w <T as ComponentOrPairId>::CastType;
    type SliceType<'w> = &'w [<T as ComponentOrPairId>::CastType];
    type OnlyType = T;
    type OnlyPairType = <T as ComponentOrPairId>::CastType;
    const IS_IMMUTABLE: bool = true;
    const IS_OPTIONAL: bool = false;

    #[inline(always)]
    fn populate_term(term: &mut sys::ecs_term_t) {
        term.inout = InOutKind::In as i16;
    }

    #[inline(always)]
    fn create_tuple_data<'a>(array_components_data: *mut u8, index: usize) -> Self::ActualType<'a> {
        let data_ptr = array_components_data as Self::CastType;
        unsafe { &*data_ptr.add(index) }
    }

    #[inline(always)]
    fn create_tuple_with_ref_data<'a>(
        array_components_data: *mut u8,
        is_ref: bool,
        index: usize,
    ) -> Self::ActualType<'a> {
        let data_ptr = array_components_data as Self::CastType;
        unsafe {
            if is_ref {
                &*data_ptr.add(0)
            } else {
                &*data_ptr.add(index)
            }
        }
    }
}

impl<T> IterableTypeOperation for &mut T
where
    T: ComponentOrPairId,
{
    type CastType = *mut <T as ComponentOrPairId>::CastType;
    type ActualType<'w> = &'w mut <T as ComponentOrPairId>::CastType;
    type SliceType<'w> = &'w mut [<T as ComponentOrPairId>::CastType];
    type OnlyType = T;
    type OnlyPairType = <T as ComponentOrPairId>::CastType;
    const IS_IMMUTABLE: bool = false;
    const IS_OPTIONAL: bool = false;

    #[inline(always)]
    fn populate_term(term: &mut sys::ecs_term_t) {
        term.inout = InOutKind::InOut as i16;
    }

    #[inline(always)]
    fn create_tuple_data<'a>(array_components_data: *mut u8, index: usize) -> Self::ActualType<'a> {
        let data_ptr = array_components_data as Self::CastType;
        unsafe { &mut *data_ptr.add(index) }
    }

    #[inline(always)]
    fn create_tuple_with_ref_data<'a>(
        array_components_data: *mut u8,
        is_ref: bool,
        index: usize,
    ) -> Self::ActualType<'a> {
        let data_ptr = array_components_data as Self::CastType;
        unsafe {
            if is_ref {
                &mut *data_ptr.add(0)
            } else {
                &mut *data_ptr.add(index)
            }
        }
    }
}

impl<T> IterableTypeOperation for Option<&T>
where
    T: ComponentOrPairId,
{
    type CastType = *const <T as ComponentOrPairId>::CastType;
    type ActualType<'w> = Option<&'w <T as ComponentOrPairId>::CastType>;
    type SliceType<'w> = Option<&'w [<T as ComponentOrPairId>::CastType]>;
    type OnlyType = T;
    type OnlyPairType = <T as ComponentOrPairId>::CastType;
    const IS_IMMUTABLE: bool = true;
    const IS_OPTIONAL: bool = true;

    fn populate_term(term: &mut sys::ecs_term_t) {
        term.inout = InOutKind::In as i16;
        term.oper = OperKind::Optional as i16;
    }

    #[inline(always)]
    fn create_tuple_data<'a>(array_components_data: *mut u8, index: usize) -> Self::ActualType<'a> {
        let data_ptr = array_components_data as Self::CastType;
        if data_ptr.is_null() {
            None
        } else {
            Some(unsafe { &*data_ptr.add(index) })
        }
    }

    #[inline(always)]
    fn create_tuple_with_ref_data<'a>(
        array_components_data: *mut u8,
        is_ref: bool,
        index: usize,
    ) -> Self::ActualType<'a> {
        let data_ptr = array_components_data as Self::CastType;
        if data_ptr.is_null() {
            None
        } else if is_ref {
            Some(unsafe { &*data_ptr.add(0) })
        } else {
            Some(unsafe { &*data_ptr.add(index) })
        }
    }
}

impl<T> IterableTypeOperation for Option<&mut T>
where
    T: ComponentOrPairId,
{
    type CastType = *mut <T as ComponentOrPairId>::CastType;
    type ActualType<'w> = Option<&'w mut <T as ComponentOrPairId>::CastType>;
    type SliceType<'w> = Option<&'w mut [<T as ComponentOrPairId>::CastType]>;
    type OnlyType = T;
    type OnlyPairType = <T as ComponentOrPairId>::CastType;
    const IS_IMMUTABLE: bool = false;
    const IS_OPTIONAL: bool = true;

    #[inline(always)]
    fn populate_term(term: &mut sys::ecs_term_t) {
        term.inout = InOutKind::InOut as i16;
        term.oper = OperKind::Optional as i16;
    }

    #[inline(always)]
    fn create_tuple_data<'a>(array_components_data: *mut u8, index: usize) -> Self::ActualType<'a> {
        let data_ptr = array_components_data as Self::CastType;
        if data_ptr.is_null() {
            None
        } else {
            Some(unsafe { &mut *data_ptr.add(index) })
        }
    }

    #[inline(always)]
    fn create_tuple_with_ref_data<'a>(
        array_components_data: *mut u8,
        is_ref: bool,
        index: usize,
    ) -> Self::ActualType<'a> {
        let data_ptr = array_components_data as Self::CastType;
        if data_ptr.is_null() {
            None
        } else if is_ref {
            Some(unsafe { &mut *data_ptr.add(0) })
        } else {
            Some(unsafe { &mut *data_ptr.add(index) })
        }
    }
}

pub trait QueryTuple: Sized {
    type Pointers: ComponentPointers<Self>;
    type TupleType<'a>;
    const CONTAINS_ANY_TAG_TERM: bool;
    /// Whether every term declares the `DontFragment` trait at compile time
    /// (`#[flecs(traits(DontFragment))]`), no term is a tag, and no term
    /// declares the `(OnInstantiate, Inherit)` policy. Equivalent to the C++
    /// `is_sparse_query` trait; used to validate
    /// [`SparseQuery`][crate::core::SparseQuery] at compile time.
    const IS_SPARSE_QUERY: bool;
    const COUNT: i32;
    const COUNT_IMMUTABLE: usize;
    const COUNT_MUTABLE: usize;
    const COUNT_OPTIONAL_IMMUTABLE: usize;
    const COUNT_OPTIONAL_MUTABLE: usize;

    #[inline(always)]
    fn create_ptrs(iter: &sys::ecs_iter_t) -> (IsAnyArray, Self::Pointers) {
        Self::Pointers::new(iter)
    }

    fn populate<'a>(query: &mut impl QueryBuilderImpl<'a>);

    #[inline(always)]
    fn register_ids_descriptor(world: *mut sys::ecs_world_t, desc: &mut sys::ecs_query_desc_t) {
        Self::register_ids_descriptor_at(world, &mut desc.terms[..], &mut 0);
    }

    fn register_ids_descriptor_at(
        world: *mut sys::ecs_world_t,
        terms: &mut [sys::ecs_term_t],
        index: &mut usize,
    );

    fn populate_array_ptrs(
        it: &sys::ecs_iter_t,
        components: &mut [*mut u8],
        is_ref: &mut [bool],
        is_row: &mut [bool],
        indexes: &mut [i8],
        #[cfg(feature = "flecs_safety_locks")] table_records: &mut [TableColumnSafety],
    ) -> IsAnyArray;

    fn populate_self_array_ptrs(
        it: &sys::ecs_iter_t,
        components: &mut [*mut u8],
        #[cfg(feature = "flecs_safety_locks")] table_records: &mut [TableColumnSafety],
    );

    fn create_tuple(array_components: &[*mut u8], index: usize) -> Self::TupleType<'_>;

    fn create_tuple_with_ref<'a>(
        array_components: &'a [*mut u8],
        is_ref_array_components: &[bool],
        index: usize,
    ) -> Self::TupleType<'a>;

    fn create_tuple_with_row<'a>(
        iter: &sys::ecs_iter_t,
        array_components: &'a mut [*mut u8],
        is_ref_array_components: &[bool],
        is_row_array_components: &[bool],
        indexes_array_components: &[i8],
        index_row_entity: usize,
    ) -> Self::TupleType<'a>;
}

/////////////////////
// The higher sized tuples are done by a macro towards the bottom of this file.
/////////////////////

#[rustfmt::skip]
impl<A> QueryTuple for A
where
    A: IterableTypeOperation,
{
    type Pointers = ComponentsData<A, 1>;
    type TupleType<'w> = A::ActualType<'w>;
    const CONTAINS_ANY_TAG_TERM: bool = <<A::OnlyPairType as ComponentId>::UnderlyingType as ComponentInfo>::IS_TAG;
    const IS_SPARSE_QUERY: bool = !<<A::OnlyPairType as ComponentId>::UnderlyingType as ComponentInfo>::IS_TAG
        && <<A::OnlyPairType as ComponentId>::UnderlyingType as ComponentInfo>::IS_DONT_FRAGMENT
        && !matches!(<<A::OnlyPairType as ComponentId>::UnderlyingType as ComponentInfo>::ON_INSTANTIATE, OnInstantiatePolicy::Inherit);
    const COUNT : i32 = 1;
    const COUNT_IMMUTABLE: usize = if A::IS_IMMUTABLE && !A::IS_OPTIONAL { 1 } else { 0 };
    const COUNT_MUTABLE: usize = if !A::IS_IMMUTABLE && !A::IS_OPTIONAL { 1 } else { 0 };
    const COUNT_OPTIONAL_IMMUTABLE: usize = if A::IS_IMMUTABLE && A::IS_OPTIONAL { 1 } else { 0 };
    const COUNT_OPTIONAL_MUTABLE: usize = if !A::IS_IMMUTABLE && A::IS_OPTIONAL { 1 } else { 0 };

    #[inline(always)]
    fn populate<'a>(query: &mut impl QueryBuilderImpl<'a>) {
        let _world_ptr = query.world_ptr();

        let id = <A::OnlyType as ComponentOrPairId>::get_id(query.world());

        if <A::OnlyType as ComponentOrPairId>::IS_PAIR {
            ecs_assert!(
                unsafe { sys::ecs_get_typeid(_world_ptr, id) } != 0,
                FlecsErrorCode::InvalidOperation,
                "Pair is not a (data) component. Possible cause: PairIsTag trait"
            );
        }
        
        query.with(id);
        let term = query.current_term_mut();
        A::populate_term(term);

    }

        #[expect(
        clippy::not_unsafe_ptr_arg_deref,
        reason = "x"
    )]
    #[inline(always)]
    fn register_ids_descriptor_at(
        world: *mut sys::ecs_world_t,
        terms: &mut [sys::ecs_term_t],
        index: &mut usize,
    ) {
        let world = unsafe { WorldRef::from_ptr(world) };
        terms[*index].id = <A::OnlyType as ComponentOrPairId>::get_id(world);
        A::populate_term(&mut terms[*index]);
        *index += 1;
    }

    #[inline(always)]
    fn populate_array_ptrs(
        it: &sys::ecs_iter_t,
        components: &mut [*mut u8],
        is_ref: &mut [bool],
        is_row: &mut [bool],
        indexes: &mut [i8],
        #[cfg(feature = "flecs_safety_locks")] table_records: &mut [TableColumnSafety],
    ) -> IsAnyArray {
        #[cfg(feature = "flecs_safety_locks")]
        let tr = unsafe { table_records.get_unchecked_mut(0) };
        #[cfg(feature = "flecs_safety_locks")]
        {
            let (table, column) = unsafe { flecs_field_table_column(it, 0) };
            tr.table = table;
            tr.column = column;
        }

        #[cfg(not(feature = "flecs_term_count_64"))] 
        let val = 1u32 << 0;
        #[cfg(feature = "flecs_term_count_64")]
        let val = 1u64 << 0;
        if it.row_fields & val != 0 {
            // Need to fetch the value with flecs_field_at()
            is_ref[0] = true;
            is_row[0] = true;
            indexes[0] = 0;

            #[cfg(feature = "flecs_safety_locks")]
            {
                tr.component_id = unsafe { *it.ids.add(0) };
            }
        } else {
            components[0] = flecs_field::<A::OnlyPairType>(it, 0) as *mut u8 ;
            is_ref[0] = unsafe { *it.sources.add(0) != 0 };
        };

        IsAnyArray {
            a_ref: is_ref[0],
            a_row: is_row[0],
        }
    }

    #[inline(always)]
    fn populate_self_array_ptrs(
        it: &sys::ecs_iter_t,
        components: &mut [*mut u8],
        #[cfg(feature = "flecs_safety_locks")] table_records: &mut [TableColumnSafety],

    ) {
        #[cfg(feature = "flecs_safety_locks")]
        {
            let tr = unsafe { table_records.get_unchecked_mut(0) };
            let (table, column) = unsafe { flecs_self_field_table_column(it, 0) };
            tr.table = table;
            tr.column = column;
        }
        components[0] = flecs_field::<A::OnlyPairType>(it, 0) as *mut u8 ;
    }

    #[inline(always)]
    fn create_tuple(array_components: &[*mut u8], index: usize) -> Self::TupleType<'_> {
        A::create_tuple_data(unsafe { *array_components.get_unchecked(0) }, index)

    }

    // TODO since it's only one component, we don't need to check if it's a ref array or not, we can just return the first element of the array
    // I think this is the case for all tuples of size 1
    fn create_tuple_with_ref<'a>(
        array_components: &'a [*mut u8],
        is_ref_array_components: &[bool],
        index: usize
    ) -> Self::TupleType<'a> {
        A::create_tuple_with_ref_data(array_components[0], is_ref_array_components[0], index)
    }

    #[inline(always)]
    fn create_tuple_with_row<'a>(
            iter: &sys::ecs_iter_t,
            array_components: &'a mut [*mut u8],
            is_ref_array_components: &[bool],
            is_row_array_components: &[bool],
            indexes_array_components: &[i8],
            index_row_entity: usize
        ) -> Self::TupleType<'a> {

        if is_row_array_components[0] {
            let ptr_to_first_index_array = &mut array_components[0];
            *ptr_to_first_index_array = unsafe { flecs_field_at::<A::OnlyPairType>(iter, *indexes_array_components.get_unchecked(0), index_row_entity as i32) } as *mut u8;
        }

        A::create_tuple_with_ref_data(
            array_components[0],
            is_ref_array_components[0],
            index_row_entity,
        )
    }
}

macro_rules! tuple_count {
    () => { 0 };
    ($head:ident) => { 1 };
    ($head:ident, $($tail:ident),*) => { 1 + tuple_count!($($tail),*) };
}

// macro_rules! count_immutable {
//     () => { 0 };
//     ($head:ident) => { if $head::IS_IMMUTABLE && !$head::IS_OPTIONAL { 1 } else { 0 } };
//     ($head:ident, $($tail:ident),*) => {
//         (if $head::IS_IMMUTABLE && !$head::IS_OPTIONAL { 1 } else { 0 }) + count_immutable!($($tail),*)
//     };
// }

// macro_rules! count_mutable {
//     () => { 0 };
//     ($head:ident) => { if !$head::IS_IMMUTABLE && !$head::IS_OPTIONAL { 1 } else { 0 } };
//     ($head:ident, $($tail:ident),*) => {
//         (if !$head::IS_IMMUTABLE && !$head::IS_OPTIONAL { 1 } else { 0 }) + count_mutable!($($tail),*)
//     };
// }

macro_rules! impl_iterable {
    ($($t:ident),*) => {
        impl<$($t: IterableTypeOperation),*> QueryTuple for ($($t,)*) {
            type TupleType<'w> = ($(
                $t::ActualType<'w>,
            )*);

            const CONTAINS_ANY_TAG_TERM: bool = $(<<$t::OnlyPairType as ComponentId>::UnderlyingType as ComponentInfo>::IS_TAG ||)* false;

            const IS_SPARSE_QUERY: bool = (tuple_count!($($t),*) > 0) && $((
                !<<$t::OnlyPairType as ComponentId>::UnderlyingType as ComponentInfo>::IS_TAG
                && <<$t::OnlyPairType as ComponentId>::UnderlyingType as ComponentInfo>::IS_DONT_FRAGMENT
                && !matches!(<<$t::OnlyPairType as ComponentId>::UnderlyingType as ComponentInfo>::ON_INSTANTIATE, OnInstantiatePolicy::Inherit)
            ) &&)* true;

            type Pointers = ComponentsData<Self, { tuple_count!($($t),*) }>;
            const COUNT : i32 = tuple_count!($($t),*);
            const COUNT_IMMUTABLE: usize = $((if $t::IS_IMMUTABLE && !$t::IS_OPTIONAL { 1 } else { 0 }) +)* 0;
            const COUNT_MUTABLE: usize = $((if !$t::IS_IMMUTABLE && !$t::IS_OPTIONAL { 1 } else { 0 }) +)* 0;
            const COUNT_OPTIONAL_IMMUTABLE: usize = $((if $t::IS_IMMUTABLE && $t::IS_OPTIONAL { 1 } else { 0 }) +)* 0;
            const COUNT_OPTIONAL_MUTABLE: usize = $((if !$t::IS_IMMUTABLE && $t::IS_OPTIONAL { 1 } else { 0 }) +)* 0;

            #[inline(always)]
            fn populate<'a>(query: &mut impl QueryBuilderImpl<'a>) {
                let _world = query.world();
                let _world_ptr = query.world_ptr();

                $(
                    let id = <$t::OnlyType as ComponentOrPairId>::get_id(_world);

                    if <$t::OnlyType as ComponentOrPairId>::IS_PAIR {
                        ecs_assert!(
                            unsafe { sys::ecs_get_typeid(_world_ptr, id) } != 0,
                            FlecsErrorCode::InvalidOperation,
                            "Pair is not a (data) component. Possible cause: PairIsTag trait"
                        );
                    }

                    query.with(id);
                    let term = query.current_term_mut();
                    $t::populate_term(term);

                )*
            }

            #[allow(unused)]
            #[inline(always)]
            fn register_ids_descriptor_at(world: *mut sys::ecs_world_t, terms: &mut [sys::ecs_term_t], index: &mut usize) {
                $( $t::register_ids_descriptor_at(world, terms, index); )*
            }

            #[allow(unused)]
            #[inline(always)]
            fn populate_array_ptrs(
                it: &sys::ecs_iter_t,
                components: &mut [*mut u8],
                is_ref: &mut [bool],
                is_row: &mut [bool],
                indexes: &mut [i8],
                #[cfg(feature = "flecs_safety_locks")] table_records: &mut [TableColumnSafety],
            ) -> IsAnyArray {
                let mut index : usize = 0;
                let mut any_ref = false;
                let mut any_row = false;
                let _alias_keys: [crate::core::tuple_alias::TermAliasKey; { tuple_count!($($t),*) }] = [$( crate::core::tuple_alias::TermAliasKey::new::<$t::OnlyType>(!$t::IS_IMMUTABLE) ),*];
                let _needs_alias_check = crate::core::tuple_alias::needs_runtime_alias_check(&_alias_keys);
                let mut _mut_flags = [false; { tuple_count!($($t),*) }];
                let mut _row_ids = [0u64; { tuple_count!($($t),*) }];
                let mut _row_srcs = [0u64; { tuple_count!($($t),*) }];
                #[cfg(feature = "flecs_safety_locks")]
                let mut index_immutable : usize = 0;
                #[cfg(feature = "flecs_safety_locks")]
                let mut index_mutable : usize = const { Self::COUNT_IMMUTABLE };
                #[cfg(feature = "flecs_safety_locks")]
                let mut index_optional_immutable : usize = const { Self::COUNT_IMMUTABLE + Self::COUNT_MUTABLE };
                #[cfg(feature = "flecs_safety_locks")]
                let mut index_optional_mutable : usize = const { Self::COUNT_IMMUTABLE + Self::COUNT_MUTABLE + Self::COUNT_OPTIONAL_IMMUTABLE };
                $(
                    #[cfg(feature = "flecs_safety_locks")]
                    let idx = match ($t::IS_IMMUTABLE, $t::IS_OPTIONAL) {
                            (true,  false) => &mut index_immutable,
                            (true,  true)  => &mut index_optional_immutable,
                            (false, false) => &mut index_mutable,
                            (false, true)  => &mut index_optional_mutable,
                        };

                    #[cfg(feature = "flecs_safety_locks")]
                    let tr = unsafe { table_records.get_unchecked_mut(*idx) };

                    #[cfg(feature = "flecs_safety_locks")]
                    {
                        let (table, column) = unsafe { flecs_field_table_column(it, index) };
                        tr.table = table;
                        tr.column = column;
                    }

                    #[cfg(feature = "flecs_safety_locks")]
                    {
                        *idx += 1;
                    }

                    _mut_flags[index] = !$t::IS_IMMUTABLE;
                    #[cfg(not(feature = "flecs_term_count_64"))]
                    let val = 1u32 << index;
                    #[cfg(feature = "flecs_term_count_64")]
                    let val = 1u64 << index;
                    if (it.row_fields & val) != 0 {
                        // Need to fetch the value with flecs_field_at()
                        is_ref[index] =  true;
                        is_row[index] = true;
                        indexes[index] = index as i8;
                        any_ref |= true;
                        any_row |= true;
                        let row_id = unsafe { *it.ids.add(index) };
                        let row_src = unsafe { *it.sources.add(index) };
                        if _needs_alias_check {
                            _row_ids[index] = row_id;
                            _row_srcs[index] = row_src;
                            let mut _j: usize = 0;
                            while _j < index {
                                assert!(
                                    _row_ids[_j] != row_id
                                        || _row_srcs[_j] != row_src
                                        || ($t::IS_IMMUTABLE && !_mut_flags[_j]),
                                    "query tuple `{}` has multiple terms resolving to the same sparse component `{}` on the same source, which would alias mutable access",
                                    core::any::type_name::<Self>(),
                                    core::any::type_name::<$t::OnlyPairType>()
                                );
                                _j += 1;
                            }
                        }
                        #[cfg(feature = "flecs_safety_locks")]
                        {
                            tr.component_id = row_id;
                        }
                    } else {
                        components[index] =
                            flecs_field::<$t::OnlyPairType>(it, index as i8) as *mut u8;
                        let is_ref_val = unsafe { *it.sources.add(index ) != 0 };
                        is_ref[index] = is_ref_val;
                        any_ref |= is_ref_val;
                        let _ptr = components[index];
                        if _needs_alias_check && !_ptr.is_null() {
                            let mut _j: usize = 0;
                            while _j < index {
                                assert!(
                                    components[_j] != _ptr || ($t::IS_IMMUTABLE && !_mut_flags[_j]),
                                    "query tuple `{}` has multiple terms resolving to the same component data for `{}`, which would alias mutable access",
                                    core::any::type_name::<Self>(),
                                    core::any::type_name::<$t::OnlyPairType>()
                                );
                                _j += 1;
                            }
                        }
                    }
                    index += 1;
                )*
                IsAnyArray {
                    a_ref: any_ref,
                    a_row: any_row,
                }
            }

            #[allow(unused)]
            #[inline(always)]
            fn populate_self_array_ptrs(
                it: &sys::ecs_iter_t,
                components: &mut [*mut u8],
                #[cfg(feature = "flecs_safety_locks")] table_records: &mut [TableColumnSafety],
            ) {
                let mut index : usize = 0;
                let _alias_keys: [crate::core::tuple_alias::TermAliasKey; { tuple_count!($($t),*) }] = [$( crate::core::tuple_alias::TermAliasKey::new::<$t::OnlyType>(!$t::IS_IMMUTABLE) ),*];
                let _needs_alias_check = crate::core::tuple_alias::needs_runtime_alias_check(&_alias_keys);
                let mut _mut_flags = [false; { tuple_count!($($t),*) }];
                #[cfg(feature = "flecs_safety_locks")]
                let mut index_immutable : usize = 0;
                #[cfg(feature = "flecs_safety_locks")]
                let mut index_mutable : usize = const { Self::COUNT_IMMUTABLE };
                #[cfg(feature = "flecs_safety_locks")]
                let mut index_optional_immutable : usize = const { Self::COUNT_IMMUTABLE + Self::COUNT_MUTABLE };
                #[cfg(feature = "flecs_safety_locks")]
                let mut index_optional_mutable : usize = const { Self::COUNT_IMMUTABLE + Self::COUNT_MUTABLE + Self::COUNT_OPTIONAL_IMMUTABLE };
                $(
                    components[index] =
                        flecs_field::<$t::OnlyPairType>(it, index as i8) as *mut u8;
                    _mut_flags[index] = !$t::IS_IMMUTABLE;
                    let _ptr = components[index];
                    if _needs_alias_check && !_ptr.is_null() {
                        let mut _j: usize = 0;
                        while _j < index {
                            assert!(
                                components[_j] != _ptr || ($t::IS_IMMUTABLE && !_mut_flags[_j]),
                                "query tuple `{}` has multiple terms resolving to the same component data for `{}`, which would alias mutable access",
                                core::any::type_name::<Self>(),
                                core::any::type_name::<$t::OnlyPairType>()
                            );
                            _j += 1;
                        }
                    }
                    #[cfg(feature = "flecs_safety_locks")]
                    {
                        let idx = match ($t::IS_IMMUTABLE, $t::IS_OPTIONAL) {
                            (true,  false) => &mut index_immutable,
                            (true,  true)  => &mut index_optional_immutable,
                            (false, false) => &mut index_mutable,
                            (false, true)  => &mut index_optional_mutable,
                        };

                        let tr = unsafe { table_records.get_unchecked_mut(*idx) };
                        let (table, column) = unsafe { flecs_self_field_table_column(it, index) };
                        tr.table = table;
                        tr.column = column;
                        *idx += 1;
                    }
                    index += 1;
                )*

            }

            #[allow(unused, clippy::unused_unit)]
            #[inline(always)]
            fn create_tuple(array_components: &[*mut u8], index: usize) -> Self::TupleType<'_> {
                let mut column: usize = 0;

                ($({
                    let data_ptr = unsafe { *array_components.get_unchecked(column) };
                    column += 1;
                    $t::create_tuple_data(data_ptr, index)
                },)*)
            }

            #[allow(unused, clippy::unused_unit)]
            #[inline(always)]
            fn create_tuple_with_ref<'a>(array_components: &'a [*mut u8], is_ref_array_components: &[bool], index: usize) -> Self::TupleType<'a> {
                let mut column: usize = 0;
                ($({
                    let data_ptr = unsafe { *array_components.get_unchecked(column) };
                    let is_ref = unsafe { *is_ref_array_components.get_unchecked(column) };
                    column += 1;
                    $t::create_tuple_with_ref_data(data_ptr, is_ref, index)
                },)*)
            }

            #[allow(unused, clippy::unused_unit)]
            #[inline(always)]
            fn create_tuple_with_row<'a>(
                iter: &sys::ecs_iter_t,
                array_components: &'a mut [*mut u8],
                is_ref_array_components: &[bool],
                is_row_array_components: &[bool],
                indexes_array_components: &[i8],
                index_row_entity: usize
            ) -> Self::TupleType<'a> {
                let mut column: usize = 0;
                ($({
                    let is_row = unsafe { *is_row_array_components.get_unchecked(column) };
                    if is_row {
                        let ptr_to_first_index_array = unsafe { &mut *array_components.get_unchecked_mut(column) };
                        let index_array_component = unsafe { *indexes_array_components.get_unchecked(column) };
                        *ptr_to_first_index_array = unsafe { flecs_field_at::<$t::OnlyPairType>(iter, index_array_component, index_row_entity as i32) } as *mut $t::OnlyPairType as *mut u8;
                    }
                    let data_ptr = unsafe { *array_components.get_unchecked(column) };
                    let is_ref = unsafe { *is_ref_array_components.get_unchecked(column) };
                    column += 1;
                    $t::create_tuple_with_ref_data(data_ptr, is_ref, index_row_entity)
                },)*)
            }
        }
    }
}

tuples!(impl_iterable, 0, 32);

/// Marker for a single query term that only *reads* its component: `&T`,
/// `Option<&T>`, or a tag accessed as `&Tag`. Deliberately not implemented for
/// `&mut T` or `Option<&mut T>`.
///
/// This is the per-element building block that
/// [`ReadOnlyTerms`] lifts over term tuples (via the `tuples!` macro) to decide
/// whether a [`QueryHandle`](crate::core::QueryHandle) may be shared across
/// threads.
pub trait ReadOnlyTerm {}

impl<T> ReadOnlyTerm for &T where T: ComponentOrPairId {}
impl<T> ReadOnlyTerm for Option<&T> where T: ComponentOrPairId {}

/// Marker for a query term set whose every term is a [`ReadOnlyTerm`].
///
/// This is the bound that makes [`QueryHandle`](crate::core::QueryHandle)
/// `Sync`. A handle may be shared across threads only when concurrent iteration
/// cannot hand out a `&mut` term: the per-stage safety-lock maps are
/// structurally blind across stages, so a mutable term iterated from two stages
/// would alias undetected. Read-only terms only ever produce shared references,
/// which cannot alias mutably.
///
/// Implemented for a bare read-only term and, via the crate's `tuples!` macro,
/// for every tuple whose elements are all [`ReadOnlyTerm`].
#[diagnostic::on_unimplemented(
    message = "the query terms `{Self}` are not all read-only, so a `QueryHandle` over them is not `Sync`",
    label = "this term set contains a `&mut T` or `Option<&mut T>` term",
    note = "a `QueryHandle` is `Sync` only when every term is read-only (`&T`, `Option<&T>`, or a tag)",
    note = "to mutate across threads, use a partitioned `par_*` system or an `unsafe` twin"
)]
pub trait ReadOnlyTerms {}

impl<A> ReadOnlyTerms for A where A: ReadOnlyTerm {}

macro_rules! impl_read_only_terms {
    ($($t:ident),*) => {
        impl<$($t: ReadOnlyTerm),*> ReadOnlyTerms for ($($t,)*) {}
    };
}

tuples!(impl_read_only_terms, 0, 32);

// #[cfg(test)]
// mod count_tests {
//     use super::*;
//     use crate::prelude::*;

//     #[derive(Component)]
//     struct Position {
//         x: f32,
//         y: f32,
//     }

//     #[derive(Component)]
//     struct Velocity {
//         x: f32,
//         y: f32,
//     }

//     #[derive(Component)]
//     struct Health(u32);

//     #[test]
//     fn test_count_immutable() {
//         // Test single types
//         assert_eq!(<&Position as QueryTuple>::COUNT_IMMUTABLE, 1);
//         assert_eq!(<&mut Position as QueryTuple>::COUNT_IMMUTABLE, 0);
//         assert_eq!(<Option<&Position> as QueryTuple>::COUNT_IMMUTABLE, 0);
//         assert_eq!(<Option<&mut Position> as QueryTuple>::COUNT_IMMUTABLE, 0);

//         // Test tuple types
//         assert_eq!(<(&Position, &Velocity) as QueryTuple>::COUNT_IMMUTABLE, 2);
//         assert_eq!(
//             <(&Position, &mut Velocity) as QueryTuple>::COUNT_IMMUTABLE,
//             1
//         );
//         assert_eq!(
//             <(&mut Position, &mut Velocity) as QueryTuple>::COUNT_IMMUTABLE,
//             0
//         );
//         assert_eq!(
//             <(&Position, Option<&Velocity>, &Health) as QueryTuple>::COUNT_IMMUTABLE,
//             2
//         );
//         assert_eq!(
//             <(&Position, Option<&mut Velocity>, &Health) as QueryTuple>::COUNT_IMMUTABLE,
//             2
//         );

//         // Test larger tuples
//         assert_eq!(
//             <(&Position, &Velocity, &Health) as QueryTuple>::COUNT_IMMUTABLE,
//             3
//         );
//         assert_eq!(
//             <(&mut Position, &mut Velocity, &mut Health) as QueryTuple>::COUNT_IMMUTABLE,
//             0
//         );
//         assert_eq!(<(Option<&Position>, Option<&Velocity>, Option<&Health>) as QueryTuple>::COUNT_IMMUTABLE, 0);
//         assert_eq!(
//             <(&Position, &mut Velocity, &Health, Option<&Position>) as QueryTuple>::COUNT_IMMUTABLE,
//             2
//         );
//     }

//     #[test]
//     fn test_count_mutable() {
//         // Test single types
//         assert_eq!(<&Position as QueryTuple>::COUNT_MUTABLE, 0);
//         assert_eq!(<&mut Position as QueryTuple>::COUNT_MUTABLE, 1);
//         assert_eq!(<Option<&Position> as QueryTuple>::COUNT_MUTABLE, 0);
//         assert_eq!(<Option<&mut Position> as QueryTuple>::COUNT_MUTABLE, 0);

//         // Test tuple types
//         assert_eq!(<(&Position, &Velocity) as QueryTuple>::COUNT_MUTABLE, 0);
//         assert_eq!(<(&Position, &mut Velocity) as QueryTuple>::COUNT_MUTABLE, 1);
//         assert_eq!(
//             <(&mut Position, &mut Velocity) as QueryTuple>::COUNT_MUTABLE,
//             2
//         );
//         assert_eq!(
//             <(&Position, Option<&Velocity>, &Health) as QueryTuple>::COUNT_MUTABLE,
//             0
//         );
//         assert_eq!(
//             <(&Position, Option<&mut Velocity>, &Health) as QueryTuple>::COUNT_MUTABLE,
//             0
//         );

//         // Test larger tuples
//         assert_eq!(
//             <(&Position, &Velocity, &Health) as QueryTuple>::COUNT_MUTABLE,
//             0
//         );
//         assert_eq!(
//             <(&mut Position, &mut Velocity, &mut Health) as QueryTuple>::COUNT_MUTABLE,
//             3
//         );
//         assert_eq!(
//             <(Option<&Position>, Option<&Velocity>, Option<&Health>) as QueryTuple>::COUNT_MUTABLE,
//             0
//         );
//         assert_eq!(
//             <(&Position, &mut Velocity, &Health, Option<&Position>) as QueryTuple>::COUNT_MUTABLE,
//             1
//         );
//     }

//     #[test]
//     fn test_count_optional_immutable() {
//         // Test single types
//         assert_eq!(<&Position as QueryTuple>::COUNT_OPTIONAL_IMMUTABLE, 0);
//         assert_eq!(<&mut Position as QueryTuple>::COUNT_OPTIONAL_IMMUTABLE, 0);
//         assert_eq!(
//             <Option<&Position> as QueryTuple>::COUNT_OPTIONAL_IMMUTABLE,
//             1
//         );
//         assert_eq!(
//             <Option<&mut Position> as QueryTuple>::COUNT_OPTIONAL_IMMUTABLE,
//             0
//         );

//         // Test tuple types
//         assert_eq!(
//             <(&Position, &Velocity) as QueryTuple>::COUNT_OPTIONAL_IMMUTABLE,
//             0
//         );
//         assert_eq!(
//             <(&Position, &mut Velocity) as QueryTuple>::COUNT_OPTIONAL_IMMUTABLE,
//             0
//         );
//         assert_eq!(
//             <(&mut Position, &mut Velocity) as QueryTuple>::COUNT_OPTIONAL_IMMUTABLE,
//             0
//         );
//         assert_eq!(
//             <(&Position, Option<&Velocity>, &Health) as QueryTuple>::COUNT_OPTIONAL_IMMUTABLE,
//             1
//         );
//         assert_eq!(
//             <(&Position, Option<&mut Velocity>, &Health) as QueryTuple>::COUNT_OPTIONAL_IMMUTABLE,
//             0
//         );

//         // Test larger tuples
//         assert_eq!(
//             <(&Position, &Velocity, &Health) as QueryTuple>::COUNT_OPTIONAL_IMMUTABLE,
//             0
//         );
//         assert_eq!(
//             <(&mut Position, &mut Velocity, &mut Health) as QueryTuple>::COUNT_OPTIONAL_IMMUTABLE,
//             0
//         );
//         assert_eq!(<(Option<&Position>, Option<&Velocity>, Option<&Health>) as QueryTuple>::COUNT_OPTIONAL_IMMUTABLE, 3);
//         assert_eq!(
//             <(&Position, &mut Velocity, &Health, Option<&Position>) as QueryTuple>::COUNT_OPTIONAL_IMMUTABLE,
//             1
//         );
//         assert_eq!(
//             <(Option<&Position>, Option<&mut Velocity>, Option<&Health>) as QueryTuple>::COUNT_OPTIONAL_IMMUTABLE,
//             2
//         );
//     }

//     #[test]
//     fn test_count_optional_mutable() {
//         // Test single types
//         assert_eq!(<&Position as QueryTuple>::COUNT_OPTIONAL_MUTABLE, 0);
//         assert_eq!(<&mut Position as QueryTuple>::COUNT_OPTIONAL_MUTABLE, 0);
//         assert_eq!(<Option<&Position> as QueryTuple>::COUNT_OPTIONAL_MUTABLE, 0);
//         assert_eq!(
//             <Option<&mut Position> as QueryTuple>::COUNT_OPTIONAL_MUTABLE,
//             1
//         );

//         // Test tuple types
//         assert_eq!(
//             <(&Position, &Velocity) as QueryTuple>::COUNT_OPTIONAL_MUTABLE,
//             0
//         );
//         assert_eq!(
//             <(&Position, &mut Velocity) as QueryTuple>::COUNT_OPTIONAL_MUTABLE,
//             0
//         );
//         assert_eq!(
//             <(&mut Position, &mut Velocity) as QueryTuple>::COUNT_OPTIONAL_MUTABLE,
//             0
//         );
//         assert_eq!(
//             <(&Position, Option<&Velocity>, &Health) as QueryTuple>::COUNT_OPTIONAL_MUTABLE,
//             0
//         );
//         assert_eq!(
//             <(&Position, Option<&mut Velocity>, &Health) as QueryTuple>::COUNT_OPTIONAL_MUTABLE,
//             1
//         );

//         // Test larger tuples
//         assert_eq!(
//             <(&Position, &Velocity, &Health) as QueryTuple>::COUNT_OPTIONAL_MUTABLE,
//             0
//         );
//         assert_eq!(
//             <(&mut Position, &mut Velocity, &mut Health) as QueryTuple>::COUNT_OPTIONAL_MUTABLE,
//             0
//         );
//         assert_eq!(<(Option<&Position>, Option<&Velocity>, Option<&Health>) as QueryTuple>::COUNT_OPTIONAL_MUTABLE, 0);
//         assert_eq!(
//             <(&Position, &mut Velocity, &Health, Option<&Position>) as QueryTuple>::COUNT_OPTIONAL_MUTABLE,
//             0
//         );
//         assert_eq!(
//             <(Option<&Position>, Option<&mut Velocity>, Option<&Health>) as QueryTuple>::COUNT_OPTIONAL_MUTABLE,
//             1
//         );
//         assert_eq!(
//             <(
//                 Option<&mut Position>,
//                 Option<&mut Velocity>,
//                 Option<&mut Health>
//             ) as QueryTuple>::COUNT_OPTIONAL_MUTABLE,
//             3
//         );
//     }

//     #[test]
//     fn test_total_counts_add_up() {
//         // Verify that all counts add up to the total count
//         // Using const assertions instead of type alias to avoid lifetime issues

//         const TOTAL_COUNT: usize = <(
//             &Position,
//             &mut Velocity,
//             Option<&Health>,
//             Option<&mut Position>,
//         ) as QueryTuple>::COUNT;
//         const IMMUTABLE_COUNT: usize = <(
//             &Position,
//             &mut Velocity,
//             Option<&Health>,
//             Option<&mut Position>,
//         ) as QueryTuple>::COUNT_IMMUTABLE;
//         const MUTABLE_COUNT: usize = <(
//             &Position,
//             &mut Velocity,
//             Option<&Health>,
//             Option<&mut Position>,
//         ) as QueryTuple>::COUNT_MUTABLE;
//         const OPTIONAL_IMMUTABLE_COUNT: usize = <(
//             &Position,
//             &mut Velocity,
//             Option<&Health>,
//             Option<&mut Position>,
//         ) as QueryTuple>::COUNT_OPTIONAL_IMMUTABLE;
//         const OPTIONAL_MUTABLE_COUNT: usize = <(
//             &Position,
//             &mut Velocity,
//             Option<&Health>,
//             Option<&mut Position>,
//         ) as QueryTuple>::COUNT_OPTIONAL_MUTABLE;

//         assert_eq!(TOTAL_COUNT, 4);
//         assert_eq!(IMMUTABLE_COUNT, 1);
//         assert_eq!(MUTABLE_COUNT, 1);
//         assert_eq!(OPTIONAL_IMMUTABLE_COUNT, 1);
//         assert_eq!(OPTIONAL_MUTABLE_COUNT, 1);

//         // Verify they all add up
//         assert_eq!(
//             TOTAL_COUNT,
//             IMMUTABLE_COUNT + MUTABLE_COUNT + OPTIONAL_IMMUTABLE_COUNT + OPTIONAL_MUTABLE_COUNT
//         );
//     }
// }
