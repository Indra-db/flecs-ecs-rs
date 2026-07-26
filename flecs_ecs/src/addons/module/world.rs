use crate::addons::module::Module;
use crate::prelude::*;
use crate::sys;

/// Module mixin implementation
impl World {
    /// Import a module.
    ///
    /// This operation will load a module. The module name will be used to verify if
    /// the module was already loaded, in which case it won't be reimported.
    ///
    /// Module contents will be stored as children of the module entity. This
    /// prevents modules from accidentally defining conflicting identifiers. This is
    /// enforced by setting the scope before and after loading the module to the
    /// module entity id.
    ///
    /// ```
    /// # use flecs_ecs::prelude::*;
    /// # #[derive(Component)]
    /// # struct MyModule;
    /// # impl Module for MyModule {
    /// #     fn module(_world: &World) {
    /// #     }
    /// # }
    /// # let world = World::new();
    /// world.import::<MyModule>();
    /// ```
    ///
    /// # See also
    ///
    /// * [`addons::module`](crate::addons::module)
    /// * [`Module`]
    /// * [`World::module()`]
    pub fn import<T: Module>(&self) -> EntityView<'_> {
        // Reset scope
        let prev_scope = self.set_scope(0);
        let world = self.world();
        let raw_world = self.raw_world.as_ptr();
        let symbol_name = core::any::type_name::<T>();
        let symbol = compact_str::format_compact!("{}\0", symbol_name);
        let m =
            unsafe { sys::ecs_lookup_symbol(raw_world, symbol.as_ptr() as *const _, true, false) };
        let module = if T::is_registered_with_world(self) && m != 0 {
            self.component::<T>().entity
        } else {
            let id = self.entity_from_id(register_component_data_explicit::<T, true>(
                world,
                core::ptr::null(),
            ));
            let id_u64 = *id.id();
            // `T::index()` is a per-type static that is shared across every
            // monomorphization of a generic type, so two instantiations of one
            // generic module would alias the same array slot. Only non-generic
            // modules may use the array; generic modules are keyed by `TypeId`.
            if !T::IS_GENERIC {
                let index = T::index() as usize;
                let components_array = self.components_array();
                if index >= components_array.len() {
                    components_array.reserve(index + 1 - components_array.len());
                    let capacity = components_array.capacity();
                    unsafe {
                        core::ptr::write_bytes(
                            components_array.as_mut_ptr().add(components_array.len()),
                            0,
                            capacity - components_array.len(),
                        );
                        components_array.set_len(capacity);
                    }
                }
                components_array[index] = id_u64;
            }
            #[cfg(feature = "flecs_meta")]
            {
                self.components_map()
                    .insert(core::any::TypeId::of::<T>(), id_u64);
            }
            id
        };

        // If we have already registered this type don't re-create the module
        if module.has(id::<flecs::Module>()) {
            return module;
        }

        // Make module component sparse so that it'll never move in memory. This
        // guarantees that a module drop / destructor can be reliably used to cleanup
        // module resources.
        module.add_trait::<flecs::Sparse>();

        module.add_trait::<flecs::Singleton>();

        // Set scope to our module
        self.set_scope(module);

        let module_id = *module.id();
        unsafe { sys::ecs_add_id(raw_world, module_id, module_id) };
        unsafe { sys::ecs_modified_id(raw_world, module_id, module_id) };

        // Build the module
        T::module(self);

        // Return out scope to the previous scope
        self.set_scope(prev_scope);

        // Initialise component for the module and add Module tag
        unsafe { module.add_id_unchecked(flecs::Module::ID) };

        module
    }

    /// Define a module.
    ///
    /// This operation is not mandatory, but can be called inside the module ctor to
    /// obtain the entity associated with the module, or override the module name.
    ///
    /// # Type Parameters
    ///
    /// * `M` - The type of the module.
    ///
    /// # Arguments
    ///
    /// * `name` - The name to give the module.
    ///
    /// # Returns
    ///
    /// The module entity.
    ///
    /// # See also
    ///
    /// * [`addons::module`](crate::addons::module)
    /// * [`Module`]
    /// * [`World::import()`]
    pub fn module<M: ComponentId>(&self, name: &str) -> EntityView<'_> {
        let comp = self.component::<M>();
        let id = comp.id();

        if let Some(existing) = self.try_lookup_recursive(name) {
            self.set_scope(existing);
            return existing;
        }

        if !name.is_empty() {
            let name = compact_str::format_compact!("{}\0", name);
            let prev_parent = comp.parent().unwrap_or(EntityView::new_null(self));
            unsafe {
                sys::ecs_add_path_w_sep(
                    self.raw_world.as_ptr(),
                    *id,
                    0,
                    name.as_ptr() as *const _,
                    SEPARATOR.as_ptr(),
                    SEPARATOR.as_ptr(),
                );
            }
            let parent = comp.parent().unwrap_or(EntityView::new_null(self));

            if parent != prev_parent {
                // Module was reparented, cleanup old parent(s)
                let mut cur = prev_parent;
                let mut next;

                while cur.id != 0 {
                    next = cur.parent().unwrap_or(EntityView::new_null(self));

                    cur.destruct();

                    // Prevent increasing the generation count of the temporary
                    // parent. This allows entities created during
                    // initialization to keep non-recycled ids.
                    self.set_version(cur);

                    cur = next;

                    if cur.id == 0 {
                        break;
                    }
                }
            }
        }

        EntityView::new_from(self, *id)
    }
}
