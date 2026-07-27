#[cfg(feature = "flecs_safety_locks")]
use crate::core::QueryTuple;
use crate::core::{IdOperations, IdView};

use super::WorldRef;
use flecs_ecs::sys;

/// Reserve the highest bit as the write flag.
const WRITE_FLAG: u16 = 1 << 15;
/// The remaining bits hold the read count.
const READ_MASK: u16 = WRITE_FLAG - 1;

type ComponentOrPairId = u64;
type TableId = u64;

// we use u128 over (u64, u64) to avoid the extra hash calculation and slightly better memory footprint.
type ComponentOrPairIdAndTableId = u128;

pub(crate) fn combone_ids(id: ComponentOrPairId, table_id: TableId) -> ComponentOrPairIdAndTableId {
    ((id as u128) << 64) | (table_id as u128)
}

// pub(crate) struct ReadWriteCounter {
//     counter: AtomicU16,
// }

// impl ReadWriteCounter {
//     pub(crate) fn new() -> Self {
//         Self {
//             counter: AtomicU16::new(0),
//         }
//     }

//     pub(crate) fn increment_read(&self) -> Result<(), ()> {
//         loop {
//             let curr = self.counter.load(Ordering::Relaxed);
//             if curr & WRITE_FLAG != 0 {
//                 return Err(());
//             }

//             if self
//                 .counter
//                 .compare_exchange_weak(curr, curr + 1, Ordering::Relaxed, Ordering::Relaxed)
//                 .is_ok()
//             {
//                 return Ok(());
//             }
//         }
//     }

//     pub(crate) fn decrement_read(&self) {
//         loop {
//             let curr = self.counter.load(Ordering::Relaxed);

//             debug_assert!(curr & READ_MASK != 0);

//             if self
//                 .counter
//                 .compare_exchange_weak(curr, curr - 1, Ordering::Relaxed, Ordering::Relaxed)
//                 .is_ok()
//             {
//                 break;
//             }
//         }
//     }

//     pub(crate) fn set_write(&self) -> Result<(), ()> {
//         loop {
//             let curr = self.counter.load(Ordering::Relaxed);
//             if curr != 0 {
//                 return Err(());
//             }
//             if self
//                 .counter
//                 .compare_exchange_weak(curr, WRITE_FLAG, Ordering::Relaxed, Ordering::Relaxed)
//                 .is_ok()
//             {
//                 return Ok(());
//             }
//         }
//     }

//     pub(crate) fn clear_write(&self) {
//         loop {
//             let curr = self.counter.load(Ordering::Relaxed);
//             debug_assert!(curr & WRITE_FLAG != 0);

//             if self
//                 .counter
//                 .compare_exchange_weak(curr, 0, Ordering::Relaxed, Ordering::Relaxed)
//                 .is_ok()
//             {
//                 break;
//             }
//         }
//     }
// }

// //for bulk entity test where I'm catching the panic.
// impl core::panic::RefUnwindSafe for ReadWriteComponentsMap {}

// /// A thread-safe map to track entity access
// pub(crate) struct ReadWriteComponentsMap {
//     // Maps entity ID to number of readers
//     pub(crate) read_write: DashMap<ComponentOrPairIdAndTableId, ReadWriteCounter, RandomState>,
// }

// impl ReadWriteComponentsMap {
//     pub(crate) fn new() -> Self {
//         Self {
//             read_write: DashMap::with_hasher(RandomState::default()),
//         }
//     }

//     pub(crate) fn add_entry_with(
//         &self,
//         id: ComponentOrPairIdAndTableId,
//         counter: ReadWriteCounter,
//     ) {
//         self.read_write.insert(id, counter);
//     }

//     pub(crate) fn remove_entry(&self, id: ComponentOrPairId, table_id: TableId) {
//         self.read_write.remove(&combone_ids(id, table_id));
//     }

//     pub(crate) fn increment_read(
//         &self,
//         comp_id: ComponentOrPairId,
//         table_id: TableId,
//         world: &WorldRef,
//     ) {
//         let id = combone_ids(comp_id, table_id);
//         if let Some(counter) = self.read_write.get(&id) {
//             if counter.increment_read().is_err() {
//                 panic!(
//                     "Cannot increment read: write already set for component: {} with table id: {}",
//                     {
//                         let id = IdView::new_from_id(world, comp_id);
//                         if id.is_pair() {
//                             format!(
//                                 "({}, {})",
//                                 world.entity_from_id(id.first_id()),
//                                 world.entity_from_id(id.second_id())
//                             )
//                         } else {
//                             format!("{}", id.entity_view())
//                         }
//                     },
//                     table_id
//                 );
//             }
//         } else {
//             let counter = ReadWriteCounter::new();
//             let _ = counter.increment_read();
//             self.add_entry_with(id, counter);
//         }
//     }

//     pub(crate) fn decrement_read(&self, id: ComponentOrPairId, table_id: TableId) {
//         if let Some(counter) = self.read_write.get(&combone_ids(id, table_id)) {
//             counter.decrement_read();
//         }
//     }

//     pub(crate) fn set_write(
//         &self,
//         comp_id: ComponentOrPairId,
//         table_id: TableId,
//         world: &WorldRef,
//     ) {
//         let id = combone_ids(comp_id, table_id);
//         if let Some(counter) = self.read_write.get(&id) {
//             if counter.set_write().is_err() {
//                 panic!(
//                     "Cannot set write: reads already present or write already set for component: {} with table id: {}",
//                     {
//                         let id = IdView::new_from_id(world, comp_id);
//                         if id.is_pair() {
//                             format!(
//                                 "({}, {})",
//                                 world.entity_from_id(id.first_id()),
//                                 world.entity_from_id(id.second_id())
//                             )
//                         } else {
//                             format!("{}", id.entity_view())
//                         }
//                     },
//                     table_id
//                 );
//             }
//         } else {
//             let counter = ReadWriteCounter::new();
//             let _ = counter.set_write();
//             self.add_entry_with(id, counter);
//         }
//     }

//     pub(crate) fn clear_write(&self, id: ComponentOrPairId, table_id: TableId) {
//         if let Some(counter) = self.read_write.get(&combone_ids(id, table_id)) {
//             counter.clear_write();
//         }
//     }

//     pub(crate) fn increment_counters_from_iter(
//         &self,
//         iter: &sys::ecs_iter_t,
//         world: &WorldRef,
//     ) -> SmallVec<[u8; 10]> {
//         let terms = unsafe { (*iter.query).terms };
//         let terms_count = unsafe { (*iter.query).term_count };
//         let ids = unsafe { core::slice::from_raw_parts(iter.ids, terms_count as usize) };
//         let table_id = unsafe { sys::flecs_table_id(iter.table) };
//         // we don't expect more than 20 indices
//         //TODO we can put this outside the while loop, optimize later
//         let mut indices = smallvec![0_u8; 20_usize];
//         for (i, id) in ids.iter().enumerate().take(terms_count as usize) {
//             if *id == 0 {
//                 indices.push(i as u8);
//                 continue;
//             }
//             let term = unsafe { &*terms.add(i) };

//             match term.inout as u32 {
//                 sys::ecs_inout_kind_t_EcsIn => {
//                     self.increment_read(term.id, table_id, world);
//                 }
//                 sys::ecs_inout_kind_t_EcsInOut | sys::ecs_inout_kind_t_EcsOut => {
//                     self.set_write(term.id, table_id, world);
//                 }
//                 _ => {}
//             }
//         }
//         indices
//     }

//     pub(crate) fn decrement_counters_from_iter(&self, iter: &sys::ecs_iter_t) {
//         let table_id = unsafe { sys::flecs_table_id(iter.table) };
//         let terms = unsafe { (*iter.query).terms };
//         let terms_count = unsafe { (*iter.query).term_count } as usize;

//         for term_index in 0..terms_count {
//             let term = unsafe { &*terms.add(term_index) };
//             match term.inout as u32 {
//                 sys::ecs_inout_kind_t_EcsIn => {
//                     self.decrement_read(term.id, table_id);
//                 }
//                 sys::ecs_inout_kind_t_EcsInOut | sys::ecs_inout_kind_t_EcsOut => {
//                     self.clear_write(term.id, table_id);
//                 }
//                 _ => {}
//             }
//         }
//     }

//     pub(crate) fn increment_counters_from_id(
//         &self,
//         id: ReadWriteId,
//         table_id: TableId,
//         world: &WorldRef,
//     ) {
//         match id {
//             ReadWriteId::Read(id) => {
//                 self.increment_read(id, table_id, world);
//             }
//             ReadWriteId::Write(id) => {
//                 self.set_write(id, table_id, world);
//             }
//         }
//     }

//     pub(crate) fn increment_counters_from_ids(
//         &self,
//         ids: &[ReadWriteId],
//         table_id: TableId,
//         world: &WorldRef,
//     ) {
//         for id in ids {
//             match id {
//                 ReadWriteId::Read(id) => {
//                     self.increment_read(*id, table_id, world);
//                 }
//                 ReadWriteId::Write(id) => {
//                     self.set_write(*id, table_id, world);
//                 }
//             }
//         }
//     }

//     pub(crate) fn decrement_counters_from_id(&self, id: ReadWriteId, table_id: TableId) {
//         match id {
//             ReadWriteId::Read(id) => {
//                 self.decrement_read(id, table_id);
//             }
//             ReadWriteId::Write(id) => {
//                 self.clear_write(id, table_id);
//             }
//         }
//     }

//     pub(crate) fn decrement_counters_from_ids(&self, ids: &[ReadWriteId], table_id: TableId) {
//         for id in ids {
//             match id {
//                 ReadWriteId::Read(id) => {
//                     self.decrement_read(*id, table_id);
//                 }
//                 ReadWriteId::Write(id) => {
//                     self.clear_write(*id, table_id);
//                 }
//             }
//         }
//     }

//     pub(crate) fn panic_if_any_write_is_set(&self, ids: &[u64], table_id: TableId) {
//         for id in ids {
//             if let Some(counter) = self.read_write.get(&combone_ids(*id, table_id))
//                 && counter.counter.load(Ordering::Relaxed) & WRITE_FLAG != 0
//             {
//                 panic!("Write already set");
//             }
//         }
//     }
// }

#[cfg(feature = "flecs_safety_locks")]
pub(super) const INCREMENT: bool = true;
#[cfg(feature = "flecs_safety_locks")]
pub(super) const DECREMENT: bool = false;

#[cfg(feature = "flecs_safety_locks")]
#[inline(always)]
fn lock_table<const INCREMENT: bool, const READONLY: bool, const MULTITHREADED: bool>(
    world: &WorldRef,
    table: *mut sys::ecs_table_t,
    col: i16,
    stage: i32,
) {
    if READONLY {
        if INCREMENT {
            get_table_column_lock_read_begin::<MULTITHREADED>(world, table, col, stage);
        } else {
            table_column_lock_read_end::<MULTITHREADED>(table, col, stage);
        }
    } else if INCREMENT {
        get_table_column_lock_write_begin::<MULTITHREADED>(world, table, col, stage);
    } else {
        table_column_lock_write_end::<MULTITHREADED>(table, col, stage);
    }
}

#[cfg(feature = "flecs_safety_locks")]
#[inline(always)]
fn lock_sparse<const INCREMENT: bool, const READONLY: bool, const MULTITHREADED: bool>(
    world: &WorldRef,
    idr: *mut sys::ecs_component_record_t,
) {
    if READONLY {
        if INCREMENT {
            sparse_id_record_lock_read_begin::<MULTITHREADED>(world, idr);
        } else {
            sparse_id_record_lock_read_end::<MULTITHREADED>(idr);
        }
    } else if INCREMENT {
        sparse_id_record_lock_write_begin::<MULTITHREADED>(world, idr);
    } else {
        sparse_id_record_lock_write_end::<MULTITHREADED>(idr);
    }
}

#[inline]
#[cfg(feature = "flecs_safety_locks")]
pub(crate) fn do_read_write_locks<
    const INCREMENT: bool,
    const ANY_SPARSE_TERMS: bool,
    T: QueryTuple,
>(
    world: &WorldRef,
    table_records: &[super::TableColumnSafety],
) {
    let multithreaded = world.is_currently_multithreaded();

    if multithreaded {
        let stage = world.stage_id();
        __internal_do_read_write_locks::<INCREMENT, true, ANY_SPARSE_TERMS, T>(
            world,
            stage,
            table_records,
        );
    } else {
        __internal_do_read_write_locks::<INCREMENT, false, ANY_SPARSE_TERMS, T>(
            world,
            0, /* dummy */
            table_records,
        );
    }
}

#[inline(always)]
fn __internal_do_read_write_locks<
    const INCREMENT: bool,
    const MULTITHREADED: bool,
    const ANY_SPARSE_TERMS: bool,
    T: QueryTuple,
>(
    world: &WorldRef<'_>,
    stage: i32,
    table_records: &[super::TableColumnSafety],
) {
    // Inside a multithreaded system this `WorldRef` is a stage, and
    // `flecs_components_get` asserts on `ecs_world_t_magic`, so every sparse
    // term in a threaded system aborted the process.
    let real_world = unsafe {
        sys::ecs_get_world(world.raw_world.as_ptr().cast_const().cast()).cast_mut()
    }
    .cast::<sys::ecs_world_t>();

    let count_immutable: usize = const { T::COUNT_IMMUTABLE };
    let start_index_mutable: usize = const { T::COUNT_IMMUTABLE };
    let start_index_optional_immutable: usize = const { T::COUNT_IMMUTABLE + T::COUNT_MUTABLE };
    let start_index_optional_mutable: usize =
        const { T::COUNT_IMMUTABLE + T::COUNT_MUTABLE + T::COUNT_OPTIONAL_IMMUTABLE };
    let end_index_mutable: usize = const { T::COUNT_IMMUTABLE + T::COUNT_MUTABLE };
    let end_index_optional_immutable: usize =
        const { T::COUNT_IMMUTABLE + T::COUNT_MUTABLE + T::COUNT_OPTIONAL_IMMUTABLE };
    let end_index_optional_mutable: usize = const {
        T::COUNT_IMMUTABLE
            + T::COUNT_MUTABLE
            + T::COUNT_OPTIONAL_IMMUTABLE
            + T::COUNT_OPTIONAL_MUTABLE
    };

    unsafe {
        for i in 0..count_immutable {
            let info = table_records.get_unchecked(i);

            //if component_id is set, that means this term is a row term
            if ANY_SPARSE_TERMS && info.component_id != 0 {
                let idr = sys::flecs_components_get(real_world, info.component_id);
                lock_sparse::<INCREMENT, true, MULTITHREADED>(world, idr);
                continue;
            }

            if info.table.is_null() {
                continue;
            }
            lock_table::<INCREMENT, true, MULTITHREADED>(world, info.table, info.column, stage);
        }
        for i in start_index_mutable..end_index_mutable {
            let info = table_records.get_unchecked(i);

            //if component_id is set, that means this term is a row term
            if ANY_SPARSE_TERMS && info.component_id != 0 {
                let idr = sys::flecs_components_get(real_world, info.component_id);
                lock_sparse::<INCREMENT, false, MULTITHREADED>(world, idr);
                continue;
            }

            if info.table.is_null() {
                continue;
            }
            lock_table::<INCREMENT, false, MULTITHREADED>(world, info.table, info.column, stage);
        }
        for i in start_index_optional_immutable..end_index_optional_immutable {
            //this is done by the tr.null check
            // if !sys::ecs_field_is_set(iter, i as i8) {
            //     continue;
            // }
            let info = table_records.get_unchecked(i);

            if ANY_SPARSE_TERMS && info.component_id != 0 {
                let idr = sys::flecs_components_get(real_world, info.component_id);
                lock_sparse::<INCREMENT, true, MULTITHREADED>(world, idr);
                continue;
            }

            if info.table.is_null() {
                continue;
            }
            lock_table::<INCREMENT, true, MULTITHREADED>(world, info.table, info.column, stage);
        }
        for i in start_index_optional_mutable..end_index_optional_mutable {
            //this is done by the tr.null check
            // if !sys::ecs_field_is_set(iter, i as i8) {
            //     continue;
            // }
            let info = table_records.get_unchecked(i);

            if ANY_SPARSE_TERMS && info.component_id != 0 {
                let idr = sys::flecs_components_get(real_world, info.component_id);
                lock_sparse::<INCREMENT, false, MULTITHREADED>(world, idr);
                continue;
            }

            if info.table.is_null() {
                continue;
            }
            lock_table::<INCREMENT, false, MULTITHREADED>(world, info.table, info.column, stage);
        }
    }
}

#[inline(always)]
fn component_id_from_table_column(table: *mut sys::ecs_table_t, column: i16) -> u64 {
    unsafe {
        *(*sys::ecs_table_get_type(table))
            .array
            .add(sys::ecs_table_column_to_type_index(table, column as i32) as usize)
    }
}

#[inline(always)]
pub(crate) fn sparse_id_record_lock_read_begin<const MULTITHREADED: bool>(
    world: &WorldRef,
    idr: *mut sys::ecs_component_record_t,
) {
    let val = if MULTITHREADED {
        unsafe { sys::flecs_sparse_id_record_read_begin_multithreaded(idr) }
    } else {
        unsafe { sys::flecs_sparse_id_record_read_begin(idr) }
    };
    if val {
        panic!(
            "Cannot increment read: write already set for component: {}",
            {
                let id = IdView::new_from_id(world, unsafe { sys::flecs_component_get_id(idr) });
                if id.is_pair() {
                    format!(
                        "({}, {})",
                        world.entity_from_id(id.first_id()),
                        world.entity_from_id(id.second_id())
                    )
                } else {
                    format!("{}", id.entity_view())
                }
            },
        );
    }
}

#[inline(always)]
pub(crate) fn sparse_id_record_lock_read_end<const MULTITHREADED: bool>(
    idr: *mut sys::ecs_component_record_t,
) {
    if MULTITHREADED {
        unsafe {
            sys::flecs_sparse_id_record_read_end_multithreaded(idr);
        }
    } else {
        unsafe {
            sys::flecs_sparse_id_record_read_end(idr);
        }
    }
}

#[inline(always)]
pub(crate) fn sparse_id_record_lock_write_begin<const MULTITHREADED: bool>(
    world: &WorldRef,
    idr: *mut sys::ecs_component_record_t,
) {
    let val = if MULTITHREADED {
        unsafe { sys::flecs_sparse_id_record_write_begin_multithreaded(idr) }
    } else {
        unsafe { sys::flecs_sparse_id_record_write_begin(idr) }
    };
    unsafe {
        if val {
            panic!(
                "Cannot set write: reads already present or write already set for component: {}",
                {
                    let id = IdView::new_from_id(world, sys::flecs_component_get_id(idr));
                    if id.is_pair() {
                        format!(
                            "({}, {})",
                            world.entity_from_id(id.first_id()),
                            world.entity_from_id(id.second_id())
                        )
                    } else {
                        format!("{}", id.entity_view())
                    }
                },
            );
        }
    }
}

#[inline(always)]
pub(crate) fn sparse_id_record_lock_write_end<const MULTITHREADED: bool>(
    idr: *mut sys::ecs_component_record_t,
) {
    if MULTITHREADED {
        unsafe {
            sys::flecs_sparse_id_record_write_end_multithreaded(idr);
        }
    } else {
        unsafe {
            sys::flecs_sparse_id_record_write_end(idr);
        }
    }
}

#[inline(always)]
pub(crate) fn get_table_column_lock_read_begin<const MULTITHREADED: bool>(
    world: &WorldRef,
    table: *mut sys::ecs_table_t,
    column: i16,
    _stage_id: i32,
) {
    let val = if MULTITHREADED {
        unsafe { sys::flecs_table_column_read_begin_multithreaded(table, column, _stage_id) }
    } else {
        unsafe { sys::flecs_table_column_read_begin(table, column) }
    };

    if val {
        panic!(
            "Cannot increment read: write already set for component: {}",
            {
                let id = IdView::new_from_id(world, component_id_from_table_column(table, column));
                if id.is_pair() {
                    format!(
                        "({}, {})",
                        world.entity_from_id(id.first_id()),
                        world.entity_from_id(id.second_id())
                    )
                } else {
                    format!("{}", id.entity_view())
                }
            },
        );
    }
}

#[inline(always)]
/// Returns `true` when a write is already set, in which case no read lock is held:
/// the counter increment performed by the failed attempt is rolled back.
pub(crate) fn table_column_lock_read_begin<const MULTITHREADED: bool>(
    _world: &WorldRef,
    table: *mut sys::ecs_table_t,
    column: i16,
    _stage_id: i32,
) -> bool {
    let locked = if MULTITHREADED {
        unsafe { sys::flecs_table_column_read_begin_multithreaded(table, column, _stage_id) }
    } else {
        unsafe { sys::flecs_table_column_read_begin(table, column) }
    };
    if locked {
        table_column_lock_read_end::<MULTITHREADED>(table, column, _stage_id);
    }
    locked
}

#[inline(always)]
pub(crate) fn table_column_lock_read_end<const MULTITHREADED: bool>(
    table: *mut sys::ecs_table_t,
    column: i16,
    _stage_id: i32,
) {
    if MULTITHREADED {
        unsafe {
            sys::flecs_table_column_read_end_multithreaded(table, column, _stage_id);
        }
    } else {
        unsafe {
            sys::flecs_table_column_read_end(table, column);
        }
    }
}

#[inline(always)]
/// Returns `true` when a read or write is already set, in which case no write lock is held:
/// the counter decrement performed by the failed attempt is rolled back.
pub(crate) fn table_column_lock_write_begin<const MULTITHREADED: bool>(
    _world: &WorldRef,
    table: *mut sys::ecs_table_t,
    column: i16,
    _stage_id: i32,
) -> bool {
    let locked = if MULTITHREADED {
        unsafe { sys::flecs_table_column_write_begin_multithreaded(table, column, _stage_id) }
    } else {
        unsafe { sys::flecs_table_column_write_begin(table, column) }
    };
    if locked {
        table_column_lock_write_end::<MULTITHREADED>(table, column, _stage_id);
    }
    locked
}

pub(crate) fn get_table_column_lock_write_begin<const MULTITHREADED: bool>(
    world: &WorldRef,
    table: *mut sys::ecs_table_t,
    column: i16,
    _stage_id: i32,
) {
    let val = if MULTITHREADED {
        unsafe { sys::flecs_table_column_write_begin_multithreaded(table, column, _stage_id) }
    } else {
        unsafe { sys::flecs_table_column_write_begin(table, column) }
    };

    if val {
        panic!(
            "Cannot set write: reads already present or write already set for component: {}",
            {
                let id = IdView::new_from_id(world, component_id_from_table_column(table, column));
                if id.is_pair() {
                    format!(
                        "({}, {})",
                        world.entity_from_id(id.first_id()),
                        world.entity_from_id(id.second_id())
                    )
                } else {
                    format!("{}", id.entity_view())
                }
            },
        );
    }
}

#[inline(always)]
pub(crate) fn table_column_lock_write_end<const MULTITHREADED: bool>(
    table: *mut sys::ecs_table_t,
    column: i16,
    _stage_id: i32,
) {
    if MULTITHREADED {
        unsafe { sys::flecs_table_column_write_end_multithreaded(table, column, _stage_id) }
    } else {
        unsafe { sys::flecs_table_column_write_end(table, column) }
    };
}

// #[test]
// fn read_write_counter() {
//     let counter = ReadWriteCounter::new();
//     assert!(counter.increment_read().is_ok());
//     assert!(counter.increment_read().is_ok());
//     assert!(counter.increment_read().is_ok());
//     counter.decrement_read();
//     counter.decrement_read();
//     counter.decrement_read();
//     assert!(counter.set_write().is_ok());
//     counter.clear_write();
//     assert!(counter.increment_read().is_ok());
//     assert!(counter.increment_read().is_ok());
//     counter.decrement_read();
//     counter.decrement_read();
// }

// #[test]
// fn read_write_counter_panic() {
//     let counter = ReadWriteCounter::new();
//     let _ = counter.increment_read();
//     assert!(counter.set_write().is_err());
// }
