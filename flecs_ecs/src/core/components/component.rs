//! Registering and working with components

use core::{
    cmp::Ordering, ffi::c_void, fmt::Debug, fmt::Display, marker::PhantomData, ops::Deref,
};

use crate::core::*;
#[cfg(feature = "flecs_meta")]
use crate::prelude::FetchedId;
use crate::sys;

#[cfg(feature = "std")]
extern crate std;

extern crate alloc;
use alloc::boxed::Box;
use flecs_ecs_derive::extern_abi;

/// Component class.
/// Class used to register components and component metadata.
pub struct Component<'a, T> {
    pub base: UntypedComponent<'a>,
    _marker: PhantomData<T>,
}

impl<T> Display for Component<'_, T> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{}", self.base.entity)
    }
}

impl<T> Debug for Component<'_, T> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{:?}", self.base.entity)
    }
}

impl<T> Clone for Component<'_, T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T> Copy for Component<'_, T> {}

impl<'a, T> Deref for Component<'a, T> {
    type Target = UntypedComponent<'a>;

    fn deref(&self) -> &Self::Target {
        &self.base
    }
}

impl<'a, T: ComponentId> Component<'a, T> {
    /// Create a new component that is marked within Rust.
    ///
    /// # Arguments
    ///
    /// * `world`: the world.
    pub(crate) fn new(world: impl WorldProvider<'a>) -> Self {
        let world = world.world();
        let id = T::__register_or_get_id::<false>(world);

        let world = world.world();
        Self {
            base: UntypedComponent::new_from(world, id),
            _marker: PhantomData,
        }
    }

    /// Create a new component with a name.
    ///
    /// # Arguments
    ///
    /// * `world`: the world.
    /// * `name`: the name of the component.
    pub(crate) fn new_named(world: impl WorldProvider<'a>, name: &str) -> Self {
        let id = T::__register_or_get_id_named::<false>(world.world(), name);

        let world = world.world();
        Self {
            base: UntypedComponent::new_from(world, id),
            _marker: PhantomData,
        }
    }

    #[doc(hidden)]
    pub fn new_with_id(world: impl WorldProvider<'a>, id: impl IntoEntity) -> Self {
        Self {
            base: UntypedComponent::new_from(world, id),
            _marker: PhantomData,
        }
    }
}
impl<'a, T> Component<'a, T> {
    /// Create a new component that is not marked within Rust.
    ///
    /// # Arguments
    ///
    /// * `world`: the world.
    #[cfg(feature = "flecs_meta")]
    pub(crate) fn new_id(world: impl WorldProvider<'a>, id: FetchedId<T>) -> Self {
        let world = world.world();

        Self {
            base: UntypedComponent::new_from(world, id),
            _marker: PhantomData,
        }
    }

    /// Create a new component with a name.
    ///
    /// # Arguments
    ///
    /// * `world`: the world.
    /// * `name`: the name of the component.
    ///   Return the component as an entity
    #[cfg(feature = "flecs_meta")]
    pub fn new_named_id(world: impl WorldProvider<'a>, id: FetchedId<T>, name: &str) -> Self {
        let _name = compact_str::format_compact!("{}\0", name);
        let world = world.world();
        let entity = world.entity_from_id(id.id());
        entity.get_name().map_or_else(
            || {
                entity.set_name(name);
            },
            |current_name| {
                if current_name != name {
                    entity.set_name(name);
                }
            },
        );

        Self {
            base: UntypedComponent::new_from(world, id),
            _marker: PhantomData,
        }
    }
    /// Return the component as an entity
    #[inline(always)]
    pub fn entity(self) -> EntityView<'a> {
        self.base.entity
    }

    /// Get the binding context for the component.
    ///
    /// # Arguments
    ///
    /// * `type_hooks`: the type hooks.
    fn get_binding_context(type_hooks: &mut sys::ecs_type_hooks_t) -> &mut ComponentBindingCtx {
        let mut binding_ctx: *mut ComponentBindingCtx = type_hooks.binding_ctx as *mut _;

        if binding_ctx.is_null() {
            let new_binding_ctx = Box::<ComponentBindingCtx>::default();
            let static_ref = Box::leak(new_binding_ctx);
            binding_ctx = static_ref;
            type_hooks.binding_ctx = binding_ctx as *mut c_void;
            type_hooks.binding_ctx_free = Some(Self::binding_ctx_drop);
        }
        unsafe { &mut *binding_ctx }
    }

    /// Get the type hooks for the component.
    pub fn get_hooks(&self) -> sys::ecs_type_hooks_t {
        let type_hooks: *const sys::ecs_type_hooks_t =
            unsafe { sys::ecs_get_hooks_id(self.world.world_ptr(), *self.id) };
        if type_hooks.is_null() {
            sys::ecs_type_hooks_t::default()
        } else {
            unsafe { *type_hooks }
        }
    }

    /// Function to free the binding context.
    #[extern_abi]
    unsafe fn binding_ctx_drop(ptr: *mut c_void) {
        let ptr_struct: *mut ComponentBindingCtx = ptr as *mut ComponentBindingCtx;
        unsafe {
            drop(Box::from_raw(ptr_struct));
        }
    }

    /// Register on add hook.
    pub fn on_add<Func>(self, func: Func) -> Self
    where
        Func: FnMut(EntityView, &mut T) + 'static,
    {
        let mut type_hooks: sys::ecs_type_hooks_t = self.get_hooks();

        ecs_assert!(
            type_hooks.on_add.is_none(),
            FlecsErrorCode::InvalidOperation,
            "on_add hook already set for component {}",
            core::any::type_name::<T>()
        );

        let binding_ctx = Self::get_binding_context(&mut type_hooks);
        let boxed_func = Box::new(func);
        let static_ref = Box::leak(boxed_func);
        binding_ctx.on_add = Some(static_ref as *mut _ as *mut c_void);
        binding_ctx.free_on_add = Some(Self::on_add_drop::<Func>);
        type_hooks.on_add = Some(Self::run_add::<Func>);
        unsafe { sys::ecs_set_hooks_id(self.world.world_ptr_mut(), *self.id, &type_hooks) };
        self
    }

    /// Register on remove hook.
    pub fn on_remove<Func>(self, func: Func) -> Self
    where
        Func: FnMut(EntityView, &mut T) + 'static,
    {
        let mut type_hooks: sys::ecs_type_hooks_t = self.get_hooks();

        ecs_assert!(
            type_hooks.on_remove.is_none(),
            FlecsErrorCode::InvalidOperation,
            "on_remove hook already set for component {}",
            core::any::type_name::<T>()
        );

        let binding_ctx = Self::get_binding_context(&mut type_hooks);
        let boxed_func = Box::new(func);
        let static_ref = Box::leak(boxed_func);
        binding_ctx.on_remove = Some(static_ref as *mut _ as *mut c_void);
        binding_ctx.free_on_remove = Some(Self::on_remove_drop::<Func>);
        type_hooks.on_remove = Some(Self::run_remove::<Func>);
        unsafe { sys::ecs_set_hooks_id(self.world.world_ptr_mut(), *self.id, &type_hooks) };
        self
    }

    /// Register on set hook.
    pub fn on_set<Func>(self, func: Func) -> Self
    where
        Func: FnMut(EntityView, &mut T) + 'static,
    {
        let mut type_hooks: sys::ecs_type_hooks_t = self.get_hooks();

        ecs_assert!(
            type_hooks.on_set.is_none(),
            FlecsErrorCode::InvalidOperation,
            "on_set hook already set for component {}",
            core::any::type_name::<T>()
        );

        let binding_ctx = Self::get_binding_context(&mut type_hooks);
        let boxed_func = Box::new(func);
        let static_ref = Box::leak(boxed_func);
        binding_ctx.on_set = Some(static_ref as *mut _ as *mut c_void);
        binding_ctx.free_on_set = Some(Self::on_set_drop::<Func>);
        type_hooks.on_set = Some(Self::run_set::<Func>);
        unsafe { sys::ecs_set_hooks_id(self.world.world_ptr_mut(), *self.id, &type_hooks) };
        self
    }

    /// Register on replace hook.
    ///
    /// The callback receives `(entity, prev, next)` and is invoked before `next`
    /// is written over `prev`. It only fires when the component already existed
    /// on the entity, so `prev` is always a valid value:
    ///
    /// - Setting a component for the first time does not fire the hook.
    /// - Inside a deferred batch, "already existed" is evaluated against the
    ///   entity's state before the batch started. A component added earlier in
    ///   the same batch does not count, and none of the batched sets fire the hook.
    /// - Registering an `on_replace` hook prevents using operations that return
    ///   a mutable pointer to the component, like `get_mut`, `ensure` and `emplace`.
    pub fn on_replace<Func>(self, func: Func) -> Self
    where
        Func: FnMut(EntityView, &mut T, &mut T) + 'static,
    {
        let mut type_hooks: sys::ecs_type_hooks_t = self.get_hooks();

        ecs_assert!(
            type_hooks.on_replace.is_none(),
            FlecsErrorCode::InvalidOperation,
            "on_replace hook already set for component {}",
            core::any::type_name::<T>()
        );

        let binding_ctx = Self::get_binding_context(&mut type_hooks);
        let boxed_func = Box::new(func);
        let static_ref = Box::leak(boxed_func);
        binding_ctx.on_replace = Some(static_ref as *mut _ as *mut c_void);
        binding_ctx.free_on_replace = Some(Self::on_replace_drop::<Func>);
        type_hooks.on_replace = Some(Self::run_replace::<Func>);
        unsafe { sys::ecs_set_hooks_id(self.world.world_ptr_mut(), *self.id, &type_hooks) };
        self
    }

    /// Register a hook that is invoked before the `on_set` hook and `OnSet`
    /// observers run. When the hook returns `false`, `on_set` and `OnSet`
    /// observers are not invoked for the entity.
    pub fn on_validate<Func>(self, func: Func) -> Self
    where
        T: ComponentId,
        Func: FnMut(EntityView, &mut T) -> bool + 'static,
    {
        let mut type_hooks: sys::ecs_type_hooks_t = self.get_hooks();

        ecs_assert!(
            type_hooks.on_validate.is_none(),
            FlecsErrorCode::InvalidOperation,
            "on_validate hook already set for component {}",
            core::any::type_name::<T>()
        );

        let binding_ctx = Self::get_binding_context(&mut type_hooks);
        let boxed_func = Box::new(func);
        let static_ref = Box::leak(boxed_func);
        binding_ctx.on_validate = Some(static_ref as *mut _ as *mut c_void);
        binding_ctx.free_on_validate = Some(Self::on_validate_drop::<Func>);
        type_hooks.on_validate = Some(Self::run_validate::<Func>);
        unsafe { sys::ecs_set_hooks_id(self.world.world_ptr_mut(), *self.id, &type_hooks) };
        self
    }

    /// Function to free the on validate hook.
    #[extern_abi]
    unsafe fn on_validate_drop<Func>(func: *mut c_void)
    where
        Func: FnMut(EntityView, &mut T) -> bool + 'static,
    {
        let ptr_func: *mut Func = func as *mut Func;
        unsafe {
            drop(Box::from_raw(ptr_func));
        }
    }

    /// Function to run the on validate hook.
    #[extern_abi]
    unsafe fn run_validate<Func>(
        world: *mut sys::ecs_world_t,
        entity: sys::ecs_entity_t,
        ptr: *mut c_void,
    ) -> bool
    where
        T: ComponentId,
        Func: FnMut(EntityView, &mut T) -> bool + 'static,
    {
        unsafe {
            let world_ref = WorldRef::from_ptr(world);
            let component_id = <T as ComponentId>::entity_id(world_ref);
            let hooks = sys::ecs_get_hooks_id(world, component_id);
            let ctx = (*hooks).binding_ctx as *mut ComponentBindingCtx;
            let on_validate = (*ctx).on_validate.unwrap();
            let on_validate = &mut *(on_validate as *mut Func);
            let entity = EntityView::new_from(world_ref, entity);
            on_validate(entity, &mut *(ptr as *mut T))
        }
    }

    /// Function to free the on add hook.
    #[extern_abi]
    unsafe fn on_add_drop<Func>(func: *mut c_void)
    where
        Func: FnMut(EntityView, &mut T) + 'static,
    {
        let ptr_func: *mut Func = func as *mut Func;
        unsafe {
            drop(Box::from_raw(ptr_func));
        }
    }

    /// Function to free the on remove hook.
    #[extern_abi]
    unsafe fn on_remove_drop<Func>(func: *mut c_void)
    where
        Func: FnMut(EntityView, &mut T) + 'static,
    {
        let ptr_func: *mut Func = func as *mut Func;
        unsafe {
            drop(Box::from_raw(ptr_func));
        }
    }

    /// Function to free the on set hook.
    #[extern_abi]
    unsafe fn on_set_drop<Func>(func: *mut c_void)
    where
        Func: FnMut(EntityView, &mut T) + 'static,
    {
        let ptr_func: *mut Func = func as *mut Func;
        unsafe {
            drop(Box::from_raw(ptr_func));
        }
    }

    /// Function to free the on replace hook.
    #[extern_abi]
    unsafe fn on_replace_drop<Func>(func: *mut c_void)
    where
        Func: FnMut(EntityView, &mut T, &mut T) + 'static,
    {
        let ptr_func: *mut Func = func as *mut Func;
        unsafe {
            drop(Box::from_raw(ptr_func));
        }
    }

    /// Function to run the on add hook.
    #[extern_abi]
    unsafe fn run_add<Func>(iter: *mut sys::ecs_iter_t)
    where
        Func: FnMut(EntityView, &mut T) + 'static,
    {
        unsafe {
            let iter = &*iter;
            let ctx: *mut ComponentBindingCtx = iter.callback_ctx as *mut _;
            let on_add = (*ctx).on_add.unwrap();
            let on_add = on_add as *mut Func;
            let on_add = &mut *on_add;
            let world = WorldRef::from_ptr(iter.world);
            let entity = EntityView::new_from(world, *iter.entities);
            let component = if (iter.ref_fields | iter.up_fields) == 0 {
                flecs_field::<T>(iter, 0)
            } else {
                flecs_field_at::<T>(iter, 0, 0)
            };
            on_add(entity, &mut *component);
        }
    }

    /// Function to run the on set hook.
    #[extern_abi]
    unsafe fn run_set<Func>(iter: *mut sys::ecs_iter_t)
    where
        Func: FnMut(EntityView, &mut T) + 'static,
    {
        let iter = unsafe { &*iter };
        let ctx: *mut ComponentBindingCtx = iter.callback_ctx as *mut _;
        let on_set = unsafe { (*ctx).on_set.unwrap() };
        let on_set = on_set as *mut Func;
        let on_set = unsafe { &mut *on_set };
        let world = unsafe { WorldRef::from_ptr(iter.world) };
        let entity = EntityView::new_from(world, unsafe { *iter.entities });
        let component = if (iter.ref_fields | iter.up_fields) == 0 {
            flecs_field::<T>(iter, 0)
        } else {
            unsafe { flecs_field_at::<T>(iter, 0, 0) }
        };
        on_set(entity, unsafe { &mut *component });
    }

    /// Function to run the on replace hook.
    #[extern_abi]
    unsafe fn run_replace<Func>(iter: *mut sys::ecs_iter_t)
    where
        Func: FnMut(EntityView, &mut T, &mut T) + 'static,
    {
        let iter = unsafe { &*iter };

        // other_table is the entity's pre-operation table, set by
        // flecs_invoke_replace_hook. The component existed before the
        // operation only if other_table contains it (upstream flecs contract:
        // "to find out whether the component existed before the operation,
        // call ecs_table_has_id() on other_table"). If it did not exist,
        // `prev` points to unconstructed memory — not a valid Rust value —
        // so the hook is skipped entirely.
        let existed = !iter.other_table.is_null()
            && unsafe { sys::ecs_table_has_id(iter.real_world, iter.other_table, iter.event_id) };
        if !existed {
            return;
        }

        let ctx: *mut ComponentBindingCtx = iter.callback_ctx as *mut _;
        let on_replace = unsafe { (*ctx).on_replace.unwrap() };
        let on_replace = on_replace as *mut Func;
        let on_replace = unsafe { &mut *on_replace };
        let world = unsafe { WorldRef::from_ptr(iter.world) };
        let entity = EntityView::new_from(world, unsafe { *iter.entities });
        let (prev, next) = if (iter.ref_fields | iter.up_fields) == 0 {
            (flecs_field::<T>(iter, 0), flecs_field::<T>(iter, 1))
        } else {
            unsafe {
                (
                    flecs_field_at::<T>(iter, 0, 0),
                    flecs_field_at::<T>(iter, 1, 0),
                )
            }
        };
        on_replace(entity, unsafe { &mut *prev }, unsafe { &mut *next });
    }

    /// Function to run the on remove hook.
    #[extern_abi]
    unsafe fn run_remove<Func>(iter: *mut sys::ecs_iter_t)
    where
        Func: FnMut(EntityView, &mut T) + 'static,
    {
        unsafe {
            let iter = &*iter;
            let ctx: *mut ComponentBindingCtx = iter.callback_ctx as *mut _;
            let on_remove = (*ctx).on_remove.unwrap();
            let on_remove = on_remove as *mut Func;
            let on_remove = &mut *on_remove;
            let world = WorldRef::from_ptr(iter.world);
            let entity = EntityView::new_from(world, *iter.entities);
            let component = if (iter.ref_fields | iter.up_fields) == 0 {
                flecs_field::<T>(iter, 0)
            } else {
                flecs_field_at::<T>(iter, 0, 0)
            };
            on_remove(entity, &mut *component);
        }
    }

    /// Register compare hook with a custom function.
    ///
    /// For types that implement [`PartialOrd`], the hook is registered automatically.
    /// Use this method for types that don't implement `PartialOrd` or need custom ordering.
    ///
    /// # Arguments
    /// * `cmp`: pure function returning `Ordering` for two component references
    pub fn on_compare(self, cmp: fn(&T, &T) -> core::cmp::Ordering) -> Self {
        let mut type_hooks: sys::ecs_type_hooks_t = self.get_hooks();

        // Guard against double-registering a *custom* fn. Overriding the auto-registered
        // panic stub (set when T does not impl PartialOrd) is allowed.
        let binding_ctx_ptr = type_hooks.binding_ctx as *const ComponentBindingCtx;
        let _already_custom =
            !binding_ctx_ptr.is_null() && unsafe { (*binding_ctx_ptr).on_compare.is_some() };
        ecs_assert!(
            !_already_custom,
            FlecsErrorCode::InvalidOperation,
            "cmp hook already set for component {}",
            core::any::type_name::<T>()
        );

        let binding_ctx = Self::get_binding_context(&mut type_hooks);
        binding_ctx.on_compare = Some(cmp as *mut c_void);
        type_hooks.cmp = Some(Self::run_compare::<T>);
        unsafe { sys::ecs_set_hooks_id(self.world.world_ptr_mut(), *self.id, &type_hooks) };
        self
    }

    /// Register equals hook with a custom function.
    ///
    /// For types that implement [`PartialEq`], the hook is registered automatically.
    /// Use this method for types that don't implement `PartialEq` or need custom equality.
    ///
    /// # Arguments
    /// * `eq`: pure function returning `bool` for two component references
    pub fn on_equals(self, eq: fn(&T, &T) -> bool) -> Self {
        let mut type_hooks: sys::ecs_type_hooks_t = self.get_hooks();

        // Guard against double-registering a *custom* fn. Overriding the auto-registered
        // panic stub (set when T does not impl PartialEq) is allowed.
        let binding_ctx_ptr = type_hooks.binding_ctx as *const ComponentBindingCtx;
        let _already_custom =
            !binding_ctx_ptr.is_null() && unsafe { (*binding_ctx_ptr).on_equals.is_some() };
        ecs_assert!(
            !_already_custom,
            FlecsErrorCode::InvalidOperation,
            "equals hook already set for component {}",
            core::any::type_name::<T>()
        );

        let binding_ctx = Self::get_binding_context(&mut type_hooks);
        binding_ctx.on_equals = Some(eq as *mut c_void);
        type_hooks.equals = Some(Self::run_equals::<T>);
        unsafe { sys::ecs_set_hooks_id(self.world.world_ptr_mut(), *self.id, &type_hooks) };
        self
    }

    /// Trampoline for the custom compare hook.
    #[extern_abi]
    unsafe fn run_compare<U>(
        a: *const c_void,
        b: *const c_void,
        type_info: *const sys::ecs_type_info_t,
    ) -> i32 {
        // SAFETY: binding_ctx is set in on_compare before this callback is registered
        let ctx = unsafe { (*type_info).hooks.binding_ctx as *const ComponentBindingCtx };
        let func_ptr = unsafe { (*ctx).on_compare.unwrap() };
        let func: fn(&U, &U) -> core::cmp::Ordering = unsafe { core::mem::transmute(func_ptr) };
        match func(unsafe { &*(a as *const U) }, unsafe { &*(b as *const U) }) {
            core::cmp::Ordering::Less => -1,
            core::cmp::Ordering::Equal => 0,
            core::cmp::Ordering::Greater => 1,
        }
    }

    /// Trampoline for the custom equals hook.
    #[extern_abi]
    unsafe fn run_equals<U>(
        a: *const c_void,
        b: *const c_void,
        type_info: *const sys::ecs_type_info_t,
    ) -> bool {
        // SAFETY: binding_ctx is set in on_equals before this callback is registered
        let ctx = unsafe { (*type_info).hooks.binding_ctx as *const ComponentBindingCtx };
        let func_ptr = unsafe { (*ctx).on_equals.unwrap() };
        let func: fn(&U, &U) -> bool = unsafe { core::mem::transmute(func_ptr) };
        func(unsafe { &*(a as *const U) }, unsafe { &*(b as *const U) })
    }

    /// Compare two values using the registered comparison hook.
    ///
    /// Returns `Some(Ordering)` if a comparison hook is registered, `None` otherwise.
    ///
    /// # Arguments
    ///
    /// * `a` - First value to compare
    /// * `b` - Second value to compare
    ///
    /// # Returns
    ///
    /// `Some(Ordering::Less)` if `a < b`, `Some(Ordering::Equal)` if `a == b`,
    /// `Some(Ordering::Greater)` if `a > b`, or `None` if no hook is registered.
    pub fn compare(&self, a: &T, b: &T) -> Option<Ordering> {
        use crate::core::WorldProvider;

        unsafe {
            let ti_ptr =
                sys::ecs_get_type_info(self.base.entity.world.world_ptr() as *mut _, *self.id);
            if ti_ptr.is_null() {
                return None;
            }
            let ti = &*ti_ptr;

            let cmp_fn = ti.hooks.cmp?;
            let result = cmp_fn(
                a as *const T as *const c_void,
                b as *const T as *const c_void,
                ti,
            );
            Some(match result {
                r if r < 0 => Ordering::Less,
                r if r > 0 => Ordering::Greater,
                _ => Ordering::Equal,
            })
        }
    }

    /// Check equality using the registered equals hook.
    ///
    /// Returns `Some(bool)` if an equals hook is registered, `None` otherwise.
    ///
    /// # Arguments
    ///
    /// * `a` - First value to compare
    /// * `b` - Second value to compare
    ///
    /// # Returns
    ///
    /// `Some(true)` if `a == b`, `Some(false)` if `a != b`, or `None` if no hook is registered.
    pub fn are_equal(&self, a: &T, b: &T) -> Option<bool> {
        use crate::core::WorldProvider;

        unsafe {
            let ti_ptr =
                sys::ecs_get_type_info(self.base.entity.world.world_ptr() as *mut _, *self.id);
            if ti_ptr.is_null() {
                return None;
            }
            let ti = &*ti_ptr;

            let eq_fn = ti.hooks.equals?;
            let result = eq_fn(
                a as *const T as *const c_void,
                b as *const T as *const c_void,
                ti,
            );
            Some(result)
        }
    }
}

mod eq_operations {
    use super::*;

    impl<'a, T: ComponentId> PartialEq<Component<'a, T>> for u64 {
        #[inline]
        fn eq(&self, other: &Component<'a, T>) -> bool {
            *self == other.base.entity.id
        }
    }

    impl<T: ComponentId> PartialEq<u64> for Component<'_, T> {
        #[inline]
        fn eq(&self, other: &u64) -> bool {
            self.base.entity.id == *other
        }
    }

    impl<T: ComponentId> PartialEq<Entity> for Component<'_, T> {
        #[inline]
        fn eq(&self, other: &Entity) -> bool {
            self.base.entity.id == *other
        }
    }

    impl<T: ComponentId> PartialEq<Id> for Component<'_, T> {
        #[inline]
        fn eq(&self, other: &Id) -> bool {
            self.base.entity.id == *other
        }
    }

    impl<'a, T: ComponentId> PartialEq<EntityView<'a>> for Component<'a, T> {
        #[inline]
        fn eq(&self, other: &EntityView<'a>) -> bool {
            self.base.entity == *other
        }
    }

    impl<'a, T: ComponentId> PartialEq<IdView<'a>> for Component<'a, T> {
        #[inline]
        fn eq(&self, other: &IdView<'a>) -> bool {
            self.base.entity == other.id
        }
    }

    impl<'a, T: ComponentId> PartialEq<UntypedComponent<'a>> for Component<'a, T> {
        #[inline]
        fn eq(&self, other: &UntypedComponent<'a>) -> bool {
            self.base.entity == other.entity
        }
    }

    impl<T: ComponentId> PartialEq for Component<'_, T> {
        #[inline]
        fn eq(&self, other: &Self) -> bool {
            self.base.entity == other.base.entity
        }
    }

    impl<T: ComponentId> Eq for Component<'_, T> {}

    impl<'a, T: ComponentId> PartialOrd<Component<'a, T>> for u64 {
        #[inline]
        fn partial_cmp(&self, other: &Component<'a, T>) -> Option<core::cmp::Ordering> {
            self.partial_cmp(&other.base.entity.id)
        }
    }
}

mod ord_operations {
    use super::*;
    impl<T: ComponentId> PartialOrd<u64> for Component<'_, T> {
        #[inline]
        fn partial_cmp(&self, other: &u64) -> Option<core::cmp::Ordering> {
            self.base.entity.id.partial_cmp(other)
        }
    }

    impl<T: ComponentId> PartialOrd<Entity> for Component<'_, T> {
        #[inline]
        fn partial_cmp(&self, other: &Entity) -> Option<core::cmp::Ordering> {
            self.base.entity.id.partial_cmp(other)
        }
    }

    impl<T: ComponentId> PartialOrd<Id> for Component<'_, T> {
        #[inline]
        fn partial_cmp(&self, other: &Id) -> Option<core::cmp::Ordering> {
            self.base.entity.id.partial_cmp(other)
        }
    }

    impl<'a, T: ComponentId> PartialOrd<EntityView<'a>> for Component<'a, T> {
        #[inline]
        fn partial_cmp(&self, other: &EntityView<'a>) -> Option<core::cmp::Ordering> {
            self.base.entity.partial_cmp(other)
        }
    }

    impl<'a, T: ComponentId> PartialOrd<IdView<'a>> for Component<'a, T> {
        #[inline]
        fn partial_cmp(&self, other: &IdView<'a>) -> Option<core::cmp::Ordering> {
            self.base.entity.partial_cmp(&other.id)
        }
    }

    impl<'a, T: ComponentId> PartialOrd<UntypedComponent<'a>> for Component<'a, T> {
        #[inline]
        fn partial_cmp(&self, other: &UntypedComponent<'a>) -> Option<core::cmp::Ordering> {
            self.base.entity.partial_cmp(&other.entity)
        }
    }

    impl<T: ComponentId> PartialOrd for Component<'_, T> {
        #[inline]
        fn partial_cmp(&self, other: &Self) -> Option<core::cmp::Ordering> {
            Some(self.cmp(other))
        }
    }

    impl<T: ComponentId> Ord for Component<'_, T> {
        #[inline]
        fn cmp(&self, other: &Self) -> core::cmp::Ordering {
            self.base.entity.cmp(&other.base.entity)
        }
    }
}
