#![allow(dead_code)]
use flecs_ecs::sys;

#[test]
fn ecs_rust_get_ptr_t_default_terminates() {
    let value = sys::ecs_rust_get_ptr_t::default();
    assert!(value.ptr.is_null());
    assert_eq!(value.lock_key, 0);
}

/// Hard invariant (spec §13.3): `ecs_rust_get_ptr_t` stays 16 bytes so it is
/// returned in registers rather than through a hidden sret pointer, which is
/// what lets `get` beat upstream `ecs_get_id` by 19-30%. Widening it silently
/// regresses the hot path, so the width is also pinned by a `const _` assertion
/// beside its use in `core::get_tuple`; this test mirrors that guard in the
/// required suite.
#[test]
fn ecs_rust_get_ptr_t_is_16_bytes() {
    assert_eq!(core::mem::size_of::<sys::ecs_rust_get_ptr_t>(), 16);
}
