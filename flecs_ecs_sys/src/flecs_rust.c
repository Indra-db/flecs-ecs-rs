// On Windows, prevent winsock.h from being included by windows.h
// This must be defined before any Windows headers are included
#ifdef _WIN32
#ifndef WIN32_LEAN_AND_MEAN
#define WIN32_LEAN_AND_MEAN
#endif
#ifndef NOMINMAX
#define NOMINMAX
#endif
#endif

#include "flecs_rust.h"
/* This uses internals from flecs which aren't in the header. */
#include "flecs.c"

int32_t ecs_rust_rel_count(
    const ecs_world_t *world,
    ecs_id_t id,
    ecs_table_t* table)
{
    ecs_check(world != NULL, ECS_INVALID_PARAMETER, NULL);

    if (!table) return -1;

    flecs_poly_assert(world, ecs_world_t);
    ecs_assert(id != 0, ECS_INVALID_PARAMETER, NULL);

    ecs_component_record_t *cr = flecs_components_get(world, id);
    if (!cr) {
        return -1;
    }
    ecs_table_record_t *tr = ecs_table_cache_get(&cr->cache, table);
    if (!tr) {
        return -1;
    }
    return tr->count;
error:
    return -1;
}


ecs_entity_t ecs_rust_get_typeid(
    const ecs_world_t *world,
    ecs_id_t id,
    const ecs_component_record_t* idr)     
{
    ecs_check(world != NULL, ECS_INVALID_PARAMETER, NULL);
    const ecs_type_info_t *ti = ecs_rust_get_type_info_from_record(world, id, idr);
    if (ti) {
        ecs_assert(ti->component != 0, ECS_INTERNAL_ERROR, NULL);
        return ti->component;
    }
error:
    return 0;
} 

const ecs_type_info_t* ecs_rust_get_type_info_from_record(
    const ecs_world_t *world,
    ecs_id_t id,
    const ecs_component_record_t* idr)
{
    ecs_check(world != NULL, ECS_INVALID_PARAMETER, NULL);
    ecs_check(id != 0, ECS_INVALID_PARAMETER, NULL);

    if (!idr && ECS_IS_PAIR(id)) {
        world = ecs_get_world(world);
        idr = flecs_components_get(world,
            ecs_pair(ECS_PAIR_FIRST(id), EcsWildcard));
        if (!idr || !idr->type_info) {
            idr = NULL;
        }
        if (!idr) {
            ecs_entity_t first = ecs_pair_first(world, id);
            if (!first || !ecs_has_id(world, first, EcsPairIsTag)) {
                idr = flecs_components_get(world,
                    ecs_pair(EcsWildcard, ECS_PAIR_SECOND(id)));
                if (!idr || !idr->type_info) {
                    idr = NULL;
                }
            }
        }
    }

    if (idr) {
        return idr->type_info;
    } else if (!(id & ECS_ID_FLAGS_MASK)) {
        world = ecs_get_world(world);
        return flecs_type_info_get(world, id);
    }
error:
    return NULL;
}

/* Like flecs_ensure but adds new components WITHOUT constructor.
 * Mirrors flecs_ensure's fast paths (component_map for low IDs, sparse
 * handling) with the single difference: flecs_add_id_w_record(emplace_id=component).
 * We don't modify upstream flecs_ensure because "ensure" semantically implies
 * construction. We can't use ecs_emplace_id because it debug-asserts no
 * on_replace (that assert protects the emplace public API where no new_ptr
 * is available, but we DO have new_ptr so it doesn't apply). */
static
flecs_component_ptr_t flecs_rust_ensure(
    ecs_world_t *world,
    ecs_entity_t entity,
    ecs_id_t component,
    ecs_record_t *r,
    ecs_size_t size)
{
    flecs_component_ptr_t dst = {0};

    ecs_assert(r != NULL, ECS_INTERNAL_ERROR, NULL);

    ecs_component_record_t *cr = NULL;
    ecs_table_t *table = r->table;
    ecs_assert(table != NULL, ECS_INTERNAL_ERROR, NULL);

    if (component < FLECS_HI_COMPONENT_ID) {
        int16_t column_index = table->component_map[component];
        if (column_index > 0) {
            ecs_column_t *column = &table->data.columns[column_index - 1];
            ecs_assert(column->ti->size == size, ECS_INTERNAL_ERROR, NULL);
            dst.ptr = ECS_ELEM(column->data, size, ECS_RECORD_TO_ROW(r->row));
            dst.ti = column->ti;
            return dst;
        } else if (column_index < 0) {
            column_index = flecs_ito(int16_t, -column_index - 1);
            const ecs_table_record_t *tr = &table->_->records[column_index];
            cr = tr->hdr.cr;
            if (cr->flags & EcsIdSparse) {
                dst.ptr = flecs_component_sparse_get(
                    world, cr, r->table, entity);
                dst.ti = cr->type_info;
                ecs_assert(dst.ti->size == size, ECS_INTERNAL_ERROR, NULL);
                return dst;
            }
        }
    } else {
        cr = flecs_components_get(world, component);
        dst = flecs_get_component_ptr(
            world, table, ECS_RECORD_TO_ROW(r->row), cr);
        if (dst.ptr) {
            ecs_assert(dst.ti->size == size, ECS_INTERNAL_ERROR, NULL);
            return dst;
        }
    }

    /* Entity doesn't have component — add WITHOUT constructor (emplace).
     * Exception: EcsParent must be default-constructed, because the
     * non-fragmenting child move hooks read Parent.value during the add
     * itself, before the caller has written the new value. */
    ecs_id_t emplace_id = component;
    if (component == ecs_id(EcsParent)) {
        emplace_id = 0;
    }
    flecs_add_id_w_record(world, entity, r, component, emplace_id);

    /* Flush so the pointer we're fetching is stable */
    flecs_defer_end(world, world->stages[0]);
    flecs_defer_begin(world, world->stages[0]);

    if (!cr) {
        cr = flecs_components_get(world, component);
        ecs_assert(cr != NULL, ECS_INTERNAL_ERROR, NULL);
    }

    ecs_assert(r->table != NULL, ECS_INTERNAL_ERROR, NULL);
    return flecs_get_component_ptr(
        world, r->table, ECS_RECORD_TO_ROW(r->row), cr);
}

/* Like flecs_defer_cpp_set but skips ctor for new components.
 * New components use EcsCmdEmplace (no ctor on flush). Existing components
 * use EcsCmdAddModified with on_replace handling (same as cpp). */
static
void* flecs_defer_rust_set(
    ecs_world_t *world,
    ecs_stage_t *stage,
    ecs_entity_t entity,
    ecs_id_t id,
    ecs_size_t size,
    const void *value,
    bool *is_new)
{
    ecs_assert(value != NULL, ECS_INTERNAL_ERROR, NULL);
    ecs_assert(size != 0, ECS_INTERNAL_ERROR, NULL);

    ecs_cmd_t *cmd = flecs_cmd_new_batched(stage, entity);
    ecs_assert(cmd != NULL, ECS_INTERNAL_ERROR, NULL);
    cmd->entity = entity;
    cmd->id = id;

    ecs_record_t *r = flecs_entities_get(world, entity);
    flecs_component_ptr_t ptr = flecs_defer_get_existing(
        world, entity, r, id, size);

    bool new_component = (ptr.ptr == NULL);
    if (is_new) {
        *is_new = new_component;
    }

    const ecs_type_info_t *ti = ptr.ti;
    ecs_check(ti != NULL, ECS_INVALID_PARAMETER,
        "provided component is not a type");
    ecs_assert(size == ti->size, ECS_INVALID_PARAMETER,
        "mismatching size specified for component in ensure/emplace/set");

    /* Handle trivial set command (no hooks, OnSet observers) */
    if (id < FLECS_HI_COMPONENT_ID) {
        if (!world->non_trivial_set[id]) {
            if (new_component) {
                ptr.ptr = flecs_stack_alloc(
                    &stage->cmd->stack, size, ti->alignment);

                /* No OnSet observers, so ensure is enough */
                cmd->kind = EcsCmdEnsure;
                cmd->is._1.size = size;
                cmd->is._1.value = ptr.ptr;
            } else {
                /* No OnSet observers, so only thing we need to do is make sure
                 * that a preceding remove command doesn't cause the entity to
                 * end up without the component. */
                cmd->kind = EcsCmdAdd;
            }

            ecs_os_memcpy(ptr.ptr, value, size);
            return ptr.ptr;
        }
    }

    if (new_component) {
        if (!ti->hooks.on_replace) {
            /* No on_replace: use EcsCmdEmplace (no ctor on flush). Rust writes
             * value directly into the cmd buffer. Then queue EcsCmdModified so
             * OnSet observers fire (EcsCmdEmplace alone skips OnSet). */
            cmd->kind = EcsCmdEmplace;
            cmd->is._1.size = size;
            ptr.ptr = cmd->is._1.value =
                flecs_stack_alloc(&stage->cmd->stack, size, ti->alignment);

            /* Queue Modified so OnSet fires at flush. Uses flecs_cmd_new
             * (not batched) so it's processed after the emplace in the main
             * flush loop. */
            ecs_cmd_t *mod_cmd = flecs_cmd_new(stage);
            if (mod_cmd) {
                mod_cmd->kind = EcsCmdModified;
                mod_cmd->id = id;
                mod_cmd->entity = entity;
            }
        } else {
            /* Has on_replace: can't use EcsCmdEmplace (ecs_emplace_id
             * debug-asserts no on_replace). Fall back to EcsCmdSet which
             * handles on_replace at flush. Trade-off: flush ctors dst then
             * move_dtor drops it (extra ctor+drop). Only hits types with
             * explicit .on_replace() registration. */
            cmd->kind = EcsCmdSet;
            cmd->is._1.size = size;
            ptr.ptr = cmd->is._1.value =
                flecs_stack_alloc(&stage->cmd->stack, size, ti->alignment);
        }
    } else {
        cmd->kind = EcsCmdAddModified;

        if (ti->hooks.on_replace) {
            flecs_invoke_replace_hook(
                world, r->table, entity, id, ptr.ptr, value, ti, r->table);
        }
    }

    return ptr.ptr;
error:
    return NULL;
}

/* Rust-optimized set: like ecs_cpp_set but skips ctor for new components.
 * In C++, ctor + assignment operator is the pattern. In Rust, there's no
 * assignment operator, so the ctor output would just be dropped, wasted work.
 * Both deferred and non-deferred paths skip construction for new components. */
ecs_rust_set_t ecs_rust_set(
    ecs_world_t *world,
    ecs_entity_t entity,
    ecs_id_t id,
    const void *new_ptr,
    size_t size)
{
    ecs_check(world != NULL, ECS_INVALID_PARAMETER, NULL);
    ecs_check(ecs_is_alive(world, entity), ECS_INVALID_PARAMETER, NULL);

    ecs_stage_t *stage = flecs_stage_from_world(&world);
    ecs_rust_set_t result;

    if (flecs_defer_cmd(stage)) {
        result.ptr = flecs_defer_rust_set(world, stage, entity, id,
            flecs_utosize(size), new_ptr, &result.is_new);
        result.call_modified = false;
        return result;
    }

    ecs_record_t *r = flecs_entities_get(world, entity);
    ecs_table_t *prev_table = r->table;
    flecs_component_ptr_t dst = flecs_rust_ensure(world, entity, id, r,
        flecs_uto(int32_t, size));

    result.ptr = dst.ptr;
    result.is_new = (r->table != prev_table);

    if (id < FLECS_HI_COMPONENT_ID) {
        if (!world->non_trivial_set[id]) {
            result.call_modified = false;
            goto done;
        }
    }

    /* Not deferring, so need to call modified after setting the component */
    result.call_modified = true;

    if (dst.ti->hooks.on_replace) {
        flecs_invoke_replace_hook(
            world, r->table, entity, id, dst.ptr, new_ptr, dst.ti, prev_table);
    }

done:
    flecs_defer_end(world, stage);

    return result;
error:
    return (ecs_rust_set_t){0};
}
/* Key encodings mirrored by safety_map.rs dense_lock_key / sparse_lock_key. */
#define ECS_RUST_DENSE_KEY(table_, col_) \
    ((((uint64_t)(uintptr_t)(table_)) << 16) | (uint64_t)(uint16_t)(col_))

#define ECS_RUST_SPARSE_KEY(cr_) \
    (((uint64_t)(uintptr_t)(cr_)) | (1ull << 63))

#define ECS_RUST_GET_PTR(ptr_, cr_, table_, col_) \
    (ecs_rust_get_ptr_t){ \
        .ptr = (ptr_), \
        .lock_key = (cr_) != NULL \
            ? ECS_RUST_SPARSE_KEY(cr_) \
            : ECS_RUST_DENSE_KEY(table_, col_) \
    }

#define ECS_RUST_GET_PTR_NULL (ecs_rust_get_ptr_t){0}

ecs_rust_get_ptr_t ecs_rust_get_sparse_id(
    const ecs_world_t *world,
    ecs_entity_t entity,
    ecs_id_t id,
    size_t size)
{
    ecs_check(world != NULL, ECS_INVALID_PARAMETER, NULL);
    flecs_assert_entity_valid(world, entity, "get_sparse");
    ecs_check(!ecs_id_is_wildcard(id), ECS_INVALID_PARAMETER,
        "cannot call get_sparse() with wildcard component '%s'",
            flecs_errstr(ecs_id_str(world, id)));
    ecs_check(ecs_id_is_valid(world, id), ECS_INVALID_PARAMETER, NULL);

    world = ecs_get_world(world);

    ecs_component_record_t *cr = flecs_components_get(world, id);
    if (!cr) {
        return ECS_RUST_GET_PTR_NULL;
    }

    ecs_check(cr->flags & EcsIdSparse, ECS_INVALID_PARAMETER,
        "cannot call get_sparse() for non-sparse component '%s' "
        "(use get()/get_mut())",
            flecs_errstr(ecs_id_str(world, id)));
    ecs_check(!(cr->flags & EcsIdOnInstantiateInherit), ECS_INVALID_PARAMETER,
        "cannot call get_sparse() for component '%s' with the "
        "(OnInstantiate, Inherit) trait (use get())",
            flecs_errstr(ecs_id_str(world, id)));
    ecs_assert(cr->sparse != NULL, ECS_INTERNAL_ERROR, NULL);

    return ECS_RUST_GET_PTR(
        flecs_sparse_get(cr->sparse, flecs_utosize(size), entity),
        cr, NULL, -1);
error:
    return ECS_RUST_GET_PTR_NULL;
}

/* Mirrors flecs_get_base_component (which only returns the pointer), but also
 * reports which storage the pointer originates from so the Rust side can key
 * its mut-alias tracking against the base entity's storage. */
static
ecs_rust_get_ptr_t flecs_rust_get_base_component(
    const ecs_world_t *world,
    ecs_table_t *table,
    ecs_id_t component,
    ecs_component_record_t *cr,
    int32_t recur_depth)
{
    ecs_check(recur_depth < ECS_MAX_RECURSION, ECS_INVALID_OPERATION,
        "cycle detected in IsA relationship");

    if (!(table->flags & EcsTableHasIsA)) {
        return ECS_RUST_GET_PTR_NULL;
    }

    if (!(cr->flags & EcsIdOnInstantiateInherit)) {
        return ECS_RUST_GET_PTR_NULL;
    }

    if (component == ecs_pair(ecs_id(EcsIdentifier), EcsName)) {
        return ECS_RUST_GET_PTR_NULL;
    }

    const ecs_table_record_t *tr_isa = flecs_component_get_table(
        world->cr_isa_wildcard, table);
    ecs_check(tr_isa != NULL, ECS_INTERNAL_ERROR, NULL);

    ecs_type_t type = table->type;
    ecs_id_t *ids = type.array;
    int32_t i = tr_isa->index, end = tr_isa->count + tr_isa->index;
    ecs_rust_get_ptr_t ptr = ECS_RUST_GET_PTR_NULL;

    do {
        ecs_id_t pair = ids[i ++];
        ecs_entity_t base = ecs_pair_second(world, pair);

        ecs_record_t *r = flecs_entities_get(world, base);
        ecs_assert(r != NULL, ECS_INTERNAL_ERROR, NULL);

        table = r->table;
        ecs_assert(table != NULL, ECS_INTERNAL_ERROR, NULL);

        const ecs_table_record_t *tr = flecs_component_get_table(cr, table);
        if (!tr) {
            if (cr->flags & EcsIdDontFragment) {
                void *sparse_ptr = flecs_component_sparse_get(
                    world, cr, table, base);
                if (sparse_ptr) {
                    ptr = ECS_RUST_GET_PTR(sparse_ptr, cr, NULL, -1);
                }
            }

            if (!ptr.ptr) {
                ptr = flecs_rust_get_base_component(world, table, component,
                    cr, recur_depth + 1);
            }
        } else {
            if (cr->flags & EcsIdSparse) {
                return ECS_RUST_GET_PTR(
                    flecs_component_sparse_get(world, cr, table, base),
                    cr, NULL, -1);
            } else if (tr->column != -1) {
                int32_t row = ECS_RECORD_TO_ROW(r->row);
                int16_t column = tr->column;
                return ECS_RUST_GET_PTR(
                    flecs_table_get_component(table, column, row).ptr,
                    NULL, table, column);
            }
        }
    } while (!ptr.ptr && (i < end));

    return ptr;
error:
    return ECS_RUST_GET_PTR_NULL;
}

ecs_rust_get_ptr_t ecs_rust_record_get_id(
    const ecs_world_t *world,
    ecs_entity_t entity,
    const ecs_record_t *r,
    ecs_id_t component)
{
    ecs_check(world != NULL, ECS_INVALID_PARAMETER, NULL);
    flecs_poly_assert(world, ecs_world_t);
    flecs_assert_entity_valid(world, entity, "get");
    ecs_assert(r != NULL, ECS_INVALID_PARAMETER, NULL);
    ecs_check(ecs_id_is_valid(world, component) || ecs_id_is_wildcard(component),
        ECS_INVALID_PARAMETER, NULL);

    ecs_table_t *table = r->table;
    ecs_assert(table != NULL, ECS_INTERNAL_ERROR, NULL);

    if (component < FLECS_HI_COMPONENT_ID) {
        /* EcsWildcard and EcsAny are FLECS_HI_COMPONENT_ID + 14/15 and a
         * wildcard pair carries the pair flag, so nothing that reaches this
         * branch can be a wildcard: the fast path never pays for the test
         * below. */
        if (!world->non_trivial_lookup[component]) {
            ecs_assert(table->component_map != NULL, ECS_INTERNAL_ERROR, NULL);
            int16_t column_index = table->component_map[component];
            if (column_index > 0) {
                column_index --;
                ecs_column_t *column = &table->data.columns[column_index];
                return ECS_RUST_GET_PTR(
                    ECS_ELEM(column->data, column->ti->size,
                        ECS_RECORD_TO_ROW(r->row)),
                    NULL, table, column_index);
            }
            return ECS_RUST_GET_PTR_NULL;
        }
    } else if ((component == EcsWildcard) || (component == EcsAny) ||
        (ECS_IS_PAIR(component) && ecs_id_is_wildcard(component)))
    {
        /* Resolve a wildcard to the concrete id the entity's table holds, so
         * the lock key we hand back names the same storage a concrete get
         * would. */
        ecs_type_t type = table->type;
        int32_t i, count = type.count;
        for (i = 0; i < count; i ++) {
            ecs_id_t id = type.array[i];
            if (ecs_id_match(id, component) ||
                (component == EcsAny) ||
                (ECS_IS_PAIR(id) && ECS_IS_PAIR(component) &&
                    ((ECS_PAIR_FIRST(component) == EcsWildcard) ||
                        (ECS_PAIR_FIRST(component) == EcsAny) ||
                        (ECS_PAIR_FIRST(component) == ECS_PAIR_FIRST(id))) &&
                    ((ECS_PAIR_SECOND(component) == EcsWildcard) ||
                        (ECS_PAIR_SECOND(component) == EcsAny) ||
                        (ECS_PAIR_SECOND(component) == ECS_PAIR_SECOND(id)))))
            {
                component = id;
                break;
            }
        }
        if (i == count) {
            return ECS_RUST_GET_PTR_NULL;
        }
    }

    ecs_component_record_t *cr = flecs_components_get(world, component);
    if (!cr) {
        return ECS_RUST_GET_PTR_NULL;
    }

    if (cr->flags & EcsIdDontFragment) {
        void *ptr = flecs_component_sparse_get(world, cr, table, entity);
        if (ptr) {
            return ECS_RUST_GET_PTR(ptr, cr, NULL, -1);
        }
    }

    const ecs_table_record_t *tr = flecs_component_get_table(cr, table);
    if (!tr) {
        return flecs_rust_get_base_component(world, table, component, cr, 0);
    } else {
        if (cr->flags & EcsIdSparse) {
            return ECS_RUST_GET_PTR(
                flecs_component_sparse_get(world, cr, table, entity),
                cr, NULL, -1);
        }
        ecs_check(tr->column != -1, ECS_INVALID_PARAMETER,
            "component '%s' passed to get() is a tag/zero sized",
                flecs_errstr(ecs_id_str(world, component)));
    }

    int32_t row = ECS_RECORD_TO_ROW(r->row);
    int16_t column_index = tr->column;
    return ECS_RUST_GET_PTR(
        flecs_table_get_component(table, column_index, row).ptr,
        NULL, table, column_index);
error:
    return ECS_RUST_GET_PTR_NULL;
}

ecs_rust_get_ptr_t ecs_rust_record_get_mut_id(
    const ecs_world_t *world,
    const ecs_record_t *r,
    ecs_id_t component)
{
    ecs_check(world != NULL, ECS_INVALID_PARAMETER, NULL);
    flecs_poly_assert(world, ecs_world_t);
    ecs_assert(r != NULL, ECS_INVALID_PARAMETER, NULL);
    ecs_check(ecs_id_is_valid(world, component), ECS_INVALID_PARAMETER, NULL);
    ecs_dbg_assert(!flecs_component_has_on_replace(world, component, "get_mut"),
        ECS_INVALID_PARAMETER,
        "cannot call get_mut() for component '%s' which has an on_replace hook "
        "(use set()/assign())",
            flecs_errstr(ecs_id_str(world, component)));

    flecs_check_exclusive_world_access_write(world);

    ecs_table_t *table = r->table;
    ecs_assert(table != NULL, ECS_INTERNAL_ERROR, NULL);
    int32_t row = ECS_RECORD_TO_ROW(r->row);

    if (component < FLECS_HI_COMPONENT_ID) {
        if (!world->non_trivial_lookup[component]) {
            ecs_assert(table->component_map != NULL, ECS_INTERNAL_ERROR, NULL);
            int16_t column_index = table->component_map[component];
            if (column_index > 0) {
                column_index --;
                ecs_column_t *column = &table->data.columns[column_index];
                return ECS_RUST_GET_PTR(
                    ECS_ELEM(column->data, column->ti->size, row),
                    NULL, table, column_index);
            }
            return ECS_RUST_GET_PTR_NULL;
        }
    }

    ecs_component_record_t *cr = flecs_components_get(world, component);
    if (!cr) {
        return ECS_RUST_GET_PTR_NULL;
    }

    if (cr->flags & (EcsIdSparse|EcsIdDontFragment)) {
        return ECS_RUST_GET_PTR(
            flecs_component_sparse_get(world, cr, table,
                ecs_table_entities(table)[row]),
            cr, NULL, -1);
    }

    const ecs_table_record_t *tr = flecs_component_get_table(cr, table);
    if (!tr || (tr->column == -1)) {
        return ECS_RUST_GET_PTR_NULL;
    }

    int16_t column_index = tr->column;
    return ECS_RUST_GET_PTR(
        flecs_table_get_component(table, column_index, row).ptr,
        NULL, table, column_index);
error:
    return ECS_RUST_GET_PTR_NULL;
}

ecs_rust_get_ptr_t ecs_rust_ref_get_scope_begin(
    ecs_world_t *world,
    ecs_ref_t *ref,
    ecs_id_t id,
    uint64_t cached_key_table_id)
{
    void *ptr = ecs_ref_get_id(world, ref, id);
    if (!ptr) {
        return ECS_RUST_GET_PTR_NULL;
    }

    ecs_defer_begin(world);

    /* The ref still points into the same table: the caller's cached lock key
     * is still valid; lock_key 0 signals "reuse cached". */
    if (ref->table_id == cached_key_table_id) {
        return (ecs_rust_get_ptr_t){ .ptr = ptr, .lock_key = 0 };
    }

    /* Table changed (or first access): recompute the storage key. Only paid
     * when the entity moved archetype since the previous access. */
    const ecs_world_t *w = ecs_get_world(world);
    ecs_component_record_t *cr = flecs_components_get(w, id);
    ecs_assert(cr != NULL, ECS_INTERNAL_ERROR, NULL);

    if (cr->flags & (EcsIdSparse|EcsIdDontFragment)) {
        return (ecs_rust_get_ptr_t){
            .ptr = ptr, .lock_key = ECS_RUST_SPARSE_KEY(cr) };
    }

    ecs_record_t *r = flecs_entities_get_any(w, ref->entity);
    ecs_assert(r != NULL, ECS_INTERNAL_ERROR, NULL);
    ecs_table_t *table = r->table;
    ecs_assert(table != NULL, ECS_INTERNAL_ERROR, NULL);
    const ecs_table_record_t *tr = flecs_component_get_table(cr, table);
    ecs_assert(tr != NULL && tr->column != -1, ECS_INTERNAL_ERROR, NULL);

    return (ecs_rust_get_ptr_t){
        .ptr = ptr, .lock_key = ECS_RUST_DENSE_KEY(table, tr->column) };
}

ecs_rust_get_ptr_t ecs_rust_ref_get_stage_scope_begin(
    ecs_world_t *stage,
    ecs_world_t *world,
    ecs_ref_t *ref,
    ecs_id_t id,
    uint64_t cached_key_table_id)
{
    void *ptr = ecs_ref_get_id(world, ref, id);
    if (!ptr) {
        return ECS_RUST_GET_PTR_NULL;
    }

    ecs_defer_begin(stage);

    if (ref->table_id == cached_key_table_id) {
        return (ecs_rust_get_ptr_t){ .ptr = ptr, .lock_key = 0 };
    }

    ecs_component_record_t *cr = flecs_components_get(world, id);
    ecs_assert(cr != NULL, ECS_INTERNAL_ERROR, NULL);

    if (cr->flags & (EcsIdSparse|EcsIdDontFragment)) {
        return (ecs_rust_get_ptr_t){
            .ptr = ptr, .lock_key = ECS_RUST_SPARSE_KEY(cr) };
    }

    ecs_record_t *r = flecs_entities_get_any(world, ref->entity);
    ecs_assert(r != NULL, ECS_INTERNAL_ERROR, NULL);
    ecs_table_t *table = r->table;
    ecs_assert(table != NULL, ECS_INTERNAL_ERROR, NULL);
    const ecs_table_record_t *tr = flecs_component_get_table(cr, table);
    ecs_assert(tr != NULL && tr->column != -1, ECS_INTERNAL_ERROR, NULL);

    return (ecs_rust_get_ptr_t){
        .ptr = ptr, .lock_key = ECS_RUST_DENSE_KEY(table, tr->column) };
}

const ecs_record_t* ecs_rust_get_scope_begin(
    ecs_world_t *world,
    ecs_entity_t entity)
{
    /* ecs_record_find asserts on dead entities in debug builds; gate on
     * liveness so the Rust side gets NULL and can panic with its own
     * message. */
    if (!ecs_is_alive(world, entity)) {
        return NULL;
    }
    const ecs_record_t *r = ecs_record_find(world, entity);
    if (!r) {
        return NULL;
    }
    ecs_defer_begin(world);
    return r;
}

const ecs_record_t* ecs_rust_get_record(
    ecs_world_t *world,
    ecs_entity_t entity)
{
    /* Liveness check + record lookup with no defer level: the shared-register
     * guards track a Rust-side pin counter instead, and open a defer level
     * lazily only on the first write while a guard is live. Returns NULL for a
     * dead entity so the Rust side can map it to AccessError::NotAlive. */
    if (!ecs_is_alive(world, entity)) {
        return NULL;
    }
    return ecs_record_find(world, entity);
}

void ecs_rust_scope_end(
    ecs_world_t *world)
{
    ecs_defer_end(world);
}

size_t ecs_rust_sizeof_ecs_ref_t(void) {
    return sizeof(ecs_ref_t);
}

size_t ecs_rust_sizeof_ecs_map_t(void) {
    return sizeof(ecs_map_t);
}

size_t ecs_rust_sizeof_ecs_map_iter_t(void) {
    return sizeof(ecs_map_iter_t);
}

size_t ecs_rust_sizeof_ecs_stack_t(void) {
    return sizeof(ecs_stack_t);
}

size_t ecs_rust_sizeof_ecs_stack_cursor_t(void) {
    return sizeof(ecs_stack_cursor_t);
}
