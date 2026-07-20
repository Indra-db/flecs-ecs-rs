#![allow(dead_code)]
use flecs_ecs::sys;

#[test]
fn ecs_rust_get_ptr_t_default_terminates() {
    let value = sys::ecs_rust_get_ptr_t::default();
    assert!(value.ptr.is_null());
    assert!(value.lock_target.cr.is_null());
    assert!(value.lock_target.table.is_null());
    assert_eq!(value.lock_target.column_index, -1);
}
