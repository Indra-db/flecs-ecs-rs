//! [`AlertBuilder`] is a builder pattern for creating [`Alert`]s.

use super::*;
use crate::core::internals::*;
use crate::core::*;
use crate::sys;

use core::mem::ManuallyDrop;

#[cfg(feature = "std")]
extern crate std;

extern crate alloc;
use alloc::string::ToString;
use alloc::{format, string::String, vec::Vec};

/// [`AlertBuilder`] is a builder pattern for creating [`Alert`]s.
pub struct AlertBuilder<'a, T>
where
    T: QueryTuple,
{
    pub(crate) desc: sys::ecs_alert_desc_t,
    term_builder: TermBuilder,
    world: WorldRef<'a>,
    severity_filter_count: i32,
    str_ptrs_to_free: Vec<ManuallyDrop<String>>,
    _phantom: core::marker::PhantomData<&'a T>,
}

impl<T> Drop for AlertBuilder<'_, T>
where
    T: QueryTuple,
{
    fn drop(&mut self) {
        for s in self.str_ptrs_to_free.iter_mut() {
            unsafe { ManuallyDrop::drop(s) };
        }
        self.str_ptrs_to_free.clear();
    }
}

impl<'a, T> AlertBuilder<'a, T>
where
    T: QueryTuple,
{
    /// Create a new `AlertBuilder`
    pub(crate) fn new(world: &'a World) -> Self {
        let mut obj = Self {
            desc: Default::default(),
            term_builder: TermBuilder::default(),
            world: world.into(),
            severity_filter_count: 0,
            str_ptrs_to_free: Vec::new(),
            _phantom: core::marker::PhantomData,
        };

        T::populate(&mut obj);

        obj
    }

    pub(crate) fn new_from_desc(world: &'a World, desc: sys::ecs_alert_desc_t) -> Self {
        let mut obj = Self {
            desc,
            term_builder: TermBuilder::default(),
            world: world.into(),
            severity_filter_count: 0,
            str_ptrs_to_free: Vec::new(),
            _phantom: core::marker::PhantomData,
        };

        if obj.desc.entity == 0 {
            obj.desc.entity =
                unsafe { sys::ecs_entity_init(obj.world_ptr_mut(), &Default::default()) };
        }

        T::populate(&mut obj);

        #[cfg(feature = "flecs_pipeline")]
        unsafe {
            sys::ecs_add_id(
                world.world_ptr_mut(),
                obj.desc.entity,
                ecs_dependson(ECS_ON_UPDATE),
            );
            sys::ecs_add_id(world.world_ptr_mut(), obj.desc.entity, ECS_ON_UPDATE);
        }

        obj
    }

    /// Create a new `AlertBuilder` with a name
    pub(crate) fn new_named(world: &'a World, name: &str) -> Self {
        let name = compact_str::format_compact!("{}\0", name);

        let mut obj = Self {
            desc: Default::default(),
            term_builder: TermBuilder::default(),
            world: world.into(),
            severity_filter_count: 0,
            str_ptrs_to_free: Vec::new(),
            _phantom: core::marker::PhantomData,
        };

        let entity_desc = sys::ecs_entity_desc_t {
            name: name.as_ptr() as *const _,
            sep: SEPARATOR.as_ptr(),
            root_sep: SEPARATOR.as_ptr(),
            ..Default::default()
        };

        obj.desc.entity = unsafe { sys::ecs_entity_init(world.world_ptr_mut(), &entity_desc) };

        T::populate(&mut obj);
        obj
    }

    /// Set the alert message.
    ///
    /// # Arguments
    ///
    /// * `message` - The alert message.
    ///
    /// # See also
    ///
    /// * `ecs_alert_desc_t::message`
    pub fn message(mut self, message: &str) -> Self {
        let message = ManuallyDrop::new(format!("{message}\0"));
        self.desc.message = message.as_ptr() as *const _;
        self.str_ptrs_to_free.push(message);
        self
    }

    /// Set brief description for alert.
    ///
    /// # Arguments
    ///
    /// * `brief` - Brief description.
    ///
    /// # See also
    ///
    /// * `ecs_alert_desc_t::brief`
    pub fn brief(mut self, brief: &str) -> Self {
        let brief = ManuallyDrop::new(format!("{brief}\0"));
        self.desc.brief = brief.as_ptr() as *const _;
        self.str_ptrs_to_free.push(brief);
        self
    }

    /// Set doc name for alert.
    ///
    /// # Arguments
    ///
    /// * `doc_name` - Documentation name.
    ///
    /// # See also
    ///
    /// * `ecs_alert_desc_t::doc_name`
    pub fn doc_name(mut self, doc_name: &str) -> Self {
        let doc_name = ManuallyDrop::new(format!("{doc_name}\0"));
        self.desc.doc_name = doc_name.as_ptr() as *const _;
        self.str_ptrs_to_free.push(doc_name);
        self
    }

    /// Set severity of alert (default is Error).
    ///
    /// # Arguments
    ///
    /// * `severity` - The severity entity.
    ///
    /// # See also
    ///
    /// * `ecs_alert_desc_t::severity`
    pub fn severity(mut self, severity: impl IntoEntity) -> Self {
        self.desc.severity = *severity.into_entity(self.world);
        self
    }

    /// Set retain period of alert.
    ///
    /// # Arguments
    ///
    /// * `period` - Retain period.
    ///
    /// # See also
    ///
    /// * `ecs_alert_desc_t::retain_period`
    pub fn retain_period(mut self, period: f32) -> Self {
        self.desc.retain_period = period;
        self
    }

    /// Add severity filter.
    ///
    /// # Arguments
    ///
    /// * `severity` - Severity entity.
    /// * `with` - Filter with this id.
    /// * `var` - Variable name (optional).
    ///
    /// # See also
    ///
    /// * `ecs_alert_desc_t::severity_filters`
    pub fn severity_filter(
        mut self,
        severity: impl IntoEntity,
        with: impl IntoId,
        var: Option<&str>,
    ) -> Self {
        ecs_assert!(
            self.severity_filter_count < sys::ECS_ALERT_MAX_SEVERITY_FILTERS as i32,
            "Maximum number of severity filters reached"
        );

        let filter = &mut self.desc.severity_filters[self.severity_filter_count as usize];
        self.severity_filter_count += 1;
        filter.severity = *severity.into_entity(self.world);
        filter.with = *with.into_id(self.world);
        if let Some(var) = var {
            let var = ManuallyDrop::new(format!("{var}\0"));
            filter.var = var.as_ptr() as *const _;
            self.str_ptrs_to_free.push(var);
        }
        self
    }

    /// Add severity filter for non-enum components.
    ///
    /// # Type Parameters
    ///
    /// * `Severity` - Severity component.
    /// * `T` - Component type.
    ///
    /// # Arguments
    ///
    /// * `var` - Variable name (optional).
    ///
    /// # See also
    ///
    /// * `ecs_alert_desc_t::severity_filters`
    pub fn severity_filter_component<Severity, With>(self, var: Option<&str>) -> Self
    where
        Severity: ComponentId,
        With: ComponentId + ComponentType<Struct>,
    {
        let severity_id = Severity::entity_id(self.world());
        let with_id = With::entity_id(self.world());
        self.severity_filter(severity_id, with_id, var)
    }

    /// Add severity filter for enum components.
    ///
    /// # Type Parameters
    ///
    /// * `Severity` - Severity component.
    /// * `T` - Enum component type.
    ///
    /// # Arguments
    ///
    /// * `with` - Enum variant.
    /// * `var` - Variable name (optional).
    ///
    /// # See also
    ///
    /// * `ecs_alert_desc_t::severity_filters`
    pub fn severity_filter_enum<Severity, With>(
        self,
        with: With,
        var: Option<&str>,
    ) -> Self
    where
        Severity: ComponentId,
        With: EnumComponentInfo + ComponentType<Enum>,
    {
        let world = self.world();
        let severity_id = Severity::entity_id(world);
        let with_id = With::entity_id(world);
        let constant_id = with.id_variant(world);
        let pair_id = ecs_pair(with_id, *constant_id.id());
        self.severity_filter(severity_id, pair_id, var)
    }

    /// Set member to create an alert for out of range values.
    ///
    /// # Arguments
    ///
    /// * `member` - Member entity.
    ///
    /// # See also
    ///
    /// * `ecs_alert_desc_t::member`
    pub fn member(mut self, member: impl IntoEntity) -> Self {
        self.desc.member = *member.into_entity(self.world);
        self
    }

    /// Set (component) id for member (optional). If `.member()` is set and id
    /// is not set, the id will default to the member parent.
    ///
    /// # Arguments
    ///
    /// * `id` - Component id.
    ///
    /// # See also
    ///
    /// * `ecs_alert_desc_t::id`
    pub fn id(mut self, id: impl IntoId) -> Self {
        self.desc.id = *id.into_id(self.world);
        self
    }

    /// Set member to create an alert for out of range values.
    ///
    /// # Type Parameters
    ///
    /// * `T` - Component type.
    ///
    /// # Arguments
    ///
    /// * `member_name` - Member name.
    /// * `var` - Variable name (optional).
    ///
    /// # See also
    ///
    /// * `ecs_alert_desc_t::member`
    pub fn member_type<With>(mut self, member_name: &str, var: Option<&str>) -> Self
    where
        With: ComponentId,
    {
        let member_name = compact_str::format_compact!("{}\0", member_name);
        let world = self.world();
        let id = With::entity_id(world);
        let member_id = unsafe {
            sys::ecs_lookup_path_w_sep(
                world.world_ptr_mut(),
                id,
                member_name.as_ptr() as *const _,
                SEPARATOR.as_ptr(),
                SEPARATOR.as_ptr(),
                false,
            )
        };

        ecs_assert!(
            member_id != 0,
            FlecsErrorCode::InvalidParameter,
            "Member {} not found in component {}",
            member_name,
            core::any::type_name::<With>()
        );

        if let Some(var) = var {
            let var = ManuallyDrop::new(format!("{var}\0"));
            self.desc.var = var.as_ptr() as *const _;
            self.str_ptrs_to_free.push(var);
        }

        self.member(member_id)
    }

    /// Set source variable for member (optional, defaults to `$this`).
    ///
    /// # Arguments
    ///
    /// * `var` - Variable name.
    ///
    /// # See also
    ///
    /// * `ecs_alert_desc_t::var`
    pub fn var(mut self, var: &str) -> Self {
        let var = ManuallyDrop::new(format!("{var}\0"));
        self.desc.var = var.as_ptr() as *const _;
        self.str_ptrs_to_free.push(var);
        self
    }
}

#[doc(hidden)]
impl<'a, T: QueryTuple> internals::QueryConfig<'a> for AlertBuilder<'a, T> {
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

impl<'a, T: QueryTuple> TermBuilderImpl<'a> for AlertBuilder<'a, T> {}

impl<'a, T: QueryTuple> QueryBuilderImpl<'a> for AlertBuilder<'a, T> {}

impl<'a, T> Builder<'a> for AlertBuilder<'a, T>
where
    T: QueryTuple,
{
    type BuiltType = Alert<'a>;

    /// Build the `AlertBuilder` into an Alert
    fn build(mut self) -> Self::BuiltType {
        let alert = Alert::new(self.world(), self.desc);
        for s in self.term_builder.str_ptrs_to_free.iter_mut() {
            unsafe { ManuallyDrop::drop(s) };
        }
        self.term_builder.str_ptrs_to_free.clear();
        alert
    }
}

impl<'a, T> AlertBuilder<'a, T>
where
    T: QueryTuple,
{
    /// Set a runtime query expression, transitioning to the fallible-build typestate.
    ///
    /// A purely-typed alert builder is infallible: [`build()`](Builder::build) returns
    /// an [`Alert`] directly. A runtime expression string can fail to parse, so calling
    /// `expr()` consumes the builder and returns a [`FallibleAlertBuilder`] whose
    /// terminal [`build()`](FallibleAlertBuilder::build) returns a
    /// `Result<Alert, AlertBuildError>`. All remaining configuration methods are still
    /// available on the returned builder.
    pub fn expr(self, expr: &str) -> FallibleAlertBuilder<'a, T> {
        let inner = QueryBuilderImpl::expr(self, expr);
        FallibleAlertBuilder {
            inner,
            expr: expr.to_string(),
        }
    }
}

/// A malformed alert construction reported by the fallible-build typestate.
///
/// Only a builder that took a runtime [`expr()`](AlertBuilder::expr) can produce this
/// error; a purely-typed alert builder is infallible.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum AlertBuildError {
    /// The runtime `expr()` string failed to parse or validate.
    InvalidExpr {
        /// The offending expression string.
        expr: String,
    },
    /// `ecs_alert_init` rejected the descriptor for another reason.
    Init,
}

impl core::fmt::Display for AlertBuildError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            AlertBuildError::InvalidExpr { expr } => {
                write!(f, "invalid alert query expression: {expr:?}")
            }
            AlertBuildError::Init => write!(f, "alert initialization failed"),
        }
    }
}

impl core::error::Error for AlertBuildError {}

/// The fallible-build typestate of [`AlertBuilder`], entered via
/// [`AlertBuilder::expr()`].
pub struct FallibleAlertBuilder<'a, T>
where
    T: QueryTuple,
{
    inner: AlertBuilder<'a, T>,
    expr: String,
}

#[doc(hidden)]
impl<'a, T: QueryTuple> internals::QueryConfig<'a> for FallibleAlertBuilder<'a, T> {
    #[inline(always)]
    fn term_builder(&self) -> &TermBuilder {
        self.inner.term_builder()
    }

    #[inline(always)]
    fn term_builder_mut(&mut self) -> &mut TermBuilder {
        self.inner.term_builder_mut()
    }

    #[inline(always)]
    fn query_desc(&self) -> &sys::ecs_query_desc_t {
        self.inner.query_desc()
    }

    #[inline(always)]
    fn query_desc_mut(&mut self) -> &mut sys::ecs_query_desc_t {
        self.inner.query_desc_mut()
    }

    #[inline(always)]
    fn count_generic_terms(&self) -> i32 {
        self.inner.count_generic_terms()
    }
}

impl<'a, T: QueryTuple> TermBuilderImpl<'a> for FallibleAlertBuilder<'a, T> {}

impl<'a, T: QueryTuple> QueryBuilderImpl<'a> for FallibleAlertBuilder<'a, T> {}

impl<'a, T: QueryTuple> WorldProvider<'a> for FallibleAlertBuilder<'a, T> {
    fn world(&self) -> WorldRef<'a> {
        self.inner.world()
    }
}

impl<'a, T> Builder<'a> for FallibleAlertBuilder<'a, T>
where
    T: QueryTuple,
{
    type BuiltType = Result<Alert<'a>, AlertBuildError>;

    /// Build the alert, returning [`AlertBuildError`] if the descriptor is malformed.
    fn build(mut self) -> Self::BuiltType {
        let alert = self.inner.build();
        if *alert.id() == 0 {
            Err(AlertBuildError::InvalidExpr {
                expr: core::mem::take(&mut self.expr),
            })
        } else {
            Ok(alert)
        }
    }
}

impl<'a, T: QueryTuple> WorldProvider<'a> for AlertBuilder<'a, T> {
    fn world(&self) -> WorldRef<'a> {
        self.world
    }
}
