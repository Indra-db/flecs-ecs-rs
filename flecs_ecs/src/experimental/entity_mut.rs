//! Exclusive-register entity handle: [`EntityMut`] (spec §2.2, §3.5).
//!
//! [`EntityMut<'w>`] is the exclusive-register twin of [`EntityView`]. It is
//! obtained from `&mut World` ([`World::entity_new`] / [`World::entity_mut`]),
//! so the borrow checker already proves that no other access to the world can
//! exist for the handle's lifetime. That proof is what lets every operation run
//! **zero-lock**: component reads hand back plain `&T` / `&mut T` (no
//! [`Ref`](crate::experimental::Ref) / [`Mut`](crate::experimental::Mut) guard,
//! no stage-lock bookkeeping, no pin), and structural ops (`set` / `add` /
//! `remove` / `insert`) apply **immediately** rather than being deferred behind
//! a guard episode.
//!
//! ## The exclusivity argument
//!
//! [`EntityMut<'w>`] carries a `PhantomData<&'w mut World>`, so it is `!Copy`,
//! `!Send`, `!Sync`, and **invariant** in `'w`. Because [`World::entity_new`] /
//! [`World::entity_mut`] take `&'w mut self` and return `EntityMut<'w>`, the
//! `&mut World` borrow stays live for as long as the handle is used (ordinary
//! NLL reborrow extension). While that borrow is live the borrow checker
//! excludes:
//!
//! * every `&self` / `&mut self` method on the same [`World`] — including
//!   [`World::entity`], [`World::entity_new`], guard acquisition, and query
//!   iteration — so no other handle into this world's storage can coexist;
//! * a second [`EntityMut`] / exclusive borrow of the same world.
//!
//! It does **not** exclude a *different* [`World`] (a different borrow), but that
//! world owns different storage and cannot alias this one's components. Nor does
//! it attempt to exclude raw-pointer aliasing produced by `unsafe` code, which
//! is already outside the safe API. This is exactly the guarantee validated by
//! [`WorldExclusiveExt::get_exclusive`](crate::experimental::exclusive::WorldExclusiveExt::get_exclusive).

use core::marker::PhantomData;

use crate::core::{
    ComponentId, ComponentOrPairId, ComponentType, DataComponent, Entity, EntityView, GetComponentPointers,
    GetTuple, IntoEntity, IntoId, Struct, World, WorldProvider, WorldRef,
};
use crate::sys;

use super::bundle::{Bundle, EntityBundleExt};

/// Exclusive-register handle to a live entity (spec §2.2, §3.5).
///
/// `!Copy` / `!Clone`: it carries the `&mut World` borrow that proves exclusive
/// access, so it cannot be duplicated. Obtain one from [`World::entity_new`]
/// (create) or [`World::entity_mut`] (an existing, live entity). Every operation
/// is lock-free and structural ops are immediate; see the module docs for the
/// exclusivity argument.
pub struct EntityMut<'w> {
    world: WorldRef<'w>,
    id: Entity,
    /// Ties the handle to the originating `&'w mut World` borrow: invariant in
    /// `'w`, and `!Copy` / `!Send` / `!Sync`. This is the whole exclusivity
    /// proof (module docs).
    _exclusive: PhantomData<&'w mut World>,
}

impl<'w> EntityMut<'w> {
    /// # Safety
    /// `world` and `id` must name the same live world, and the caller must hold
    /// the `&'w mut World` borrow this handle stands in for (so no other access
    /// to that world can exist for `'w`). Constructed only by
    /// [`World::entity_new`] / [`World::entity_mut`].
    #[inline(always)]
    pub(crate) unsafe fn new(world: WorldRef<'w>, id: Entity) -> Self {
        Self {
            world,
            id,
            _exclusive: PhantomData,
        }
    }

    /// The entity id this handle refers to. Copying the id out of the exclusive
    /// borrow is how callers hand the entity to a later `&World` access.
    #[inline(always)]
    pub fn id(&self) -> Entity {
        self.id
    }

    /// A shared [`EntityView`] over the same entity, borrowing `self`. Useful for
    /// the read-only, closure, and relationship helpers that live on the shared
    /// view; it cannot outlive the exclusive borrow.
    #[inline(always)]
    pub fn entity_view(&self) -> EntityView<'_> {
        EntityView {
            world: self.world,
            id: self.id,
        }
    }

    /// A fresh Copy [`EntityView`] with the handle's own `'w`. Private: used only
    /// to delegate the immediate structural ops to the shared-view setters, whose
    /// write-episode hook is a no-op here (no guard can be live on the exclusive
    /// register, so the pin is zero and every op applies immediately).
    #[inline(always)]
    fn view(&self) -> EntityView<'w> {
        EntityView {
            world: self.world,
            id: self.id,
        }
    }

    /// Borrow a component immutably. Returns a plain `&T` tied to `self` (no
    /// guard, no lock: the `&mut World` behind this handle is the proof).
    /// `None` if the component is absent (the entity is always live for an
    /// `EntityMut`).
    #[inline]
    pub fn get<T>(&self) -> Option<&<T as ComponentOrPairId>::CastType>
    where
        T: ComponentOrPairId + DataComponent,
    {
        let world_ptr = self.world.world_ptr_mut();
        let record = unsafe { sys::ecs_record_find(world_ptr, *self.id) };
        if record.is_null() {
            return None;
        }
        // SAFETY: record was just looked up for `self.id` on this world.
        let data = unsafe { <&T as GetTuple>::create_ptrs::<false>(self.world, self.id, record) };
        let ptr = data.component_ptrs()[0];
        if ptr.is_null() {
            return None;
        }
        // SAFETY: ptr is a valid pointer to the component; the returned reference
        // borrows `self`, which holds the exclusive `&mut World`, so nothing can
        // alias it for the borrow.
        Some(unsafe { &*(ptr as *const <T as ComponentOrPairId>::CastType) })
    }

    /// Borrow a component mutably. Returns a plain `&mut T` tied to `self` (no
    /// guard, no lock). `None` if the component is absent.
    #[inline]
    pub fn get_mut<T>(&mut self) -> Option<&mut <T as ComponentOrPairId>::CastType>
    where
        T: ComponentOrPairId + DataComponent,
    {
        let world_ptr = self.world.world_ptr_mut();
        let record = unsafe { sys::ecs_record_find(world_ptr, *self.id) };
        if record.is_null() {
            return None;
        }
        // SAFETY: record was just looked up for `self.id` on this world.
        let data = unsafe { <&mut T as GetTuple>::create_ptrs::<false>(self.world, self.id, record) };
        let ptr = data.component_ptrs()[0];
        if ptr.is_null() {
            return None;
        }
        // SAFETY: ptr is a valid, uniquely-owned pointer; the returned reference
        // borrows `&mut self` (the exclusive `&mut World`), so nothing aliases it.
        Some(unsafe { &mut *(ptr as *mut <T as ComponentOrPairId>::CastType) })
    }

    /// Fused multi-component exclusive borrow (spec §3.5): resolve every element
    /// of the tuple in one call, with **zero locks** (the exclusive borrow is the
    /// proof). `get_many::<(&mut A, &B)>()` yields `Option<(&mut A, &B)>`. A
    /// non-optional element that is absent makes the whole call `None`; optional
    /// elements (`Option<&T>` / `Option<&mut T>`) yield `None` in place.
    ///
    /// ## Duplicate-mutable rejection
    ///
    /// Requesting the same component mutably twice (`(&mut A, &mut A)`), or once
    /// mutably and once shared, would alias `&mut` storage with no lock to catch
    /// it. This is rejected: the fused resolver runs the crate's
    /// `static_alias_conflict` duplicate-mutable check, keyed on each term's real
    /// `TypeId`, as the resolution's first step. The check is fully determined by
    /// the requested
    /// types, so after monomorphization it folds to a constant and, on a
    /// duplicate, to an unconditional panic before any pointer is handed out. The
    /// following therefore panics deterministically (the message names the
    /// duplicated component):
    ///
    /// ```should_panic
    /// use flecs_ecs::prelude::*;
    /// use flecs_ecs::experimental::prelude::*;
    ///
    /// #[derive(Component)]
    /// struct A { x: i32 }
    ///
    /// let mut world = World::new();
    /// let mut e = world.entity_new();
    /// e.set(A { x: 1 });
    /// // Same component requested mutably twice: rejected before any borrow.
    /// let _ = e.get_many::<(&mut A, &mut A)>();
    /// ```
    ///
    /// A true compile-time rejection (`TypeId` / `type_name` compared in a
    /// `const` context) is not expressible on the pinned stable toolchain, so the
    /// duplicate-mutable check is enforced at monomorphization as the sound
    /// const-folded panic above, matching the shared-register `get`
    /// (`create_ptrs`) convention.
    #[inline]
    pub fn get_many<G: GetTuple>(&mut self) -> Option<G::TupleType<'_>> {
        let world_ptr = self.world.world_ptr_mut();
        let record = unsafe { sys::ecs_record_find(world_ptr, *self.id) };
        if record.is_null() {
            return None;
        }
        // SAFETY: record was just looked up for `self.id` on this world. This
        // runs the static duplicate-mutable check (see doc comment) and resolves
        // every element's pointer with no lock.
        let data = unsafe { G::create_ptrs::<false>(self.world, self.id, record) };
        if !data.has_all_components() {
            return None;
        }
        // The references borrow the entity's storage for `&mut self`; the
        // exclusive `&mut World` keeps them unaliased and unmoved.
        Some(data.get_tuple())
    }

    /// Set a component immediately (spec §3.5). Runs live-storage mutation each
    /// call; returns `&mut Self` for chaining.
    #[inline]
    pub fn set<T: ComponentId>(&mut self, value: T) -> &mut Self {
        self.view().set(value);
        self
    }

    /// Set a component under an explicit id (component or pair) immediately.
    #[inline]
    pub fn set_id<T>(&mut self, value: T, id: impl IntoId) -> &mut Self
    where
        T: ComponentId + DataComponent,
    {
        self.view().set_id(value, id);
        self
    }

    /// Add a component, tag, pair, or entity id immediately.
    #[inline]
    pub fn add<T: IntoId>(&mut self, id: T) -> &mut Self {
        self.view().add(id);
        self
    }

    /// Remove a component, tag, pair, or entity id immediately.
    #[inline]
    pub fn remove<T: IntoId>(&mut self, id: T) -> &mut Self {
        self.view().remove(id);
        self
    }

    /// Set a pair value using both element types (spec GAP-10, exclusive side).
    #[inline]
    pub fn set_pair<First, Second>(
        &mut self,
        data: <(First, Second) as ComponentOrPairId>::CastType,
    ) -> &mut Self
    where
        First: ComponentId,
        Second: ComponentId,
        (First, Second): ComponentOrPairId,
    {
        self.view().set_pair::<First, Second>(data);
        self
    }

    /// Set a pair value using the first element type and a second id.
    #[inline]
    pub fn set_first<First>(&mut self, first: First, second: impl IntoEntity) -> &mut Self
    where
        First: ComponentId + DataComponent,
    {
        self.view().set_first(first, second);
        self
    }

    /// Set a pair value using a first id and the second element type.
    #[inline]
    pub fn set_second<Second>(&mut self, first: impl IntoEntity, second: Second) -> &mut Self
    where
        Second: ComponentId + ComponentType<Struct> + DataComponent,
    {
        self.view().set_second(first, second);
        self
    }

    /// Insert a whole [`Bundle`] in a single archetype move (spec §3.5, §4.11).
    ///
    /// Reuses the bundle machinery's immediate `ecs_commit` path unconditionally:
    /// no guard can be live on the exclusive register, so there is no guard
    /// episode to check and the move always applies now.
    #[inline]
    pub fn insert<B: Bundle>(&mut self, bundle: B) -> &mut Self {
        self.view().insert(bundle);
        self
    }
}

/// `&mut World` entry points for the exclusive register (spec §3.5). Provisional
/// home; these are the intended final inherent `World` methods.
impl World {
    /// Create a new entity and return it as an [`EntityMut`] for immediate
    /// construction (spec §3.5). The returned handle holds the `&mut World`
    /// borrow, so structural ops on it apply immediately and read back at once.
    ///
    /// ```
    /// use flecs_ecs::prelude::*;
    /// use flecs_ecs::experimental::prelude::*;
    ///
    /// #[derive(Component)]
    /// struct Position { x: i32, y: i32 }
    ///
    /// let mut world = World::new();
    /// let mut e = world.entity_new();
    /// e.set(Position { x: 1, y: 2 });
    /// assert_eq!(e.get::<Position>().unwrap().x, 1);
    /// ```
    #[inline]
    pub fn entity_new(&mut self) -> EntityMut<'_> {
        let world_ptr = self.raw_world.as_ptr();
        let id = unsafe { sys::ecs_new(world_ptr) };
        // SAFETY: `world_ptr` is this world's live pointer; we hold `&mut self`
        // for the returned handle's lifetime, and `id` was just created live.
        let world = unsafe { WorldRef::from_ptr(world_ptr) };
        unsafe { EntityMut::new(world, Entity::new(id)) }
    }

    /// Return an [`EntityMut`] for an existing entity, or `None` if it is not
    /// alive (spec §3.5).
    ///
    /// ```
    /// use flecs_ecs::prelude::*;
    /// use flecs_ecs::experimental::prelude::*;
    ///
    /// let mut world = World::new();
    /// let e = world.entity_new().id();
    /// assert!(world.entity_mut(e).is_some());
    /// world.entity_from_id(e).destruct();
    /// assert!(world.entity_mut(e).is_none());
    /// ```
    #[inline]
    pub fn entity_mut(&mut self, entity: impl Into<Entity>) -> Option<EntityMut<'_>> {
        let entity: Entity = entity.into();
        let world_ptr = self.raw_world.as_ptr();
        if !unsafe { sys::ecs_is_alive(world_ptr, *entity) } {
            return None;
        }
        // SAFETY: `world_ptr` is this world's live pointer; `&mut self` is held
        // for the returned handle's lifetime; `entity` was just checked alive.
        let world = unsafe { WorldRef::from_ptr(world_ptr) };
        Some(unsafe { EntityMut::new(world, entity) })
    }
}
