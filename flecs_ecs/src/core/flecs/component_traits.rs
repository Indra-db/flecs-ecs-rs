//! Component traits are tags and pairs that can be added to components to modify their behavior.

use super::*;

/// Marker trait for Flecs component traits.
pub trait FlecsComponentTrait {}

// Component traits

/// A relationship can be marked with the `Acyclic` trait to indicate that it cannot contain cycles.
/// Both the builtin `ChildOf` and `IsA` relationships are marked acyclic. Knowing whether a relationship
/// is acyclic allows the storage to detect and throw errors when a cyclic relationship is introduced by accident.
///
/// Note that because cycle detection requires expensive algorithms, adding `Acyclic` to a relationship does not
/// guarantee that an error will be thrown when a cycle is accidentally introduced. While detection may improve
/// over time, an application that runs without errors is no guarantee that it does not contain acyclic
/// relationships with cycles.
#[derive(Debug, Default, Clone)]
pub struct Acyclic;

impl_component_trait!(Acyclic, ECS_ACYCLIC);

/// The `CanToggle` trait allows a component to be toggled. Component toggling can (temporarily) disable a
/// component, which excludes it from queries. Component toggling can be used as a cheaper alternative to
/// adding/removing as toggling relies on setting a bitset, and doesn't require the entity to be moved between
/// tables. Component toggling can also be used to restore a component with its old value.
///
/// Queries treat a disabled component as if the entity doesn't have it. `CanToggle` components add a small
/// amount of overhead to query evaluation, even for entities that did not toggle their component.
///
/// # Example
/// ```rust
/// # use flecs_ecs::prelude::*;
///
/// # #[derive(Component)]
/// # struct Position {
/// #     x: f32,
/// #     y: f32,
/// # }
///
/// # let world = World::new();
///
/// world
///     .component::<Position>()
///     .add_trait::<flecs::CanToggle>();
///
/// let e = world.entity().set(Position { x: 10.0, y: 20.0 });
///
/// e.disable(Position::id()); // Disable component
/// assert!(!e.is_enabled(Position::id()));
///
/// e.enable(Position::id()); // Enable component
/// assert!(e.is_enabled(Position::id()));
/// ```
#[derive(Debug, Default, Clone)]
pub struct CanToggle;

impl_component_trait!(CanToggle, ECS_CAN_TOGGLE);

/// Cleanup traits ensure that the store does not contain any dangling references when entities are deleted.
///
/// When entities that are used as tags, components, relationships or relationship targets are deleted,
/// cleanup traits ensure that the store does not contain any dangling references. Any cleanup policy
/// provides this guarantee, so while they are configurable, applications cannot configure traits that
/// allow for dangling references.
///
/// **Note**: this only applies to entities (like tags, components, relationships) that are added _to_
/// other entities. It does not apply to components that store an entity value, so:
///
/// ```no_run
/// # use flecs_ecs::prelude::*;
/// # let world = World::new();
/// # let parent = world.entity();
/// # let e = world.entity();
/// #[derive(Component)]
/// struct MyComponent {
///     e: Entity, // Not covered by cleanup traits
/// }
///
/// e.child_of(parent); // Covered by cleanup traits
/// ```
///
/// The default policy is that any references to the entity will be **removed**. For example, when the
/// tag `Archer` is deleted, it will be removed from all entities that have it. Which is similar to invoking
/// the [`World::remove_all()`](flecs_ecs::core::World::remove_all) method.
///
/// Since entities can be used in relationship pairs, just calling `remove_all` on just the entity itself
/// does not guarantee that no dangling references are left. A more comprehensive description of what happens is:
///
/// ```
/// # use flecs_ecs::prelude::*;
/// # let world = World::new();
/// # let archer = world.entity();
/// world.remove_all(archer);
/// world.remove_all((archer, flecs::Wildcard));
/// world.remove_all((flecs::Wildcard, archer));
/// ```
///
/// This succeeds in removing all possible references to `Archer`. Sometimes this behavior is not what we want however.
/// Consider a parent-child hierarchy, where we want to delete the child entities when the parent is deleted.
/// Instead of removing `(ChildOf, parent)` from all children, we need to *delete* the children.
///
/// We also want to specify this per relationship. If an entity has `(Likes, parent)` we may not want to delete that entity,
/// meaning the cleanup we want to perform for `Likes` and `ChildOf` may not be the same.
///
/// This is what cleanup traits are for: to specify which action needs to be executed under which condition.
/// They are applied *to* entities that have a reference to the entity being deleted:
/// if I delete the `Archer` tag I remove the tag *from* all entities that have it.
///
/// To configure a cleanup policy for an entity, a `(Condition, Action)` pair can be added to it.
/// If no policy is specified, the default cleanup action (`Remove`) is performed.
///
/// There are three cleanup actions:
/// - [`Remove`]: as if doing `remove_all(entity)` (default)
/// - [`Delete`]: as if doing `delete_with(entity)`
/// - [`Panic`]: throw a fatal error (default for components)
///
/// There are two cleanup conditions:
/// - [`OnDelete`]: the component, tag or relationship is deleted
/// - [`OnDeleteTarget`]: a target used with the relationship is deleted
///
/// Policies apply to both regular and pair instances, so to all entities with `T` as well as `(T, *)`.
///
/// # Cleanup order
///
/// While cleanup actions allow for specifying what needs to happen when a particular entity is deleted,
/// or when an entity used with a particular relationship is deleted, they do not enforce a strict cleanup *order*.
/// The reason for this is that there can be many orderings that satisfy the cleanup traits.
///
/// This is important to consider especially when writing `OnRemove` triggers or hooks,
/// as the order in which they are invoked highly depends on the order in which entities are cleaned up.
///
/// Take an example with a parent and a child that both have the `Node` tag:
///
/// ```rust
/// # use flecs_ecs::prelude::*;
/// # #[derive(Component)]
/// # struct Node;
/// # let world = World::new();
/// world
///     .observer::<flecs::OnRemove, ()>()
///     .with(Node)
///     .each_entity(|e, _| {
///         // This observer will be invoked when a Node is removed
///     });
///
/// let p = world.entity().add(Node);
/// let c = world.entity().add(Node).child_of(p);
/// ```
///
/// In this example, when calling `p.destruct()` the observer is first invoked for the child, and then for the parent,
/// which is to be expected as the child is deleted before the parent.
/// Cleanup traits do not however guarantee that this is always the case.
///
/// An application could also call `world.component::<Node>().destruct()` which would delete the `Node` component and
/// all of its instances. In this scenario the cleanup traits for the `flecs::ChildOf` relationship are not considered,
/// and therefore the ordering is undefined. Another typical scenario in which ordering is undefined is when an application
/// has cyclical relationships with a `Delete` cleanup action.
///
/// #### Cleanup order during world teardown
/// Cleanup issues often show up during world teardown as the ordering in which entities are deleted is controlled by the application.
/// While world teardown respects cleanup traits, there can be many entity delete orderings that are valid according to the cleanup traits,
/// but not all of them are equally useful. There are ways to organize entities that helps world cleanup to do the right thing. These are:
///
/// **Organize components, triggers, observers and systems in modules.**
/// Storing these entities in modules ensures that they stay alive for as long as possible.
/// This leads to more predictable cleanup ordering as components will be deleted as their entities are,
/// vs. when the component is deleted. It also ensures that triggers and observers are not deleted while matching events are still being generated.
///
/// **Avoid organizing components, triggers, observers and systems under entities that are not modules**.
/// If a non-module entity with children is stored in the root, it will get cleaned up along with other regular entities.
/// If you have entities such as these organized in a non-module scope, consider adding the `flecs::Module` tag to the root of that scope.
///
/// The next section goes into more detail on why this improves cleanup behavior and what happens during world teardown.
///
/// #### World teardown sequence
/// To understand why some ways to organize entities work better than others, having an overview of what happens during world teardown is useful.
/// Here is a list of the steps that happen when a world is deleted:
///
/// 1. **Find all root entities**
///    World teardown starts by finding all root entities, which are entities that do not have the builtin `ChildOf` relationship.
///    Note that empty entities (entities without any components) are not found during this step.
///
/// 2. **Query out modules, components, observers and systems**
///    This ensures that components are not cleaned up before the entities that use them, and triggers,
///    observers and systems are not cleaned up while there are still conditions under which they could be invoked.
///
/// 3. **Query out entities that have no children**
///    If entities have no children they cannot cause complex cleanup logic. This also decreases the likelihood of initiating cleanup actions that could impact other entities.
///
/// 4. **Delete root entities**
///    The root entities that were not filtered out will be deleted.
///
/// 5. **Delete everything else**
///    The last step will delete all remaining entities. At this point cleanup traits are no longer considered and cleanup order is undefined.
pub mod cleanup {
    use super::*;

    /// Cleanup action. Remove any references to the entity being deleted. This is the default cleanup action.
    /// see [`cleanup`] for general information on cleanup traits.
    ///
    /// # Example
    /// ```
    /// # use flecs_ecs::prelude::*;
    /// # #[derive(Component)]
    /// # struct Archer;
    /// # let world = World::new();
    ///
    /// // Remove Archer from entities when Archer is deleted
    /// world
    ///     .component::<Archer>()
    ///     .add_trait::<(flecs::OnDelete, flecs::Remove)>();
    ///
    /// let e = world.entity().add(Archer::id());
    ///
    /// // This will remove Archer from e
    /// world.component::<Archer>().destruct();
    /// ```
    #[derive(Debug, Default, Clone)]
    pub struct Remove;

    impl_component_trait!(Remove, ECS_REMOVE);

    /// Cleanup action. Delete any entities that have a reference to the entity being deleted.
    /// see [`cleanup`] for general information on cleanup traits.
    ///
    /// # Example
    ///
    /// ```rust
    /// # use flecs_ecs::prelude::*;
    /// # #[derive(Component)]
    /// # struct Archer;
    /// # let world = World::new();
    ///
    /// // Delete entities with Archer when Archer is deleted
    /// world
    ///     .component::<Archer>()
    ///     .add_trait::<(flecs::OnDelete, flecs::Delete)>();
    ///
    /// let e = world.entity().add(Archer::id());
    ///
    /// // This will delete e
    /// world.component::<Archer>().destruct();
    /// ```
    ///
    /// ```rust
    /// # use flecs_ecs::prelude::*;
    /// # let world = World::new();
    /// world
    ///     .component::<flecs::ChildOf>()
    ///     .add_trait::<(flecs::OnDeleteTarget, flecs::Delete)>();
    /// let p = world.entity();
    /// let e = world.entity().child_of(p);
    ///
    /// // This will delete both p and e
    /// p.destruct();
    /// ```
    #[derive(Debug, Default, Clone)]
    pub struct Delete;

    impl_component_trait!(Delete, ECS_DELETE);

    /// Cleanup action. Throw a fatal error. This is the default for components.
    /// see [`cleanup`] for general information on cleanup traits.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # //TODO this doesn't panic in release mode, texted Sander 22 october 2025
    /// # use flecs_ecs::prelude::*;
    /// # #[derive(Component)]
    /// # struct Archer;
    /// # let world = World::new();
    ///
    /// // Panic when Archer is deleted
    /// world
    ///     .component::<Archer>()
    ///     .add_trait::<(flecs::OnDelete, flecs::Panic)>();
    ///
    /// let e = world.entity().add(Archer::id());
    ///
    /// // This will panic
    /// world.component::<Archer>().destruct();
    /// ```
    #[derive(Debug, Default, Clone)]
    pub struct Panic;

    impl_component_trait!(Panic, ECS_PANIC);

    /// Cleanup condition. Specifies what action to take when the component, tag or relationship is deleted.
    ///
    /// Used as a pair with one of [`Remove`], [`Delete`], or [`Panic`].
    ///
    /// see [`cleanup`] for general information on cleanup traits.
    ///
    /// # Example
    ///
    /// ```rust
    /// # use flecs_ecs::prelude::*;
    /// # #[derive(Component)]
    /// # struct Archer;
    /// # let world = World::new();
    ///
    /// // Delete entities with Archer when Archer is deleted
    /// world
    ///     .component::<Archer>()
    ///     .add_trait::<(flecs::OnDelete, flecs::Delete)>();
    ///
    /// let e = world.entity().add(Archer::id());
    ///
    /// // This will delete e
    /// world.component::<Archer>().destruct();
    /// ```
    ///
    /// # See also
    /// * [`OnDeleteTarget`]
    #[derive(Debug, Default, Clone)]
    pub struct OnDelete;

    impl_component_trait!(OnDelete, ECS_ON_DELETE);

    /// Cleanup condition. Specifies what action to take when a target used with the relationship is deleted.
    ///
    /// Used as a pair with one of [`Remove`], [`Delete`], or [`Panic`].
    ///
    /// see [`cleanup`] for general information on cleanup traits.
    ///
    /// # Example
    ///
    /// ```rust
    /// # use flecs_ecs::prelude::*;
    /// # let world = World::new();
    /// world
    ///     .component::<flecs::ChildOf>()
    ///     .add_trait::<(flecs::OnDeleteTarget, flecs::Delete)>();
    /// let p = world.entity();
    /// let e = world.entity().child_of(p);
    ///
    /// // This will delete both p and e
    /// p.destruct();
    /// ```
    ///
    /// # See also
    /// * [`OnDelete`]
    #[derive(Debug, Default, Clone)]
    pub struct OnDeleteTarget;

    impl_component_trait!(OnDeleteTarget, ECS_ON_DELETE_TARGET);
}

pub use cleanup::*;

/// The `DontFragment` trait uses the same sparse storage as the `Sparse` trait, but does not fragment tables.
/// This can be desirable especially if a component or relationship is very sparse (e.g. it is only added to a
/// few entities) as this would otherwise result in many tables that only contain a small number of entities.
///
/// The following code example shows how to mark a component as `DontFragment`:
///
/// # Example
/// ```no_run
/// # use flecs_ecs::prelude::*;
/// #[derive(Component)]
/// #[flecs(traits(DontFragment))]
/// struct Position {
///     x: f32,
///     y: f32,
/// }
/// # let world = World::new();
///
/// // or
/// world
///     .component::<Position>()
///     .add_trait::<flecs::DontFragment>();
/// ```
///
/// Components with the `DontFragment` trait have the following limitations:
/// - They don't show up in types [`Archetype`](flecs_ecs::core::Archetype)
/// - Monitors don't trigger on `DontFragment` components. The reason for this is that monitors compare the
///   previous table with the current table of an entity to determine if an entity started matching, and
///   `DontFragment` components aren't part of the table.
///
/// Support for `DontFragment` has a number of (temporary) limitations:
/// - `target_for` does not yet work for `DontFragment` components.
/// - `DontFragment` components are not serialized yet to JSON (and don't show up in the explorer).
/// - `Or`, `Optional`, `AndFrom` and `NotFrom` operators are not yet supported.
/// - Component inheritance and transitivity are not yet supported.
/// - Queries for `DontFragment` components may run slower than expected.
///
/// What does work:
/// - ECS operations (`add`, `remove`, `get`, `get_mut`, `ensure`, `emplace`, `set`, `delete`).
/// - Relationships (including `Exclusive` relationships).
/// - Simple component queries.
/// - Wildcard queries.
/// - Queries with variables.
#[derive(Debug, Default, Clone)]
pub struct DontFragment;

impl_component_trait!(DontFragment, ECS_DONT_FRAGMENT);

/// The `Exclusive` trait enforces that an entity can have only a single instance of a relationship. When a
/// second instance is added, it replaces the first instance. An example of a relationship with the `Exclusive`
/// trait is the builtin `ChildOf` relationship.
///
/// # Usage Example
/// ```no_run
/// # use flecs_ecs::prelude::*;
/// # let world = World::new();
/// # let e = world.entity();
/// # let parent_a = world.entity();
/// # let parent_b = world.entity();
/// e.child_of(parent_a);
/// e.child_of(parent_b); // replaces (ChildOf, parent_a)
/// ```
///
/// to create a custom exclusive relationship, add the `Exclusive` trait to it:
/// ```
/// # use flecs_ecs::prelude::*;
/// # let world = World::new();
/// let married_to = world.entity().add_trait::<flecs::Exclusive>();
/// ```
#[derive(Debug, Default, Clone)]
pub struct Exclusive;

impl_component_trait!(Exclusive, ECS_EXCLUSIVE);

/// Entities can be annotated with the `Final` trait, which prevents using them with `IsA` relationship.
/// This is similar to the concept of a final class as something that cannot be extended.
///
/// # Example
/// ```
/// # use flecs_ecs::prelude::*;
/// # let world = World::new();
/// let e = world.entity().add_trait::<flecs::Final>();
///
/// // not allowed
/// // let i = world.entity().is_a(e);
/// ```
///
/// Queries may use the final trait to optimize, as they do not have to explore subsets of a final entity.
#[derive(Debug, Default, Clone)]
pub struct Final;

impl_component_trait!(Final, ECS_FINAL);

/// The `Inheritable` trait indicates that a component can be inherited from (it can be used as target of an
/// `IsA` relationship). It is not required to add this trait to components before using them as target of an
/// `IsA` pair, but it can be used to ensure that queries for the component take into account component inheritance.
///
/// # Example
/// ```
/// # use flecs_ecs::prelude::*;
/// # #[derive(Component)]
/// # struct Unit;
/// # #[derive(Component)]
/// # struct Warrior;
/// # let world = World::new();
/// world.component::<Unit>().add_trait::<flecs::Inheritable>();
///
/// let q = world.query::<()>().with(Unit).build();
///
/// world.component::<Warrior>().is_a(Unit::id());
///
/// q.each_entity(|e, _| {
///     // ...
/// });
/// ```
///
/// Queries must be aware of (potential) inheritance relationships when they are created. A query will be
/// created with support for inheritance under the following conditions:
///  - If the component has the `Inheritable` trait
///  - If the component inherits from another component and is not `Final`
///
/// If a query was not aware of inheritance relationships at creation time and one or more of the components
/// in the query were inherited from, query iteration will fail in debug mode.
#[derive(Debug, Default, Clone)]
pub struct Inheritable;

impl_component_trait!(Inheritable, ECS_INHERITABLE);

/// The `OneOf` trait enforces that the target of the relationship is a child of a specified entity. `OneOf`
/// can be used to indicate that the target needs to be either a child of the relationship (common for enum
/// relationships), or of another entity.
///
/// # Example - Constrain target to child of relationship
/// ```
/// # use flecs_ecs::prelude::*;
/// # let world = World::new();
///
/// // Enforce that target of relationship is child of Food
/// let food = world.entity().add_trait::<flecs::OneOf>();
/// let apples = world.entity().child_of(food);
/// let fork = world.entity();
///
/// // This is ok, Apples is a child of Food
/// let a = world.entity().add((food, apples));
///
/// // not allowed - Fork is not a child of Food
/// // let b = world.entity().add((food, fork));
/// ```
///
/// # Example - Constrain target to child of another entity
/// The following example shows how `OneOf` can be used to enforce that the relationship
/// target is the child of an entity other than the relationship:
/// ```
/// # use flecs_ecs::prelude::*;
/// # let world = World::new();
///
/// // Enforce that target of relationship is child of Food
/// let food = world.entity();
/// let eats = world.entity().add((flecs::OneOf::id(), food));
/// let apples = world.entity().child_of(food);
/// let fork = world.entity();
///
/// // This is ok, Apples is a child of Food
/// let a = world.entity().add((eats, apples));
///
/// // not allowed - Fork is not a child of Food
/// // let b = world.entity().add((eats, fork));
/// ```
#[derive(Debug, Default, Clone)]
pub struct OneOf;

impl_component_trait!(OneOf, ECS_ONE_OF);

/// `OnInstantiate` traits configure component behavior during entity instantiation.
///
/// The `OnInstantiate` trait configures the behavior of components when an entity is instantiated from
/// another entity (usually a prefab). Instantiation happens when an `IsA` pair is added to an entity.
///
/// By default, when an entity is instantiated, the components from the base entity (the `IsA` target)
/// are copied to the instance. This behavior can be modified with the `OnInstantiate` trait, which can
/// be used as a pair in combination with three targets:
///
/// | Target | Description |
/// |--------|-------------|
/// | [`Override`] | Copy component from base to instance (default) |
/// | [`Inherit`] | Inherit component from base |
/// | [`DontInherit`] | Don't inherit (and don't copy) component from base |
pub mod on_instantiate {
    use super::*;

    /// The `OnInstantiate` trait configures the behavior of components when an entity is instantiated from another
    /// entity (usually a prefab). Instantiation happens when an `IsA` pair is added to an entity.
    ///
    /// By default, when an entity is instantiated, the components from the base entity (the `IsA` target) are copied
    /// to the instance. This behavior can be modified with the `OnInstantiate` trait, which can be used as pair in
    /// combination with three targets: [`Override`], [`Inherit`], or [`DontInherit`].
    #[derive(Debug, Default, Clone)]
    pub struct OnInstantiate;

    impl_component_trait!(OnInstantiate, ECS_ON_INSTANTIATE);

    /// The default behavior for `OnInstantiate` is `Override`, which means that the component is copied to the
    /// instance. This means that after instantiation, the instance has an owned copy for the component that masks
    /// the base component (the "override").
    ///
    /// Note that for an override to work correctly, a component has to be clone-able.
    ///
    /// # Example
    /// ```
    /// # use flecs_ecs::prelude::*;
    /// # use flecs_ecs::experimental::prelude::*;
    ///
    /// #[derive(Component, Clone, PartialEq)]
    /// struct Mass {
    ///     value: f32,
    /// }
    /// # let world = World::new();
    ///
    /// // Register component with trait. Optional, since this is the default behavior.
    /// world
    ///     .component::<Mass>()
    ///     .add_trait::<(flecs::OnInstantiate, flecs::Override)>();
    ///
    /// let base = world.entity().set(Mass { value: 100.0 });
    /// let inst = world.entity().is_a(base); // Mass is copied to inst
    ///
    /// assert!(inst.owns(Mass::id()));
    /// assert!(base.cloned::<&Mass>() == inst.cloned::<&Mass>());
    /// ```
    #[derive(Debug, Default, Clone)]
    pub struct Override;

    impl_component_trait!(Override, ECS_OVERRIDE);

    /// Components with the `Inherit` trait are inherited from a base entity (the `IsA` target) on instantiation.
    /// Inherited components are not copied to the instance, and are only stored once in memory. Operations such as
    /// `get` and `has`, and queries will automatically lookup inheritable components by following the `IsA` relationship.
    ///
    /// Inheritable components can be overridden manually by adding the component to the instance. This results in
    /// the same behavior as the `Override` trait, where the component is copied from the base entity.
    ///
    /// # Example
    /// ```
    /// # use flecs_ecs::prelude::*;
    /// # use flecs_ecs::experimental::prelude::*;
    /// #[derive(Component, Clone, PartialEq)]
    /// struct Mass {
    ///     value: f32,
    /// }
    /// # let world = World::new();
    ///
    /// // Register component with trait
    /// world
    ///     .component::<Mass>()
    ///     .add_trait::<(flecs::OnInstantiate, flecs::Inherit)>();
    ///
    /// let base = world.entity().set(Mass { value: 100.0 });
    /// let inst = world.entity().is_a(base);
    ///
    /// assert!(inst.has(Mass::id()));
    /// assert!(!inst.owns(Mass::id()));
    /// // Inherited component points to the same data
    /// assert!(base.cloned::<&Mass>() == inst.cloned::<&Mass>());
    /// ```
    #[derive(Debug, Default, Clone)]
    pub struct Inherit;

    impl_component_trait!(Inherit, ECS_INHERIT);

    /// Components with the `DontInherit` trait are not inherited from a base entity (the `IsA` target) on instantiation,
    /// and are not copied to the instance. Operations such as `has` and `get` will not find the component,
    /// and queries will not match it.
    ///
    /// Components with the `DontInherit` cannot be overridden manually.
    /// When a component is added to an instance and the base also has the component,
    /// the base component is ignored and its value is not copied to the instance.
    ///
    /// # Example
    /// ```
    /// # use flecs_ecs::prelude::*;
    /// # use flecs_ecs::experimental::prelude::*;
    /// # let world = World::new();
    /// # #[derive(Component, Clone, PartialEq)]
    /// # struct Mass {
    /// #     value: f32,
    /// # }
    /// // Register component with trait
    /// world
    ///     .component::<Mass>()
    ///     .add_trait::<(flecs::OnInstantiate, flecs::DontInherit)>();
    ///
    /// let base = world.entity().set(Mass { value: 100.0 });
    /// let inst = world.entity().is_a(base);
    ///
    /// assert!(!inst.has(Mass::id()));
    /// assert!(!inst.owns(Mass::id()));
    /// assert!(inst.get::<&Mass>().is_none());
    /// ```
    #[derive(Debug, Default, Clone)]
    pub struct DontInherit;

    impl_component_trait!(DontInherit, ECS_DONT_INHERIT);
}

pub use on_instantiate::*;

/// The `OrderedChildren` trait can be added to entities to indicate that creation order or a custom order should be preserved.
///
/// When this trait is added to a parent, the entity ids returned by the [`EntityView::each_child`](flecs_ecs::core::EntityView::each_child) operations will be in creation or custom order.
/// Children of a parent with the `OrderedChildren` trait are guaranteed to be returned in a single result.
///
/// The trait does not affect the order in which entities are returned by queries.
///
/// The stored order can be modified by an application with the [`EntityView::set_child_order`](flecs_ecs::core::EntityView::set_child_order) operation.
///
/// # Example
/// ```rust
/// # use flecs_ecs::prelude::*;
/// # let world = World::new();
/// # #[derive(Component)]
/// # struct Position { x: f32, y: f32 }
/// let parent = world.entity().add_trait::<flecs::OrderedChildren>();
///
/// let child_1 = world.entity().child_of(parent);
/// let child_2 = world.entity().child_of(parent);
/// let child_3 = world.entity().child_of(parent);
///
/// // Adding/removing components usually changes the order in which children are
/// // iterated, but with the OrderedChildren trait order is preserved.
/// child_2.set(Position { x: 10.0, y: 20.0 });
///
/// parent.each_child(|child| {
///     // 1st result: child_1
///     // 2nd result: child_2
///     // 3rd result: child_3
/// });
/// ```
#[derive(Debug, Default, Clone)]
pub struct OrderedChildren;

impl_component_trait!(OrderedChildren, ECS_ORDERED_CHILDREN);

/// A relationship can be marked with `PairIsTag` in which case a pair with the relationship will never contain data.
/// By default the data associated with a pair is determined by whether either the relationship or target are components.
/// For some relationships however, even if the target is a component, no data should be added to the relationship.
///
/// # Example
/// TODO
#[derive(Debug, Default, Clone)]
pub struct PairIsTag;

impl_component_trait!(PairIsTag, ECS_PAIR_IS_TAG);

/// Component trait. Enforces that an entity can only be used as a relationship.
///
/// # Example
///
/// ```rust, no_run
/// # use flecs_ecs::prelude::*;
/// # let world = World::new();
/// #[derive(Component)]
/// struct Likes;
///
/// #[derive(Component)]
/// struct Apples;
///
/// world
///     .component::<Likes>()
///     .add_trait::<flecs::Relationship>();
///
/// let e = world
///     .entity()
///     // .add(Likes::id()) // Panic, 'Likes' is not used as relationship
///     // .add((Apples::id(), Likes::id())) // Panic, 'Likes' is not used as relationship, but as target
///     .add((Likes::id(), Apples::id())); // OK
/// ```
///
/// Entities marked with `Relationship` may still be used as target if the relationship part of the pair has the `Trait` trait.
/// This ensures the relationship can still be used to configure the behavior of other entities.
///
/// # Example
///
/// ```rust
/// # use flecs_ecs::prelude::*;
/// # let world = World::new();
/// #[derive(Component)]
/// struct Likes;
///
/// #[derive(Component)]
/// struct Loves;
///
/// world
///     .component::<Likes>()
///     .add_trait::<flecs::Relationship>();
///
/// // Even though Likes is marked as relationship and used as target here, this
/// // won't panic as With is marked as trait.
/// world
///     .component::<Loves>()
///     .add_trait::<(flecs::With, Likes)>();
/// ```
#[derive(Debug, Default, Clone)]
pub struct Relationship;

impl_component_trait!(Relationship, ECS_RELATIONSHIP);

/// A relationship can be marked reflexive which means that a query like `Relationship(Entity, Entity)` should evaluate to true.
/// The utility of `Reflexive` becomes more obvious with an example:
///
/// Given this dataset:
/// ```ignore
/// IsA(Oak, Tree)
/// ```
///
/// we can ask whether an oak is a tree:
/// ```ignore
/// IsA(Oak, Tree)
/// - Yes, an Oak is a tree (Oak has (IsA, Tree))
/// ```
///
/// We can also ask whether a tree is a tree, which it obviously is:
/// ```ignore
/// IsA(Tree, Tree)
/// - Yes, even though Tree does not have (IsA, Tree)
/// ```
///
/// However, this does not apply to all relationships. Consider a dataset with a `LocatedIn` relationship:
///
/// ```ignore
/// LocatedIn(SanFrancisco, UnitedStates)
/// ```
///
/// we can now ask whether `SanFrancisco` is located in `SanFrancisco`, which it is not:
/// ```ignore
/// LocatedIn(SanFrancisco, SanFrancisco)
/// - No
/// ```
///
/// In these examples, `IsA` is a reflexive relationship, whereas `LocatedIn` is not.
#[derive(Debug, Default, Clone)]
pub struct Reflexive;

impl_component_trait!(Reflexive, ECS_REFLEXIVE);

/// The `Singleton` trait enforces that a component can only be instantiated once in the world.
/// A singleton component can only be added to the entity that is associated with the component.
/// This happens automatically when using the singleton APIs:
///
/// # Example
///
/// ```rust
/// # use flecs_ecs::prelude::*;
/// # let world = World::new();
/// # #[derive(Component)]
/// # struct TimeOfDay(f32);
/// world
///     .component::<TimeOfDay>()
///     .add_trait::<flecs::Singleton>();
///
/// world.set(TimeOfDay(0.0));
/// ```
///
/// Attempting to add the component to other entities beside itself will panic.
///
/// When a query is created for a component with the `Singleton` trait,
/// the query will automatically match the singleton component on the component entity.
/// This is the same as specifying the component itself as source for the term:
///
/// ```rust
/// # use flecs_ecs::prelude::*;
/// # let world = World::new();
/// # #[derive(Component)]
/// # struct Position { x: f32, y: f32 }
/// # #[derive(Component)]
/// # struct Velocity { x: f32, y: f32 }
/// # #[derive(Component)]
/// # struct TimeOfDay(f32);
/// // Automatically matches TimeOfDay as singleton
/// let q = world.new_query::<(&Position, &Velocity, &TimeOfDay)>();
///
/// // Is the same as
/// let q = world
///     .query::<(&Position, &Velocity, &TimeOfDay)>()
///     .term_at(2)
///     .set_src(TimeOfDay::id())
///     .build();
/// ```
#[derive(Debug, Default, Clone)]
pub struct Singleton;

impl_component_trait!(Singleton, ECS_SINGLETON);

/// The `Sparse` trait configures a component to use sparse storage.
/// Sparse components are stored outside of tables, which means they do not have to be moved.
/// Sparse components are also guaranteed to have stable pointers, which means that a component pointer
/// is not invalidated when an entity moves to a new table. ECS operations and queries work as expected with sparse components.
///
/// Sparse components trade in query speed for component add/remove speed.
/// Adding and removing sparse components still requires an archetype change.
///
/// They also enable storage of non-movable components.
///
/// # Example
/// adding the trait
/// ```rust
/// # use flecs_ecs::prelude::*;
/// # #[derive(Component)]
/// # struct Position { x: f32, y: f32 }
/// # let world = World::new();
/// world.component::<Position>().add_trait::<flecs::Sparse>();
/// ```
#[derive(Debug, Default, Clone)]
pub struct Sparse;

impl_component_trait!(Sparse, ECS_SPARSE);

/// The `Symmetric` trait enforces that when a relationship `(R, Y)` is added to entity `X`, the relationship
/// `(R, X)` will be added to entity `Y`. The reverse is also true, if relationship `(R, Y)` is removed from `X`,
/// relationship `(R, X)` will be removed from `Y`.
///
/// The symmetric trait is useful for relationships that do not make sense unless they are bidirectional.
/// Examples of such relationships are `AlliesWith`, `MarriedTo`, `TradingWith` and so on.
///
/// # Example
/// ```
/// # use flecs_ecs::prelude::*;
/// # let world = World::new();
/// let married_to = world.entity().add_trait::<flecs::Symmetric>();
/// let bob = world.entity();
/// let alice = world.entity();
/// bob.add((married_to, alice)); // Also adds (MarriedTo, Bob) to Alice
/// ```
#[derive(Debug, Default, Clone)]
pub struct Symmetric;

impl_component_trait!(Symmetric, ECS_SYMMETRIC);

/// The target trait enforces that an entity can only be used as relationship target.
///
/// # Example
/// ```
/// # use flecs_ecs::prelude::*;
/// # let world = World::new();
/// #[derive(Component)]
/// struct Likes;
///
/// #[derive(Component)]
/// struct Apples;
///
/// world.component::<Apples>().add_trait::<flecs::Target>();
///
/// let e = world
///     .entity()
///     // .add(Apples::id()) // Panic, 'Apples' is not used as target
///     // .add((Apples::id(), Likes::id())) // Panic, 'Apples' is not used as target, but as relationship
///     .add((Likes::id(), Apples::id())); // OK
/// ```
#[derive(Debug, Default, Clone)]
pub struct Target;

impl_component_trait!(Target, ECS_TARGET);

/// The trait trait marks an entity as a trait, which is any tag that is added to another tag/component/relationship
/// to modify its behavior. All traits in this manual are marked as trait. It is not required to mark a trait as
/// a trait before adding it to another tag/component/relationship. The main reason for the trait trait is to ease
/// some of the constraints on relationships (see the [`Relationship`] trait).
///
/// # Example
/// ```
/// # use flecs_ecs::prelude::*;
/// # let world = World::new();
/// #[derive(Component)]
/// struct Serializable;
///
/// world
///     .component::<Serializable>()
///     .add_trait::<flecs::Trait>();
/// ```
#[derive(Debug, Default, Clone)]
pub struct Trait;

impl_component_trait!(Trait, ECS_TRAIT);

/// Relationships can be marked as transitive. A formal-ish definition of transitivity in the context of
/// relationships is:
///
/// ```text
/// If   Relationship(EntityA, EntityB)
/// And  Relationship(EntityB, EntityC)
/// Then Relationship(EntityA, EntityC)
/// ```
///
/// What this means becomes more obvious when translated to a real-life example:
///
/// > If Manhattan is located in New York, and New York is located in the USA, then Manhattan is located in the USA.
///
/// In this example, `LocatedIn` is the relationship and `Manhattan`, `New York` and `USA` are entities `A`, `B` and `C`.
/// Another common example of transitivity is found in OOP inheritance:
///
/// > If a Square is a Rectangle and a Rectangle is a Shape, then a Square is a Shape.
///
/// In this example `IsA` is the relationship and `Square`, `Rectangle` and `Shape` are the entities.
///
/// When relationships in Flecs are marked as transitive, queries can follow the transitive relationship to see
/// if an entity matches.
///
/// # Example
/// ```
/// # use flecs_ecs::prelude::*;
/// # let world = World::new();
/// let locatedin = world.entity();
/// let manhattan = world.entity();
/// let newyork = world.entity();
/// let usa = world.entity();
///
/// // Make the LocatedIn relationship transitive
/// locatedin.add_trait::<flecs::Transitive>();
///
/// manhattan.add((locatedin, newyork));
/// newyork.add((locatedin, usa));
///
/// // Now querying for (LocatedIn, USA) will match both NewYork and Manhattan
/// ```
///
/// If we were to query for `(LocatedIn, USA)` without making it transitive, we would only match `NewYork`,
/// because we never added `(LocatedIn, USA)` to `Manhattan`. By adding the transitive trait to the relationship
/// entity, queries will follow the `LocatedIn` relationship and return both `NewYork` and `Manhattan`.
#[derive(Debug, Default, Clone)]
pub struct Transitive;

impl_component_trait!(Transitive, ECS_TRANSITIVE);

/// Traversable relationships are allowed to be traversed automatically by queries, for example using the
/// `up` traversal (upwards traversal). Traversable relationships are also marked as [`Acyclic`], which ensures
/// a query won't accidentally attempt to traverse a relationship that contains cycles.
///
/// Events are propagated along the edges of traversable relationships. A typical example of this is when a
/// component value is changed on a prefab. The event of this change will be propagated by traversing the `IsA`
/// relationship downwards, for all instances of the prefab. Event propagation does not happen for relationships
/// that are not marked with `Traversable`.
#[derive(Debug, Default, Clone)]
pub struct Traversable;

impl_component_trait!(Traversable, ECS_TRAVERSABLE);

/// The `With` relationship can be added to components to indicate that it must always come together with
/// another component.
///
/// # Example - With regular components/tags
/// ```rust
/// # use flecs_ecs::prelude::*;
/// # let world = World::new();
/// let responsibility = world.entity();
/// let power = world.entity().add((flecs::With, responsibility));
///
/// // Create new entity that has both Power and Responsibility
/// let e = world.entity().add(power);
/// ```
///
/// When the `With` relationship is added to a relationship, the additional id added to the entity will be
/// a relationship pair as well, with the same target as the original relationship:
///
/// # Example - With relationships
/// ```rust
/// # use flecs_ecs::prelude::*;
/// # let world = World::new();
/// let likes = world.entity();
/// let loves = world.entity().add((flecs::With::id(), likes));
/// let pears = world.entity();
///
/// // Create new entity with both (Loves, Pears) and (Likes, Pears)
/// let e = world.entity().add((loves, pears));
/// ```
#[derive(Debug, Default, Clone)]
pub struct With;

impl_component_trait!(With, ECS_WITH);
