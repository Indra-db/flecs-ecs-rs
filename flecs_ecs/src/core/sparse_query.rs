//! Direct iteration of sparse component storages.

use crate::core::*;
use crate::sys;

/// Query that iterates sparse component storages directly.
///
/// All components must declare the `DontFragment` trait at compile time via
/// `#[flecs(traits(DontFragment))]` and must not declare the
/// `(OnInstantiate, Inherit)` policy; this is checked at compile time,
/// matching the C++ `sparse_query` `static_assert`. Policies added at runtime
/// with `add_trait` are checked with a runtime assert on construction.
/// Created with [`World::sparse_query()`].
///
/// Unlike a regular [`Query`], a sparse query has no cache and does not
/// allocate; it walks the sparse storages on every iteration.
///
/// A component without the `DontFragment` trait is rejected at compile time:
///
/// ```compile_fail
/// # use flecs_ecs::prelude::*;
/// #[derive(Component)]
/// struct Position {
///     x: f32,
/// }
///
/// let world = World::new();
/// let query = world.sparse_query::<&Position>();
/// ```
///
/// As is a component that declares the `(OnInstantiate, Inherit)` policy:
///
/// ```compile_fail
/// # use flecs_ecs::prelude::*;
/// #[derive(Component)]
/// #[flecs(traits(DontFragment, (OnInstantiate, Inherit)))]
/// struct Position {
///     x: f32,
/// }
///
/// let world = World::new();
/// let query = world.sparse_query::<&Position>();
/// ```
pub struct SparseQuery<'a, T: QueryTuple> {
    world: WorldRef<'a>,
    ids: [sys::ecs_id_t; 32],
    count: usize,
    _phantom: core::marker::PhantomData<T>,
}

impl<'a, T: QueryTuple> SparseQuery<'a, T> {
    pub(crate) fn new(world: impl WorldProvider<'a>) -> Self {
        const {
            assert!(
                T::IS_SPARSE_QUERY,
                "all SparseQuery components must declare the DontFragment trait via #[flecs(traits(DontFragment))] and must not declare the (OnInstantiate, Inherit) policy"
            );
        }
        let world = world.world();
        let world_ptr = world.world_ptr_mut();
        let mut terms: [sys::ecs_term_t; 32] = Default::default();
        let mut index = 0;
        T::register_ids_descriptor_at(world_ptr, &mut terms, &mut index);

        let mut ids = [0; 32];
        for (id, term) in ids.iter_mut().zip(terms.iter()).take(index) {
            *id = term.id;
        }

        let query = Self {
            world,
            ids,
            count: index,
            _phantom: core::marker::PhantomData,
        };
        query.assert_policies();
        query
    }

    fn assert_policies(&self) {
        for &_id in &self.ids[..self.count] {
            ecs_assert!(
                unsafe {
                    sys::ecs_get_target(self.world.world_ptr_mut(), _id, ECS_ON_INSTANTIATE, 0)
                } != ECS_INHERIT,
                FlecsErrorCode::InvalidOperation,
                "sparse query component has the (OnInstantiate, Inherit) trait, which sparse queries cannot match"
            );
        }
    }

    fn storage(&self, id: sys::ecs_id_t) -> *mut sys::ecs_sparse_t {
        let cr = unsafe { sys::flecs_components_get(self.world.world_ptr_mut(), id) };
        if cr.is_null() {
            core::ptr::null_mut()
        } else {
            unsafe { sys::flecs_component_get_sparse(cr) }
        }
    }

    fn each_impl(&self, mut func: impl FnMut(sys::ecs_entity_t, &[*mut u8])) {
        let world_ptr = self.world.world_ptr_mut();
        let n = self.count;

        let mut sparse: [*mut sys::ecs_sparse_t; 32] = [core::ptr::null_mut(); 32];
        let mut sizes: [sys::ecs_size_t; 32] = [0; 32];
        for f in 0..n {
            let s = self.storage(self.ids[f]);
            if s.is_null() {
                return;
            }
            sparse[f] = s;
            let cr = unsafe { sys::flecs_components_get(world_ptr, self.ids[f]) };
            let ti = unsafe { sys::ecs_rust_get_type_info_from_record(world_ptr, self.ids[f], cr) };
            ecs_assert!(
                !ti.is_null(),
                FlecsErrorCode::InvalidOperation,
                "sparse query component is not a component"
            );
            sizes[f] = unsafe { (*ti).size };
        }

        let mut lead = 0;
        for f in 1..n {
            if unsafe { (*sparse[f]).count } < unsafe { (*sparse[lead]).count } {
                lead = f;
            }
        }

        let entities = unsafe { sys::flecs_sparse_ids(sparse[lead]) };
        let count = unsafe { (*sparse[lead]).count } - 1;

        let mut ptrs: [*mut u8; 32] = [core::ptr::null_mut(); 32];
        'entity: for i in 0..count {
            let e = unsafe { *entities.add(i as usize) };
            for f in 0..n {
                let ptr =
                    unsafe { sys::flecs_sparse_get_w_check(sparse[f], sizes[f], e, f != lead) };
                if ptr.is_null() {
                    continue 'entity;
                }
                ptrs[f] = ptr as *mut u8;
            }

            let record = unsafe { sys::ecs_record_find(world_ptr, e) };
            let table = unsafe { (*record).table };
            let flags = unsafe { sys::flecs_table_flags(table) };
            if flags & (sys::EcsTableNotQueryable | sys::EcsTableIsPrefab | sys::EcsTableIsDisabled)
                != 0
            {
                continue;
            }

            func(e, &ptrs[..n]);
        }
    }

    /// Iterate the query.
    pub fn each(&self, mut func: impl FnMut(T::TupleType<'_>)) {
        self.each_impl(|_, ptrs| {
            func(T::create_tuple(ptrs, 0));
        });
    }

    /// Iterate the query, passing the matched entity to the callback.
    pub fn each_entity(&self, mut func: impl FnMut(EntityView, T::TupleType<'_>)) {
        let world = self.world;
        self.each_impl(|e, ptrs| {
            func(EntityView::new_from(world, e), T::create_tuple(ptrs, 0));
        });
    }

    /// Return the number of entities matched by the query.
    pub fn count(&self) -> i32 {
        let mut result = 0;
        self.each_impl(|_, _| result += 1);
        result
    }

    /// Convert to a regular [`Query`] for the same components.
    pub fn to_query(&self) -> Query<T> {
        let builder = QueryBuilder::<T>::new(&self.world);
        builder.build()
    }
}
