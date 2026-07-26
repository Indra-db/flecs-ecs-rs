//! Ids are the things that can be added to an entity. They can represent either an [`Entity`] or an ECS relationship pair id.

use core::{
    fmt::Display,
    ops::{BitAnd, BitOr, Deref},
};

use crate::core::*;

/// An Identifier for what could represent either what [`Entity`]
/// as well as an ECS relationship pair and can have optional id flags.
/// Ids are the things that can be added to an entity.
#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq, PartialOrd, Ord)]
#[repr(transparent)]
pub struct Id(pub(crate) u64);

impl Id {
    #[inline]
    pub fn new(id: u64) -> Self {
        Self(id)
    }

    /// Convert the id to an [`EntityView`] with the given world.
    ///
    /// # Safety
    ///
    /// The caller must ensure this id was produced by `world`. Ids are only
    /// meaningful in the world that minted them: the same recycled `u64` can
    /// name a different entity in another world, so binding an id to the wrong
    /// world causes type confusion reachable from otherwise-safe code. Use
    /// [`Id::entity_view_checked`] for a validated conversion.
    ///
    /// # Arguments
    ///
    /// * `world` - The world the entity belongs to
    #[inline]
    pub unsafe fn entity_view<'a>(&self, world: impl WorldProvider<'a>) -> EntityView<'a> {
        EntityView::new_from(world, self.0)
    }

    /// Convert the id to an [`EntityView`], validating that it is a live entity
    /// in `world` first. Returns `None` when the id is null or not valid in
    /// `world`.
    #[inline]
    pub fn entity_view_checked<'a>(
        &self,
        world: impl WorldProvider<'a>,
    ) -> Option<EntityView<'a>> {
        let world = world.world();
        if self.0 != 0 && unsafe { crate::sys::ecs_is_valid(world.world_ptr(), self.0) } {
            Some(EntityView::new_from(world, self.0))
        } else {
            None
        }
    }
}

impl Deref for Id {
    type Target = u64;

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Display for Id {
    #[inline]
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        self.0.fmt(f)
    }
}

mod bit_operations {
    use super::*;

    impl BitOr for Id {
        type Output = Id;

        #[inline]
        fn bitor(self, rhs: Self) -> Self::Output {
            Id(self.0 | rhs.0)
        }
    }

    impl BitOr<u64> for Id {
        type Output = Id;

        #[inline]
        fn bitor(self, rhs: u64) -> Self::Output {
            Id(self.0 | rhs)
        }
    }

    impl BitOr<Entity> for Id {
        type Output = Id;

        #[inline]
        fn bitor(self, rhs: Entity) -> Self::Output {
            Id(self.0 | *rhs)
        }
    }

    impl BitOr<Id> for u64 {
        type Output = Id;

        #[inline]
        fn bitor(self, rhs: Id) -> Self::Output {
            Id(self | rhs.0)
        }
    }

    impl BitAnd for Id {
        type Output = Id;

        #[inline]
        fn bitand(self, rhs: Self) -> Self::Output {
            Id(self.0 & rhs.0)
        }
    }

    impl BitAnd<u64> for Id {
        type Output = Id;

        #[inline]
        fn bitand(self, rhs: u64) -> Self::Output {
            Id(self.0 & rhs)
        }
    }

    impl BitAnd<Entity> for Id {
        type Output = Id;

        #[inline]
        fn bitand(self, rhs: Entity) -> Self::Output {
            Id(self.0 & *rhs)
        }
    }
}
mod from_operations {
    use super::*;

    impl From<u64> for Id {
        #[inline]
        fn from(id: u64) -> Self {
            Id::new(id)
        }
    }

    impl From<Entity> for Id {
        #[inline]
        fn from(id: Entity) -> Self {
            Id(*id)
        }
    }

    impl<'a> From<EntityView<'a>> for Id {
        #[inline]
        fn from(view: EntityView<'a>) -> Self {
            view.id.into()
        }
    }

    impl<'a> From<IdView<'a>> for Id {
        #[inline]
        fn from(view: IdView<'a>) -> Self {
            view.id
        }
    }

    impl<'a, T> From<Component<'a, T>> for Id
    where
        T: ComponentId,
    {
        #[inline]
        fn from(component: Component<'a, T>) -> Self {
            component.base.entity.id.into()
        }
    }

    impl<'a> From<UntypedComponent<'a>> for Id {
        #[inline]
        fn from(component: UntypedComponent<'a>) -> Self {
            component.entity.id.into()
        }
    }
}
mod eq_operations {
    use super::*;

    impl PartialEq<Id> for u64 {
        #[inline]
        fn eq(&self, other: &Id) -> bool {
            self == &other.0
        }
    }

    impl PartialEq<u64> for Id {
        #[inline]
        fn eq(&self, other: &u64) -> bool {
            &self.0 == other
        }
    }

    impl PartialEq<Entity> for Id {
        #[inline]
        fn eq(&self, other: &Entity) -> bool {
            self.0 == other.0
        }
    }

    impl<'a> PartialEq<EntityView<'a>> for Id {
        #[inline]
        fn eq(&self, other: &EntityView<'a>) -> bool {
            self.0 == other.id.0
        }
    }

    impl<'a> PartialEq<IdView<'a>> for Id {
        #[inline]
        fn eq(&self, other: &IdView<'a>) -> bool {
            self.0 == other.id.0
        }
    }

    impl<'a, T> PartialEq<Component<'a, T>> for Id
    where
        T: ComponentId,
    {
        #[inline]
        fn eq(&self, other: &Component<'a, T>) -> bool {
            self.0 == other.base.entity.id.0
        }
    }

    impl<'a> PartialEq<UntypedComponent<'a>> for Id {
        #[inline]
        fn eq(&self, other: &UntypedComponent<'a>) -> bool {
            self.0 == other.entity.id.0
        }
    }
}
mod ord_operations {
    use super::*;

    impl PartialOrd<Id> for u64 {
        #[inline]
        fn partial_cmp(&self, other: &Id) -> Option<core::cmp::Ordering> {
            self.partial_cmp(&other.0)
        }
    }

    impl PartialOrd<u64> for Id {
        #[inline]
        fn partial_cmp(&self, other: &u64) -> Option<core::cmp::Ordering> {
            self.0.partial_cmp(other)
        }
    }

    impl PartialOrd<Entity> for Id {
        #[inline]
        fn partial_cmp(&self, other: &Entity) -> Option<core::cmp::Ordering> {
            self.0.partial_cmp(&other.0)
        }
    }

    impl<'a> PartialOrd<EntityView<'a>> for Id {
        #[inline]
        fn partial_cmp(&self, other: &EntityView<'a>) -> Option<core::cmp::Ordering> {
            self.0.partial_cmp(&other.id.0)
        }
    }

    impl<'a> PartialOrd<IdView<'a>> for Id {
        #[inline]
        fn partial_cmp(&self, other: &IdView<'a>) -> Option<core::cmp::Ordering> {
            self.0.partial_cmp(&other.id.0)
        }
    }

    impl<'a, T> PartialOrd<Component<'a, T>> for Id
    where
        T: ComponentId,
    {
        #[inline]
        fn partial_cmp(&self, other: &Component<'a, T>) -> Option<core::cmp::Ordering> {
            self.0.partial_cmp(&other.base.entity.id.0)
        }
    }

    impl<'a> PartialOrd<UntypedComponent<'a>> for Id {
        #[inline]
        fn partial_cmp(&self, other: &UntypedComponent<'a>) -> Option<core::cmp::Ordering> {
            self.0.partial_cmp(&other.entity.id.0)
        }
    }
}
