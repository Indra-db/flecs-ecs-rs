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
//!   See
//!   [`WorldExclusiveExt::get_exclusive`](crate::experimental::exclusive::WorldExclusiveExt::get_exclusive)
//!   and
//!   [`QueryExclusiveExt::each_exclusive`](crate::experimental::exclusive::QueryExclusiveExt::each_exclusive).
//!
//! * **Shared register** — `&World` and the Copy views (`EntityView`, query
//!   iteration) only prove shared access, so returns are RAII guards
//!   ([`Ref`](crate::experimental::Ref) / [`Mut`](crate::experimental::Mut)) backed by the per-stage lock maps in
//!   [`safety_map`](crate::core). A conflicting borrow panics like a `RefCell`;
//!   the `try_*` entry points return the conflict as an
//!   [`AccessError`](crate::experimental::AccessError).
//!
//! # Tier-0 disjointness (see [`disjoint`](crate::experimental::disjoint))
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
//!   [`disjoint::is_proven_disjoint`](crate::experimental::disjoint::is_proven_disjoint) for the (conservative) analysis and the
//!   adversarial cases it rejects.

/// Seals the experimental `#[doc(hidden)]` kernel traits (spec §9.5) so
/// downstream crates cannot implement them: each kernel trait names
/// [`sealed::Sealed`] as a supertrait, and `Sealed` is only implemented for the
/// crate's own blessed types.
pub(crate) mod sealed {
    /// Private supertrait that seals the experimental kernel traits.
    pub trait Sealed {}
}

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

pub mod batches;
pub mod bundle;
pub mod chunks;
pub mod disjoint;
pub mod entity_mut;
pub mod exclusive;
pub mod iter_ctx;
pub mod shared;
pub mod stage;

pub use batches::{LockedBatches, QueryBatchesExt};
pub use bundle::{Bundle, EntityBundleExt, WorldBundleExt};
pub use entity_mut::EntityMut;
pub use chunks::{ChunkCursor, EachArity, EachCursor, QueryChunksExt};
pub use disjoint::is_proven_disjoint;
pub use exclusive::{QueryExclusiveExt, WorldExclusiveExt};
pub use iter_ctx::{Iter, QueryIterCtxExt};
pub use shared::QuerySharedExt;
pub use stage::Stage;

/// Iterate a [`chunks`](QueryChunksExt::chunks) cursor with a fused per-row
/// loop, binding each row's components by the given names.
///
/// ```ignore
/// each!((pos, vel) in query.chunks(&mut world) {
///     pos.x += vel.x;
/// });
/// ```
///
/// The body is inlined textually into the row loop, so native `break`,
/// `continue`, and `?` all work as written. A `&mut T` column binds as
/// `&mut T` per row, a `&T` column as `&T`. The number of bound names must
/// match the number of query columns.
///
/// The inner row loop iterates the columns' native slice iterators zipped
/// together (`RowSlice::rows`), so the bounds check is hoisted out of the per-row
/// path and the loop advances by pointer, matching hand-written slice iteration.
///
/// # Binding-count vs column-count mismatch (spec §9.5)
///
/// A bound-name count that does not equal the query's column count is a
/// **named** compile error — `each!: the number of bound names does not equal
/// the query's column count` — not a raw tuple-destructure mismatch. The query
/// below has two columns but three bound names, so it fails to compile:
///
/// ```compile_fail
/// use flecs_ecs::prelude::*;
/// use flecs_ecs::experimental::prelude::*;
///
/// #[derive(Component)]
/// struct A { x: i32 }
/// #[derive(Component)]
/// struct B { y: i32 }
///
/// let mut world = World::new();
/// let q = world.new_query::<(&A, &B)>();
/// each!((a, b, c) in q.chunks(&mut world) {
///     let _ = (a, b, c);
/// });
/// ```
#[macro_export]
macro_rules! each {
    ( ( $($name:ident),+ $(,)? ) in $($rest:tt)+ ) => {
        $crate::each!(@tuple ( $($name),+ ) () $($rest)+)
    };
    ( $name:ident in $($rest:tt)+ ) => {
        $crate::each!(@single $name () $($rest)+)
    };

    // Tuple muncher: accumulate the cursor expression token-by-token until only
    // the trailing body block remains (an `expr` fragment cannot precede a
    // block, so the cursor cannot be captured as `:expr` directly).
    (@tuple ( $($name:ident),+ ) ( $($cur:tt)* ) { $($body:tt)* }) => {{
        // Named arity check (spec §9.5): a bound-name count that does not equal
        // the query's column count is a compile error with an `each!` message,
        // not a raw tuple-destructure mismatch. The cursor is threaded through a
        // zero-cost identity that binds its column count `N` at the type level;
        // the bound fails with the named message when it does not match.
        let mut __cursor = $($cur)*;
        $crate::experimental::chunks::arity_assert::<_, { $crate::each!(@count $($name),+) }>(
            &__cursor,
        );
        while let ::core::option::Option::Some(( $(mut $name,)+ )) =
            $crate::experimental::chunks::EachCursor::each_next(&mut __cursor)
        {
            // Zip the columns' native slice iterators so the row loop is a
            // bounds-check-free pointer walk. The zip nesting and its matching
            // destructure pattern are built by the same left fold.
            for $crate::each!(@pat $($name),+) in $crate::each!(@zip $($name),+) {
                { $($body)* }
            }
        }
    }};
    (@tuple ( $($name:ident),+ ) ( $($cur:tt)* ) $next:tt $($rest:tt)*) => {
        $crate::each!(@tuple ( $($name),+ ) ( $($cur)* $next ) $($rest)*)
    };

    // Single-binding muncher.
    (@single $name:ident ( $($cur:tt)* ) { $($body:tt)* }) => {{
        // Named arity check (spec §9.5): one bound name requires a one-column
        // query, bound via a zero-cost `&`-ref assert.
        let mut __cursor = $($cur)*;
        $crate::experimental::chunks::arity_assert::<_, 1>(&__cursor);
        while let ::core::option::Option::Some(mut $name) =
            $crate::experimental::chunks::EachCursor::each_next(&mut __cursor)
        {
            for $name in $crate::experimental::chunks::RowSlice::rows(&mut $name) {
                { $($body)* }
            }
        }
    }};
    (@single $name:ident ( $($cur:tt)* ) $next:tt $($rest:tt)*) => {
        $crate::each!(@single $name ( $($cur)* $next ) $($rest)*)
    };

    // Left fold: nest `RowSlice::rows(&mut a).zip(rows(&mut b)).zip(...)`.
    (@zip $first:ident $(, $rest:ident)*) => {
        $crate::each!(@zip_acc
            ( $crate::experimental::chunks::RowSlice::rows(&mut $first) )
            $($rest),*)
    };
    (@zip_acc ( $($acc:tt)* ) $next:ident $(, $rest:ident)*) => {
        $crate::each!(@zip_acc
            ( ($($acc)*).zip($crate::experimental::chunks::RowSlice::rows(&mut $next)) )
            $($rest),*)
    };
    (@zip_acc ( $($acc:tt)* )) => { $($acc)* };

    // Left fold: the destructure pattern matching the zip nesting, `(((a, b), c), d)`.
    (@pat $first:ident $(, $rest:ident)*) => {
        $crate::each!(@pat_acc ( $first ) $($rest),*)
    };
    (@pat_acc ( $($acc:tt)* ) $next:ident $(, $rest:ident)*) => {
        $crate::each!(@pat_acc ( ($($acc)*, $next) ) $($rest),*)
    };
    (@pat_acc ( $($acc:tt)* )) => { $($acc)* };

    // Count bound names into a `usize` for the arity check.
    (@count $first:ident $(, $rest:ident)*) => {
        1usize + $crate::each!(@count $($rest),*)
    };
    (@count) => { 0usize };
}

/// Convenience re-exports for the experimental surface.
pub mod prelude {
    #[cfg(feature = "flecs_safety_locks")]
    pub use super::cached_ref::{CachedRef, WorldEntityRefExt};
    #[cfg(feature = "flecs_safety_locks")]
    pub use super::entity_access::{EntityGuardExt, GuardTuple};
    pub use super::batches::QueryBatchesExt;
    pub use super::bundle::{Bundle, EntityBundleExt, WorldBundleExt};
    pub use super::entity_mut::EntityMut;
    pub use super::chunks::QueryChunksExt;
    pub use super::exclusive::{QueryExclusiveExt, WorldExclusiveExt};
    pub use super::iter_ctx::{Iter, QueryIterCtxExt};
    pub use super::shared::QuerySharedExt;
    pub use super::stage::Stage;
    #[cfg(feature = "flecs_safety_locks")]
    pub use super::guard::{AccessError, Mut, Ref};
    #[cfg(feature = "flecs_safety_locks")]
    pub use super::singleton::WorldSingletonExt;
    pub use crate::each;
}
