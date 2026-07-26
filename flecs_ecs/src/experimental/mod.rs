//! Prototype of the next-generation flecs Rust API (feature `flecs_experimental`).
//!
//! # Two-register model
//!
//! Component access is split by how uniqueness is proven:
//!
//! * **Exclusive register** — an operation that takes `&mut World` proves
//!   unique access to the whole world at the type level, so no runtime lock is
//!   needed and plain `&T` / `&mut T` can be returned. The borrow checker then
//!   forbids every other world access for as long as the reference is live.
//!   See [`WorldExclusiveExt::get_exclusive`] and
//!   [`QueryExclusiveExt::each_exclusive`].
//!
//! * **Shared register** — `&World` and the Copy views (`EntityView`, query
//!   iteration) only prove shared access, so returns are RAII guards
//!   ([`Ref`] / [`Mut`]) backed by the per-stage lock maps in
//!   [`safety_map`](crate::core). A conflicting borrow panics like a `RefCell`;
//!   the `try_*` entry points return the conflict as an [`AccessError`].
//!
//! # Tier-0 disjointness (see [`disjoint`])
//!
//! A query whose terms provably cannot alias can skip the intra-query per-term
//! borrow bookkeeping. Crucially this is only *sound where no outside borrow
//! can exist*:
//!
//! * On the **exclusive register** (`each_exclusive`, `&mut World`) there can be
//!   no outstanding entity guard, so a proven-disjoint query can skip lock
//!   traffic entirely.
//! * On the **shared register** a concurrently held entity guard is invisible
//!   to the query's build-time proof, so the query must still register its term
//!   borrows so that guard-vs-query conflicts are detected. The proof there can
//!   only drop the *intra-query* conflict check between its own terms — which
//!   the batch fast path
//!   ([`acquire_batch_locks`](crate::core)) already skips for an empty map.
//!   So on the shared path tier-0 buys nothing beyond what batching already
//!   does, and this prototype does not ship a shared-path skip. See
//!   [`disjoint::is_proven_disjoint`] for the (conservative) analysis and the
//!   adversarial cases it rejects.

#[cfg(feature = "flecs_safety_locks")]
mod entity_access;
#[cfg(feature = "flecs_safety_locks")]
mod guard;

#[cfg(feature = "flecs_safety_locks")]
pub use entity_access::{EntityGuardExt, GuardElement, GuardTuple};
#[cfg(feature = "flecs_safety_locks")]
pub use guard::{AccessError, Mut, Ref};

/// Convenience re-exports for the experimental surface.
pub mod prelude {
    #[cfg(feature = "flecs_safety_locks")]
    pub use super::entity_access::{EntityGuardExt, GuardTuple};
    #[cfg(feature = "flecs_safety_locks")]
    pub use super::guard::{AccessError, Mut, Ref};
}
