//! Table access and iteration utilities.
//!
//! This module provides direct access to tables and their component data. Tables are the
//! internal storage structure in Flecs that group entities with the same component composition
//! (archetype). Understanding and working with tables is essential for high-performance ECS operations.
//!
//! # What are Tables?
//!
//! In Flecs, entities with the same set of components are stored together in tables. Each table
//! represents a unique archetype - a specific combination of components. This storage model enables:
//!
//! - **Cache-friendly iteration**: Components of the same type are stored contiguously in memory
//! - **Fast component access**: Direct array indexing without indirection
//! - **Efficient queries**: Query matching can be done at the table level
//!
//! # Module Organization
//!
//! - [`Table`]: The main wrapper providing access to table metadata and component arrays
//! - [`TableIter`]: Iterator for traversing tables and accessing component data during queries
//! - [`Field`] and [`FieldMut`]: Typed access to component columns (fields) within a table
//! - [`FieldUntyped`] and [`FieldUntypedMut`]: Untyped access for dynamic component types
//! - [`FieldIndex`]: Type-safe index for accessing specific entity rows in a field
//! - [`TableFlags`]: Bitflags describing table properties and capabilities
//!
//! # Common Use Cases
//!
//! ## Accessing Table Information
//!
//! ```rust, no_run
//! # use flecs_ecs::prelude::*;
//! # #[derive(Component)]
//! # struct Position { x: f32, y: f32 }
//! # #[derive(Component)]
//! # struct Velocity { x: f32, y: f32 }
//! # let world = World::new();
//! let e = world
//!     .entity()
//!     .set(Position { x: 1.0, y: 2.0 })
//!     .set(Velocity { x: 0.0, y: 0.0 });
//! if let Some(table) = e.table() {
//!     // Access table directly
//!     println!(
//!         "Entity is in table: {table:?}, with archetype: {:?}",
//!         table.archetype()
//!     );
//! }
//! ```
//!
//! ## Iterating with Fields
//!
//! The most common pattern is using [`TableIter`] within queries to access component data:
//!
//! ```rust,no_run
//! # use flecs_ecs::prelude::*;
//! # #[derive(Component)]
//! # struct Position { x: f32, y: f32 }
//! # #[derive(Component)]
//! # struct Velocity { x: f32, y: f32 }
//! # let world = World::new();
//! # world.entity().set(Position { x: 1.0, y: 2.0 }).set(Velocity { x: 0.1, y: 0.2 });
//! let query = world.new_query::<(&mut Position, &Velocity)>();
//!
//! query.run(|mut it| {
//!     while it.next() {
//!         // For each matching table
//!         let mut pos = it.field_mut::<Position>(0);
//!         let vel = it.field::<Velocity>(1);
//!
//!         for i in it.iter() {
//!             // For each entity in the table
//!             pos[i].x += vel[i].x;
//!             pos[i].y += vel[i].y;
//!         }
//!     }
//! });
//! ```
//!
//! ## Type-Safe Indexing
//!
//! [`FieldIndex`] provides type-safe indexing when iterating:
//!
//! ```rust,no_run
//! # use flecs_ecs::prelude::*;
//! # #[derive(Component)]
//! # struct Position { x: f32, y: f32 }
//! # let world = World::new();
//! # world.entity().set(Position { x: 1.0, y: 2.0 });
//! # let query = world.new_query::<&Position>();
//! query.run(|mut it| {
//!     while it.next() {
//!         let pos = it.field::<Position>(0);
//!
//!         // iter() returns FieldIndex
//!         for i in it.iter() {
//!             let position = &pos[i];
//!         }
//!     }
//! });
//! ```

mod field;
mod flags;
mod iter;
mod multi_src_get;

use core::{ffi::CStr, ffi::c_void, ptr::NonNull};
#[cfg(feature = "flecs_safety_locks")]
pub(crate) use field::flecs_field_table_column;
pub use field::{Field, FieldAt, FieldAtMut, FieldIndex, FieldMut, FieldUntyped, FieldUntypedMut};
pub(crate) use field::{flecs_field, flecs_field_with_size};
pub use multi_src_get::*;

pub use flags::TableFlags;
pub use iter::{FieldError, TableIter};
#[cfg(any(debug_assertions, feature = "flecs_force_enable_ecs_asserts"))]
pub(crate) use iter::{table_lock, table_unlock};

use crate::core::*;
use crate::sys;

#[cfg(feature = "std")]
extern crate std;

extern crate alloc;
use alloc::string::String;

/// A wrapper class that gives direct access to the component arrays of a table, the table data
#[derive(Debug, Clone, Copy, Eq)]
#[repr(C)]
pub struct Table<'a> {
    pub(crate) table: NonNull<sys::ecs_table_t>,
    world: WorldRef<'a>,
}

impl PartialEq for Table<'_> {
    fn eq(&self, other: &Self) -> bool {
        self.table == other.table
    }
}

impl<'a> Table<'a> {
    /// Creates a wrapper around a table
    ///
    /// # Arguments
    ///
    /// * `world` - The world the table is in
    /// * `table` - The table to wrap
    ///
    /// # Safety
    ///
    /// `table` must point to a table that is currently alive and owned by
    /// `world`, and must remain alive for the lifetime `'a`. Passing a stale
    /// pointer (e.g. one obtained from [`Table::raw_table_ptr()`] after the
    /// table was deleted) results in undefined behavior when the returned
    /// `Table` is used.
    pub unsafe fn new(world: impl WorldProvider<'a>, table: NonNull<sys::ecs_table_t>) -> Self {
        Self {
            world: world.world(),
            table,
        }
    }

    /// Returns the raw table pointer
    pub fn raw_table_ptr(&self) -> *mut sys::ecs_table_t {
        self.table.as_ptr()
    }
}

#[derive(Debug, Clone, Copy)]
pub struct TableRange<'a> {
    pub table: Table<'a>,
    offset: i32,
    count: i32,
}

impl<'a> TableRange<'a> {
    /// Creates a new table range
    ///
    /// # Arguments
    ///
    /// * `table` - The table to wrap
    /// * `offset` - The offset to start from
    /// * `count` - The count of the range
    ///
    /// # Returns
    ///
    /// A new table range
    pub fn new(table: Table<'a>, offset: i32, count: i32) -> Self {
        Self {
            table,
            offset,
            count,
        }
    }

    /// Creates a new table range from raw table ptr.
    ///
    /// # Arguments
    ///
    /// * `world` - The world the table is in
    /// * `table` - The table to wrap
    /// * `offset` - The offset to start from
    /// * `count` - The count of the range
    ///
    /// # Returns
    ///
    /// A new table range
    ///
    /// # Safety
    ///
    /// The world and table pointers must be valid
    pub(crate) fn new_raw(
        world: impl WorldProvider<'a>,
        table: NonNull<sys::ecs_table_t>,
        offset: i32,
        count: i32,
    ) -> Self {
        Self {
            // SAFETY: caller upholds this function's contract that the world
            // and table pointers are valid.
            table: unsafe { Table::new(world, table) },
            offset,
            count,
        }
    }
}

/// RAII guard providing read access to the entity ids of a table (range).
///
/// While the guard is alive, the world is kept in deferred mode: structural
/// changes (creating, deleting or moving entities) are queued and applied when
/// the guard is dropped, so the entity array cannot be reallocated or freed
/// while it is borrowed.
///
/// Dereferences to `&[Entity]`.
pub struct TableEntities<'a> {
    slice: &'a [Entity],
    world: WorldRef<'a>,
}

impl core::ops::Deref for TableEntities<'_> {
    type Target = [Entity];

    #[inline(always)]
    fn deref(&self) -> &Self::Target {
        self.slice
    }
}

impl Drop for TableEntities<'_> {
    fn drop(&mut self) {
        self.world.defer_end();
    }
}

/// RAII guard providing mutable access to a table column (component array).
///
/// While the guard is alive, the world is kept in deferred mode: structural
/// changes (creating, deleting or moving entities) are queued and applied when
/// the guard is dropped, so the column storage cannot be reallocated or freed
/// while it is borrowed. When the `flecs_safety_locks` feature (part of the
/// default features) is enabled, a write lock is additionally held on the
/// column for the lifetime of the guard: constructing a second guard (or
/// field accessor) for the same column while one is alive panics instead of
/// aliasing the `&mut` data.
///
/// Dereferences to `&[T]` / `&mut [T]`.
pub struct TableColumnMut<'a, T> {
    slice: &'a mut [T],
    world: WorldRef<'a>,
    #[cfg(feature = "flecs_safety_locks")]
    table: NonNull<sys::ecs_table_t>,
    #[cfg(feature = "flecs_safety_locks")]
    column_index: i16,
    #[cfg(feature = "flecs_safety_locks")]
    locks: core::ptr::NonNull<StageLocks>,
}

impl<'a, T> TableColumnMut<'a, T> {
    /// # Safety
    ///
    /// `table` must be a live table owned by `world`, `column_index` must be a
    /// valid column index of that table, and `ptr`/`count` must describe that
    /// column's component array of type `T`.
    unsafe fn new(
        world: WorldRef<'a>,
        _table: NonNull<sys::ecs_table_t>,
        _column_index: i16,
        ptr: *mut T,
        count: usize,
    ) -> Self {
        #[cfg(feature = "flecs_safety_locks")]
        let locks = {
            let locks = stage_locks_dyn(&world);
            get_table_column_lock_write_begin(&world, locks, _table.as_ptr(), _column_index);
            locks
        };
        world.defer_begin();
        // SAFETY: caller guarantees ptr/count describe the column array. The
        // world stays in deferred mode until this guard is dropped, so the
        // array cannot be reallocated or freed while the slice is alive, and
        // the column write lock prevents aliased access through other guards.
        let slice = unsafe { core::slice::from_raw_parts_mut(ptr, count) };
        Self {
            slice,
            world,
            #[cfg(feature = "flecs_safety_locks")]
            table: _table,
            #[cfg(feature = "flecs_safety_locks")]
            column_index: _column_index,
            #[cfg(feature = "flecs_safety_locks")]
            locks,
        }
    }
}

impl<T> core::ops::Deref for TableColumnMut<'_, T> {
    type Target = [T];

    #[inline(always)]
    fn deref(&self) -> &Self::Target {
        self.slice
    }
}

impl<T> core::ops::DerefMut for TableColumnMut<'_, T> {
    #[inline(always)]
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.slice
    }
}

impl<T> Drop for TableColumnMut<'_, T> {
    fn drop(&mut self) {
        self.world.defer_end();
        #[cfg(feature = "flecs_safety_locks")]
        table_column_lock_write_end(self.locks, self.table.as_ptr(), self.column_index);
    }
}

pub trait TableOperations<'a>: IntoTable {
    fn table(&self) -> Table<'a>;
    fn offset(&self) -> i32;
    fn world(&self) -> WorldRef<'a>;

    /// Returns the table count
    fn count(&self) -> i32 {
        let table = self.table_ptr_mut();
        // SAFETY: table_ptr_mut points to the live table backing this range.
        unsafe { sys::ecs_table_count(table) }
    }

    /// Get number of allocated elements in table
    fn size(&self) -> i32 {
        let table = self.table_ptr_mut();
        // SAFETY: table_ptr_mut points to the live table backing this range.
        unsafe { sys::ecs_table_size(table) }
    }

    /// Get array with entity ids.
    ///
    /// For a [`TableRange`] this returns the entities within
    /// `[offset, offset + count)`.
    ///
    /// The returned guard keeps the world in deferred mode: structural changes
    /// are queued and applied when the guard is dropped, so the entity array
    /// stays valid while it is borrowed.
    fn entities(&self) -> TableEntities<'a> {
        let world = self.world();
        world.defer_begin();
        let table = self.table_ptr_mut();
        // SAFETY: table_ptr_mut points to the live table backing this range.
        let entities = unsafe { sys::ecs_table_entities(table) };
        if entities.is_null() {
            return TableEntities { slice: &[], world };
        }
        // SAFETY: table_ptr_mut points to the live table backing this range.
        let table_count = unsafe { sys::ecs_table_count(table) }.max(0) as usize;
        let offset = (self.offset().max(0) as usize).min(table_count);
        let count = (self.count().max(0) as usize).min(table_count - offset);
        // SAFETY: entities points to `table_count` valid entity ids and
        // offset + count <= table_count. The world stays in deferred mode
        // until the guard is dropped, so the array cannot be reallocated or
        // freed while the slice is alive.
        let slice =
            unsafe { core::slice::from_raw_parts((entities as *const Entity).add(offset), count) };
        TableEntities { slice, world }
    }

    fn clear_entities(&self) {
        let world = self.world().world_ptr_mut();
        let table = self.table_ptr_mut();
        // SAFETY: world and table come from this live table/world pair.
        unsafe { sys::ecs_table_clear_entities(world, table) };
    }

    /// Converts table type to string
    fn to_string(&self) -> Option<String> {
        unsafe {
            let raw_ptr = sys::ecs_table_str(self.world().world_ptr(), self.table_ptr_mut());

            if raw_ptr.is_null() {
                return None;
            }

            // Copy into a Rust-allocated String, then free the C allocation
            // with the flecs allocator; freeing it via Vec/String would use the
            // Rust global allocator, which is UB when the allocators differ.
            let string = CStr::from_ptr(raw_ptr).to_string_lossy().into_owned();
            sys::ecs_os_api.free_.expect("os api is missing")(raw_ptr as *mut core::ffi::c_void);
            Some(string)
        }
    }

    /// Returns the type of the table
    fn archetype(&self) -> Archetype<'a> {
        // SAFETY: table_ptr_mut points to the live table backing this range;
        // ecs_table_get_type returns a valid ecs_type_t pointer for it.
        let type_vec = unsafe { sys::ecs_table_get_type(self.table_ptr_mut()) };
        // SAFETY: type_vec was just returned by ecs_table_get_type above and
        // is valid to dereference.
        let slice = if unsafe { !(*type_vec).array.is_null() && (*type_vec).count != 0 } {
            // SAFETY: the branch condition just checked that `array` is
            // non-null and `count` is nonzero, describing `count` valid ids.
            unsafe {
                core::slice::from_raw_parts((*type_vec).array as _, (*type_vec).count as usize)
            }
        } else {
            &[]
        };
        let world = self.world();
        // Safety: we already know table_ptr is NonNull
        unsafe {
            Archetype::new_locked(
                world,
                slice,
                TableLock::new(world, NonNull::new_unchecked(self.table_ptr_mut())),
            )
        }
    }

    /// Find type index for (component) id
    ///
    /// # Arguments
    ///
    /// * `id` - The id of the component
    ///
    /// # Returns
    ///
    /// The index of the id in the table type, or `None` if the id is not found
    fn find_type_index(&self, id: impl IntoId) -> Option<i32> {
        // SAFETY: world_ptr/table_ptr_mut come from this live table/world
        // pair, and id is resolved against the same world.
        let index = unsafe {
            sys::ecs_table_get_type_index(
                self.world().world_ptr(),
                self.table_ptr_mut(),
                *id.into_id(self.world()),
            )
        };
        if index == -1 { None } else { Some(index) }
    }

    /// Find index for (component) id in table type
    ///
    /// This operation returns the index of first occurrence of the id in the table type. The id may be a wildcard.
    /// The found id may be different from the provided id if it is a wildcard.
    ///
    /// This is a constant time operation.
    ///
    /// # Arguments
    ///
    /// * `id` - The id of the component
    ///
    /// # Returns
    ///
    /// The index of the id in the table, or `None` if the id is not in the table
    fn find_column_index(&self, id: impl IntoId) -> Option<i32> {
        // SAFETY: world_ptr/table_ptr_mut come from this live table/world
        // pair, and id is resolved against the same world.
        let index = unsafe {
            sys::ecs_table_get_column_index(
                self.world().world_ptr(),
                self.table_ptr_mut(),
                *id.into_id(self.world()),
            )
        };
        if index == -1 { None } else { Some(index) }
    }

    /// Test if table has (component) id
    ///
    /// This is a constant time operation.
    ///
    /// # Arguments
    ///
    /// * `id` - The id of the component
    ///
    /// # Returns
    ///
    /// True if the table has the component id, false otherwise
    fn has(&self, id: impl IntoId) -> bool {
        self.find_type_index(id).is_some()
    }

    /// Get column, components array ptr from table by column index.
    ///
    /// # Arguments
    ///
    /// * `index` - The index of the component
    ///
    /// # Returns
    ///
    /// Some(Pointer) to the column, or `None` if not a component
    fn column_untyped(&self, index: i32) -> Option<*mut c_void> {
        // SAFETY: table_ptr_mut points to the live table backing this range.
        let ptr = unsafe { sys::ecs_table_get_column(self.table_ptr_mut(), index, self.offset()) };
        if ptr.is_null() { None } else { Some(ptr) }
    }

    /// Get mutable access to a components array from the table by component type.
    ///
    /// The returned guard dereferences to `&mut [T]`. While it is alive the
    /// world is kept in deferred mode (structural changes are queued) and,
    /// when the `flecs_safety_locks` feature (part of the default features)
    /// is enabled, a write lock is held on the column: obtaining a second
    /// mutable guard or field accessor for the same column panics instead of
    /// aliasing.
    ///
    /// For a [`TableRange`] the slice covers `[offset, offset + count)`.
    ///
    /// # Type parameters
    ///
    /// * `T` - The type of the component
    ///
    /// # Returns
    ///
    /// Some(guard) for the column, or `None` if not found
    fn get_mut<T: ComponentId>(&mut self) -> Option<TableColumnMut<'a, T>> {
        let world = self.world();
        world.check_thread_affinity_exclusive::<T>();
        let id = T::entity_id(world);
        let index = self.find_column_index(id)?;
        let ptr = self.column_untyped(index)?;
        let table = self.table_ptr_mut();
        // SAFETY: table_ptr_mut points to the live table backing this range.
        let table_count = unsafe { sys::ecs_table_count(table) }.max(0) as usize;
        let offset = (self.offset().max(0) as usize).min(table_count);
        let count = (self.count().max(0) as usize).min(table_count - offset);
        // SAFETY: table_ptr_mut is non-null, index is a valid column index for
        // this table and ptr/count describe that column's array of T starting
        // at this range's offset (applied by column_untyped).
        Some(unsafe {
            TableColumnMut::new(
                world,
                NonNull::new_unchecked(table),
                index as i16,
                ptr as *mut T,
                count,
            )
        })
    }

    /// Get column, components array ptr from table by component type.
    ///
    /// # Arguments
    ///
    /// * `id` - The id of the component
    ///
    /// # Returns
    ///
    /// Some(Pointer) to the column, or `None` if not found
    fn get_mut_untyped(&self, id: sys::ecs_id_t) -> Option<*mut c_void> {
        if let Some(index) = self.find_column_index(id) {
            self.column_untyped(index)
        } else {
            None
        }
    }

    /// Get column, components array ptr from table by pair ids.
    ///
    /// # Arguments
    ///
    /// * `first` - The id of the first component
    /// * `second` - The id of the second component
    ///
    /// # Returns
    ///
    /// Some(Pointer) to the column, or `None` if not found
    fn get_pair_ids_mut_untyped(
        &self,
        first: impl Into<Entity>,
        second: impl Into<Entity>,
    ) -> Option<*mut c_void> {
        self.get_mut_untyped(ecs_pair(*first.into(), *second.into()))
    }

    /// Get column, components array ptr from table by pair of component types.
    ///
    /// # Type parameters
    ///
    /// * `First` - The type of the first component
    /// * `Second` - The type of the second component
    ///
    /// # Returns
    ///
    /// Some(Pointer) to the column, or `None` if not found
    fn get_pair_mut_untyped<First: ComponentId, Second: ComponentId>(&self) -> Option<*mut c_void> {
        let world = self.world();
        self.get_pair_ids_mut_untyped(First::entity_id(world), Second::entity_id(world))
    }

    /// Get column size from table at the provided column index.
    ///
    /// # Arguments
    ///
    /// * `index` - The index of the column
    ///
    /// # Returns
    ///
    /// The size of the column
    fn column_size(&self, index: i32) -> usize {
        // SAFETY: table_ptr_mut points to the live table backing this range.
        unsafe { sys::ecs_table_get_column_size(self.table_ptr_mut(), index) }
    }

    /// Return depth for table in tree for relationship type.
    /// Depth is determined by counting the number of targets encountered while traversing up the
    /// relationship tree for rel. Only acyclic relationships are supported.
    ///
    /// # Arguments
    ///
    /// * `rel` - The id of the relationship
    ///
    /// # Returns
    ///
    /// The depth of the relationship
    fn depth(&self, rel: impl IntoEntity) -> i32 {
        let world = self.world();
        // SAFETY: world_ptr_mut/table_ptr_mut come from this live table/world
        // pair, and rel is a valid entity id resolved against the same world.
        unsafe {
            sys::ecs_table_get_depth(
                world.world_ptr_mut(),
                self.table_ptr_mut(),
                *rel.into_entity(world),
            )
        }
    }

    /// get table records array
    fn records(&self) -> &[sys::ecs_table_record_t] {
        // SAFETY: table_ptr_mut points to the live table backing this range.
        let records = unsafe { sys::flecs_table_records(self.table_ptr_mut()) };

        // SAFETY: flecs_table_records returns a valid ecs_vec_t whose
        // `array`/`count` describe `count` initialized records.
        unsafe { core::slice::from_raw_parts(records.array, records.count as usize) }
    }

    /// get table id
    fn id(&self) -> u64 {
        // SAFETY: table_ptr_mut points to the live table backing this range.
        unsafe { sys::flecs_table_id(self.table_ptr_mut()) }
    }

    /// lock table
    fn lock(&self) {
        // SAFETY: world and table_ptr_mut come from this live table/world pair.
        unsafe { sys::ecs_table_lock(self.world().world_ptr_mut(), self.table_ptr_mut()) };
    }

    /// unlock table
    fn unlock(&self) {
        // SAFETY: world and table_ptr_mut come from this live table/world pair.
        unsafe { sys::ecs_table_unlock(self.world().world_ptr_mut(), self.table_ptr_mut()) };
    }

    fn has_flags(&self, flags: TableFlags) -> bool {
        // SAFETY: table_ptr_mut points to the live table backing this range.
        unsafe { sys::ecs_table_has_flags(self.table_ptr_mut(), flags.bits()) }
    }
}

impl<'a> TableOperations<'a> for Table<'a> {
    fn table(&self) -> Table<'a> {
        *self
    }

    fn offset(&self) -> i32 {
        0
    }

    fn world(&self) -> WorldRef<'a> {
        self.world
    }

    /// Returns the table count
    fn count(&self) -> i32 {
        // SAFETY: table_ptr_mut derives from Table's NonNull `table` field,
        // which is guaranteed alive and owned by the world for `'a`.
        unsafe { sys::ecs_table_count(self.table_ptr_mut()) }
    }
}

impl<'a> TableOperations<'a> for TableRange<'a> {
    fn table(&self) -> Table<'a> {
        self.table
    }

    fn offset(&self) -> i32 {
        self.offset
    }

    fn world(&self) -> WorldRef<'a> {
        self.table.world
    }

    /// Returns the table range count
    fn count(&self) -> i32 {
        self.count
    }
}

/// A lock on a [`Table`].
///
/// When a table is locked, modifications to it will throw an assert. When the
/// table is locked recursively, it will take an equal amount of unlock
/// operations to actually unlock the table.
///
/// Table locks can be used to build safe iterators where it is guaranteed that
/// the contents of a table are not modified while it is being iterated.
///
/// The operation only works when called on the world, and has no side effects
/// when called on a stage. The assumption is that when called on a stage,
/// operations are deferred already.
pub(crate) struct TableLock<'a> {
    world: WorldRef<'a>,
    table: NonNull<sys::ecs_table_t>,
}

impl<'a> TableLock<'a> {
    pub fn new(world: impl WorldProvider<'a>, table: NonNull<sys::ecs_table_t>) -> Self {
        // SAFETY: caller supplies a valid world and a NonNull table pointer
        // owned by that world.
        unsafe { sys::ecs_table_lock(world.world_ptr_mut(), table.as_ptr()) };
        Self {
            world: world.world(),
            table,
        }
    }
}

impl Drop for TableLock<'_> {
    fn drop(&mut self) {
        // SAFETY: table was locked by `new` and this guard's world/table
        // remain valid for the guard's lifetime.
        unsafe {
            sys::ecs_table_unlock(self.world.world_ptr_mut(), self.table.as_ptr());
        }
    }
}
