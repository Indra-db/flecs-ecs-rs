//! Entity and component access surface (spec §3).
//!
//! Split by how uniqueness is proven:
//!
//! * **Shared register** — `&World` and the `Copy` views ([`EntityView`](crate::core::EntityView),
//!   query iteration) only prove shared access, so component reads return RAII
//!   guards ([`Ref`] / [`Mut`]) backed by the per-stage lock maps. A conflicting
//!   borrow panics like a `RefCell`; the `try_*` entry points return the conflict
//!   as an [`AccessError`]. Repeated single-entity access resolves once into a
//!   [`CachedRef`].
//!
//! * **Exclusive register** — an operation that takes `&mut World`
//!   ([`WorldExclusiveExt::get_mut`], [`EntityMut`]) proves unique access at the
//!   type level, so no runtime lock is needed and plain `&T` / `&mut T` is
//!   returned.

pub(crate) mod exclusive;
pub use exclusive::WorldExclusiveExt;

pub mod entity_mut;
pub use entity_mut::EntityMut;

#[cfg(feature = "flecs_safety_locks")]
mod cached_ref;
#[cfg(feature = "flecs_safety_locks")]
mod entity_access;
#[cfg(feature = "flecs_safety_locks")]
mod guard;
#[cfg(feature = "flecs_safety_locks")]
mod singleton;

#[cfg(feature = "flecs_safety_locks")]
pub use cached_ref::{CachedRef, WorldEntityRefExt};
#[cfg(feature = "flecs_safety_locks")]
pub use entity_access::{EntityGuardExt, GuardElement, GuardTuple};
#[cfg(feature = "flecs_safety_locks")]
pub use guard::{AccessError, Mut, Ref};
#[cfg(feature = "flecs_safety_locks")]
pub use singleton::WorldSingletonExt;

/// Seals the `#[doc(hidden)]` kernel traits of the access surface (spec §9.5) so
/// downstream crates cannot implement them: [`GuardElement`] names
/// [`sealed::Sealed`] as a supertrait, and `Sealed` is only implemented for the
/// crate's own guard-element types.
#[cfg(feature = "flecs_safety_locks")]
pub(crate) mod sealed {
    use crate::core::ComponentOrPairId;

    /// Private supertrait that seals the access kernel traits.
    pub trait Sealed {}

    impl<T: ComponentOrPairId> Sealed for &T {}
    impl<T: ComponentOrPairId> Sealed for &mut T {}
    impl<T: ComponentOrPairId> Sealed for Option<&T> {}
    impl<T: ComponentOrPairId> Sealed for Option<&mut T> {}
}
