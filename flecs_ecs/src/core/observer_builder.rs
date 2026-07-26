//! builder for [`Observer`].

use core::{default, ffi::c_void};

use crate::core::internals::*;
use crate::core::private::internal_SystemAPI;
use crate::core::*;
use crate::sys;

/// [`ObserverBuilder`] is used to configure and build [`Observer`]s.
///
/// Observers are systems that react to events.
/// Observers let applications register callbacks for ECS events.
///
/// These are typically constructed via [`World::observer()`].
pub struct ObserverBuilder<'a, P = (), T: QueryTuple = ()> {
    desc: sys::ecs_observer_desc_t,
    term_builder: TermBuilder,
    world: WorldRef<'a>,
    event_count: usize,
    _phantom: core::marker::PhantomData<&'a (T, P)>,
}

impl<'a, P: ComponentId, T: QueryTuple> ObserverBuilder<'a, P, T> {
    /// Create a new observer builder
    ///
    /// # Arguments
    ///
    /// * `world` - The world to create the observer in
    pub(crate) fn new(world: impl WorldProvider<'a>) -> Self {
        let desc = Default::default();
        let mut obj = Self {
            desc,
            term_builder: TermBuilder::default(),
            event_count: 1,
            world: world.world(),
            _phantom: core::marker::PhantomData,
        };

        obj.desc.events[0] = P::UnderlyingType::entity_id(world.world());

        T::populate(&mut obj);

        obj
    }

    /// Create a new observer builder with a name
    ///
    /// # Arguments
    ///
    /// * `world` - The world to create the observer in
    /// * `name` - The name of the observer
    pub fn new_named(world: impl WorldProvider<'a>, name: &str) -> Self {
        let name = compact_str::format_compact!("{}\0", name);

        let desc = Default::default();
        let mut obj = Self {
            desc,
            term_builder: TermBuilder::default(),
            event_count: 1,
            world: world.world(),
            _phantom: core::marker::PhantomData,
        };
        let entity_desc: sys::ecs_entity_desc_t = sys::ecs_entity_desc_t {
            name: name.as_ptr() as *const _,
            sep: SEPARATOR.as_ptr(),
            root_sep: SEPARATOR.as_ptr(),
            ..default::Default::default()
        };

        obj.desc.events[0] = P::entity_id(world.world());
        obj.desc.entity = unsafe { sys::ecs_entity_init(obj.world_ptr_mut(), &entity_desc) };

        T::populate(&mut obj);
        obj
    }
}

impl<'a, P, T: QueryTuple> ObserverBuilder<'a, P, T> {
    pub(crate) fn new_untyped(world: impl WorldProvider<'a>) -> ObserverBuilder<'a, (), T> {
        let desc = Default::default();
        let mut obj = ObserverBuilder {
            desc,
            term_builder: TermBuilder::default(),
            event_count: 0,
            world: world.world(),
            _phantom: core::marker::PhantomData,
        };

        T::populate(&mut obj);
        obj
    }

    /// Create a new observer builder from an existing descriptor
    ///
    /// # Arguments
    ///
    /// * `world` - The world to create the observer in
    /// * `desc` - The descriptor to create the observer from
    #[expect(dead_code)]
    pub(crate) fn new_from_desc(
        world: impl WorldProvider<'a>,
        desc: sys::ecs_observer_desc_t,
    ) -> Self {
        let mut obj = Self {
            desc,
            term_builder: TermBuilder::default(),
            event_count: 0,
            world: world.world(),
            _phantom: core::marker::PhantomData,
        };

        T::populate(&mut obj);
        obj
    }

    /// Attempts to build the observer, returning `None` if observer creation fails.
    ///
    /// This is the fallible counterpart of [`build()`](Builder::build): it returns
    /// `None` instead of a handle to an invalid entity when the underlying
    /// `ecs_observer_init` call fails, most commonly due to an invalid query
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
    pub fn try_build(&mut self) -> Option<Observer<'a>> {
        let observer = self.build();
        if *observer.id() == 0 {
            None
        } else {
            Some(observer)
        }
    }
}

impl<P, T: QueryTuple> ObserverBuilder<'_, P, T> {
    /// set observer flags, which are the same as Query flags
    ///
    /// # Arguments
    ///
    /// * `flags` - the flags to set
    pub fn set_observer_flags(&mut self, flags: ObserverFlags) -> &mut Self {
        self.desc.flags_ |= flags.bits();
        self
    }

    /// Specify the event(s) for when the observer should run.
    ///
    /// # Arguments
    ///
    /// * `event` - The event to add
    pub fn add_event(&mut self, event: impl IntoEntity) -> &mut ObserverBuilder<'_, (), T> {
        let event = *event.into_entity(self.world);
        self.desc.events[self.event_count] = event;
        self.event_count += 1;
        // SAFETY: Same layout
        unsafe { core::mem::transmute(self) }
    }

    /// Invoke observer for anything that matches its query on creation
    ///
    /// # Arguments
    ///
    /// * `should_yield` - If true, the observer will be invoked for all existing entities that match its query
    pub fn yield_existing(&mut self) -> &mut Self {
        self.desc.yield_existing = true;
        self
    }
}

#[doc(hidden)]
impl<'a, P, T: QueryTuple> internals::QueryConfig<'a> for ObserverBuilder<'a, P, T> {
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
impl<'a, P, T: QueryTuple> TermBuilderImpl<'a> for ObserverBuilder<'a, P, T> {}

impl<'a, P, T: QueryTuple> QueryBuilderImpl<'a> for ObserverBuilder<'a, P, T> {}

impl<'a, P, T> Builder<'a> for ObserverBuilder<'a, P, T>
where
    T: QueryTuple,
{
    type BuiltType = Observer<'a>;

    #[doc(hidden)]
    /// Build the `observer_builder` into an `observer`
    fn build(&mut self) -> Self::BuiltType {
        if self.desc.callback.is_none() && self.desc.run.is_none() {
            panic!("you should not call this fn manually. Use `.each` , `.run` instead")
        }
        // ensure that the observer doesn't fetch components for OnAdd events, where data is not initialized
        if self.desc.events[0] == flecs::OnAdd::ID {
            for term in self.desc.query.terms.iter_mut() {
                if (term.first.id | term.id | term.second.id | term.src.id) == 0 {
                    break;
                }

                if term.inout == sys::ecs_inout_kind_t_EcsInOutDefault as i16 {
                    term.inout = sys::ecs_inout_kind_t_EcsInOutNone as i16;
                }
            }
        }

        let observer = Observer::new(self.world(), self.desc);
        if *observer.id() == 0 {
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
        observer
    }
}

impl<'a, P, T: QueryTuple> WorldProvider<'a> for ObserverBuilder<'a, P, T> {
    fn world(&self) -> WorldRef<'a> {
        self.world
    }
}

implement_reactor_api!(ObserverBuilder<'a, P, T>);

impl<P, T: QueryTuple> core::fmt::Debug for ObserverBuilder<'_, P, T> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let qd = &self.desc.query;
        let mut ds = f.debug_struct("ObserverBuilder");
        ds.field("events", &&self.desc.events[..self.event_count]);
        ds.field("terms", &debug_term_list(&qd.terms));
        ds.finish()
    }
}

/// Updates an existing [`Observer`]'s callback or context via
/// `ecs_observer_update()`. Created with [`Observer::update()`].
pub struct ObserverUpdater<'a, P = (), T: QueryTuple = ()> {
    desc: sys::ecs_observer_desc_t,
    world: WorldRef<'a>,
    entity: EntityView<'a>,
    _phantom: core::marker::PhantomData<&'a (T, P)>,
}

impl<'a, P, T: QueryTuple> ObserverUpdater<'a, P, T> {
    pub(crate) fn new(entity: EntityView<'a>) -> Self {
        Self {
            desc: Default::default(),
            world: entity.world(),
            entity,
            _phantom: core::marker::PhantomData,
        }
    }
}

impl<'a, P, T> Builder<'a> for ObserverUpdater<'a, P, T>
where
    T: QueryTuple,
{
    type BuiltType = Observer<'a>;

    #[doc(hidden)]
    fn build(&mut self) -> Self::BuiltType {
        if self.desc.callback.is_none() && self.desc.run.is_none() {
            panic!("you should not call this fn manually. Use `.each` , `.run` instead")
        }
        unsafe {
            sys::ecs_observer_update(self.world.world_ptr_mut(), *self.entity.id(), &self.desc);
        }
        Observer::new_from_existing(self.entity)
    }
}

impl<'a, P, T: QueryTuple> WorldProvider<'a> for ObserverUpdater<'a, P, T> {
    fn world(&self) -> WorldRef<'a> {
        self.world
    }
}

implement_reactor_api!(ObserverUpdater<'a, P, T>);
