use crate::core::*;

use core::ptr::NonNull;

/// Take the borrow locks for a tuple access, run the callback, release.
///
/// Deferring is the caller's responsibility (see `ScopeEndGuard` /
/// `ecs_rust_get_scope_begin`): the defer scope must enclose this call so a
/// violation panic or callback panic unwinds through the caller's guard and
/// closes the defer bracket. Releasing locks inside the caller's still-open
/// defer scope also means command-flush side effects (hooks, observers)
/// never run under these locks.
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

    for (index, si) in safety_info.iter().enumerate() {
        if unsafe { components.get_unchecked(index).is_null() } {
            continue;
        }
        // SAFETY: stage map is owned by this thread; the reference does not
        // outlive this iteration, so the callback below can nest accesses.
        let map = unsafe { &mut *locks.as_ptr() };
        match si {
            SafetyInfo::Read(li) => {
                if map.read_begin(li.key) {
                    alias_violation_panic(world, li.id, false);
                }
            }
            SafetyInfo::Write(li) => {
                if map.write_begin(li.key) {
                    alias_violation_panic(world, li.id, true);
                }
            }
        }
    }

    let ret = callback(tuple);

    for (index, si) in safety_info.iter().enumerate() {
        if unsafe { components.get_unchecked(index).is_null() } {
            continue;
        }
        // SAFETY: as above.
        let map = unsafe { &mut *locks.as_ptr() };
        match si {
            SafetyInfo::Read(li) => map.read_end(li.key),
            SafetyInfo::Write(li) => map.write_end(li.key),
        }
    }
    ret
}

/// Transient read check for `cloned`: verify no write is held on any source,
/// releasing each probe immediately.
#[inline(always)]
pub(crate) fn clone_locking<const MULTITHREADED: bool>(
    world: WorldRef<'_>,
    components: &[*mut core::ffi::c_void],
    safety_info: &[LockInfo],
) {
    let locks: NonNull<StageLocks> = stage_locks::<MULTITHREADED>(&world);

    for (index, li) in safety_info.iter().enumerate() {
        // skip missing components
        if unsafe { components.get_unchecked(index).is_null() } {
            continue;
        }

        // SAFETY: stage map is owned by this thread.
        let map = unsafe { &mut *locks.as_ptr() };
        if map.read_begin(li.key) {
            alias_violation_panic(&world, li.id, false);
        }
        map.read_end(li.key);
    }
}
