//! `SystemBuilder` is a builder pattern for creating systems.

use crate::addons::system::*;
use crate::core::internals::*;
use crate::core::private::{internal_ParSystemAPI, internal_SystemAPI};
use crate::core::*;

extern crate alloc;
use alloc::string::{String, ToString};

/// `SystemBuilder` is a builder pattern for creating systems.
///
/// # Single-use terminal
///
/// The iteration terminals (`each` / `run` / ...) consume the builder by value,
/// so invoking a terminal twice on one builder is a compile error rather than a
/// silent logic bug:
///
/// ```compile_fail
/// use flecs_ecs::prelude::*;
/// let world = World::new();
/// let builder = world.system::<()>();
/// let _first = builder.each(|_| {});
/// let _second = builder.each(|_| {}); // error: `builder` was already consumed
/// ```
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
    pub fn kind(mut self, phase: impl IntoEntity) -> Self {
        self.desc.phase = *phase.into_entity(self.world);
        self
    }

    /// Specify in which enum phase the system should run
    ///
    /// # Arguments
    ///
    /// * `phase` - the phase
    pub fn kind_enum<Phase>(self, phase: Phase) -> Self
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
    pub fn immediate(mut self, value: bool) -> Self {
        self.desc.immediate = value;
        self
    }

    /// Mark the system multithreaded (spec §6): flecs partitions its matched
    /// tables across worker stages. Pairs with the `par_*` terminals, which set
    /// this too; calling it explicitly is idempotent.
    pub fn multi_threaded(mut self) -> Self {
        self.desc.multi_threaded = true;
        self
    }

    /// Set a runtime query expression, transitioning to the fallible-build typestate.
    ///
    /// A purely-typed system builder is infallible: its terminals (`each` / `run` /
    /// ...) return a [`System`] directly. A runtime expression string can fail to
    /// parse, so calling `expr()` consumes the builder and returns a
    /// [`FallibleSystemBuilder`] whose terminals return
    /// `Result<System, SystemBuildError>`. All remaining configuration methods are
    /// still available on the returned builder.
    pub fn expr(self, expr: &str) -> FallibleSystemBuilder<'a, T> {
        let mut me = QueryBuilderImpl::expr(self, expr);
        FallibleSystemBuilder {
            desc: me.desc,
            term_builder: core::mem::take(&mut me.term_builder),
            world: me.world,
            expr: expr.to_string(),
            _phantom: core::marker::PhantomData,
        }
    }

    /// Store a typed value owned by the system (spec §5.3).
    ///
    /// The capture-based replacement for the `*mut c_void`
    /// [`set_context`](crate::core::SystemAPI::set_context): the value is owned by
    /// the system entity and dropped when the system is dropped (world teardown or
    /// explicit deletion). Retrieve it with [`System::ctx`](crate::addons::system::System::ctx)
    /// / [`System::ctx_mut`](crate::addons::system::System::ctx_mut).
    ///
    /// Do not mix with the raw [`set_context`](crate::core::SystemAPI::set_context)
    /// on the same system: they share the one context slot.
    pub fn ctx<C: 'static>(mut self, value: C) -> Self {
        // Drop any previously-installed typed ctx before overwriting the slot.
        if let Some(free) = self.desc.ctx_free.take()
            && !self.desc.ctx.is_null()
        {
            unsafe { free(self.desc.ctx) };
        }
        let boxed: alloc::boxed::Box<dyn core::any::Any> = alloc::boxed::Box::new(value);
        let thin = alloc::boxed::Box::into_raw(alloc::boxed::Box::new(boxed))
            as *mut core::ffi::c_void;
        self.desc.ctx = thin;
        self.desc.ctx_free = Some(free_typed_system_ctx);
        self
    }
}

/// Drop trampoline for a typed system context installed by
/// [`SystemBuilder::ctx`]. Runs when flecs frees the system's context slot
/// (system deletion / world teardown).
#[flecs_ecs_derive::extern_abi]
fn free_typed_system_ctx(ptr: *mut core::ffi::c_void) {
    if !ptr.is_null() {
        // SAFETY: `ptr` is the thin `*mut Box<dyn Any>` installed by
        // `SystemBuilder::ctx`; flecs calls this exactly once for it.
        unsafe {
            drop(alloc::boxed::Box::from_raw(
                ptr as *mut alloc::boxed::Box<dyn core::any::Any>,
            ));
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
    fn build(mut self) -> Self::BuiltType {
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
            for s in self.term_builder.str_ptrs_to_free.iter_mut() {
                unsafe { core::mem::ManuallyDrop::drop(s) };
            }
            self.term_builder.str_ptrs_to_free.clear();
            panic!(
                "failed to initialize system: the descriptor was rejected by ecs_system_init. \
                 A purely-typed system builder cannot produce this; use `expr()` for a fallible \
                 build when passing a runtime query expression."
            );
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
    fn build(self) -> Self::BuiltType {
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

/// A malformed system construction reported by the fallible-build typestate.
///
/// Only a builder that took a runtime [`expr()`](SystemBuilder::expr) can produce
/// this error; a purely-typed system builder is infallible.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum SystemBuildError {
    /// The runtime `expr()` string failed to parse or validate.
    InvalidExpr {
        /// The offending expression string.
        expr: String,
    },
    /// `ecs_system_init` rejected the descriptor for another reason.
    Init,
}

impl core::fmt::Display for SystemBuildError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            SystemBuildError::InvalidExpr { expr } => {
                write!(f, "invalid system query expression: {expr:?}")
            }
            SystemBuildError::Init => write!(f, "system initialization failed"),
        }
    }
}

impl core::error::Error for SystemBuildError {}

/// The fallible-build typestate of [`SystemBuilder`], entered via
/// [`SystemBuilder::expr()`].
///
/// It carries the same configuration surface as [`SystemBuilder`], but its terminals
/// (`each` / `each_entity` / `each_iter` / `run` / `par_each` / ...) return a
/// `Result<System, SystemBuildError>` because the descriptor can be malformed.
pub struct FallibleSystemBuilder<'a, T>
where
    T: QueryTuple,
{
    pub(crate) desc: sys::ecs_system_desc_t,
    term_builder: TermBuilder,
    world: WorldRef<'a>,
    expr: String,
    _phantom: core::marker::PhantomData<&'a T>,
}

impl<'a, T> FallibleSystemBuilder<'a, T>
where
    T: QueryTuple,
{
    /// Specify in which phase the system should run.
    pub fn kind(mut self, phase: impl IntoEntity) -> Self {
        self.desc.phase = *phase.into_entity(self.world);
        self
    }

    /// Specify in which enum phase the system should run.
    pub fn kind_enum<Phase>(self, phase: Phase) -> Self
    where
        Phase: ComponentId + ComponentType<Enum> + EnumComponentInfo,
    {
        let enum_id = phase.id_variant(self.world());
        self.kind(enum_id)
    }

    /// Specify whether system should be ran in staged context.
    pub fn immediate(mut self, value: bool) -> Self {
        self.desc.immediate = value;
        self
    }
}

#[doc(hidden)]
impl<'a, T: QueryTuple> internals::QueryConfig<'a> for FallibleSystemBuilder<'a, T> {
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

impl<'a, T: QueryTuple> TermBuilderImpl<'a> for FallibleSystemBuilder<'a, T> {}

impl<'a, T: QueryTuple> QueryBuilderImpl<'a> for FallibleSystemBuilder<'a, T> {}

impl<'a, T: QueryTuple> WorldProvider<'a> for FallibleSystemBuilder<'a, T> {
    fn world(&self) -> WorldRef<'a> {
        self.world
    }
}

impl<'a, T> Builder<'a> for FallibleSystemBuilder<'a, T>
where
    T: QueryTuple,
{
    type BuiltType = Result<System<'a>, SystemBuildError>;

    #[doc(hidden)]
    /// Build the system, returning [`SystemBuildError`] if the descriptor is malformed.
    fn build(mut self) -> Self::BuiltType {
        if self.desc.callback.is_none() && self.desc.run.is_none() {
            panic!("you should not call this fn manually. Use `.each` , `.run` instead")
        }
        let system = System::new(self.world(), self.desc);
        let failed = *system.id() == 0;
        if failed {
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
        if failed {
            Err(SystemBuildError::InvalidExpr {
                expr: core::mem::take(&mut self.expr),
            })
        } else {
            Ok(system)
        }
    }
}

implement_reactor_api!((), FallibleSystemBuilder<'a, T>);
implement_reactor_par_api!((), FallibleSystemBuilder<'a, T>);
