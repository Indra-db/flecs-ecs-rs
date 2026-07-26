use core::ffi::c_char;

use flecs_ecs::core::*;
use flecs_ecs::sys;

#[cfg(feature = "std")]
extern crate std;

extern crate alloc;
use alloc::string::String;

use flecs_ecs_derive::extern_abi;

/// Custom error type for `try_first_only` failures.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum FirstOnlyError {
    NoEntities,
    MoreThanOneEntity,
}

impl core::fmt::Display for FirstOnlyError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            FirstOnlyError::NoEntities => write!(f, "no entities matched the query"),
            FirstOnlyError::MoreThanOneEntity => {
                write!(f, "more than one entity matched the query")
            }
        }
    }
}

impl core::error::Error for FirstOnlyError {}

/// Convert a query to a string expression using `ecs_query_str`.
/// The resulting expression can be parsed to create the same query.
pub(crate) fn query_expr_string(query: *const sys::ecs_query_t) -> String {
    let result: *mut c_char = unsafe { sys::ecs_query_str(query) };
    if result.is_null() {
        return String::new();
    }

    let rust_string = unsafe { core::ffi::CStr::from_ptr(result) }
        .to_string_lossy()
        .into_owned();
    unsafe {
        if let Some(free_func) = sys::ecs_os_api.free_ {
            free_func(result as *mut _);
        }
    }
    rust_string
}

pub trait IterOperations {
    #[doc(hidden)]
    fn retrieve_iter(&self) -> sys::ecs_iter_t;

    #[doc(hidden)]
    fn retrieve_iter_stage<'a>(&self, stage: impl WorldProvider<'a>) -> sys::ecs_iter_t;

    #[doc(hidden)]
    fn iter_next(&self, iter: &mut sys::ecs_iter_t) -> bool;

    #[doc(hidden)]
    fn iter_next_func(&self) -> ExternIterNextFn;

    #[doc(hidden)]
    fn query_ptr(&self) -> *const sys::ecs_query_t;
}

pub trait QueryAPI<'a, P, T>: IterOperations + WorldProvider<'a>
where
    T: QueryTuple,
{
    // TODO once we have tests in place, I will split this functionality up into multiple functions, which should give a small performance boost
    // by caching if the query has used a "is_ref" operation.
    // is_ref is true for any query that contains fields that are not matched on the entity itself
    // so parents, prefabs but also singletons, or fields that are matched on a fixed entity (.with<Foo>().src_id(my_entity))
    /// Each iterator.
    /// The "each" iterator accepts a function that is invoked for each matching entity.
    /// The following function signatures is valid:
    ///  - func(comp1 : &mut T1, comp2 : &mut T2, ...)
    ///
    /// # Safety
    ///
    /// When the `flecs_safety_locks` feature is enabled, this function performs runtime checks
    /// for mutable component aliasing and sparse component handling.
    ///
    /// # See also
    ///
    /// * [`World::each()`]
    /// * [`QueryAPI::each_unchecked()`] - Unsafe variant without aliasing checks for maximum performance
    #[inline(always)]
    fn each(&self, func: impl FnMut(T::TupleType<'_>)) {
        self.each_internal::<true>(func);
    }

    /// Each iterator without aliasing checks.
    ///
    /// This is an unsafe variant of [`each()`](QueryAPI::each) that skips all mutable aliasing checks
    /// for maximum performance. Use this when you are certain that no component aliasing can occur.
    ///
    /// When the `flecs_safety_locks` feature is enabled, this function bypasses all runtime
    /// safety checks that [`each()`](QueryAPI::each) would normally perform.
    ///
    /// # Safety
    ///
    /// The caller must ensure that:
    /// - No mutable component references alias with each other
    /// - No mutable references alias with immutable references to the same component
    /// - The query terms don't create situations where the same component is accessed mutably multiple times
    ///
    /// Violating these guarantees will result in undefined behavior.
    ///
    /// # Example
    ///
    /// ```
    /// use flecs_ecs::prelude::*;
    ///
    /// #[derive(Component)]
    /// struct Position {
    ///     x: f32,
    ///     y: f32,
    /// }
    ///
    /// #[derive(Component)]
    /// struct Velocity {
    ///     x: f32,
    ///     y: f32,
    /// }
    ///
    /// let world = World::new();
    /// let query = world.new_query::<(&mut Position, &Velocity)>();
    ///
    /// // Safe: Position and Velocity are different components
    /// unsafe {
    ///     query.each_unchecked(|(pos, vel)| {
    ///         pos.x += vel.x;
    ///         pos.y += vel.y;
    ///     });
    /// }
    /// ```
    ///
    /// # See also
    ///
    /// * [`QueryAPI::each()`] - Safe variant with aliasing checks
    #[inline(always)]
    unsafe fn each_unchecked(&self, func: impl FnMut(T::TupleType<'_>)) {
        self.each_internal::<false>(func);
    }

    /// Internal implementation for both checked and unchecked each iteration.
    #[inline(always)]
    fn each_internal<const CHECKED: bool>(&self, mut func: impl FnMut(T::TupleType<'_>)) {
        let mut iter = IterGuard::new(self.retrieve_iter());
        let world = self.world();

        #[cfg(not(feature = "flecs_safety_locks"))]
        {
            while iter.next(|i| self.iter_next(i)) {
                internal_each_iter_next::<T, false, false>(&mut iter, &world, &mut func);
            }
        }

        #[cfg(feature = "flecs_safety_locks")]
        {
            if CHECKED {
                if iter.row_fields == 0 {
                    while iter.next(|i| self.iter_next(i)) {
                        internal_each_iter_next::<T, false, false>(&mut iter, &world, &mut func);
                    }
                } else {
                    while iter.next(|i| self.iter_next(i)) {
                        internal_each_iter_next::<T, false, true>(&mut iter, &world, &mut func);
                    }
                }
            } else {
                // Unchecked: always use false for sparse terms check
                while iter.next(|i| self.iter_next(i)) {
                    internal_each_iter_next::<T, false, false>(&mut iter, &world, &mut func);
                }
            }
        }
    }

    /// Each iterator.
    /// The "each" iterator accepts a function that is invoked for each matching entity.
    /// The following function signatures is valid:
    ///  - func(e : Entity , comp1 : &mut T1, comp2 : &mut T2, ...)
    ///
    /// When the `flecs_safety_locks` feature is enabled, this function performs runtime checks
    /// for mutable component aliasing and sparse component handling.
    ///
    /// # See also
    ///
    /// * [`World::each_entity()`]
    /// * [`QueryAPI::each_entity_unchecked()`] - Unsafe variant without aliasing checks for maximum performance
    #[inline(always)]
    fn each_entity(&self, func: impl FnMut(EntityView, T::TupleType<'_>)) {
        self.each_entity_internal::<true>(func);
    }

    /// Each entity iterator without aliasing checks.
    ///
    /// This is an unsafe variant of [`each_entity()`](QueryAPI::each_entity) that skips all mutable
    /// aliasing checks for maximum performance. Use this when you are certain that no component
    /// aliasing can occur.
    ///
    /// When the `flecs_safety_locks` feature is enabled, this function bypasses all runtime
    /// safety checks that [`each_entity()`](QueryAPI::each_entity) would normally perform.
    ///
    /// # Safety
    ///
    /// The caller must ensure that:
    /// - No mutable component references alias with each other
    /// - No mutable references alias with immutable references to the same component
    /// - The query terms don't create situations where the same component is accessed mutably multiple times
    ///
    /// Violating these guarantees will result in undefined behavior.
    ///
    /// # Example
    ///
    /// ```
    /// use flecs_ecs::prelude::*;
    ///
    /// #[derive(Component)]
    /// struct Position {
    ///     x: f32,
    ///     y: f32,
    /// }
    ///
    /// let world = World::new();
    /// let query = world.new_query::<&mut Position>();
    ///
    /// // Safe: Only one mutable component per entity
    /// unsafe {
    ///     query.each_entity_unchecked(|entity, pos| {
    ///         println!("{}: ({}, {})", entity.name(), pos.x, pos.y);
    ///         pos.x += 1.0;
    ///     });
    /// }
    /// ```
    ///
    /// # See also
    ///
    /// * [`QueryAPI::each_entity()`] - Safe variant with aliasing checks
    #[inline(always)]
    unsafe fn each_entity_unchecked(&self, func: impl FnMut(EntityView, T::TupleType<'_>)) {
        self.each_entity_internal::<false>(func);
    }

    /// Internal implementation for both checked and unchecked `each_entity` iteration.
    #[inline(always)]
    fn each_entity_internal<const CHECKED: bool>(
        &self,
        mut func: impl FnMut(EntityView, T::TupleType<'_>),
    ) {
        let world = self.world();
        let mut iter = IterGuard::new(self.retrieve_iter());

        #[cfg(not(feature = "flecs_safety_locks"))]
        {
            while iter.next(|i| self.iter_next(i)) {
                internal_each_entity_iter_next::<T, false, false>(&mut iter, &world, &mut func);
            }
        }
        #[cfg(feature = "flecs_safety_locks")]
        {
            if CHECKED {
                if iter.row_fields == 0 {
                    while iter.next(|i| self.iter_next(i)) {
                        internal_each_entity_iter_next::<T, false, false>(
                            &mut iter, &world, &mut func,
                        );
                    }
                } else {
                    while iter.next(|i| self.iter_next(i)) {
                        internal_each_entity_iter_next::<T, false, true>(
                            &mut iter, &world, &mut func,
                        );
                    }
                }
            } else {
                // Unchecked: always use false for sparse terms check
                while iter.next(|i| self.iter_next(i)) {
                    internal_each_entity_iter_next::<T, false, false>(&mut iter, &world, &mut func);
                }
            }
        }
    }

    /// Each iterator. This variant of `each` provides access to the [`TableIter`] object,
    /// which contains more information about the object being iterated.
    /// The `usize` argument contains the index of the entity being iterated,
    /// which can be used to obtain entity-specific data from the `TableIter` object.
    ///
    /// When the `flecs_safety_locks` feature is enabled, this function performs runtime checks
    /// for mutable component aliasing and sparse component handling.
    ///
    /// # Example
    /// ```
    /// use flecs_ecs::prelude::*;
    ///
    /// #[derive(Component, Debug)]
    /// struct Position {
    ///     x: i32,
    ///     y: i32,
    /// }
    ///
    /// #[derive(Component, Debug)]
    /// struct Likes;
    ///
    /// let world = World::new();
    ///
    /// let eva = world.entity_named("eva");
    ///
    /// world
    ///     .entity_named("adam")
    ///     .set(Position { x: 10, y: 20 })
    ///     .add((Likes::id(), eva));
    ///
    /// world
    ///     .query::<&Position>()
    ///     .with((Likes::id(), id::<flecs::Wildcard>()))
    ///     .build()
    ///     .each_iter(|it, index, p| {
    ///         let e = it.entity(index);
    ///         println!("{:?}: {:?} - {:?}", e.name(), p, it.id(1).to_str());
    ///     });
    ///
    /// // Output:
    /// //  "adam": Position { x: 10, y: 20 } - "(flecs_ecs.main.Likes,eva)"
    /// ```
    ///
    /// # See also
    ///
    /// * [`QueryAPI::each_iter_unchecked()`] - Unsafe variant without aliasing checks for maximum performance
    fn each_iter(&self, func: impl FnMut(TableIter<false, P>, FieldIndex, T::TupleType<'_>))
    where
        P: ComponentId,
    {
        self.each_iter_internal::<true>(func);
    }

    /// Each iterator without aliasing checks.
    ///
    /// This is an unsafe variant of [`each_iter()`](QueryAPI::each_iter) that skips all mutable
    /// aliasing checks for maximum performance. Use this when you are certain that no component
    /// aliasing can occur.
    ///
    /// When the `flecs_safety_locks` feature is enabled, this function bypasses all runtime
    /// safety checks that [`each_iter()`](QueryAPI::each_iter) would normally perform.
    ///
    /// # Safety
    ///
    /// The caller must ensure that:
    /// - No mutable component references alias with each other
    /// - No mutable references alias with immutable references to the same component
    /// - The query terms don't create situations where the same component is accessed mutably multiple times
    ///
    /// Violating these guarantees will result in undefined behavior.
    ///
    /// # Example
    ///
    /// ```
    /// use flecs_ecs::prelude::*;
    ///
    /// #[derive(Component, Debug)]
    /// struct Position {
    ///     x: i32,
    ///     y: i32,
    /// }
    ///
    /// let world = World::new();
    /// let query = world.new_query::<&mut Position>();
    ///
    /// // Safe: Only one mutable component per entity
    /// unsafe {
    ///     query.each_iter_unchecked(|it, index, pos| {
    ///         let entity = it.entity(index);
    ///         println!("{}: {:?}", entity.name(), pos);
    ///         pos.x += 1;
    ///     });
    /// }
    /// ```
    ///
    /// # See also
    ///
    /// * [`QueryAPI::each_iter()`] - Safe variant with aliasing checks
    unsafe fn each_iter_unchecked(
        &self,
        func: impl FnMut(TableIter<false, P>, FieldIndex, T::TupleType<'_>),
    ) where
        P: ComponentId,
    {
        self.each_iter_internal::<false>(func);
    }

    /// Internal implementation for both checked and unchecked `each_iter` iteration.
    #[inline(always)]
    fn each_iter_internal<const CHECKED: bool>(
        &self,
        mut func: impl FnMut(TableIter<false, P>, FieldIndex, T::TupleType<'_>),
    ) where
        P: ComponentId,
    {
        let world = self.world();
        let mut iter = IterGuard::new(self.retrieve_iter());

        #[cfg(not(feature = "flecs_safety_locks"))]
        {
            while iter.next(|i| self.iter_next(i)) {
                internal_each_iter::<T, P, false, false>(&mut iter, &world, &mut func);
            }
        }
        #[cfg(feature = "flecs_safety_locks")]
        {
            if CHECKED {
                if iter.row_fields == 0 {
                    while iter.next(|i| self.iter_next(i)) {
                        internal_each_iter::<T, P, false, false>(&mut iter, &world, &mut func);
                    }
                } else {
                    while iter.next(|i| self.iter_next(i)) {
                        internal_each_iter::<T, P, false, true>(&mut iter, &world, &mut func);
                    }
                }
            } else {
                // Unchecked: always use false for sparse terms check
                while iter.next(|i| self.iter_next(i)) {
                    internal_each_iter::<T, P, false, false>(&mut iter, &world, &mut func);
                }
            }
        }
    }

    /// find iterator to find an entity
    /// The "find" iterator accepts a function that is invoked for each matching entity and checks if the condition is true.
    /// if it is, it returns that entity.
    /// The following function signatures is valid:
    ///  - func(comp1 : &mut T1, comp2 : &mut T2, ...)
    ///
    /// # Returns
    ///
    /// * `Some(EntityView<'_>)` if the entity was found, `None` if no entity was found.
    fn find(&self, mut func: impl FnMut(T::TupleType<'_>) -> bool) -> Option<EntityView<'a>> {
        unsafe {
            let mut iter = IterGuard::new(self.retrieve_iter());
            let mut entity: Option<EntityView> = None;
            let world_ptr = iter.world;
            let world = WorldRef::from_ptr(world_ptr);

            #[cfg(feature = "flecs_safety_locks")]
            if iter.row_fields == 0 {
                while iter.next(|i| self.iter_next(i)) {
                    __internal_find_impl::<T, false>(
                        &mut func,
                        *iter,
                        &mut entity,
                        world_ptr,
                        &world,
                    );
                }
            } else {
                while iter.next(|i| self.iter_next(i)) {
                    __internal_find_impl::<T, true>(
                        &mut func,
                        *iter,
                        &mut entity,
                        world_ptr,
                        &world,
                    );
                }
            }

            #[cfg(not(feature = "flecs_safety_locks"))]
            {
                while iter.next(|i| self.iter_next(i)) {
                    __internal_find_impl::<T, false>(
                        &mut func,
                        *iter,
                        &mut entity,
                        world_ptr,
                        &world,
                    );
                }
            }
            entity
        }
    }

    /// find iterator to find an entity
    /// The "find" iterator accepts a function that is invoked for each matching entity and checks if the condition is true.
    /// if it is, it returns that entity.
    /// The following function signatures is valid:
    ///  - func(entity : Entity, comp1 : &mut T1, comp2 : &mut T2, ...)
    ///
    /// # Returns
    ///
    /// * `Some(EntityView<'_>)` if the entity was found, `None` if no entity was found.
    fn find_entity(
        &self,
        mut func: impl FnMut(EntityView, T::TupleType<'_>) -> bool,
    ) -> Option<EntityView<'a>> {
        unsafe {
            let mut iter = IterGuard::new(self.retrieve_iter());
            let mut entity_result: Option<EntityView> = None;
            let world_ptr = iter.world;
            let world = WorldRef::from_ptr(world_ptr);

            #[cfg(feature = "flecs_safety_locks")]
            if iter.row_fields == 0 {
                while iter.next(|i| self.iter_next(i)) {
                    __internal_find_entity_impl::<T, false>(
                        &mut func,
                        *iter,
                        &mut entity_result,
                        world_ptr,
                        &world,
                    );
                }
            } else {
                while iter.next(|i| self.iter_next(i)) {
                    __internal_find_entity_impl::<T, true>(
                        &mut func,
                        *iter,
                        &mut entity_result,
                        world_ptr,
                        &world,
                    );
                }
            }

            #[cfg(not(feature = "flecs_safety_locks"))]
            {
                while iter.next(|i| self.iter_next(i)) {
                    __internal_find_entity_impl::<T, false>(
                        &mut func,
                        *iter,
                        &mut entity_result,
                        world_ptr,
                        &world,
                    );
                }
            }
            entity_result
        }
    }

    /// Run iterator.
    ///
    /// The "run" iterator accepts a function that is invoked for each matching
    /// table. The following function signature is valid:
    ///  - func(it: &mut `TableIter`)
    ///
    /// allows for more control over how entities
    /// are iterated as it provides multiple entities in the same callback
    /// and allows to determine what should happen before and past iteration.
    ///
    /// # Example
    ///
    /// ```
    /// use flecs_ecs::prelude::*;
    ///
    /// #[derive(Component, Debug)]
    /// struct Tag;
    ///
    /// #[derive(Debug, Component, Default)]
    /// pub struct Position {
    ///     pub x: i32,
    ///     pub y: i32,
    /// }
    ///
    /// #[derive(Debug, Component, Default)]
    /// pub struct Velocity {
    ///     pub x: i32,
    ///     pub y: i32,
    /// }
    ///
    /// let world = World::new();
    ///
    /// world.entity().add(Tag).add(Position::id());
    /// world.entity().add(Tag).add(Position::id());
    /// world
    ///     .entity()
    ///     .add(Tag)
    ///     .add(Position::id())
    ///     .add(Velocity::id());
    ///
    /// let query = world.new_query::<(&Tag, &Position)>();
    ///
    /// let mut count_tables = 0;
    /// let mut count_entities = 0;
    ///
    /// query.run(|mut it| {
    ///     println!("start operations");
    ///     while it.next() {
    ///         count_tables += 1;
    ///         let pos = it.field::<Position>(1); //at index 1 in (&Tag, &Position)
    ///         for i in it.iter() {
    ///             count_entities += 1;
    ///             let entity = it.get_entity(i).unwrap();
    ///             println!("{:?}: {:?}", entity, pos[i]);
    ///         }
    ///     }
    ///     println!("end operations");
    /// });
    ///
    /// assert_eq!(count_tables, 2);
    /// assert_eq!(count_entities, 3);
    ///
    /// // Output:
    /// //  start operations
    /// //  Entity name:  -- id: 508 -- archetype: flecs_ecs.main.Tag, flecs_ecs.main.Position: Position { x: 0, y: 0 }
    /// //  Entity name:  -- id: 511 -- archetype: flecs_ecs.main.Tag, flecs_ecs.main.Position: Position { x: 0, y: 0 }
    /// //  Entity name:  -- id: 512 -- archetype: flecs_ecs.main.Tag, flecs_ecs.main.Position, flecs_ecs.main.Velocity: Position { x: 0, y: 0 }
    /// //  end operations
    /// ```
    fn run(&self, mut func: impl FnMut(TableIter<true, P>))
    where
        P: ComponentId,
    {
        let mut iter = self.retrieve_iter();
        internal_run::<P>(&mut iter, &mut func, self.world());
    }

    /// Run iterator with each forwarding.
    /// The "iter" iterator accepts a function that is invoked for each matching
    /// table. The following function signature is valid:
    ///  - `func`: (it: &mut `TableIter`) + `func_each`: (comp1 : &mut T1, comp2 : &mut T2, ...)
    ///
    /// allows for more control over how entities
    /// are iterated as it provides multiple entities in the same callback
    /// and allows to determine what should happen before and past iteration.
    ///
    /// # Example
    ///
    /// ```
    /// use flecs_ecs::prelude::*;
    ///
    /// #[derive(Component, Debug)]
    /// struct Tag;
    ///
    /// #[derive(Debug, Component, Default)]
    /// pub struct Position {
    ///     pub x: i32,
    ///     pub y: i32,
    /// }
    ///
    /// #[derive(Debug, Component, Default)]
    /// pub struct Velocity {
    ///     pub x: i32,
    ///     pub y: i32,
    /// }
    ///
    /// let world = World::new();
    ///
    /// world.entity().add(Tag).add(Position::id());
    /// world.entity().add(Tag).add(Position::id());
    /// world
    ///     .entity()
    ///     .add(Tag)
    ///     .add(Position::id())
    ///     .add(Velocity::id());
    ///
    /// let query = world.query::<(&Position)>().with(Tag).build();
    ///
    /// let mut count_tables = 0;
    /// let mut count_entities = 0;
    ///
    /// query.run_each(
    ///     |mut it| {
    ///         println!("start operations");
    ///         while it.next() {
    ///             count_tables += 1;
    ///             it.each();
    ///         }
    ///         println!("end operations");
    ///     },
    ///     |pos| {
    ///         count_entities += 1;
    ///         println!("{:?}", pos);
    ///     },
    /// );
    ///
    /// assert_eq!(count_tables, 2);
    /// assert_eq!(count_entities, 3);
    ///
    /// // Output:
    /// //  start operations
    /// //  Tag, Position { x: 0, y: 0 }
    /// //  Tag, Position { x: 0, y: 0 }
    /// //  Tag, Position { x: 0, y: 0 }
    /// //  end operations
    /// ```
    fn run_each<FuncEach>(&self, mut func: impl FnMut(TableIter<true, P>), mut func_each: FuncEach)
    where
        P: ComponentId,
        FuncEach: FnMut(T::TupleType<'_>),
    {
        let mut iter = self.retrieve_iter();
        iter.callback_ctx = &mut func_each as *mut _ as *mut core::ffi::c_void;
        iter.callback = Some(__internal_query_execute_each_from_run::<T, FuncEach> as ExternIterFn);
        internal_run::<P>(&mut iter, &mut func, self.world());
        #[allow(unused_assignments)]
        {
            iter.callback = None;
            iter.callback_ctx = core::ptr::null_mut();
        }
    }

    /// Run iterator with each entity forwarding.
    ///
    /// The "iter" iterator accepts a function that is invoked for each matching
    /// table. The following function signature is valid:
    /// - `func`: (it: &mut `TableIter`) + `func_each`: (entity: Entity, comp1 : &mut T1, comp2 : &mut T2, ...)
    ///
    /// allows for more control over how entities
    /// are iterated as it provides multiple entities in the same callback
    /// and allows to determine what should happen before and past iteration.
    ///
    /// # Example
    ///
    /// ```
    /// use flecs_ecs::prelude::*;
    ///
    /// #[derive(Component, Debug)]
    /// struct Tag;
    ///
    /// #[derive(Debug, Component, Default)]
    /// pub struct Position {
    ///     pub x: i32,
    ///     pub y: i32,
    /// }
    ///
    /// #[derive(Debug, Component, Default)]
    /// pub struct Velocity {
    ///     pub x: i32,
    ///     pub y: i32,
    /// }
    ///
    /// let world = World::new();
    ///
    /// world.entity().add(Tag).add(Position::id());
    /// world.entity().add(Tag).add(Position::id());
    /// world
    ///     .entity()
    ///     .add(Tag)
    ///     .add(Position::id())
    ///     .add(Velocity::id());
    ///
    /// let query = world.query::<(&Position)>().with(Tag).build();
    ///
    /// let mut count_tables = 0;
    /// let mut count_entities = 0;
    ///
    /// query.run_each_entity(
    ///     |mut it| {
    ///         println!("start operations");
    ///         while it.next() {
    ///             count_tables += 1;
    ///             it.each();
    ///         }
    ///         println!("end operations");
    ///     },
    ///     |e, pos| {
    ///         count_entities += 1;
    ///         println!("{:?} : {:?}", e, pos);
    ///     },
    /// );
    ///
    /// assert_eq!(count_tables, 2);
    /// assert_eq!(count_entities, 3);
    ///
    /// // Output:
    /// //  start operations
    /// //  Entity name:  -- id: 508 -- archetype: flecs_ecs.main.Tag, flecs_ecs.main.Position : Position { x: 0, y: 0 }
    /// //  Entity name:  -- id: 511 -- archetype: flecs_ecs.main.Tag, flecs_ecs.main.Position : Position { x: 0, y: 0 }
    /// //  Entity name:  -- id: 512 -- archetype: flecs_ecs.main.Tag, flecs_ecs.main.Position, flecs_ecs.main.Velocity : Position { x: 0, y: 0 }
    /// //  end operations
    /// ```
    fn run_each_entity<FuncEachEntity>(
        &self,
        mut func: impl FnMut(TableIter<true, P>),
        mut func_each: FuncEachEntity,
    ) where
        P: ComponentId,
        FuncEachEntity: FnMut(EntityView, T::TupleType<'_>),
    {
        let mut iter = self.retrieve_iter();
        iter.callback_ctx = &mut func_each as *mut _ as *mut core::ffi::c_void;
        iter.callback = Some(
            __internal_query_execute_each_entity_from_run::<T, FuncEachEntity> as ExternIterFn,
        );
        let world = self.world();
        let mut iter_t = unsafe { TableIter::new(&mut iter, world) };
        iter_t.iter_mut().flags &= !sys::EcsIterIsValid;
        func(iter_t);
        #[allow(unused_assignments)]
        {
            iter.callback = None;
            iter.callback_ctx = core::ptr::null_mut();
        }
    }

    /// Each iterator. This variant of `each` provides access to the [`TableIter`] object,
    /// which contains more information about the object being iterated.
    /// The `usize` argument contains the index of the entity being iterated,
    /// which can be used to obtain entity-specific data from the `TableIter` object.
    ///
    /// # Example
    /// ```
    /// use flecs_ecs::prelude::*;
    ///
    /// #[derive(Component, Debug)]
    /// struct Position {
    ///     x: i32,
    ///     y: i32,
    /// }
    ///
    /// #[derive(Component, Debug)]
    /// struct Likes;
    ///
    /// let world = World::new();
    ///
    /// let eva = world.entity_named("eva");
    ///
    /// world
    ///     .entity_named("adam")
    ///     .set(Position { x: 10, y: 20 })
    ///     .add((Likes::id(), eva));
    ///
    /// world
    ///     .query::<&Position>()
    ///     .with((Likes::id(), id::<flecs::Wildcard>()))
    ///     .build()
    ///     .each_iter(|it, index, p| {
    ///         let e = it.entity(index);
    ///         println!("{:?}: {:?} - {:?}", e.name(), p, it.id(1).to_str());
    ///     });
    ///
    /// // Output:
    /// //  "adam": Position { x: 10, y: 20 } - "(flecs_ecs.main.Likes,eva)"
    /// ```
    fn run_each_iter<FuncEachIter>(
        &self,
        mut func: impl FnMut(TableIter<true, P>),
        mut func_each: FuncEachIter,
    ) where
        P: ComponentId,
        FuncEachIter: FnMut(TableIter<false, P>, FieldIndex, T::TupleType<'_>),
    {
        let mut iter = self.retrieve_iter();
        iter.callback_ctx = &mut func_each as *mut _ as *mut core::ffi::c_void;
        iter.callback =
            Some(__internal_query_execute_each_iter_from_run::<T, P, FuncEachIter> as ExternIterFn);
        let world = self.world();
        let mut iter_t = unsafe { TableIter::new(&mut iter, world) };
        iter_t.iter_mut().flags &= !sys::EcsIterIsValid;
        func(iter_t);
        #[allow(unused_assignments)]
        {
            iter.callback = None;
            iter.callback_ctx = core::ptr::null_mut();
        }
    }

    /// Get the entity of the current query
    ///
    /// # Arguments
    ///
    /// * `query`: the query to get the entity from
    ///
    /// # Returns
    ///
    /// The entity of the current query
    fn entity(&self) -> EntityView<'_>;

    /// Each term iterator.
    /// The `each_term` iterator accepts a function that is invoked for each term
    /// in the query. The following function signature is valid:
    ///  - func(term: &mut Term)
    fn each_term(&self, mut func: impl FnMut(&TermRef)) {
        let query = self.query_ptr();
        ecs_assert!(
            !query.is_null(),
            FlecsErrorCode::InvalidParameter,
            "query filter is null"
        );
        let query = unsafe { &*query };
        for i in 0..query.term_count {
            let term = TermRef::new(unsafe { &*(query.terms.add(i as usize) as *const _) });
            func(&term);
        }
    }

    /// Get a immutable reference of the term of the current query at the given index
    /// This is mostly used for debugging purposes.
    ///
    /// # Arguments
    ///
    /// * `index`: the index of the term to get
    /// * `query`: the query to get the term from
    ///
    /// # Returns
    ///
    /// The term requested
    fn term(&self, index: usize) -> TermRef<'_> {
        let query = self.query_ptr();
        ecs_assert!(
            !query.is_null(),
            FlecsErrorCode::InvalidParameter,
            "query filter is null"
        );
        let query = unsafe { &*query };
        TermRef::new(unsafe { &*(query.terms.add(index) as *const _) })
    }

    /// Get the field count of the current query
    ///
    /// # Arguments
    ///
    /// * `query`: the query to get the field count from
    ///
    /// # Returns
    ///
    /// The field count of the current query
    fn field_count(&self) -> i8 {
        let query = self.query_ptr();
        unsafe { (*query).field_count }
    }

    /// Get the count of terms set of the current query
    fn term_count(&self) -> u32 {
        let query = self.query_ptr();
        unsafe { (*query).term_count as u32 }
    }

    fn find_var(&self, name: &str) -> Option<i32> {
        let name = compact_str::format_compact!("{}\0", name);

        let var_index =
            unsafe { sys::ecs_query_find_var(self.query_ptr(), name.as_ptr() as *const _) };
        if var_index == -1 {
            None
        } else {
            Some(var_index)
        }
    }

    fn plan(&self) -> String {
        let query = self.query_ptr();
        let result: *mut c_char = unsafe { sys::ecs_query_plan(query as *const _) };

        if result.is_null() {
            return String::new();
        }

        // Safe now: non-null and assumed NUL-terminated by the C API.
        let plan = unsafe { core::ffi::CStr::from_ptr(result) }
            .to_string_lossy() // avoid panic on invalid UTF-8
            .into_owned();

        unsafe {
            if let Some(free_func) = sys::ecs_os_api.free_ {
                free_func(result as *mut core::ffi::c_void);
            }
        }

        plan
    }

    fn iterable(&self) -> QueryIter<'_, P, T> {
        QueryIter::new(self.retrieve_iter(), self.iter_next_func())
    }

    fn iter_stage(&'a self, stage: impl WorldProvider<'a>) -> QueryIter<'a, P, T> {
        QueryIter::new(self.retrieve_iter_stage(stage), self.iter_next_func())
    }

    /// Return a paged iterator.
    ///
    /// Paged iterators limit the results to those starting at `offset`, returning at most `limit` results.
    ///
    /// # Arguments
    ///
    /// * `offset` - Number of entities to skip.
    /// * `limit`  - Maximum number of entities to return.
    fn page(&'a self, offset: i32, limit: i32) -> ChainedIter<'a, P, T> {
        self.iterable().page(offset, limit)
    }

    /// Return a worker iterator.
    ///
    /// Worker iterators evenly distribute matched entities across N workers.
    ///
    /// # Arguments
    ///
    /// * `index` - Zero-based index of this worker (0..count-1).
    /// * `count` - Total number of workers.
    fn worker(&'a self, index: i32, count: i32) -> ChainedIter<'a, P, T> {
        self.iterable().worker(index, count)
    }

    /// Return first matching entity.
    ///
    /// # Returns
    ///
    /// * `Some(EntityView<'_>)` if the entity was found, `None` if no entity was found.
    ///
    /// # See also
    ///
    /// ```rust
    /// use flecs_ecs::prelude::*;
    ///
    /// #[derive(Component, Debug)]
    /// struct Position {
    ///     x: i32,
    ///     y: i32,
    /// }
    ///
    /// let world = World::new();
    ///
    /// let query = world.query::<&Position>().build();
    ///
    /// let entity = query.try_first_entity();
    ///
    /// assert!(entity.is_none());
    ///
    /// let ent = world.entity().set(Position { x: 10, y: 20 });
    ///
    /// let entity = query.try_first_entity();
    ///
    /// assert_eq!(entity.unwrap(), ent);
    /// ```
    ///
    /// # See also
    ///
    /// * [`Query::try_first_entity`]
    /// * [`Query::try_first`]
    /// * [`Query::first`]
    /// * [`Query::try_first_only`]
    /// * [`Query::first_only`]
    fn try_first_entity(&self) -> Option<EntityView<'a>> {
        let mut it = IterGuard::new(self.retrieve_iter());

        if it.next(|i| self.iter_next(i)) {
            let ent = if it.count > 0 {
                Some(EntityView::new_from(self.world(), unsafe {
                    *it.entities.add(0)
                }))
            } else {
                None
            };
            it.fini();
            ent
        } else {
            None
        }
    }

    /// Return first matching entity.
    ///
    /// # Returns
    ///
    /// The first entity in the iterator.
    ///
    /// # Panics
    ///
    /// if there are no entities.
    ///
    /// # See also
    ///
    /// ```rust
    /// use flecs_ecs::prelude::*;
    ///
    /// #[derive(Component, Debug)]
    /// struct Position {
    ///     x: i32,
    ///     y: i32,
    /// }
    ///
    /// let world = World::new();
    ///
    /// let query = world.query::<&Position>().build();
    ///
    /// let ent = world.entity().set(Position { x: 10, y: 20 });
    ///
    /// let entity = query.first_entity();
    ///
    /// assert_eq!(entity, ent);
    /// ```
    ///
    /// # See also
    ///
    /// * [`Query::try_first_entity`]
    /// * [`Query::try_first`]
    /// * [`Query::first`]
    /// * [`Query::try_first_only`]
    /// * [`Query::first_only`]
    fn first_entity(&self) -> EntityView<'a> {
        self.try_first_entity()
            .expect("Expected at least one entity, but none were found.")
    }

    /// iterates over the first entity in the iterator and returns a user-defined result.
    ///
    /// # Returns
    ///
    /// * `Some(result)` if there is at least one entity, otherwise returns `None`.
    ///
    /// ```rust
    /// use flecs_ecs::prelude::*;
    ///
    /// #[derive(Component, Debug)]
    /// struct Position {
    ///     x: i32,
    ///     y: i32,
    /// }
    ///
    /// let world = World::new();
    /// let query = world.new_query::<&Position>();
    ///
    /// assert_eq!(query.try_first(|pos| pos.x), None);
    ///
    /// world.entity().set(Position { x: 10, y: 20 });
    /// world.entity().set(Position { x: 40, y: 20 });
    ///
    /// let x_pos = query.try_first(|pos| pos.x);
    ///
    /// assert_eq!(x_pos, Some(10));
    /// ```
    ///
    /// # See also
    ///
    /// * [`Query::try_first_entity`]
    /// * [`Query::first_entity`]
    /// * [`Query::first`]
    /// * [`Query::try_first_only`]
    /// * [`Query::first_only`]
    fn try_first<R>(&self, func: impl FnOnce(T::TupleType<'_>) -> R) -> Option<R> {
        let mut it = IterGuard::new(self.retrieve_iter());

        #[cfg(feature = "flecs_safety_locks")]
        {
            let world = self.world();
            if it.row_fields == 0 {
                // Proceed only if there is at least one entity in the iterator
                if it.next(|i| self.iter_next(i)) && it.count > 0 {
                    return __internal_try_first_impl::<T, R, false>(func, it, world);
                }
                return None;
            }
            if it.next(|i| self.iter_next(i)) && it.count > 0 {
                // Proceed only if there is at least one entity in the iterator
                return __internal_try_first_impl::<T, R, true>(func, it, world);
            }
            None
        }

        #[cfg(not(feature = "flecs_safety_locks"))]
        {
            // Proceed only if there is at least one entity in the iterator
            if it.next(|i| self.iter_next(i)) && it.count > 0 {
                return __internal_try_first_impl::<T, R, false>(func, it);
            }
            None
        }
    }

    /// iterates over the first entity in the iterator and returns a user-defined result.
    ///
    /// # Returns
    ///
    /// The result of the callback function.
    ///
    /// # Panics
    ///
    /// if there are no entities.
    ///
    /// ```rust
    /// use flecs_ecs::prelude::*;
    ///
    /// #[derive(Component, Debug)]
    /// struct Position {
    ///     x: i32,
    ///     y: i32,
    /// }
    ///
    /// let world = World::new();
    ///
    /// let query = world.new_query::<&Position>();
    ///
    /// world.entity().set(Position { x: 10, y: 20 });
    ///
    /// let x_pos = query.first(|pos| pos.x);
    ///
    /// assert_eq!(x_pos, 10);
    /// ```
    ///
    /// # See also
    ///
    /// * [`Query::try_first_entity`]
    /// * [`Query::first_entity`]
    /// * [`Query::try_first`]
    /// * [`Query::try_first_only`]
    /// * [`Query::first_only`]
    fn first<R>(&self, func: impl FnOnce(T::TupleType<'_>) -> R) -> R {
        self.try_first(func)
            .expect("Expected at least one entity, but none were found.")
    }

    /// Iterates over the first entity in the iterator and returns a user-defined result.
    ///
    /// # Returns
    ///
    /// * `Ok(result)` if there is exactly one entity.
    /// * `Err(FirstOnlyError)` if there are no entities or more than one entity.
    ///
    /// ```rust
    /// use flecs_ecs::prelude::*;
    ///
    /// #[derive(Component, Debug)]
    /// struct Position {
    ///     x: i32,
    ///     y: i32,
    /// }
    ///
    /// let world = World::new();
    ///
    /// let query = world.new_query::<&Position>();
    ///
    /// assert_eq!(
    ///     query.try_first_only(|pos| pos.x),
    ///     Err(FirstOnlyError::NoEntities)
    /// );
    ///
    /// world.entity().set(Position { x: 10, y: 20 });
    ///
    /// let x_pos = query.try_first_only(|pos| pos.x);
    ///
    /// assert_eq!(x_pos, Ok(10));
    ///
    /// world.entity().set(Position { x: 40, y: 20 });
    ///
    /// let x_pos = query.try_first_only(|pos| pos.x);
    ///
    /// assert_eq!(x_pos, Err(FirstOnlyError::MoreThanOneEntity));
    /// ```
    ///
    /// # See also
    ///
    /// * [`Query::try_first_entity`]
    /// * [`Query::first_entity`]
    /// * [`Query::try_first`]
    /// * [`Query::first`]
    fn try_first_only<R>(
        &self,
        func: impl FnOnce(T::TupleType<'_>) -> R,
    ) -> Result<R, FirstOnlyError> {
        let mut it = IterGuard::new(self.retrieve_iter());

        #[cfg(feature = "flecs_safety_locks")]
        {
            let world = self.world();

            if it.row_fields == 0 {
                // Proceed only if we can iterate
                if it.next(|i| self.iter_next(i)) {
                    return __internal_try_first_only_impl::<T, R, false>(func, it, world);
                }
                // No entities in the iterator
                return Err(FirstOnlyError::NoEntities);
            }
            if it.next(|i| self.iter_next(i)) {
                // Proceed only if we can iterate
                return __internal_try_first_only_impl::<T, R, true>(func, it, world);
            }
            // No entities in the iterator
            Err(FirstOnlyError::NoEntities)
        }

        #[cfg(not(feature = "flecs_safety_locks"))]
        // Proceed only if we can iterate
        if it.next(|i| self.iter_next(i)) {
            __internal_try_first_only_impl::<T, R, false>(func, it)
        } else {
            // No entities in the iterator
            Err(FirstOnlyError::NoEntities)
        }
    }

    /// iterates over the first entity in the iterator and returns a user-defined result.
    ///
    /// # Returns
    ///
    /// The result of the callback function.
    ///
    /// # Panics
    ///  
    /// if there are no entities, or if there are more than one entity.
    ///
    /// ```rust
    /// use flecs_ecs::prelude::*;
    ///
    /// #[derive(Component, Debug)]
    /// struct Position {
    ///     x: i32,
    ///     y: i32,
    /// }
    ///
    /// let world = World::new();
    ///
    /// let query = world.new_query::<&Position>();
    ///
    /// world.entity().set(Position { x: 10, y: 20 });
    ///
    /// let x_pos = query.first_only(|pos| pos.x);
    ///
    /// assert_eq!(x_pos, 10);
    /// ```
    ///
    /// # See also
    ///
    /// * [`Query::try_first_entity`]
    /// * [`Query::first_entity`]
    /// * [`Query::try_first`]
    /// * [`Query::first`]
    /// * [`Query::try_first_only`]
    fn first_only<R>(&self, func: impl FnOnce(T::TupleType<'_>) -> R) -> R {
        match self.try_first_only(func) {
            Ok(result) => result,
            Err(FirstOnlyError::MoreThanOneEntity) => {
                panic!("Expected exactly one entity, but found more than one.");
            }
            Err(FirstOnlyError::NoEntities) => {
                panic!("Expected one entity, but none were found.");
            }
        }
    }

    /// Returns true if iterator yields at least once result.
    fn is_true(&self) -> bool {
        let mut it = IterGuard::new(self.retrieve_iter());

        let result = it.next(|i| self.iter_next(i));
        if result {
            it.fini();
        }
        result
    }

    /// Return total number of entities in result.
    ///
    /// # Returns
    ///
    /// The total number of entities in the result
    fn count(&self) -> i32 {
        let mut it = IterGuard::new(self.retrieve_iter());
        let mut result = 0;
        while it.next(|i| self.iter_next(i)) {
            result += it.count;
        }
        result
    }

    /// Returns a new iterator limited to tables with the specified group id (grouped queries only).
    ///
    /// Does not modify `self`. To constrain an existing [`QueryIter`] in place,
    /// use [`QueryIter::set_group`].
    ///
    /// # Arguments
    ///
    /// * `group_id`: the group id to set
    #[must_use = "returns a new iterator; the original is unchanged. Use `QueryIter::set_group` to modify in place"]
    fn with_group(&self, group_id: impl IntoEntity) -> QueryIter<'_, P, T> {
        let mut iter = self.iterable();
        QueryIter::<P, T>::set_group(&mut iter, group_id);
        iter
    }

    /// Returns a new iterator with the query variable set to the given entity.
    ///
    /// Does not modify `self`. To constrain an existing [`QueryIter`] in place,
    /// use [`QueryIter::set_var`].
    ///
    /// # Arguments
    ///
    /// * `var_id`: the variable id to set
    ///
    /// * `value`: the value to set
    #[must_use = "returns a new iterator; the original is unchanged. Use `QueryIter::set_var` to modify in place"]
    fn with_var(&self, var_id: i32, value: impl Into<Entity>) -> QueryIter<'_, P, T> {
        let mut iter = self.iterable();
        QueryIter::<P, T>::set_var(&mut iter, var_id, value);
        iter
    }

    /// Returns a new iterator with the query variable set to the given table range.
    ///
    /// Does not modify `self`. To constrain an existing [`QueryIter`] in place,
    /// use [`QueryIter::set_var_table`].
    ///
    /// # Arguments
    ///
    /// * `var_id`: the variable id to set
    ///
    /// * `range`: the range to set
    #[must_use = "returns a new iterator; the original is unchanged. Use `QueryIter::set_var_table` to modify in place"]
    fn with_var_table(&self, var_id: i32, table: impl IntoTableRange) -> QueryIter<'_, P, T> {
        let mut iter = self.iterable();
        QueryIter::<P, T>::set_var_table(&mut iter, var_id, table);
        iter
    }

    /// Returns a new iterator with the named query variable set to the given entity.
    ///
    /// Does not modify `self`. To constrain an existing [`QueryIter`] in place,
    /// use [`QueryIter::set_var_expr`].
    ///
    /// # Arguments
    ///
    /// * `name`: the name of the variable to set
    /// * `value`: the value to set
    #[must_use = "returns a new iterator; the original is unchanged. Use `QueryIter::set_var_expr` to modify in place"]
    fn with_var_expr(&self, name: &str, value: impl Into<Entity>) -> QueryIter<'_, P, T> {
        let mut iter = self.iterable();
        QueryIter::<P, T>::set_var_expr(&mut iter, name, value);
        iter
    }

    /// Returns a new iterator with the named query variable set to the given table range.
    ///
    /// Does not modify `self`. To constrain an existing [`QueryIter`] in place,
    /// use [`QueryIter::set_var_table_expr`].
    ///
    /// # Arguments
    ///
    /// * `name`: the name of the variable to set
    /// * `range`: the range to set
    #[must_use = "returns a new iterator; the original is unchanged. Use `QueryIter::set_var_table_expr` to modify in place"]
    fn with_var_table_expr(&self, name: &str, table: impl IntoTableRange) -> QueryIter<'_, P, T> {
        let mut iter = self.iterable();
        QueryIter::<P, T>::set_var_table_expr(&mut iter, name, table);
        iter
    }

    /// Serialize iterator result to JSON.
    #[cfg(feature = "flecs_json")]
    fn to_json(&self, desc: Option<&crate::prelude::json::IterToJsonDesc>) -> Option<String> {
        let desc_ptr = desc
            .map(|d| d as *const crate::prelude::json::IterToJsonDesc)
            .unwrap_or(core::ptr::null());

        let mut iter = self.retrieve_iter();

        unsafe {
            let json_ptr = sys::ecs_iter_to_json(&mut iter, desc_ptr);
            if json_ptr.is_null() {
                return None;
            }
            let json = core::ffi::CStr::from_ptr(json_ptr)
                .to_string_lossy()
                .into_owned();
            sys::ecs_os_api.free_.expect("os api is missing")(json_ptr as *mut core::ffi::c_void);
            Some(json)
        }
    }

    fn cache_query(&self) -> Option<Query<()>> {
        let query = self.query_ptr();
        unsafe {
            let cache_query = sys::ecs_query_get_cache_query(query);
            if cache_query.is_null() {
                None
            } else {
                Some(Query::new_from(core::ptr::NonNull::new_unchecked(
                    cache_query as *mut _,
                )))
            }
        }
    }
}

#[inline(always)]
fn __internal_try_first_only_impl<T, R, const ANY_SPARSE_TERMS: bool>(
    func: impl FnOnce(T::TupleType<'_>) -> R,
    mut it: IterGuard,
    #[cfg(feature = "flecs_safety_locks")] world: WorldRef<'_>,
) -> Result<R, FirstOnlyError>
where
    T: QueryTuple,
{
    if it.count == 1 {
        let (is_any_array, mut components_data) = T::create_ptrs(&it);

        let result = {
            #[cfg(feature = "flecs_safety_locks")]
            let (__locks, __lock_token) = acquire_batch_locks::<ANY_SPARSE_TERMS, T>(
                &world,
                components_data.safety_table_records(),
            );

            let tuple = if !is_any_array.a_row && !is_any_array.a_ref {
                components_data.get_tuple(0)
            } else if is_any_array.a_row {
                components_data.get_tuple_with_row(&it, 0)
            } else {
                components_data.get_tuple_with_ref(0)
            };

            let ret = func(tuple);

            #[cfg(feature = "flecs_safety_locks")]
            release_batch_locks::<ANY_SPARSE_TERMS, T>(
                &world,
                __locks,
                __lock_token,
                components_data.safety_table_records(),
            );
            ret
        };

        // Clean up iterator resources safely
        it.fini();

        Ok(result)
    } else {
        // More than one entity
        it.fini();
        Err(FirstOnlyError::MoreThanOneEntity)
    }
}

#[inline(always)]
fn __internal_try_first_impl<T, R, const ANY_SPARSE_TERMS: bool>(
    func: impl FnOnce(T::TupleType<'_>) -> R,
    mut it: IterGuard,
    #[cfg(feature = "flecs_safety_locks")] world: WorldRef<'_>,
) -> Option<R>
where
    T: QueryTuple,
{
    let (is_any_array, mut components_data) = T::create_ptrs(&it);

    let result = {
        #[cfg(feature = "flecs_safety_locks")]
        let (__locks, __lock_token) = acquire_batch_locks::<ANY_SPARSE_TERMS, T>(
            &world,
            components_data.safety_table_records(),
        );

        let tuple = if !is_any_array.a_row && !is_any_array.a_ref {
            components_data.get_tuple(0)
        } else if is_any_array.a_row {
            components_data.get_tuple_with_row(&it, 0)
        } else {
            components_data.get_tuple_with_ref(0)
        };

        let ret = Some(func(tuple));

        #[cfg(feature = "flecs_safety_locks")]
        release_batch_locks::<ANY_SPARSE_TERMS, T>(
            &world,
            __locks,
            __lock_token,
            components_data.safety_table_records(),
        );
        ret
    };

    // Clean up iterator resources safely
    it.fini();

    result
}

#[inline(always)]
fn __internal_find_entity_impl<'a, T, const ANY_SPARSE_TERMS: bool>(
    func: &mut impl FnMut(EntityView<'a>, T::TupleType<'_>) -> bool,
    iter: flecs_ecs_sys::ecs_iter_t,
    entity_result: &mut Option<EntityView<'a>>,
    _world_ptr: *mut flecs_ecs_sys::ecs_world_t,
    world: &WorldRef<'a>,
) where
    T: QueryTuple,
{
    let (is_any_array, mut components_data) = T::create_ptrs(&iter);
    let iter_count = iter.count as usize;

    #[cfg(feature = "flecs_safety_locks")]
    let (__locks, __lock_token) = acquire_batch_locks::<ANY_SPARSE_TERMS, T>(
        world,
        components_data.safety_table_records(),
    );

    #[cfg(any(debug_assertions, feature = "flecs_force_enable_ecs_asserts"))]
    let _table_lock = IterTableLock::new(_world_ptr, iter.table);

    unsafe {
        if !is_any_array.a_ref && !is_any_array.a_row {
            for i in 0..iter_count {
                let entity = EntityView::new_from(world, *iter.entities.add(i));

                let tuple = components_data.get_tuple(i);
                if func(entity, tuple) {
                    *entity_result = Some(entity);
                    break;
                }
            }
        } else if is_any_array.a_row {
            for i in 0..iter_count {
                let entity = EntityView::new_from(world, *iter.entities.add(i));
                let tuple = components_data.get_tuple_with_row(&iter, i);
                if func(entity, tuple) {
                    *entity_result = Some(entity);
                    break;
                }
            }
        } else {
            // is_any_array.a_ref
            for i in 0..iter_count {
                let entity = EntityView::new_from(world, *iter.entities.add(i));
                let tuple = components_data.get_tuple_with_ref(i);
                if func(entity, tuple) {
                    *entity_result = Some(entity);
                    break;
                }
            }
        }
    }

    #[cfg(feature = "flecs_safety_locks")]
    release_batch_locks::<ANY_SPARSE_TERMS, T>(
        world,
        __locks,
        __lock_token,
        components_data.safety_table_records(),
    );
}

#[inline(always)]
fn __internal_find_impl<'a, T, const ANY_SPARSE_TERMS: bool>(
    func: &mut impl FnMut(T::TupleType<'_>) -> bool,
    iter: flecs_ecs_sys::ecs_iter_t,
    entity: &mut Option<EntityView<'a>>,
    _world_ptr: *mut flecs_ecs_sys::ecs_world_t,
    world: &WorldRef<'a>,
) where
    T: QueryTuple,
{
    let (is_any_array, mut components_data) = T::create_ptrs(&iter);
    let iter_count = iter.count as usize;

    #[cfg(feature = "flecs_safety_locks")]
    let (__locks, __lock_token) = acquire_batch_locks::<ANY_SPARSE_TERMS, T>(
        world,
        components_data.safety_table_records(),
    );

    #[cfg(any(debug_assertions, feature = "flecs_force_enable_ecs_asserts"))]
    let _table_lock = IterTableLock::new(_world_ptr, iter.table);

    // SAFETY: i ranges over 0..iter_count, and iter.entities has iter.count valid entries.
    unsafe {
        if !is_any_array.a_ref && !is_any_array.a_row {
            for i in 0..iter_count {
                let tuple = components_data.get_tuple(i);
                if func(tuple) {
                    *entity = Some(EntityView::new_from(world, *iter.entities.add(i)));
                    break;
                }
            }
        } else if is_any_array.a_row {
            for i in 0..iter_count {
                let tuple = components_data.get_tuple_with_row(&iter, i);
                if func(tuple) {
                    *entity = Some(EntityView::new_from(world, *iter.entities.add(i)));
                    break;
                }
            }
        } else {
            for i in 0..iter_count {
                let tuple = components_data.get_tuple_with_ref(i);
                if func(tuple) {
                    *entity = Some(EntityView::new_from(world, *iter.entities.add(i)));
                    break;
                }
            }
        }
    }

    #[cfg(feature = "flecs_safety_locks")]
    release_batch_locks::<ANY_SPARSE_TERMS, T>(
        world,
        __locks,
        __lock_token,
        components_data.safety_table_records(),
    );
}

#[inline(always)]
#[extern_abi]
fn __internal_query_execute_each_from_run<T, Func>(iter: *mut sys::ecs_iter_t)
where
    T: QueryTuple,
    Func: FnMut(T::TupleType<'_>),
{
    // SAFETY: iter is the non-null iterator pointer C passes to the run callback.
    let iter = unsafe { &mut *iter };
    // SAFETY: callback_ctx was set to &mut func_each of type Func before this callback ran.
    let func = unsafe { &mut *(iter.callback_ctx as *mut Func) };
    // SAFETY: iter.world is the live world pointer supplied by the C iterator.
    let world = unsafe { WorldRef::from_ptr(iter.world) };

    #[cfg(feature = "flecs_safety_locks")]
    {
        if iter.row_fields == 0 {
            internal_each_iter_next::<T, true, false>(iter, &world, func);
        } else {
            internal_each_iter_next::<T, true, true>(iter, &world, func);
        }
    }

    #[cfg(not(feature = "flecs_safety_locks"))]
    {
        internal_each_iter_next::<T, true, false>(iter, &world, func);
    }
}

#[inline(always)]
#[extern_abi]
unsafe fn __internal_query_execute_each_entity_from_run<T, Func>(iter: *mut sys::ecs_iter_t)
where
    T: QueryTuple,
    Func: FnMut(EntityView, T::TupleType<'_>),
{
    // SAFETY: iter is the non-null iterator pointer C passes to the run callback; callback_ctx
    // was set to &mut func_each before this callback was invoked.
    unsafe {
        let iter = &mut *iter;
        let func = &mut *(iter.callback_ctx as *mut Func);
        let world = WorldRef::from_ptr(iter.world);

        #[cfg(feature = "flecs_safety_locks")]
        {
            if iter.row_fields == 0 {
                internal_each_entity_iter_next::<T, true, false>(iter, &world, func);
            } else {
                internal_each_entity_iter_next::<T, true, true>(iter, &world, func);
            }
        }

        #[cfg(not(feature = "flecs_safety_locks"))]
        {
            internal_each_entity_iter_next::<T, true, false>(iter, &world, func);
        }
    }
}

#[inline(always)]
#[extern_abi]
fn __internal_query_execute_each_iter_from_run<T, P, Func>(iter: *mut sys::ecs_iter_t)
where
    T: QueryTuple,
    P: ComponentId,
    Func: FnMut(TableIter<false, P>, FieldIndex, T::TupleType<'_>),
{
    // SAFETY: iter is the non-null iterator pointer C passes to the run callback; callback_ctx
    // was set to &mut func_each before this callback was invoked.
    unsafe {
        let iter = &mut *iter;
        let func = &mut *(iter.callback_ctx as *mut Func);
        let world = WorldRef::from_ptr(iter.world);

        #[cfg(not(feature = "flecs_safety_locks"))]
        {
            internal_each_iter::<T, P, false, false>(iter, &world, func);
        }
        #[cfg(feature = "flecs_safety_locks")]
        {
            if iter.row_fields == 0 {
                internal_each_iter::<T, P, false, false>(iter, &world, func);
            } else {
                internal_each_iter::<T, P, false, true>(iter, &world, func);
            }
        }
    }
}

/// This struct holds indices for terms that are variable.
/// e.g. wildcard pairs, which need to be determined on a per table iteration
/// set to 320 bytes, 5 full cache lines, hopefully logically distributed.
#[derive(Default)]
#[allow(dead_code)] // possibly used in future
struct ReadWriteCachedInstructions {
    // we expect more reads than writes, hence the 20 to 8 split
    read_ids: smallvec::SmallVec<[u64; 20]>,
    write_ids: smallvec::SmallVec<[u64; 8]>,
    //u8 to cache the variable indices because we never expect there to be more than 256 terms.
    variable_reads: smallvec::SmallVec<[u8; 10]>,
    variable_writes: smallvec::SmallVec<[u8; 10]>,
}

// TODO might be able to optimize this by only iterating the terms? Find the value that determines if the term
// idea is not final
/// This function is to cache which ids are meant for read and write operations
/// and which ids need to be re-fetched on a per table iteration basis.
fn _determine_ids_plus_indices_for_wildcard_terms(
    iter: &sys::ecs_iter_t,
) -> ReadWriteCachedInstructions {
    // SAFETY: iter.query is a valid pointer to the live query that produced this iterator.
    let terms = unsafe { (*iter.query).terms };
    // SAFETY: iter.query is a valid pointer to the live query that produced this iterator.
    let terms_count = unsafe { (*iter.query).term_count };
    // SAFETY: C guarantees iter.ids has exactly term_count entries for this query's iterator.
    let ids = unsafe { core::slice::from_raw_parts(iter.ids, terms_count as usize) };

    let mut read_write = ReadWriteCachedInstructions::default();

    for (i, id) in ids.iter().enumerate().take(terms_count as usize) {
        // SAFETY: i is bounded by terms_count, matching the terms array length from the query.
        let term = unsafe { &*terms.add(i) };
        if *id == 0 {
            match term.inout as sys::ecs_inout_kind_t {
                sys::ecs_inout_kind_t_EcsIn => {
                    read_write.variable_reads.push(i as u8);
                }
                sys::ecs_inout_kind_t_EcsInOut | sys::ecs_inout_kind_t_EcsOut => {
                    read_write.variable_writes.push(i as u8);
                }
                _ => {}
            }

            continue;
        }

        match term.inout as sys::ecs_inout_kind_t {
            sys::ecs_inout_kind_t_EcsIn => {
                read_write.read_ids.push(term.id);
            }
            sys::ecs_inout_kind_t_EcsInOut | sys::ecs_inout_kind_t_EcsOut => {
                read_write.write_ids.push(term.id);
            }
            _ => {}
        }
    }
    read_write
}
