//! An id that represents either an entity, component, [`Query`], [`Observer`] or [`System`][crate::addons::system::System] in the [`World`].

use core::fmt::Display;
use core::ops::Deref;
use core::ops::{BitAnd, BitOr};

use crate::core::*;

/// An identifier that represents an entity.
///
/// An `Entity` is a numeric id that represents either an entity, component, query, observer or system in the world.
/// It does not represent a pair. See [`Id`] for that.
///
/// Entity ids consist of a number unique to the entity in the lower 32 bits,
/// and a generation counter used to track entity liveliness in the upper 32
/// bits. When an id is recycled, its generation count is increased. This
/// causes recycled ids to be very large (>4 billion), which is normal.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
pub struct Entity(pub u64);

impl Entity {
    #[inline(always)]
    pub fn new(id: u64) -> Self {
        Self(id)
    }

    #[inline(always)]
    pub fn null() -> Self {
        Self(0)
    }

    /// Returns `true` if the id is not the null (0) id.
    ///
    /// This is a pure numeric check: `Entity` carries no world reference, so this
    /// cannot detect dead, deleted or recycled entities. For a full liveliness
    /// check against a world, use [`EntityView::is_valid()`] or
    /// [`World::is_valid()`][crate::core::World::is_valid].
    #[inline(always)]
    pub fn is_valid(&self) -> bool {
        self.0 != 0
    }

    /// Convert the entity id to an [`EntityView`] with the given world.
    ///
    /// # Safety
    ///
    /// The caller must ensure this id was produced by `world`. Entity ids are
    /// only meaningful in the world that minted them: the same recycled `u64`
    /// can name a different, unrelated entity in another world, so binding an id
    /// to the wrong world causes type confusion reachable from otherwise-safe
    /// code. Use [`Entity::entity_view_checked`] for a validated conversion.
    ///
    /// Because it can bind an arbitrary id to a world, it is `unsafe`:
    ///
    /// ```compile_fail,E0133
    /// # use flecs_ecs::prelude::*;
    /// # let world = World::new();
    /// let id = Entity::new(1234);
    /// let _view = id.entity_view(&world); // requires `unsafe`
    /// ```
    ///
    /// # Arguments
    ///
    /// * `world` - The world the entity belongs to
    #[inline(always)]
    pub unsafe fn entity_view<'a>(&self, world: impl WorldProvider<'a>) -> EntityView<'a> {
        EntityView::new_from(world, *self)
    }

    /// Convert the entity id to an [`EntityView`], validating that the id is a
    /// live entity in `world` first.
    ///
    /// Returns `None` when the id is null or is not a valid entity in `world`,
    /// which makes this the safe counterpart of [`Entity::entity_view`] for ids
    /// whose owning world is not statically known.
    #[inline]
    pub fn entity_view_checked<'a>(
        &self,
        world: impl WorldProvider<'a>,
    ) -> Option<EntityView<'a>> {
        let world = world.world();
        if self.0 != 0 && unsafe { crate::sys::ecs_is_valid(world.world_ptr(), self.0) } {
            Some(EntityView::new_from(world, *self))
        } else {
            None
        }
    }

    /// Convert the entity id to an [`IdView`] with the given world.
    ///
    /// # Safety
    ///
    /// The caller must ensure this id was produced by `world`; see
    /// [`Entity::entity_view`] for why binding an id to the wrong world is
    /// unsound. Use [`Entity::id_view_checked`] for a validated conversion.
    ///
    /// # Arguments
    ///
    /// * `world` - The world the entity belongs to
    #[inline(always)]
    pub unsafe fn id_view<'a>(&self, world: impl WorldProvider<'a>) -> IdView<'a> {
        IdView::new_from_id(world, *self)
    }

    /// Convert the entity id to an [`IdView`], validating that the id is a live
    /// entity in `world` first. Returns `None` otherwise.
    #[inline]
    pub fn id_view_checked<'a>(&self, world: impl WorldProvider<'a>) -> Option<IdView<'a>> {
        let world = world.world();
        if self.0 != 0 && unsafe { crate::sys::ecs_is_valid(world.world_ptr(), self.0) } {
            Some(IdView::new_from_id(world, *self))
        } else {
            None
        }
    }
}

// impl ComponentInfo for Entity {
//     const IS_GENERIC: bool = false;
//     const IS_ENUM: bool = false;
//     const IS_TAG: bool = false;
//     const IMPLS_CLONE: bool = true;
//     const IMPLS_DEFAULT: bool = false;
//     const IS_REF: bool = false;
//     const IS_MUT: bool = false;
//     type TagType = FlecsNotATag;
// }

// impl ComponentId for Entity {
//     type UnderlyingType = Entity;
//     type UnderlyingEnumType = NoneEnum;
// type UnderlyingTypeOfEnum = NoneEnum;

//     #[inline(always)]
//     fn index() -> u32 {
//         static INDEX: core::sync::atomic::AtomicU32 = core::sync::atomic::AtomicU32::new(u32::MAX);
//         Self::get_or_init_index(&INDEX)
//     }

//     fn __register_or_get_id<'a, const MANUAL_REGISTRATION_CHECK: bool>(
//         _world: impl WorldProvider<'a>,
//     ) -> sys::ecs_entity_t {
//         // already registered by flecs in World
//         unsafe { sys::FLECS_IDecs_entity_tID_ }
//     }

//     #[inline]
//     fn __register_or_get_id_named<'a, const MANUAL_REGISTRATION_CHECK: bool>(
//         _world: impl WorldProvider<'a>,
//         _name: &str,
//     ) -> sys::ecs_entity_t {
//         // already registered by flecs in World
//         unsafe { sys::FLECS_IDecs_entity_tID_ }
//     }

//     #[inline]
//     fn is_registered_with_world<'a>(_: impl WorldProvider<'a>) -> bool {
//         //because this is always registered in the c world
//         true
//     }

//     #[inline]
//     fn id<'a>(_world: impl WorldProvider<'a>) -> sys::ecs_id_t {
//         //this is safe because it's already registered in flecs_c / world
//         unsafe { sys::FLECS_IDecs_entity_tID_ }
//     }
// }

impl Deref for Entity {
    type Target = u64;

    #[inline(always)]
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Display for Entity {
    #[inline]
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        self.0.fmt(f)
    }
}

mod bit_operations {
    use super::*;

    impl BitOr for Entity {
        type Output = Entity;

        #[inline]
        fn bitor(self, rhs: Self) -> Self::Output {
            Entity(self.0 | rhs.0)
        }
    }

    impl BitOr<u64> for Entity {
        type Output = Entity;

        #[inline]
        fn bitor(self, rhs: u64) -> Self::Output {
            Entity(self.0 | rhs)
        }
    }

    impl BitOr<Id> for Entity {
        type Output = Entity;

        #[inline]
        fn bitor(self, rhs: Id) -> Self::Output {
            Entity(self.0 | *rhs)
        }
    }

    impl BitOr<Entity> for u64 {
        type Output = Entity;

        #[inline]
        fn bitor(self, rhs: Entity) -> Self::Output {
            Entity(self | rhs.0)
        }
    }

    impl BitAnd for Entity {
        type Output = Entity;

        #[inline]
        fn bitand(self, rhs: Self) -> Self::Output {
            Entity(self.0 & rhs.0)
        }
    }

    impl BitAnd<u64> for Entity {
        type Output = Entity;

        #[inline]
        fn bitand(self, rhs: u64) -> Self::Output {
            Entity(self.0 & rhs)
        }
    }

    impl BitAnd<Id> for Entity {
        type Output = Entity;

        #[inline]
        fn bitand(self, rhs: Id) -> Self::Output {
            Entity(self.0 & *rhs)
        }
    }
}

mod from_operations {
    #[cfg(feature = "flecs_system")]
    use crate::prelude::system::System;

    use super::*;
    impl From<u64> for Entity {
        #[inline]
        fn from(id: u64) -> Self {
            Entity::new(id)
        }
    }

    impl<'a> From<EntityView<'a>> for Entity {
        #[inline]
        fn from(view: EntityView<'a>) -> Self {
            view.id
        }
    }

    impl<'a, T> From<Component<'a, T>> for Entity
    where
        T: ComponentId,
    {
        #[inline]
        fn from(component: Component<'a, T>) -> Self {
            component.base.entity.id
        }
    }

    impl<'a> From<UntypedComponent<'a>> for Entity {
        #[inline]
        fn from(component: UntypedComponent<'a>) -> Self {
            component.entity.id
        }
    }

    impl<T> From<Query<T>> for Entity
    where
        T: QueryTuple,
    {
        #[inline]
        fn from(query: Query<T>) -> Self {
            // SAFETY: `query.query` is a NonNull pointer to a live ecs_query_t that the Query keeps alive.
            Entity(unsafe { query.query.as_ref().entity })
        }
    }

    impl<'a> From<Observer<'a>> for Entity {
        #[inline]
        fn from(observer: Observer<'a>) -> Self {
            observer.id
        }
    }

    #[cfg(feature = "flecs_system")]
    impl<'a> From<System<'a>> for Entity {
        #[inline]
        fn from(system: System<'a>) -> Self {
            system.id
        }
    }

    impl From<Entity> for u64 {
        #[inline]
        fn from(id: Entity) -> Self {
            id.0
        }
    }
}

mod eq_operations {
    use super::*;

    impl PartialEq<Entity> for u64 {
        #[inline]
        fn eq(&self, other: &Entity) -> bool {
            self == &other.0
        }
    }

    impl PartialEq<u64> for Entity {
        #[inline]
        fn eq(&self, other: &u64) -> bool {
            &self.0 == other
        }
    }

    impl PartialEq<Id> for Entity {
        #[inline]
        fn eq(&self, other: &Id) -> bool {
            self.0 == other.0
        }
    }

    impl<'a> PartialEq<EntityView<'a>> for Entity {
        #[inline]
        fn eq(&self, other: &EntityView<'a>) -> bool {
            self.0 == other.id.0
        }
    }

    impl<'a> PartialEq<&EntityView<'a>> for Entity {
        #[inline]
        fn eq(&self, other: &&EntityView<'a>) -> bool {
            self.0 == other.id.0
        }
    }

    impl<'a> PartialEq<&mut EntityView<'a>> for Entity {
        #[inline]
        fn eq(&self, other: &&mut EntityView<'a>) -> bool {
            self.0 == other.id.0
        }
    }

    impl<'a> PartialEq<IdView<'a>> for Entity {
        #[inline]
        fn eq(&self, other: &IdView<'a>) -> bool {
            self.0 == other.id.0
        }
    }

    impl<'a, T> PartialEq<Component<'a, T>> for Entity
    where
        T: ComponentId,
    {
        #[inline]
        fn eq(&self, other: &Component<'a, T>) -> bool {
            self.0 == other.base.entity.id.0
        }
    }

    impl<'a> PartialEq<UntypedComponent<'a>> for Entity {
        #[inline]
        fn eq(&self, other: &UntypedComponent<'a>) -> bool {
            self.0 == other.entity.id.0
        }
    }
}

mod ord_operations {
    use super::*;

    impl PartialOrd<Entity> for u64 {
        #[inline]
        fn partial_cmp(&self, other: &Entity) -> Option<core::cmp::Ordering> {
            Some(self.cmp(&other.0))
        }
    }

    impl PartialOrd<u64> for Entity {
        #[inline]
        fn partial_cmp(&self, other: &u64) -> Option<core::cmp::Ordering> {
            Some(self.0.cmp(other))
        }
    }

    impl PartialOrd<Id> for Entity {
        #[inline]
        fn partial_cmp(&self, other: &Id) -> Option<core::cmp::Ordering> {
            Some(self.0.cmp(&other.0))
        }
    }

    impl<'a> PartialOrd<EntityView<'a>> for Entity {
        #[inline]
        fn partial_cmp(&self, other: &EntityView<'a>) -> Option<core::cmp::Ordering> {
            Some(self.0.cmp(&other.id.0))
        }
    }

    impl<'a> PartialOrd<IdView<'a>> for Entity {
        #[inline]
        fn partial_cmp(&self, other: &IdView<'a>) -> Option<core::cmp::Ordering> {
            Some(self.0.cmp(&other.id.0))
        }
    }

    impl<'a, T> PartialOrd<Component<'a, T>> for Entity
    where
        T: ComponentId,
    {
        #[inline]
        fn partial_cmp(&self, other: &Component<'a, T>) -> Option<core::cmp::Ordering> {
            Some(self.0.cmp(&other.base.entity.id.0))
        }
    }

    impl<'a> PartialOrd<UntypedComponent<'a>> for Entity {
        #[inline]
        fn partial_cmp(&self, other: &UntypedComponent<'a>) -> Option<core::cmp::Ordering> {
            Some(self.0.cmp(&other.entity.id.0))
        }
    }
}
