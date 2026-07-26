use crate::core::*;

use core::ptr::NonNull;

/// Transient read check for `cloned`: verify no write is held on any source.
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
        if unsafe { (*locks.as_ptr()).check_read(li.key) } {
            alias_violation_panic(&world, li.id, false);
        }
    }
}
