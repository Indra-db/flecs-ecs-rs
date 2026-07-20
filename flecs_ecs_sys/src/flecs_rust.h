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

/* Identifies the storage a component pointer originates from, so the Rust
 * side can key its mut-alias tracking. cr set: sparse / non-fragmenting
 * storage. table set: dense table column (which may belong to an IsA base
 * entity's table for inherited reads). */
typedef struct ecs_rust_lock_target_t {
    ecs_component_record_t *cr;
    ecs_table_t *table;
    int16_t column_index;
} ecs_rust_lock_target_t;

/* Component pointer plus its storage origin. ptr is NULL when the entity
 * does not have the component. */
typedef struct ecs_rust_get_ptr_t {
    void *ptr;
    ecs_rust_lock_target_t lock_target;
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

/* ABI guards: C-side sizeof for structs with FLECS_DEBUG-gated fields, so the
 * Rust side can assert its bindings match the compiled profile. */
FLECS_API size_t ecs_rust_sizeof_ecs_ref_t(void);
FLECS_API size_t ecs_rust_sizeof_ecs_map_t(void);
FLECS_API size_t ecs_rust_sizeof_ecs_map_iter_t(void);
FLECS_API size_t ecs_rust_sizeof_ecs_stack_t(void);
FLECS_API size_t ecs_rust_sizeof_ecs_stack_cursor_t(void);
