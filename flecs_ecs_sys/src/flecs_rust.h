#include "flecs.h"

typedef struct ecs_rust_set_t {
    void *ptr;
    bool call_modified;
    bool is_new;
} ecs_rust_set_t;

FLECS_API
    int32_t ecs_rust_rel_count(
    const ecs_world_t *world,
    ecs_id_t id,
    ecs_table_t* table);

FLECS_API
ecs_entity_t ecs_rust_get_typeid(
    const ecs_world_t *world,
    ecs_id_t id,
    const ecs_component_record_t* idr);

FLECS_API
const ecs_type_info_t* ecs_rust_get_type_info_from_record(
    const ecs_world_t *world,
    ecs_id_t id,
    const ecs_component_record_t* idr);

FLECS_API
ecs_rust_set_t ecs_rust_set(
    ecs_world_t *world,
    ecs_entity_t entity,
    ecs_id_t id,
    const void *new_ptr,
    size_t size);

/* Component pointer plus the key the Rust side uses for its mut-alias
 * tracking. ptr is NULL when the entity does not have the component.
 *
 * Key encoding (must stay in sync with safety_map.rs dense_lock_key /
 * sparse_lock_key): dense storage = ((uintptr_t)table << 16) | column, which
 * may name an IsA base entity's table for inherited reads; sparse /
 * non-fragmenting storage = (uintptr_t)component_record | (1ull << 63).
 * Assumes user-space pointers stay below 2^47 (true on all tier-1 targets).
 * 16 bytes so the struct returns in registers. */
typedef struct ecs_rust_get_ptr_t {
    void *ptr;
    uint64_t lock_key;
} ecs_rust_get_ptr_t;

/* Get an immutable component pointer by entity record, including components
 * inherited through an IsA relationship. Mirrors ecs_get_id() but skips the
 * repeat record lookup and reports the storage origin. */
FLECS_API
ecs_rust_get_ptr_t ecs_rust_record_get_id(
    const ecs_world_t *world,
    ecs_entity_t entity,
    const ecs_record_t *r,
    ecs_id_t component);

/* Get a mutable component pointer by entity record. Does not return
 * inherited components, mirroring ecs_get_mut_id(). */
FLECS_API
ecs_rust_get_ptr_t ecs_rust_record_get_mut_id(
    const ecs_world_t *world,
    const ecs_record_t *r,
    ecs_id_t component);

/* Fast path for compile-time-known sparse / dont_fragment components without
 * the (OnInstantiate, Inherit) trait. Mirrors ecs_get_sparse_id() but reports
 * the storage origin. */
FLECS_API
ecs_rust_get_ptr_t ecs_rust_get_sparse_id(
    const ecs_world_t *world,
    ecs_entity_t entity,
    ecs_id_t id,
    size_t size);

/* Combined ecs_ref_get_id + defer_begin for cached refs, the entry half of a
 * component access scope (ecs_rust_scope_end is the exit half). Returns a
 * NULL ptr without starting a defer scope when the component is gone. A
 * lock_key of 0 means the ref's table is unchanged and the caller's cached
 * key is still valid; otherwise lock_key carries the recomputed key. */
FLECS_API
ecs_rust_get_ptr_t ecs_rust_ref_get_scope_begin(
    ecs_world_t *world,
    ecs_ref_t *ref,
    ecs_id_t id,
    uint64_t cached_key_table_id);

FLECS_API
ecs_rust_get_ptr_t ecs_rust_ref_get_stage_scope_begin(
    ecs_world_t *stage,
    ecs_world_t *world,
    ecs_ref_t *ref,
    ecs_id_t id,
    uint64_t cached_key_table_id);

/* Combined entity-record lookup + defer_begin, the entry half of a component
 * access scope (ecs_rust_scope_end is the exit half). Returns NULL without
 * starting a defer scope when the entity is not alive. */
FLECS_API
const ecs_record_t* ecs_rust_get_scope_begin(
    ecs_world_t *world,
    ecs_entity_t entity);

/* Liveness check + entity-record lookup with no defer level. The entry half of
 * the shared-register guard acquire: the guards track a Rust-side pin counter
 * and open a defer level lazily on the first write while a guard is live, so
 * the read path pays no defer FFI. Returns NULL when the entity is not alive. */
FLECS_API
const ecs_record_t* ecs_rust_get_record(
    ecs_world_t *world,
    ecs_entity_t entity);

FLECS_API
void ecs_rust_scope_end(
    ecs_world_t *world);

/* ABI guards: C-side sizeof for structs with FLECS_DEBUG-gated fields, so the
 * Rust side can assert its bindings match the compiled profile. */
FLECS_API size_t ecs_rust_sizeof_ecs_ref_t(void);
FLECS_API size_t ecs_rust_sizeof_ecs_map_t(void);
FLECS_API size_t ecs_rust_sizeof_ecs_map_iter_t(void);
FLECS_API size_t ecs_rust_sizeof_ecs_stack_t(void);
FLECS_API size_t ecs_rust_sizeof_ecs_stack_cursor_t(void);
