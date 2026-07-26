//! Resolved-once repeated single-entity access on the guard model (spec §3.7).
//!
//! [`CachedRef<T>`] is the experimental replacement for the legacy
//! [`core::CachedRef`](crate::core::CachedRef). It resolves one entity's `T`
//! once — caching the `ecs_ref_t`, the component id, and the storage lock key —
//! and then hands back the standard shared-register guards
//! ([`Ref`](super::Ref) / [`Mut`](super::Mut)) on each access.
//!
//! Unlike the legacy path, an access opens **no** flecs defer level: the pin
//! model (spec §7.1) tracks guard liveness Rust-side, so a warm [`get`] pays
//! only a table-version revalidation (`ecs_rust_ref_get`, the non-defer twin of
//! `ecs_rust_ref_get_scope_begin`) plus a pin increment and a lock-map begin.
//!
//! [`CachedRef`] is **never `Copy`** (a copied cache could outlive the table
//! revalidation contract) and is `!Send`/`!Sync` (it holds a raw `ecs_ref_t`).
//! [`get`](CachedRef::get) takes `&self` and yields a read guard, so several
//! read guards from one cache may coexist; [`get_mut`](CachedRef::get_mut) takes
//! `&mut self` (which also lets it refresh the cached table / column on an
//! archetype change) and yields a write guard, so the borrow checker forbids a
//! read guard and a write guard from the *same* cache at once. Conflicts between
//! *different* caches (or a cache and an entity guard) on the same storage are
//! caught at runtime by the stage lock map and panic like `RefCell`.

use core::marker::PhantomData;
use core::ptr::NonNull;

use crate::core::{
    alias_violation_panic, stage_locks_dyn, ComponentId, DataComponent, Entity, LockKey, World,
    WorldProvider,
};
use crate::sys;

use super::guard::{Mut, Ref};

/// A cache of one entity's `T` component for fast repeated shared-register
/// access (spec §3.7). Build one with
/// [`entity_ref`](WorldEntityRefExt::entity_ref); read through
/// [`get`](CachedRef::get) / [`get_mut`](CachedRef::get_mut).
///
/// Not `Copy`, not `Send`, not `Sync`.
pub struct CachedRef<T> {
    component_ref: sys::ecs_ref_t,
    component_id: sys::ecs_id_t,
    /// Cached storage lock key, valid while the entity stays in
    /// `lock_key_table_id`'s table; recomputed C-side on an archetype change.
    lock_key: LockKey,
    /// Table id the cached `lock_key` was computed for. `u64::MAX` forces a
    /// recompute on the next access.
    lock_key_table_id: u64,
    _marker: PhantomData<fn() -> T>,
}

impl<T: ComponentId + DataComponent> CachedRef<T> {
    /// Warm read: revalidate the entity's table by id, then hand back a
    /// [`Ref`] read guard (pin + lock, no defer FFI on the read path).
    ///
    /// Returns `None` if the entity moved tables and no longer has `T`, or is
    /// dead. Because this takes `&self` it does not persist a re-resolution, so
    /// after an archetype change it re-resolves each call (use
    /// [`get_mut`](CachedRef::get_mut) to refresh the cache).
    ///
    /// # Panics
    /// If a conflicting borrow (a live write guard) of the same storage is held,
    /// like [`RefCell::borrow`](core::cell::RefCell::borrow).
    #[inline]
    #[track_caller]
    pub fn get<'w>(&self, world: &'w World) -> Option<Ref<'w, T>> {
        let world_ptr = world.raw_world.as_ptr();
        // Copy the ref so a &self access never persists a re-resolution; the
        // cached record pointer is stable for the entity's lifetime, so
        // re-resolving from it stays correct (and warm-path cheap).
        let mut component_ref = self.component_ref;
        // SAFETY: world_ptr is a live world pointer and component_ref/id name a
        // valid ref for it.
        let get = unsafe {
            sys::ecs_rust_ref_get(
                world_ptr,
                &mut component_ref,
                self.component_id,
                self.lock_key_table_id,
            )
        };
        let ptr = NonNull::new(get.ptr.cast::<T>())?;
        let key = if get.lock_key != 0 {
            get.lock_key
        } else {
            self.lock_key
        };
        let world_ref = world.world();
        let locks = stage_locks_dyn(&world_ref);
        // SAFETY: the stage map is owned by the calling thread.
        if unsafe { (*locks.as_ptr()).read_begin(key) } {
            alias_violation_panic(&world_ref, self.component_id, false);
        }
        // SAFETY: as above. Balanced by the guard's Drop.
        unsafe { (*locks.as_ptr()).pin_inc() };
        // SAFETY: ptr is a live `T` kept valid for `'w` by the pin this guard
        // holds; the read borrow for `key` is registered and the pin taken.
        Some(unsafe {
            Ref::from_parts(
                ptr,
                NonNull::new_unchecked(world_ptr),
                locks,
                key,
                #[cfg(debug_assertions)]
                self.component_ref.entity,
                #[cfg(debug_assertions)]
                self.component_id,
            )
        })
    }

    /// Warm mutable access: revalidate by table id (refreshing the cached table
    /// / column on an archetype change, which the `&mut self` allows), then hand
    /// back a [`Mut`] write guard.
    ///
    /// Returns `None` if the entity moved tables and no longer has `T`, or is
    /// dead.
    ///
    /// # Panics
    /// If any conflicting borrow (a live read or write guard) of the same
    /// storage is held, like [`RefCell::borrow_mut`](core::cell::RefCell::borrow_mut).
    #[inline]
    #[track_caller]
    pub fn get_mut<'w>(&mut self, world: &'w World) -> Option<Mut<'w, T>> {
        let world_ptr = world.raw_world.as_ptr();
        // SAFETY: world_ptr is a live world pointer and self.component_ref/id
        // name a valid ref for it. Persisted: refreshes the ref's cache.
        let get = unsafe {
            sys::ecs_rust_ref_get(
                world_ptr,
                &mut self.component_ref,
                self.component_id,
                self.lock_key_table_id,
            )
        };
        let ptr = NonNull::new(get.ptr.cast::<T>())?;
        if get.lock_key != 0 {
            self.lock_key = get.lock_key;
            self.lock_key_table_id = self.component_ref.table_id;
        }
        let key = self.lock_key;
        let world_ref = world.world();
        let locks = stage_locks_dyn(&world_ref);
        // SAFETY: the stage map is owned by the calling thread.
        if unsafe { (*locks.as_ptr()).write_begin(key) } {
            alias_violation_panic(&world_ref, self.component_id, true);
        }
        // SAFETY: as above. Balanced by the guard's Drop.
        unsafe { (*locks.as_ptr()).pin_inc() };
        // SAFETY: ptr is a live, uniquely-borrowed `T` kept valid for `'w` by
        // the pin; the write borrow for `key` is registered and the pin taken.
        Some(unsafe {
            Mut::from_parts(
                ptr,
                NonNull::new_unchecked(world_ptr),
                locks,
                key,
                #[cfg(debug_assertions)]
                self.component_ref.entity,
                #[cfg(debug_assertions)]
                self.component_id,
            )
        })
    }

    /// The entity this cache refers to.
    #[inline]
    pub fn entity(&self) -> Entity {
        Entity(self.component_ref.entity)
    }
}

/// Build a [`CachedRef`] for repeated single-entity access on the guard model
/// (spec §3.7).
pub trait WorldEntityRefExt {
    /// Resolve `entity`'s `T` once, caching the ref, component id, and storage
    /// lock key. Returns `None` if the entity is not alive or does not have
    /// `T`.
    fn entity_ref<T: ComponentId + DataComponent>(
        &self,
        entity: impl Into<Entity>,
    ) -> Option<CachedRef<T>>;
}

impl WorldEntityRefExt for World {
    fn entity_ref<T: ComponentId + DataComponent>(
        &self,
        entity: impl Into<Entity>,
    ) -> Option<CachedRef<T>> {
        let world_ref = self.world();
        // ecs_ref_* operations require the real world, not a stage.
        let world_ptr = unsafe {
            sys::ecs_get_world(world_ref.world_ptr_mut() as *const _) as *mut sys::ecs_world_t
        };
        let entity = *entity.into();
        if !unsafe { sys::ecs_is_alive(world_ptr, entity) } {
            return None;
        }
        let component_id = T::entity_id(world_ref);
        let mut component_ref = unsafe { sys::ecs_ref_init_id(world_ptr, entity, component_id) };
        if component_ref.entity == 0 {
            return None;
        }
        // Resolve once to confirm `T` is present and cache the storage key.
        // `u64::MAX` forces the key recompute (no table can have that id).
        let get = unsafe {
            sys::ecs_rust_ref_get(world_ptr, &mut component_ref, component_id, u64::MAX)
        };
        if get.ptr.is_null() {
            return None;
        }
        let (lock_key, lock_key_table_id) = if get.lock_key != 0 {
            (get.lock_key, component_ref.table_id)
        } else {
            (0, u64::MAX)
        };
        Some(CachedRef {
            component_ref,
            component_id,
            lock_key,
            lock_key_table_id,
            _marker: PhantomData,
        })
    }
}
