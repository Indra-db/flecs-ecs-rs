use core::ffi::c_void;

use crate::core::*;

#[cfg(feature = "std")]
extern crate std;

extern crate alloc;
use alloc::boxed::Box;

/// Reclaim a binding context that was leaked into a system/observer descriptor
/// when the underlying `ecs_*_init` call failed.
///
/// On a failed init (for example an invalid query expression) flecs returns `0`
/// before taking ownership of the descriptor's binding contexts, so their
/// registered free trampolines are never invoked and the leaked closure boxes
/// would leak. Calling this with the descriptor's `ctx`/`ctx_free` pair on
/// failure runs the same trampoline flecs would have run, reclaiming the box.
///
/// # Safety
///
/// Must only be called when flecs did not take ownership of `ctx` (i.e. the
/// init returned `0`); `ctx_free` must be the free function registered for
/// `ctx`.
pub(crate) unsafe fn reclaim_leaked_binding_ctx(
    ctx: *mut c_void,
    ctx_free: flecs_ecs_sys::ecs_ctx_free_t,
) {
    if let Some(free) = ctx_free
        && !ctx.is_null()
    {
        unsafe { free(ctx) };
    }
}

pub trait SystemAPI<'a, P, T>: Builder<'a> + private::internal_SystemAPI<'a, P, T>
where
    T: QueryTuple,
    P: ComponentId,
{
    /// Set context
    fn set_context(&mut self, context: *mut c_void) -> &mut Self;

    /// Each iterator for systems.
    ///
    /// The "each" iterator accepts a function that is invoked for each matching entity.
    /// The function signature is: `func(comp1: &mut T1, comp2: &mut T2, ...)`
    ///
    /// When the `flecs_safety_locks` feature is enabled, this function performs runtime checks
    /// for mutable component aliasing and sparse component handling.
    ///
    /// # See also
    ///
    /// * [`SystemAPI::each_unchecked()`] - Unsafe variant without aliasing checks for maximum performance
    fn each<Func>(&mut self, func: Func) -> <Self as builder::Builder<'a>>::BuiltType
    where
        Func: FnMut(T::TupleType<'_>) + 'static,
    {
        self.each_internal::<Func, true>(func)
    }

    /// Each iterator for systems without aliasing checks.
    ///
    /// This is an unsafe variant of [`each()`](SystemAPI::each) that skips all mutable aliasing
    /// checks for maximum performance. Use this when you are certain that no component aliasing
    /// can occur.
    ///
    /// When the `flecs_safety_locks` feature is enabled, this function bypasses all runtime
    /// safety checks that [`each()`](SystemAPI::each) would normally perform.
    ///
    /// # Safety
    ///
    /// The caller must ensure that:
    /// - No mutable component references alias with each other
    /// - No mutable references alias with immutable references to the same component
    /// - The system's query terms don't create situations where the same component is accessed mutably multiple times
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
    ///
    /// // Safe: Position and Velocity are different components
    /// unsafe {
    ///     world
    ///         .system::<(&mut Position, &Velocity)>()
    ///         .each_unchecked(|(pos, vel)| {
    ///             pos.x += vel.x;
    ///             pos.y += vel.y;
    ///         });
    /// }
    /// ```
    ///
    /// # See also
    ///
    /// * [`SystemAPI::each()`] - Safe variant with aliasing checks
    unsafe fn each_unchecked<Func>(
        &mut self,
        func: Func,
    ) -> <Self as builder::Builder<'a>>::BuiltType
    where
        Func: FnMut(T::TupleType<'_>) + 'static,
    {
        self.each_internal::<Func, false>(func)
    }

    /// Internal implementation for both checked and unchecked each iteration.
    fn each_internal<Func, const CHECKED: bool>(
        &mut self,
        func: Func,
    ) -> <Self as builder::Builder<'a>>::BuiltType
    where
        Func: FnMut(T::TupleType<'_>) + 'static,
    {
        const {
            assert!(
                !T::CONTAINS_ANY_TAG_TERM,
                "a type provided in the query signature is a Tag and cannot be used with `.each`. use `.run` instead or provide the tag with `.with()`"
            );
        }

        // Faster version of each() that iterates the query on the Rust side instead of C side.
        let each_func = Box::new(func);
        let each_static_ref = Box::leak(each_func);

        self.set_run_binding_context(each_static_ref as *mut _ as *mut c_void);
        self.set_run_binding_context_free(Some(Self::free_callback::<Func>));

        if CHECKED {
            self.set_desc_run(Some(Self::execute_run_each::<Func, true> as ExternIterFn));
        } else {
            self.set_desc_run(Some(Self::execute_run_each::<Func, false> as ExternIterFn));
        }

        self.build()
    }

    /// Each entity iterator for systems.
    ///
    /// The `each_entity` iterator accepts a function that is invoked for each matching entity.
    /// The function signature is: `func(entity: EntityView, comp1: &mut T1, comp2: &mut T2, ...)`
    ///
    /// When the `flecs_safety_locks` feature is enabled, this function performs runtime checks
    /// for mutable component aliasing and sparse component handling.
    ///
    /// # See also
    ///
    /// * [`SystemAPI::each_entity_unchecked()`] - Unsafe variant without aliasing checks for maximum performance
    fn each_entity<Func>(&mut self, func: Func) -> <Self as builder::Builder<'a>>::BuiltType
    where
        Func: FnMut(EntityView, T::TupleType<'_>) + 'static,
    {
        self.each_entity_internal::<Func, true>(func)
    }

    /// Each entity iterator for systems without aliasing checks.
    ///
    /// This is an unsafe variant of [`each_entity()`](SystemAPI::each_entity) that skips all
    /// mutable aliasing checks for maximum performance. Use this when you are certain that no
    /// component aliasing can occur.
    ///
    /// When the `flecs_safety_locks` feature is enabled, this function bypasses all runtime
    /// safety checks that [`each_entity()`](SystemAPI::each_entity) would normally perform.
    ///
    /// # Safety
    ///
    /// The caller must ensure that:
    /// - No mutable component references alias with each other
    /// - No mutable references alias with immutable references to the same component
    /// - The system's query terms don't create situations where the same component is accessed mutably multiple times
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
    ///
    /// // Safe: Only one mutable component per entity
    /// unsafe {
    ///     world
    ///         .system::<&mut Position>()
    ///         .each_entity_unchecked(|entity, pos| {
    ///             println!("{}: ({}, {})", entity.name(), pos.x, pos.y);
    ///             pos.x += 1.0;
    ///         });
    /// }
    /// ```
    ///
    /// # See also
    ///
    /// * [`SystemAPI::each_entity()`] - Safe variant with aliasing checks
    unsafe fn each_entity_unchecked<Func>(
        &mut self,
        func: Func,
    ) -> <Self as builder::Builder<'a>>::BuiltType
    where
        Func: FnMut(EntityView, T::TupleType<'_>) + 'static,
    {
        self.each_entity_internal::<Func, false>(func)
    }

    /// Internal implementation for both checked and unchecked `each_entity` iteration.
    fn each_entity_internal<Func, const CHECKED: bool>(
        &mut self,
        func: Func,
    ) -> <Self as builder::Builder<'a>>::BuiltType
    where
        Func: FnMut(EntityView, T::TupleType<'_>) + 'static,
    {
        const {
            assert!(
                !T::CONTAINS_ANY_TAG_TERM,
                "a type provided in the query signature is a Tag and cannot be used with `.each`. use `.run` instead or provide the tag with `.with()`"
            );
        }

        let each_func = Box::new(func);
        let each_static_ref = Box::leak(each_func);

        self.set_run_binding_context(each_static_ref as *mut _ as *mut c_void);
        self.set_run_binding_context_free(Some(Self::free_callback::<Func>));

        if CHECKED {
            self.set_desc_run(Some(
                Self::execute_run_each_entity::<Func, true> as ExternIterFn,
            ));
        } else {
            self.set_desc_run(Some(
                Self::execute_run_each_entity::<Func, false> as ExternIterFn,
            ));
        }

        self.build()
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
    ///     .system::<&Position>()
    ///     .with((Likes::id(), id::<flecs::Wildcard>()))
    ///     .each_iter(|it, index, p| {
    ///         let e = it.entity(index);
    ///         println!("{:?}: {:?} - {:?}", e.name(), p, it.id(1).to_str());
    ///     })
    ///     .run();
    ///
    /// // Output:
    /// //  "adam": Position2 { x: 10, y: 20 } - "(flecs_ecs.main.Likes,eva)"
    /// ```
    ///
    /// When the `flecs_safety_locks` feature is enabled, this function performs runtime checks
    /// for mutable component aliasing and sparse component handling.
    ///
    /// # See also
    ///
    /// * [`SystemAPI::each_iter_unchecked()`] - Unsafe variant without aliasing checks for maximum performance
    fn each_iter<Func>(&mut self, func: Func) -> <Self as builder::Builder<'a>>::BuiltType
    where
        Func: FnMut(TableIter<false, P>, FieldIndex, T::TupleType<'_>) + 'static,
    {
        self.each_iter_internal::<Func, true>(func)
    }

    /// Unchecked version of `each_iter` that bypasses mutable aliasing checks.
    ///
    /// When the `flecs_safety_locks` feature is enabled, this function bypasses all runtime
    /// safety checks that [`each_iter()`](SystemAPI::each_iter) would normally perform.
    ///
    /// # Safety
    ///
    /// This function bypasses the runtime mutable aliasing checks when `flecs_safety_locks` feature is enabled.
    /// The caller must ensure that:
    /// - No two components in the query signature alias the same type mutably
    /// - Component access patterns don't violate Rust's aliasing rules
    ///
    /// **When to use:**
    /// - Performance-critical hot paths where aliasing is guaranteed safe by design
    /// - After profiling shows significant overhead from safety checks
    /// - In production code where query signatures are well-tested and known safe
    ///
    /// **Performance benefit:**
    /// - Zero runtime cost from aliasing detection logic
    /// - More efficient sparse component handling
    /// - Optimal code generation without conditional safety branches
    ///
    /// # Examples
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
    ///
    /// // Safe: no aliasing possible with single component
    /// unsafe {
    ///     world
    ///         .system::<&mut Position>()
    ///         .each_iter_unchecked(|it, index, pos| {
    ///             pos.x += 1.0;
    ///         });
    /// }
    /// ```
    unsafe fn each_iter_unchecked<Func>(
        &mut self,
        func: Func,
    ) -> <Self as builder::Builder<'a>>::BuiltType
    where
        Func: FnMut(TableIter<false, P>, FieldIndex, T::TupleType<'_>) + 'static,
    {
        self.each_iter_internal::<Func, false>(func)
    }

    /// Internal implementation for `each_iter` and `each_iter_unchecked`.
    ///
    /// Uses const generic `CHECKED` parameter to control safety checking at compile time.
    /// When `CHECKED = true`, performs aliasing checks (if `flecs_safety_locks` enabled).
    /// When `CHECKED = false`, bypasses all safety checks for maximum performance.
    #[inline]
    fn each_iter_internal<Func, const CHECKED: bool>(
        &mut self,
        func: Func,
    ) -> <Self as builder::Builder<'a>>::BuiltType
    where
        Func: FnMut(TableIter<false, P>, FieldIndex, T::TupleType<'_>) + 'static,
    {
        const {
            assert!(
                !T::CONTAINS_ANY_TAG_TERM,
                "a type provided in the query signature is a Tag and cannot be used with `.each`. use `.run` instead or provide the tag with `.with()`"
            );
        }

        let each_iter_func = Box::new(func);
        let each_iter_static_ref = Box::leak(each_iter_func);

        self.set_run_binding_context(each_iter_static_ref as *mut _ as *mut c_void);
        self.set_run_binding_context_free(Some(Self::free_callback::<Func>));

        self.set_desc_run(Some(
            Self::execute_run_each_iter::<Func, CHECKED> as ExternIterFn,
        ));

        self.build()
    }

    /// Run iterator. This operation expects manual iteration over the tables with `iter.next()` and `iter.iter()`.
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
    /// use std::{rc::Rc, cell::RefCell};
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
    /// let count_entities = Rc::new(RefCell::new(0));
    /// let count_tables = Rc::new(RefCell::new(0));
    /// // Clone the `Rc` to retain shared ownership and move it into the closure to satisfy the 'static lifetime requirement.
    /// let count_entities_ref = count_entities.clone();
    /// let count_tables_ref = count_tables.clone();
    ///
    /// let system = world.system::<(&Tag, &Position)>().run(move |mut it| {
    ///     println!("start operations");
    ///     while it.next() {
    ///         *count_tables_ref.borrow_mut() += 1;
    ///         let pos = it.field::<Position>(1); //at index 1 in (&Tag, &Position)
    ///         for i in it.iter() {
    ///             *count_entities_ref.borrow_mut() += 1;
    ///             let entity = it.get_entity(i).unwrap();
    ///             println!("{:?}: {:?}", entity, pos[i]);
    ///         }
    ///     }
    ///     println!("end operations");
    /// });
    ///
    /// system.run();
    ///
    /// assert_eq!(*count_tables.borrow(), 2);
    /// assert_eq!(*count_entities.borrow(), 3);
    ///
    /// // Output:
    /// //  start operations
    /// //  Entity name:  -- id: 508 -- archetype: flecs_ecs.main.Tag, flecs_ecs.main.Position: Position { x: 0, y: 0 }
    /// //  Entity name:  -- id: 511 -- archetype: flecs_ecs.main.Tag, flecs_ecs.main.Position: Position { x: 0, y: 0 }
    /// //  Entity name:  -- id: 512 -- archetype: flecs_ecs.main.Tag, flecs_ecs.main.Position, flecs_ecs.main.Velocity: Position { x: 0, y: 0 }
    /// //  end operations
    /// ```
    fn run<Func>(&mut self, func: Func) -> <Self as builder::Builder<'a>>::BuiltType
    where
        Func: FnMut(TableIter<true, P>) + 'static,
    {
        let run = Box::new(func);
        let run_static_ref = Box::leak(run);

        self.set_run_binding_context(run_static_ref as *mut _ as *mut c_void);
        self.set_run_binding_context_free(Some(Self::free_callback::<Func>));

        self.set_desc_run(Some(Self::execute_run::<Func> as ExternIterFn));
        self.build()
    }

    /// Run iterator with each forwarding. This operation expects manual
    /// iteration over the tables with `iter.next()` and `iter.each()`
    ///
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
    /// use std::{rc::Rc, cell::RefCell};
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
    /// let count_entities = Rc::new(RefCell::new(0));
    /// let count_tables = Rc::new(RefCell::new(0));
    /// // Clone the `Rc` to retain shared ownership and move it into the closure to satisfy the 'static lifetime requirement.
    /// let count_entities_ref = count_entities.clone();
    /// let count_tables_ref = count_tables.clone();
    ///
    /// let system = world.system::<(&Position)>().with(Tag)
    /// .run_each(
    ///     move |mut it| {
    ///         println!("start operations");
    ///         while it.next() {
    ///             *count_tables_ref.borrow_mut() += 1;
    ///             it.each();
    ///         }
    ///         println!("end operations");
    ///     },
    ///     move |pos| {
    ///         *count_entities_ref.borrow_mut() += 1;
    ///         println!("{:?}", pos);
    ///     },
    /// );
    ///
    /// system.run();
    ///
    /// assert_eq!(*count_tables.borrow(), 2);
    /// assert_eq!(*count_entities.borrow(), 3);
    ///
    /// // Output:
    /// //  start operations
    /// //  Position { x: 0, y: 0 }
    /// //  Position { x: 0, y: 0 }
    /// //  Position { x: 0, y: 0 }
    /// //  end operations
    /// ```
    fn run_each<Func, FuncEach>(
        &mut self,
        func: Func,
        func_each: FuncEach,
    ) -> <Self as builder::Builder<'a>>::BuiltType
    where
        Func: FnMut(TableIter<true, P>) + 'static,
        FuncEach: FnMut(T::TupleType<'_>) + 'static,
    {
        let run_func = Box::new(func);
        let run_static_ref = Box::leak(run_func);

        self.set_run_binding_context(run_static_ref as *mut _ as *mut c_void);
        self.set_run_binding_context_free(Some(Self::free_callback::<Func>));

        self.set_desc_run(Some(Self::execute_run::<Func> as ExternIterFn));

        let each_func = Box::new(func_each);
        let each_static_ref = Box::leak(each_func);

        self.set_callback_binding_context(each_static_ref as *mut _ as *mut c_void);
        self.set_callback_binding_context_free(Some(Self::free_callback::<FuncEach>));

        self.set_desc_callback(Some(Self::execute_each::<true, FuncEach> as ExternIterFn));

        self.build()
    }

    /// Run iterator with each entity forwarding. This operation expects manual
    /// iteration over the tables with `iter.next()` and `iter.each()`  
    ///
    /// The "iter" iterator accepts a function that is invoked for each matching
    /// table. The following function signature is valid:
    /// - `func`: (it: &mut `TableIter`) + `func_each_entity`: (entity: `EntityView`, comp1 : &mut T1, comp2 : &mut T2, ...)
    ///
    /// allows for more control over how entities
    /// are iterated as it provides multiple entities in the same callback
    /// and allows to determine what should happen before and past iteration.
    ///
    /// # Example
    ///
    /// ```
    /// use std::{rc::Rc, cell::RefCell};
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
    /// let count_entities = Rc::new(RefCell::new(0));
    /// let count_tables = Rc::new(RefCell::new(0));
    /// // Clone the `Rc` to retain shared ownership and move it into the closure to satisfy the 'static lifetime requirement.
    /// let count_entities_ref = count_entities.clone();
    /// let count_tables_ref = count_tables.clone();
    ///
    /// let system = world.system::<(&Position)>().with(Tag)
    /// .run_each_entity(
    ///     move |mut it| {
    ///         println!("start operations");
    ///         while it.next() {
    ///             *count_tables_ref.borrow_mut() += 1;
    ///             it.each();
    ///         }
    ///         println!("end operations");
    ///     },
    ///     move |e, pos| {
    ///         *count_entities_ref.borrow_mut() += 1;
    ///         println!("{:?}: {:?}", e, pos);
    ///     },
    /// );
    ///
    /// system.run();
    ///
    /// assert_eq!(*count_tables.borrow(), 2);
    /// assert_eq!(*count_entities.borrow(), 3);
    ///
    /// // Output:
    /// //  start operations
    /// //  Entity name:  -- id: 508 -- archetype: flecs_ecs.main.Position: Position { x: 0, y: 0 }
    /// //  Entity name:  -- id: 511 -- archetype: flecs_ecs.main.Position: Position { x: 0, y: 0 }
    /// //  Entity name:  -- id: 512 -- archetype: flecs_ecs.main.Position, flecs_ecs.main.Velocity: Position { x: 0, y: 0 }
    /// //  end operations
    /// ```
    fn run_each_entity<Func, FuncEachEntity>(
        &mut self,
        func: Func,
        func_each_entity: FuncEachEntity,
    ) -> <Self as builder::Builder<'a>>::BuiltType
    where
        Func: FnMut(TableIter<true, P>) + 'static,
        FuncEachEntity: FnMut(EntityView, T::TupleType<'_>) + 'static,
    {
        let run_func = Box::new(func);
        let run_static_ref = Box::leak(run_func);

        self.set_run_binding_context(run_static_ref as *mut _ as *mut c_void);
        self.set_run_binding_context_free(Some(Self::free_callback::<Func>));

        self.set_desc_run(Some(Self::execute_run::<Func> as ExternIterFn));

        let each_entity_func = Box::new(func_each_entity);
        let each_entity_static_ref = Box::leak(each_entity_func);

        self.set_callback_binding_context(each_entity_static_ref as *mut _ as *mut c_void);
        self.set_callback_binding_context_free(Some(Self::free_callback::<FuncEachEntity>));

        self.set_desc_callback(Some(
            Self::execute_each_entity::<true, FuncEachEntity> as ExternIterFn,
        ));

        self.build()
    }

    fn run_each_iter<Func, FuncEachIter>(
        &mut self,
        func: Func,
        func_each_iter: FuncEachIter,
    ) -> <Self as builder::Builder<'a>>::BuiltType
    where
        Func: FnMut(TableIter<true, P>) + 'static,
        FuncEachIter: FnMut(TableIter<false, P>, FieldIndex, T::TupleType<'_>) + 'static,
    {
        let run_func = Box::new(func);
        let run_static_ref = Box::leak(run_func);

        self.set_run_binding_context(run_static_ref as *mut _ as *mut c_void);
        self.set_run_binding_context_free(Some(Self::free_callback::<Func>));

        self.set_desc_run(Some(Self::execute_run::<Func> as ExternIterFn));

        let each_iter_func = Box::new(func_each_iter);
        let each_iter_static_ref = Box::leak(each_iter_func);

        self.set_callback_binding_context(each_iter_static_ref as *mut _ as *mut c_void);
        self.set_callback_binding_context_free(Some(Self::free_callback::<FuncEachIter>));

        self.set_desc_callback(Some(
            Self::execute_each_iter::<false, FuncEachIter> as ExternIterFn,
        ));

        self.build()
    }

    /// Opt-in world access (spec §5.2): iterate every matched row, handing the
    /// callback the component tuple and a per-callback
    /// [`Stage`](crate::experimental::Stage) through which deferred commands are
    /// issued.
    ///
    /// Enabling stage access makes the system **always register its term locks**
    /// (never the lock-free tiered path): the stage handle can reach storage the
    /// disjointness proof does not cover, so the term borrows must be visible to
    /// any guard taken inside the callback.
    #[cfg(feature = "flecs_experimental")]
    fn each_with<Func>(&mut self, func: Func) -> <Self as builder::Builder<'a>>::BuiltType
    where
        Func: FnMut(T::TupleType<'_>, crate::experimental::Stage<'_>) + 'static,
    {
        const {
            assert!(
                !T::CONTAINS_ANY_TAG_TERM,
                "a type provided in the query signature is a Tag and cannot be used with `.each_with`. use `.run` instead or provide the tag with `.with()`"
            );
        }

        let each_func = Box::new(func);
        let each_static_ref = Box::leak(each_func);

        self.set_run_binding_context(each_static_ref as *mut _ as *mut c_void);
        self.set_run_binding_context_free(Some(Self::free_callback::<Func>));
        self.set_desc_run(Some(Self::execute_run_each_with::<Func> as ExternIterFn));

        self.build()
    }

    /// Entity + item + stage (spec §5.2, GAP-6): as [`each_with`](SystemAPI::each_with)
    /// but the callback also receives each matched [`EntityView`], the "visit each
    /// entity and issue a deferred command on it" shape.
    ///
    /// Like `each_with`, always registers this system's term locks.
    #[cfg(feature = "flecs_experimental")]
    fn each_entity_with<Func>(&mut self, func: Func) -> <Self as builder::Builder<'a>>::BuiltType
    where
        Func: FnMut(EntityView, T::TupleType<'_>, crate::experimental::Stage<'_>) + 'static,
    {
        const {
            assert!(
                !T::CONTAINS_ANY_TAG_TERM,
                "a type provided in the query signature is a Tag and cannot be used with `.each_entity_with`. use `.run` instead or provide the tag with `.with()`"
            );
        }

        let each_func = Box::new(func);
        let each_static_ref = Box::leak(each_func);

        self.set_run_binding_context(each_static_ref as *mut _ as *mut c_void);
        self.set_run_binding_context_free(Some(Self::free_callback::<Func>));
        self.set_desc_run(Some(
            Self::execute_run_each_entity_with::<Func> as ExternIterFn,
        ));

        self.build()
    }
}

pub trait ParSystemAPI<'a, P, T>:
    SystemAPI<'a, P, T> + private::internal_ParSystemAPI<'a, P, T>
where
    T: QueryTuple,
    P: ComponentId,
{
    /// Variant of [`SystemAPI::each`] which allows the system to run in multiple threads.
    ///
    /// When the `flecs_safety_locks` feature is enabled, this function performs runtime checks
    /// for mutable component aliasing and sparse component handling.
    ///
    /// # See also
    ///
    /// * [`ParSystemAPI::par_each_unchecked()`] - Unsafe variant without aliasing checks
    fn par_each<Func>(&mut self, func: Func) -> <Self as builder::Builder<'a>>::BuiltType
    where
        Func: Fn(T::TupleType<'_>) + Send + Sync + 'static,
        for<'w> T::TupleType<'w>: Send,
    {
        self.set_multi_threaded(true);
        self.each(func)
    }

    /// Variant of [`SystemAPI::each_unchecked`] which allows the system to run in multiple threads.
    ///
    /// This is an unsafe variant of [`par_each()`](ParSystemAPI::par_each) that skips all mutable
    /// aliasing checks for maximum performance in parallel execution.
    ///
    /// When the `flecs_safety_locks` feature is enabled, this function bypasses all runtime
    /// safety checks that [`par_each()`](ParSystemAPI::par_each) would normally perform.
    ///
    /// # Safety
    ///
    /// The caller must ensure that:
    /// - No mutable component references alias with each other across parallel executions
    /// - No mutable references alias with immutable references to the same component
    /// - The system's query terms don't create situations where the same component is accessed mutably multiple times
    /// - Thread safety is maintained when running in parallel
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
    ///
    /// // Safe: Position and Velocity are different components
    /// unsafe {
    ///     world
    ///         .system::<(&mut Position, &Velocity)>()
    ///         .par_each_unchecked(|(pos, vel)| {
    ///             pos.x += vel.x;
    ///             pos.y += vel.y;
    ///         });
    /// }
    /// ```
    ///
    /// # See also
    ///
    /// * [`ParSystemAPI::par_each()`] - Safe variant with aliasing checks
    unsafe fn par_each_unchecked<Func>(
        &mut self,
        func: Func,
    ) -> <Self as builder::Builder<'a>>::BuiltType
    where
        Func: Fn(T::TupleType<'_>) + Send + Sync + 'static,
        for<'w> T::TupleType<'w>: Send,
    {
        self.set_multi_threaded(true);
        unsafe { self.each_unchecked(func) }
    }

    /// Variant of [`SystemAPI::each_entity`] which allows the system to run in multiple threads.
    ///
    /// When the `flecs_safety_locks` feature is enabled, this function performs runtime checks
    /// for mutable component aliasing and sparse component handling.
    ///
    /// # See also
    ///
    /// * [`ParSystemAPI::par_each_entity_unchecked()`] - Unsafe variant without aliasing checks
    fn par_each_entity<Func>(&mut self, func: Func) -> <Self as builder::Builder<'a>>::BuiltType
    where
        Func: Fn(EntityView, T::TupleType<'_>) + Send + Sync + 'static,
        for<'w> T::TupleType<'w>: Send,
    {
        self.set_multi_threaded(true);
        self.each_entity(func)
    }

    /// Variant of [`SystemAPI::each_entity_unchecked`] which allows the system to run in multiple threads.
    ///
    /// This is an unsafe variant of [`par_each_entity()`](ParSystemAPI::par_each_entity) that skips
    /// all mutable aliasing checks for maximum performance in parallel execution.
    ///
    /// When the `flecs_safety_locks` feature is enabled, this function bypasses all runtime
    /// safety checks that [`par_each_entity()`](ParSystemAPI::par_each_entity) would normally perform.
    ///
    /// # Safety
    ///
    /// The caller must ensure that:
    /// - No mutable component references alias with each other across parallel executions
    /// - No mutable references alias with immutable references to the same component
    /// - The system's query terms don't create situations where the same component is accessed mutably multiple times
    /// - Thread safety is maintained when running in parallel
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
    ///
    /// // Safe: Only one mutable component per entity
    /// unsafe {
    ///     world
    ///         .system::<&mut Position>()
    ///         .par_each_entity_unchecked(|entity, pos| {
    ///             pos.x += 1.0;
    ///             pos.y += 1.0;
    ///         });
    /// }
    /// ```
    ///
    /// # See also
    ///
    /// * [`ParSystemAPI::par_each_entity()`] - Safe variant with aliasing checks
    unsafe fn par_each_entity_unchecked<Func>(
        &mut self,
        func: Func,
    ) -> <Self as builder::Builder<'a>>::BuiltType
    where
        Func: Fn(EntityView, T::TupleType<'_>) + Send + Sync + 'static,
        for<'w> T::TupleType<'w>: Send,
    {
        self.set_multi_threaded(true);
        unsafe { self.each_entity_unchecked(func) }
    }

    /// Variant of [`SystemAPI::each_iter`] which allows the system to run in multiple threads.
    ///
    /// When the `flecs_safety_locks` feature is enabled, this function performs runtime checks
    /// for mutable component aliasing and sparse component handling.
    ///
    /// # See also
    ///
    /// * [`ParSystemAPI::par_each_iter_unchecked()`] - Unsafe variant without aliasing checks
    fn par_each_iter<Func>(&mut self, func: Func) -> <Self as builder::Builder<'a>>::BuiltType
    where
        Func: FnMut(TableIter<false, P>, FieldIndex, T::TupleType<'_>) + Send + Sync + 'static,
        for<'w> T::TupleType<'w>: Send,
    {
        self.set_multi_threaded(true);
        self.each_iter(func)
    }

    /// Variant of [`SystemAPI::each_iter_unchecked`] which allows the system to run in multiple threads.
    ///
    /// This is an unsafe variant of [`par_each_iter()`](ParSystemAPI::par_each_iter) that skips
    /// all mutable aliasing checks for maximum performance in parallel execution.
    ///
    /// When the `flecs_safety_locks` feature is enabled, this function bypasses all runtime
    /// safety checks that [`par_each_iter()`](ParSystemAPI::par_each_iter) would normally perform.
    ///
    /// # Safety
    ///
    /// The caller must ensure that:
    /// - No mutable component references alias with each other across parallel executions
    /// - No mutable references alias with immutable references to the same component
    /// - The system's query terms don't create situations where the same component is accessed mutably multiple times
    /// - Thread safety is maintained when running in parallel
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
    ///
    /// // Safe: no aliasing possible with single component
    /// unsafe {
    ///     world
    ///         .system::<&mut Position>()
    ///         .par_each_iter_unchecked(|it, index, pos| {
    ///             pos.x += 1.0;
    ///         });
    /// }
    /// ```
    ///
    /// # See also
    ///
    /// * [`ParSystemAPI::par_each_iter()`] - Safe variant with aliasing checks
    unsafe fn par_each_iter_unchecked<Func>(
        &mut self,
        func: Func,
    ) -> <Self as builder::Builder<'a>>::BuiltType
    where
        Func: FnMut(TableIter<false, P>, FieldIndex, T::TupleType<'_>) + Send + Sync + 'static,
        for<'w> T::TupleType<'w>: Send,
    {
        self.set_multi_threaded(true);
        unsafe { self.each_iter_unchecked(func) }
    }

    /// Variant of [`SystemAPI::run`] which allows the system to run in multiple threads
    fn par_run<Func>(&mut self, func: Func) -> <Self as builder::Builder<'a>>::BuiltType
    where
        Func: Fn(TableIter<true, P>) + Send + Sync + 'static,
        for<'w> T::TupleType<'w>: Send,
    {
        self.set_multi_threaded(true);
        self.run(func)
    }

    /// Variant of [`SystemAPI::run_each`] which allows the system to run in multiple threads
    fn par_run_each<Func, FuncEach>(
        &mut self,
        func: Func,
        func_each: FuncEach,
    ) -> <Self as builder::Builder<'a>>::BuiltType
    where
        Func: Fn(TableIter<true, P>) + Send + Sync + 'static,
        FuncEach: Fn(T::TupleType<'_>) + Send + Sync + 'static,
        for<'w> T::TupleType<'w>: Send,
    {
        self.set_multi_threaded(true);
        self.run_each(func, func_each)
    }

    /// Variant of [`SystemAPI::run_each_entity`] which allows the system to run in multiple threads
    fn par_run_each_entity<Func, FuncEachEntity>(
        &mut self,
        func: Func,
        func_each_entity: FuncEachEntity,
    ) -> <Self as builder::Builder<'a>>::BuiltType
    where
        Func: Fn(TableIter<true, P>) + Send + Sync + 'static,
        FuncEachEntity: Fn(EntityView, T::TupleType<'_>) + Send + Sync + 'static,
        for<'w> T::TupleType<'w>: Send,
    {
        self.set_multi_threaded(true);
        self.run_each_entity(func, func_each_entity)
    }

    /// Variant of [`SystemAPI::run_each_iter`] which allows the system to run in multiple threads
    fn par_run_each_iter<Func, FuncEachIter>(
        &mut self,
        func: Func,
        func_each_iter: FuncEachIter,
    ) -> <Self as builder::Builder<'a>>::BuiltType
    where
        Func: Fn(TableIter<true, P>) + Send + Sync + 'static,
        FuncEachIter: Fn(TableIter<false, P>, FieldIndex, T::TupleType<'_>) + Send + Sync + 'static,
        for<'w> T::TupleType<'w>: Send,
    {
        self.set_multi_threaded(true);
        self.run_each_iter(func, func_each_iter)
    }

    /// Multithreaded variant of [`SystemAPI::each_with`] (spec §6.2): flecs
    /// partitions matched tables across workers so each row is visited by exactly
    /// one worker, and each worker's callback gets a [`Stage`](crate::experimental::Stage)
    /// over its own stage for deferred commands.
    ///
    /// The `Item: Send` bound is what makes handing a `&mut T` to another thread
    /// sound (`&T` needs `T: Sync`, `&mut T` needs `T: Send`).
    #[cfg(feature = "flecs_experimental")]
    fn par_each_with<Func>(&mut self, func: Func) -> <Self as builder::Builder<'a>>::BuiltType
    where
        Func: Fn(T::TupleType<'_>, crate::experimental::Stage<'_>) + Send + Sync + 'static,
        for<'w> T::TupleType<'w>: Send,
    {
        self.set_multi_threaded(true);
        self.each_with(func)
    }

    /// Multithreaded variant of [`SystemAPI::each_entity_with`] (spec §6.2).
    #[cfg(feature = "flecs_experimental")]
    fn par_each_entity_with<Func>(
        &mut self,
        func: Func,
    ) -> <Self as builder::Builder<'a>>::BuiltType
    where
        Func: Fn(EntityView, T::TupleType<'_>, crate::experimental::Stage<'_>)
            + Send
            + Sync
            + 'static,
        for<'w> T::TupleType<'w>: Send,
    {
        self.set_multi_threaded(true);
        self.each_entity_with(func)
    }
}

macro_rules! implement_reactor_api {
    ($param:ty, $type:ty) => {
        impl<'a, T> internal_SystemAPI<'a, $param, T> for $type
        where
            T: QueryTuple,
        {
            fn set_callback_binding_context(&mut self, binding_ctx: *mut c_void) -> &mut Self {
                self.desc.callback_ctx = binding_ctx;
                self
            }

            fn set_callback_binding_context_free(
                &mut self,
                binding_ctx_free: flecs_ecs_sys::ecs_ctx_free_t,
            ) -> &mut Self {
                self.desc.callback_ctx_free = binding_ctx_free;
                self
            }

            fn set_run_binding_context(&mut self, binding_ctx: *mut c_void) -> &mut Self {
                self.desc.run_ctx = binding_ctx;
                self
            }

            fn set_run_binding_context_free(
                &mut self,
                run_ctx_free: flecs_ecs_sys::ecs_ctx_free_t,
            ) -> &mut Self {
                self.desc.run_ctx_free = run_ctx_free;
                self
            }

            fn desc_binding_context(&self) -> *mut c_void {
                self.desc.callback_ctx
            }

            fn set_desc_callback(&mut self, callback: Option<crate::core::utility::ExternIterFn>) {
                self.desc.callback = callback;
            }

            fn set_desc_run(&mut self, callback: Option<crate::core::utility::ExternIterFn>) {
                self.desc.run = callback;
            }
        }

        impl<'a, T> SystemAPI<'a, $param, T> for $type
        where
            T: QueryTuple,
        {
            fn set_context(&mut self, context: *mut c_void) -> &mut Self {
                self.desc.ctx = context;
                self
            }
        }
    };
    ($type:ty) => {
        impl<'a, P, T> internal_SystemAPI<'a, P, T> for $type
        where
            T: QueryTuple,
            P: ComponentId,
        {
            fn set_callback_binding_context(&mut self, binding_ctx: *mut c_void) -> &mut Self {
                self.desc.callback_ctx = binding_ctx;
                self
            }

            fn set_callback_binding_context_free(
                &mut self,
                binding_ctx_free: flecs_ecs_sys::ecs_ctx_free_t,
            ) -> &mut Self {
                self.desc.callback_ctx_free = binding_ctx_free;
                self
            }

            fn set_run_binding_context(&mut self, binding_ctx: *mut c_void) -> &mut Self {
                self.desc.run_ctx = binding_ctx;
                self
            }

            fn set_run_binding_context_free(
                &mut self,
                run_ctx_free: flecs_ecs_sys::ecs_ctx_free_t,
            ) -> &mut Self {
                self.desc.run_ctx_free = run_ctx_free;
                self
            }

            fn desc_binding_context(&self) -> *mut c_void {
                self.desc.callback_ctx
            }

            fn set_desc_callback(&mut self, callback: Option<crate::core::utility::ExternIterFn>) {
                self.desc.callback = callback;
            }

            fn set_desc_run(&mut self, callback: Option<crate::core::utility::ExternIterFn>) {
                self.desc.run = callback;
            }
        }

        impl<'a, P, T> SystemAPI<'a, P, T> for $type
        where
            T: QueryTuple,
            P: ComponentId,
        {
            fn set_context(&mut self, context: *mut c_void) -> &mut Self {
                self.desc.ctx = context;
                self
            }
        }
    };
}

#[cfg(feature = "flecs_system")]
macro_rules! implement_reactor_par_api {
    ($param:ty, $type:ty) => {
        impl<'a, T> internal_ParSystemAPI<'a, $param, T> for $type
        where
            T: QueryTuple,
        {
            fn set_multi_threaded(&mut self, multi_threaded: bool) {
                self.desc.multi_threaded = multi_threaded;
            }
        }

        impl<'a, T> ParSystemAPI<'a, $param, T> for $type where T: QueryTuple {}
    };
    ($type:ty) => {
        impl<'a, P, T> internal_ParSystemAPI<'a, P, T> for $type
        where
            T: QueryTuple,
            P: ComponentId,
        {
            fn set_multi_threaded(&mut self, multi_threaded: bool) {
                self.desc.multi_threaded = multi_threaded;
            }
        }

        impl<'a, P, T> ParSystemAPI<'a, P, T> for $type
        where
            T: QueryTuple,
            P: ComponentId,
        {
        }
    };
}

pub(crate) use implement_reactor_api;
#[cfg(feature = "flecs_system")]
pub(crate) use implement_reactor_par_api;
