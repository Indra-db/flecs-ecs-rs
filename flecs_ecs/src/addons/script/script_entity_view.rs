use core::ops::Deref;
use core::ops::DerefMut;

use flecs_ecs::core::*;
use flecs_ecs::sys;

#[cfg(feature = "std")]
extern crate std;

extern crate alloc;
use alloc::{borrow::ToOwned, string::String};

/// [`ScriptEntityView`] is a wrapper around an entity that is associated with a script.
#[derive(Clone, Copy)]
pub struct ScriptEntityView<'a> {
    entity: EntityView<'a>,
}

impl<'a> Deref for ScriptEntityView<'a> {
    type Target = EntityView<'a>;

    #[inline(always)]
    fn deref(&self) -> &Self::Target {
        &self.entity
    }
}

impl DerefMut for ScriptEntityView<'_> {
    #[inline(always)]
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.entity
    }
}

impl<'a> ScriptEntityView<'a> {
    /// Create a new script entity view.
    pub fn new_from(world: impl WorldProvider<'a>, entity: impl IntoEntity) -> Self {
        let entity = EntityView::new_from(world, entity);
        ecs_assert!(
            entity.has(id::<flecs::Script>()),
            FlecsErrorCode::InvalidParameter,
            "Entity does not have a script component"
        );
        ScriptEntityView { entity }
    }

    /// Update script with new code.
    ///
    /// # Arguments
    ///
    /// * code - The script code.
    ///
    /// * script - The script entity.
    ///
    /// * instance - An template instance (optional).
    ///
    /// # Returns
    ///
    /// True if success, false if failed.
    ///
    /// # See also
    ///
    /// * C API: `ecs_script_update`
    pub fn update(
        &self,
        world: impl WorldProvider<'a>,
        instance: Option<impl Into<Entity>>,
        code: &str,
    ) -> bool {
        let code = compact_str::format_compact!("{}\0", code);
        unsafe {
            sys::ecs_script_update(
                world.world_ptr_mut(),
                *self.id,
                instance.map(|e| *e.into()).unwrap_or(0),
                code.as_ptr() as *const _,
            ) == 0
        }
    }

    /// Convert script AST to string.
    /// This operation converts the script abstract syntax tree to a string, which can be used to debug a script.
    ///
    /// # Returns
    ///
    /// Some String if success, None if failed.
    ///
    /// # See also
    ///
    /// * C API: `script_ast_to_buf`
    pub fn ast(&mut self) -> Option<String> {
        let script_ptr = self.get_untyped(flecs::Script::id()) as *const flecs::Script;
        assert!(
            !script_ptr.is_null(),
            "entity does not have a flecs::Script component"
        );
        // SAFETY: script_ptr is a valid, non-null pointer to the entity's
        // flecs::Script component, checked above.
        let script = unsafe { (*script_ptr).script };

        let ast = unsafe { sys::ecs_script_ast_to_str(script, false) };

        if ast.is_null() {
            ecs_assert!(
                false,
                FlecsErrorCode::InvalidOperation,
                "Script AST already exists"
            );
            return None;
        }

        let c_str = unsafe { core::ffi::CStr::from_ptr(ast) };
        let str = c_str.to_str().unwrap().to_owned();
        unsafe { sys::ecs_os_api.free_.expect("os api is missing")(ast as *mut core::ffi::c_void) };
        Some(str)
    }
}
