//! Iterators used to iterate over tables and table rows in [`Query`], [`System`][crate::addons::system::System] and [`Observer`].
use core::marker::PhantomData;
use core::{ffi::CStr, ffi::c_void, ptr::NonNull};

use crate::core::*;
use crate::sys;

/// Errors returned by the fallible field accessors [`TableIter::try_field()`] and
/// [`TableIter::try_field_mut()`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FieldError {
    /// The field index is out of bounds for the query.
    InvalidIndex,
    /// The field at the given index does not match the requested component type.
    WrongType,
    /// The field's column is already locked for conflicting access.
    /// Only returned when the `flecs_safety_locks` feature is enabled.
    Locked,
    /// The field has no data for the current table: the field is not set or matched zero entities.
    NoMatchesCount0,
}

impl core::fmt::Display for FieldError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            FieldError::InvalidIndex => write!(f, "field index is out of bounds for the query"),
            FieldError::WrongType => {
                write!(f, "field does not match the requested component type")
            }
            FieldError::Locked => {
                write!(f, "field column is already locked for conflicting access")
            }
            FieldError::NoMatchesCount0 => write!(f, "field has no data for the current table"),
        }
    }
}

impl core::error::Error for FieldError {}

#[cold]
#[inline(never)]
fn field_type_mismatch(expected: u64, term_id: u64) -> ! {
    panic!(
        "{}: id mismatch: expected {expected}, got term id {term_id} whose component type does not match",
        FlecsErrorCode::InvalidParameter,
    );
}

pub struct TableIter<'a, const IS_RUN: bool = true, P = ()> {
    pub(crate) iter: &'a mut sys::ecs_iter_t,
    pub(crate) count: usize,
    pub(crate) world: WorldRef<'a>,
    #[cfg(feature = "flecs_safety_locks")]
    currently_multithreaded: bool,
    marker: PhantomData<P>,
}

impl<'a, const IS_RUN: bool, P> TableIter<'a, IS_RUN, P>
where
    P: ComponentId,
{
    /// The world. Can point to stage when in deferred/readonly mode.
    pub fn world(&self) -> WorldRef<'a> {
        self.world
    }

    /// Actual world. Never points to a stage.
    pub fn real_world(&self) -> WorldRef<'a> {
        unsafe { WorldRef::from_ptr(self.iter.real_world) }
    }

    /// Returns the raw flecs iterator pointer for direct C API interop.
    ///
    /// # Safety
    ///
    /// Mutating the iterator through this pointer bypasses the safety bookkeeping
    /// (table locking, field validation) that the safe accessors maintain.
    pub fn raw_iter_ptr(&mut self) -> *mut sys::ecs_iter_t {
        self.iter
    }

    /// Constructs iterator from C iterator object.
    ///
    /// This operation is typically not invoked directly by the user.
    ///
    /// # Arguments
    ///
    /// * `iter` - C iterator
    ///
    /// # See also
    ///
    /// # Safety
    /// - caller must ensure that iter.param points to type T
    pub(crate) unsafe fn new(iter: &'a mut sys::ecs_iter_t, world: WorldRef<'a>) -> Self {
        let count = iter.count as usize;
        Self {
            iter,
            count,
            #[cfg(feature = "flecs_safety_locks")]
            currently_multithreaded: world.is_currently_multithreaded(),
            world,
            marker: PhantomData,
        }
    }

    /// Return the iterator type
    ///
    /// # Example
    ///
    /// ```
    /// #[derive(Component)]
    /// struct Position {
    ///     x: f32,
    ///     y: f32,
    /// }
    ///
    /// use flecs_ecs::prelude::*;
    ///
    /// let world = World::new();
    ///
    /// world.entity().set(Position { x: 1.0, y: 2.0 });
    ///
    /// let query = world.new_query::<&Position>();
    ///
    /// query.run(|mut it| {
    ///     while it.next() {
    ///         //for each different table
    ///         let pos = it.field::<Position>(0);
    ///         for i in it.iter() {
    ///             //for each entity in the table
    ///             assert_eq!(pos[0].x, 1.0);
    ///             assert_eq!(pos[0].y, 2.0);
    ///         }
    ///     }
    /// });
    /// ```
    #[inline(always)]
    pub fn iter(&self) -> impl ExactSizeIterator<Item = FieldIndex> + use<IS_RUN, P> {
        // Range has no bounds checks, .map(FieldIndex) is inlined out to a single `add+load`
        (0..self.count).map(FieldIndex)
    }

    /// Wrap the system id in the iterator in an [`EntityView`] object.
    pub fn system(&self) -> EntityView<'a> {
        EntityView::new_from(self.world(), self.iter.system)
    }

    /// Wrap the event id in the iterator in an [`EntityView`] object.
    pub fn event(&self) -> EntityView<'a> {
        EntityView::new_from(self.world(), self.iter.event)
    }

    /// Wrap the event id in the iterator in an [`IdView`] object.
    pub fn event_id(&self) -> IdView<'a> {
        IdView::new_from_id(self.world(), self.iter.event_id)
    }

    /// Obtain mutable handle to entity being iterated over.
    ///
    /// # Arguments
    ///
    /// * `row` - Row being iterated over
    pub fn entity(&self, row: impl Into<usize>) -> EntityView<'a> {
        let row = row.into();
        assert!(
            row < self.count,
            "row {row} out of bounds for iterator count {}",
            self.count
        );
        let ptr = unsafe { self.iter.entities.add(row) };
        if ptr.is_null() {
            panic!("no entity at row {row}");
        }
        unsafe { EntityView::new_from(self.world(), *ptr) }
    }

    /// Obtain mutable handle to entity being iterated over.
    ///
    /// # Arguments
    ///
    /// * `row` - Row being iterated over
    pub fn get_entity(&self, row: impl Into<usize>) -> Option<EntityView<'a>> {
        let row = row.into();
        if row >= self.count {
            return None;
        }
        let ptr = unsafe { self.iter.entities.add(row) };
        if ptr.is_null() {
            return None;
        }
        Some(unsafe { EntityView::new_from(self.world(), *ptr) })
    }

    /// Obtain entity id being iterated over. Can return Entity with id 0 if the entity at the specified row is not found.
    ///
    /// # Arguments
    ///
    /// * `row` - Row being iterated over
    #[inline(always)]
    pub fn entity_id(&self, row: impl Into<usize>) -> Entity {
        let row = row.into();
        assert!(
            row < self.count,
            "row {row} out of bounds for iterator count {}",
            self.count
        );
        let ptr = unsafe { self.iter.entities.add(row) };

        if ptr.is_null() {
            return Entity::null();
        }
        unsafe { Entity(*ptr) }
    }

    /// Return a mut reference to the raw iterator object.
    pub fn iter_mut(&mut self) -> &mut sys::ecs_iter_t {
        self.iter
    }

    /// Return the count of entities in the iterator.
    #[inline(always)]
    pub fn count(&self) -> usize {
        //TODO soft assert
        // ecs_check(iter_->flags & EcsIterIsValid, ECS_INVALID_PARAMETER,
        //     "operation invalid before calling next()");
        self.count
    }

    /// Return the delta time stored in the iterator.
    ///
    /// This is the time since the last frame.
    ///
    /// # See also
    ///
    /// * [`TableIter::delta_system_time()`]
    pub fn delta_time(&self) -> FTime {
        self.iter.delta_time
    }

    /// Return the delta system time stored in the iterator.
    ///
    /// This is the time since the last system invocation.
    ///
    /// # See also
    ///
    /// * [`TableIter::delta_time()`]
    pub fn delta_system_time(&self) -> FTime {
        self.iter.delta_system_time
    }

    /// Return the table stored in the iterator as an `Archetype` object
    pub fn archetype(&self) -> Option<Archetype<'a>> {
        self.table().map(|t| t.archetype())
    }

    pub fn table(&self) -> Option<Table<'a>> {
        // SAFETY: the iterator holds a live table owned by the real world.
        NonNull::new(self.iter.table).map(|ptr| unsafe { Table::new(self.real_world(), ptr) })
    }

    pub fn other_table(&self) -> Option<Table<'a>> {
        // SAFETY: the iterator holds a live table owned by the real world.
        NonNull::new(self.iter.other_table).map(|ptr| unsafe { Table::new(self.real_world(), ptr) })
    }

    pub fn range(&self) -> Option<TableRange<'a>> {
        self.table()
            .map(|t| TableRange::new(t, self.iter.offset, self.count as i32))
    }

    /// Get the variable of the iterator
    ///
    /// # Arguments
    ///
    /// * `var_id` - The variable id
    pub fn get_var(&self, var_id: i32) -> EntityView<'a> {
        ecs_assert!(var_id != -1, FlecsErrorCode::InvalidParameter, 0);
        let var =
            unsafe { sys::ecs_iter_get_var(self.iter as *const _ as *mut sys::ecs_iter_t, var_id) };
        let world = self.world();
        EntityView::new_from(world, var)
    }

    /// Get the variable of the iterator by name
    ///
    /// # Arguments
    ///
    /// * `var_id` - The variable id
    pub fn get_var_by_name(&self, name: &str) -> EntityView<'a> {
        let name = compact_str::format_compact!("{}\0", name);

        let world = self.world();
        let rule_query = self.iter.query;
        let var_id = unsafe { sys::ecs_query_find_var(rule_query, name.as_ptr() as *const _) };
        ecs_assert!(
            var_id != -1,
            FlecsErrorCode::InvalidParameter,
            name.as_str()
        );
        EntityView::new_from(world, unsafe {
            sys::ecs_iter_get_var(self.iter as *const _ as *mut _, var_id)
        })
    }

    /// Access ctx.
    /// ctx contains the context pointer assigned to a system
    ///
    /// # See also
    ///
    /// # Safety
    /// - caller must ensure the ctx variable was set to a type accessible as C and is not aliased
    pub unsafe fn context<T>(&mut self) -> &'a mut T {
        unsafe { &mut *(self.iter.ctx as *mut T) }
    }

    /// Access ctx.
    /// ctx contains the context pointer assigned to a system
    pub fn context_ptr(&self) -> *mut c_void {
        self.iter.ctx
    }

    /// Access param.
    /// param contains the pointer passed to the param argument of `system::run`
    ///
    /// # Safety
    ///
    /// - Caller must ensure the type is correct when accessing the pointer.
    pub unsafe fn param_untyped(&self) -> *mut c_void {
        self.iter.param
    }

    /// Access param.
    /// param contains the pointer passed to the param argument of `system::run` or the event payload
    ///
    /// There is no `param_mut` function to discourage the user from making changes to events sent to observers as
    /// order of execution is not defined.
    pub fn param(&self) -> &P::UnderlyingType {
        const {
            assert!(
                !P::IS_TAG,
                "called `.param()` on a ZST / tag data, which cannot be used when no payload is provided"
            );
        }

        self.world()
            .check_thread_affinity_shared::<P::UnderlyingType>();

        let ptr = self.iter.param as *const P::UnderlyingType;

        assert!(
            !ptr.is_null(),
            "Tried to get param on an iterator where it was null."
        );

        unsafe { &*ptr }
    }

    /// # Arguments
    ///
    /// * `index` - Index of the field to check
    ///
    /// # Returns
    ///
    /// Returns whether field is matched on self
    /// # Panics
    ///
    /// Panics if `index` is negative or not smaller than the iterator's field count.
    #[inline(always)]
    pub fn is_self(&self, index: i8) -> bool {
        assert!(
            index >= 0 && index < self.iter.field_count,
            "field index {index} out of range (field_count = {})",
            self.iter.field_count
        );
        self.iter.sources.is_null() || unsafe { (*self.iter.sources.add(index as usize)) == 0 }
    }

    /// # Arguments
    ///
    /// * `index` - Index of the field to check
    ///
    /// # Returns
    ///
    /// Returns whether field is set
    /// # Panics
    ///
    /// Panics if `index` is negative or not smaller than the iterator's field count.
    pub fn is_set(&self, index: i8) -> bool {
        assert!(
            index >= 0 && index < self.iter.field_count,
            "field index {index} out of range (field_count = {})",
            self.iter.field_count
        );
        #[cfg(not(feature = "flecs_term_count_64"))]
        let val = 1u32 << (index as usize);
        #[cfg(feature = "flecs_term_count_64")]
        let val = 1u64 << (index as usize);
        self.iter.set_fields & val != 0
    }

    /// # Arguments
    ///
    /// * `index` - Index of the field to check
    ///
    /// # Returns
    ///
    /// Returns whether field is readonly
    pub fn is_readonly(&self, index: i8) -> bool {
        unsafe { sys::ecs_field_is_readonly(self.iter, index) }
    }

    /// Number of fields in iterator.
    pub fn field_count(&self) -> i8 {
        self.iter.field_count
    }

    /// Size of field data type.
    ///
    /// # Arguments
    ///
    /// * `index` - The field id.
    pub fn size(&self, index: i8) -> usize {
        unsafe { sys::ecs_field_size(self.iter, index) }
    }

    /// Obtain field source (0 if This).
    ///
    /// # Arguments
    ///
    /// * `index` - The field index.
    pub fn src(&self, index: usize) -> EntityView<'a> {
        unsafe { EntityView::new_from(self.world(), sys::ecs_field_src(self.iter, index as i8)) }
    }

    /// Obtain id matched for field.
    ///
    /// # Arguments
    ///
    /// * `index` - The field index.
    pub fn id(&self, index: i8) -> IdView<'a> {
        unsafe { IdView::new_from_id(self.world(), sys::ecs_field_id(self.iter, index)) }
    }

    /// Obtain pair id matched for field.
    /// This operation will return `None` if the field is not a pair.
    ///
    /// # Arguments
    ///
    /// * `index` - The field index.
    ///
    /// # See also
    /// * [`TableIter::pair`]
    pub fn get_pair(&self, index: i8) -> Option<IdView<'a>> {
        unsafe {
            let id = sys::ecs_field_id(self.iter, index);
            if sys::ecs_id_is_pair(id) {
                Some(IdView::new_from_id(self.world(), id))
            } else {
                None
            }
        }
    }

    /// Obtain pair id matched for field.
    /// This operation will panic if the field is not a pair.
    ///
    /// # Arguments
    ///
    /// * `index` - The field index.
    ///
    /// # See also
    /// * [`TableIter::get_pair`]
    pub fn pair(&self, index: i8) -> IdView<'a> {
        unsafe {
            let id = sys::ecs_field_id(self.iter, index);
            if sys::ecs_id_is_pair(id) {
                IdView::new_from_id(self.world(), id)
            } else {
                panic!("Field at index {index} is not a pair");
            }
        }
    }

    /// Obtain column index for field.
    ///
    /// # Arguments
    ///
    /// * `index` - The field index.
    pub fn column_index(&self, index: i8) -> i32 {
        unsafe { sys::ecs_field_column(self.iter, index) }
    }

    /// Obtain term that triggered an observer
    pub fn term_index(&self) -> i8 {
        self.iter.term_index
    }

    /// Convert current iterator result to string
    pub fn to_str(&self) -> &'a CStr {
        let c_str = unsafe { sys::ecs_iter_str(self.iter) };
        ecs_assert!(!c_str.is_null(), FlecsErrorCode::InvalidParameter);
        unsafe { CStr::from_ptr(c_str) }
    }

    #[inline(always)]
    #[allow(clippy::extra_unused_type_parameters)] //release build clippy warning
    fn field_safety_checks<
        T: ComponentId,
        const READONLY: bool,
        const TYPED: bool,
        const IS_FIELD_AT: bool,
    >(
        &self,
        _index: i8,
    ) {
        ecs_assert!(
            _index < self.iter.field_count,
            FlecsErrorCode::InvalidParameter,
            _index
        );
        if !IS_FIELD_AT {
            ecs_assert!(
                (self.iter.flags & sys::EcsIterCppEach == 0)
                    || unsafe { sys::ecs_field_src(self.iter, _index) != 0 },
                FlecsErrorCode::InvalidOperation,
                "cannot .field_handle from .each, use .field_at instead",
            );
        }

        if !READONLY {
            ecs_assert!(
                !unsafe { sys::ecs_field_is_readonly(self.iter, _index) },
                FlecsErrorCode::AccessViolation,
                "field is readonly, check if your specified query terms are set &mut"
            );
        }
        if TYPED {
            // Always checked, also in release builds: a mismatch would let the
            // field data be reinterpreted as the wrong type.
            let term_id = unsafe { sys::ecs_field_id(self.iter, _index) };
            let id = <T::UnderlyingType as ComponentId>::entity_id(self.world());
            let matches = id == term_id
                || (unsafe { sys::ecs_id_is_pair(term_id) }
                    && unsafe { sys::ecs_get_typeid(self.world().world_ptr_mut(), term_id) == id });
            if !matches {
                field_type_mismatch(id, term_id);
            }
        }
    }

    /// Get immutable access to field data.
    ///
    /// # Arguments
    ///
    /// * `field_handle` - The field handle.
    ///
    /// # Returns
    ///
    /// Returns a column object that can be used to access the field data.
    ///
    /// # Example
    ///
    /// ```
    /// # use flecs_ecs::prelude::*;
    /// # #[derive(Component)]
    /// # struct Position { x: f32, y: f32 }
    /// # #[derive(Component)]
    /// # struct Velocity { x: f32, y: f32 }
    /// # let world = World::new();
    /// # world.entity().set(Position { x: 1.0, y: 2.0 });
    /// # let query = world.new_query::<(&mut Position, &Velocity)>();
    /// query.run(|mut it| {
    ///     while it.next() {
    ///         let mut pos = it.field_mut::<Position>(0);
    ///         let vel = it.field::<Velocity>(1);
    ///         for i in it.iter() {
    ///             let mut position = &mut pos[i];
    ///             let velocity = &vel[i];
    ///             position.x += velocity.x;
    ///             position.y += velocity.y;
    ///         }
    ///     }
    /// });
    /// ```
    ///
    /// # Panics
    ///
    /// When the `flecs_safety_locks` feature is enabled (default), panics if the field's column
    /// is already locked for conflicting access (e.g. a [`FieldMut`] for the same field is still
    /// alive). Use [`TableIter::try_field()`] to handle this case without panicking.
    ///
    /// # See also
    /// * [`TableIter::try_field`]
    /// * [`TableIter::get_field`]
    /// * [`TableIter::field_mut`]
    /// * [`TableIter::get_field_mut`]
    /// * [`TableIter::field_unchecked`] - Unsafe variant without aliasing checks
    #[inline(always)]
    pub fn field<T: ComponentId>(&self, index: i8) -> Field<'_, T::UnderlyingType, true> {
        self.field_safety_checks::<T, true, true, false>(index);
        self.world()
            .check_thread_affinity_shared::<T::UnderlyingType>();
        self.field_internal::<T::UnderlyingType, true>(index)
    }

    /// Fallible variant of [`TableIter::field()`].
    ///
    /// Unlike [`TableIter::field()`]/[`TableIter::get_field()`], this never panics on a
    /// safety-lock conflict when the `flecs_safety_locks` feature is enabled; it returns
    /// [`FieldError::Locked`] instead.
    ///
    /// # Arguments
    ///
    /// * `index` - The field index.
    ///
    /// # Errors
    ///
    /// * [`FieldError::NoMatchesCount0`] - the field is not set for the current table or
    ///   matched zero entities.
    /// * [`FieldError::Locked`] - the field's column is already locked for conflicting access
    ///   (only with the `flecs_safety_locks` feature).
    ///
    /// # See also
    /// * [`TableIter::field`]
    /// * [`TableIter::try_field_mut`]
    #[inline(always)]
    pub fn try_field<T: ComponentId>(
        &self,
        index: i8,
    ) -> Result<Field<'_, T::UnderlyingType, true>, FieldError> {
        self.field_result::<T>(index)
    }

    /// Get immutable access to field data without aliasing checks.
    ///
    /// This is an unsafe variant of [`field()`](TableIter::field) that bypasses the mutable
    /// aliasing detection when the `flecs_safety_locks` feature is enabled.
    ///
    /// When the `flecs_safety_locks` feature is enabled, this function bypasses all runtime
    /// safety checks that [`field()`](TableIter::field) would normally perform.
    ///
    /// # Safety
    ///
    /// The caller must ensure that:
    /// - No mutable references to the same component exist simultaneously
    /// - The field data is not being modified while this immutable reference exists
    ///
    /// # Arguments
    ///
    /// * `index` - The field index.
    ///
    /// # Returns
    ///
    /// Returns a column object that can be used to access the field data.
    ///
    /// # Example
    ///
    /// ```
    /// # use flecs_ecs::prelude::*;
    /// # #[derive(Component)]
    /// # struct Position { x: f32, y: f32 }
    /// # let world = World::new();
    /// # world.entity().set(Position { x: 1.0, y: 2.0 });
    /// # let query = world.new_query::<&Position>();
    /// query.run(|mut it| {
    ///     while it.next() {
    ///         // Safe: read-only access with no mutable aliases
    ///         let pos = unsafe { it.field_unchecked::<Position>(0) };
    ///         for i in it.iter() {
    ///             println!("Position: ({}, {})", pos[i].x, pos[i].y);
    ///         }
    ///     }
    /// });
    /// ```
    ///
    /// # See also
    /// * [`TableIter::field`] - Safe variant with aliasing checks
    /// * [`TableIter::get_field_unchecked`]
    /// * [`TableIter::field_mut_unchecked`]
    /// * [`TableIter::get_field_mut_unchecked`]
    #[inline(always)]
    pub unsafe fn field_unchecked<T: ComponentId>(
        &self,
        index: i8,
    ) -> Field<'_, T::UnderlyingType, false> {
        self.field_safety_checks::<T, true, true, false>(index);
        self.field_internal::<T::UnderlyingType, false>(index)
    }

    /// Get immutable access to field data.
    ///
    /// # Arguments
    ///
    /// * `field_handle` - The field handle.
    ///
    /// # Returns
    ///
    /// Returns a column object that can be used to access the field data.
    ///
    /// # Example
    ///
    /// ```
    /// # use flecs_ecs::prelude::*;
    /// # #[derive(Component)]
    /// # struct Position { x: f32, y: f32 }
    /// # #[derive(Component)]
    /// # struct Velocity { x: f32, y: f32 }
    /// # let world = World::new();
    /// # world.entity().set(Position { x: 1.0, y: 2.0 });
    /// # let query = world.new_query::<(&mut Position, Option<&Velocity>)>();
    /// query.run(|mut it| {
    ///     while it.next() {
    ///         let mut pos = it.get_field_mut::<Position>(0).unwrap();
    ///         let vel = it.get_field::<Velocity>(1);
    ///
    ///         if let Some(vel) = vel {
    ///             for i in it.iter() {
    ///                 let mut position = &mut pos[i];
    ///                 let velocity = &vel[i];
    ///                 position.x += velocity.x;
    ///                 position.y += velocity.y;
    ///             }
    ///         } else {
    ///             for i in it.iter() {
    ///                 let mut position = &mut pos[i];
    ///                 position.x += 0.1;
    ///                 position.y += 0.1;
    ///             }
    ///         }
    ///     }
    /// });
    /// ```
    ///
    /// # Panics
    ///
    /// When the `flecs_safety_locks` feature is enabled (default), panics if the field's column
    /// is already locked for conflicting access (e.g. a [`FieldMut`] for the same field is still
    /// alive). Use [`TableIter::try_field()`] to handle this case without panicking.
    ///
    /// # See also
    /// * [`TableIter::field`]
    /// * [`TableIter::try_field`]
    /// * [`TableIter::field_mut`]
    /// * [`TableIter::get_field_mut`]
    /// * [`TableIter::get_field_unchecked`] - Unsafe variant without aliasing checks
    #[inline(always)]
    pub fn get_field<T: ComponentId>(
        &self,
        index: i8,
    ) -> Option<Field<'_, T::UnderlyingType, true>> {
        self.field_safety_checks::<T, true, true, false>(index);
        self.world()
            .check_thread_affinity_shared::<T::UnderlyingType>();
        self.get_field_internal::<T::UnderlyingType, true>(index)
    }

    /// Get immutable access to field data wrapped in Option, without aliasing checks.
    ///
    /// This is an unsafe variant of [`get_field()`](TableIter::get_field) that bypasses the mutable
    /// aliasing detection when the `flecs_safety_locks` feature is enabled. This is useful with optional fields.
    ///
    /// When the `flecs_safety_locks` feature is enabled, this function bypasses all runtime
    /// safety checks that [`get_field()`](TableIter::get_field) would normally perform.
    ///
    /// # Safety
    ///
    /// The caller must ensure that:
    /// - No mutable references to the same component exist simultaneously
    /// - The field data is not being modified while this immutable reference exists
    /// - Thread safety is maintained in multi-threaded contexts
    ///
    /// # Arguments
    ///
    /// * `index` - The field index.
    ///
    /// # Returns
    ///
    /// Returns Some with a column object that can be used to access the field data, or None if the field doesn't exist.
    ///
    /// # Example
    ///
    /// ```
    /// # use flecs_ecs::prelude::*;
    /// # #[derive(Component)]
    /// # struct Position { x: f32, y: f32 }
    /// # #[derive(Component)]
    /// # struct Velocity { x: f32, y: f32 }
    /// # let world = World::new();
    /// # world.entity().set(Position { x: 1.0, y: 2.0 });
    /// # let query = world.new_query::<(&Position, Option<&Velocity>)>();
    /// query.run(|mut it| {
    ///     while it.next() {
    ///         // Safe: read-only access with no mutable aliases
    ///         let pos = unsafe { it.get_field_unchecked::<Position>(0).unwrap() };
    ///         let vel = unsafe { it.get_field_unchecked::<Velocity>(1) };
    ///
    ///         if let Some(vel) = vel {
    ///             for i in it.iter() {
    ///                 println!(
    ///                     "Pos: ({}, {}), Vel: ({}, {})",
    ///                     pos[i].x, pos[i].y, vel[i].x, vel[i].y
    ///                 );
    ///             }
    ///         }
    ///     }
    /// });
    /// ```
    ///
    /// # See also
    /// * [`TableIter::get_field`] - Safe variant with aliasing checks
    /// * [`TableIter::field_unchecked`]
    /// * [`TableIter::field_mut_unchecked`]
    /// * [`TableIter::get_field_mut_unchecked`]
    #[inline(always)]
    pub unsafe fn get_field_unchecked<T: ComponentId>(
        &self,
        index: i8,
    ) -> Option<Field<'_, T::UnderlyingType, false>> {
        self.field_safety_checks::<T, true, true, false>(index);
        self.get_field_internal::<T::UnderlyingType, false>(index)
    }

    /// Get mutable access to field data.
    ///
    /// # Arguments
    ///
    /// * `field_handle` - The field handle.
    ///
    /// # Returns
    ///
    /// Returns a column object that can be used to access the field data.
    ///
    /// # Example
    ///
    /// ```
    /// # use flecs_ecs::prelude::*;
    /// # #[derive(Component)]
    /// # struct Position { x: f32, y: f32 }
    /// # #[derive(Component)]
    /// # struct Velocity { x: f32, y: f32 }
    /// # let world = World::new();
    /// # world.entity().set(Position { x: 1.0, y: 2.0 });
    /// # let query = world.new_query::<(&mut Position, &Velocity)>();
    /// query.run(|mut it| {
    ///     while it.next() {
    ///         let mut pos = it.field_mut::<Position>(0);
    ///         let vel = it.field::<Velocity>(1);
    ///         for i in it.iter() {
    ///             let mut position = &mut pos[i];
    ///             let velocity = &vel[i];
    ///             position.x += velocity.x;
    ///             position.y += velocity.y;
    ///         }
    ///     }
    /// });
    /// ```
    ///
    /// # Panics
    ///
    /// When the `flecs_safety_locks` feature is enabled (default), panics if the field's column
    /// is already locked (e.g. another [`Field`] or [`FieldMut`] for the same field is still
    /// alive). Use [`TableIter::try_field_mut()`] to handle this case without panicking.
    ///
    /// # See also
    /// * [`TableIter::field`]
    /// * [`TableIter::try_field_mut`]
    /// * [`TableIter::get_field`]
    /// * [`TableIter::get_field_mut`]
    /// * [`TableIter::field_mut_unchecked`] - Unsafe variant without aliasing checks
    #[inline(always)]
    pub fn field_mut<T: ComponentId>(&self, index: i8) -> FieldMut<'_, T::UnderlyingType, true> {
        self.field_safety_checks::<T, false, true, false>(index);
        self.world()
            .check_thread_affinity_exclusive::<T::UnderlyingType>();
        self.field_internal_mut::<T::UnderlyingType, true>(index)
    }

    /// Fallible variant of [`TableIter::field_mut()`].
    ///
    /// Unlike [`TableIter::field_mut()`]/[`TableIter::get_field_mut()`], this never panics on a
    /// safety-lock conflict when the `flecs_safety_locks` feature is enabled; it returns
    /// [`FieldError::Locked`] instead.
    ///
    /// # Arguments
    ///
    /// * `index` - The field index.
    ///
    /// # Errors
    ///
    /// * [`FieldError::NoMatchesCount0`] - the field is not set for the current table or
    ///   matched zero entities.
    /// * [`FieldError::Locked`] - the field's column is already locked
    ///   (only with the `flecs_safety_locks` feature).
    ///
    /// # See also
    /// * [`TableIter::field_mut`]
    /// * [`TableIter::try_field`]
    #[inline(always)]
    pub fn try_field_mut<T: ComponentId>(
        &self,
        index: i8,
    ) -> Result<FieldMut<'_, T::UnderlyingType, true>, FieldError> {
        self.field_result_mut::<T>(index)
    }

    /// Get mutable access to field data without aliasing checks.
    ///
    /// This is an unsafe variant of [`field_mut()`](TableIter::field_mut) that bypasses the mutable
    /// aliasing detection when the `flecs_safety_locks` feature is enabled.
    ///
    /// When the `flecs_safety_locks` feature is enabled, this function bypasses all runtime
    /// safety checks that [`field_mut()`](TableIter::field_mut) would normally perform.
    ///
    /// # Safety
    ///
    /// The caller must ensure that:
    /// - No other references (mutable or immutable) to the same component exist simultaneously
    /// - The field data is not being accessed from multiple threads without proper synchronization
    /// - No aliasing violations occur with other field accesses
    ///
    /// # Arguments
    ///
    /// * `index` - The field index.
    ///
    /// # Returns
    ///
    /// Returns a column object that can be used to mutate the field data.
    ///
    /// # Example
    ///
    /// ```
    /// # use flecs_ecs::prelude::*;
    /// # #[derive(Component)]
    /// # struct Position { x: f32, y: f32 }
    /// # #[derive(Component)]
    /// # struct Velocity { x: f32, y: f32 }
    /// # let world = World::new();
    /// # world.entity().set(Position { x: 1.0, y: 2.0 }).set(Velocity { x: 0.1, y: 0.2 });
    /// # let query = world.new_query::<(&mut Position, &Velocity)>();
    /// query.run(|mut it| {
    ///     while it.next() {
    ///         // Safe: Position and Velocity are different components
    ///         let mut pos = unsafe { it.field_mut_unchecked::<Position>(0) };
    ///         let vel = unsafe { it.field_unchecked::<Velocity>(1) };
    ///         for i in it.iter() {
    ///             pos[i].x += vel[i].x;
    ///             pos[i].y += vel[i].y;
    ///         }
    ///     }
    /// });
    /// ```
    ///
    /// # See also
    /// * [`TableIter::field_mut`] - Safe variant with aliasing checks
    /// * [`TableIter::field_unchecked`]
    /// * [`TableIter::get_field_unchecked`]
    /// * [`TableIter::get_field_mut_unchecked`]
    #[inline(always)]
    pub unsafe fn field_mut_unchecked<T: ComponentId>(
        &self,
        index: i8,
    ) -> FieldMut<'_, T::UnderlyingType, false> {
        self.field_safety_checks::<T, false, true, false>(index);
        self.field_internal_mut::<T::UnderlyingType, false>(index)
    }

    /// Get mutable access to field data wrapped in Option. This is useful with optional fields.
    ///
    /// # Arguments
    ///
    /// * `field_handle` - The field handle.
    ///
    /// # Returns
    ///
    /// Returns a column object that can be used to access the field data.
    ///
    /// # Example
    ///
    /// ```
    /// # use flecs_ecs::prelude::*;
    /// # #[derive(Component)]
    /// # struct Position { x: f32, y: f32 }
    /// # #[derive(Component)]
    /// # struct Velocity { x: f32, y: f32 }
    /// # let world = World::new();
    /// # world.entity().set(Position { x: 1.0, y: 2.0 });
    /// # let query = world.new_query::<(&mut Position, Option<&Velocity>)>();
    /// query.run(|mut it| {
    ///     while it.next() {
    ///         let mut pos = it.get_field_mut::<Position>(0).unwrap();
    ///         let vel = it.get_field::<Velocity>(1);
    ///
    ///         if let Some(vel) = vel {
    ///             for i in it.iter() {
    ///                 let mut position = &mut pos[i];
    ///                 let velocity = &vel[i];
    ///                 position.x += velocity.x;
    ///                 position.y += velocity.y;
    ///             }
    ///         } else {
    ///             for i in it.iter() {
    ///                 let mut position = &mut pos[i];
    ///                 position.x += 0.1;
    ///                 position.y += 0.1;
    ///             }
    ///         }
    ///     }
    /// });
    /// ```
    ///
    /// # Panics
    ///
    /// When the `flecs_safety_locks` feature is enabled (default), panics if the field's column
    /// is already locked (e.g. another [`Field`] or [`FieldMut`] for the same field is still
    /// alive). Use [`TableIter::try_field_mut()`] to handle this case without panicking.
    ///
    /// # See also
    /// * [`TableIter::field`]
    /// * [`TableIter::field_mut`]
    /// * [`TableIter::get_field`]
    /// * [`TableIter::try_field_mut`]
    /// * [`TableIter::get_field_mut_unchecked`] - Unsafe variant without aliasing checks
    #[inline(always)]
    pub fn get_field_mut<T: ComponentId>(
        &self,
        index: i8,
    ) -> Option<FieldMut<'_, T::UnderlyingType, true>> {
        self.field_safety_checks::<T, false, true, false>(index);
        self.world()
            .check_thread_affinity_exclusive::<T::UnderlyingType>();
        self.get_field_internal_mut::<T::UnderlyingType, true>(index)
    }

    /// Get mutable access to field data wrapped in Option, without aliasing checks.
    ///
    /// This is an unsafe variant of [`get_field_mut()`](TableIter::get_field_mut) that bypasses the mutable
    /// aliasing detection when the `flecs_safety_locks` feature is enabled. This is useful with optional fields.
    ///
    /// When the `flecs_safety_locks` feature is enabled, this function bypasses all runtime
    /// safety checks that [`get_field_mut()`](TableIter::get_field_mut) would normally perform.
    ///
    /// # Safety
    ///
    /// The caller must ensure that:
    /// - No other references (mutable or immutable) to the same component exist simultaneously
    /// - The field data is not being accessed from multiple threads without proper synchronization
    /// - No aliasing violations occur with other field accesses
    ///
    /// # Arguments
    ///
    /// * `index` - The field index.
    ///
    /// # Returns
    ///
    /// Returns Some with a column object that can be used to mutate the field data, or None if the field doesn't exist.
    ///
    /// # Example
    ///
    /// ```
    /// # use flecs_ecs::prelude::*;
    /// # #[derive(Component)]
    /// # struct Position { x: f32, y: f32 }
    /// # #[derive(Component)]
    /// # struct Velocity { x: f32, y: f32 }
    /// # let world = World::new();
    /// # world.entity().set(Position { x: 1.0, y: 2.0 });
    /// # let query = world.new_query::<(&mut Position, Option<&Velocity>)>();
    /// query.run(|mut it| {
    ///     while it.next() {
    ///         // Safe: Position and Velocity are different components
    ///         let mut pos = unsafe { it.get_field_mut_unchecked::<Position>(0).unwrap() };
    ///         let vel = unsafe { it.get_field_unchecked::<Velocity>(1) };
    ///
    ///         if let Some(vel) = vel {
    ///             for i in it.iter() {
    ///                 pos[i].x += vel[i].x;
    ///                 pos[i].y += vel[i].y;
    ///             }
    ///         } else {
    ///             for i in it.iter() {
    ///                 pos[i].x += 0.1;
    ///                 pos[i].y += 0.1;
    ///             }
    ///         }
    ///     }
    /// });
    /// ```
    ///
    /// # See also
    /// * [`TableIter::get_field_mut`] - Safe variant with aliasing checks
    /// * [`TableIter::field_unchecked`]
    /// * [`TableIter::get_field_unchecked`]
    /// * [`TableIter::field_mut_unchecked`]
    #[inline(always)]
    pub unsafe fn get_field_mut_unchecked<T: ComponentId>(
        &self,
        index: i8,
    ) -> Option<FieldMut<'_, T::UnderlyingType, false>> {
        self.field_safety_checks::<T, false, true, false>(index);
        self.get_field_internal_mut::<T::UnderlyingType, false>(index)
    }

    /// Get immutable access to untyped field data.
    ///
    /// # Arguments
    ///
    /// * `field_handle` - The field handle.
    ///
    /// # Returns
    ///
    /// Returns a column object that can be used to access the field data.
    #[inline(always)]
    pub fn field_untyped(&self, index: i8) -> FieldUntyped {
        #[cfg(any(debug_assertions, feature = "flecs_force_enable_ecs_asserts"))]
        self.field_safety_checks::<(), true, false, false>(index);
        self.field_untyped_internal(index)
    }

    /// Get immutable access to untyped field data.
    ///
    /// # Arguments
    ///
    /// * `field_handle` - The field handle.
    ///
    /// # Returns
    ///
    /// Returns a column object that can be used to access the field data.
    #[inline(always)]
    pub fn get_field_untyped(&self, index: i8) -> Option<FieldUntyped> {
        #[cfg(any(debug_assertions, feature = "flecs_force_enable_ecs_asserts"))]
        self.field_safety_checks::<(), true, false, false>(index);
        self.get_field_untyped_internal(index)
    }

    /// Get mutable access to untyped field data.
    ///
    /// # Arguments
    ///
    /// * `field_handle` - The field handle.
    ///
    /// # Returns
    ///
    /// Returns a column object that can be used to access the field data.
    #[inline(always)]
    pub fn field_untyped_mut(&self, index: i8) -> FieldUntypedMut {
        #[cfg(any(debug_assertions, feature = "flecs_force_enable_ecs_asserts"))]
        self.field_safety_checks::<(), false, false, false>(index);
        self.field_untyped_internal_mut(index)
    }

    /// Get mutable access to untyped field data.
    ///
    /// # Arguments
    ///
    /// * `field_handle` - The field handle.
    ///
    /// # Returns
    ///
    /// Returns a column object that can be used to access the field data.
    #[inline(always)]
    pub fn get_field_untyped_mut(&self, index: i8) -> Option<FieldUntypedMut> {
        #[cfg(any(debug_assertions, feature = "flecs_force_enable_ecs_asserts"))]
        self.field_safety_checks::<(), false, false, false>(index);
        self.get_field_untyped_internal_mut(index)
    }

    /// Get immutable access to a sparse component at a specific row.
    ///
    /// This method is used to access sparse components (marked with the [`Sparse`](crate::core::flecs::Sparse) trait)
    /// on a per-entity basis during iteration. Unlike [`field()`](Self::field) which returns an array
    /// of densely-packed components, `field_at()` retrieves a single component instance for the
    /// specified row.
    ///
    /// # Sparse Components
    ///
    /// Sparse components are not stored contiguously in memory, so they must be accessed one at
    /// a time using this method. This is more efficient for components that are only present on
    /// a small subset of entities or where add/remove speed matters more than query speed.
    ///
    /// # Arguments
    ///
    /// * `index` - The field index in the query
    /// * `row` - The row index (entity index within the current table iteration)
    ///
    /// # Returns
    ///
    /// A [`FieldAt`] accessor providing immutable access to the component
    ///
    /// # Panics
    ///
    /// Panics if the field is null or the row is out of bounds.
    ///
    /// # Example
    ///
    /// ```
    /// # use flecs_ecs::prelude::*;
    /// # #[derive(Component)]
    /// # struct Position { x: f32, y: f32 }
    /// # #[derive(Component)]
    /// # struct Velocity { x: f32, y: f32 }
    /// # let world = World::new();
    /// // Mark Position as sparse
    /// world.component::<Position>().add_trait::<flecs::Sparse>();
    ///
    /// # world.entity().set(Position { x: 10.0, y: 20.0 }).set(Velocity { x: 1.0, y: 2.0 });
    /// let query = world.query::<(&Position, &Velocity)>().build();
    ///
    /// query.run(|mut it| {
    ///     while it.next() {
    ///         let vel = it.field::<Velocity>(1);
    ///
    ///         for i in it.iter() {
    ///             // Access sparse component for this specific row
    ///             let pos = it.field_at::<Position>(0, i);
    ///             println!("Entity at ({}, {})", pos.x, pos.y);
    ///         }
    ///     }
    /// });
    /// ```
    ///
    /// # See Also
    ///
    /// - [`field_at_mut()`](Self::field_at_mut) for mutable access
    /// - [`field()`](Self::field) for dense component access
    /// - [`FieldAt`] for the returned accessor type
    pub fn field_at<T: ComponentId>(
        &self,
        index: i8,
        row: impl Into<usize>,
    ) -> FieldAt<'_, T::UnderlyingType> {
        self.field_safety_checks::<T, true, true, true>(index);
        self.world()
            .check_thread_affinity_shared::<T::UnderlyingType>();
        let row = row.into();
        #[cfg(not(feature = "flecs_term_count_64"))]
        let val = 1u32 << (index as usize);
        #[cfg(feature = "flecs_term_count_64")]
        let val = 1u64 << (index as usize);
        if self.iter.row_fields & val != 0 {
            // Sparse component: use per-row C accessor
            self.field_at_sparse_internal::<T>(index, row)
        } else {
            #[cfg(not(feature = "flecs_safety_locks"))]
            {
                FieldAt::<T::UnderlyingType>::new_dense(
                    self.field_at_dense_internal::<T>(index, row),
                )
            }
            #[cfg(feature = "flecs_safety_locks")]
            {
                let component = self.field_at_dense_internal::<T>(index, row);
                let (table, column_index) = self.field_table_column(index);
                FieldAt::<T::UnderlyingType>::new_dense(component, &self.world, table, column_index)
            }
        }
    }

    /// Get the typed field data for the specified row
    /// This function may be used to access shared fields when row is set to 0.
    ///
    /// # Arguments
    ///
    /// * `index` - The index.
    /// * `row` - The row index.
    ///
    /// # Returns
    ///
    /// An option containing a reference to the field data
    pub fn get_field_at<T: ComponentId>(
        &self,
        index: i8,
        row: impl Into<usize>,
    ) -> Option<FieldAt<'_, T::UnderlyingType>> {
        self.field_safety_checks::<T, true, true, true>(index);
        self.world()
            .check_thread_affinity_shared::<T::UnderlyingType>();
        let row = row.into();
        #[cfg(not(feature = "flecs_term_count_64"))]
        let val = 1u32 << (index as usize);
        #[cfg(feature = "flecs_term_count_64")]
        let val = 1u64 << (index as usize);
        if self.iter.row_fields & val != 0 {
            // Sparse component: use per-row C accessor
            self.get_field_at_sparse_internal::<T>(index, row)
        } else {
            // Dense component: index into the contiguous array
            let (array, _is_shared, count) = self.field_internal_parts::<T::UnderlyingType>(index);
            if array.is_null() || row >= count {
                return None;
            }
            #[cfg(not(feature = "flecs_safety_locks"))]
            {
                Some(FieldAt::<T::UnderlyingType>::new_dense(unsafe {
                    &*array.add(row)
                }))
            }
            #[cfg(feature = "flecs_safety_locks")]
            {
                let (table, column_index) = self.field_table_column(index);
                Some(FieldAt::<T::UnderlyingType>::new_dense(
                    unsafe { &*array.add(row) },
                    &self.world,
                    table,
                    column_index,
                ))
            }
        }
    }

    /// Get mutable access to a sparse component at a specific row.
    ///
    /// This method provides mutable access to sparse components on a per-entity basis during
    /// iteration. It works the same way as [`field_at()`](Self::field_at), but allows
    /// modification of the component data.
    ///
    /// # Arguments
    ///
    /// * `index` - The field index in the query
    /// * `row` - The row index (entity index within the current table iteration)
    ///
    /// # Returns
    ///
    /// A [`FieldAtMut`] accessor providing mutable access to the component
    ///
    /// # Panics
    ///
    /// Panics if the field is null or the row is out of bounds.
    ///
    /// # Example
    ///
    /// ```
    /// # use flecs_ecs::prelude::*;
    /// # #[derive(Component)]
    /// # struct Position { x: f32, y: f32 }
    /// # #[derive(Component)]
    /// # struct Velocity { x: f32, y: f32 }
    /// # let world = World::new();
    /// // Mark Position as sparse
    /// world.component::<Position>().add_trait::<flecs::Sparse>();
    ///
    /// # world.entity().set(Position { x: 10.0, y: 20.0 }).set(Velocity { x: 1.0, y: 2.0 });
    /// let query = world.query::<(&mut Position, &Velocity)>().build();
    ///
    /// query.run(|mut it| {
    ///     while it.next() {
    ///         let vel = it.field::<Velocity>(1);
    ///
    ///         for i in it.iter() {
    ///             // Mutably access sparse component for this specific row
    ///             let mut pos = it.field_at_mut::<Position>(0, i);
    ///             pos.x += vel[i].x;
    ///             pos.y += vel[i].y;
    ///         }
    ///     }
    /// });
    /// ```
    ///
    /// # See Also
    ///
    /// - [`field_at()`](Self::field_at) for immutable access
    /// - [`field_mut()`](Self::field_mut) for dense mutable component access
    /// - [`FieldAtMut`] for the returned accessor type
    #[allow(clippy::mut_from_ref)]
    pub fn field_at_mut<T: ComponentId>(
        &self,
        index: i8,
        row: impl Into<usize>,
    ) -> FieldAtMut<'_, T::UnderlyingType> {
        self.field_safety_checks::<T, false, true, true>(index);
        self.world()
            .check_thread_affinity_exclusive::<T::UnderlyingType>();
        let row = row.into();

        #[cfg(not(feature = "flecs_term_count_64"))]
        let val = 1u32 << (index as usize);
        #[cfg(feature = "flecs_term_count_64")]
        let val = 1u64 << (index as usize);
        if self.iter.row_fields & val != 0 {
            // Sparse component: use per-row C accessor
            self.field_at_sparse_internal_mut::<T>(index, row)
        } else {
            #[cfg(not(feature = "flecs_safety_locks"))]
            {
                FieldAtMut::<T::UnderlyingType>::new_dense(
                    self.field_at_dense_internal_mut::<T>(index, row),
                )
            }
            #[cfg(feature = "flecs_safety_locks")]
            {
                let component = self.field_at_dense_internal_mut::<T>(index, row);
                let (table, column_index) = self.field_table_column(index);
                FieldAtMut::<T::UnderlyingType>::new_dense(
                    component,
                    &self.world,
                    table,
                    column_index,
                )
            }
        }
    }

    /// Get the mutable typed field data for the specified row
    /// This function may be used to access shared fields when row is set to 0.
    #[allow(clippy::mut_from_ref)]
    pub fn get_field_at_mut<T: ComponentId>(
        &self,
        index: i8,
        row: impl Into<usize>,
    ) -> Option<FieldAtMut<'_, T::UnderlyingType>> {
        self.field_safety_checks::<T, false, true, true>(index);
        self.world()
            .check_thread_affinity_exclusive::<T::UnderlyingType>();
        let row = row.into();

        #[cfg(not(feature = "flecs_term_count_64"))]
        let val = 1u32 << (index as usize);
        #[cfg(feature = "flecs_term_count_64")]
        let val = 1u64 << (index as usize);
        if self.iter.row_fields & val != 0 {
            // Sparse component: use per-row C accessor
            self.get_field_at_sparse_internal_mut::<T>(index, row)
        } else {
            // Dense component: index into the contiguous array
            let (array, _is_shared, count) = self.field_internal_parts::<T::UnderlyingType>(index);
            if array.is_null() || row >= count {
                return None;
            }
            #[cfg(not(feature = "flecs_safety_locks"))]
            {
                Some(FieldAtMut::<T::UnderlyingType>::new_dense(unsafe {
                    &mut *array.add(row)
                }))
            }
            #[cfg(feature = "flecs_safety_locks")]
            {
                let (table, column_index) = self.field_table_column(index);
                Some(FieldAtMut::<T::UnderlyingType>::new_dense(
                    unsafe { &mut *array.add(row) },
                    &self.world,
                    table,
                    column_index,
                ))
            }
        }
    }

    /// Get the untyped field data for the specified row
    /// This function may be used to access shared fields when row is set to 0.
    ///
    /// # Panics
    ///
    /// Panics if `row` is out of bounds for the field.
    pub fn field_at_untyped(&self, index: i8, row: usize) -> *const c_void {
        self.field_safety_checks::<(), true, false, false>(index);

        #[cfg(not(feature = "flecs_term_count_64"))]
        let val = 1u32 << (index as usize);
        #[cfg(feature = "flecs_term_count_64")]
        let val = 1u64 << (index as usize);
        if self.iter.row_fields & val != 0 {
            self.field_at_untyped_internal(index, row)
        } else {
            let field = self.get_field_untyped_internal(index);
            if field.is_none() {
                return core::ptr::null();
            }
            let field = field.unwrap();
            assert!(
                row < field.count,
                "field_at_untyped: row {row} out of bounds (count={})",
                field.count
            );
            // SAFETY: row < count is checked above, so the offset stays within the column.
            unsafe { field.array.add(row * field.size) }
        }
    }

    /// Get the mutable untyped field data for the specified row
    /// This function may be used to access shared fields when row is set to 0.
    ///
    /// # Panics
    ///
    /// Panics if `row` is out of bounds for the field.
    pub fn field_at_untyped_mut(&self, index: i8, row: usize) -> *mut c_void {
        self.field_safety_checks::<(), false, false, false>(index);

        #[cfg(not(feature = "flecs_term_count_64"))]
        let val = 1u32 << (index as usize);
        #[cfg(feature = "flecs_term_count_64")]
        let val = 1u64 << (index as usize);

        if self.iter.row_fields & val != 0 {
            self.field_at_untyped_internal_mut(index, row)
        } else {
            let field = self.get_field_untyped_internal_mut(index);
            if field.is_none() {
                return core::ptr::null_mut();
            }
            let field = field.unwrap();
            assert!(
                row < field.count,
                "field_at_untyped_mut: row {row} out of bounds (count={})",
                field.count
            );
            // SAFETY: row < count is checked above, so the offset stays within the column.
            unsafe { field.array.add(row * field.size) }
        }
    }

    /// Get the component id of the field matched with the specified index.
    ///
    /// # Arguments
    ///
    /// * `index` - The field index.
    ///
    /// ```
    /// use flecs_ecs::prelude::*;
    ///
    /// #[derive(Component)]
    /// struct Action;
    ///
    /// #[derive(Component)]
    /// struct DerivedAction;
    ///
    /// #[derive(Component)]
    /// struct DerivedAction2;
    ///
    /// let mut world = World::new();
    ///
    /// let comp = world.component::<Action>();
    ///
    /// world.component::<DerivedAction>().is_a(Action::id());
    ///
    /// world.component::<DerivedAction>().is_a(Action::id());
    ///
    /// world.component::<DerivedAction2>().is_a(Action::id());
    ///
    /// let entity = world.entity().add(DerivedAction).add(DerivedAction2);
    ///
    /// world.new_query::<&Action>().run(|mut it| {
    ///     let mut vec = vec![];
    ///     while it.next() {
    ///         for i in it.iter() {
    ///             vec.push(it.component_id_at(0));
    ///         }
    ///     }
    ///     let id = world.component_id::<DerivedAction>();
    ///     let id2 = world.component_id::<DerivedAction2>();
    ///     assert_eq!(vec, vec![id, id2]);
    /// });
    /// ```
    pub fn component_id_at(&self, index: i8) -> Id {
        ecs_assert!(
            index < self.iter.field_count,
            FlecsErrorCode::InvalidParameter,
            index
        );

        let id = unsafe { self.iter.ids.add(index as usize).read() };
        crate::core::Id::new(id)
    }

    /// Get readonly access to entity ids of the sources.
    ///
    /// Returns an empty slice when the iterator has no sources array.
    pub fn sources(&self) -> &[Entity] {
        if self.iter.sources.is_null() {
            return &[];
        }
        // SAFETY: sources is non-null (checked above) and contains field_count entries.
        unsafe {
            core::slice::from_raw_parts(
                self.iter.sources as *const Entity,
                self.iter.field_count as usize,
            )
        }
    }

    /// Get readonly access to entity ids.
    ///
    /// # Returns
    ///
    /// The entity ids. Returns an empty slice when the iterator has no entities array.
    pub fn entities(&self) -> &[Entity] {
        if self.iter.entities.is_null() {
            return &[];
        }
        // SAFETY: entities is non-null (checked above) and contains count entries.
        unsafe { core::slice::from_raw_parts(self.iter.entities as *const Entity, self.count) }
    }

    /// Check if the current table has changed since the last iteration.
    ///
    /// # Returns
    ///
    /// Returns true if the current table has changed.
    ///
    /// # Safety
    ///
    /// Can only be used when iterating queries and/or systems.
    ///
    /// # See also
    ///
    /// * [`Query::is_changed()`]
    pub fn is_changed(&mut self) -> bool {
        unsafe { sys::ecs_iter_changed(self.iter) }
    }

    /// Returns whether the query has any data changed since the last iteration.
    ///
    /// This operation must be invoked before obtaining the iterator, as this will
    /// reset the changed state.
    ///
    /// # Returns
    ///
    /// The operation will return `true` after:
    /// - new entities have been matched with
    /// - matched entities were deleted
    /// - matched components were changed
    ///
    /// Otherwise, it will return `false`.
    ///
    /// # See also
    ///
    /// * [`TableIter::is_changed()`]
    /// * [`Query::is_changed()`]
    pub fn is_any_changed(&self) -> bool {
        unsafe { sys::ecs_query_changed(self.iter.query as *mut sys::ecs_query_t) }
    }

    /// Skip current table.
    /// This indicates to the query that the data in the current table is not
    /// modified. By default, iterating a table with a query will mark the
    /// iterated components as dirty if they are annotated with `InOut` or Out.
    ///
    /// When this operation is invoked, the components of the current table will
    /// not be marked dirty.
    pub fn skip(&mut self) {
        unsafe { sys::ecs_iter_skip(self.iter) };
    }

    /// # Returns
    ///
    /// Return group id for current table
    ///
    /// # Safety
    ///
    /// grouped queries only
    // TODO: this should be &self after I upgrade flecs
    pub fn group_id(&self) -> sys::ecs_id_t {
        unsafe { sys::ecs_iter_get_group(self.iter) }
    }

    #[inline(always)]
    pub(crate) fn field_result<T: ComponentId>(
        &self,
        index: i8,
    ) -> Result<Field<'_, T::UnderlyingType, true>, FieldError> {
        self.field_safety_checks::<T, true, true, false>(index);
        self.world()
            .check_thread_affinity_shared::<T::UnderlyingType>();

        self.field_result_internal::<T::UnderlyingType, true>(index)
    }

    #[inline(always)]
    pub(crate) fn field_result_mut<T: ComponentId>(
        &self,
        index: i8,
    ) -> Result<FieldMut<'_, T::UnderlyingType, true>, FieldError> {
        self.field_safety_checks::<T, false, true, false>(index);
        self.world()
            .check_thread_affinity_exclusive::<T::UnderlyingType>();

        self.field_result_internal_mut::<T::UnderlyingType, true>(index)
    }

    #[inline(always)]
    pub(crate) fn field_result_internal<T, const LOCK: bool>(
        &self,
        index: i8,
    ) -> Result<Field<'_, T, LOCK>, FieldError> {
        let (array, is_shared, count) = self.field_internal_parts::<T>(index);

        if count == 0 || array.is_null() {
            return Err(FieldError::NoMatchesCount0);
        }

        // SAFETY: we already validated index/type before calling
        let slice = unsafe { core::slice::from_raw_parts(array as *const T, count) };

        #[cfg(not(feature = "flecs_safety_locks"))]
        {
            //does not actually do any locking
            Ok(Field::<T, LOCK>::new(slice, is_shared))
        }

        #[cfg(feature = "flecs_safety_locks")]
        {
            let (table, column_index) =
                unsafe { flecs_field_table_column(self.iter, index as usize) };
            debug_assert!(!table.is_null(), "field {index} has no table column");
            let world_ref = &self.world;
            if world_ref.is_currently_multithreaded() {
                Field::<T, LOCK>::new_result::<true>(
                    slice,
                    is_shared,
                    world_ref.stage_id(),
                    column_index,
                    index,
                    unsafe { NonNull::new_unchecked(table) },
                    world_ref,
                )
            } else {
                Field::<T, LOCK>::new_result::<false>(
                    slice,
                    is_shared,
                    world_ref.stage_id(),
                    column_index,
                    index,
                    unsafe { NonNull::new_unchecked(table) },
                    world_ref,
                )
            }
        }
    }

    #[inline(always)]
    pub(crate) fn field_result_internal_mut<T, const LOCK: bool>(
        &self,
        index: i8,
    ) -> Result<FieldMut<'_, T, LOCK>, FieldError> {
        let (array, is_shared, count) = self.field_internal_parts::<T>(index);

        if count == 0 || array.is_null() {
            return Err(FieldError::NoMatchesCount0);
        }

        // SAFETY: we already validated index/type before calling
        let slice = unsafe { core::slice::from_raw_parts_mut(array, count) };

        #[cfg(not(feature = "flecs_safety_locks"))]
        {
            //does not actually do any locking
            Ok(FieldMut::<T, LOCK>::new(slice, is_shared))
        }

        #[cfg(feature = "flecs_safety_locks")]
        {
            let (table, column_index) =
                unsafe { flecs_field_table_column(self.iter, index as usize) };
            debug_assert!(!table.is_null(), "field {index} has no table column");
            let world_ref = &self.world;

            if world_ref.is_currently_multithreaded() {
                FieldMut::<T, LOCK>::new_result::<true>(
                    slice,
                    is_shared,
                    world_ref.stage_id(),
                    column_index,
                    index,
                    unsafe { NonNull::new_unchecked(table) },
                    world_ref,
                )
            } else {
                FieldMut::<T, LOCK>::new_result::<false>(
                    slice,
                    is_shared,
                    world_ref.stage_id(),
                    column_index,
                    index,
                    unsafe { NonNull::new_unchecked(table) },
                    world_ref,
                )
            }
        }
    }

    #[inline(always)]
    pub(crate) fn get_field_internal<T, const LOCK: bool>(
        &self,
        index: i8,
    ) -> Option<Field<'a, T, LOCK>> {
        let (array, is_shared, count) = self.field_internal_parts::<T>(index);
        if count == 0 || array.is_null() {
            return None;
        }

        // SAFETY: we already validated index/type before calling
        let slice = unsafe { core::slice::from_raw_parts(array as *const T, count) };

        #[cfg(not(feature = "flecs_safety_locks"))]
        {
            //does not actually do any locking
            Some(Field::<T, LOCK>::new(slice, is_shared))
        }

        #[cfg(feature = "flecs_safety_locks")]
        {
            let (table, column_index) =
                unsafe { flecs_field_table_column(self.iter, index as usize) };
            debug_assert!(!table.is_null(), "field {index} has no table column");
            let world_ref = &self.world;
            if world_ref.is_currently_multithreaded() {
                Some(Field::<T, LOCK>::new::<true>(
                    slice,
                    is_shared,
                    world_ref.stage_id(),
                    column_index,
                    index,
                    unsafe { NonNull::new_unchecked(table) },
                    world_ref,
                ))
            } else {
                Some(Field::<T, LOCK>::new::<false>(
                    slice,
                    is_shared,
                    world_ref.stage_id(),
                    column_index,
                    index,
                    unsafe { NonNull::new_unchecked(table) },
                    world_ref,
                ))
            }
        }
    }

    #[inline(always)]
    pub(crate) fn field_internal<T, const LOCK: bool>(&self, index: i8) -> Field<'a, T, LOCK> {
        let (array, is_shared, count) = self.field_internal_parts::<T>(index);
        // Optional fields that are not set have null array / count=0.
        // Return empty slice — caller must use it.is_set(index) before indexing.
        // This matches C++ where it.field<Optional>() returns null for unset optionals.
        let slice = if count == 0 || array.is_null() {
            &[]
        } else {
            // SAFETY: array is non-null, count > 0, pointer is valid for 'a
            unsafe { core::slice::from_raw_parts(array as *const T, count) }
        };

        #[cfg(not(feature = "flecs_safety_locks"))]
        {
            //does not actually do any locking
            Field::<T, LOCK>::new(slice, is_shared)
        }

        #[cfg(feature = "flecs_safety_locks")]
        {
            // Optional unset fields have empty slice — return without acquiring lock.
            if slice.is_empty() {
                return Field::<T, LOCK>::new_empty(is_shared);
            }
            let (table, column_index) =
                unsafe { flecs_field_table_column(self.iter, index as usize) };
            debug_assert!(!table.is_null(), "field {index} has no table column");
            let world_ref = &self.world;
            if world_ref.is_currently_multithreaded() {
                Field::<T, LOCK>::new::<true>(
                    slice,
                    is_shared,
                    world_ref.stage_id(),
                    column_index,
                    index,
                    unsafe { NonNull::new_unchecked(table) },
                    world_ref,
                )
            } else {
                Field::<T, LOCK>::new::<false>(
                    slice,
                    is_shared,
                    world_ref.stage_id(),
                    column_index,
                    index,
                    unsafe { NonNull::new_unchecked(table) },
                    world_ref,
                )
            }
        }
    }

    #[inline(always)]
    fn field_internal_parts<T>(&self, index: i8) -> (*mut T, bool, usize) {
        let is_shared = !self.is_self(index);
        // If a shared column is retrieved with 'column', there will only be a
        // single value. Ensure that the application does not accidentally read
        // out of bounds.
        let count = if is_shared {
            1
        } else {
            // If column is owned, there will be as many values as there are
            // entities.
            self.count()
        };

        let array = flecs_field::<T>(self.iter, index);

        (array, is_shared, count)
    }

    #[inline(always)]
    fn field_untyped_internal_parts(&self, index: i8) -> (*mut c_void, bool, usize, usize) {
        let size = unsafe { sys::ecs_field_size(self.iter, index) };
        let is_shared = !self.is_self(index);

        // If a shared column is retrieved with 'column', there will only be a
        // single value. Ensure that the application does not accidentally read
        // out of bounds.
        let count = if is_shared {
            1
        } else {
            // If column is owned, there will be as many values as there are entities
            self.count()
        };

        let array = flecs_field_with_size(self.iter, size, index);

        (array, is_shared, count, size)
    }

    /// the “Option” version
    #[inline(always)]
    pub(crate) fn get_field_internal_mut<T, const LOCK: bool>(
        &self,
        index: i8,
    ) -> Option<FieldMut<'a, T, LOCK>> {
        let (array, is_shared, count) = self.field_internal_parts::<T>(index);
        if count == 0 || array.is_null() {
            return None;
        }

        let slice = unsafe { core::slice::from_raw_parts_mut(array, count) };

        #[cfg(not(feature = "flecs_safety_locks"))]
        {
            Some(FieldMut::<T, LOCK>::new(slice, is_shared))
        }

        #[cfg(feature = "flecs_safety_locks")]
        {
            let (table, column_index) =
                unsafe { flecs_field_table_column(self.iter, index as usize) };
            debug_assert!(!table.is_null(), "field {index} has no table column");
            let world_ref = &self.world;
            if world_ref.is_currently_multithreaded() {
                Some(FieldMut::<T, LOCK>::new::<true>(
                    slice,
                    is_shared,
                    world_ref.stage_id(),
                    column_index,
                    index,
                    unsafe { NonNull::new_unchecked(table) },
                    world_ref,
                ))
            } else {
                Some(FieldMut::<T, LOCK>::new::<false>(
                    slice,
                    is_shared,
                    world_ref.stage_id(),
                    column_index,
                    index,
                    unsafe { NonNull::new_unchecked(table) },
                    world_ref,
                ))
            }
        }
    }

    /// the “panic” version
    #[inline(always)]
    pub(crate) fn field_internal_mut<T, const LOCK: bool>(
        &self,
        index: i8,
    ) -> FieldMut<'a, T, LOCK> {
        let (array, is_shared, count) = self.field_internal_parts::<T>(index);
        // Optional unset fields have null array / count=0 — return empty slice.
        let slice = if count == 0 || array.is_null() {
            &mut []
        } else {
            // SAFETY: array is non-null, count > 0, pointer is valid for 'a
            unsafe { core::slice::from_raw_parts_mut(array, count) }
        };

        #[cfg(not(feature = "flecs_safety_locks"))]
        {
            FieldMut::<T, LOCK>::new(slice, is_shared)
        }

        #[cfg(feature = "flecs_safety_locks")]
        {
            if slice.is_empty() {
                return FieldMut::<T, LOCK>::new_empty(is_shared);
            }
            let (table, column_index) =
                unsafe { flecs_field_table_column(self.iter, index as usize) };
            debug_assert!(!table.is_null(), "field {index} has no table column");
            let world_ref = &self.world;
            if world_ref.is_currently_multithreaded() {
                FieldMut::<T, LOCK>::new::<true>(
                    slice,
                    is_shared,
                    world_ref.stage_id(),
                    column_index,
                    index,
                    unsafe { NonNull::new_unchecked(table) },
                    world_ref,
                )
            } else {
                FieldMut::<T, LOCK>::new::<false>(
                    slice,
                    is_shared,
                    world_ref.stage_id(),
                    column_index,
                    index,
                    unsafe { NonNull::new_unchecked(table) },
                    world_ref,
                )
            }
        }
    }

    pub(crate) fn field_untyped_internal(&self, index: i8) -> FieldUntyped {
        let (array, is_shared, count, size) = self.field_untyped_internal_parts(index);

        if count == 0 || array.is_null() {
            panic!(
                "field_untyped_internal: no values at index {index} — ensure the field exists and has entries"
            );
        }

        FieldUntyped::new(array, size, count, is_shared)
    }

    pub(crate) fn get_field_untyped_internal(&self, index: i8) -> Option<FieldUntyped> {
        let (array, is_shared, count, size) = self.field_untyped_internal_parts(index);

        if count == 0 || array.is_null() {
            return None;
        }

        Some(FieldUntyped::new(array, size, count, is_shared))
    }

    pub(crate) fn field_untyped_internal_mut(&self, index: i8) -> FieldUntypedMut {
        let (array, is_shared, count, size) = self.field_untyped_internal_parts(index);

        if count == 0 || array.is_null() {
            panic!(
                "field_untyped_internal_mut: no values at index {index} — ensure the field exists and has entries"
            );
        }

        FieldUntypedMut::new(array, size, count, is_shared)
    }

    pub(crate) fn get_field_untyped_internal_mut(&self, index: i8) -> Option<FieldUntypedMut> {
        let (array, is_shared, count, size) = self.field_untyped_internal_parts(index);

        if count == 0 || array.is_null() {
            return None;
        }

        Some(FieldUntypedMut::new(array, size, count, is_shared))
    }

    pub(crate) fn field_at_untyped_internal(&self, index: i8, row: usize) -> *const c_void {
        let size = unsafe { sys::ecs_field_size(self.iter, index) };
        unsafe { sys::ecs_field_at_w_size(self.iter, size, index, row as i32) }
    }

    pub(crate) fn field_at_untyped_internal_mut(&self, index: i8, row: usize) -> *mut c_void {
        let size = unsafe { sys::ecs_field_size(self.iter, index) };
        unsafe { sys::ecs_field_at_w_size(self.iter, size, index, row as i32) }
    }

    #[cfg(feature = "flecs_safety_locks")]
    #[inline(always)]
    fn field_table_column(&self, index: i8) -> (NonNull<sys::ecs_table_t>, i16) {
        // SAFETY: callers only invoke this for a set, dense (non-sparse) field,
        // so the field resolves to a table column, matching field_internal.
        unsafe {
            let (table, column) = flecs_field_table_column(self.iter, index as usize);
            debug_assert!(!table.is_null(), "field {index} has no table column");
            (NonNull::new_unchecked(table), column)
        }
    }

    fn field_at_dense_internal<T: ComponentId>(&self, index: i8, row: usize) -> &T::UnderlyingType {
        let (array, _is_shared, count) = self.field_internal_parts::<T::UnderlyingType>(index);
        assert!(
            !array.is_null(),
            "field_at_dense: null array at index {index}"
        );
        assert!(
            row < count,
            "field_at_dense: row {row} out of bounds (count={count})"
        );
        // SAFETY: `array` is non-null (asserted above), `row < count` (asserted above).
        // The pointer is valid for reads for the duration of the borrow of `self`.
        unsafe { &*array.add(row) }
    }

    #[allow(clippy::mut_from_ref)]
    fn field_at_dense_internal_mut<T: ComponentId>(
        &self,
        index: i8,
        row: usize,
    ) -> &mut T::UnderlyingType {
        let (array, _is_shared, count) = self.field_internal_parts::<T::UnderlyingType>(index);
        assert!(
            !array.is_null(),
            "field_at_dense_mut: null array at index {index}"
        );
        assert!(
            row < count,
            "field_at_dense_mut: row {row} out of bounds (count={count})"
        );
        // SAFETY: `array` is non-null (asserted above), `row < count` (asserted above).
        // Aliasing is the caller's responsibility, matching the contract of field_internal_mut.
        unsafe { &mut *array.add(row) }
    }

    pub(crate) fn field_at_sparse_internal<T>(
        &self,
        index: i8,
        row: usize,
    ) -> FieldAt<'_, T::UnderlyingType>
    where
        T: ComponentId,
    {
        let component = unsafe {
            sys::ecs_field_at_w_size(
                self.iter,
                const { core::mem::size_of::<T::UnderlyingType>() },
                index,
                row as i32,
            ) as *mut T::UnderlyingType
        };

        if component.is_null() {
            panic!("Tried to access field at index {index} for row {row}, but field row is null.");
        }

        let component_ref = unsafe { &*component };

        #[cfg(not(feature = "flecs_safety_locks"))]
        {
            FieldAt::<T::UnderlyingType>::new(component_ref)
        }

        #[cfg(feature = "flecs_safety_locks")]
        {
            let world_ref = &self.world;
            let tr = if self.iter.trs.is_null() {
                core::ptr::null()
            } else {
                unsafe { *self.iter.trs.add(index as usize) }
            };
            let idr = if !tr.is_null() {
                unsafe { (*tr).hdr.cr }
            } else {
                let comp_id = unsafe { *self.iter.ids.add(index as usize) };
                unsafe { sys::flecs_components_get(world_ref.raw_world.as_ptr(), comp_id) }
            };

            FieldAt::<T::UnderlyingType>::new(component_ref, world_ref, unsafe {
                NonNull::new_unchecked(idr)
            })
        }
    }

    pub(crate) fn get_field_at_sparse_internal<T>(
        &self,
        index: i8,
        row: usize,
    ) -> Option<FieldAt<'_, T::UnderlyingType>>
    where
        T: ComponentId,
    {
        let component = unsafe {
            sys::ecs_field_at_w_size(
                self.iter,
                const { core::mem::size_of::<T::UnderlyingType>() },
                index,
                row as i32,
            ) as *mut T::UnderlyingType
        };

        if component.is_null() {
            return None;
        }

        let component_ref = unsafe { &*component };

        #[cfg(not(feature = "flecs_safety_locks"))]
        {
            Some(FieldAt::<T::UnderlyingType>::new(component_ref))
        }

        #[cfg(feature = "flecs_safety_locks")]
        {
            let world_ref = &self.world;
            let tr = if self.iter.trs.is_null() {
                core::ptr::null()
            } else {
                unsafe { *self.iter.trs.add(index as usize) }
            };
            let idr = if !tr.is_null() {
                unsafe { (*tr).hdr.cr }
            } else {
                let comp_id = unsafe { *self.iter.ids.add(index as usize) };
                unsafe { sys::flecs_components_get(world_ref.raw_world.as_ptr(), comp_id) }
            };

            Some(FieldAt::<T::UnderlyingType>::new(
                component_ref,
                world_ref,
                unsafe { NonNull::new_unchecked(idr) },
            ))
        }
    }

    pub(crate) fn field_at_sparse_internal_mut<T>(
        &self,
        index: i8,
        row: usize,
    ) -> FieldAtMut<'_, T::UnderlyingType>
    where
        T: ComponentId,
    {
        let component = unsafe {
            sys::ecs_field_at_w_size(
                self.iter,
                const { core::mem::size_of::<T::UnderlyingType>() },
                index,
                row as i32,
            ) as *mut T::UnderlyingType
        };

        if component.is_null() {
            panic!("Tried to access field at index {index} for row {row}, but field row is null.");
        }

        let component_ref = unsafe { &mut *component };

        #[cfg(not(feature = "flecs_safety_locks"))]
        {
            FieldAtMut::<T::UnderlyingType>::new(component_ref)
        }

        #[cfg(feature = "flecs_safety_locks")]
        {
            let tr = unsafe { *self.iter.trs.add(index as usize) };
            let world_ref = &self.world;

            let idr = if !tr.is_null() {
                unsafe { (*tr).hdr.cr }
            } else {
                let comp_id = unsafe { *self.iter.ids.add(index as usize) };
                unsafe { sys::flecs_components_get(world_ref.raw_world.as_ptr(), comp_id) }
            };

            FieldAtMut::<T::UnderlyingType>::new(component_ref, world_ref, unsafe {
                NonNull::new_unchecked(idr)
            })
        }
    }

    pub(crate) fn get_field_at_sparse_internal_mut<T>(
        &self,
        index: i8,
        row: usize,
    ) -> Option<FieldAtMut<'_, T::UnderlyingType>>
    where
        T: ComponentId,
    {
        let component = unsafe {
            sys::ecs_field_at_w_size(
                self.iter,
                const { core::mem::size_of::<T::UnderlyingType>() },
                index,
                row as i32,
            ) as *mut T::UnderlyingType
        };

        if component.is_null() {
            return None;
        }

        let component_ref = unsafe { &mut *component };

        #[cfg(not(feature = "flecs_safety_locks"))]
        {
            Some(FieldAtMut::<T::UnderlyingType>::new(component_ref))
        }

        #[cfg(feature = "flecs_safety_locks")]
        {
            let world_ref = &self.world;
            let tr = if self.iter.trs.is_null() {
                core::ptr::null()
            } else {
                unsafe { *self.iter.trs.add(index as usize) }
            };
            let idr = if !tr.is_null() {
                unsafe { (*tr).hdr.cr }
            } else {
                let comp_id = unsafe { *self.iter.ids.add(index as usize) };
                unsafe { sys::flecs_components_get(world_ref.raw_world.as_ptr(), comp_id) }
            };

            Some(FieldAtMut::<T::UnderlyingType>::new(
                component_ref,
                world_ref,
                unsafe { NonNull::new_unchecked(idr) },
            ))
        }
    }

    /// Forward to each.
    /// If a system has an each callback registered, this operation will forward
    /// the current iterator to the each callback.
    pub fn each(&mut self) {
        if let Some(each) = self.iter.callback {
            unsafe {
                each(self.iter);
            }
        }
    }

    /// Iterate targets for pair field.
    ///
    /// # Arguments
    ///
    /// * index: the field index
    /// * func: callback invoked for each target with the signature fn(EntityView entity)
    ///
    /// # Example
    ///
    /// ```
    /// use flecs_ecs::prelude::*;
    ///
    /// let world = World::new();
    ///
    /// let likes = world.entity();
    /// let pizza = world.entity();
    /// let salad = world.entity();
    /// let alice = world.entity().add((likes, pizza)).add((likes, salad));
    ///
    /// let q = world
    ///     .query::<()>()
    ///     .with((likes, id::<flecs::Any>()))
    ///     .build();
    ///
    /// let mut count = 0;
    /// let mut tgt_count = 0;
    ///
    /// q.run(|mut it| {
    ///     while it.next() {
    ///         for row in it.iter() {
    ///             let e = it.entity(row);
    ///             assert_eq!(e, alice);
    ///
    ///             it.targets(0, |tgt| {
    ///                 if tgt_count == 0 {
    ///                     assert_eq!(tgt, pizza);
    ///                 }
    ///                 if tgt_count == 1 {
    ///                     assert_eq!(tgt, salad);
    ///                 }
    ///                 tgt_count += 1;
    ///             });
    ///
    ///             count += 1;
    ///         }
    ///     }
    /// });
    ///
    /// assert_eq!(count, 1);
    /// assert_eq!(tgt_count, 2);
    /// ```
    pub fn targets(&self, index: i8, mut func: impl FnMut(EntityView)) {
        ecs_assert!(!self.iter.table.is_null(), FlecsErrorCode::InvalidOperation);
        ecs_assert!(
            index < self.iter.field_count,
            FlecsErrorCode::InvalidOperation
        );
        ecs_assert!(
            unsafe { sys::ecs_field_is_set(self.iter, index) },
            FlecsErrorCode::InvalidOperation
        );
        let table_type = unsafe { sys::ecs_table_get_type(self.iter.table) };
        let table_record = unsafe { *self.iter.trs.add(index as usize) };
        let mut i = unsafe { (*table_record).index };
        let end = i + unsafe { (*table_record).count };
        while i < end {
            let id = unsafe { *(*table_type).array.add(i as usize) };
            ecs_assert!(
                ecs_is_pair(id),
                FlecsErrorCode::InvalidParameter,
                "field does not match a pair"
            );
            let target = EntityView::new_from(
                self.world(),
                ecs_second(id, unsafe { WorldRef::from_ptr(self.iter.world) }),
            );
            func(target);
            i += 1;
        }
    }

    #[inline(always)]
    pub(crate) fn internal_next(&mut self) -> bool {
        if IS_RUN {
            #[cfg(any(debug_assertions, feature = "flecs_force_enable_ecs_asserts"))]
            if self.iter.flags & sys::EcsIterIsValid != 0 && !self.iter.table.is_null() {
                unsafe {
                    sys::ecs_table_unlock(self.iter.world, self.iter.table);
                };
            }
        }

        let result = match self.iter.next {
            Some(next) => {
                let r = unsafe { next(self.iter) };
                self.count = self.iter.count as usize;
                r
            }
            None => {
                // no more items → clear valid and return
                self.iter.flags &= !sys::EcsIterIsValid;
                return false;
            }
        };

        self.iter.flags |= sys::EcsIterIsValid;
        if IS_RUN {
            #[cfg(any(debug_assertions, feature = "flecs_force_enable_ecs_asserts"))]
            {
                if result && !self.iter.table.is_null() {
                    unsafe {
                        sys::ecs_table_lock(self.iter.world, self.iter.table);
                    };
                }
            }
        }

        result
    }
}

impl<P> TableIter<'_, true, P>
where
    P: ComponentId,
{
    /// Progress iterator.
    /// this operation is not available on functions where the iterator is not from `.run()`
    #[allow(clippy::should_implement_trait)]
    #[inline(always)]
    pub fn next(&mut self) -> bool {
        self.internal_next()
    }

    /// Free iterator resources.
    /// This operation only needs to be called when the iterator is not iterated
    /// until completion (e.g. the last call to `next()` did not return false).
    ///
    /// Failing to call this operation on an unfinished iterator will throw a
    /// `fatal LEAK_DETECTED` error.
    ///
    /// # Example
    ///
    /// ```
    /// use flecs_ecs::prelude::*;
    ///
    /// #[derive(Debug, Component, Default)]
    /// pub struct Position {
    ///     pub x: i32,
    ///     pub y: i32,
    /// }
    ///
    /// let world = World::new();
    ///
    /// world.new_query::<&mut Position>().run(|it| {
    ///     // this will ensure that the iterator is freed and no assertion will happen
    ///     it.fini();
    /// });
    /// ```
    pub fn fini(self) {
        #[cfg(any(debug_assertions, feature = "flecs_force_enable_ecs_asserts"))]
        if self.iter.flags & sys::EcsIterIsValid != 0 && !self.iter.table.is_null() {
            unsafe {
                sys::ecs_table_unlock(self.iter.world, self.iter.table);
            };
        }

        unsafe {
            sys::ecs_iter_fini(self.iter);
        }
    }
}

#[inline(always)]
pub(crate) fn table_lock(_world_ptr: *mut sys::ecs_world_t, _table_ptr: *mut sys::ecs_table_t) {
    #[cfg(any(debug_assertions, feature = "flecs_force_enable_ecs_asserts"))]
    unsafe {
        sys::ecs_table_lock(_world_ptr, _table_ptr);
    }
}

#[inline(always)]
pub(crate) fn table_unlock(_world_ptr: *mut sys::ecs_world_t, _table_ptr: *mut sys::ecs_table_t) {
    #[cfg(any(debug_assertions, feature = "flecs_force_enable_ecs_asserts"))]
    unsafe {
        sys::ecs_table_unlock(_world_ptr, _table_ptr);
    }
}

/// Balances [`table_lock`] with [`table_unlock`] across early returns and
/// unwinds. Like both of those it compiles to nothing when the flecs table
/// asserts are disabled.
pub(crate) struct IterTableLock {
    world_ptr: *mut sys::ecs_world_t,
    table_ptr: *mut sys::ecs_table_t,
}

impl IterTableLock {
    #[inline(always)]
    pub(crate) fn new(
        world_ptr: *mut sys::ecs_world_t,
        table_ptr: *mut sys::ecs_table_t,
    ) -> IterTableLock {
        table_lock(world_ptr, table_ptr);
        IterTableLock {
            world_ptr,
            table_ptr,
        }
    }
}

impl Drop for IterTableLock {
    #[inline(always)]
    fn drop(&mut self) {
        table_unlock(self.world_ptr, self.table_ptr);
    }
}
