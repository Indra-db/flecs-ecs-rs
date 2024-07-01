//! The doc module allows for documenting entities (and thus components, systems)
//! by adding brief and/or detailed descriptions as components. Documentation
//! added with the doc module can be retrieved at runtime, and can be used by
//! tooling such as UIs or documentation frameworks.

use crate::core::*;
use crate::sys;

///
///
/// ```
/// use flecs_ecs::{addons::doc::Doc, core::World, macros::Component};
///
/// #[derive(Component)]
/// struct Tag;
///
/// let world = World::default();
/// world.component::<Tag>()
///      .set_doc_name(&world, "A tag");
///
/// world.entity()
///      .set_doc_brief(&world, "A vast expanse of nothingness.");
/// ```
pub trait Doc {
    /// Add human-readable name to entity.
    ///
    /// Contrary to entity names, human readable names do not have to be unique and
    /// can contain special characters used in the query language like '*'.
    ///
    /// # Arguments
    ///
    /// * `world` - The world.
    /// * `name` - The name to add.
    ///
    /// # See also
    ///
    /// * [`World::set_doc_name()`]
    /// * [`World::set_doc_name_id()`]
    /// * C++ API: `doc::set_name()`
    fn set_doc_name<'a>(self, world: impl IntoWorld<'a>, name: &str);

    /// Add brief description to entity.
    ///
    /// # Arguments
    ///
    /// * `world` - The world.
    /// * `brief` - The brief description to add.
    ///
    /// # See also
    ///
    /// * [`World::set_doc_brief()`]
    /// * [`World::set_doc_brief_id()`]
    /// * C++ API: `doc::set_brief()`
    fn set_doc_brief<'a>(self, world: impl IntoWorld<'a>, brief: &str);

    /// Add detailed description to entity.
    ///
    /// # Arguments
    ///
    /// * `world` - The world.
    /// * `detail` - The detailed description to add.
    ///
    /// # See also
    ///
    /// * [`World::set_doc_detail()`]
    /// * [`World::set_doc_detail_id()`]
    /// * C++ API: `doc::set_detail()`
    fn set_doc_detail<'a>(self, world: impl IntoWorld<'a>, detail: &str);

    /// Add link to external documentation to entity.
    ///
    /// # Arguments
    ///
    /// * `world` - The world.
    /// * `link` - The link to add.
    ///
    /// # See also
    ///
    /// * [`World::set_doc_link()`]
    /// * [`World::set_doc_link_id()`]
    /// * C++ API: `doc::set_link()`
    fn set_doc_link<'a>(self, world: impl IntoWorld<'a>, link: &str);

    /// Add color to entity.
    ///
    /// UIs can use color as hint to improve visualizing entities.
    ///
    /// # Arguments
    ///
    /// * `world` - The world.
    /// * `color` - The color to add.
    ///
    /// # See also
    ///
    /// * [`World::set_doc_color()`]
    /// * [`World::set_doc_color_id()`]
    /// * C++ API: `doc::set_color()`
    fn set_doc_color<'a>(self, world: impl IntoWorld<'a>, color: &str);
}

impl<T: Into<Entity>> Doc for T {
    fn set_doc_name<'a>(self, world: impl IntoWorld<'a>, name: &str) {
        world.world().set_doc_name_id(self, name);
    }

    fn set_doc_brief<'a>(self, world: impl IntoWorld<'a>, brief: &str) {
        world.world().set_doc_brief_id(self, brief);
    }

    fn set_doc_detail<'a>(self, world: impl IntoWorld<'a>, detail: &str) {
        world.world().set_doc_detail_id(self, detail);
    }

    fn set_doc_link<'a>(self, world: impl IntoWorld<'a>, link: &str) {
        world.world().set_doc_link_id(self, link);
    }

    fn set_doc_color<'a>(self, world: impl IntoWorld<'a>, color: &str) {
        world.world().set_doc_color_id(self, color);
    }
}

/// ```
/// use flecs_ecs::{addons::doc::Doc, core::World, macros::Component};
///
/// #[derive(Component)]
/// struct Tag;
///
/// let world = World::default();
/// world.component::<Tag>();
/// world.set_doc_name::<Tag>("A tag");
/// ```
///
impl World {
    /// Add human-readable name to entity.
    ///
    /// Contrary to entity names, human readable names do not have to be unique and
    /// can contain special characters used in the query language like '*'.
    ///
    /// # Type Parameters
    ///
    /// * `T` - The type that implements `ComponentId`.
    ///
    /// # Arguments
    ///
    /// * `name` - The name to add.
    ///
    /// # See also
    ///
    /// * [`Doc::set_doc_name()`]
    /// * [`World::set_doc_name_id()`]
    /// * C++ API: `doc::get_name()`
    #[doc(alias = "world::set_doc_name")]
    #[inline(always)]
    pub fn set_doc_name<T: ComponentId>(&self, name: &str) {
        self.set_doc_name_id(T::get_id(self), name);
    }

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
    /// * [`Doc::set_doc_name()`]
    /// * [`World::set_doc_name()`]
    /// * C++ API: `world::set_doc_name()`
    #[doc(alias = "world::set_doc_name")]
    #[inline(always)]
    pub fn set_doc_name_id(&self, entity: impl Into<Entity>, name: &str) {
        let name = compact_str::format_compact!("{}\0", name);
        unsafe { sys::ecs_doc_set_name(self.ptr_mut(), *entity.into(), name.as_ptr() as *const _) };
    }

    /// Add brief description to entity.
    ///
    /// # Type Parameters
    ///
    /// * `T` - The type that implements `ComponentId`.
    ///
    /// # Arguments
    ///
    /// * `brief` - The brief description to add.
    ///
    /// # See also
    ///
    /// * [`Doc::set_doc_brief()`]
    /// * [`World::set_doc_brief_id()`]
    /// * C++ API: `world::set_doc_brief()`
    #[doc(alias = "world::set_doc_brief")]
    #[inline(always)]
    pub fn set_doc_brief<T: ComponentId>(&self, brief: &str) {
        self.set_doc_brief_id(T::get_id(self), brief);
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
    /// * [`Doc::set_doc_brief()`]
    /// * [`World::set_doc_brief()`]
    /// * C++ API: `world::set_doc_brief()`
    #[doc(alias = "world::set_doc_brief")]
    #[inline(always)]
    pub fn set_doc_brief_id(&self, entity: impl Into<Entity>, brief: &str) {
        let brief = compact_str::format_compact!("{}\0", brief);
        unsafe {
            sys::ecs_doc_set_brief(self.ptr_mut(), *entity.into(), brief.as_ptr() as *const _);
        };
    }

    /// Add detailed description to entity.
    ///
    /// # Type Parameters
    ///
    /// * `T` - The type that implements `ComponentId`.
    ///
    /// # Arguments
    ///
    /// * `detail` - The detailed description to add.
    ///
    /// # See also
    ///
    /// * [`Doc::set_doc_detail()`]
    /// * [`World::set_doc_detail_id()`]
    /// * C++ API: `world::set_doc_detail()`
    #[doc(alias = "world::set_doc_detail")]
    #[inline(always)]
    pub fn set_doc_detail<T: ComponentId>(&self, detail: &str) {
        self.set_doc_detail_id(T::get_id(self), detail);
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
    /// * [`Doc::set_doc_detail()`]
    /// * [`World::set_doc_detail()`]
    /// * C++ API: `world::set_doc_detail()`
    #[doc(alias = "world::set_doc_detail")]
    #[inline(always)]
    pub fn set_doc_detail_id(&self, entity: impl Into<Entity>, detail: &str) {
        let detail = compact_str::format_compact!("{}\0", detail);
        unsafe {
            sys::ecs_doc_set_detail(self.ptr_mut(), *entity.into(), detail.as_ptr() as *const _);
        };
    }

    /// Add link to external documentation to entity.
    ///
    /// # Type Parameters
    ///
    /// * `T` - The type that implements `ComponentId`.
    ///
    /// # Arguments
    ///
    /// * `link` - The link to add.
    ///
    /// # See also
    ///
    /// * [`Doc::set_doc_link()`]
    /// * [`World::set_doc_link_id()`]
    /// * C++ API: `world::set_doc_link()`
    #[doc(alias = "world::set_doc_link")]
    #[inline(always)]
    pub fn set_doc_link<T: ComponentId>(&self, link: &str) {
        self.set_doc_link_id(T::get_id(self), link);
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
    /// * [`Doc::set_doc_link()`]
    /// * [`World::set_doc_link()`]
    /// * C++ API: `world::set_doc_link()`
    #[doc(alias = "world::set_doc_link")]
    #[inline(always)]
    pub fn set_doc_link_id(&self, entity: impl Into<Entity>, link: &str) {
        let link = compact_str::format_compact!("{}\0", link);
        unsafe { sys::ecs_doc_set_link(self.ptr_mut(), *entity.into(), link.as_ptr() as *const _) };
    }

    /// Add color to entity.
    ///
    /// UIs can use color as hint to improve visualizing entities.
    ///
    /// # Type Parameters
    ///
    /// * `T` - The type that implements `ComponentId`.
    ///
    /// # Arguments
    ///
    /// * `color` - The color to add.
    ///
    /// # See also
    ///
    /// * [`Doc::set_doc_color()`]
    /// * [`World::set_doc_color_id()`]
    /// * C++ API: `world::set_doc_color()`
    #[doc(alias = "world::set_doc_color")]
    #[inline(always)]
    pub fn set_doc_color<T: ComponentId>(&self, color: &str) {
        self.set_doc_color_id(T::get_id(self), color);
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
    /// * [`Doc::set_doc_color()`]
    /// * [`World::set_doc_color()`]
    /// * C++ API: `world::set_doc_color()`
    #[doc(alias = "world::set_doc_color")]
    #[inline(always)]
    pub fn set_doc_color_id(&self, entity: impl Into<Entity>, color: &str) {
        let color = compact_str::format_compact!("{}\0", color);
        unsafe {
            sys::ecs_doc_set_color(self.ptr_mut(), *entity.into(), color.as_ptr() as *const _);
        };
    }
}
