pub use crate::addons::*;
pub use crate::core::*;
// Guard types: explicit re-export so `Mut` resolves to the access guard, not the
// unused `table::multi_src_get::Mut` query-field marker in the `core::*` glob.
#[cfg(feature = "flecs_safety_locks")]
pub use crate::core::access::{Mut, Ref};
pub use flecs_ecs_derive::*;
pub use flecs_ecs_sys::EcsComponent;

#[cfg(feature = "flecs_meta")]
pub use crate::{assert_is_type, component, component_ext, member, member_ext};
