use std::{
    ffi::{c_void, CStr},
    ops::{Deref, DerefMut},
    ptr::{self, NonNull},
};

use crate::sys;
use flecs_ecs::core::*;
use sys::ecs_get_with;
#[cfg(not(feature = "flecs_unsafe_get"))]
use sys::ecs_record_t;

/// `EntityView` is a wrapper around an entity id with the world. It provides methods to interact with entities.
#[derive(Clone, Copy)]
pub struct EntityView<'a> {
    pub(crate) world: WorldRef<'a>,
    pub(crate) id: Entity,
}

impl<'a> Deref for EntityView<'a> {
    type Target = Entity;

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.id
    }
}

impl<'a> DerefMut for EntityView<'a> {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.id
    }
}

impl<'a> std::fmt::Display for EntityView<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some(name) = self.get_name() {
            write!(f, "{}", name)
        } else {
            write!(f, "{}", *self.id)
        }
    }
}

impl<'a> std::fmt::Debug for EntityView<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let name = self.name();
        let id = self.id;
        let archetype_str = self
            .archetype()
            .to_string()
            .unwrap_or_else(|| "empty".to_string());
        write!(
            f,
            "Entity name: {} -- id: {} -- archetype: {}",
            name, id, archetype_str
        )
    }
}

impl<'a> EntityView<'a> {
    /// Create new entity.
    ///
    /// # See also
    ///
    /// * C++ API: `entity::entity`
    #[doc(alias = "entity::entity")]
    #[allow(clippy::not_unsafe_ptr_arg_deref)]
    pub(crate) fn new(world: impl IntoWorld<'a>) -> Self {
        let world_ptr = world.world_ptr_mut();
        let id = if unsafe { sys::ecs_get_scope(world_ptr) == 0 && ecs_get_with(world_ptr) == 0 } {
            unsafe { sys::ecs_new(world_ptr) }
        } else {
            let desc = sys::ecs_entity_desc_t::default();
            unsafe { sys::ecs_entity_init(world_ptr, &desc) }
        };
        Self {
            world: world.world(),
            id: id.into(),
        }
    }

    /// Creates a wrapper around an existing entity / id.
    ///
    /// # See also
    ///
    /// * C++ API: `entity::entity`
    #[doc(alias = "entity::entity")]
    pub(crate) fn new_from(world: impl IntoWorld<'a>, id: impl Into<Entity>) -> Self {
        Self {
            world: world.world(),
            id: id.into(),
        }
    }

    /// Create a named entity.
    ///
    /// Named entities can be looked up with the lookup functions. Entity names
    /// may be scoped, where each element in the name is separated by "::".
    /// For example: "`Foo::Bar`". If parts of the hierarchy in the scoped name do
    /// not yet exist, they will be automatically created.
    ///
    /// # See also
    ///
    /// * C++ API: `entity::entity`
    #[doc(alias = "entity::entity")]
    pub(crate) fn new_named(world: impl IntoWorld<'a>, name: &str) -> Self {
        let name = compact_str::format_compact!("{}\0", name);

        let desc = sys::ecs_entity_desc_t {
            name: name.as_ptr() as *const _,
            sep: SEPARATOR.as_ptr(),
            root_sep: SEPARATOR.as_ptr(),
            _canary: 0,
            id: 0,
            parent: 0,
            symbol: std::ptr::null(),
            use_low_id: false,
            add: std::ptr::null(),
            add_expr: std::ptr::null(),
            set: std::ptr::null(),
        };
        let id = unsafe { sys::ecs_entity_init(world.world_ptr_mut(), &desc) };
        Self {
            world: world.world(),
            id: id.into(),
        }
    }

    pub(crate) fn new_named_cstr(world: impl IntoWorld<'a>, name: &CStr) -> Self {
        let desc = sys::ecs_entity_desc_t {
            name: name.as_ptr(),
            sep: SEPARATOR.as_ptr(),
            root_sep: SEPARATOR.as_ptr(),
            _canary: 0,
            id: 0,
            parent: 0,
            symbol: std::ptr::null(),
            use_low_id: false,
            add: std::ptr::null(),
            add_expr: std::ptr::null(),
            set: std::ptr::null(),
        };
        let id = unsafe { sys::ecs_entity_init(world.world_ptr_mut(), &desc) };
        Self {
            world: world.world(),
            id: id.into(),
        }
    }

    /// Entity id 0.
    /// This function is useful when the API must provide an entity that
    /// belongs to a world, but the entity id is 0.
    ///
    /// # See also
    ///
    /// * C++ API: `entity::null`
    #[doc(alias = "entity::null")]
    pub(crate) fn new_null(world: &'a World) -> EntityView<'a> {
        Self::new_from(world, 0)
    }

    /// Get the [`IdView`] representation of the `entity_view`.
    pub fn id_view(&self) -> IdView {
        IdView::new_from(self.world, *self.id)
    }

    /// checks if entity is valid
    ///
    /// # Example
    ///
    /// ```
    /// use flecs_ecs::prelude::*;
    ///
    /// let world = World::new();
    ///
    /// let entity = world.entity();
    /// let entity_rust_copy = entity; //this is a rust copy, not an entity clone, these have the same id.
    /// let entity_cloned = entity.duplicate(false); //this is an entity clone, these have different ids.
    ///
    /// assert!(entity.is_valid());
    /// assert!(entity_rust_copy.is_valid());
    /// assert!(entity_cloned.is_valid());
    ///
    /// entity.destruct(); //takes self, entity becomes not useable
    ///
    /// assert!(!entity_rust_copy.is_valid());
    /// assert!(entity_cloned.is_valid());
    /// ```
    ///
    /// # See also
    ///
    /// * C++ API: `entity_view::is_valid`
    #[doc(alias = "entity_view::is_valid")]
    pub fn is_valid(self) -> bool {
        unsafe { sys::ecs_is_valid(self.world.world_ptr_mut(), *self.id) }
    }

    /// Checks if entity is alive.
    ///
    /// # See also
    ///
    /// * C++ API: `entity_view::is_alive`
    #[doc(alias = "entity_view::is_alive")]
    pub fn is_alive(self) -> bool {
        unsafe { sys::ecs_is_alive(self.world.world_ptr_mut(), *self.id) }
    }

    /// Returns the entity name.
    ///
    /// if the entity has no name, this will return an empty string
    ///
    /// # See also
    ///
    /// * C++ API: `entity_view::name`
    #[doc(alias = "entity_view::name")]
    pub fn name(self) -> &'a str {
        self.get_name().unwrap_or("")
    }

    /// Returns the entity name.
    ///
    /// if the entity has no name, this will return none
    ///
    /// # See also
    ///
    /// * C++ API: `entity_view::name`
    #[doc(alias = "entity_view::name")]
    pub fn get_name(self) -> Option<&'a str> {
        self.get_name_cstr().and_then(|s| s.to_str().ok())
    }

    /// Returns the entity name as a `CStr`.
    ///
    /// if the entity has no name, this will return an empty string
    ///
    /// # See also
    ///
    /// * C++ API: `entity_view::name`
    pub fn name_cstr(self) -> &'a CStr {
        self.get_name_cstr().unwrap_or(c"")
    }

    /// Returns the entity name as a `CStr`.
    ///
    /// If the entity has no name, this will return `None`.
    ///
    /// # See also
    ///
    /// * C++ API: `entity_view::name`
    pub fn get_name_cstr(self) -> Option<&'a CStr> {
        NonNull::new(unsafe { sys::ecs_get_name(self.world.world_ptr_mut(), *self.id) } as *mut _)
            .map(|s| unsafe { CStr::from_ptr(s.as_ptr()) })
    }

    /// Returns the entity symbol.
    ///
    /// # See also
    ///
    /// * C++ API: `entity_view::symbol`
    #[doc(alias = "entity_view::symbol")]
    pub fn symbol_cstr(self) -> &'a CStr {
        unsafe { CStr::from_ptr(sys::ecs_get_symbol(self.world.world_ptr_mut(), *self.id)) }
    }

    /// Returns the entity symbol.
    ///
    /// # See also
    ///
    /// * C++ API: `entity_view::symbol`
    #[doc(alias = "entity_view::symbol")]
    pub fn symbol(self) -> &'a str {
        self.symbol_cstr().to_str().unwrap()
    }

    /// Return the hierarchical entity path.
    /// # Note
    /// if you're using the default separator "::" you can use `get_hierarchy_path_default`
    /// which does no extra heap allocations to communicate with C
    ///
    /// # See also
    ///
    /// * C++ API: `entity_view::path`
    #[doc(alias = "entity_view::path")]
    pub fn path_w_sep(self, sep: &str, init_sep: &str) -> Option<String> {
        self.path_from_id_w_sep(0, sep, init_sep)
    }

    /// Return the hierarchical entity path using the default separator "::".
    ///
    /// # See also
    ///
    /// * C++ API: `entity_view::path`
    #[doc(alias = "entity_view::path")]
    pub fn path(self) -> Option<String> {
        self.path_from_id(0)
    }

    /// Return the hierarchical entity path relative to a parent.
    ///
    /// if you're using the default separator "::" you can use `get_hierarchy_path_default`
    /// which does no extra heap allocations to communicate with C
    ///
    /// # See also
    ///
    /// * C++ API: `entity_view::path_from`
    #[doc(alias = "entity_view::path_from")]
    pub fn path_from_id_w_sep(
        &self,
        parent: impl Into<Entity>,
        sep: &str,
        init_sep: &str,
    ) -> Option<String> {
        let sep = compact_str::format_compact!("{}\0", sep);
        let init_sep = compact_str::format_compact!("{}\0", init_sep);

        NonNull::new(unsafe {
            sys::ecs_get_path_w_sep(
                self.world.world_ptr_mut(),
                *parent.into(),
                *self.id,
                sep.as_ptr() as *const _,
                init_sep.as_ptr() as *const _,
            )
        })
        .map(|s| unsafe {
            let len = CStr::from_ptr(s.as_ptr()).to_bytes().len();
            // Convert the C string to a Rust String without any new heap allocation.
            // The String will de-allocate the C string when it goes out of scope.
            String::from_utf8_unchecked(Vec::from_raw_parts(s.as_ptr() as *mut u8, len, len))
        })
    }

    fn path_from_id_default_sep(&self, parent: impl Into<Entity>) -> Option<String> {
        NonNull::new(unsafe {
            sys::ecs_get_path_w_sep(
                self.world.world_ptr_mut(),
                *parent.into(),
                *self.id,
                SEPARATOR.as_ptr(),
                SEPARATOR.as_ptr(),
            )
        })
        .map(|s| unsafe {
            let len = CStr::from_ptr(s.as_ptr()).to_bytes().len();
            // Convert the C string to a Rust String without any new heap allocation.
            // The String will de-allocate the C string when it goes out of scope.
            String::from_utf8_unchecked(Vec::from_raw_parts(s.as_ptr() as *mut u8, len, len))
        })
    }

    /// Return the hierarchical entity path relative to a parent id using the default separator "::".
    ///
    /// # See also
    ///
    /// * C++ API: `entity_view::path_from`
    #[doc(alias = "entity_view::path_from")]
    pub fn path_from_id(self, parent: impl Into<Entity>) -> Option<String> {
        NonNull::new(unsafe {
            sys::ecs_get_path_w_sep(
                self.world.world_ptr_mut(),
                *parent.into(),
                *self.id,
                SEPARATOR.as_ptr(),
                SEPARATOR.as_ptr(),
            )
        })
        .map(|s| unsafe {
            let len = CStr::from_ptr(s.as_ptr()).to_bytes().len();

            // Convert the C string to a Rust String without any new heap allocation.
            // The String will de-allocate the C string when it goes out of scope.
            String::from_utf8_unchecked(Vec::from_raw_parts(s.as_ptr() as *mut u8, len, len))
        })
    }

    /// Return the hierarchical entity path relative to a parent type.
    /// # Note
    /// if you're using the default separator "::" you can use `get_hierarchy_path_default`
    /// which does no extra heap allocations to communicate with C
    ///
    /// # See also
    ///
    /// * C++ API: `entity_view::path_from`
    #[doc(alias = "entity_view::path_from")]
    pub fn path_from<T: ComponentId>(self) -> Option<String> {
        self.path_from_id_default_sep(T::id(self.world))
    }

    pub fn path_from_w_sep<T: ComponentId>(&self, sep: &str, init_sep: &str) -> Option<String> {
        self.path_from_id_w_sep(T::id(self.world), sep, init_sep)
    }

    /// Return the hierarchical entity path relative to a parent type using the default separator "::".
    ///
    /// # See also
    ///
    /// * C++ API: `entity_view::path_from`
    #[doc(alias = "entity_view::path_from")]
    pub fn hierarchy_path_from_parent_type<T: ComponentId>(self) -> Option<String> {
        self.path_from_id(T::id(self.world))
    }

    /// Checks if the entity is enabled.
    ///
    /// # Returns
    ///
    /// True if the entity is enabled, false if disabled.
    ///
    /// # See also
    ///
    /// * C++ API: `entity_view::enabled`
    #[doc(alias = "entity_view::enabled")]
    pub fn is_enabled_self(self) -> bool {
        unsafe { !sys::ecs_has_id(self.world.world_ptr_mut(), *self.id, flecs::Disabled::ID) }
    }

    /// get the entity's archetype
    ///
    /// # See also
    ///
    /// * C++ API: `entity_view::type`
    #[doc(alias = "entity_view::type")]
    #[inline(always)]
    pub fn archetype(self) -> Archetype<'a> {
        self.table()
            .map(|t| t.archetype())
            .unwrap_or(Archetype::new(self.world, &[]))
    }

    /// get the entity's type/table
    ///
    /// # See also
    ///
    /// * C++ API: `entity_view::table`
    #[doc(alias = "entity_view::table")]
    #[inline(always)]
    pub fn table(self) -> Option<Table<'a>> {
        NonNull::new(unsafe { sys::ecs_get_table(self.world.world_ptr_mut(), *self.id) })
            .map(|t| Table::new(self.world, t))
    }

    /// Get table range for the entity.
    /// # Returns
    /// Returns a range with the entity's row as offset and count set to 1. If
    /// the entity is not stored in a table, the function returns a range with
    /// count 0.
    ///
    /// # See also
    ///
    /// * C++ API: `entity_view::range`
    #[doc(alias = "entity_view::range")]
    #[inline]
    pub fn table_range(self) -> Option<TableRange<'a>> {
        NonNull::new(unsafe { sys::ecs_record_find(self.world.world_ptr_mut(), *self.id) }).map(
            |record| unsafe {
                TableRange::new_raw(
                    self.world,
                    NonNull::new_unchecked((*record.as_ptr()).table),
                    ecs_record_to_row((*record.as_ptr()).row),
                    1,
                )
            },
        )
    }

    /// Iterate over component ids of an entity.
    ///
    /// # Arguments
    /// * `func` - The closure invoked for each matching ID. Must match the signature `FnMut(Id)`.
    ///
    /// # See also
    ///
    /// * C++ API: `entity_view::each`
    #[doc(alias = "entity_view::each")]
    pub fn each_component(self, mut func: impl FnMut(IdView)) {
        let archetype = self.archetype();

        for &id in archetype.as_slice() {
            let ent = IdView::new_from(self.world, id);
            func(ent);
        }
    }

    /// Iterates over matching pair IDs of an entity.
    ///
    /// # Arguments
    ///
    /// * `first` - The first ID to match against.
    /// * `second` - The second ID to match against.
    /// * `func` - The closure invoked for each matching ID. Must match the signature `FnMut(Id)`.
    ///
    /// # See also
    ///
    /// * C++ API: `entity_view::each`
    #[doc(alias = "entity_view::each")]
    pub fn each_pair(
        &self,
        pred: impl Into<Entity>,
        obj: impl Into<Entity>,
        mut func: impl FnMut(IdView),
    ) {
        // this is safe because we are only reading the world
        let real_world = self.world.real_world();

        let Some(table) =
            NonNull::new(unsafe { sys::ecs_get_table(real_world.world_ptr_mut(), *self.id) })
        else {
            return;
        };

        let table = Table::new(real_world, table);

        let mut pattern: IdT = *pred.into();
        let obj_id = *obj.into();
        if obj_id != 0 {
            pattern = ecs_pair(pattern, obj_id);
        }

        let mut cur: i32 = 0;
        let archetype = table.archetype();
        let ids = archetype.as_slice();

        while {
            cur = unsafe {
                sys::ecs_search_offset(
                    real_world.world_ptr(),
                    table.table_ptr_mut(),
                    cur,
                    pattern,
                    &mut 0,
                )
            };
            cur != -1
        } {
            let ent = IdView::new_from(self.world, ids[cur as usize]);
            func(ent);
            cur += 1;
        }
    }

    /// Iterate over targets for a given relationship.
    ///
    /// # Arguments
    ///
    /// * `relationship` - The relationship for which to iterate the targets.
    /// * `func` - The closure invoked for each target. Must match the signature `FnMut(EntityView)`.
    ///
    /// # See also
    ///
    /// * C++ API: `entity_view::each`
    #[doc(alias = "entity_view::each")]
    pub fn each_target_id(self, relationship: impl Into<Entity>, mut func: impl FnMut(EntityView)) {
        self.each_pair(relationship.into(), ECS_WILDCARD, |id| {
            let obj = id.second_id();
            func(obj);
        });
    }

    /// Get the count of targets for a given relationship.
    ///
    /// # Arguments
    ///
    /// * `relationship` - The relationship for which to get the target count.
    ///
    /// # Returns
    ///
    /// The count of targets for the given relationship.
    /// If it doesn't have the relationship, this function will return `None`.
    pub fn target_id_count(self, relationship: impl Into<Entity>) -> Option<i32> {
        let world = self.world.real_world().ptr_mut();
        let id = ecs_pair(*relationship.into(), ECS_WILDCARD);
        let table = unsafe { sys::ecs_get_table(self.world.world_ptr_mut(), *self.id) };

        let count = unsafe { sys::ecs_rust_rel_count(world, id, table) };

        if count == -1 {
            None
        } else {
            Some(count)
        }
    }

    /// Iterate over targets for a given relationship.
    ///
    /// # Type Parameters
    ///
    /// * `Relationship` - The relationship for which to iterate the targets.
    ///
    /// # Arguments
    ///
    /// * `func` - The function invoked for each target.
    ///
    /// # See also
    ///
    /// * C++ API: `entity_view::each`
    #[doc(alias = "entity_view::each")]
    pub fn each_target<T>(self, func: impl FnMut(EntityView))
    where
        T: ComponentId,
    {
        self.each_target_id(EntityView::new_from(self.world, T::id(self.world)), func);
    }

    /// Get the count of targets for a given relationship.
    ///
    /// # Type Parameters
    ///
    /// * `T` - The relationship for which to get the target count.
    ///
    /// # Returns
    ///
    /// The count of targets for the given relationship.
    /// If it doesn't have the relationship, this function will return `None`.
    pub fn each_target_count<T>(self) -> Option<i32>
    where
        T: ComponentId,
    {
        self.target_id_count(T::id(self.world))
    }

    /// Iterate children for entity
    ///
    /// # Arguments
    ///
    /// * `relationship` - The relationship to follow
    /// * `func` - The function invoked for each child. Must match the signature `FnMut(EntityView)`.
    ///
    /// # See also
    ///
    /// * C++ API: `entity_view::children`
    #[doc(alias = "entity_view::children")]
    pub fn each_child_of_id(
        self,
        relationship: impl Into<Entity>,
        mut func: impl FnMut(EntityView),
    ) {
        // When the entity is a wildcard, this would attempt to query for all
        //entities with (ChildOf, *) or (ChildOf, _) instead of querying for
        //the children of the wildcard entity.
        if self.id == flecs::Wildcard::ID || self.id == flecs::Any::ID {
            // this is correct, wildcard entities don't have children
            return;
        }

        let mut it: sys::ecs_iter_t =
            unsafe { sys::ecs_each_id(self.world_ptr(), ecs_pair(*relationship.into(), *self.id)) };
        while unsafe { sys::ecs_each_next(&mut it) } {
            for i in 0..it.count as usize {
                unsafe {
                    let id = it.entities.add(i);
                    let ent = EntityView::new_from(self.world, *id);
                    func(ent);
                }
            }
        }
    }

    /// Iterate children for entity
    ///
    /// # Arguments
    ///
    /// * T - The relationship to follow
    /// * `func` - The function invoked for each child. Must match the signature `FnMut(EntityView)`.
    ///
    /// # See also
    ///
    /// * C++ API: `entity_view::children`
    #[doc(alias = "entity_view::children")]
    pub fn each_child_of<T>(self, func: impl FnMut(EntityView))
    where
        T: ComponentId,
    {
        self.each_child_of_id(T::id(self.world), func);
    }

    /// Iterate children for entity
    /// This operation follows the `ChildOf` relationship.
    /// # Arguments
    ///
    /// * `func` - The function invoked for each child. Must match the signature `FnMut(EntityView)`.
    ///
    /// # See also
    ///
    /// * C++ API: `entity_view::children`
    #[doc(alias = "entity_view::children")]
    pub fn each_child(self, func: impl FnMut(EntityView)) {
        self.each_child_of_id(flecs::ChildOf::ID, func);
    }

    /// gets mutable or immutable component(s) and/or relationship(s) from an entity in a callback.
    /// each component type must be marked `&` or `&mut` to indicate if it is mutable or not.
    /// use `Option` wrapper to indicate if the component is optional.
    /// use `()` tuple format when getting multiple components.
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
    /// - If the callback has ran.
    ///
    /// # Example
    ///
    /// ```
    /// use flecs_ecs::prelude::*;
    ///
    /// #[derive(Component)] struct Tag;
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
    /// let entity = world.entity()
    ///                   .set(Position { x: 10.0, y: 20.0 })
    ///                   .set_pair::<Tag, Position>(Position { x: 30.0, y: 40.0 });
    ///    
    /// let has_run = entity.try_get::<&Position>(|(pos)| {
    ///     assert_eq!(pos.x, 10.0);
    /// });
    /// assert!(has_run);
    ///
    /// let has_run = entity.try_get::<(Option<&Velocity>, &Position)>( |(tag, pos)| {
    ///     assert_eq!(pos.x, 10.0);
    ///     assert!(tag.is_none());
    /// });
    /// assert!(has_run);
    ///
    /// let has_run = entity.try_get::<(&mut(Tag,Position), &Position)>(|(tag_pos_rel, pos)| {
    ///     assert_eq!(pos.x, 10.0);
    ///     assert_eq!(tag_pos_rel.x, 30.0);
    /// });
    /// assert!(has_run);
    ///
    /// ```
    pub fn try_get<T: GetTuple>(self, callback: impl for<'e> FnOnce(T::TupleType<'e>)) -> bool {
        let world_ptr = self.world.world_ptr_mut();

        #[cfg(not(feature = "flecs_unsafe_get"))]
        let record = unsafe { sys::ecs_read_begin(world_ptr, *self.id) as *mut ecs_record_t };

        #[cfg(feature = "flecs_unsafe_get")]
        let record = unsafe { sys::ecs_record_find(world_ptr, *self.id) };

        #[cfg(not(feature = "flecs_unsafe_get"))]
        if record.is_null() {
            return false;
        }

        #[cfg(feature = "flecs_unsafe_get")]
        if unsafe { (*record).table.is_null() } {
            return false;
        }

        let tuple_data = T::create_ptrs::<false>(self.world, self.id, record);
        let has_all_components = tuple_data.has_all_components();

        if has_all_components {
            let tuple = tuple_data.get_tuple();
            callback(tuple);
        }

        #[cfg(not(feature = "flecs_unsafe_get"))]
        unsafe {
            sys::ecs_read_end(record);
        }

        has_all_components
    }

    /// gets mutable or immutable component(s) and/or relationship(s) from an entity in a callback.
    /// each component type must be marked `&` or `&mut` to indicate if it is mutable or not.
    /// use `Option` wrapper to indicate if the component is optional.
    /// use `()` tuple format when getting multiple components.
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
    /// #[derive(Component)] struct Tag;
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
    /// let entity = world.entity()
    ///                   .set(Position { x: 10.0, y: 20.0 })
    ///                   .set_pair::<Tag, Position>(Position { x: 30.0, y: 40.0 });
    ///    
    /// entity.get::<&Position>(|(pos)| {
    ///     assert_eq!(pos.x, 10.0);
    /// });
    /// entity.get::<(Option<&Velocity>, &Position)>( |(vel, pos)| {
    ///     assert_eq!(pos.x, 10.0);
    ///     assert!(vel.is_none());
    /// });
    /// entity.get::<(&mut(Tag,Position), &Position)>(|(tag_pos_rel, pos)| {
    ///     assert_eq!(pos.x, 10.0);
    ///    assert_eq!(tag_pos_rel.x, 30.0);
    /// });
    /// ```
    pub fn get<T: GetTuple>(self, callback: impl for<'e> FnOnce(T::TupleType<'e>)) {
        let world_ptr = self.world.world_ptr_mut();

        #[cfg(not(feature = "flecs_unsafe_get"))]
        let record = unsafe { sys::ecs_read_begin(world_ptr, *self.id) as *mut ecs_record_t };

        #[cfg(feature = "flecs_unsafe_get")]
        let record = unsafe { sys::ecs_record_find(world_ptr, *self.id) };

        #[cfg(not(feature = "flecs_unsafe_get"))]
        if record.is_null() {
            return;
        }

        #[cfg(feature = "flecs_unsafe_get")]
        if unsafe { (*record).table.is_null() } {
            return;
        }

        let tuple_data = T::create_ptrs::<true>(self.world, self.id, record);
        let tuple = tuple_data.get_tuple();

        callback(tuple);

        #[cfg(not(feature = "flecs_unsafe_get"))]
        unsafe {
            sys::ecs_read_end(record);
        }
    }

    /// Clones components and/or relationship(s) from an entity and returns it.
    /// each component type must be marked `&`. This helps Rust type checker to determine if it's a relationship.
    /// use `Option` wrapper to indicate if the component is optional.
    /// use `()` tuple format when getting multiple components.
    ///
    /// # Note
    ///
    /// - You cannot clone single component tags with this function.
    /// - You can only get relationships with a payload, so where one is not a tag / not a zst.
    ///
    /// # Panics
    ///
    /// - `cloned` assumes when not using `Option` wrapper, that the entity has the component.
    ///   This will panic if the entity does not have the component. If unsure, use `Option` wrapper or `try_cloned` function instead.
    ///   `try_cloned` will return a `None` tuple instead if the entity does not have the component that isn't marked `Option`.
    ///
    /// # Example
    ///
    /// ```
    /// use flecs_ecs::prelude::*;
    ///
    /// #[derive(Component)] struct Tag;
    ///
    /// #[derive(Component, Clone)]
    /// pub struct Velocity {
    ///     pub x: f32,
    ///     pub y: f32,
    /// }
    ///
    /// #[derive(Component, Clone)]
    /// pub struct Position {
    ///     pub x: f32,
    ///     pub y: f32,
    /// }
    ///
    /// let world = World::new();
    ///
    /// let entity = world.entity()
    ///                   .set(Position { x: 10.0, y: 20.0 })
    ///                   .set_pair::<Tag, Position>(Position { x: 30.0, y: 40.0 });
    ///    
    /// let pos = entity.cloned::<&Position>();
    /// assert_eq!(pos.x, 10.0);
    ///
    /// let (vel, pos) = entity.cloned::<(Option<&Velocity>, &Position)>();
    /// assert_eq!(pos.x, 10.0);
    /// assert!(vel.is_none());
    ///
    /// let (tag_pos_rel, pos) = entity.cloned::<(&(Tag,Position), &Position)>();
    /// assert_eq!(pos.x, 10.0);
    /// assert_eq!(tag_pos_rel.x, 30.0);
    /// ```
    pub fn cloned<T: ClonedTuple>(self) -> T::TupleType<'a> {
        let world_ptr = self.world.world_ptr_mut();

        let record = unsafe { sys::ecs_record_find(world_ptr, *self.id) };

        if unsafe { (*record).table.is_null() } {
            panic!("Entity does not have any components");
        }

        let tuple_data = T::create_ptrs::<true>(self.world, record);
        tuple_data.get_tuple()
    }

    /// Clones components and/or relationship(s) from an entity and returns an `Option`.
    /// `None` if the entity does not have all components that are not marked `Option`, otherwise `Some(tuple)`.
    /// each component type must be marked `&`. This helps Rust type checker to determine if it's a relationship.
    /// use `Option` wrapper to indicate if the component is optional.
    /// use `()` tuple format when getting multiple components.
    ///
    /// # Note
    ///
    /// - You cannot clone single component tags with this function.
    /// - You can only clone relationships with a payload, so where one is not a tag / not a zst.
    ///
    /// # Returns
    ///
    /// - `Some(tuple)` if the entity has all components, `None` otherwise.
    ///
    /// # Example
    ///
    /// ```
    /// use flecs_ecs::prelude::*;
    ///
    /// #[derive(Component)] struct Tag;
    ///
    /// #[derive(Component, Clone)]
    /// pub struct Velocity {
    ///     pub x: f32,
    ///     pub y: f32,
    /// }
    ///
    /// #[derive(Component, Clone)]
    /// pub struct Position {
    ///     pub x: f32,
    ///     pub y: f32,
    /// }
    ///
    /// let world = World::new();
    ///
    /// let entity = world.entity()
    ///                   .set(Position { x: 10.0, y: 20.0 })
    ///                   .set_pair::<Tag, Position>(Position { x: 30.0, y: 40.0 });
    ///    
    /// let pos = entity.try_cloned::<&Position>();
    /// assert!(pos.is_some());
    /// assert_eq!(pos.unwrap().x, 10.0);
    ///
    /// let (vel, pos) = entity.try_cloned::<(Option<&Velocity>, &Position)>().unwrap();
    /// assert_eq!(pos.x, 10.0);
    /// assert!(vel.is_none());
    ///
    /// let (tag_pos_rel, pos) = entity.try_cloned::<(&(Tag,Position), &Position)>().unwrap();
    /// assert_eq!(pos.x, 10.0);
    /// assert_eq!(tag_pos_rel.x, 30.0);
    /// ```
    pub fn try_cloned<T: ClonedTuple>(self) -> Option<T::TupleType<'a>> {
        let world_ptr = self.world.world_ptr_mut();

        let record = unsafe { sys::ecs_record_find(world_ptr, *self.id) };

        if unsafe { (*record).table.is_null() } {
            return None;
        }

        let tuple_data = T::create_ptrs::<false>(self.world, record);
        //todo we can maybe early return if we don't yet if doesn't have all. Same for try_get
        let has_all_components = tuple_data.has_all_components();

        if has_all_components {
            Some(tuple_data.get_tuple())
        } else {
            None
        }
    }

    /// gets mutable or immutable component(s) and/or relationship(s) from an entity in a callback and return a value.
    /// each component type must be marked `&` or `&mut` to indicate if it is mutable or not.
    /// use `Option` wrapper to indicate if the component is optional.
    /// use `()` tuple format when getting multiple components.
    ///
    /// - `try_map` assumes when not using `Option` wrapper, that the entity has the component.
    ///   If it does not, it will not run the callback and return `None`.
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
    /// - a `Some(value)` if the callback has ran. Where the type of value is specified in `Return` generic (can be elided).
    ///   `None` if the callback has not ran.
    ///
    /// # Example
    ///
    /// ```
    /// use flecs_ecs::prelude::*;
    ///
    /// #[derive(Component)] struct Tag;
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
    /// let entity = world.entity()
    ///                   .set(Position { x: 10.0, y: 20.0 })
    ///                   .set_pair::<Tag, Position>(Position { x: 30.0, y: 40.0 });
    ///    
    /// let pos_x = entity.try_map::<&Position, _>(|(pos)| {
    ///     assert_eq!(pos.x, 10.0);
    ///     Some(pos.x)
    /// });
    /// assert!(pos_x.is_some());
    /// assert_eq!(pos_x.unwrap(), 10.0);
    ///
    /// let is_pos_x_10 = entity.try_map::<(Option<&Velocity>, &Position), _>( |(tag, pos)| {
    ///     assert_eq!(pos.x, 10.0);
    ///     assert!(tag.is_none());
    ///     Some(pos.x == 10.0)
    /// });
    /// assert!(is_pos_x_10.is_some());
    /// assert!(is_pos_x_10.unwrap());
    ///
    /// // no return type
    /// let has_run = entity.try_map::<(&mut(Tag,Position), &Position),_>(|(tag_pos_rel, pos)| {
    ///     assert_eq!(pos.x, 10.0);
    ///     assert_eq!(tag_pos_rel.x, 30.0);
    ///     Some(())
    /// });
    /// assert!(has_run.is_some());
    ///
    /// ```
    pub fn try_map<T: GetTuple, Return>(
        self,
        callback: impl for<'e> FnOnce(T::TupleType<'e>) -> Option<Return>,
    ) -> Option<Return> {
        let world_ptr = self.world.world_ptr_mut();

        #[cfg(not(feature = "flecs_unsafe_get"))]
        let record = unsafe { sys::ecs_read_begin(world_ptr, *self.id) as *mut ecs_record_t };

        #[cfg(feature = "flecs_unsafe_get")]
        let record = unsafe { sys::ecs_record_find(world_ptr, *self.id) };

        #[cfg(not(feature = "flecs_unsafe_get"))]
        if record.is_null() {
            return None;
        }

        #[cfg(feature = "flecs_unsafe_get")]
        if unsafe { (*record).table.is_null() } {
            return None;
        }

        let tuple_data = T::create_ptrs::<false>(self.world, self.id, record);
        let has_all_components = tuple_data.has_all_components();

        let ret = if has_all_components {
            let tuple = tuple_data.get_tuple();
            callback(tuple)
        } else {
            None
        };

        #[cfg(not(feature = "flecs_unsafe_get"))]
        unsafe {
            sys::ecs_read_end(record);
        }

        ret
    }

    /// gets mutable or immutable component(s) and/or relationship(s) from an entity in a callback.
    /// each component type must be marked `&` or `&mut` to indicate if it is mutable or not.
    /// use `Option` wrapper to indicate if the component is optional.
    /// use `()` tuple format when getting multiple components.
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
    /// #[derive(Component)] struct Tag;
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
    /// let entity = world.entity()
    ///                   .set(Position { x: 10.0, y: 20.0 })
    ///                   .set_pair::<Tag, Position>(Position { x: 30.0, y: 40.0 });
    ///
    /// let position_parent = Position { x: 20.0, y: 30.0 };
    ///
    /// let pos_actual = entity.map::<&Position, _>(|pos| {
    ///     assert_eq!(pos.x, 10.0);
    ///     // Calculate actual position
    ///     Position {
    ///         x: pos.x + position_parent.x,
    ///         y: pos.y + position_parent.y,
    ///     }
    /// });
    ///
    /// let pos_x = entity.map::<(Option<&Velocity>, &Position),_>( |(vel, pos)| {
    ///     assert_eq!(pos.x, 10.0);
    ///     assert!(vel.is_none());
    ///     pos.x
    /// });
    /// assert_eq!(pos_x, 10.0);
    ///
    /// let is_x_10 = entity.map::<(&mut(Tag,Position), &Position), _>(|(tag_pos_rel, pos)| {
    ///     assert_eq!(pos.x, 10.0);
    ///     assert_eq!(tag_pos_rel.x, 30.0);
    ///     pos.x == 10.0
    /// });
    /// assert!(is_x_10);
    ///
    /// ```
    pub fn map<T: GetTuple, Return>(
        self,
        callback: impl for<'e> FnOnce(T::TupleType<'e>) -> Return,
    ) -> Return {
        let world_ptr = self.world.world_ptr_mut();

        #[cfg(not(feature = "flecs_unsafe_get"))]
        let record = unsafe { sys::ecs_read_begin(world_ptr, *self.id) as *mut ecs_record_t };

        #[cfg(feature = "flecs_unsafe_get")]
        let record = unsafe { sys::ecs_record_find(world_ptr, *self.id) };

        #[cfg(not(feature = "flecs_unsafe_get"))]
        if record.is_null() {
            panic!("Entity does not have any components");
        }

        #[cfg(feature = "flecs_unsafe_get")]
        if unsafe { (*record).table.is_null() } {
            panic!("Entity does not have any components");
        }

        let tuple_data = T::create_ptrs::<true>(self.world, self.id, record);
        let tuple = tuple_data.get_tuple();

        let ret = callback(tuple);

        #[cfg(not(feature = "flecs_unsafe_get"))]
        unsafe {
            sys::ecs_read_end(record);
        }

        ret
    }

    /// Get component value or pair as untyped pointer
    ///
    /// # Arguments
    ///
    /// * `component_id` - The component to get
    ///
    /// # Returns
    ///
    /// * `*const c_void` - Pointer to the component value, nullptr if the entity does not have the component
    ///
    /// # See also
    ///
    /// * C++ API: `entity_view::get`
    #[doc(alias = "entity_view::get")]
    pub fn get_untyped(self, component_id: impl IntoId) -> *const c_void {
        unsafe { sys::ecs_get_id(self.world.world_ptr_mut(), *self.id, *component_id.into()) }
    }
    /// Get mutable component value or pair (untyped).
    /// This operation returns a mutable ref to the component. If a base entity had
    /// the component, it will be overridden, and the value of the base component
    /// will be copied to the entity before this function returns.
    ///
    /// # Arguments
    ///
    /// * `comp`: The component to get.
    ///
    /// # Returns
    ///
    /// Pointer to the component value.
    ///
    /// # See also
    ///
    /// * C++ API: `entity_view::get_mut`
    #[doc(alias = "entity_view::get_mut")]
    pub fn get_untyped_mut(self, id: impl IntoId) -> *mut c_void {
        unsafe { sys::ecs_get_mut_id(self.world.world_ptr_mut(), *self.id(), *id.into()) }
    }

    /// Get target for a given pair.
    ///
    /// This operation returns the target for a given pair. The optional
    /// index can be used to iterate through targets, in case the entity `get_has`
    /// multiple instances for the same relationship.
    ///
    /// # Type Parameters
    ///
    /// * `First` - The first element of the pair.
    ///
    /// # Arguments
    ///
    /// * `index` - The index (0 for the first instance of the relationship).
    ///
    /// # See also
    ///
    /// * C++ API: `entity_view::target`
    #[doc(alias = "entity_view::target")]
    pub fn target<First: ComponentId>(self, index: i32) -> EntityView<'a> {
        EntityView::new_from(self.world, unsafe {
            sys::ecs_get_target(
                self.world.world_ptr_mut(),
                *self.id,
                First::id(self.world),
                index,
            )
        })
    }

    /// Get target for a given pair.
    ///
    /// This operation returns the target for a given pair. The optional
    /// index can be used to iterate through targets, in case the entity `get_has`
    /// multiple instances for the same relationship.
    ///
    /// # Arguments
    ///
    /// * `first` - The first element of the pair for which to retrieve the target.
    /// * `index` - The index (0 for the first instance of the relationship).
    ///
    /// # See also
    ///
    /// * C++ API: `entity_view::target`
    #[doc(alias = "entity_view::target")]
    pub fn target_id(self, first: impl Into<Entity>, index: i32) -> EntityView<'a> {
        EntityView::new_from(self.world, unsafe {
            sys::ecs_get_target(self.world.world_ptr_mut(), *self.id, *first.into(), index)
        })
    }

    /// Get the target of a pair for a given relationship id.
    ///
    /// This operation returns the first entity that has the provided id by following
    /// the specified relationship. If the entity itself has the id then entity will
    /// be returned. If the id cannot be found on the entity or by following the
    /// relationship, the operation will return 0.
    ///
    /// This operation can be used to lookup, for example, which prefab is providing
    /// a component by specifying the `IsA` pair:
    ///
    #[cfg_attr(doctest, doc = " ````no_test")]
    /// ```
    /// // Is Position provided by the entity or one of its base entities?
    /// get_target_by_relationship_and_component_id(world, EcsIsA, T::id<Position>(world))
    /// ```
    ///
    /// # Arguments
    ///
    /// * `relationship` - The relationship to follow.
    /// * `id` - The id to lookup.
    ///
    /// # Returns
    ///
    /// * The entity for which the target `get_has` been found.
    ///
    /// # See also
    ///
    /// * C++ API: `entity_view::target_for`
    #[doc(alias = "entity_view::target_for")]
    pub fn target_for_id(
        &self,
        relationship: impl Into<Entity>,
        component_id: impl IntoId,
    ) -> EntityView<'a> {
        EntityView::new_from(self.world, unsafe {
            sys::ecs_get_target_for_id(
                self.world.world_ptr_mut(),
                *self.id,
                *relationship.into(),
                *component_id.into(),
            )
        })
    }

    /// Get the target for a given component and relationship.
    ///
    /// This function is a convenient wrapper around `get_target_by_relationship_and_component_id`,
    /// allowing callers to provide a type and automatically deriving the component id.
    ///
    /// # Type Parameters
    ///
    /// * `T` - The component type to use for deriving the id.
    ///
    /// # Arguments
    ///
    /// * `relationship` - The relationship to follow.
    ///
    /// # Returns
    ///
    /// * The entity for which the target `get_has` been found.
    ///
    /// # See also
    ///
    /// * C++ API: `entity_view::target`
    #[doc(alias = "entity_view::target_for")]
    #[inline(always)]
    pub fn target_for<T: ComponentOrPairId>(
        self,
        relationship: impl Into<Entity>,
    ) -> EntityView<'a> {
        self.target_for_id(relationship, T::get_id(self.world))
    }

    // TODO this needs a better name and documentation, the rest of the cpp functions still have to be done as well
    // TODO, I removed the second template parameter and changed the fn parameter second to entityT, check validity
    /// Get the target for a given pair of components and a relationship.
    ///
    /// # Type Parameters
    ///
    /// * `First` - The first component type to use for deriving the id.
    ///
    /// # Arguments
    ///
    /// * `second` - The second element of the pair.
    ///
    /// # Returns
    ///
    /// * The entity for which the target has been found.
    ///
    /// # See also
    ///
    /// * C++ API: `entity_view::target`
    #[doc(alias = "entity_view::target_for")]
    pub fn target_for_first<First: ComponentId + DataComponent>(
        &self,
        second: impl Into<Entity>,
    ) -> *const First {
        let comp_id = First::id(self.world);
        ecs_assert!(
            std::mem::size_of::<First>() != 0,
            FlecsErrorCode::InvalidParameter,
            "First element is size 0"
        );
        unsafe {
            sys::ecs_get_id(
                self.world.world_ptr_mut(),
                comp_id,
                ecs_pair(comp_id, *second.into()),
            ) as *const First
        }
    }

    /// Get the depth for the given relationship.
    ///
    /// # Arguments
    ///
    /// * `relationship` - The relationship for which to get the depth.
    ///
    /// # Returns
    ///
    /// * The depth of the relationship.
    ///
    /// # See also
    ///
    /// * C++ API: `entity_view::depth`
    #[doc(alias = "entity_view::depth")]
    #[inline(always)]
    pub fn depth_id(self, relationship: impl Into<Entity>) -> i32 {
        unsafe { sys::ecs_get_depth(self.world.world_ptr_mut(), *self.id, *relationship.into()) }
    }

    /// Retrieves the depth for a specified relationship.
    ///
    /// This function is a convenient wrapper around `get_depth_id`, allowing callers
    /// to provide a type and automatically deriving the relationship id.
    ///
    /// # Type Parameters
    ///
    /// * `T` - The relationship type to use for deriving the id.
    ///
    /// # Returns
    ///
    /// * The depth of the relationship.
    ///
    /// # See also
    ///
    /// * C++ API: `entity_view::depth`
    #[doc(alias = "entity_view::depth")]
    #[inline(always)]
    pub fn depth<T: ComponentId>(self) -> i32 {
        self.depth_id(T::id(self.world))
    }

    /// Retrieves the parent of the entity.
    ///
    /// This function is shorthand for getting the target using the `EcsChildOf` relationship.
    ///
    /// # Returns
    ///
    /// * The parent of the entity.
    ///
    /// # See also
    ///
    /// * C++ API: `entity_view::parent`
    #[doc(alias = "entity_view::parent")]
    #[inline(always)]
    pub fn parent(self) -> EntityView<'a> {
        self.target_id(ECS_CHILD_OF, 0)
    }

    /// Lookup an entity by name.
    ///
    /// Lookup an entity in the scope of this entity. The provided path may
    /// contain double colons as scope separators, for example: "`Foo::Bar`".
    ///
    /// # Arguments
    ///
    /// * `path` - The name of the entity to lookup.
    /// * `recursively` - Recursively traverse up the tree until entity is found.
    ///
    /// # Returns
    ///
    /// The found entity, or `Entity::null` if no entity matched.
    ///
    /// # See also
    ///
    /// * C++ API: `entity_view::lookup`
    #[doc(alias = "entity_view::lookup")]
    #[inline(always)]
    fn try_lookup_impl(self, name: &str, recursively: bool) -> Option<EntityView<'a>> {
        let name = compact_str::format_compact!("{}\0", name);

        ecs_assert!(
            self.id != 0,
            FlecsErrorCode::InvalidParameter,
            "invalid lookup from null handle"
        );
        let id = unsafe {
            sys::ecs_lookup_path_w_sep(
                self.world.world_ptr_mut(),
                *self.id,
                name.as_ptr() as *const _,
                SEPARATOR.as_ptr(),
                SEPARATOR.as_ptr(),
                recursively,
            )
        };

        if id == 0 {
            None
        } else {
            Some(EntityView::new_from(self.world, id))
        }
    }

    /// Lookup an entity by name.
    /// The entity is searched recursively recursively traversing
    /// up the tree until found.
    ///
    /// The provided path may contain double colons as scope separators,
    /// for example: "`Foo::Bar`".
    ///
    /// # Arguments
    ///
    /// * `name` - The name of the entity to lookup.
    ///
    /// # Returns
    ///
    /// The entity if found, otherwise `None`.
    ///
    /// # See also
    ///
    /// * C++ API: `entity_view::lookup`
    #[doc(alias = "entity_view::lookup")]
    #[inline(always)]
    pub fn try_lookup_recursive(&self, name: &str) -> Option<EntityView> {
        self.try_lookup_impl(name, true)
    }

    /// Lookup an entity by name, only in the current scope of the entity.
    ///
    /// Lookup an entity in the scope of this entity. The provided path may
    /// contain double colons as scope separators, for example: "`Foo::Bar`".
    ///
    /// # Arguments
    ///
    /// * `name` - The name of the entity to lookup.
    ///
    /// # Returns
    ///
    /// The entity if found, otherwise `None`.
    ///
    /// # See also
    ///
    /// * C++ API: `entity_view::lookup`
    #[doc(alias = "entity_view::lookup")]
    #[inline(always)]
    pub fn try_lookup(&self, name: &str) -> Option<EntityView> {
        self.try_lookup_impl(name, false)
    }

    /// Lookup an entity by name.
    /// The entity is searched recursively recursively traversing
    /// up the tree until found.
    ///
    /// The provided path may contain double colons as scope separators,
    /// for example: "`Foo::Bar`".
    ///
    /// # Safety
    ///
    /// This function can return an entity with id 0 if the entity is not found.
    /// Ensure that the entity exists before using it.
    ///
    /// # Arguments
    ///
    /// * `name` - The name of the entity to lookup.
    ///
    /// # Returns
    ///
    /// The entity, entity id will be 0 if not found.
    ///
    /// # See also
    ///
    /// * C++ API: `entity_view::lookup`
    #[doc(alias = "entity_view::lookup")]
    #[inline(always)]
    pub fn lookup_recursively(&self, name: &str) -> EntityView {
        self.try_lookup_recursive(name)
            .expect("Entity not found, when unsure, use try_lookup_recursive")
    }

    /// Lookup an entity by name, only in the current scope of the entity.
    ///
    /// Lookup an entity in the scope of this entity. The provided path may
    /// contain double colons as scope separators, for example: "`Foo::Bar`".
    ///
    /// # Safety
    ///
    /// This function can return an entity with id 0 if the entity is not found.
    /// Ensure that the entity exists before using it.
    ///
    /// # Arguments
    ///
    /// * `name` - The name of the entity to lookup.
    ///
    /// # Returns
    ///
    /// The entity, entity id will be 0 if not found.
    ///
    /// # See also
    ///
    /// * C++ API: `entity_view::lookup`
    #[doc(alias = "entity_view::lookup")]
    #[inline(always)]
    pub fn lookup(&self, name: &str) -> EntityView {
        self.try_lookup(name)
            .expect("Entity not found, when unsure, use try_lookup")
    }

    /// Check if entity has the provided entity.
    ///
    /// # Arguments
    ///
    /// * `entity` - The entity to check.
    ///
    /// # Returns
    ///
    /// True if the entity has the provided entity, false otherwise.
    ///
    /// # See also
    ///
    /// * C++ API: `entity_view::has`
    #[doc(alias = "entity_view::has")]
    #[inline(always)]
    pub fn has_id(self, entity: impl IntoId) -> bool {
        unsafe { sys::ecs_has_id(self.world.world_ptr_mut(), *self.id, *entity.into()) }
    }

    /// Check if entity has the provided struct component.
    ///
    /// # Type Parameters
    ///
    /// * `T` - The component to check.
    ///
    /// # Returns
    ///
    /// True if the entity has the provided component, false otherwise.
    ///
    /// # See also
    ///
    /// * C++ API: `entity_view::has`
    #[doc(alias = "entity_view::has")]
    pub fn has<T: ComponentOrPairId>(self) -> bool {
        if !T::IS_ENUM {
            unsafe { sys::ecs_has_id(self.world.world_ptr_mut(), *self.id, T::get_id(self.world)) }
        } else {
            let component_id = T::get_id(self.world);
            self.has_id((component_id, ECS_WILDCARD))
        }
    }

    /// Check if entity has the provided enum constant.
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
    /// True if the entity has the provided constant, false otherwise.
    ///
    /// # See also
    ///
    /// * C++ API: `entity_view::has`
    #[doc(alias = "entity_view::has")]
    pub fn has_enum<T>(self, constant: T) -> bool
    where
        T: ComponentId + ComponentType<Enum> + EnumComponentInfo,
    {
        let component_id: IdT = T::id(self.world);
        // Safety: we know the enum fields are registered because of the previous T::id call
        let enum_constant_entity_id = unsafe { constant.id_variant_unchecked(self.world) };

        ecs_assert!(
            *enum_constant_entity_id.id != 0,
            FlecsErrorCode::InvalidParameter,
            "Constant was not found in Enum reflection data. Did you mean to use has<E>() instead of has(E)?"
        );

        self.has_id((component_id, enum_constant_entity_id))
    }

    // this is pub(crate) because it's used for development purposes only
    pub(crate) fn has_enum_id<T>(self, enum_id: impl Into<Entity>, constant: T) -> bool
    where
        T: ComponentId + ComponentType<Enum> + EnumComponentInfo,
    {
        let enum_constant_entity_id = constant.id_variant(self.world);
        self.has_id((enum_id.into(), enum_constant_entity_id))
    }

    /// Check if entity has the provided pair.
    ///
    /// # Type Parameters
    ///
    /// * `First` - The first element of the pair.
    ///
    /// # Arguments
    ///
    /// * `second` - The second element of the pair.
    ///
    /// # Returns
    ///
    /// True if the entity has the provided component, false otherwise.
    ///
    /// # See also
    ///
    /// * C++ API: `entity_view::has`
    #[doc(alias = "entity_view::has")]
    pub fn has_first<First: ComponentId>(self, second: impl Into<Entity>) -> bool {
        self.has_id((First::id(self.world), second.into()))
    }

    /// Check if entity has the provided pair.
    ///
    /// # Type Parameters
    ///
    /// * `Second` - The second element of the pair.
    ///
    /// # Arguments
    ///
    /// * `first` - The first element of the pair.
    ///
    /// # Returns
    ///
    /// True if the entity has the provided component, false otherwise.
    ///
    /// # See also
    ///
    /// * C++ API: `entity_view::has`
    #[doc(alias = "entity_view::has")]
    pub fn has_second<Second: ComponentId>(self, first: impl Into<Entity>) -> bool {
        self.has_id((first.into(), Second::id(self.world)))
    }

    /// Check if entity has the provided pair with an enum constant.
    ///
    /// # Type Parameters
    ///
    /// * `T` - The first element of the pair.
    /// * `U` - The second element of the pair as an enum constant.
    ///
    /// # Arguments
    ///
    /// * `constant` - The enum constant.
    ///
    /// # Returns
    ///
    /// True if the entity has the provided component, false otherwise.
    ///
    /// # See also
    ///
    /// * C++ API: `entity_view::has`
    #[doc(alias = "entity_view::has")]
    pub fn has_pair_enum<T: ComponentId, U: ComponentId + EnumComponentInfo>(
        &self,
        constant: U,
    ) -> bool {
        let component_id: IdT = T::id(self.world);
        let enum_constant_entity_id = constant.id_variant(self.world);

        self.has_id((component_id, enum_constant_entity_id))
    }

    /// Check if the entity owns the provided entity (pair, component, entity).
    /// An entity is owned if it is not shared from a base entity.
    ///
    /// # Arguments
    /// - `entity_id`: The entity to check.
    ///
    /// # Returns
    /// - `true` if the entity owns the provided entity, `false` otherwise.
    ///
    /// # See also
    ///
    /// * C++ API: `entity_view::owns`
    #[doc(alias = "entity_view::owns")]
    pub fn owns_id(self, entity_id: impl IntoId) -> bool {
        unsafe { sys::ecs_owns_id(self.world.world_ptr_mut(), *self.id, *entity_id.into()) }
    }

    /// Check if the entity owns the provided component.
    /// A component is owned if it is not shared from a base entity.
    ///
    /// # Type Parameters
    ///
    /// - `T`: The component to check.
    ///
    /// # Returns
    ///
    /// - `true` if the entity owns the provided component, `false` otherwise.
    ///
    /// # See also
    ///
    /// * C++ API: `entity_view::owns`
    #[doc(alias = "entity_view::owns")]
    pub fn owns<T: ComponentOrPairId>(self) -> bool {
        unsafe { sys::ecs_owns_id(self.world.world_ptr_mut(), *self.id, T::get_id(self.world)) }
    }

    /// Check if the entity owns the provided pair.
    /// A pair is owned if it is not shared from a base entity.
    ///
    /// # Type Parameters
    /// - `First`: The first element of the pair.
    ///
    /// # Arguments
    ///
    /// - `second`: The second element of the pair.
    ///
    /// # Returns
    ///
    /// - `true` if the entity owns the provided pair, `false` otherwise.
    ///
    /// # See also
    ///
    /// * C++ API: `entity_view::owns`
    #[doc(alias = "entity_view::owns")]
    pub fn owns_first<First: ComponentId>(self, second: impl Into<Entity>) -> bool {
        unsafe {
            sys::ecs_owns_id(
                self.world.world_ptr_mut(),
                *self.id,
                ecs_pair(First::id(self.world), *second.into()),
            )
        }
    }

    /// Check if the entity owns the provided pair.
    /// A pair is owned if it is not shared from a base entity.
    ///
    /// # Type Parameters
    /// - `Second`: The first element of the pair.
    ///
    /// # Arguments
    ///
    /// - `first`: The second element of the pair.
    ///
    /// # Returns
    ///
    /// - `true` if the entity owns the provided pair, `false` otherwise.
    ///
    /// # See also
    ///
    /// * C++ API: `entity_view::owns`
    #[doc(alias = "entity_view::owns")]
    pub fn owns_second<Second: ComponentId>(self, first: impl Into<Entity>) -> bool {
        unsafe {
            sys::ecs_owns_id(
                self.world.world_ptr_mut(),
                *self.id,
                ecs_pair(*first.into(), Second::id(self.world)),
            )
        }
    }

    /// Test if id is enabled.
    ///
    /// # Arguments
    /// - `id`: The id to test.
    ///
    /// # Returns
    /// - `true` if enabled, `false` if not.
    ///
    /// # See also
    ///
    /// * C++ API: `entity_view::enabled`
    #[doc(alias = "entity_view::enabled")]
    pub fn is_enabled_id(self, id: impl IntoId) -> bool {
        unsafe { sys::ecs_is_enabled_id(self.world.world_ptr_mut(), *self.id, *id.into()) }
    }

    /// Test if component is enabled.
    ///
    /// # Type Parameters
    /// - `T`: The component to test.
    ///
    /// # Returns
    /// - `true` if enabled, `false` if not.
    ///
    /// # See also
    ///
    /// * C++ API: `entity_view::enabled`
    #[doc(alias = "entity_view::enabled")]
    pub fn is_enabled<T: ComponentOrPairId>(self) -> bool {
        unsafe {
            sys::ecs_is_enabled_id(self.world.world_ptr_mut(), *self.id, T::get_id(self.world))
        }
    }

    /// Test if pair is enabled.
    ///
    /// # Type Parameters
    /// - `T`: The first element of the pair.
    ///
    /// # Arguments
    /// - `second`: The second element of the pair.
    ///
    /// # Returns
    /// - `true` if enabled, `false` if not.
    ///
    /// # See also
    ///
    /// * C++ API: `entity_view::enabled`
    #[doc(alias = "entity_view::enabled")]
    pub fn is_enabled_first<T: ComponentId>(self, second: impl Into<Entity>) -> bool {
        self.is_enabled_id((T::id(self.world), second.into()))
    }

    /// Test if pair is enabled.
    ///
    /// # Type Parameters
    /// - `T`: The second element of the pair.
    ///
    /// # Arguments
    /// - `first`: The second element of the pair.
    ///
    /// # Returns
    /// - `true` if enabled, `false` if not.
    ///
    /// # See also
    ///
    /// * C++ API: `entity_view::enabled`
    #[doc(alias = "entity_view::enabled")]
    pub fn is_enabled_second<U: ComponentId>(self, first: impl Into<Entity>) -> bool {
        self.is_enabled_id((first.into(), U::id(self.world)))
    }

    /// Clones the current entity to a new or specified entity.
    ///
    /// This function creates a clone of the current entity. If `dest_id` is provided
    /// (i.e., not zero), it will clone the current entity to the entity with `dest_id`.
    /// If `dest_id` is zero, it will create a new entity and clone the current entity
    /// to the newly created entity.
    ///
    /// If `copy_value` is set to `true`, the value of the current entity is also copied to
    /// the destination entity. Otherwise, only the entity's structure is cloned without copying the value.
    ///
    /// # Arguments
    /// - `copy_value`: A boolean indicating whether to copy the entity's value to the destination entity.
    /// - `dest_id`: The identifier of the destination entity. If zero, a new entity is created.
    ///
    /// # Returns
    /// - An `Entity` object representing the destination entity.
    ///
    /// # See also
    ///
    /// * C++ API: `entity_view::clone`
    #[doc(alias = "entity_view::clone")]
    #[inline(always)]
    pub fn duplicate(self, copy_value: bool) -> EntityView<'a> {
        let dest_entity = EntityView::new(self.world());
        unsafe {
            sys::ecs_clone(
                self.world.world_ptr_mut(),
                *dest_entity.id,
                *self.id,
                copy_value,
            )
        };
        dest_entity
    }

    /// Clones the current entity to a new or specified entity.
    ///
    /// This function creates a clone of the current entity. If `dest_id` is provided
    /// (i.e., not zero), it will clone the current entity to the entity with `dest_id`.
    /// If `dest_id` is zero, it will create a new entity and clone the current entity
    /// to the newly created entity.
    ///
    /// If `copy_value` is set to `true`, the value of the current entity is also copied to
    /// the destination entity. Otherwise, only the entity's structure is cloned without copying the value.
    ///
    /// # Arguments
    /// - `copy_value`: A boolean indicating whether to copy the entity's value to the destination entity.
    /// - `dest_id`: The identifier of the destination entity. If zero, a new entity is created.
    ///
    /// # Returns
    /// - An `Entity` object representing the destination entity.
    ///
    /// ## Safety
    /// This function makes use of `unsafe` operations to interact with the underlying ECS.
    /// Ensure that the provided `dest_id` is valid or zero
    ///
    /// # See also
    ///
    /// * C++ API: `entity_view::clone`
    #[doc(alias = "entity_view::clone")]
    #[inline(always)]
    pub fn duplicate_into(self, copy_value: bool, dest_id: impl Into<Entity>) -> EntityView<'a> {
        let mut dest_id = *dest_id.into();
        if dest_id == 0 {
            dest_id = unsafe { sys::ecs_new(self.world.world_ptr_mut()) };
        }

        let dest_entity = EntityView::new_from(self.world, dest_id);
        unsafe { sys::ecs_clone(self.world.world_ptr_mut(), dest_id, *self.id, copy_value) };
        dest_entity
    }

    /// Returns a mutable entity handle for the current stage.
    ///
    /// When an entity handle created from the world is used while the world is
    /// in staged mode, it will only allow for readonly operations since
    /// structural changes are not allowed on the world while in staged mode.
    ///
    /// To perform mutations on the entity, this operation provides a handle to the
    /// entity that uses the stage instead of the actual world.
    ///
    /// Note that staged entity handles should never be stored persistently, in
    /// components or elsewhere. An entity handle should always point to the
    /// main world.
    ///
    /// Also note that this operation is not necessary when doing mutations on an
    /// entity outside of a system. It is allowed to perform entity operations
    /// directly on the world, as long as the world is not in staged mode.
    ///
    /// # Arguments
    /// - `stage`: The current stage.
    ///
    /// # Returns
    /// - An entity handle that allows for mutations in the current stage.
    ///
    /// # See also
    ///
    /// * C++ API: `entity_view::mut`
    #[doc(alias = "entity_view::mut")]
    pub fn mut_current_stage(self, stage: impl IntoWorld<'a>) -> EntityView<'a> {
        ecs_assert!(
            !stage.world().is_readonly(),
            FlecsErrorCode::InvalidParameter,
            "cannot use readonly world/stage to create mutable handle"
        );

        EntityView::new_from(stage, *self.id)
    }

    /// Returns a mutable entity handle for the current stage from another entity.
    ///
    /// This operation allows for the construction of a mutable entity handle
    /// from another entity. This is useful in `each` functions, which only
    /// provide a handle to the entity being iterated over.
    ///
    /// # Arguments
    /// - `entity`: Another mutable entity.
    ///
    /// # Returns
    /// - An entity handle that allows for mutations in the current stage.
    ///
    /// # See also
    ///
    /// * C++ API: `entity_view::mut`
    #[doc(alias = "entity_view::mut")]
    pub fn mut_stage_of<T>(self, entity: T) -> EntityView<'a>
    where
        T: Into<Entity> + IntoWorld<'a>,
    {
        ecs_assert!(
            !entity.world().is_readonly(),
            FlecsErrorCode::InvalidParameter,
            "cannot use entity created for readonly world/stage to create mutable handle"
        );

        EntityView::new_from(entity.world(), *self.id)
    }

    //might not be needed, in the original c++ impl it was used in the get_mut functions.
    ///
    /// # See also
    ///
    /// * C++ API: `entity_view::set_stage`
    #[doc(alias = "entity_view::set_stage")]
    #[doc(hidden)]
    fn set_stage(self, stage: impl IntoWorld<'a>) -> EntityView<'a> {
        EntityView::new_from(stage, *self.id)
    }
}

// Event mixin
impl<'a> EntityView<'a> {
    /// Emit event for entity
    ///
    /// # Safety
    /// Caller must ensure that any type associated with `event` is a ZST
    ///
    /// # Arguments
    ///
    /// * event - the event to emit
    ///
    /// # See also
    ///
    /// * C++ API: `entity_view::emit`
    #[doc(alias = "entity_view::emit")]
    pub unsafe fn emit_id(self, event: impl Into<Entity>) {
        self.world().event_id(event).target(self).emit(&mut ());
    }

    /// Emit event with an immutable payload for entity.
    ///
    /// # Type Parameters
    ///
    /// * T - the event type to emit.
    ///
    /// # See also
    ///
    /// * C++ API: `entity_view::emit`
    #[doc(alias = "entity_view::emit")]
    pub fn emit<T: ComponentId>(self, event: &mut T) {
        self.world().event().target(self).emit(event);
    }

    /// Enqueue event for entity.
    ///
    /// # Safety
    ///
    ///
    /// # Arguments
    ///
    /// * event - the event to enqueue
    ///
    /// # See also
    ///
    /// * C++ API: `entity_view::enqueue`
    #[doc(alias = "entity_view::enqueue")]
    pub unsafe fn enqueue_id(self, event: impl Into<Entity>) {
        self.world().event_id(event).target(self).enqueue(());
    }

    /// enqueue event with payload for entity.
    ///
    /// # Type Parameters
    ///
    /// * T - the event type to enqueue. Type must contain data (not empty struct).
    ///
    /// # Usage:
    ///
    #[cfg_attr(doctest, doc = " ````no_test")]
    /// ```rust
    /// struct Resize {
    ///    width: i32,
    ///   height: i32,
    /// }
    ///
    /// world.defer_begin();
    /// entity.enqueue(Resize{width: 10, height: 20});
    /// world.defer_end();
    /// ```
    ///
    /// # See also
    ///
    /// * C++ API: `entity_view::enqueue`
    #[doc(alias = "entity_view::enqueue")]
    pub fn enqueue<T: ComponentId>(self, event: T) {
        self.world().event().target(self).enqueue(event);
    }
}

// Event/Observe mixin
impl<'a> EntityView<'a> {
    /// Register the callback for the entity observer for empty events.
    ///
    /// The "empty" iterator accepts a function that is invoked for each matching event.
    /// The following function signature is valid:
    ///  - `func()`
    ///
    /// # Arguments
    ///
    /// * `func` - The callback function
    ///
    /// See also
    ///
    /// * C++ API: `entity_builder::observe`
    #[doc(alias = "entity_builder::observe")]
    pub fn observe<C>(self, func: impl FnMut() + 'static) -> Self
    where
        C: ComponentId + TagComponent,
    {
        self.observe_impl::<C, _>(func)
    }

    fn observe_impl<C, Func>(self, func: Func) -> Self
    where
        Func: FnMut() + 'static,
        C: ComponentId,
    {
        let new_binding_ctx = Box::<ObserverEntityBindingCtx>::default();
        let binding_ctx = Box::leak(new_binding_ctx);

        let empty_func = Box::new(func);
        let empty_static_ref = Box::leak(empty_func);

        binding_ctx.empty = Some(empty_static_ref as *mut _ as *mut c_void);
        binding_ctx.free_empty = Some(Self::on_free_empty);

        Self::entity_observer_create(
            self.world.world_ptr_mut(),
            C::id(self.world),
            *self.id,
            binding_ctx,
            Some(Self::run_empty::<Func> as unsafe extern "C" fn(_)),
        );
        self
    }

    /// Register the callback for the entity observer for empty events with entity parameter.
    ///
    /// The `empty_entity` iterator accepts a function that is invoked for each matching event.
    /// The following function signature is valid:
    ///  - `func(&mut EntityView)`
    ///
    /// # Arguments
    ///
    /// * `func` - The callback function
    ///
    /// See also
    ///
    /// * C++ API: `entity_builder::observe`
    #[doc(alias = "entity_builder::observe")]
    pub fn observe_entity<C>(self, func: impl FnMut(&mut EntityView) + 'static) -> Self
    where
        C: ComponentId + TagComponent,
    {
        self.observe_entity_impl::<C, _>(func)
    }

    fn observe_entity_impl<C, Func>(self, func: Func) -> Self
    where
        Func: FnMut(&mut EntityView) + 'static,
        C: ComponentId,
    {
        let new_binding_ctx = Box::<ObserverEntityBindingCtx>::default();
        let binding_ctx = Box::leak(new_binding_ctx);

        let empty_func = Box::new(func);
        let empty_static_ref = Box::leak(empty_func);

        binding_ctx.empty_entity = Some(empty_static_ref as *mut _ as *mut c_void);
        binding_ctx.free_empty_entity = Some(Self::on_free_empty_entity);

        Self::entity_observer_create(
            self.world.world_ptr_mut(),
            C::id(self.world),
            *self.id,
            binding_ctx,
            Some(Self::run_empty_entity::<Func> as unsafe extern "C" fn(_)),
        );
        self
    }

    /// Register the callback for the entity observer for `payload` events.
    ///
    /// The "payload" iterator accepts a function that is invoked for each matching event.
    /// The following function signature is valid:
    ///  - `func(&mut EventData)`
    ///
    /// # Arguments
    ///
    /// * `func` - The callback function
    ///
    /// See also
    ///
    /// * C++ API: `entity_builder::observe`
    #[doc(alias = "entity_builder::observe")]
    pub fn observe_payload<C>(self, func: impl FnMut(&C) + 'static) -> Self
    where
        C: ComponentId + DataComponent,
    {
        self.observe_payload_impl::<C, _>(func)
    }

    fn observe_payload_impl<C, Func>(self, func: Func) -> Self
    where
        Func: FnMut(&C) + 'static,
        C: ComponentId,
    {
        let new_binding_ctx = Box::<ObserverEntityBindingCtx>::default();
        let binding_ctx = Box::leak(new_binding_ctx);

        let empty_func = Box::new(func);
        let empty_static_ref = Box::leak(empty_func);

        binding_ctx.payload = Some(empty_static_ref as *mut _ as *mut c_void);
        binding_ctx.free_payload = Some(Self::on_free_payload::<C>);

        Self::entity_observer_create(
            self.world.world_ptr_mut(),
            C::id(self.world),
            *self.id,
            binding_ctx,
            Some(Self::run_payload::<C, Func> as unsafe extern "C" fn(_)),
        );
        self
    }

    /// Register the callback for the entity observer for an event with payload and entity parameter.
    ///
    /// The "payload" iterator accepts a function that is invoked for each matching event.
    /// The following function signature is valid:
    ///  - `func(&mut EntityView, &mut EventData)`
    ///
    /// # Arguments
    ///
    /// * `func` - The callback function
    ///
    /// See also
    ///
    /// * C++ API: `entity_builder::observe`
    #[doc(alias = "entity_builder::observe")]
    pub fn observe_payload_entity<C>(self, func: impl FnMut(&mut EntityView, &C) + 'static) -> Self
    where
        C: ComponentId + DataComponent,
    {
        self.observe_payload_entity_impl::<C, _>(func)
    }

    fn observe_payload_entity_impl<C, Func>(self, func: Func) -> Self
    where
        Func: FnMut(&mut EntityView, &C) + 'static,
        C: ComponentId,
    {
        let new_binding_ctx = Box::<ObserverEntityBindingCtx>::default();
        let binding_ctx = Box::leak(new_binding_ctx);

        let empty_func = Box::new(func);
        let empty_static_ref = Box::leak(empty_func);

        binding_ctx.payload_entity = Some(empty_static_ref as *mut _ as *mut c_void);
        binding_ctx.free_payload_entity = Some(Self::on_free_payload_entity::<C>);

        Self::entity_observer_create(
            self.world.world_ptr_mut(),
            C::id(self.world),
            *self.id,
            binding_ctx,
            Some(Self::run_payload_entity::<C, Func> as unsafe extern "C" fn(_)),
        );
        self
    }
}

// entity observer creation
impl<'a> EntityView<'a> {
    pub(crate) fn entity_observer_create(
        world: *mut WorldT,
        event: EntityT,
        entity: EntityT,
        binding_ctx: *mut ObserverEntityBindingCtx,
        callback: sys::ecs_iter_action_t,
    ) {
        let mut desc = sys::ecs_observer_desc_t::default();
        desc.events[0] = event;
        desc.query.terms[0].id = ECS_ANY;
        desc.query.terms[0].src.id = entity;
        desc.callback = callback;
        desc.callback_ctx = binding_ctx as *mut c_void;
        desc.callback_ctx_free = Some(Self::binding_entity_ctx_drop);

        let observer = unsafe { sys::ecs_observer_init(world, &desc) };
        ecs_add_pair(world, observer, ECS_CHILD_OF, entity);
    }

    /// Callback of the observe functionality
    ///
    /// # Arguments
    ///
    /// * `iter` - The iterator which gets passed in from `C`
    ///
    /// # See also
    ///
    /// * C++ API: `entity_observer_delegate::invoke`
    #[doc(alias = "entity_observer_delegate::invoke")]
    pub(crate) unsafe extern "C" fn run_empty<Func>(iter: *mut IterT)
    where
        Func: FnMut(),
    {
        let ctx: *mut ObserverEntityBindingCtx = (*iter).callback_ctx as *mut _;
        let empty = (*ctx).empty.unwrap();
        let empty = &mut *(empty as *mut Func);
        let iter_count = (*iter).count as usize;

        sys::ecs_table_lock((*iter).world, (*iter).table);

        for _i in 0..iter_count {
            empty();
        }

        sys::ecs_table_unlock((*iter).world, (*iter).table);
    }

    /// Callback of the observe functionality
    ///
    /// # Arguments
    ///
    /// * `iter` - The iterator which gets passed in from `C`
    ///
    /// # See also
    ///
    /// * C++ API: `entity_observer_delegate::invoke`
    #[doc(alias = "entity_observer_delegate::invoke")]
    pub(crate) unsafe extern "C" fn run_empty_entity<Func>(iter: *mut IterT)
    where
        Func: FnMut(&mut EntityView),
    {
        let ctx: *mut ObserverEntityBindingCtx = (*iter).callback_ctx as *mut _;
        let empty = (*ctx).empty_entity.unwrap();
        let empty = &mut *(empty as *mut Func);
        let iter_count = (*iter).count as usize;

        sys::ecs_table_lock((*iter).world, (*iter).table);

        for _i in 0..iter_count {
            let world = WorldRef::from_ptr((*iter).world);
            empty(&mut EntityView::new_from(
                world,
                sys::ecs_field_src(iter, 0),
            ));
        }

        sys::ecs_table_unlock((*iter).world, (*iter).table);
    }

    /// Callback of the observe functionality
    ///
    /// # Arguments
    ///
    /// * `iter` - The iterator which gets passed in from `C`
    ///
    /// # See also
    ///
    /// * C++ API: `entity_payload_observer_delegate::invoke`
    #[doc(alias = "entity_payload_observer_delegate::invoke")]
    pub(crate) unsafe extern "C" fn run_payload<C, Func>(iter: *mut IterT)
    where
        Func: FnMut(&C),
    {
        let ctx: *mut ObserverEntityBindingCtx = (*iter).callback_ctx as *mut _;
        let empty = (*ctx).payload.unwrap();
        let empty = &mut *(empty as *mut Func);
        let iter_count = (*iter).count as usize;

        sys::ecs_table_lock((*iter).world, (*iter).table);

        for _i in 0..iter_count {
            let data = (*iter).param as *mut C;
            let data_ref = &mut *data;
            empty(data_ref);
        }

        sys::ecs_table_unlock((*iter).world, (*iter).table);
    }

    /// Callback of the observe functionality
    ///
    /// # Arguments
    ///
    /// * `iter` - The iterator which gets passed in from `C`
    ///
    /// # See also
    ///
    /// * C++ API: `entity_payload_observer_delegate::invoke`
    #[doc(alias = "entity_payload_observer_delegate::invoke")]
    pub(crate) unsafe extern "C" fn run_payload_entity<C, Func>(iter: *mut IterT)
    where
        Func: FnMut(&mut EntityView, &C),
    {
        let ctx: *mut ObserverEntityBindingCtx = (*iter).callback_ctx as *mut _;
        let empty = (*ctx).payload_entity.unwrap();
        let empty = &mut *(empty as *mut Func);
        let iter_count = (*iter).count as usize;

        sys::ecs_table_lock((*iter).world, (*iter).table);

        for _i in 0..iter_count {
            let data = (*iter).param as *mut C;
            let data_ref = &mut *data;
            let world = WorldRef::from_ptr((*iter).world);
            empty(
                &mut EntityView::new_from(world, sys::ecs_field_src(iter, 0)),
                data_ref,
            );
        }

        sys::ecs_table_unlock((*iter).world, (*iter).table);
    }

    /// Callback to free the memory of the `empty` callback
    pub(crate) extern "C" fn on_free_empty(ptr: *mut c_void) {
        let ptr_func: *mut fn() = ptr as *mut fn();
        unsafe {
            ptr::drop_in_place(ptr_func);
        }
    }

    /// Callback to free the memory of the `empty_entity` callback
    pub(crate) extern "C" fn on_free_empty_entity(ptr: *mut c_void) {
        let ptr_func: *mut fn(&mut EntityView) = ptr as *mut fn(&mut EntityView);
        unsafe {
            ptr::drop_in_place(ptr_func);
        }
    }

    /// Callback to free the memory of the `payload` callback
    pub(crate) extern "C" fn on_free_payload<C>(ptr: *mut c_void) {
        let ptr_func: *mut fn(&mut C) = ptr as *mut fn(&mut C);
        unsafe {
            ptr::drop_in_place(ptr_func);
        }
    }

    /// Callback to free the memory of the `payload_entity` callback
    pub(crate) extern "C" fn on_free_payload_entity<C>(ptr: *mut c_void) {
        let ptr_func: *mut fn(&mut EntityView, &mut C) = ptr as *mut fn(&mut EntityView, &mut C);
        unsafe {
            ptr::drop_in_place(ptr_func);
        }
    }

    /// Executes the drop for the system binding context, meant to be used as a callback
    pub(crate) extern "C" fn binding_entity_ctx_drop(ptr: *mut c_void) {
        let ptr_struct: *mut ObserverEntityBindingCtx = ptr as *mut ObserverEntityBindingCtx;
        unsafe {
            ptr::drop_in_place(ptr_struct);
        }
    }
}

impl<'a> EntityView<'a> {
    /// Get Component from entity
    /// use `.unwrap()` or `.unwrap_unchecked()` or `get_unchecked()` if you're sure the entity has the component
    ///
    /// # Safety
    ///
    /// - This guarantees no safety with table locking that the reference cannot be invalidated by other operations.
    ///   Use with caution or use `try_get`, `get`, `map`, `try_map` variants.
    ///
    /// # See also
    ///
    /// * C++ API: `entity_view::get`
    #[doc(alias = "entity_view::get")]
    #[inline(always)]
    pub fn try_get_unchecked<T: ComponentId>(self) -> Option<&'a T::UnderlyingType> {
        if !T::IS_ENUM {
            if T::IS_TAG {
                // ecs_assert!(
                //     false,
                //     FlecsErrorCode::InvalidParameter,
                //     "component {} has no size",
                //     std::any::type_name::<T>()
                // );
                // None

                let component_id = T::id(self.world);

                unsafe {
                    (sys::ecs_get_id(self.world.world_ptr_mut(), *self.id, component_id)
                        as *const T::UnderlyingType)
                        .as_ref()
                }
            } else {
                let component_id = T::id(self.world);

                unsafe {
                    (sys::ecs_get_id(self.world.world_ptr_mut(), *self.id, component_id)
                        as *const T::UnderlyingType)
                        .as_ref()
                }
            }
        } else {
            let component_id: IdT = T::id(self.world);
            let target: IdT = unsafe {
                sys::ecs_get_target(self.world.world_ptr_mut(), *self.id, component_id, 0)
            };

            if target == 0 {
                // if there is no matching pair for (r,*), try just r
                unsafe {
                    (sys::ecs_get_id(self.world.world_ptr_mut(), *self.id, component_id)
                        as *const T::UnderlyingType)
                        .as_ref()
                }
            } else {
                // get constant value from constant entity
                let constant_value = unsafe {
                    sys::ecs_get_id(self.world.world_ptr_mut(), target, component_id)
                        as *const T::UnderlyingType
                };

                ecs_assert!(
                    !constant_value.is_null(),
                    FlecsErrorCode::InternalError,
                    "missing enum constant value {}",
                    std::any::type_name::<T>()
                );

                unsafe { constant_value.as_ref() }
            }
        }
    }
}
