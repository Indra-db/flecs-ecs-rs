use std::ffi::c_void;

use flecs_ecs_sys::{ecs_get_entity, ecs_iter_set_var, ecs_query_set_group};

use crate::{core::FlecsErrorCode, ecs_assert};

use super::{
    ComponentInfo, Entity, FilterT, IntoEntityId, IntoWorld, IterAPI, IterOperations, IterT,
    Iterable, WorldT,
};

pub struct IterIterable<'a, T>
where
    T: Iterable<'a>,
{
    iter: IterT,
    iter_next: unsafe extern "C" fn(*mut IterT) -> bool,
    _phantom: std::marker::PhantomData<&'a T>,
}

impl<'a, T> IterIterable<'a, T>
where
    T: Iterable<'a>,
{
    pub fn new(iter: IterT, iter_next: unsafe extern "C" fn(*mut IterT) -> bool) -> Self {
        Self {
            iter,
            iter_next,
            _phantom: std::marker::PhantomData,
        }
    }

    /// Limit results to tables with specified group id (grouped queries only)
    ///
    /// # Arguments
    ///
    /// * `group_id`: the group id to set
    ///
    /// # See also
    ///
    /// * C++ API: `iter_iterable::set_group`
    #[doc(alias = "iter_iterable::set_group")]
    pub fn set_group_id(&mut self, group_id: impl IntoEntityId) {
        unsafe { ecs_query_set_group(&mut self.iter, group_id.get_id()) }
    }

    /// Limit results to tables with specified group id (grouped queries only)
    ///
    /// # Type parameters
    ///
    /// * `Group`: the group to set
    ///
    /// # See also
    ///
    /// * C++ API: `iter_iterable::set_group`
    #[doc(alias = "iter_iterable::set_group")]
    pub fn set_group<Group: ComponentInfo>(&mut self) -> &Self {
        unsafe { ecs_query_set_group(&mut self.iter, Group::get_id(self.iter.real_world)) }
        self
    }

    /// set variable of iter
    ///
    /// # Arguments
    ///
    /// * `var_id`: the variable id to set
    ///
    /// * `value`: the value to set
    ///
    /// # See also
    ///
    /// * C++ API: `iter_iterable::set_var`
    #[doc(alias = "iter_iterable::set_var")]
    pub fn set_var(&mut self, var_id: i32, value: impl IntoEntityId) -> &Self {
        ecs_assert!(var_id != -1, FlecsErrorCode::InvalidParameter, 0);
        unsafe { ecs_iter_set_var(&mut self.iter, var_id, value.get_id()) };
        self
    }
}

impl<'a, T> IterOperations for IterIterable<'a, T>
where
    T: Iterable<'a>,
{
    fn retrieve_iter(&self) -> IterT {
        self.iter
    }

    fn iter_next(&self, iter: &mut IterT) -> bool {
        unsafe { (self.iter_next)(iter) }
    }

    fn get_filter_ptr(&self) -> *const FilterT {
        self.iter.query
    }

    fn iter_next_func(&self) -> unsafe extern "C" fn(*mut IterT) -> bool {
        self.iter_next
    }
}

impl<'a, T> IterAPI<'a, T> for IterIterable<'a, T>
where
    T: Iterable<'a>,
{
    fn as_entity(&self) -> Entity {
        Entity::new_from_existing_raw(self.iter.real_world, unsafe {
            ecs_get_entity(self.iter.query as *const c_void)
        })
    }
}

impl<'a, T> IntoWorld for IterIterable<'a, T>
where
    T: Iterable<'a>,
{
    fn get_world_raw_mut(&self) -> *mut WorldT {
        self.iter.real_world
    }
}

// TODO : worker_iterable and page_iterable not implemented yet