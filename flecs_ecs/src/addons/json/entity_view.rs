use super::*;
use crate::core::*;
use crate::sys;
use alloc::string::String;
use alloc::string::ToString;

impl EntityView<'_> {
    /// Set component or pair id from JSON.
    pub fn set_json(self, comp: impl IntoId, json: &str, desc: Option<&FromJsonDesc>) -> Self {
        let comp: u64 = *comp.into_id(self.world);
        // Shared-register write hook (spec §3.6, §7.1): `ecs_ensure_id` adds `comp`
        // to this entity, moving it, so it must defer behind a live guard.
        crate::core::ensure_write_episode(&self.world);
        let world = self.world_ptr_mut();
        let id = *self.id;
        unsafe {
            let ti = sys::ecs_get_type_info(world, comp);
            if ti.is_null() {
                //sys::ecs_err(b"id is not a type\0".as_ptr() as *const _);
                //TODO implement ecs_err
                return self;
            }

            let type_ = (*ti).component;
            let ptr = sys::ecs_ensure_id(world, id, comp, (*ti).size as usize);
            ecs_assert!(
                !ptr.is_null(),
                FlecsErrorCode::InternalError,
                "could not add comp to entity"
            );
            let json = compact_str::format_compact!("{}\0", json);
            if let Some(desc) = desc {
                sys::ecs_ptr_from_json(world, type_, ptr, json.as_ptr() as *const _, desc);
            } else {
                sys::ecs_ptr_from_json(
                    world,
                    type_,
                    ptr,
                    json.as_ptr() as *const _,
                    core::ptr::null(),
                );
            }
            sys::ecs_modified_id(world, id, comp);
        }
        self
    }

    /// Serialize entity to JSON.
    pub fn to_json(&self, desc: Option<&EntityToJsonDesc>) -> String {
        let world = self.world_ptr();
        let id = *self.id;
        let desc_ptr = desc
            .map(|d| d as *const EntityToJsonDesc)
            .unwrap_or(core::ptr::null());

        unsafe {
            let json_ptr = sys::ecs_entity_to_json(world.cast_mut(), id, desc_ptr);
            assert!(!json_ptr.is_null(), "entity to JSON serialization failed");
            let json = core::ffi::CStr::from_ptr(json_ptr)
                .to_str()
                .unwrap()
                .to_string();
            sys::ecs_os_api.free_.expect("os api is missing")(json_ptr as *mut core::ffi::c_void);
            json
        }
    }

    /// Deserialize entity to JSON.
    pub fn from_json(self, json: &str) -> Self {
        // Shared-register write hook (spec §3.6, §7.1): deserialization adds/sets
        // components on this entity, so it must defer behind a live guard.
        crate::core::ensure_write_episode(&self.world);
        let world = self.world_ptr_mut();
        let id = *self.id;
        //TODO we should have an Json Type so we don't need to make these conversions multiple times.
        let json = compact_str::format_compact!("{}\0", json);
        unsafe {
            sys::ecs_entity_from_json(world, id, json.as_ptr() as *const _, core::ptr::null());
        }
        self
    }
}
