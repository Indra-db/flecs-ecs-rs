//! `SystemBuilder` is a builder pattern for creating systems.

use crate::addons::system::*;
use crate::core::internals::*;
use crate::core::private::{internal_ParSystemAPI, internal_SystemAPI};
use crate::core::*;

/// `SystemBuilder` is a builder pattern for creating systems.
pub struct SystemBuilder<'a, T>
where
    T: QueryTuple,
{
    pub(crate) desc: sys::ecs_system_desc_t,
    term_builder: TermBuilder,
    world: WorldRef<'a>,
    _phantom: core::marker::PhantomData<&'a T>,
}

impl<'a, T> SystemBuilder<'a, T>
where
    T: QueryTuple,
{
    /// Create a new system builder
    pub(crate) fn new(world: &'a World) -> Self {
        let mut obj = Self {
            desc: Default::default(),
            term_builder: TermBuilder::default(),
            world: world.into(),
            _phantom: core::marker::PhantomData,
        };

        T::populate(&mut obj);

        #[cfg(feature = "flecs_pipeline")]
        {
            obj.desc.phase = ECS_ON_UPDATE;
        }

        obj
    }

    pub(crate) fn new_from_desc(world: &'a World, desc: sys::ecs_system_desc_t) -> Self {
        let mut obj = Self {
            desc,
            term_builder: TermBuilder::default(),
            world: world.into(),
            _phantom: core::marker::PhantomData,
        };

        T::populate(&mut obj);

        #[cfg(feature = "flecs_pipeline")]
        if obj.desc.phase == 0 {
            obj.desc.phase = ECS_ON_UPDATE;
        }

        obj
    }

    /// Create a new system builder with a name
    pub(crate) fn new_named(world: &'a World, name: &str) -> Self {
        let name = compact_str::format_compact!("{}\0", name);

        let mut obj = Self {
            desc: Default::default(),
            term_builder: TermBuilder::default(),
            world: world.into(),
            _phantom: core::marker::PhantomData,
        };

        let entity_desc: sys::ecs_entity_desc_t = sys::ecs_entity_desc_t {
            name: name.as_ptr() as *const _,
            sep: SEPARATOR.as_ptr(),
            root_sep: SEPARATOR.as_ptr(),
            ..Default::default()
        };
        obj.desc.entity = unsafe { sys::ecs_entity_init(obj.world_ptr_mut(), &entity_desc) };

        T::populate(&mut obj);

        #[cfg(feature = "flecs_pipeline")]
        {
            obj.desc.phase = ECS_ON_UPDATE;
        }
        obj
    }

    /// Specify in which phase the system should run
    ///
    /// # Arguments
    ///
    /// * `phase` - the phase
    pub fn kind(&mut self, phase: impl IntoEntity) -> &mut Self {
        self.desc.phase = *phase.into_entity(self.world);
        self
    }

    /// Specify in which enum phase the system should run
    ///
    /// # Arguments
    ///
    /// * `phase` - the phase
    pub fn kind_enum<Phase>(&mut self, phase: Phase) -> &mut Self
    where
        Phase: ComponentId + ComponentType<Enum> + EnumComponentInfo,
    {
        let enum_id = phase.id_variant(self.world());
        self.kind(enum_id)
    }

    /// Specify whether system should be ran in staged context.
    ///
    /// # Arguments
    ///
    /// * `value` - If false,  system will always run staged.
    pub fn immediate(&mut self, value: bool) -> &mut Self {
        self.desc.immediate = value;
        self
    }

    /// Attempts to build the system, returning `None` if system creation fails.
    ///
    /// This is the fallible counterpart of [`build()`](Builder::build): it returns
    /// `None` instead of a handle to an invalid entity when the underlying
    /// `ecs_system_init` call fails, most commonly due to an invalid query
    /// expression passed to `expr()`.
    ///
    /// # Panics
    ///
    /// Panics if neither a callback nor a run function was set, like
    /// [`build()`](Builder::build). Use `.each*` / `.run*` to set one.
    ///
    /// # See also
    ///
    /// * [`QueryBuilder::try_build()`]
    pub fn try_build(&mut self) -> Option<System<'a>> {
        let system = self.build();
        if *system.id() == 0 {
            None
        } else {
            Some(system)
        }
    }
}

#[doc(hidden)]
impl<'a, T: QueryTuple> internals::QueryConfig<'a> for SystemBuilder<'a, T> {
    #[inline(always)]
    fn term_builder(&self) -> &TermBuilder {
        &self.term_builder
    }

    #[inline(always)]
    fn term_builder_mut(&mut self) -> &mut TermBuilder {
        &mut self.term_builder
    }

    #[inline(always)]
    fn query_desc(&self) -> &sys::ecs_query_desc_t {
        &self.desc.query
    }

    #[inline(always)]
    fn query_desc_mut(&mut self) -> &mut sys::ecs_query_desc_t {
        &mut self.desc.query
    }
    #[inline(always)]
    fn count_generic_terms(&self) -> i32 {
        T::COUNT
    }
}

impl<'a, T: QueryTuple> TermBuilderImpl<'a> for SystemBuilder<'a, T> {}

impl<'a, T: QueryTuple> QueryBuilderImpl<'a> for SystemBuilder<'a, T> {}

impl<'a, T> Builder<'a> for SystemBuilder<'a, T>
where
    T: QueryTuple,
{
    type BuiltType = System<'a>;

    #[doc(hidden)]
    /// Build the `system_builder` into an system
    fn build(&mut self) -> Self::BuiltType {
        if self.desc.callback.is_none() && self.desc.run.is_none() {
            panic!("you should not call this fn manually. Use `.each` , `.run` instead")
        }
        let system = System::new(self.world(), self.desc);
        if *system.id() == 0 {
            unsafe {
                reclaim_leaked_binding_ctx(self.desc.ctx, self.desc.ctx_free);
                reclaim_leaked_binding_ctx(self.desc.callback_ctx, self.desc.callback_ctx_free);
                reclaim_leaked_binding_ctx(self.desc.run_ctx, self.desc.run_ctx_free);
            }
        }
        for s in self.term_builder.str_ptrs_to_free.iter_mut() {
            unsafe { core::mem::ManuallyDrop::drop(s) };
        }
        self.term_builder.str_ptrs_to_free.clear();
        system
    }
}

impl<'a, T: QueryTuple> WorldProvider<'a> for SystemBuilder<'a, T> {
    fn world(&self) -> WorldRef<'a> {
        self.world
    }
}

implement_reactor_api!((), SystemBuilder<'a, T>);
implement_reactor_par_api!((), SystemBuilder<'a, T>);

/// Updates an existing [`System`]'s callback or context via
/// `ecs_system_update()`. Created with [`System::update()`].
pub struct SystemUpdater<'a, T: QueryTuple = ()> {
    pub(crate) desc: sys::ecs_system_desc_t,
    world: WorldRef<'a>,
    entity: EntityView<'a>,
    _phantom: core::marker::PhantomData<&'a T>,
}

impl<'a, T: QueryTuple> SystemUpdater<'a, T> {
    pub(crate) fn new(entity: EntityView<'a>) -> Self {
        Self {
            desc: Default::default(),
            world: entity.world(),
            entity,
            _phantom: core::marker::PhantomData,
        }
    }
}

impl<'a, T> Builder<'a> for SystemUpdater<'a, T>
where
    T: QueryTuple,
{
    type BuiltType = System<'a>;

    #[doc(hidden)]
    fn build(&mut self) -> Self::BuiltType {
        if self.desc.callback.is_none() && self.desc.run.is_none() {
            panic!("you should not call this fn manually. Use `.each` , `.run` instead")
        }
        unsafe {
            sys::ecs_system_update(self.world.world_ptr_mut(), *self.entity.id(), &self.desc);
        }
        System::new_from_existing(self.entity)
    }
}

impl<'a, T: QueryTuple> WorldProvider<'a> for SystemUpdater<'a, T> {
    fn world(&self) -> WorldRef<'a> {
        self.world
    }
}

implement_reactor_api!((), SystemUpdater<'a, T>);
