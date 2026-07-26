use core::ffi::CStr;
use core::ptr::NonNull;

use crate::core::*;
use crate::sys;

extern crate alloc;
use alloc::string::{String, ToString};

//MARK: impl World
/// ```
/// use flecs_ecs::prelude::*;
///
/// #[derive(Component)]
/// struct Tag;
///
/// let world = World::default();
/// world.component::<Tag>();
/// world.set_doc_name(Tag, "A tag");
/// ```
impl World {
    //MARK: _World::getters

    /// Get human readable name for an entity.
    ///
    /// # Arguments
    ///
    /// * `entity` - The entity for which to get the human readable name.
    ///
    /// # Returns
    ///
    /// The human readable name of the entity.
    ///
    /// # See also
    ///
    /// * [`Doc::doc_name()`](super::Doc::doc_name)
    /// * [`World::doc_name()`]
    #[inline(always)]
    pub fn doc_name(&self, entity: impl IntoEntity) -> Option<String> {
        let cstr = NonNull::new(unsafe {
            sys::ecs_doc_get_name(self.world_ptr(), *entity.into_entity(self))
        } as *mut _)
        .map(|s| unsafe { CStr::from_ptr(s.as_ptr()) });
        cstr.and_then(|s| s.to_str().ok().map(ToString::to_string))
    }

    /// Get brief description for an entity.
    ///
    /// # Arguments
    ///
    /// * `entity` - The entity for which to get the brief description
    ///
    /// # Returns
    ///
    /// The brief description of the entity.
    ///
    /// # See also
    ///
    /// * [`Doc::doc_brief()`](super::Doc::doc_brief)
    /// * [`World::doc_brief()`]
    #[inline(always)]
    pub fn doc_brief(&self, entity: impl IntoEntity) -> Option<String> {
        let cstr = NonNull::new(unsafe {
            sys::ecs_doc_get_brief(self.world_ptr(), *entity.into_entity(self))
        } as *mut _)
        .map(|s| unsafe { CStr::from_ptr(s.as_ptr()) });
        cstr.and_then(|s| s.to_str().ok().map(ToString::to_string))
    }

    /// Get detailed description for an entity.
    ///
    /// # Arguments
    ///
    /// * `entity` - The entity for which to get the detailed description.
    ///
    /// # Returns
    ///
    /// The detailed description of the entity.
    ///
    /// # See also
    ///
    /// * [`Doc::doc_detail()`](super::Doc::doc_detail)
    /// * [`World::doc_detail()`]
    #[inline(always)]
    pub fn doc_detail(&self, entity: impl IntoEntity) -> Option<String> {
        let cstr = NonNull::new(unsafe {
            sys::ecs_doc_get_detail(self.world_ptr(), *entity.into_entity(self))
        } as *mut _)
        .map(|s| unsafe { CStr::from_ptr(s.as_ptr()) });
        cstr.and_then(|s| s.to_str().ok().map(ToString::to_string))
    }

    /// Get link to external documentation for an entity.
    /// # Arguments
    ///
    /// * `entity` - The entity for which to get the link to external documentation.
    ///
    /// # Returns
    ///
    /// The link to external documentation of the entity.
    ///
    /// # See also
    ///
    /// * [`Doc::doc_link()`](super::Doc::doc_link)
    /// * [`World::doc_link()`]
    #[inline(always)]
    pub fn doc_link(&self, entity: impl IntoEntity) -> Option<String> {
        let cstr = NonNull::new(unsafe {
            sys::ecs_doc_get_link(self.world_ptr(), *entity.into_entity(self))
        } as *mut _)
        .map(|s| unsafe { CStr::from_ptr(s.as_ptr()) });
        cstr.and_then(|s| s.to_str().ok().map(ToString::to_string))
    }

    /// Get color for an entity.
    /// # Arguments
    ///
    /// * `entity` - The entity for which to get the color.
    ///
    /// # Returns
    ///
    /// The color of the entity.
    ///
    /// # See also
    ///
    /// * [`Doc::doc_color()`](super::Doc::doc_color)
    /// * [`World::doc_color()`]
    #[inline(always)]
    pub fn doc_color(&self, entity: impl IntoEntity) -> Option<String> {
        let cstr = NonNull::new(unsafe {
            sys::ecs_doc_get_color(self.world_ptr(), *entity.into_entity(self))
        } as *mut _)
        .map(|s| unsafe { CStr::from_ptr(s.as_ptr()) });
        cstr.and_then(|s| s.to_str().ok().map(ToString::to_string))
    }

    /// Get UUID for entity
    ///
    /// # See Also
    ///
    /// * [`World::doc_uuid()`]
    /// * [`Doc::doc_uuid()`](super::Doc::doc_uuid)
    /// * [`Doc::set_doc_uuid()`](super::Doc::set_doc_uuid)
    pub fn doc_uuid(&self, entity: impl IntoEntity) -> Option<String> {
        let cstr = NonNull::new(unsafe {
            sys::ecs_doc_get_uuid(self.world_ptr(), *entity.into_entity(self))
        } as *mut _)
        .map(|s| unsafe { CStr::from_ptr(s.as_ptr()) });
        cstr.and_then(|s| s.to_str().ok().map(ToString::to_string))
    }

    //MARK: _World::setters

    /// Add human-readable name to entity.
    ///
    /// Contrary to entity names, human readable names do not have to be unique and
    /// can contain special characters used in the query language like '*'.
    ///
    /// # Arguments
    ///
    /// * `entity` - The entity to which to add the name.
    /// * `name` - The name to add.
    ///
    /// # See also
    ///
    /// * [`Doc::set_doc_name()`](super::Doc::set_doc_name)
    /// * [`World::set_doc_name()`]
    #[inline(always)]
    pub fn set_doc_name(&self, entity: impl IntoEntity, name: &str) {
        // Shared-register write hook (spec §3.6, §7.1): adds EcsDocDescription to
        // `entity`, so it must defer behind a live guard on `entity`'s data.
        crate::core::ensure_write_episode(&self.world());
        let name = compact_str::format_compact!("{}\0", name);
        unsafe {
            sys::ecs_doc_set_name(
                self.ptr_mut(),
                *entity.into_entity(self),
                name.as_ptr() as *const _,
            );
        };
    }

    /// Add brief description to entity.
    ///
    /// # Arguments
    ///
    /// * `entity` - The entity to which to add the brief description.
    /// * `brief` - The brief description to add.
    ///
    /// # See also
    ///
    /// * [`Doc::set_doc_brief()`](super::Doc::set_doc_brief)
    /// * [`World::set_doc_brief()`]
    #[inline(always)]
    pub fn set_doc_brief(&self, entity: impl IntoEntity, brief: &str) {
        // Shared-register write hook (spec §3.6, §7.1): adds EcsDocDescription to
        // `entity`, so it must defer behind a live guard on `entity`'s data.
        crate::core::ensure_write_episode(&self.world());
        let brief = compact_str::format_compact!("{}\0", brief);
        unsafe {
            sys::ecs_doc_set_brief(
                self.ptr_mut(),
                *entity.into_entity(self),
                brief.as_ptr() as *const _,
            );
        };
    }

    /// Add detailed description to entity.
    ///
    /// # Arguments
    ///
    /// * `entity` - The entity to which to add the detailed description.
    /// * `detail` - The detailed description to add.
    ///
    /// # See also
    ///
    /// * [`Doc::set_doc_detail()`](super::Doc::set_doc_detail)
    /// * [`World::set_doc_detail()`]
    #[inline(always)]
    pub fn set_doc_detail(&self, entity: impl IntoEntity, detail: &str) {
        // Shared-register write hook (spec §3.6, §7.1): adds EcsDocDescription to
        // `entity`, so it must defer behind a live guard on `entity`'s data.
        crate::core::ensure_write_episode(&self.world());
        let detail = compact_str::format_compact!("{}\0", detail);
        unsafe {
            sys::ecs_doc_set_detail(
                self.ptr_mut(),
                *entity.into_entity(self),
                detail.as_ptr() as *const _,
            );
        };
    }

    /// Add link to external documentation to entity.
    ///
    /// # Arguments
    ///
    /// * `entity` - The entity to which to add the link.
    /// * `link` - The link to add.
    ///
    /// # See also
    ///
    /// * [`Doc::set_doc_link()`](super::Doc::set_doc_link)
    /// * [`World::set_doc_link()`]
    #[inline(always)]
    pub fn set_doc_link(&self, entity: impl IntoEntity, link: &str) {
        // Shared-register write hook (spec §3.6, §7.1): adds EcsDocDescription to
        // `entity`, so it must defer behind a live guard on `entity`'s data.
        crate::core::ensure_write_episode(&self.world());
        let link = compact_str::format_compact!("{}\0", link);
        unsafe {
            sys::ecs_doc_set_link(
                self.ptr_mut(),
                *entity.into_entity(self),
                link.as_ptr() as *const _,
            );
        };
    }

    /// Add color to entity.
    ///
    /// UIs can use color as hint to improve visualizing entities.
    ///
    /// # Arguments
    ///
    /// * `entity` - The entity to which to add the color.
    /// * `color` - The color to add.
    ///
    /// # See also
    ///
    /// * [`Doc::set_doc_color()`](super::Doc::set_doc_color)
    /// * [`World::set_doc_color()`]
    #[inline(always)]
    pub fn set_doc_color(&self, entity: impl IntoEntity, color: &str) {
        // Shared-register write hook (spec §3.6, §7.1): adds EcsDocDescription to
        // `entity`, so it must defer behind a live guard on `entity`'s data.
        crate::core::ensure_write_episode(&self.world());
        let color = compact_str::format_compact!("{}\0", color);
        unsafe {
            sys::ecs_doc_set_color(
                self.ptr_mut(),
                *entity.into_entity(self),
                color.as_ptr() as *const _,
            );
        };
    }

    /// Add UUID to entity.
    /// This adds `(flecs.doc.Description, flecs.doc.Uuid)` to the entity.
    ///
    /// # Arguments
    ///
    /// * `entity` - The entity to which to add the UUID.
    /// * `uuid` - The UUID to add.
    ///
    /// # See also
    ///
    /// * [`Doc::set_doc_uuid()`](super::Doc::set_doc_uuid)
    /// * [`World::set_doc_uuid()`]
    /// * [`World::doc_uuid()`]
    /// * [`Doc::doc_uuid()`](super::Doc::doc_uuid)
    pub fn set_doc_uuid(&self, entity: impl IntoEntity, uuid: &str) {
        // Shared-register write hook (spec §3.6, §7.1): adds EcsDocDescription to
        // `entity`, so it must defer behind a live guard on `entity`'s data.
        crate::core::ensure_write_episode(&self.world());
        let uuid = compact_str::format_compact!("{}\0", uuid);
        unsafe {
            sys::ecs_doc_set_uuid(
                self.ptr_mut(),
                *entity.into_entity(self),
                uuid.as_ptr() as *const _,
            );
        };
    }
}
