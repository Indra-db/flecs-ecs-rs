use crate::core::*;
use crate::sys;

#[cfg(feature = "flecs_safety_locks")]
use core::ptr::NonNull;

#[cfg(feature = "flecs_safety_locks")]
#[inline(always)]
fn lock_target_begin(
    world: &WorldRef,
    locks: NonNull<StageLocks>,
    si: &SafetyInfo,
) {
    match si {
        SafetyInfo::Read(si) => {
            if !si.cr.is_null() {
                sparse_id_record_lock_read_begin(world, locks, si.cr);
            } else {
                get_table_column_lock_read_begin(world, locks, si.table, si.column_index);
            }
        }
        SafetyInfo::Write(si) => {
            if !si.cr.is_null() {
                sparse_id_record_lock_write_begin(world, locks, si.cr);
            } else {
                get_table_column_lock_write_begin(world, locks, si.table, si.column_index);
            }
        }
    }
}

#[cfg(feature = "flecs_safety_locks")]
#[inline(always)]
fn lock_target_end(locks: NonNull<StageLocks>, si: &SafetyInfo) {
    match si {
        SafetyInfo::Read(si) => {
            if !si.cr.is_null() {
                sparse_id_record_lock_read_end(locks, si.cr);
            } else {
                table_column_lock_read_end(locks, si.table, si.column_index);
            }
        }
        SafetyInfo::Write(si) => {
            if !si.cr.is_null() {
                sparse_id_record_lock_write_end(locks, si.cr);
            } else {
                table_column_lock_write_end(locks, si.table, si.column_index);
            }
        }
    }
}

pub(crate) fn rw_locking<T: GetTuple, Return, const MULTITHREADED: bool>(
    world: &WorldRef,
    callback: impl FnOnce(<T as GetTuple>::TupleType<'_>) -> Return,
    tuple_data: <T as GetTuple>::Pointers,
    tuple: <T as GetTuple>::TupleType<'_>,
) -> Return {
    let components = tuple_data.component_ptrs();
    let safety_info = tuple_data.safety_info();
    // Resolve the stage map from the calling context's world: inside a
    // multithreaded system callback this is the stage world, so each worker
    // uses its own stage map.
    let locks = stage_locks::<MULTITHREADED>(world);
    let world = world.real_world();

    for (index, si) in safety_info.iter().enumerate() {
        if unsafe { components.get_unchecked(index).is_null() } {
            continue;
        }
        lock_target_begin(&world, locks, si);
    }

    world.defer_begin();
    let ret = callback(tuple);
    world.defer_end();

    for (index, si) in safety_info.iter().enumerate() {
        if unsafe { components.get_unchecked(index).is_null() } {
            continue;
        }
        lock_target_end(locks, si);
    }
    ret
}

#[cfg(feature = "flecs_safety_locks")]
#[inline(always)]
pub(crate) fn clone_locking<const MULTITHREADED: bool>(
    world: WorldRef<'_>,
    components: &[*mut core::ffi::c_void],
    safety_info: &[sys::ecs_rust_lock_target_t],
) {
    let locks = stage_locks::<MULTITHREADED>(&world);

    for (index, si) in safety_info.iter().enumerate() {
        // skip missing components
        if unsafe { components.get_unchecked(index).is_null() } {
            continue;
        }

        // transient read: verify no write is present, then release immediately
        if !si.cr.is_null() {
            sparse_id_record_lock_read_begin(&world, locks, si.cr);
            sparse_id_record_lock_read_end(locks, si.cr);
            continue;
        }

        get_table_column_lock_read_begin(&world, locks, si.table, si.column_index);
        table_column_lock_read_end(locks, si.table, si.column_index);
    }
}
