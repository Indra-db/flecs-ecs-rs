//! Tier-0 build-time disjointness proof.
//!
//! [`is_proven_disjoint`] answers, conservatively, whether a query's term set
//! can be shown at build time to never mutably alias itself: no two data terms
//! touch the same storage. The proof is intentionally strict — anything it
//! cannot cheaply certify fails — so a `true` result is always sound to rely on.
//!
//! # Where a proven result may be used
//!
//! Skipping the per-term borrow bookkeeping is only sound where no *outside*
//! borrow can exist either. That holds on the **exclusive register**
//! (`&mut World`), so [`each_exclusive`](super::QueryExclusiveExt::each_exclusive)
//! and [`chunks`](super::QueryChunksExt::chunks) use this proof to decide
//! whether the lock-free path is safe, falling back to the locked path (or
//! panicking, for `chunks`, which hands out `&mut` slices) when it is not.
//!
//! On the **shared register** an entity guard held across iteration is invisible
//! to this proof, so a proven-disjoint query must *still* register its term
//! borrows to detect guard-vs-query conflicts. There, tier-0 could at most drop
//! the intra-query conflict check between its own terms — which the batch fast
//! path (`acquire_batch_locks`) already skips for an empty map. So on the shared
//! path tier-0 buys nothing beyond batching, and no shared-path lock skip is
//! shipped.
//!
//! # What fails the proof (conservative rejections)
//!
//! * any term with a wildcard / `Any` id, or a pair with a wildcard element;
//! * any term whose source is not `$this` with `Self` traversal only
//!   (fixed-entity sources, `up` / `cascade` traversal, variables);
//! * any non-`And` term (`Or`, `Optional`, `Not`, `*From`);
//! * any sparse / `DontFragment` component (storage disjointness is not
//!   established by the id alone);
//! * any two data terms sharing the same concrete component id.
//!
//! Tag terms (no component data) cannot alias storage and are ignored.

use crate::core::{QueryDisjointCache, WorldProvider, WorldRef};
use crate::sys;

/// Return the disjointness verdict for `query`, reading it from `cache` when
/// present (spec §4.7) and computing it once on a cache miss. Transient
/// iterables (`QueryIter`, `ChainedIter`) have no cache and recompute per call.
#[inline]
pub(crate) fn is_proven_disjoint_cached(
    cache: Option<&QueryDisjointCache>,
    world: WorldRef<'_>,
    query: *const sys::ecs_query_t,
) -> bool {
    match cache {
        Some(cache) => cache.get_or_init(|| is_proven_disjoint(world, query)),
        None => is_proven_disjoint(world, query),
    }
}

/// Maximum number of data terms the fixed-size scratch buffer tracks. A query
/// with more terms than this fails the proof (conservative).
const MAX_TRACKED_TERMS: usize = 64;

/// Conservatively decide whether `query`'s terms provably never mutably alias.
///
/// A `true` result guarantees the query's data terms address pairwise-distinct,
/// dense, self-sourced storage, so iterating it lock-free (on the exclusive
/// register) cannot produce aliasing `&mut` access. `false` means "not proven"
/// — the query may still be disjoint in fact, but it could not be certified.
///
/// `query` must be a live query pointer for `world`, e.g. from
/// `IterOperations::query_ptr` (doc-hidden plumbing on the query types). A null
/// pointer
/// yields `false`.
#[allow(clippy::not_unsafe_ptr_arg_deref)]
pub fn is_proven_disjoint<'a>(world: impl WorldProvider<'a>, query: *const sys::ecs_query_t) -> bool {
    if query.is_null() {
        return false;
    }
    let world_ptr = world.world().real_world().world_ptr();
    // SAFETY: query is a valid pointer to a live query for the given world.
    let q = unsafe { &*query };
    let term_count = q.term_count as usize;
    if term_count > MAX_TRACKED_TERMS {
        return false;
    }
    // SAFETY: q.terms has term_count entries for this query.
    let terms = unsafe { core::slice::from_raw_parts(q.terms, term_count) };

    let mut seen: [u64; MAX_TRACKED_TERMS] = [0; MAX_TRACKED_TERMS];
    let mut n = 0usize;

    for term in terms {
        // Only plain conjunctive terms.
        if u32::from(term.oper as u16) != sys::ecs_oper_kind_t_EcsAnd {
            return false;
        }
        // Source must be $this, and traverse Self only (no up/cascade).
        // SAFETY: term points into the live query's term array.
        if !unsafe { sys::ecs_term_match_this(term) } {
            return false;
        }
        if term.src.id & (sys::EcsUp | sys::EcsCascade) != 0 {
            return false;
        }

        let id = term.id;
        if id == 0 {
            return false;
        }
        // SAFETY: id is a valid component id.
        if unsafe { sys::ecs_id_is_wildcard(id) } {
            return false;
        }

        // SAFETY: world_ptr is the live real world; id is valid.
        let typeid = unsafe { sys::ecs_get_typeid(world_ptr, id) };
        if typeid == 0 {
            // Tag / no-data term: cannot alias component storage.
            continue;
        }

        // Sparse / non-fragmenting storage is not certified disjoint by id.
        // SAFETY: world_ptr live; typeid valid; EcsSparse/EcsDontFragment are
        // builtin ids.
        if unsafe {
            sys::ecs_has_id(world_ptr, typeid, sys::EcsSparse)
                || sys::ecs_has_id(world_ptr, typeid, sys::EcsDontFragment)
        } {
            return false;
        }

        // Pairwise-distinct concrete ids.
        for &prev in &seen[..n] {
            if prev == id {
                return false;
            }
        }
        if n >= MAX_TRACKED_TERMS {
            return false;
        }
        seen[n] = id;
        n += 1;
    }

    true
}
