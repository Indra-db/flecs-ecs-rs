//! Builder for constructing complex queries with additional conditions and configuration.
//!
//!
//! The [`QueryBuilder`] provides a fluent interface for building queries incrementally,
//! allowing you to add terms, configure caching behavior, and apply advanced filtering
//! before creating the final [`Query`] object.
//!
//! > **For comprehensive documentation**, see the [Flecs Query Manual](https://www.flecs.dev/flecs/md_docs_2Queries.html)
//!
//! > **See also**: The `query!` macro from the [DSL module](crate::dsl) provides
//! > a declarative syntax for creating queries that closely mirrors the Flecs Query Language.
//!
//! # Quick Start
//!
//! ```
//! # use flecs_ecs::prelude::*;
//! #
//! # #[derive(Component)]
//! # struct Position { x: f32, y: f32 }
//! #
//! # #[derive(Component)]
//! # struct Velocity { x: f32, y: f32 }
//! #
//! # #[derive(Component)]
//! # struct Enemy;
//! #
//! # #[derive(Component)]
//! # struct Mars;
//! #
//! # #[derive(Component)]
//! # struct Planet;
//! # let world = World::new();
//!
//! // Build a query with additional conditions
//! let query = world
//!     .query::<(&Position, &Velocity)>()
//!     .with(Enemy)
//!     .without((Planet, Mars))
//!     .build();
//!
//! query.each(|(pos, vel)| {
//!     // Only processes enemies with Position and Velocity, but not those who are on Mars
//! });
//! ```
//!
//! # Builder vs Direct Creation
//!
//! There are two ways to create queries:
//!
//! ## Direct Creation (Simple Queries)
//!
//! Use [`World::new_query()`] for simple queries with just component types:
//!
//! ```
//! # use flecs_ecs::prelude::*;
//! # #[derive(Component)] struct Position { x: f32, y: f32 }
//! # #[derive(Component)] struct Velocity { x: f32, y: f32 }
//! # let world = World::new();
//! // Fast and simple - creates uncached query immediately
//! let query = world.new_query::<(&Position, &Velocity)>();
//! ```
//!
//! ## Builder Pattern (Complex Queries)
//!
//! Use [`World::query()`] to get a builder when you need:
//! - Additional filtering with `with()` / `without()`
//! - Relationship conditions
//! - Cache configuration
//! - Ordering or grouping
//! - Variables and query DSL features
//!
//! ```
//! # use flecs_ecs::prelude::*;
//! # #[derive(Component)] struct Position { x: f32, y: f32 }
//! # #[derive(Component)] struct Velocity { x: f32, y: f32 }
//! # #[derive(Component)] struct Enemy;
//! # let world = World::new();
//! let query = world
//!     .query::<(&Position, &Velocity)>()
//!     .with(Enemy)
//!     .set_cache_kind(QueryCacheKind::Auto)
//!     .build();
//! ```
//!
//! # Common Builder Methods
//!
//! ## Adding Terms with `with()`
//!
//! Add components that must be present but aren't needed in the iteration:
//!
//! ```
//! # use flecs_ecs::prelude::*;
//! # #[derive(Component)] struct Position { x: f32, y: f32 }
//! # #[derive(Component)] struct Enemy;
//! # let world = World::new();
//! // Query Position but only for entities with Enemy tag
//! let query = world.query::<&Position>().with(Enemy).build();
//!
//! // Enemy tag is required but not part of iteration
//! query.each(|pos| {
//!     println!("Enemy at ({}, {})", pos.x, pos.y);
//! });
//! ```
//!
//! ## Filtering with `without()`
//!
//! Exclude entities that have certain components:
//!
//! ```
//! # use flecs_ecs::prelude::*;
//! # #[derive(Component)] struct Position { x: f32, y: f32 }
//! # #[derive(Component)] struct Dead;
//! # let world = World::new();
//! // Query Position but exclude dead entities
//! let query = world.query::<&Position>().without(&Dead).build();
//!
//! query.each(|pos| {
//!     println!("Living entity at ({}, {})", pos.x, pos.y);
//! });
//! ```
//!
//! ## Relationship Queries
//!
//! Query entities based on their relationships:
//!
//! ```
//! # use flecs_ecs::prelude::*;
//! # #[derive(Component)] struct Position { x: f32, y: f32 }
//! #[derive(Component)]
//! struct Likes;
//!
//! # let world = World::new();
//! let apples = world.entity_named("Apples");
//!
//! // Query entities that like apples
//! let query = world.query::<&Position>().with((Likes, apples)).build();
//! ```
//!
//! ## Cache Control
//!
//! Configure query caching behavior:
//!
//! ```
//! # use flecs_ecs::prelude::*;
//! # #[derive(Component)] struct Position { x: f32, y: f32 }
//! # let world = World::new();
//! // Explicitly set cache kind
//! let cached = world
//!     .query::<&Position>()
//!     .set_cache_kind(QueryCacheKind::Auto)
//!     .build();
//!
//! let uncached = world
//!     .query::<&Position>()
//!     .set_cache_kind(QueryCacheKind::None)
//!     .build();
//! ```
//!
//! # Advanced Features
//!
//! ## Variables
//!
//! Use query variables to match related entities:
//!
//! ```
//! # use flecs_ecs::prelude::*;
//! #[derive(Component)]
//! struct Eats;
//!
//! #[derive(Component)]
//! struct Healthy;
//!
//! # let world = World::new();
//! // Find entities that eat healthy food
//! let query = world
//!     .query::<()>()
//!     .with((Eats, "$food"))
//!     .with(Healthy)
//!     .set_src("$food")
//!     .build();
//!
//! let food_var = query.find_var("food").unwrap();
//!
//! query.each_iter(|it, _, _| {
//!     println!("Eats {}", it.get_var(food_var).name());
//! });
//! ```
//!
//! ## Optional Components
//!
//! Match components that may or may not be present:
//!
//! ```
//! # use flecs_ecs::prelude::*;
//! # #[derive(Component)] struct Position { x: f32, y: f32 }
//! # #[derive(Component)] struct Health(i32);
//! # let world = World::new();
//! let query = world.query::<(&Position, Option<&Health>)>().build();
//!
//! query.each(|(pos, health)| {
//!     if let Some(health) = health {
//!         println!("Entity at ({}, {}) has {} health", pos.x, pos.y, health.0);
//!     }
//! });
//! ```
//!
//! ## Ordering
//!
//! Sort query results by component values:
//!
//! ```
//! # use flecs_ecs::prelude::*;
//! # #[derive(Component)] struct Position { x: f32, y: f32 }
//! # let world = World::new();
//! let query = world
//!     .query::<&Position>()
//!     .order_by::<Position>(|_e1, p1: &Position, _e2, p2: &Position| {
//!         if p1.x < p2.x {
//!             -1
//!         } else if p1.x > p2.x {
//!             1
//!         } else {
//!             0
//!         }
//!     })
//!     .build();
//!
//! // Entities are now iterated in order of Position.x
//! query.each(|pos| {
//!     println!("Position: ({}, {})", pos.x, pos.y);
//! });
//! ```
//!
//! ## Grouping
//!
//! Group query results by relationship target for better cache locality.
//! This groups entities that share the same relationship target together:
//!
//! ```
//! # use flecs_ecs::prelude::*;
//! # #[derive(Component)] struct Position { x: f32, y: f32 }
//! #[derive(Component)]
//! struct Group;
//!
//! #[derive(Component)]
//! struct Team1;
//!
//! #[derive(Component)]
//! struct Team2;
//!
//! # let world = World::new();
//! // Create a query grouped by the Group relationship
//! let query = world.query::<&Position>().group_by(Group).build();
//!
//! // Create entities with different group targets
//! world
//!     .entity()
//!     .add((Group, Team1))
//!     .set(Position { x: 1.0, y: 1.0 });
//!
//! world
//!     .entity()
//!     .add((Group, Team2))
//!     .set(Position { x: 2.0, y: 2.0 });
//!
//! // Entities with the same (Group, Target) are iterated together
//! query.run(|mut it| {
//!     while it.next() {
//!         let world = it.world();
//!         let group = world.entity_from_id(it.group_id());
//!         println!("Group: {}", group.name());
//!     }
//! });
//! ```
//!
//! # Performance Considerations
//!
//! ## Cached vs Uncached
//!
//! - **Cached queries** ([`QueryCacheKind::Auto`] or [`QueryCacheKind::All`]):
//!   - Faster iteration (very fast)
//!   - Slower creation
//!   - Higher memory usage
//!   - Best for: Queries evaluated every frame (e.g. systems)
//!
//! - **Uncached queries** ([`QueryCacheKind::None`]):
//!   - Faster creation
//!   - Slower iteration
//!   - Minimal memory overhead
//!   - Best for: Ad-hoc queries, one-time lookups
//!
//! # See Also
//!
//! - [`Query`] for the resulting query object
//! - [`World::query()`] to create a query builder
//! - [`World::new_query()`] to create queries directly
//! - [Query module documentation](crate::core::query) for iteration examples
//! - [Flecs Query Manual](https://www.flecs.dev/flecs/md_docs_2Queries.html) for comprehensive documentation
//!
//! [`QueryCacheKind::Auto`]
//! [`QueryCacheKind::All`]
//! [`QueryCacheKind::None`]

use core::ffi::c_void;
use core::mem::ManuallyDrop;

use crate::core::internals::*;
use crate::core::*;
use crate::sys;

#[cfg(feature = "std")]
extern crate std;

extern crate alloc;
use alloc::string::{String, ToString};
use alloc::{format, vec::Vec};
use flecs_ecs_derive::extern_abi;

/// Builder for constructing complex [`Query`] objects.
///
/// `QueryBuilder` provides a fluent interface for incrementally building queries with
/// additional terms, filters, and configuration options. It's created via [`World::query()`]
/// and finalized with [`.build()`](QueryBuilder::build).
///
/// # Basic Usage
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
/// #[derive(Component)]
/// struct Enemy;
///
/// let world = World::new();
///
/// // Build a query with additional filtering
/// let query = world.query::<(&Position, &Velocity)>().with(Enemy).build();
///
/// query.each(|(pos, vel)| {
///     // Only processes enemies with Position and Velocity
/// });
/// ```
///
/// # Common Patterns
///
/// ## Returning from Functions
///
/// When returning queries from functions, use `'static` lifetime for component types
/// to ensure they outlive the function scope:
///
/// ```
/// use flecs_ecs::prelude::*;
///
/// #[derive(Component)]
/// struct Foo(u8);
///
/// fn foo_query(world: &World) -> Query<&'static Foo> {
///     world.query::<&Foo>().build()
/// }
///
/// let world = World::new();
/// let query = foo_query(&world);
/// query.each(|foo| { /* ... */ });
/// ```
///
/// ## Use in Systems
///
/// Queries can be stored and reused within systems:
///
/// ```
/// use flecs_ecs::prelude::*;
///
/// #[derive(Component)]
/// struct Foo(u8);
///
/// fn plugin(world: &World) {
///     let foos = world.query::<&Foo>().build();
///
///     world.system::<()>().each(move |_| {
///         foos.each(|foo| {
///             // Reuse query each frame
///         });
///     });
/// }
/// ```
///
/// # Builder Methods
///
/// The builder provides many methods for configuring queries. See the
/// [module documentation](crate::core::query_builder) for comprehensive examples of:
///
/// - [`with()`](QueryBuilder::with) / [`without()`](QueryBuilder::without) - Add/exclude components
/// - [`set_cache_kind()`](QueryBuilder::set_cache_kind) - Control caching behavior
/// - [`order_by()`](QueryBuilder::order_by) - Sort results
/// - [`group_by()`](QueryBuilder::group_by) - Group results
///
/// # Single-use terminal
///
/// [`build()`](Builder::build) consumes the builder by value, so building twice
/// or configuring a builder after it has been built is a compile error rather
/// than a silent logic bug.
///
/// Building the same builder twice does not compile:
///
/// ```compile_fail
/// use flecs_ecs::prelude::*;
/// let world = World::new();
/// let builder = world.query::<()>();
/// let _first = builder.build();
/// let _second = builder.build(); // error: `builder` was already consumed
/// ```
///
/// Configuring a builder after building it does not compile:
///
/// ```compile_fail
/// use flecs_ecs::prelude::*;
/// #[derive(Component)]
/// struct Position;
/// let world = World::new();
/// let builder = world.query::<()>();
/// let _query = builder.build();
/// let _late = builder.with(Position::id()); // error: `builder` was already consumed
/// ```
///
/// # See Also
///
/// - [Module documentation](crate::core::query_builder) for detailed examples
/// - [`Query`] for the resulting query object
/// - [`World::query()`] to create a builder
/// - [`World::new_query()`] for simple queries without a builder
pub struct QueryBuilder<'a, T>
where
    T: QueryTuple,
{
    pub(crate) desc: sys::ecs_query_desc_t,
    pub(crate) term_builder: TermBuilder,
    world: WorldRef<'a>,
    _phantom: core::marker::PhantomData<T>,
}

impl<T: QueryTuple> core::fmt::Debug for QueryBuilder<'_, T> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("QueryBuilder")
            .field("terms", &debug_term_list(&self.desc.terms))
            .finish()
    }
}

bitflags::bitflags! {
    #[derive(Copy, Clone, Debug, PartialEq, Eq)]
    pub struct QueryFlags: u32 {
        const MatchPrefab = sys::EcsQueryMatchPrefab;
        const MatchDisabled = sys::EcsQueryMatchDisabled;
        const MatchEmptyTables = sys::EcsQueryMatchEmptyTables;
        const AllowUnresolvedByName = sys::EcsQueryAllowUnresolvedByName;
        const TableOnly = sys::EcsQueryTableOnly;
        const DetectChanges = sys::EcsQueryDetectChanges;
        const GroupByOrdered = sys::EcsQueryGroupByOrdered;
        const GroupByDesc = sys::EcsQueryGroupByDesc;
    }
}

bitflags::bitflags! {
    #[derive(Copy, Clone, Debug, PartialEq, Eq)]
    pub struct ObserverFlags: u32 {
        const BypassQuery = sys::EcsObserverBypassQuery;
        const YieldOnCreate = sys::EcsObserverYieldOnCreate;
        const YieldOnDelete = sys::EcsObserverYieldOnDelete;
    }
}

impl<'a, T> QueryBuilder<'a, T>
where
    T: QueryTuple,
{
    /// Create a new query builder
    ///
    /// # Arguments
    ///
    /// * `world` - The world to create the observer in
    pub fn new(world: &'a World) -> Self {
        let mut obj = Self {
            desc: Default::default(),
            world: world.world(),
            term_builder: Default::default(),
            _phantom: core::marker::PhantomData,
        };

        T::populate(&mut obj);

        obj
    }

    /// Create a new query builder with a name
    ///
    /// # Arguments
    ///
    /// * `world` - The world to create the observer in
    /// * `name` - The name of the observer
    pub fn new_named(world: &'a World, name: &str) -> Self {
        let name = compact_str::format_compact!("{}\0", name);

        let desc = Default::default();

        let mut obj = Self {
            desc,
            term_builder: Default::default(),
            world: world.world(),
            _phantom: core::marker::PhantomData,
        };

        let entity_desc = sys::ecs_entity_desc_t {
            name: name.as_ptr() as *const _,
            sep: SEPARATOR.as_ptr(),
            root_sep: SEPARATOR.as_ptr(),
            ..Default::default()
        };

        obj.desc.entity = unsafe { sys::ecs_entity_init(world.world_ptr_mut(), &entity_desc) };

        T::populate(&mut obj);
        obj
    }

    /// Create a new query builder from an existing descriptor
    ///
    /// # Arguments
    ///
    /// * `world` - The world to create the observer in
    /// * `desc` - The descriptor to create the observer from
    #[allow(dead_code)] // this may be used in the future
    pub(crate) fn new_from_desc(
        world: impl WorldProvider<'a>,
        desc: &mut sys::ecs_query_desc_t,
    ) -> Self {
        Self {
            desc: *desc,
            term_builder: Default::default(),
            world: world.world(),
            _phantom: core::marker::PhantomData,
        }
    }

    /// Create a new query builder from an existing descriptor with a term index
    ///
    /// # Arguments
    ///
    /// * `world` - The world to create the observer in
    /// * `desc` - The descriptor to create the observer from
    /// * `term_index` - The index of the term to create the observer from
    #[expect(dead_code, reason = "possibly used in the future")]
    pub(crate) fn new_from_desc_term_index(
        world: &'a World,
        desc: &mut sys::ecs_query_desc_t,
        term_index: i32,
    ) -> Self {
        let mut obj = Self {
            desc: *desc,
            term_builder: TermBuilder {
                current_term_index: term_index,
                next_term_index: term_index,
                expr_count: 0,
                term_ref_mode: TermRefMode::Src,
                str_ptrs_to_free: Vec::new(),
            },
            world: world.world(),
            _phantom: core::marker::PhantomData,
        };

        T::populate(&mut obj);
        obj
    }
}

#[doc(hidden)]
impl<'a, T: QueryTuple> internals::QueryConfig<'a> for QueryBuilder<'a, T> {
    #[inline(always)]
    fn term_builder(&self) -> &TermBuilder {
        &self.term_builder
    }

    #[inline(always)]
    fn term_builder_mut(&mut self) -> &mut TermBuilder {
        &mut self.term_builder
    }

    #[inline(always)]
    fn query_desc(&self) -> &sys::ecs_query_desc_t {
        &self.desc
    }

    #[inline(always)]
    fn query_desc_mut(&mut self) -> &mut sys::ecs_query_desc_t {
        &mut self.desc
    }

    #[inline(always)]
    fn count_generic_terms(&self) -> i32 {
        T::COUNT
    }
}

impl<'a, T: QueryTuple> TermBuilderImpl<'a> for QueryBuilder<'a, T> {}

impl<'a, T: QueryTuple> QueryBuilderImpl<'a> for QueryBuilder<'a, T> {}

impl<'a, T: QueryTuple> WorldProvider<'a> for QueryBuilder<'a, T> {
    fn world(&self) -> WorldRef<'a> {
        self.world
    }
}

impl<'a, T> Builder<'a> for QueryBuilder<'a, T>
where
    T: QueryTuple,
{
    type BuiltType = Query<T>;

    /// Build the `query_builder` into an query
    ///
    /// # Returns
    ///
    /// The built query
    ///
    /// # Example
    ///
    /// * how to return a query / query builder from a function see example in [`QueryBuilder`]
    fn build(mut self) -> Self::BuiltType {
        let world = self.world;
        let query = Query::<T>::new_from_desc(world, &mut self.desc);
        for s in self.term_builder.str_ptrs_to_free.iter_mut() {
            unsafe { ManuallyDrop::drop(s) };
        }
        self.term_builder.str_ptrs_to_free.clear();
        query
    }
}

impl<'a, T: QueryTuple> QueryBuilder<'a, T> {
    /// Set a runtime query expression, transitioning to the fallible-build typestate.
    ///
    /// A purely-typed builder is infallible: [`build()`](Builder::build) returns a
    /// [`Query`] directly. A runtime expression string can fail to parse, so calling
    /// `expr()` consumes the builder and returns a [`FallibleQueryBuilder`] whose
    /// terminal [`build()`](FallibleQueryBuilder::build) returns a
    /// `Result<Query<T>, QueryBuildError>`. All remaining configuration methods are
    /// still available on the returned builder.
    ///
    /// # Example
    ///
    /// ```
    /// use flecs_ecs::prelude::*;
    ///
    /// #[derive(Component)]
    /// struct Position { x: f32, y: f32 }
    ///
    /// let world = World::new();
    ///
    /// // Invalid query expression surfaces as an error, not a panic.
    /// let result = world.query::<()>().expr("invalid syntax!!!").build();
    /// assert!(matches!(result, Err(QueryBuildError::InvalidExpr { .. })));
    /// ```
    pub fn expr(self, expr: &str) -> FallibleQueryBuilder<'a, T> {
        let inner = QueryBuilderImpl::expr(self, expr);
        FallibleQueryBuilder {
            inner,
            expr: expr.to_string(),
        }
    }
}

/// A malformed query construction reported by the fallible-build typestate.
///
/// Only a builder that took a runtime [`expr()`](QueryBuilder::expr) (or another
/// runtime/dynamic term input) can produce this error; a purely-typed builder is
/// infallible.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum QueryBuildError {
    /// The runtime `expr()` string failed to parse or validate.
    InvalidExpr {
        /// The offending expression string.
        expr: String,
    },
    /// A dynamically-constructed term was malformed (bad id, conflicting modifiers).
    InvalidTerm {
        /// The index of the offending term.
        index: usize,
    },
    /// `ecs_query_init` rejected the descriptor for another reason.
    Init,
}

impl core::fmt::Display for QueryBuildError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            QueryBuildError::InvalidExpr { expr } => {
                write!(f, "invalid query expression: {expr:?}")
            }
            QueryBuildError::InvalidTerm { index } => {
                write!(f, "invalid query term at index {index}")
            }
            QueryBuildError::Init => write!(f, "query initialization failed"),
        }
    }
}

impl core::error::Error for QueryBuildError {}

/// The fallible-build typestate of [`QueryBuilder`], entered via
/// [`QueryBuilder::expr()`].
///
/// It carries the same configuration surface as [`QueryBuilder`] (the query builder
/// methods delegate to the wrapped builder), but its terminal
/// [`build()`](FallibleQueryBuilder::build) returns a `Result` because the descriptor
/// can be malformed.
pub struct FallibleQueryBuilder<'a, T>
where
    T: QueryTuple,
{
    inner: QueryBuilder<'a, T>,
    expr: String,
}

impl<T: QueryTuple> core::fmt::Debug for FallibleQueryBuilder<'_, T> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("FallibleQueryBuilder")
            .field("terms", &debug_term_list(&self.inner.desc.terms))
            .field("expr", &self.expr)
            .finish()
    }
}

#[doc(hidden)]
impl<'a, T: QueryTuple> internals::QueryConfig<'a> for FallibleQueryBuilder<'a, T> {
    #[inline(always)]
    fn term_builder(&self) -> &TermBuilder {
        self.inner.term_builder()
    }

    #[inline(always)]
    fn term_builder_mut(&mut self) -> &mut TermBuilder {
        self.inner.term_builder_mut()
    }

    #[inline(always)]
    fn query_desc(&self) -> &sys::ecs_query_desc_t {
        self.inner.query_desc()
    }

    #[inline(always)]
    fn query_desc_mut(&mut self) -> &mut sys::ecs_query_desc_t {
        self.inner.query_desc_mut()
    }

    #[inline(always)]
    fn count_generic_terms(&self) -> i32 {
        self.inner.count_generic_terms()
    }
}

impl<'a, T: QueryTuple> TermBuilderImpl<'a> for FallibleQueryBuilder<'a, T> {}

impl<'a, T: QueryTuple> QueryBuilderImpl<'a> for FallibleQueryBuilder<'a, T> {}

impl<'a, T: QueryTuple> WorldProvider<'a> for FallibleQueryBuilder<'a, T> {
    fn world(&self) -> WorldRef<'a> {
        self.inner.world
    }
}

impl<'a, T> Builder<'a> for FallibleQueryBuilder<'a, T>
where
    T: QueryTuple,
{
    type BuiltType = Result<Query<T>, QueryBuildError>;

    /// Build the query, returning [`QueryBuildError`] if the descriptor is malformed.
    fn build(mut self) -> Self::BuiltType {
        let world = self.inner.world;
        let query = Query::<T>::try_new_from_desc(world, &mut self.inner.desc);
        for s in self.inner.term_builder.str_ptrs_to_free.iter_mut() {
            unsafe { ManuallyDrop::drop(s) };
        }
        self.inner.term_builder.str_ptrs_to_free.clear();
        match query {
            Some(query) => Ok(query),
            None => Err(QueryBuildError::InvalidExpr {
                expr: core::mem::take(&mut self.expr),
            }),
        }
    }
}

//this doesn't work because world ptr gets misaligned
// Assuming some imports and definitions from your previous example, and adding the required ones for this example.
// #[cfg(not(target_family = "wasm"))]
// type GroupByFn = extern "C-unwind" fn(
//     *mut sys::ecs_world_t,
//     *mut sys::ecs_table_t,
//     sys::ecs_id_t,
//     *mut c_void,
// ) -> u64;

// #[cfg(target_family = "wasm")]
// type GroupByFn =
//     extern "C" fn(*mut sys::ecs_world_t, *mut sys::ecs_table_t, sys::ecs_id_t, *mut c_void) -> u64;

// Type definitions for OrderBy function pointers
#[cfg(not(target_family = "wasm"))]
type OrderByFnPtr<T> = extern "C-unwind" fn(Entity, &T, Entity, &T) -> i32;
#[cfg(target_family = "wasm")]
type OrderByFnPtr<T> = extern "C" fn(Entity, &T, Entity, &T) -> i32;

#[cfg(not(target_family = "wasm"))]
type OrderByFnPtrUnsafe = unsafe extern "C-unwind" fn(
    u64,
    *const core::ffi::c_void,
    u64,
    *const core::ffi::c_void,
) -> i32;
#[cfg(target_family = "wasm")]
type OrderByFnPtrUnsafe =
    unsafe extern "C" fn(u64, *const core::ffi::c_void, u64, *const core::ffi::c_void) -> i32;

#[cfg(not(target_family = "wasm"))]
type OrderByFnVoidPtr = extern "C-unwind" fn(Entity, *const c_void, Entity, *const c_void) -> i32;
#[cfg(target_family = "wasm")]
type OrderByFnVoidPtr = extern "C" fn(Entity, *const c_void, Entity, *const c_void) -> i32;

#[cfg(not(target_family = "wasm"))]
type OrderByFnVoidPtrUnsafe = unsafe extern "C-unwind" fn(
    u64,
    *const core::ffi::c_void,
    u64,
    *const core::ffi::c_void,
) -> i32;
#[cfg(target_family = "wasm")]
type OrderByFnVoidPtrUnsafe =
    unsafe extern "C" fn(u64, *const core::ffi::c_void, u64, *const core::ffi::c_void) -> i32;

/// Functions to build a query using terms.
pub trait QueryBuilderImpl<'a>: TermBuilderImpl<'a> {
    /// set the name of the query-like object
    fn named(mut self, name: &str) -> Self {
        let name = compact_str::format_compact!("{}\0", name);
        let world_ptr = self.world_ptr_mut();

        let entity_desc: sys::ecs_entity_desc_t = sys::ecs_entity_desc_t {
            name: name.as_ptr() as *const _,
            sep: SEPARATOR.as_ptr(),
            root_sep: SEPARATOR.as_ptr(),
            ..core::default::Default::default()
        };
        let entity_field_ref = &mut self.query_desc_mut().entity;
        if *entity_field_ref != 0 {
            unsafe { sys::ecs_delete(world_ptr, *entity_field_ref) };
        }

        *entity_field_ref = unsafe { sys::ecs_entity_init(world_ptr, &entity_desc) };
        self
    }

    /// Associate this query with an existing entity (allows `query_from` retrieval).
    /// Equivalent to C++ `query_builder(entity)` constructor.
    ///
    /// # Arguments
    ///
    /// * `entity` - The entity to bind the query to.
    fn entity_set(mut self, entity: impl Into<Entity>) -> Self {
        self.query_desc_mut().entity = *entity.into();
        self
    }

    /// set querylags
    ///
    /// # Arguments
    ///
    /// * `flags` - the flags to set
    fn query_flags(mut self, flags: QueryFlags) -> Self {
        self.query_desc_mut().flags |= flags.bits();
        self
    }

    /// Set what cache method to use for the query
    ///
    /// # Arguments
    ///
    /// * `kind` - the cache kind to set
    fn set_cache_kind(mut self, kind: QueryCacheKind) -> Self {
        self.query_desc_mut().cache_kind = kind as sys::ecs_query_cache_kind_t;
        self
    }

    /// Set the cache method to cached
    fn set_cached(self) -> Self {
        self.set_cache_kind(QueryCacheKind::Auto)
    }

    fn detect_changes(mut self) -> Self {
        self.query_desc_mut().flags |= sys::EcsQueryDetectChanges;
        self
    }

    /// set expression
    ///
    /// # Arguments
    ///
    /// * `expr` - the expression to set
    fn expr(mut self, expr: &str) -> Self {
        let expr = ManuallyDrop::new(format!("{expr}\0"));
        ecs_assert!(
            *self.expr_count_mut() == 0,
            FlecsErrorCode::InvalidOperation,
            "query_builder::expr() called more than once"
        );

        self.query_desc_mut().expr = expr.as_ptr() as *const _;
        *self.expr_count_mut() += 1;
        self.term_builder_mut().str_ptrs_to_free.push(expr);
        self
    }

    fn with<'s, T>(mut self, id: T) -> Self
    where
        Access<'s>: FromAccessArg<T>,
    {
        let access = <Access<'s> as FromAccessArg<T>>::from_access_arg(id, self.world());
        self = self.term();

        match access.target {
            AccessTarget::Entity(entity) => {
                self.init_current_term(entity);
            }
            AccessTarget::Pair(rel, target) => {
                self.init_current_term(ecs_pair(*rel, *target));
            }
            AccessTarget::Name(name) => {
                self = self.set_first::<&str>(name);
            }
            AccessTarget::PairName(rel, target) => {
                self = self.set_first::<&str>(rel).set_second::<&str>(target);
            }
            AccessTarget::PairEntityName(rel, target) => {
                self.init_current_term(rel);
                self = self.set_second::<&str>(target);
            }
            AccessTarget::PairNameEntity(rel, target) => {
                self = self.set_first::<&str>(rel);
                self = self.set_second::<Entity>(target);
            }
        }

        match access.mode {
            AccessMode::Read => {
                // Set In for explicit read access in builder API (mirrors C++ with<const T>())
                self.current_term_mut().inout = InOutKind::In as i16;
            }
            AccessMode::ReadWrite => {
                self.current_term_mut().inout = InOutKind::InOut as i16;
            }
            AccessMode::Write => {
                self.current_term_mut().inout = InOutKind::Out as i16;
            }
            _ => {}
        }
        self
    }

    /// set term with enum
    fn with_enum<T: ComponentId + ComponentType<Enum> + EnumComponentInfo>(
        self,
        value: T,
    ) -> Self {
        let enum_id = T::entity_id(self.world());
        let enum_field_id = value.id_variant(self.world());
        self.with((enum_id, enum_field_id))
    }

    /// set term with enum wildcard
    fn with_enum_wildcard<T: ComponentType<Enum> + ComponentId>(self) -> Self
    where
        (crate::core::utility::id::Id<T>, u64): InternalIntoEntity,
    {
        self.with((T::id(), flecs::Wildcard::ID))
    }

    /* Without methods, shorthand for .with(...).not() */

    /// set term without Id
    fn without<'s, T>(self, id: T) -> Self
    where
        Access<'s>: FromAccessArg<T>,
    {
        self.with(id).not()
    }

    /// set term without enum
    fn without_enum<T: ComponentId + ComponentType<Enum> + EnumComponentInfo>(
        self,
        value: T,
    ) -> Self {
        self.with_enum(value).not()
    }

    /// set term without enum wildcard
    fn without_enum_wildcard<T: ComponentId + ComponentType<Enum> + EnumComponentInfo>(
        self,
    ) -> Self
    where
        (crate::core::utility::id::Id<T>, u64): InternalIntoEntity,
    {
        self.with_enum_wildcard::<T>().not()
    }

    /// Term notation for more complex query features
    ///
    /// sets the current term to next one in term list
    fn term(mut self) -> Self {
        self.advance_term();
        self
    }

    /// Sets the current term to the one with the provided type.
    /// This loops over all terms to find the one with the provided type.
    /// For performance-critical paths, use `term_at(index: u32)` instead.
    ///
    /// # Panics
    ///
    /// Panics if no term with the provided type exists in the query.
    /// See [`try_term_at_type()`](Self::try_term_at_type) for a fallible variant.
    fn term_at_type<T: ComponentId>(self) -> Self {
        self.try_term_at_type::<T>()
            .expect("term_at_type() called with type that is not in query")
    }

    /// Sets the current term to the one with the provided type, returning `None`
    /// if no term with the provided type exists in the query.
    /// This loops over all terms to find the one with the provided type.
    /// For performance-critical paths, use `term_at(index: u32)` instead.
    fn try_term_at_type<T: ComponentId>(self) -> Option<Self> {
        let term_id = T::entity_id(self.world());
        let world_ptr = self.world_ptr_mut();

        for i in 0..=self.term_builder().next_term_index - 1 {
            let desc = self.query_desc();
            let cur_term = desc.terms[i as usize];
            let cur_term_id = cur_term.id;
            let cur_term_pair = ecs_pair(cur_term.first.id, cur_term.second.id);

            if (term_id == cur_term_id
                || (cur_term_id != 0
                    && term_id == unsafe { sys::ecs_get_typeid(world_ptr, cur_term_id) }))
                || (term_id == cur_term_pair
                    || (cur_term_pair != 0
                        && term_id == unsafe { sys::ecs_get_typeid(world_ptr, cur_term_pair) }))
            {
                return Some(self.term_at(i as u32));
            }
        }

        None
    }

    /// Sets the current term to the one at the provided index.
    fn term_at(mut self, index: u32) -> Self {
        ecs_assert!(
            index < sys::FLECS_TERM_COUNT_MAX,
            FlecsErrorCode::InvalidParameter,
            "term_at() called with invalid index"
        );

        self.set_term_ref_mode(TermRefMode::Src);

        *self.current_term_index_mut() = index as i32;

        self
    }

    //flecs_force_build_debug_c || flecs_force_enable_ecs_asserts

    /*
            /** Sets the current term to the one at the provided index and asserts that the type matches.
         */
        template <typename T>
        Base& term_at(int32_t term_index) {
            this->term_at(term_index);
    #if !defined(FLECS_NDEBUG) || defined(FLECS_KEEP_ASSERT)
            flecs::id_t term_id = _::type<T>::id(this->world_v());
            ecs_term_t cur_term = *this->term_;
            ecs_id_t cur_term_id = cur_term.id;
            ecs_id_t cur_term_pair = ecs_pair(cur_term.first.id, cur_term.second.id);

            ecs_assert((term_id == cur_term_id || (cur_term_id != 0 && term_id == ecs_get_typeid(this->world_v(), cur_term_id))) ||
                (term_id == cur_term_pair || (cur_term_pair != 0 && term_id == ecs_get_typeid(this->world_v(), cur_term_pair))),
                ECS_INVALID_PARAMETER, "term type mismatch");
    #endif
            return *this;
        }
         */

    /// Set the current term to the one with the provided id and assert that the type matches.
    /// this does not do the type checking in release unless `flecs_force_build_debug_c` or `flecs_force_enable_ecs_asserts` is enabled.
    fn term_at_checked<T: ComponentId>(mut self, index: u32) -> Self {
        ecs_assert!(
            index < sys::FLECS_TERM_COUNT_MAX,
            FlecsErrorCode::InvalidParameter,
            "term_at() called with invalid index"
        );

        self.set_term_ref_mode(TermRefMode::Src);

        *self.current_term_index_mut() = index as i32;

        #[cfg(any(
            feature = "flecs_force_build_debug_c",
            feature = "flecs_force_enable_ecs_asserts",
            debug_assertions
        ))]
        {
            let term_id = T::entity_id(self.world());
            let cur_term = self.current_term();
            let cur_term_id = cur_term.id;
            let cur_term_pair = ecs_pair(cur_term.first.id, cur_term.second.id);

            ecs_assert!(
                (term_id == cur_term_id
                    || (cur_term_id != 0
                        && term_id
                            == unsafe { sys::ecs_get_typeid(self.world_ptr_mut(), cur_term_id) }))
                    || (term_id == cur_term_pair
                        || (cur_term_pair != 0
                            && term_id
                                == unsafe {
                                    sys::ecs_get_typeid(self.world_ptr_mut(), cur_term_pair)
                                })),
                FlecsErrorCode::InvalidParameter,
                "term type mismatch"
            );
        }

        self
    }

    /// Set the id as current term and in mode out
    fn write<'s, T>(self, id: T) -> Self
    where
        Access<'s>: FromAccessArg<T>,
    {
        let me = self.with(id);
        TermBuilderImpl::write_curr(me)
    }

    /// Set the type as current term and in mode out
    fn write_enum<T: ComponentId + ComponentType<Enum> + EnumComponentInfo>(
        self,
        value: T,
    ) -> Self {
        let me = self.with_enum(value);
        TermBuilderImpl::write_curr(me)
    }

    /// Set the id as current term and in mode in
    fn read<'s, T>(self, id: T) -> Self
    where
        Access<'s>: FromAccessArg<T>,
    {
        let me = self.with(id);
        TermBuilderImpl::read_curr(me)
    }

    /// Set the type as current term and in mode in
    fn read_enum<T: ComponentId + ComponentType<Enum> + EnumComponentInfo>(
        self,
        value: T,
    ) -> Self {
        let me = self.with_enum(value);
        TermBuilderImpl::read_curr(me)
    }

    /* scope_open/scope_close shorthand notation. */

    /// Open a scope for the query
    fn scope_open(self) -> Self {
        self.with(flecs::ScopeOpen::ID).entity(0)
    }

    /// Close a scope for the query
    fn scope_close(self) -> Self {
        self.with(flecs::ScopeClose::ID).entity(0)
    }

    /// Sorts the output of a query.
    ///
    /// This enables sorting of entities across matched tables. As a result of this
    /// operation, the order of entities in the matched tables may change.
    /// Resorting occurs when a query iterator is obtained, and only if the table
    /// data has changed.
    ///
    /// If multiple queries that match the same (down)set of tables specify different
    /// sorting functions, resorting is likely to occur every time an iterator is
    /// obtained, potentially slowing down iterations significantly.
    ///
    /// The sorting function will be applied to the specified component. Resorting
    /// only occurs if that component has changed, or when the entity order in the
    /// table changes. If no component is provided, resorting only occurs when
    /// the entity order changes.
    ///
    /// # Type Parameters
    ///
    /// * `T`: The component used to sort.
    ///
    /// # Arguments
    ///
    /// * `compare`: The compare function used to sort the components.
    ///   The signature of the function must be `fn(Entity, &T, Entity, &T) -> i32`.
    fn order_by<T>(mut self, compare: impl OrderByFn<T>) -> Self
    where
        T: ComponentId,
        Self: QueryBuilderImpl<'a>,
    {
        const {
            assert!(
                !(T::IS_REF || T::IS_MUT),
                "order_by<T> requires T to not be a reference"
            );
        }

        let cmp: sys::ecs_order_by_action_t = Some(unsafe {
            core::mem::transmute::<OrderByFnPtr<T>, OrderByFnPtrUnsafe>(compare.to_extern_fn())
        });

        self.__internal_order_by_id(T::entity_id(self.world()), cmp);
        self
    }

    #[doc(hidden)]
    fn __internal_order_by_id(
        &mut self,
        component: impl Into<Entity>,
        compare: sys::ecs_order_by_action_t,
    ) {
        let desc = self.query_desc_mut();
        desc.order_by_callback = compare;
        desc.order_by = *component.into();
    }

    /// Sorts the output of a query.
    ///
    /// This is similar to `order_by<T>`, but uses a component identifier instead.
    ///
    /// # Arguments
    ///
    /// * `component`: The component used to sort.
    /// * `compare`: The compare function used to sort the components.
    fn order_by_id(
        mut self,
        component: impl Into<Entity>,
        compare: impl OrderByFnVoid,
    ) -> Self {
        let desc = self.query_desc_mut();
        let cmp: sys::ecs_order_by_action_t = Some(unsafe {
            core::mem::transmute::<OrderByFnVoidPtr, OrderByFnVoidPtrUnsafe>(compare.to_extern_fn())
        });
        desc.order_by_callback = cmp;
        desc.order_by = *component.into();
        self
    }

    /// Group and sort matched tables.
    ///
    /// This function is similar to `order_by`, but instead of sorting individual entities,
    /// it only sorts matched tables. This can be useful if a query needs to enforce a
    /// certain iteration order upon the tables it is iterating, for example by giving
    /// a certain component or tag a higher priority.
    ///
    /// The sorting function assigns a "rank" to each type, which is then used to sort
    /// the tables. Tables with higher ranks will appear later in the iteration.
    ///
    /// Resorting happens when a query iterator is obtained, and only if the set of
    /// matched tables for a query has changed. If table sorting is enabled together
    /// with entity sorting, table sorting takes precedence, and entities will be sorted
    /// within each set of tables that are assigned the same rank.
    ///
    /// # Arguments
    ///
    /// * `component`: The component used to determine the group rank.
    /// * `group_by_action`: Callback that determines group id for table.
    fn group_by_fn(
        mut self,
        component: impl IntoEntity,
        group_by_action: sys::ecs_group_by_action_t,
    ) -> Self {
        let world = self.world();
        let desc = self.query_desc_mut();
        desc.group_by_callback = group_by_action;
        desc.group_by = *component.into_entity(world);
        self
    }

    /// Group and sort matched tables.
    ///
    /// This is similar to `group_by_default<T>`, but uses a component identifier instead.
    ///
    /// # Arguments
    ///
    /// * `component`: The component used to determine the group rank.
    fn group_by(self, component: impl IntoEntity) -> Self {
        self.group_by_fn(component, None)
    }

    /// Specify context to be passed to the `group_by` function.
    ///
    /// # Arguments
    ///
    /// * `ctx`: Context to pass to the `group_by` function.
    /// * `ctx_free`: Function to clean up the context (called when the query is deleted).
    fn group_by_ctx(mut self, ctx: *mut c_void, ctx_free: sys::ecs_ctx_free_t) -> Self {
        let desc = self.query_desc_mut();
        desc.group_by_ctx = ctx;
        desc.group_by_ctx_free = ctx_free;
        self
    }

    /// Specify the `on_group_create` action.
    ///
    /// # Arguments
    ///
    /// * `action`: The action to execute when a group is created.
    fn on_group_create(mut self, action: sys::ecs_group_create_action_t) -> Self {
        let desc = self.query_desc_mut();
        desc.on_group_create = action;
        self
    }

    /// Specify the `on_group_delete` action.
    ///
    /// # Arguments
    ///
    /// * `action`: The action to execute when a group is deleted.
    fn on_group_delete(mut self, action: sys::ecs_group_delete_action_t) -> Self {
        let desc = self.query_desc_mut();
        desc.on_group_delete = action;
        self
    }
}

pub trait OrderByFn<T>
where
    T: ComponentId,
{
    fn to_extern_fn(self) -> OrderByFnPtr<T>;
}

impl<F, T: ComponentId> OrderByFn<T> for F
where
    F: Fn(Entity, &T, Entity, &T) -> i32,
{
    fn to_extern_fn(self) -> OrderByFnPtr<T> {
        const {
            assert!(core::mem::size_of::<Self>() == 0);
        }
        core::mem::forget(self);

        #[extern_abi]
        fn output<F, T>(e1: Entity, e1_data: &T, e2: Entity, e2_data: &T) -> i32
        where
            F: Fn(Entity, &T, Entity, &T) -> i32,
        {
            (unsafe { core::mem::transmute_copy::<_, F>(&()) })(e1, e1_data, e2, e2_data)
        }

        output::<F, T>
    }
}

pub trait OrderByFnVoid {
    fn to_extern_fn(self) -> OrderByFnVoidPtr;
}

impl<F> OrderByFnVoid for F
where
    F: Fn(Entity, *const c_void, Entity, *const c_void) -> i32,
{
    fn to_extern_fn(self) -> OrderByFnVoidPtr {
        const {
            assert!(core::mem::size_of::<Self>() == 0);
        }
        core::mem::forget(self);

        #[extern_abi]
        fn output<F>(e1: Entity, e1_data: *const c_void, e2: Entity, e2_data: *const c_void) -> i32
        where
            F: Fn(Entity, *const c_void, Entity, *const c_void) -> i32,
        {
            (unsafe { core::mem::transmute_copy::<_, F>(&()) })(e1, e1_data, e2, e2_data)
        }

        output::<F>
    }
}
