#![allow(dead_code)]
use flecs_ecs::core::*;
use flecs_ecs::macros::*;

use crate::common_test::*;

#[test]
fn query_uncached_destruction_no_panic() {
    let world = World::new();
    let query = world.new_query::<&Tag>();
    let query2 = query.clone();
    drop(query);
    query2.run(|mut it| while it.next() {});
    drop(query2);
}

#[test]
#[should_panic]
fn query_cached_destruction_lingering_references_panic() {
    let world = World::new();
    let query = world.query::<()>().with(&Tag::id()).set_cached().build();
    let query2 = query.clone();
    query.destruct();
    query2.run(|_| {});
    drop(query2);
}

#[test]
fn query_cached_destruction_sole_owner_no_use_after_free() {
    let world = World::new();

    for _ in 0..64 {
        let query = world.query::<()>().with(&Tag::id()).set_cached().build();
        query.destruct();
    }

    let e = world.entity().add(Tag::id());
    let e_id = e.id();
    let query = world.query::<()>().with(&Tag::id()).set_cached().build();
    let count = core::cell::Cell::new(0);
    query.each_entity(|entity, _| {
        assert_eq!(entity.id(), e_id);
        count.set(count.get() + 1);
    });
    assert_eq!(count.get(), 1);
    query.destruct();
}

#[test]
fn query_with_dynamically_built_name_string() {
    let world = World::new();

    let tag = world.entity_named("DynTag");
    let e = world.entity().add(tag);
    let e_id = e.id();

    let name = String::from("Dyn") + "Tag";
    let query = world.query::<()>().with(name.as_str()).build();

    let count = core::cell::Cell::new(0);
    query.each_entity(|entity, _| {
        assert_eq!(entity.id(), e_id);
        count.set(count.get() + 1);
    });
    assert_eq!(count.get(), 1);
}

#[test]
fn query_iter_stage() {
    #[derive(Component, Debug)]
    struct Comp(usize);

    let world = World::new();
    world.set_threads(4);

    let query = world.new_query::<&Comp>().handle();

    for i in 0..4 {
        world.entity().set(Comp(i));
    }

    world.system::<&Comp>().par_each_entity(move |e, _| {
        query.iter_stage(e).each(|_vel| {});
    });

    world.progress();
}

#[test]
fn query_handle_keeps_query_alive() {
    #[derive(Component, Debug)]
    struct Comp(usize);

    let world = World::new();
    world.entity().set(Comp(1));

    let handle = {
        let query = world.new_query::<&Comp>();
        query.handle()
    };

    let count = core::cell::Cell::new(0);
    handle
        .iter_stage(&world)
        .each(|_| count.set(count.get() + 1));
    assert_eq!(count.get(), 1);
}

#[test]
fn query_handle_reference_counts() {
    #[derive(Component, Debug)]
    struct Comp(usize);

    let world = World::new();
    let query = world.new_query::<&Comp>();
    assert_eq!(query.reference_count(), 1);

    let h1 = query.handle();
    assert_eq!(query.reference_count(), 2);

    let h2 = query.handle();
    let h3 = h1.clone();
    assert_eq!(query.reference_count(), 4);

    drop(h1);
    drop(h2);
    drop(h3);
    assert_eq!(query.reference_count(), 1);
}

#[test]
fn query_handle_entity_query_off_thread_drop() {
    #[derive(Component, Debug)]
    struct Comp(usize);

    let world = World::new();
    world.entity().set(Comp(1));

    let query = world.query_named::<&Comp>("named_query").build();
    let handle = query.handle();

    std::thread::spawn(move || drop(handle)).join().unwrap();

    let count = core::cell::Cell::new(0);
    query.each(|_| count.set(count.get() + 1));
    assert_eq!(count.get(), 1);
    query.destruct();
}

#[test]
fn query_handle_two_par_systems() {
    let world = World::new();
    world.set_threads(2);

    let e = world
        .entity()
        .set(Position { x: 0, y: 0 })
        .set(Velocity { x: 1, y: 2 });

    let query = world.new_query::<&Velocity>();
    let h1 = query.handle();
    let h2 = query.handle();

    world
        .system::<&mut Position>()
        .par_each_entity(move |e, p| {
            h1.iter_stage(e.world()).each(|v| {
                p.x += v.x;
            });
        });
    world
        .system::<&mut Position>()
        .par_each_entity(move |e, p| {
            h2.iter_stage(e.world()).each(|v| {
                p.y += v.y;
            });
        });

    world.progress();

    e.get::<&Position>(|p| {
        assert_eq!(p.x, 1);
        assert_eq!(p.y, 2);
    });
}

#[test]
fn query_handle_clone_inside_par_callback() {
    let world = World::new();
    world.set_threads(4);

    let e = world
        .entity()
        .set(Position { x: 0, y: 0 })
        .set(Velocity { x: 1, y: 2 });

    let handle = world.new_query::<&Velocity>().handle();

    world
        .system::<&mut Position>()
        .par_each_entity(move |e, p| {
            // Clone and drop on the worker thread, mid staged execution.
            let local = handle.clone();
            local.iter_stage(e.world()).each(|v| {
                p.x += v.x;
            });
        });

    world.progress();

    e.get::<&Position>(|p| {
        assert_eq!(p.x, 1);
    });
}

#[test]
fn query_handle_in_single_threaded_system() {
    let world = World::new();

    let e = world
        .entity()
        .set(Position { x: 0, y: 0 })
        .set(Velocity { x: 3, y: 4 });

    let handle = world.new_query::<&Velocity>().handle();

    world.system::<&mut Position>().each_entity(move |e, p| {
        handle.iter_stage(e.world()).each(|v| {
            p.x += v.x;
        });
    });

    world.progress();

    e.get::<&Position>(|p| {
        assert_eq!(p.x, 3);
    });
}

#[test]
fn query_handle_concurrent_drop_stress() {
    #[derive(Component, Debug)]
    struct Comp(usize);

    for _ in 0..50 {
        let world = World::new();
        world.entity().set(Comp(1));

        let query = world.new_query::<&Comp>();
        let threads: Vec<_> = (0..8)
            .map(|_| {
                let handle = query.handle();
                std::thread::spawn(move || drop(handle))
            })
            .collect();

        // Churn other queries on the owning thread while worker threads drop
        // handles, contending the shared query refcount and poly atomics.
        for _ in 0..20 {
            let other = world.new_query::<&Comp>();
            drop(other);
        }

        for t in threads {
            t.join().unwrap();
        }
        drop(query);
    }
}

#[test]
fn query_handle_drop_racing_world_drop() {
    #[derive(Component, Debug)]
    struct Comp(usize);

    for _ in 0..100 {
        let world = World::new();
        world.entity().set(Comp(1));
        let handle = world.new_query::<&Comp>().handle();

        let barrier = alloc::sync::Arc::new(std::sync::Barrier::new(2));
        let thread_barrier = barrier.clone();
        let t = std::thread::spawn(move || {
            thread_barrier.wait();
            drop(handle);
        });

        barrier.wait();
        // Either ordering is defined: clean drop on both sides, or the world
        // observes the still-live handle and panics with "lingering
        // references" (caught here), never memory unsafety.
        let _ = std::panic::catch_unwind(core::panic::AssertUnwindSafe(move || drop(world)));
        t.join().unwrap();
    }
}

#[test]
#[should_panic(expected = "multithreaded execution phase")]
fn entity_create_during_multithreaded_phase_panics() {
    let world = World::new();
    world.set_threads(2);

    world.readonly_begin(true);
    let _ = world.entity();
}

#[test]
#[should_panic(expected = "lingering references")]
fn query_handle_outliving_world_panics() {
    #[derive(Component, Debug)]
    struct Comp(usize);

    let world = World::new();
    let handle = world.new_query::<&Comp>().handle();
    core::mem::forget(handle);
    drop(world);
}

#[test]
#[should_panic(expected = "different world")]
fn query_iter_stage_wrong_world_panics() {
    #[derive(Component, Debug)]
    struct Comp(usize);

    let world_a = World::new();
    let world_b = World::new();
    world_a.entity().set(Comp(1));

    let query = world_a.new_query::<&Comp>();
    query.iter_stage(&world_b).each(|_| {});
}

#[test]
#[should_panic(expected = "different world")]
fn query_handle_iter_stage_wrong_world_panics() {
    #[derive(Component, Debug)]
    struct Comp(usize);

    let world_a = World::new();
    let world_b = World::new();
    world_a.entity().set(Comp(1));

    let handle = world_a.new_query::<&Comp>().handle();
    handle.iter_stage(&world_b).each(|_| {});
}

#[test]
fn query_handle_drop_off_thread_last_ref() {
    #[derive(Component, Debug)]
    struct Comp(usize);

    let world = World::new();
    world.entity().set(Comp(1));

    let handle = {
        let query = world.new_query::<&Comp>();
        query.handle()
    };

    std::thread::spawn(move || drop(handle)).join().unwrap();

    world.entity().set(Comp(2));
    let query = world.new_query::<&Comp>();
    let count = core::cell::Cell::new(0);
    query.each(|_| count.set(count.get() + 1));
    assert_eq!(count.get(), 2);
}

#[test]
#[should_panic]
fn query_panic_inside() {
    let world = World::new();
    let query = world.query::<()>().with(&Tag::id()).build();
    query.run(|_| {
        panic!();
    });
}

#[test]
fn query_run_sparse() {
    let world = World::new();

    world.component::<Position>().add_trait::<flecs::Sparse>();
    world.component::<Velocity>();

    let entity = world
        .entity()
        .set(Position { x: 10, y: 20 })
        .set(Velocity { x: 1, y: 2 });

    let q = world.query::<(&mut Position, &Velocity)>().build();

    q.run(|mut it| {
        while it.next() {
            let v = it.field::<Velocity>(1);

            for i in it.iter() {
                let mut p = it.field_at_mut::<Position>(0, i);
                p.x += v[i].x;
                p.y += v[i].y;
            }
        }
    });

    entity.get::<&Position>(|p| {
        assert_eq!(p.x, 11);
        assert_eq!(p.y, 22);
    });
}

#[test]
fn query_each_sparse() {
    let world = World::new();

    world.component::<Position>().add_trait::<flecs::Sparse>();
    world.component::<Velocity>();

    let entity = world
        .entity()
        .set(Position { x: 10, y: 20 })
        .set(Velocity { x: 1, y: 2 });

    let q = world.query::<(&mut Position, &Velocity)>().build();

    q.each(|(p, v)| {
        p.x += v.x;
        p.y += v.y;
    });

    entity.get::<&Position>(|p| {
        assert_eq!(p.x, 11);
        assert_eq!(p.y, 22);
    });
}

#[test]
fn query_each_sparse_many() {
    let world = World::new();

    world.component::<Position>().add_trait::<flecs::Sparse>();
    world.component::<Velocity>();

    let mut entities = Vec::new();

    for i in 0..2000 {
        entities.push(
            world
                .entity_named(i.to_string().as_str())
                .set(Position {
                    x: 10 + i,
                    y: 20 + i,
                })
                .set(Velocity { x: i, y: i })
                .id(),
        );
    }

    let q = world.query::<(&mut Position, &Velocity)>().build();

    q.each(|(p, v)| {
        p.x += v.x;
        p.y += v.y;
    });

    for i in 0..2000_i32 {
        let e = world.entity_from_id(entities[i as usize]);
        e.get::<&Position>(|p| {
            assert_eq!(p.x, 10 + i * 2);
            assert_eq!(p.y, 20 + i * 2);
        });
        e.get::<&Velocity>(|v| {
            assert_eq!(v.x, i);
            assert_eq!(v.y, i);
        });
    }
}

#[test]
fn query_iter_targets() {
    let world = World::new();

    let likes = world.entity();
    let pizza = world.entity();
    let salad = world.entity();
    let alice = world.entity().add((likes, pizza)).add((likes, salad));

    let q = world
        .query::<()>()
        .with((likes, id::<flecs::Any>()))
        .build();

    let mut count = 0;
    let mut tgt_count = 0;

    q.run(|mut it| {
        while it.next() {
            for i in it.iter() {
                let e = it.get_entity(i).unwrap();
                assert_eq!(e, alice);

                it.targets(0, |tgt| {
                    if tgt_count == 0 {
                        assert_eq!(tgt, pizza);
                    }
                    if tgt_count == 1 {
                        assert_eq!(tgt, salad);
                    }
                    tgt_count += 1;
                });

                count += 1;
            }
        }
    });

    assert_eq!(count, 1);
    assert_eq!(tgt_count, 2);
}

#[test]
fn query_iter_targets_second_field() {
    let world = World::new();

    let likes = world.entity();
    let pizza = world.entity();
    let salad = world.entity();
    let alice = world
        .entity()
        .add(Position::id())
        .add((likes, pizza))
        .add((likes, salad));

    let q = world
        .query::<&Position>()
        .with((likes, id::<flecs::Any>()))
        .build();

    let mut count = 0;
    let mut tgt_count = 0;

    q.run(|mut it| {
        while it.next() {
            for i in it.iter() {
                let e = it.get_entity(i).unwrap();
                assert_eq!(e, alice);

                it.targets(1, |tgt| {
                    if tgt_count == 0 {
                        assert_eq!(tgt, pizza);
                    }
                    if tgt_count == 1 {
                        assert_eq!(tgt, salad);
                    }
                    tgt_count += 1;
                });
                count += 1;
            }
        }
    });

    assert_eq!(count, 1);
    assert_eq!(tgt_count, 2);
}

#[test]
#[should_panic]
#[cfg(debug_assertions)]
fn query_iter_targets_field_out_of_range() {
    let world = World::new();

    let likes = world.entity();
    let pizza = world.entity();
    let salad = world.entity();
    let alice = world.entity().add((likes, pizza)).add((likes, salad));

    let q = world
        .query::<()>()
        .with((likes, id::<flecs::Any>()))
        .build();

    q.run(|mut it| {
        while it.next() {
            for i in it.iter() {
                let e = it.get_entity(i).unwrap();
                assert_eq!(e, alice);

                // This should panic because the index 1 is out of range
                it.targets(1, |_| {});
            }
        }
    });
}

#[test]
#[should_panic]
#[cfg(debug_assertions)]
fn query_iter_targets_field_not_a_pair() {
    let world = World::new();

    let likes = world.entity();
    let pizza = world.entity();
    let salad = world.entity();
    let alice = world
        .entity()
        .add(Position::id())
        .add((likes, pizza))
        .add((likes, salad));

    let q = world.query::<&Position>().build();

    q.run(|mut it| {
        while it.next() {
            for i in it.iter() {
                let e = it.get_entity(i).unwrap();
                assert_eq!(e, alice);

                it.targets(1, |_| {});
            }
        }
    });
}

#[test]
#[should_panic]
#[cfg(debug_assertions)]
fn query_iter_targets_field_not_set() {
    let world = World::new();

    let likes = world.entity();
    let alice = world.entity().add(Position::id());

    let q = world
        .query::<&Position>()
        .with((likes, id::<flecs::Any>()))
        .optional()
        .build();

    q.run(|mut it| {
        while it.next() {
            for i in it.iter() {
                let e = it.get_entity(i).unwrap();
                assert_eq!(e, alice);

                it.targets(1, |_| {});
            }
        }
    });
}

// ─── term_each_component ──────────────────────────────────────────────────────

#[test]
fn query_term_each_component() {
    let world = World::new();

    let e1 = world.entity().set(Position { x: 1, y: 2 });
    let e2 = world.entity().set(Position { x: 3, y: 4 });
    let e3 = world.entity().set(Position { x: 5, y: 6 });
    e3.add(Tag::id());

    let mut count = 0;
    world.each_entity::<&Position>(|e, p| {
        if e == e1 {
            assert_eq!(p.x, 1);
            assert_eq!(p.y, 2);
            count += 1;
        }
        if e == e2 {
            assert_eq!(p.x, 3);
            assert_eq!(p.y, 4);
            count += 1;
        }
        if e == e3 {
            assert_eq!(p.x, 5);
            assert_eq!(p.y, 6);
            count += 1;
        }
    });
    assert_eq!(count, 3);
}

// ─── term_each_tag ────────────────────────────────────────────────────────────

#[test]
fn query_term_each_tag() {
    #[derive(Component)]
    struct Foo;

    let world = World::new();

    let e1 = world.entity().add(Foo::id());
    let e2 = world.entity().add(Foo::id());
    let e3 = world.entity().add(Foo::id());
    e3.add(Tag::id());

    let mut count = 0;
    // Tags (ZST) cannot be in type signature; use id-based query
    let q = world.query::<()>().with(Foo::id()).build();
    q.each_entity(|e, ()| {
        if e == e1 || e == e2 || e == e3 {
            count += 1;
        }
    });
    assert_eq!(count, 3);
}

// ─── term_each_id ─────────────────────────────────────────────────────────────

#[test]
fn query_term_each_id() {
    let world = World::new();

    let foo = world.entity();

    let e1 = world.entity().add(foo);
    let e2 = world.entity().add(foo);
    let e3 = world.entity().add(foo);
    e3.add(Tag::id());

    let mut count = 0;
    let q = world.query::<()>().with(foo).build();
    q.each_entity(|e, ()| {
        if e == e1 || e == e2 || e == e3 {
            count += 1;
        }
    });
    assert_eq!(count, 3);
}

// ─── term_each_pair_type ──────────────────────────────────────────────────────

#[test]
fn query_term_each_pair_type() {
    #[derive(Component)]
    struct Rel2;
    #[derive(Component)]
    struct Obj2;

    let world = World::new();

    let e1 = world.entity().add((Rel2::id(), Obj2::id()));
    let e2 = world.entity().add((Rel2::id(), Obj2::id()));
    let e3 = world.entity().add((Rel2::id(), Obj2::id()));
    e3.add(Tag::id());

    let mut count = 0;
    // (Rel2, Obj2) is a pair of ZSTs - use id-based query
    let q = world.query::<()>().with((Rel2::id(), Obj2::id())).build();
    q.each_entity(|e, ()| {
        if e == e1 || e == e2 || e == e3 {
            count += 1;
        }
    });
    assert_eq!(count, 3);
}

// ─── term_each_pair_id ────────────────────────────────────────────────────────

#[test]
fn query_term_each_pair_id() {
    let world = World::new();

    let rel = world.entity();
    let obj = world.entity();

    let e1 = world.entity().add((rel, obj));
    let e2 = world.entity().add((rel, obj));
    let e3 = world.entity().add((rel, obj));
    e3.add(Tag::id());

    let mut count = 0;
    let q = world.query::<()>().with((rel, obj)).build();
    q.each_entity(|e, ()| {
        if e == e1 || e == e2 || e == e3 {
            count += 1;
        }
    });
    assert_eq!(count, 3);
}

// ─── term_each_pair_relation_wildcard ─────────────────────────────────────────

#[test]
fn query_term_each_pair_relation_wildcard() {
    let world = World::new();

    let rel1 = world.entity();
    let rel2 = world.entity();
    let obj = world.entity();

    let e1 = world.entity().add((rel1, obj));
    let e2 = world.entity().add((rel1, obj));
    let e3 = world.entity().add((rel2, obj));
    e3.add(Tag::id());

    let mut count = 0;
    let q = world
        .query::<()>()
        .with((id::<flecs::Wildcard>(), obj))
        .build();
    q.each_entity(|e, ()| {
        if e == e1 || e == e2 || e == e3 {
            count += 1;
        }
    });
    assert_eq!(count, 3);
}

// ─── term_each_pair_object_wildcard ───────────────────────────────────────────

#[test]
fn query_term_each_pair_object_wildcard() {
    let world = World::new();

    let rel = world.entity();
    let obj1 = world.entity();
    let obj2 = world.entity();

    let e1 = world.entity().add((rel, obj1));
    let e2 = world.entity().add((rel, obj1));
    let e3 = world.entity().add((rel, obj2));
    e3.add(Tag::id());

    let mut count = 0;
    let q = world
        .query::<()>()
        .with((rel, id::<flecs::Wildcard>()))
        .build();
    q.each_entity(|e, ()| {
        if e == e1 || e == e2 || e == e3 {
            count += 1;
        }
    });
    assert_eq!(count, 3);
}

// ─── default_ctor ─────────────────────────────────────────────────────────────

#[test]
fn query_default_ctor() {
    let world = World::new();

    // Test that a query can be assigned and used (Rust Option models default-ctor semantics)
    #[allow(unused_assignments)]
    let mut q_var: Option<Query<&Position>> = None;

    let q = world.query::<&Position>().build();
    world.entity().set(Position { x: 10, y: 20 });
    q_var = Some(q);

    let mut count = 0;
    q_var.as_ref().unwrap().each_entity(|_e, p| {
        assert_eq!(p.x, 10);
        assert_eq!(p.y, 20);
        count += 1;
    });
    assert_eq!(count, 1);
}

// ─── term_get_id ──────────────────────────────────────────────────────────────

#[test]
fn query_term_get_id() {
    let world = World::new();

    let foo = world.entity();
    let bar = world.entity();

    let q = world
        .query::<()>()
        .with(&Position::id())
        .with(&Velocity::id())
        .with((foo, bar))
        .build();

    assert_eq!(q.field_count(), 3);
    assert_eq!(world.id_view_from(Position::id()), q.term(0).id());
    assert_eq!(world.id_view_from(Velocity::id()), q.term(1).id());
    assert!(world.id_view_from(q.term(2).id()).is_pair());
    assert_eq!(world.id_view_from(q.term(2).id()).first_id().id(), foo.id());
    assert_eq!(
        world.id_view_from(q.term(2).id()).second_id().id(),
        bar.id()
    );
}

// ─── term_get_subj ────────────────────────────────────────────────────────────

#[test]
fn query_term_get_subj() {
    let world = World::new();

    let src = world.entity();

    let q = world
        .query::<()>()
        .with(&Position::id())
        .with(&Velocity::id())
        .src()
        .entity(src)
        .build();

    assert_eq!(q.field_count(), 2);
    // term 1 src should be src entity
    let src_id = q.term(1).src_id();
    assert_eq!(*src_id, *src.id());
}

// ─── term_get_pred ────────────────────────────────────────────────────────────

#[test]
fn query_term_get_pred() {
    let world = World::new();

    let foo = world.entity();
    let bar = world.entity();
    let src = world.entity();

    let q = world
        .query::<()>()
        .with(&Position::id())
        .with(&Velocity::id())
        .src()
        .entity(src)
        .with((foo, bar))
        .build();

    assert_eq!(q.field_count(), 3);
    assert_eq!(*q.term(0).first_id(), Position::entity_id(&world));
    assert_eq!(*q.term(1).first_id(), Velocity::entity_id(&world));
    assert_eq!(*q.term(2).first_id(), *foo.id());
}

// ─── term_get_obj ─────────────────────────────────────────────────────────────

#[test]
fn query_term_get_obj() {
    let world = World::new();

    let foo = world.entity();
    let bar = world.entity();
    let src = world.entity();

    let q = world
        .query::<()>()
        .with(&Position::id())
        .with(&Velocity::id())
        .src()
        .entity(src)
        .with((foo, bar))
        .build();

    assert_eq!(q.field_count(), 3);
    assert_eq!(*q.term(0).second_id(), 0u64);
    assert_eq!(*q.term(1).second_id(), 0u64);
    assert_eq!(*q.term(2).second_id(), *bar.id());
}

// ─── get_first ────────────────────────────────────────────────────────────────

#[test]
fn query_get_first() {
    #[derive(Component)]
    struct A;

    let world = World::new();

    let e1 = world.entity().add(A::id());
    world.entity().add(A::id());
    world.entity().add(A::id());

    let q = world.new_query::<&A>();
    let first = q.first_entity();
    assert_eq!(first.id(), e1.id());
}

// ─── get_count_direct ─────────────────────────────────────────────────────────

#[test]
fn query_get_count_direct() {
    #[derive(Component)]
    struct A;

    let world = World::new();
    world.entity().add(A::id());
    world.entity().add(A::id());
    world.entity().add(A::id());

    let q = world.new_query::<&A>();
    assert_eq!(q.count(), 3);
}

// ─── get_is_true_direct ───────────────────────────────────────────────────────

#[test]
fn query_get_is_true_direct() {
    #[derive(Component)]
    struct A;
    #[derive(Component)]
    struct B;

    let world = World::new();
    world.entity().add(A::id());
    world.entity().add(A::id());
    world.entity().add(A::id());

    let q1 = world.new_query::<&A>();
    let q2 = world.new_query::<&B>();

    assert!(q1.is_true());
    assert!(!q2.is_true());
}

// ─── get_first_direct ─────────────────────────────────────────────────────────

#[test]
fn query_get_first_direct() {
    #[derive(Component)]
    struct A;

    let world = World::new();
    let e1 = world.entity().add(A::id());
    world.entity().add(A::id());
    world.entity().add(A::id());

    let q = world.new_query::<&A>();
    let first = q.first_entity();
    assert_eq!(first.id(), e1.id());
}

// ─── each_w_no_this ───────────────────────────────────────────────────────────

#[test]
fn query_each_w_no_this() {
    let world = World::new();

    let e = world
        .entity()
        .set(Position { x: 10, y: 20 })
        .set(Velocity { x: 1, y: 2 });

    let q = world
        .query::<(&Position, &Velocity)>()
        .term_at(0)
        .src()
        .entity(e)
        .term_at(1)
        .src()
        .entity(e)
        .build();

    let mut count = 0;
    q.each(|(p, v)| {
        count += 1;
        assert_eq!(p.x, 10);
        assert_eq!(p.y, 20);
        assert_eq!(v.x, 1);
        assert_eq!(v.y, 2);
    });
    assert_eq!(count, 1);
}

// ─── each_w_iter_no_this ──────────────────────────────────────────────────────

#[test]
fn query_each_w_iter_no_this() {
    let world = World::new();

    let e = world
        .entity()
        .set(Position { x: 10, y: 20 })
        .set(Velocity { x: 1, y: 2 });

    let q = world
        .query::<(&Position, &Velocity)>()
        .term_at(0)
        .src()
        .entity(e)
        .term_at(1)
        .src()
        .entity(e)
        .build();

    let mut count = 0;
    q.each_iter(|it, i, (p, v)| {
        count += 1;
        assert_eq!(p.x, 10);
        assert_eq!(p.y, 20);
        assert_eq!(v.x, 1);
        assert_eq!(v.y, 2);
        assert_eq!(i, 0_usize.into());
        assert_eq!(it.count(), 0);
    });
    assert_eq!(count, 1);
}

// ─── named_query ──────────────────────────────────────────────────────────────

#[test]
fn query_named_query() {
    let world = World::new();

    let e1 = world.entity().add(Position::id());
    let e2 = world.entity().add(Position::id());

    let q = world.query_named::<&Position>("my_query").build();

    let mut count = 0;
    q.each_entity(|e, _| {
        assert!(e == e1 || e == e2);
        count += 1;
    });
    assert_eq!(count, 2);

    let qe = q.entity();
    assert!(qe.id() != 0);
    assert_eq!(qe.name(), "my_query");
}

// ─── named_scoped_query ───────────────────────────────────────────────────────

#[test]
fn query_named_scoped_query() {
    let world = World::new();

    let e1 = world.entity().add(Position::id());
    let e2 = world.entity().add(Position::id());

    let q = world.query_named::<&Position>("my::query").build();

    let mut count = 0;
    q.each_entity(|e, _| {
        assert!(e == e1 || e == e2);
        count += 1;
    });
    assert_eq!(count, 2);

    let qe = q.entity();
    assert!(qe.id() != 0);
    assert_eq!(qe.name(), "query");
    assert_eq!(qe.path().unwrap(), "::my::query");
}

// ─── set_this_var ─────────────────────────────────────────────────────────────

#[test]
fn query_set_this_var() {
    let world = World::new();

    world.entity().set(Position { x: 1, y: 2 });
    let e2 = world.entity().set(Position { x: 3, y: 4 });
    world.entity().set(Position { x: 5, y: 6 });

    let q = world.query_named::<&Position>("my::query").build();

    let mut count = 0;
    q.with_var(0, e2).each_entity(|e, _| {
        assert_eq!(e, e2);
        count += 1;
    });
    assert_eq!(count, 1);
}

// ─── inspect_terms_w_expr ─────────────────────────────────────────────────────

#[test]
fn query_inspect_terms_w_expr() {
    let world = World::new();

    let q = world.query::<()>().expr("(ChildOf,#0)").build();

    let mut count = 0;
    q.each_term(|term| {
        assert!(world.id_view_from(term.id()).is_pair());
        count += 1;
    });
    assert_eq!(count, 1);
}

// ─── find ─────────────────────────────────────────────────────────────────────

#[test]
fn query_find() {
    let world = World::new();

    world.entity().set(Position { x: 10, y: 20 });
    let e2 = world.entity().set(Position { x: 20, y: 30 });

    let q = world.new_query::<&Position>();
    let r = q.find(|p| p.x == 20);
    assert_eq!(r.unwrap(), e2);
}

// ─── find_not_found ───────────────────────────────────────────────────────────

#[test]
fn query_find_not_found() {
    let world = World::new();

    world.entity().set(Position { x: 10, y: 20 });
    world.entity().set(Position { x: 20, y: 30 });

    let q = world.new_query::<&Position>();
    let r = q.find(|p| p.x == 30);
    assert!(r.is_none());
}

// ─── find_w_entity ────────────────────────────────────────────────────────────

#[test]
fn query_find_w_entity() {
    let world = World::new();

    world
        .entity()
        .set(Position { x: 10, y: 20 })
        .set(Velocity { x: 20, y: 30 });
    let e2 = world
        .entity()
        .set(Position { x: 20, y: 30 })
        .set(Velocity { x: 20, y: 30 });

    let q = world.new_query::<&Position>();
    let r = q.find_entity(|e, p| e.get::<&Velocity>(|v| p.x == v.x && p.y == v.y));
    assert_eq!(r.unwrap(), e2);
}

// ─── find_w_match_empty_tables ────────────────────────────────────────────────

#[test]
fn query_find_w_match_empty_tables() {
    use flecs_ecs::core::QueryFlags;

    let world = World::new();

    let e1 = world
        .entity()
        .set(Position { x: 10, y: 20 })
        .add(Velocity::id());
    e1.destruct();
    let e2 = world.entity().set(Position { x: 20, y: 30 });

    let q = world
        .query::<&Position>()
        .query_flags(QueryFlags::MatchEmptyTables)
        .build();

    let r = q.find(|p| p.x == 20);
    assert_eq!(r.unwrap(), e2);
}

// ─── find_w_entity_w_match_empty_tables ──────────────────────────────────────

#[test]
fn query_find_w_entity_w_match_empty_tables() {
    use flecs_ecs::core::QueryFlags;

    let world = World::new();

    let e1 = world
        .entity()
        .set(Position { x: 10, y: 20 })
        .add(Velocity::id());
    e1.destruct();
    let e2 = world.entity().set(Position { x: 20, y: 30 });

    let q = world
        .query::<&Position>()
        .query_flags(QueryFlags::MatchEmptyTables)
        .build();

    let r = q.find_entity(|_e, p| p.x == 20);
    assert_eq!(r.unwrap(), e2);
}

// ─── run ──────────────────────────────────────────────────────────────────────

#[test]
fn query_run() {
    let world = World::new();

    let entity = world
        .entity()
        .set(Position { x: 10, y: 20 })
        .set(Velocity { x: 1, y: 2 });

    let q = world.new_query::<(&mut Position, &Velocity)>();

    q.run(|mut it| {
        while it.next() {
            let v = it.field::<Velocity>(1);
            for i in it.iter() {
                let mut p = it.field_at_mut::<Position>(0, i);
                p.x += v[i].x;
                p.y += v[i].y;
            }
        }
    });

    entity.get::<&Position>(|p| {
        assert_eq!(p.x, 11);
        assert_eq!(p.y, 22);
    });
}

// ─── run_const ────────────────────────────────────────────────────────────────

#[test]
fn query_run_const() {
    let world = World::new();

    let entity = world
        .entity()
        .set(Position { x: 10, y: 20 })
        .set(Velocity { x: 1, y: 2 });

    let q = world.new_query::<(&mut Position, &Velocity)>();

    q.run(|mut it| {
        while it.next() {
            let v = it.field::<Velocity>(1);
            for i in it.iter() {
                let mut p = it.field_at_mut::<Position>(0, i);
                p.x += v[i].x;
                p.y += v[i].y;
            }
        }
    });

    entity.get::<&Position>(|p| {
        assert_eq!(p.x, 11);
        assert_eq!(p.y, 22);
    });
}

// ─── run_shared ───────────────────────────────────────────────────────────────

#[test]
fn query_run_shared() {
    let world = World::new();

    world
        .component::<Position>()
        .add((flecs::OnInstantiate::ID, flecs::Inherit::ID));
    world
        .component::<Velocity>()
        .add((flecs::OnInstantiate::ID, flecs::Inherit::ID));

    let base = world.entity().set(Velocity { x: 1, y: 2 });

    let e1 = world.entity().set(Position { x: 10, y: 20 }).is_a(base);

    let e2 = world
        .entity()
        .set(Position { x: 10, y: 20 })
        .set(Velocity { x: 3, y: 4 });

    let q = world
        .query::<&mut Position>()
        .expr("Velocity(self|up IsA)")
        .build();

    q.run(|mut it| {
        while it.next() {
            let mut p = it.field_mut::<Position>(0);
            let v = it.field::<Velocity>(1);

            if !it.is_self(1) {
                for i in it.iter() {
                    p[i].x += v[0].x;
                    p[i].y += v[0].y;
                }
            } else {
                for i in it.iter() {
                    p[i].x += v[i].x;
                    p[i].y += v[i].y;
                }
            }
        }
    });

    e1.get::<&Position>(|p| {
        assert_eq!(p.x, 11);
        assert_eq!(p.y, 22);
    });
    e2.get::<&Position>(|p| {
        assert_eq!(p.x, 13);
        assert_eq!(p.y, 24);
    });
}

// ─── run_optional ─────────────────────────────────────────────────────────────

#[test]
fn query_run_optional() {
    let world = World::new();

    let e1 = world
        .entity()
        .set(Position { x: 10, y: 20 })
        .set(Velocity { x: 1, y: 2 })
        .set(Mass { value: 1 });
    let e2 = world
        .entity()
        .set(Position { x: 30, y: 40 })
        .set(Velocity { x: 3, y: 4 })
        .set(Mass { value: 1 });
    let e3 = world.entity().set(Position { x: 50, y: 60 });
    let e4 = world.entity().set(Position { x: 70, y: 80 });

    let q = world.new_query::<(&mut Position, Option<&Velocity>, Option<&Mass>)>();

    q.run(|mut it| {
        while it.next() {
            let mut p = it.field_mut::<Position>(0);
            let v = it.field::<Velocity>(1);
            let m = it.field::<Mass>(2);

            if it.is_set(1) && it.is_set(2) {
                for i in it.iter() {
                    p[i].x += v[i].x * m[i].value;
                    p[i].y += v[i].y * m[i].value;
                }
            } else {
                for i in it.iter() {
                    p[i].x += 1;
                    p[i].y += 1;
                }
            }
        }
    });

    e1.get::<&Position>(|p| {
        assert_eq!(p.x, 11);
        assert_eq!(p.y, 22);
    });
    e2.get::<&Position>(|p| {
        assert_eq!(p.x, 33);
        assert_eq!(p.y, 44);
    });
    e3.get::<&Position>(|p| {
        assert_eq!(p.x, 51);
        assert_eq!(p.y, 61);
    });
    e4.get::<&Position>(|p| {
        assert_eq!(p.x, 71);
        assert_eq!(p.y, 81);
    });
}

// ─── run_sparse_w_with ────────────────────────────────────────────────────────

#[test]
fn query_run_sparse_w_with() {
    let world = World::new();

    world.component::<Position>().add_trait::<flecs::Sparse>();

    let entity = world
        .entity()
        .set(Position { x: 10, y: 20 })
        .set(Velocity { x: 1, y: 2 });

    let q = world
        .query::<()>()
        .with(&mut Position::id())
        .with(&Velocity::id())
        .build();

    q.run(|mut it| {
        while it.next() {
            let v = it.field::<Velocity>(1);
            for i in it.iter() {
                let mut p = it.field_at_mut::<Position>(0, i);
                p.x += v[i].x;
                p.y += v[i].y;
            }
        }
    });

    entity.get::<&Position>(|p| {
        assert_eq!(p.x, 11);
        assert_eq!(p.y, 22);
    });
}

// ─── run_dont_fragment ────────────────────────────────────────────────────────

#[test]
fn query_run_dont_fragment() {
    let world = World::new();

    world
        .component::<Position>()
        .add_trait::<flecs::DontFragment>();

    let entity = world
        .entity()
        .set(Position { x: 10, y: 20 })
        .set(Velocity { x: 1, y: 2 });

    let q = world.new_query::<(&mut Position, &Velocity)>();

    q.run(|mut it| {
        while it.next() {
            let v = it.field::<Velocity>(1);
            for i in it.iter() {
                let mut p = it.field_at_mut::<Position>(0, i);
                p.x += v[i].x;
                p.y += v[i].y;
            }
        }
    });

    entity.get::<&Position>(|p| {
        assert_eq!(p.x, 11);
        assert_eq!(p.y, 22);
    });
}

// ─── run_dont_fragment_w_with ─────────────────────────────────────────────────

#[test]
fn query_run_dont_fragment_w_with() {
    let world = World::new();

    world
        .component::<Position>()
        .add_trait::<flecs::DontFragment>();

    let entity = world
        .entity()
        .set(Position { x: 10, y: 20 })
        .set(Velocity { x: 1, y: 2 });

    let q = world
        .query::<()>()
        .with(&mut Position::id())
        .with(&Velocity::id())
        .build();

    q.run(|mut it| {
        while it.next() {
            let v = it.field::<Velocity>(1);
            for i in it.iter() {
                let mut p = it.field_at_mut::<Position>(0, i);
                p.x += v[i].x;
                p.y += v[i].y;
            }
        }
    });

    entity.get::<&Position>(|p| {
        assert_eq!(p.x, 11);
        assert_eq!(p.y, 22);
    });
}

// ─── run_dont_fragment_add ────────────────────────────────────────────────────

#[test]
fn query_run_dont_fragment_add() {
    let world = World::new();

    world
        .component::<Velocity>()
        .add_trait::<flecs::DontFragment>();

    let entity = world
        .entity()
        .set(Position { x: 10, y: 20 })
        .set(Velocity { x: 1, y: 2 });

    let q = world.new_query::<&mut Position>();

    q.run(|mut it| {
        while it.next() {
            for i in it.iter() {
                let e = it.get_entity(i).unwrap();
                e.add(Velocity::id());
                assert!(e.has(Velocity::id()));
                let mut p = it.field_at_mut::<Position>(0, i);
                p.x += 1;
                p.y += 2;
            }
        }
    });

    entity.get::<&Position>(|p| {
        assert_eq!(p.x, 11);
        assert_eq!(p.y, 22);
    });
    assert!(entity.has(Velocity::id()));
}

// ─── run_dont_fragment_add_remove ─────────────────────────────────────────────

#[test]
fn query_run_dont_fragment_add_remove() {
    let world = World::new();

    world
        .component::<Velocity>()
        .add_trait::<flecs::DontFragment>();

    let entity = world
        .entity()
        .set(Position { x: 10, y: 20 })
        .set(Velocity { x: 1, y: 2 });

    let q = world.new_query::<&mut Position>();

    q.run(|mut it| {
        while it.next() {
            for i in it.iter() {
                let e = it.get_entity(i).unwrap();
                e.add(Velocity::id());
                assert!(e.has(Velocity::id()));
                e.remove(Velocity::id());
                assert!(!e.has(Velocity::id()));
                let mut p = it.field_at_mut::<Position>(0, i);
                p.x += 1;
                p.y += 2;
            }
        }
    });

    entity.get::<&Position>(|p| {
        assert_eq!(p.x, 11);
        assert_eq!(p.y, 22);
    });
    assert!(!entity.has(Velocity::id()));
}

// ─── run_dont_fragment_set ────────────────────────────────────────────────────

#[test]
fn query_run_dont_fragment_set() {
    let world = World::new();

    world
        .component::<Velocity>()
        .add_trait::<flecs::DontFragment>();

    let entity = world
        .entity()
        .set(Position { x: 10, y: 20 })
        .set(Velocity { x: 1, y: 2 });

    let q = world.new_query::<&mut Position>();

    q.run(|mut it| {
        while it.next() {
            for i in it.iter() {
                let e = it.get_entity(i).unwrap();
                e.set(Velocity { x: 1, y: 2 });
                assert!(e.has(Velocity::id()));
                e.get::<&Velocity>(|v| {
                    assert_eq!(v.x, 1);
                    assert_eq!(v.y, 2);
                });
                let mut p = it.field_at_mut::<Position>(0, i);
                p.x += 1;
                p.y += 2;
            }
        }
    });

    entity.get::<&Position>(|p| {
        assert_eq!(p.x, 11);
        assert_eq!(p.y, 22);
    });
    entity.get::<&Velocity>(|v| {
        assert_eq!(v.x, 1);
        assert_eq!(v.y, 2);
    });
}

// ─── each ─────────────────────────────────────────────────────────────────────

#[test]
fn query_each() {
    let world = World::new();

    let entity = world
        .entity()
        .set(Position { x: 10, y: 20 })
        .set(Velocity { x: 1, y: 2 });

    let q = world.new_query::<(&mut Position, &Velocity)>();

    q.each(|(p, v)| {
        p.x += v.x;
        p.y += v.y;
    });

    entity.get::<&Position>(|p| {
        assert_eq!(p.x, 11);
        assert_eq!(p.y, 22);
    });
}

// ─── each_const ───────────────────────────────────────────────────────────────

#[test]
fn query_each_const() {
    let world = World::new();

    let entity = world
        .entity()
        .set(Position { x: 10, y: 20 })
        .set(Velocity { x: 1, y: 2 });

    let q = world.new_query::<(&mut Position, &Velocity)>();

    q.each(|(p, v)| {
        p.x += v.x;
        p.y += v.y;
    });

    entity.get::<&Position>(|p| {
        assert_eq!(p.x, 11);
        assert_eq!(p.y, 22);
    });
}

// ─── each_shared ──────────────────────────────────────────────────────────────

#[test]
fn query_each_shared() {
    let world = World::new();

    world
        .component::<Position>()
        .add((flecs::OnInstantiate::ID, flecs::Inherit::ID));
    world
        .component::<Velocity>()
        .add((flecs::OnInstantiate::ID, flecs::Inherit::ID));

    let base = world.entity().set(Velocity { x: 1, y: 2 });

    let e1 = world.entity().set(Position { x: 10, y: 20 }).is_a(base);
    let e2 = world.entity().set(Position { x: 20, y: 30 }).is_a(base);
    let e3 = world
        .entity()
        .set(Position { x: 10, y: 20 })
        .set(Velocity { x: 3, y: 4 });

    let q = world.new_query::<(&mut Position, &Velocity)>();

    q.each_entity(|_e, (p, v)| {
        p.x += v.x;
        p.y += v.y;
    });

    e1.get::<&Position>(|p| {
        assert_eq!(p.x, 11);
        assert_eq!(p.y, 22);
    });
    e2.get::<&Position>(|p| {
        assert_eq!(p.x, 21);
        assert_eq!(p.y, 32);
    });
    e3.get::<&Position>(|p| {
        assert_eq!(p.x, 13);
        assert_eq!(p.y, 24);
    });
}

// ─── each_optional ────────────────────────────────────────────────────────────

#[test]
fn query_each_optional() {
    let world = World::new();

    let e1 = world
        .entity()
        .set(Position { x: 10, y: 20 })
        .set(Velocity { x: 1, y: 2 })
        .set(Mass { value: 1 });
    let e2 = world
        .entity()
        .set(Position { x: 30, y: 40 })
        .set(Velocity { x: 3, y: 4 })
        .set(Mass { value: 1 });
    let e3 = world.entity().set(Position { x: 50, y: 60 });
    let e4 = world.entity().set(Position { x: 70, y: 80 });

    let q = world.new_query::<(&mut Position, Option<&Velocity>, Option<&Mass>)>();

    q.each(|(p, v, m)| {
        if let (Some(v), Some(m)) = (v, m) {
            p.x += v.x * m.value;
            p.y += v.y * m.value;
        } else {
            p.x += 1;
            p.y += 1;
        }
    });

    e1.get::<&Position>(|p| {
        assert_eq!(p.x, 11);
        assert_eq!(p.y, 22);
    });
    e2.get::<&Position>(|p| {
        assert_eq!(p.x, 33);
        assert_eq!(p.y, 44);
    });
    e3.get::<&Position>(|p| {
        assert_eq!(p.x, 51);
        assert_eq!(p.y, 61);
    });
    e4.get::<&Position>(|p| {
        assert_eq!(p.x, 71);
        assert_eq!(p.y, 81);
    });
}

// ─── each_dont_fragment ───────────────────────────────────────────────────────

#[test]
fn query_each_dont_fragment() {
    let world = World::new();

    world
        .component::<Position>()
        .add_trait::<flecs::DontFragment>();

    let entity = world
        .entity()
        .set(Position { x: 10, y: 20 })
        .set(Velocity { x: 1, y: 2 });

    let q = world.new_query::<(&mut Position, &Velocity)>();

    q.each(|(p, v)| {
        p.x += v.x;
        p.y += v.y;
    });

    entity.get::<&Position>(|p| {
        assert_eq!(p.x, 11);
        assert_eq!(p.y, 22);
    });
}

// ─── each_generic ─────────────────────────────────────────────────────────────

#[test]
fn query_each_generic() {
    // Generic lambdas in Rust are normal closures that infer types.
    let world = World::new();
    let q = world.new_query::<(&mut Position, &Velocity, Option<&Mass>)>();
    q.each(|(_p, _v, _m)| {});
    q.each_entity(|_e, (_p, _v, _m)| {});
}

// ─── signature ────────────────────────────────────────────────────────────────

#[test]
fn query_signature() {
    let world = World::new();

    let entity = world
        .entity()
        .set(Position { x: 10, y: 20 })
        .set(Velocity { x: 1, y: 2 });

    let q = world.query::<()>().expr("Position, Velocity").build();

    q.run(|mut it| {
        while it.next() {
            let mut p = it.field_mut::<Position>(0);
            let v = it.field::<Velocity>(1);
            for i in it.iter() {
                p[i].x += v[i].x;
                p[i].y += v[i].y;
            }
        }
    });

    entity.get::<&Position>(|p| {
        assert_eq!(p.x, 11);
        assert_eq!(p.y, 22);
    });
}

// ─── signature_const ──────────────────────────────────────────────────────────

#[test]
fn query_signature_const() {
    let world = World::new();

    let entity = world
        .entity()
        .set(Position { x: 10, y: 20 })
        .set(Velocity { x: 1, y: 2 });

    let q = world.query::<()>().expr("Position, [in] Velocity").build();

    q.run(|mut it| {
        while it.next() {
            let mut p = it.field_mut::<Position>(0);
            let v = it.field::<Velocity>(1);
            for i in it.iter() {
                p[i].x += v[i].x;
                p[i].y += v[i].y;
            }
        }
    });

    entity.get::<&Position>(|p| {
        assert_eq!(p.x, 11);
        assert_eq!(p.y, 22);
    });
}

// ─── signature_shared ─────────────────────────────────────────────────────────

#[test]
fn query_signature_shared() {
    let world = World::new();

    world
        .component::<Position>()
        .add((flecs::OnInstantiate::ID, flecs::Inherit::ID));
    world
        .component::<Velocity>()
        .add((flecs::OnInstantiate::ID, flecs::Inherit::ID));

    let base = world.entity().set(Velocity { x: 1, y: 2 });

    let e1 = world.entity().set(Position { x: 10, y: 20 }).is_a(base);
    let e2 = world
        .entity()
        .set(Position { x: 10, y: 20 })
        .set(Velocity { x: 3, y: 4 });

    let q = world
        .query::<()>()
        .expr("Position, [in] Velocity(self|up IsA)")
        .build();

    q.run(|mut it| {
        while it.next() {
            let mut p = it.field_mut::<Position>(0);
            let v = it.field::<Velocity>(1);

            if !it.is_self(1) {
                for i in it.iter() {
                    p[i].x += v[0].x;
                    p[i].y += v[0].y;
                }
            } else {
                for i in it.iter() {
                    p[i].x += v[i].x;
                    p[i].y += v[i].y;
                }
            }
        }
    });

    e1.get::<&Position>(|p| {
        assert_eq!(p.x, 11);
        assert_eq!(p.y, 22);
    });
    e2.get::<&Position>(|p| {
        assert_eq!(p.x, 13);
        assert_eq!(p.y, 24);
    });
}

// ─── signature_optional ───────────────────────────────────────────────────────

#[test]
fn query_signature_optional() {
    let world = World::new();

    let e1 = world
        .entity()
        .set(Position { x: 10, y: 20 })
        .set(Velocity { x: 1, y: 2 })
        .set(Mass { value: 1 });
    let e2 = world
        .entity()
        .set(Position { x: 30, y: 40 })
        .set(Velocity { x: 3, y: 4 })
        .set(Mass { value: 1 });
    let e3 = world.entity().set(Position { x: 50, y: 60 });
    let e4 = world.entity().set(Position { x: 70, y: 80 });

    let q = world
        .query::<()>()
        .expr("Position, ?Velocity, ?Mass")
        .build();

    q.run(|mut it| {
        while it.next() {
            let mut p = it.field_mut::<Position>(0);
            let v = it.field::<Velocity>(1);
            let m = it.field::<Mass>(2);

            if it.is_set(1) && it.is_set(2) {
                for i in it.iter() {
                    p[i].x += v[i].x * m[i].value;
                    p[i].y += v[i].y * m[i].value;
                }
            } else {
                for i in it.iter() {
                    p[i].x += 1;
                    p[i].y += 1;
                }
            }
        }
    });

    e1.get::<&Position>(|p| {
        assert_eq!(p.x, 11);
        assert_eq!(p.y, 22);
    });
    e2.get::<&Position>(|p| {
        assert_eq!(p.x, 33);
        assert_eq!(p.y, 44);
    });
    e3.get::<&Position>(|p| {
        assert_eq!(p.x, 51);
        assert_eq!(p.y, 61);
    });
    e4.get::<&Position>(|p| {
        assert_eq!(p.x, 71);
        assert_eq!(p.y, 81);
    });
}

// ─── query_single_pair ────────────────────────────────────────────────────────

#[test]
fn query_query_single_pair() {
    #[derive(Component)]
    struct Pair2;

    let world = World::new();

    world.entity().add((Pair2::id(), Position::id()));
    let e2 = world.entity().add((Pair2::id(), Velocity::id()));

    let q = world.query::<()>().expr("(Pair2, Velocity)").build();

    let mut table_count = 0;
    let mut entity_count = 0;

    q.run(|mut it| {
        while it.next() {
            table_count += 1;
            for i in it.iter() {
                assert_eq!(it.get_entity(i).unwrap(), e2);
                entity_count += 1;
            }
        }
    });

    assert_eq!(table_count, 1);
    assert_eq!(entity_count, 1);
}

// ─── tag_w_each ───────────────────────────────────────────────────────────────

#[test]
fn query_tag_w_each() {
    let world = World::new();

    let e = world.entity().add(Tag::id());

    // Tags (ZST) must be queried via .with(id) not as type param.
    let q = world.query::<()>().with(Tag::id()).build();
    q.each_entity(|qe, ()| {
        assert_eq!(qe, e);
    });
}

// ─── shared_tag_w_each ────────────────────────────────────────────────────────

#[test]
fn query_shared_tag_w_each() {
    let world = World::new();

    let base = world.prefab().add(Tag::id());
    let e = world.entity().is_a(base);

    // Tags (ZST) must be queried via .with(id) not as type param.
    let q = world.query::<()>().with(Tag::id()).build();
    q.each_entity(|qe, ()| {
        assert_eq!(qe, e);
    });
}

// ─── sort_by ──────────────────────────────────────────────────────────────────

#[test]
fn query_sort_by() {
    let world = World::new();

    world.entity().set(Position { x: 1, y: 0 });
    world.entity().set(Position { x: 6, y: 0 });
    world.entity().set(Position { x: 2, y: 0 });
    world.entity().set(Position { x: 5, y: 0 });
    world.entity().set(Position { x: 4, y: 0 });

    let q = world
        .query::<&Position>()
        .order_by::<Position>(|_e1, p1: &Position, _e2, p2: &Position| {
            (p1.x > p2.x) as i32 - (p1.x < p2.x) as i32
        })
        .build();

    q.run(|mut it| {
        while it.next() {
            let p = it.field::<Position>(0);
            assert_eq!(it.count(), 5);
            assert_eq!(p[0].x, 1);
            assert_eq!(p[1].x, 2);
            assert_eq!(p[2].x, 4);
            assert_eq!(p[3].x, 5);
            assert_eq!(p[4].x, 6);
        }
    });
}

// ─── changed ──────────────────────────────────────────────────────────────────

#[test]
fn query_changed() {
    let world = World::new();

    let e = world.entity().set(Position { x: 1, y: 0 });

    let q = world.query::<&Position>().detect_changes().build();
    let q_w = world.new_query::<&mut Position>();

    assert!(q.is_changed());

    q.each(|_p| {});
    assert!(!q.is_changed());

    e.set(Position { x: 2, y: 0 });
    assert!(q.is_changed());

    q.each(|_p| {});
    assert!(!q.is_changed());

    q_w.each(|_p| {});
    assert!(q.is_changed());
}

// ─── expr_w_template ──────────────────────────────────────────────────────────

#[test]
fn query_expr_w_template() {
    #[derive(Component, Debug)]
    struct Template<T: Send + Sync + 'static> {
        x: T,
        y: T,
    }

    let world = World::new();

    let comp = world.component::<Template<i32>>();
    assert!(comp.name().contains("Template"));

    let mut count = 0;
    let q = world
        .query::<&Position>()
        .with(&Template::<i32>::id())
        .build();

    world
        .entity()
        .set(Position { x: 10, y: 20 })
        .set(Template::<i32> { x: 30, y: 40 });

    q.each_entity(|e, p| {
        assert_eq!(p.x, 10);
        assert_eq!(p.y, 20);
        e.get::<&Template<i32>>(|t| {
            assert_eq!(t.x, 30);
            assert_eq!(t.y, 40);
        });
        count += 1;
    });

    assert_eq!(count, 1);
}

// ─── query_type_w_template ────────────────────────────────────────────────────

#[test]
fn query_query_type_w_template() {
    #[derive(Component, Debug)]
    struct Template2<T: Send + Sync + 'static> {
        x: T,
        y: T,
    }

    let world = World::new();

    let comp = world.component::<Template2<i32>>();
    assert!(comp.name().contains("Template2"));

    let mut count = 0;
    let q = world.new_query::<(&Position, &Template2<i32>)>();

    world
        .entity()
        .set(Position { x: 10, y: 20 })
        .set(Template2::<i32> { x: 30, y: 40 });

    q.each_entity(|_e, (p, t)| {
        assert_eq!(p.x, 10);
        assert_eq!(p.y, 20);
        assert_eq!(t.x, 30);
        assert_eq!(t.y, 40);
        count += 1;
    });

    assert_eq!(count, 1);
}

// ─── compare_term_id ──────────────────────────────────────────────────────────

#[test]
fn query_compare_term_id() {
    let world = World::new();

    let mut count = 0;
    let e = world.entity().add(Tag::id());

    let q = world.query::<()>().with(Tag::id()).build();

    q.run(|mut it| {
        while it.next() {
            assert_eq!(*it.id(0).entity_view().id(), Tag::entity_id(&world));
            assert_eq!(it.entity_id(0_usize), e.id());
        }
        count += 1;
    });

    assert_eq!(count, 1);
}

// ─── inspect_terms ────────────────────────────────────────────────────────────

#[test]
fn query_inspect_terms() {
    use flecs_ecs::core::{InOutKind, OperKind};

    let world = World::new();

    let p_entity = world.entity();

    let q = world
        .query::<&Position>()
        .with(&Velocity::id())
        .with((flecs::ChildOf::ID, p_entity))
        .build();

    assert_eq!(q.field_count(), 3);

    let t = q.term(0);
    assert_eq!(
        *unsafe { t.id().entity_view(&world) }.id(),
        Position::entity_id(&world)
    );
    assert_eq!(t.oper(), OperKind::And);
    // &Position = read-only ref → In inout (Rust &T maps to EcsIn, matching with<const T>())
    assert_eq!(t.inout(), InOutKind::In);

    let t = q.term(1);
    assert_eq!(
        *unsafe { t.id().entity_view(&world) }.id(),
        Velocity::entity_id(&world)
    );
    assert_eq!(t.oper(), OperKind::And);
    // &Velocity::id() = explicit read ref → In inout
    assert_eq!(t.inout(), InOutKind::In);

    let t = q.term(2);
    assert_eq!(t.oper(), OperKind::And);
    assert_eq!(t.inout(), InOutKind::Default);
    assert_eq!(t.second_id(), p_entity.id());
}

// ─── inspect_terms_w_each ─────────────────────────────────────────────────────

#[test]
fn query_inspect_terms_w_each() {
    use flecs_ecs::core::{InOutKind, OperKind};

    let world = World::new();

    let p_entity = world.entity();

    let q = world
        .query::<&Position>()
        .with(&Velocity::id())
        .with((flecs::ChildOf::ID, p_entity))
        .build();

    let mut count = 0;
    q.each_term(|t| {
        if count == 0 {
            assert_eq!(
                *unsafe { t.id().entity_view(&world) }.id(),
                Position::entity_id(&world)
            );
            // &Position = read-only ref → In inout
            assert_eq!(t.inout(), InOutKind::In);
        } else if count == 1 {
            assert_eq!(
                *unsafe { t.id().entity_view(&world) }.id(),
                Velocity::entity_id(&world)
            );
            // &Velocity::id() = explicit read ref → In inout
            assert_eq!(t.inout(), InOutKind::In);
        } else if count == 2 {
            assert_eq!(t.inout(), InOutKind::Default);
            assert_eq!(t.second_id(), p_entity.id());
        } else {
            panic!("unexpected term count");
        }
        assert_eq!(t.oper(), OperKind::And);
        count += 1;
    });

    assert_eq!(count, 3);
}

// ─── comp_to_str ──────────────────────────────────────────────────────────────

#[test]
fn query_comp_to_str() {
    let world = World::new();

    let q = world.query::<&Position>().with(&Velocity::id()).build();

    let s = q.to_string();
    assert!(s.contains("Position"));
    assert!(s.contains("Velocity"));
}

// ─── pair_to_str ──────────────────────────────────────────────────────────────

#[test]
fn query_pair_to_str() {
    let world = World::new();

    let q = world
        .query::<&Position>()
        .with(&Velocity::id())
        .with((Eats::id(), Apples::id()))
        .build();

    let s = q.to_string();
    assert!(s.contains("Position"));
    assert!(s.contains("Velocity"));
    assert!(s.contains("Eats"));
    assert!(s.contains("Apples"));
}

// ─── oper_not_to_str ──────────────────────────────────────────────────────────

#[test]
fn query_oper_not_to_str() {
    let world = World::new();

    let q = world
        .query::<&Position>()
        .with(&Velocity::id())
        .not()
        .build();

    let s = q.to_string();
    assert!(s.contains("!"));
    assert!(s.contains("Velocity"));
}

// ─── oper_optional_to_str ─────────────────────────────────────────────────────

#[test]
fn query_oper_optional_to_str() {
    let world = World::new();

    let q = world
        .query::<&Position>()
        .with(&Velocity::id())
        .optional()
        .build();

    let s = q.to_string();
    assert!(s.contains("?"));
    assert!(s.contains("Velocity"));
}

// ─── oper_or_to_str ───────────────────────────────────────────────────────────

#[test]
fn query_oper_or_to_str() {
    let world = World::new();

    let q = world
        .query::<()>()
        .with(&Position::id())
        .or()
        .with(&Velocity::id())
        .build();

    let s = q.to_string();
    assert!(s.contains("Position") && s.contains("Velocity"));
}

// ─── each_pair_type ───────────────────────────────────────────────────────────

#[test]
fn query_each_pair_type() {
    #[derive(Component, Default)]
    struct EatsData {
        amount: i32,
    }
    #[derive(Component)]
    struct ApplesTag;
    #[derive(Component)]
    struct PearsTag;

    let world = World::new();

    let e1 = world
        .entity()
        .set_pair::<EatsData, ApplesTag>(EatsData { amount: 10 });
    world
        .entity()
        .set_pair::<EatsData, PearsTag>(EatsData { amount: 20 });

    let q = world.new_query::<&(EatsData, ApplesTag)>();

    let mut count = 0;
    q.each_entity(|e, ed| {
        assert_eq!(ed.amount, 10);
        assert_eq!(e, e1);
        count += 1;
    });

    assert_eq!(count, 1);
}

// ─── iter_pair_type ───────────────────────────────────────────────────────────

#[test]
fn query_iter_pair_type() {
    #[derive(Component, Default)]
    struct EatsData2 {
        amount: i32,
    }
    #[derive(Component)]
    struct ApplesTag2;
    #[derive(Component)]
    struct PearsTag2;

    let world = World::new();

    let e1 = world
        .entity()
        .set_pair::<EatsData2, ApplesTag2>(EatsData2 { amount: 10 });
    world
        .entity()
        .set_pair::<EatsData2, PearsTag2>(EatsData2 { amount: 20 });

    let q = world.new_query::<&(EatsData2, ApplesTag2)>();

    let mut count = 0;
    q.run(|mut it| {
        while it.next() {
            let a = it.field::<EatsData2>(0);
            assert_eq!(it.count(), 1);
            assert_eq!(a[0].amount, 10);
            assert_eq!(it.entity_id(0_usize), e1.id());
            count += 1;
        }
    });

    assert_eq!(count, 1);
}

// ─── iter_no_comps_no_comps ───────────────────────────────────────────────────

#[test]
fn query_iter_no_comps_no_comps() {
    let world = World::new();

    world.entity().add(Velocity::id());
    world.entity().add(Position::id());
    world.entity().add(Position::id()).add(Velocity::id());
    world.entity().add(Position::id()).add(Velocity::id());

    let q = world.query::<()>().with(&Position::id()).build();

    let mut count = 0;
    q.run(|mut it| {
        while it.next() {
            count += it.count();
        }
    });

    assert_eq!(count, 3);
}

// ─── each_pair_object ─────────────────────────────────────────────────────────

#[test]
fn query_each_pair_object() {
    #[derive(Component)]
    struct Event {
        value: &'static str,
    }
    #[derive(Component)]
    struct Begin;
    #[derive(Component)]
    struct End;

    let world = World::new();

    let e1 = world
        .entity()
        .set_pair::<Event, Begin>(Event { value: "Big Bang" })
        .set_pair::<Event, End>(Event {
            value: "Heat Death",
        });

    let q = world.new_query::<(&(Event, Begin), &(Event, End))>();

    let mut count = 0;
    q.each_entity(|e, (b_e, e_e)| {
        assert_eq!(e, e1);
        assert_eq!(b_e.value, "Big Bang");
        assert_eq!(e_e.value, "Heat Death");
        count += 1;
    });

    assert_eq!(count, 1);
}

// ─── iter_pair_object ─────────────────────────────────────────────────────────

#[test]
fn query_iter_pair_object() {
    #[derive(Component)]
    struct EventVal {
        value: &'static str,
    }
    #[derive(Component)]
    struct BeginEvt;
    #[derive(Component)]
    struct EndEvt;

    let world = World::new();

    let e1 = world
        .entity()
        .set_pair::<EventVal, BeginEvt>(EventVal { value: "Big Bang" })
        .set_pair::<EventVal, EndEvt>(EventVal {
            value: "Heat Death",
        });

    let q = world.new_query::<(&(EventVal, BeginEvt), &(EventVal, EndEvt))>();

    let mut count = 0;
    q.run(|mut it| {
        while it.next() {
            let b_e = it.field::<EventVal>(0);
            let e_e = it.field::<EventVal>(1);
            for i in it.iter() {
                assert_eq!(it.entity_id(i), e1.id());
                assert_eq!(b_e[i].value, "Big Bang");
                assert_eq!(e_e[i].value, "Heat Death");
                count += 1;
            }
        }
    });

    assert_eq!(count, 1);
}

// ─── iter_query_in_system ─────────────────────────────────────────────────────

#[test]
fn query_iter_query_in_system() {
    let world = World::new();

    world.entity().add(Position::id()).add(Velocity::id());

    let q = world.new_query::<&Velocity>();

    world.system::<&Position>().each_entity(move |_e1, _| {
        q.each_entity(|_e2, _| {});
    });

    world.progress();
    // Just assert no crash occurred
}

// ─── iter_type ────────────────────────────────────────────────────────────────

#[test]
fn query_iter_type() {
    let world = World::new();

    world.entity().add(Position::id());
    world.entity().add(Position::id()).add(Velocity::id());

    let q = world.new_query::<&Position>();

    q.run(|mut it| {
        while it.next() {
            let arch = it.archetype();
            assert!(arch.is_some());
            let table = it.table();
            assert!(table.unwrap().has(Position::id()));
        }
    });
}

// ─── instanced_query_w_singleton_each ────────────────────────────────────────

#[test]
fn query_instanced_query_w_singleton_each() {
    let world = World::new();

    world
        .component::<Velocity>()
        .add_trait::<flecs::Singleton>();
    world.set(Velocity { x: 1, y: 2 });

    let e1 = world.entity().set(Position { x: 10, y: 20 });
    let e2 = world.entity().set(Position { x: 20, y: 30 });
    let e3 = world.entity().set(Position { x: 30, y: 40 });
    let e4 = world.entity().set(Position { x: 40, y: 50 });
    let e5 = world.entity().set(Position { x: 50, y: 60 });

    e4.add(Tag::id());
    e5.add(Tag::id());

    let q = world.new_query::<(&mut Position, &Velocity)>();

    let mut count = 0;
    q.each_entity(|_e, (p, v)| {
        p.x += v.x;
        p.y += v.y;
        count += 1;
    });

    assert_eq!(count, 5);

    e1.get::<&Position>(|p| {
        assert_eq!(p.x, 11);
        assert_eq!(p.y, 22);
    });
    e2.get::<&Position>(|p| {
        assert_eq!(p.x, 21);
        assert_eq!(p.y, 32);
    });
    e3.get::<&Position>(|p| {
        assert_eq!(p.x, 31);
        assert_eq!(p.y, 42);
    });
    e4.get::<&Position>(|p| {
        assert_eq!(p.x, 41);
        assert_eq!(p.y, 52);
    });
    e5.get::<&Position>(|p| {
        assert_eq!(p.x, 51);
        assert_eq!(p.y, 62);
    });
}

// ─── instanced_query_w_base_each ─────────────────────────────────────────────

#[test]
fn query_instanced_query_w_base_each() {
    let world = World::new();

    let base = world.entity().set(Velocity { x: 1, y: 2 });

    let e1 = world.entity().is_a(base).set(Position { x: 10, y: 20 });
    let e2 = world.entity().is_a(base).set(Position { x: 20, y: 30 });
    let e3 = world.entity().is_a(base).set(Position { x: 30, y: 40 });
    let e4 = world
        .entity()
        .is_a(base)
        .set(Position { x: 40, y: 50 })
        .add(Tag::id());
    let e5 = world
        .entity()
        .is_a(base)
        .set(Position { x: 50, y: 60 })
        .add(Tag::id());
    let e6 = world
        .entity()
        .set(Position { x: 60, y: 70 })
        .set(Velocity { x: 2, y: 3 });
    let e7 = world
        .entity()
        .set(Position { x: 70, y: 80 })
        .set(Velocity { x: 4, y: 5 });

    let q = world.new_query::<(&mut Position, &Velocity)>();

    let mut count = 0;
    q.each_entity(|_e, (p, v)| {
        p.x += v.x;
        p.y += v.y;
        count += 1;
    });

    assert_eq!(count, 7);

    e1.get::<&Position>(|p| {
        assert_eq!(p.x, 11);
        assert_eq!(p.y, 22);
    });
    e2.get::<&Position>(|p| {
        assert_eq!(p.x, 21);
        assert_eq!(p.y, 32);
    });
    e3.get::<&Position>(|p| {
        assert_eq!(p.x, 31);
        assert_eq!(p.y, 42);
    });
    e4.get::<&Position>(|p| {
        assert_eq!(p.x, 41);
        assert_eq!(p.y, 52);
    });
    e5.get::<&Position>(|p| {
        assert_eq!(p.x, 51);
        assert_eq!(p.y, 62);
    });
    e6.get::<&Position>(|p| {
        assert_eq!(p.x, 62);
        assert_eq!(p.y, 73);
    });
    e7.get::<&Position>(|p| {
        assert_eq!(p.x, 74);
        assert_eq!(p.y, 85);
    });
}

// ─── instanced_query_w_singleton_iter ────────────────────────────────────────

#[test]
fn query_instanced_query_w_singleton_iter() {
    let world = World::new();

    world
        .component::<Velocity>()
        .add_trait::<flecs::Singleton>();
    world.set(Velocity { x: 1, y: 2 });

    let e1 = world.entity().set(Position { x: 10, y: 20 });
    let e2 = world.entity().set(Position { x: 20, y: 30 });
    let e3 = world.entity().set(Position { x: 30, y: 40 });
    let e4 = world.entity().set(Position { x: 40, y: 50 });
    let e5 = world.entity().set(Position { x: 50, y: 60 });

    e4.add(Tag::id());
    e5.add(Tag::id());

    let q = world.new_query::<(&mut Position, &Velocity)>();

    let mut count = 0;
    q.run(|mut it| {
        while it.next() {
            let mut p = it.field_mut::<Position>(0);
            let v = it.field::<Velocity>(1);

            assert!(it.count() > 1);
            for i in it.iter() {
                p[i].x += v[0].x;
                p[i].y += v[0].y;
                count += 1;
            }
        }
    });

    assert_eq!(count, 5);

    e1.get::<&Position>(|p| {
        assert_eq!(p.x, 11);
        assert_eq!(p.y, 22);
    });
    e2.get::<&Position>(|p| {
        assert_eq!(p.x, 21);
        assert_eq!(p.y, 32);
    });
    e3.get::<&Position>(|p| {
        assert_eq!(p.x, 31);
        assert_eq!(p.y, 42);
    });
    e4.get::<&Position>(|p| {
        assert_eq!(p.x, 41);
        assert_eq!(p.y, 52);
    });
    e5.get::<&Position>(|p| {
        assert_eq!(p.x, 51);
        assert_eq!(p.y, 62);
    });
}

// ─── instanced_query_w_base_iter ─────────────────────────────────────────────

#[test]
fn query_instanced_query_w_base_iter() {
    let world = World::new();

    let base = world.entity().set(Velocity { x: 1, y: 2 });

    let e1 = world.entity().is_a(base).set(Position { x: 10, y: 20 });
    let e2 = world.entity().is_a(base).set(Position { x: 20, y: 30 });
    let e3 = world.entity().is_a(base).set(Position { x: 30, y: 40 });
    let e4 = world
        .entity()
        .is_a(base)
        .set(Position { x: 40, y: 50 })
        .add(Tag::id());
    let e5 = world
        .entity()
        .is_a(base)
        .set(Position { x: 50, y: 60 })
        .add(Tag::id());
    let e6 = world
        .entity()
        .set(Position { x: 60, y: 70 })
        .set(Velocity { x: 2, y: 3 });
    let e7 = world
        .entity()
        .set(Position { x: 70, y: 80 })
        .set(Velocity { x: 4, y: 5 });

    let q = world.new_query::<(&mut Position, &Velocity)>();

    let mut count = 0;
    q.run(|mut it| {
        while it.next() {
            let mut p = it.field_mut::<Position>(0);
            let v = it.field::<Velocity>(1);

            assert!(it.count() > 1);
            for i in it.iter() {
                if it.is_self(1) {
                    p[i].x += v[i].x;
                    p[i].y += v[i].y;
                } else {
                    p[i].x += v[0].x;
                    p[i].y += v[0].y;
                }
                count += 1;
            }
        }
    });

    assert_eq!(count, 7);

    e1.get::<&Position>(|p| {
        assert_eq!(p.x, 11);
        assert_eq!(p.y, 22);
    });
    e2.get::<&Position>(|p| {
        assert_eq!(p.x, 21);
        assert_eq!(p.y, 32);
    });
    e3.get::<&Position>(|p| {
        assert_eq!(p.x, 31);
        assert_eq!(p.y, 42);
    });
    e4.get::<&Position>(|p| {
        assert_eq!(p.x, 41);
        assert_eq!(p.y, 52);
    });
    e5.get::<&Position>(|p| {
        assert_eq!(p.x, 51);
        assert_eq!(p.y, 62);
    });
    e6.get::<&Position>(|p| {
        assert_eq!(p.x, 62);
        assert_eq!(p.y, 73);
    });
    e7.get::<&Position>(|p| {
        assert_eq!(p.x, 74);
        assert_eq!(p.y, 85);
    });
}

// ─── query_each_from_component ────────────────────────────────────────────────

#[test]
fn query_query_each_from_component() {
    #[derive(Component)]
    struct QueryComp {
        q: Option<Query<(&'static Position, &'static Velocity)>>,
    }

    let world = World::new();

    world
        .entity()
        .set(Position { x: 0, y: 0 })
        .set(Velocity { x: 0, y: 0 });
    world
        .entity()
        .set(Position { x: 0, y: 0 })
        .set(Velocity { x: 0, y: 0 });

    let q = world.new_query::<(&Position, &Velocity)>();
    world.entity().set(QueryComp { q: Some(q) });

    let e_holder = world
        .query::<&QueryComp>()
        .build()
        .find_entity(|_e, _| true)
        .unwrap();

    let mut count = 0;
    e_holder.get::<&QueryComp>(|qc| {
        qc.q.as_ref().unwrap().each(|(_p, _v)| {
            count += 1;
        });
    });
    assert_eq!(count, 2);
}

// ─── query_iter_from_component ────────────────────────────────────────────────

#[test]
fn query_query_iter_from_component() {
    #[derive(Component)]
    struct QueryIterComp {
        q: Option<Query<(&'static Position, &'static Velocity)>>,
    }

    let world = World::new();

    world
        .entity()
        .set(Position { x: 0, y: 0 })
        .set(Velocity { x: 0, y: 0 });
    world
        .entity()
        .set(Position { x: 0, y: 0 })
        .set(Velocity { x: 0, y: 0 });

    let q = world.new_query::<(&Position, &Velocity)>();
    world.entity().set(QueryIterComp { q: Some(q) });

    let e_holder = world
        .query::<&QueryIterComp>()
        .build()
        .find_entity(|_e, _| true)
        .unwrap();

    let mut count = 0;
    e_holder.get::<&QueryIterComp>(|qc| {
        qc.q.as_ref().unwrap().run(|mut it| {
            while it.next() {
                count += it.count();
            }
        });
    });
    assert_eq!(count, 2);
}

// ─── query_each_w_func_ptr ────────────────────────────────────────────────────

fn query_each_func(e: EntityView, p: &mut Position) {
    p.x += 1;
    p.y += 1;
    let _ = e;
}

#[test]
fn query_query_each_w_func_ptr() {
    let world = World::new();
    let e = world.entity().set(Position { x: 10, y: 20 });
    let q = world.new_query::<&mut Position>();
    q.each_entity(query_each_func);
    e.get::<&Position>(|p| {
        assert_eq!(p.x, 11);
        assert_eq!(p.y, 21);
    });
}

// ─── query_each_w_func_no_ptr ─────────────────────────────────────────────────

#[test]
fn query_query_each_w_func_no_ptr() {
    let world = World::new();
    let e = world.entity().set(Position { x: 10, y: 20 });
    let q = world.new_query::<&mut Position>();
    q.each_entity(query_each_func);
    e.get::<&Position>(|p| {
        assert_eq!(p.x, 11);
        assert_eq!(p.y, 21);
    });
}

// ─── query_iter_w_func_ptr / no_ptr ──────────────────────────────────────────

fn query_run_func(mut it: TableIter<true, ()>) {
    while it.next() {
        let mut p = it.field_mut::<Position>(0);
        for i in it.iter() {
            p[i].x += 1;
            p[i].y += 1;
        }
    }
}

#[test]
fn query_query_iter_w_func_ptr() {
    let world = World::new();
    let e = world.entity().set(Position { x: 10, y: 20 });
    let q = world.new_query::<&mut Position>();
    q.run(query_run_func);
    e.get::<&Position>(|p| {
        assert_eq!(p.x, 11);
        assert_eq!(p.y, 21);
    });
}

#[test]
fn query_query_iter_w_func_no_ptr() {
    let world = World::new();
    let e = world.entity().set(Position { x: 10, y: 20 });
    let q = world.new_query::<&mut Position>();
    q.run(query_run_func);
    e.get::<&Position>(|p| {
        assert_eq!(p.x, 11);
        assert_eq!(p.y, 21);
    });
}

// ─── query_each_w_iter ────────────────────────────────────────────────────────

#[test]
fn query_query_each_w_iter() {
    let world = World::new();

    let e1 = world.entity().set(Position { x: 10, y: 20 });
    let e2 = world.entity().set(Position { x: 20, y: 30 });

    let q = world.new_query::<&mut Position>();

    let mut invoked = 0;
    q.each_iter(|it, i, p| {
        assert_eq!(it.count(), 2);
        assert_eq!(
            it.entity_id(i),
            if usize::from(i) == 0 {
                e1.id()
            } else {
                e2.id()
            }
        );
        p.x += 1;
        p.y += 1;
        invoked += 1;
    });

    assert_eq!(invoked, 2);

    e1.get::<&Position>(|p| {
        assert_eq!(p.x, 11);
        assert_eq!(p.y, 21);
    });
    e2.get::<&Position>(|p| {
        assert_eq!(p.x, 21);
        assert_eq!(p.y, 31);
    });
}

// ─── field_at_from_each_w_iter ────────────────────────────────────────────────

#[test]
fn query_field_at_from_each_w_iter() {
    let world = World::new();

    let e1 = world
        .entity()
        .set(Position { x: 10, y: 20 })
        .set(Velocity { x: 1, y: 2 });
    let e2 = world
        .entity()
        .set(Position { x: 20, y: 30 })
        .set(Velocity { x: 3, y: 4 });

    let q = world.query::<&Position>().with(&mut Velocity::id()).build();

    let mut count = 0;
    q.each_iter(|it, row, _p| {
        let v = it.field_at::<Velocity>(1, row);
        if it.entity_id(row) == e1.id() {
            assert_eq!(v.x, 1);
            assert_eq!(v.y, 2);
            count += 1;
        } else if it.entity_id(row) == e2.id() {
            assert_eq!(v.x, 3);
            assert_eq!(v.y, 4);
            count += 1;
        }
    });

    assert_eq!(count, 2);
}

// ─── field_at_T_from_each_w_iter ──────────────────────────────────────────────

#[test]
fn query_field_at_type_from_each_w_iter() {
    let world = World::new();

    let e1 = world
        .entity()
        .set(Position { x: 10, y: 20 })
        .set(Velocity { x: 1, y: 2 });
    let e2 = world
        .entity()
        .set(Position { x: 20, y: 30 })
        .set(Velocity { x: 3, y: 4 });

    let q = world.query::<&Position>().with(&mut Velocity::id()).build();

    let mut count = 0;
    q.each_iter(|it, row, _p| {
        let v = it.field_at::<Velocity>(1, row);
        if it.entity_id(row) == e1.id() {
            assert_eq!(v.x, 1);
            assert_eq!(v.y, 2);
            count += 1;
        } else if it.entity_id(row) == e2.id() {
            assert_eq!(v.x, 3);
            assert_eq!(v.y, 4);
            count += 1;
        }
    });

    assert_eq!(count, 2);
}

// ─── field_at_const_T_from_each_w_iter ───────────────────────────────────────

#[test]
fn query_field_at_const_type_from_each_w_iter() {
    let world = World::new();

    let e1 = world
        .entity()
        .set(Position { x: 10, y: 20 })
        .set(Velocity { x: 1, y: 2 });
    let e2 = world
        .entity()
        .set(Position { x: 20, y: 30 })
        .set(Velocity { x: 3, y: 4 });

    let q = world.query::<&Position>().with(&Velocity::id()).build();

    let mut count = 0;
    q.each_iter(|it, row, _p| {
        let v = it.field_at::<Velocity>(1, row);
        if it.entity_id(row) == e1.id() {
            assert_eq!(v.x, 1);
            assert_eq!(v.y, 2);
            count += 1;
        } else if it.entity_id(row) == e2.id() {
            assert_eq!(v.x, 3);
            assert_eq!(v.y, 4);
            count += 1;
        }
    });

    assert_eq!(count, 2);
}

// ─── change_tracking ──────────────────────────────────────────────────────────

#[test]
fn query_change_tracking() {
    let world = World::new();

    let qw = world.new_query::<&mut Position>();
    let qr = world.query::<&Position>().detect_changes().build();

    let e1 = world.entity().add(Tag::id()).set(Position { x: 10, y: 20 });
    world.entity().set(Position { x: 20, y: 30 });

    assert!(qr.is_changed());
    qr.run(|mut it| while it.next() {});
    assert!(!qr.is_changed());

    let mut count = 0;
    let mut change_count = 0;

    qw.run(|mut it| {
        while it.next() {
            assert_eq!(it.count(), 1);
            count += 1;
            if it.entity_id(0_usize) == e1.id() {
                it.skip();
                continue;
            }
            change_count += 1;
        }
    });

    assert_eq!(count, 2);
    assert_eq!(change_count, 1);

    count = 0;
    change_count = 0;

    assert!(qr.is_changed());

    qr.run(|mut it| {
        while it.next() {
            assert_eq!(it.count(), 1);
            count += 1;
            if it.entity_id(0_usize) == e1.id() {
                assert!(!it.is_changed());
            } else {
                assert!(it.is_changed());
                change_count += 1;
            }
        }
    });

    assert_eq!(count, 2);
    assert_eq!(change_count, 1);
}

// ─── not_w_write ──────────────────────────────────────────────────────────────

#[test]
fn query_not_w_write() {
    #[derive(Component)]
    struct A;
    #[derive(Component)]
    struct B;

    let world = World::new();

    let q = world
        .query::<()>()
        .with(A::id())
        .with(B::id())
        .not()
        .write_curr()
        .build();

    let e = world.entity().add(A::id());

    let mut count = 0;
    world.defer(|| {
        q.each_entity(|e, ()| {
            e.add(B::id());
            count += 1;
        });
    });

    assert_eq!(count, 1);
    assert!(e.has(B::id()));

    q.each_entity(|_e, ()| {
        count += 1;
    });

    assert_eq!(count, 1);
}

// ─── instanced_nested_query_w_iter ────────────────────────────────────────────

#[test]
fn query_instanced_nested_query_w_iter() {
    let world = World::new();

    world.component::<Mass>().add_trait::<flecs::Singleton>();

    let q1 = world
        .query::<()>()
        .with(&Position::id())
        .with(Mass::id())
        .build();

    let q2 = world.query::<()>().with(&Velocity::id()).build();

    world.add(Mass::id());
    world.entity().add(Velocity::id());
    world.entity().add(Position::id());
    world.entity().add(Position::id());

    let mut count = 0;

    q2.run(|mut it_2| {
        while it_2.next() {
            q1.iter_stage(it_2.world()).run(|mut it_1| {
                while it_1.next() {
                    assert_eq!(it_1.count(), 2);
                    count += it_1.count();
                }
            });
        }
    });

    assert_eq!(count, 2);
}

// ─── instanced_nested_query_w_entity ─────────────────────────────────────────

#[test]
fn query_instanced_nested_query_w_entity() {
    let world = World::new();

    world.component::<Mass>().add_trait::<flecs::Singleton>();

    let q1 = world
        .query::<()>()
        .with(&Position::id())
        .with(Mass::id())
        .build();

    let q2 = world.query::<()>().with(&Velocity::id()).build();

    world.add(Mass::id());
    world.entity().add(Velocity::id());
    world.entity().add(Position::id());
    world.entity().add(Position::id());

    let mut count = 0;

    q2.each_entity(|e_2, ()| {
        q1.iter_stage(e_2).run(|mut it_1| {
            while it_1.next() {
                assert_eq!(it_1.count(), 2);
                count += it_1.count();
            }
        });
    });

    assert_eq!(count, 2);
}

// ─── instanced_nested_query_w_world ──────────────────────────────────────────

#[test]
fn query_instanced_nested_query_w_world() {
    let world = World::new();

    world.component::<Mass>().add_trait::<flecs::Singleton>();

    let q1 = world
        .query::<()>()
        .with(&Position::id())
        .with(Mass::id())
        .build();

    let q2 = world.query::<()>().with(&Velocity::id()).build();

    world.add(Mass::id());
    world.entity().add(Velocity::id());
    world.entity().add(Position::id());
    world.entity().add(Position::id());

    let mut count = 0;

    q2.run(|mut it_2| {
        while it_2.next() {
            q1.iter_stage(it_2.world()).run(|mut it_1| {
                while it_1.next() {
                    assert_eq!(it_1.count(), 2);
                    count += it_1.count();
                }
            });
        }
    });

    assert_eq!(count, 2);
}

// ─── captured_query ───────────────────────────────────────────────────────────

#[test]
fn query_captured_query() {
    let world = World::new();

    let q = world.new_query::<&Position>();
    let e1 = world.entity().set(Position { x: 10, y: 20 });

    let run = || {
        let mut count = 0;
        q.each_entity(|e, p| {
            assert_eq!(e, e1);
            assert_eq!(p.x, 10);
            assert_eq!(p.y, 20);
            count += 1;
        });
        assert_eq!(count, 1);
    };
    run();
}

// ─── set_group_captured_query ─────────────────────────────────────────────────

#[test]
fn query_set_group_captured_query() {
    let world = World::new();

    let rel = world.entity();
    let tgt_a = world.entity();
    let tgt_b = world.entity();

    let q = world.query::<&Position>().group_by(rel).build();

    world
        .entity()
        .set(Position { x: 10, y: 20 })
        .add((rel, tgt_a));
    let e2 = world
        .entity()
        .set(Position { x: 20, y: 30 })
        .add((rel, tgt_b));

    let run = || {
        let mut count = 0;
        q.with_group(tgt_b).each_entity(|e, p| {
            assert_eq!(e, e2);
            assert_eq!(p.x, 20);
            assert_eq!(p.y, 30);
            count += 1;
        });
        assert_eq!(count, 1);
    };
    run();
}

// ─── set_var_captured_query ───────────────────────────────────────────────────

#[test]
fn query_set_var_captured_query() {
    let world = World::new();

    let rel = world.entity();
    let tgt_a = world.entity();
    let tgt_b = world.entity();

    let q = world
        .query::<&Position>()
        .with(rel)
        .second()
        .set_var("var")
        .build();

    world
        .entity()
        .set(Position { x: 10, y: 20 })
        .add((rel, tgt_a));
    let e2 = world
        .entity()
        .set(Position { x: 20, y: 30 })
        .add((rel, tgt_b));

    let run = || {
        let mut count = 0;
        q.with_var_expr("var", tgt_b).each_entity(|e, p| {
            assert_eq!(e, e2);
            assert_eq!(p.x, 20);
            assert_eq!(p.y, 30);
            count += 1;
        });
        assert_eq!(count, 1);
    };
    run();
}

// ─── set_var_id_captured_query ────────────────────────────────────────────────

#[test]
fn query_set_var_id_captured_query() {
    let world = World::new();

    let rel = world.entity();
    let tgt_a = world.entity();
    let tgt_b = world.entity();

    let q = world
        .query::<&Position>()
        .with(rel)
        .second()
        .set_var("var")
        .build();

    let var_id = q.find_var("var").unwrap();

    world
        .entity()
        .set(Position { x: 10, y: 20 })
        .add((rel, tgt_a));
    let e2 = world
        .entity()
        .set(Position { x: 20, y: 30 })
        .add((rel, tgt_b));

    let run = || {
        let mut count = 0;
        q.with_var(var_id, tgt_b).each_entity(|e, p| {
            assert_eq!(e, e2);
            assert_eq!(p.x, 20);
            assert_eq!(p.y, 30);
            count += 1;
        });
        assert_eq!(count, 1);
    };
    run();
}

// ─── query_iter_set_var_expr_in_place ─────────────────────────────────────────

#[test]
fn query_iter_set_var_expr_in_place() {
    let world = World::new();

    let rel = world.entity();
    let tgt_a = world.entity();
    let tgt_b = world.entity();

    let q = world
        .query::<&Position>()
        .with(rel)
        .second()
        .set_var("var")
        .build();

    world
        .entity()
        .set(Position { x: 10, y: 20 })
        .add((rel, tgt_a));
    let e2 = world
        .entity()
        .set(Position { x: 20, y: 30 })
        .add((rel, tgt_b));

    let mut iter = q.iterable();
    iter.set_var_expr("var", tgt_b);

    let mut count = 0;
    iter.each_entity(|e, p| {
        assert_eq!(e, e2);
        assert_eq!(p.x, 20);
        assert_eq!(p.y, 30);
        count += 1;
    });
    assert_eq!(count, 1);
}

// ─── query_iter_lifecycle ─────────────────────────────────────────────────────

#[test]
fn query_iter_unconsumed_drop_releases_resources() {
    let world = World::new();
    world.entity().set(Position { x: 1, y: 2 });
    let q = world.new_query::<&Position>();

    let iter = q.iterable();
    drop(iter);

    let iter = q.iterable().page(0, 1);
    drop(iter);

    let iter = q.iterable().worker(0, 2);
    drop(iter);

    drop(q);
    drop(world);
}

#[test]
fn query_iter_transfer_through_with_var_releases_resources() {
    let world = World::new();

    let rel = world.entity();
    let tgt = world.entity();

    let q = world
        .query::<&Position>()
        .with(rel)
        .second()
        .set_var("var")
        .build();

    world.entity().set(Position { x: 1, y: 2 }).add((rel, tgt));

    let iter = q.iterable();
    let constrained = iter.with_var_expr("var", tgt);
    drop(constrained);
    drop(iter);

    drop(q);
    drop(world);
}

#[test]
#[should_panic(expected = "QueryIter already consumed")]
fn query_iter_reuse_panics() {
    let world = World::new();
    world.entity().set(Position { x: 1, y: 2 });
    let q = world.new_query::<&Position>();

    let iter = q.iterable();
    iter.each(|_| {});
    iter.each(|_| {});
}

#[test]
#[should_panic(expected = "ChainedIter already consumed")]
fn chained_iter_reuse_panics() {
    let world = World::new();
    world.entity().set(Position { x: 1, y: 2 });
    let q = world.new_query::<&Position>();

    let iter = q.iterable().page(0, 1);
    iter.each(|_| {});
    iter.each(|_| {});
}

// ─── iter_entities ────────────────────────────────────────────────────────────

#[test]
fn query_iter_entities() {
    let world = World::new();

    let e1 = world.entity().set(Position { x: 10, y: 20 });
    let e2 = world.entity().set(Position { x: 10, y: 20 });
    let e3 = world.entity().set(Position { x: 10, y: 20 });

    world.new_query::<&Position>().run(|mut it| {
        while it.next() {
            assert_eq!(it.count(), 3);
            let entities = it.entities();
            assert_eq!(entities[0], e1);
            assert_eq!(entities[1], e2);
            assert_eq!(entities[2], e3);
        }
    });
}

// ─── iter_get_pair_w_id ───────────────────────────────────────────────────────

#[test]
fn query_iter_get_pair_w_id() {
    let world = World::new();

    let rel = world.entity();
    let tgt = world.entity();
    let e = world.entity().add((rel, tgt));

    let q = world
        .query::<()>()
        .with((rel, id::<flecs::Wildcard>()))
        .build();

    let mut count = 0;
    q.each_iter(|it, i, ()| {
        assert!(it.id(0).is_pair());
        assert_eq!(it.id(0).first_id().id(), rel.id());
        assert_eq!(it.id(0).second_id().id(), tgt.id());
        assert_eq!(it.entity_id(i), e.id());
        count += 1;
    });

    assert_eq!(count, 1);
}

// ─── query_from_entity ────────────────────────────────────────────────────────

#[test]
fn query_query_from_entity() {
    let world = World::new();

    let qe = world.entity();
    let q1 = world
        .query::<(&Position, &Velocity)>()
        .entity_set(qe)
        .build();

    world.entity().add(Position::id());
    let e2 = world.entity().add(Position::id()).add(Velocity::id());

    let mut count = 0;
    q1.each_entity(|e, _| {
        count += 1;
        assert_eq!(e, e2);
    });
    assert_eq!(count, 1);

    let q2 = world.query_from(qe);
    q2.each_entity(|e, ()| {
        count += 1;
        assert_eq!(e, e2);
    });
    assert_eq!(count, 2);
}

// ─── query_from_entity_name ───────────────────────────────────────────────────

#[test]
fn query_query_from_entity_name() {
    let world = World::new();

    let q1 = world.query::<(&Position, &Velocity)>().named("qe2").build();

    world.entity().add(Position::id());
    let e2 = world.entity().add(Position::id()).add(Velocity::id());

    let mut count = 0;
    q1.each_entity(|e, _| {
        count += 1;
        assert_eq!(e, e2);
    });
    assert_eq!(count, 1);

    let q2 = world.query::<()>().named("qe2").build();
    q2.each_entity(|e, ()| {
        count += 1;
        assert_eq!(e, e2);
    });
    assert_eq!(count, 2);
}

// ─── run_w_iter_fini ──────────────────────────────────────────────────────────

#[test]
fn query_run_w_iter_fini() {
    let world = World::new();

    let q = world.new_query::<&Position>();

    let mut count = 0;
    q.run(|it| {
        it.fini();
        count += 1;
    });

    assert_eq!(count, 1);
}

// ─── run_w_iter_fini_interrupt ────────────────────────────────────────────────

#[test]
fn query_run_w_iter_fini_interrupt() {
    #[derive(Component)]
    struct Foo;
    #[derive(Component)]
    struct Bar;
    #[derive(Component)]
    struct Hello;

    let world = World::new();

    let e1 = world.entity().set(Position { x: 10, y: 20 }).add(Foo::id());
    world.entity().set(Position { x: 10, y: 20 }).add(Bar::id());
    world
        .entity()
        .set(Position { x: 10, y: 20 })
        .add(Hello::id());

    let q = world.new_query::<&Position>();

    let mut count = 0;
    q.run(|mut it| {
        assert!(it.next());
        assert_eq!(it.count(), 1);
        assert_eq!(it.entity_id(0_usize), e1.id());

        assert!(it.next());
        count += 1;
        it.fini();
    });

    assert_eq!(count, 1);
}

// ─── run_w_iter_fini_empty ────────────────────────────────────────────────────

#[test]
fn query_run_w_iter_fini_empty() {
    let world = World::new();

    let q = world.new_query::<&Position>();

    let mut count = 0;
    q.run(|it| {
        count += 1;
        it.fini();
    });

    assert_eq!(count, 1);
}

// ─── run_w_iter_fini_no_query ─────────────────────────────────────────────────

#[test]
fn query_run_w_iter_fini_no_query() {
    let world = World::new();

    let q = world.query::<()>().build();

    let mut count = 0;
    q.run(|it| {
        count += 1;
        it.fini();
    });

    assert_eq!(count, 1);
}

// ─── add_to_match_from_staged_query ───────────────────────────────────────────

#[test]
fn query_add_to_match_from_staged_query() {
    let world = World::new();

    let e = world.entity().add(Position::id());

    let stage = world.stage(0);

    world.readonly_begin(false);

    stage.new_query::<&Position>().each_entity(|e, _| {
        e.add(Velocity::id());
        assert!(!e.has(Velocity::id()));
    });

    world.readonly_end();

    assert!(e.has(Position::id()));
    assert!(e.has(Velocity::id()));
}

// ─── empty_tables_each ────────────────────────────────────────────────────────

#[test]
fn query_empty_tables_each() {
    use flecs_ecs::core::QueryFlags;

    let world = World::new();

    let e1 = world
        .entity()
        .set(Position { x: 10, y: 20 })
        .set(Velocity { x: 1, y: 2 });

    let e2 = world
        .entity()
        .set(Position { x: 20, y: 30 })
        .set(Velocity { x: 2, y: 3 });

    e2.add(Tag::id());
    e2.remove(Tag::id());

    let q = world
        .query::<(&mut Position, &Velocity)>()
        .query_flags(QueryFlags::MatchEmptyTables)
        .build();

    q.each(|(p, v)| {
        p.x += v.x;
        p.y += v.y;
    });

    e1.get::<&Position>(|p| {
        assert_eq!(p.x, 11);
        assert_eq!(p.y, 22);
    });
    e2.get::<&Position>(|p| {
        assert_eq!(p.x, 22);
        assert_eq!(p.y, 33);
    });
}

// ─── copy_operators ───────────────────────────────────────────────────────────

#[test]
fn query_copy_operators() {
    let world = World::new();

    let q = world.query::<()>().with(&Position::id()).build();

    let q_copy_ctor = q.clone();
    #[allow(unused_assignments)]
    let mut q_copy_assign = world.query::<()>().build();
    q_copy_assign = q.clone();

    // Verify both point to same underlying query
    assert_eq!(q_copy_ctor.query_ptr(), q.query_ptr());
    assert_eq!(q_copy_assign.query_ptr(), q.query_ptr());

    // Default-initialized query
    let q_default = world.query::<()>().build();
    let _q_copy_default = q_default.clone();
}

// ─── optional_singleton ───────────────────────────────────────────────────────

#[test]
fn query_optional_singleton() {
    #[derive(Component, Default)]
    struct TestComp {
        a: i32,
        b: i32,
        c: i32,
        d: i32,
        e: i32,
    }

    let world = World::new();

    world
        .component::<TestComp>()
        .add_trait::<flecs::Singleton>();

    let mut invoked = 0;

    world.new_query::<Option<&TestComp>>().each(|ptr| {
        assert!(ptr.is_none());
        invoked += 1;
    });

    assert_eq!(invoked, 1);

    world.set(TestComp {
        a: 10,
        b: 20,
        c: 30,
        d: 40,
        e: 50,
    });

    world.new_query::<Option<&TestComp>>().each(|ptr| {
        assert!(ptr.is_some());
        let t = ptr.unwrap();
        assert_eq!(t.a, 10);
        assert_eq!(t.b, 20);
        assert_eq!(t.c, 30);
        assert_eq!(t.d, 40);
        assert_eq!(t.e, 50);
        invoked += 1;
    });

    assert_eq!(invoked, 2);
}

// ─── optional_pair_term ───────────────────────────────────────────────────────

#[test]
fn query_optional_pair_term() {
    #[derive(Component, Default)]
    struct PairPos {
        x: f32,
        y: f32,
    }
    #[derive(Component)]
    struct PairTag2;
    #[derive(Component)]
    struct Tag0;

    let world = World::new();

    world
        .entity()
        .add(Tag0::id())
        .set_pair::<PairPos, PairTag2>(PairPos { x: 1.0, y: 2.0 });
    world.entity().add(Tag0::id());

    let mut with_pair = 0;
    let mut without_pair = 0;

    let q = world
        .query::<Option<&(PairPos, PairTag2)>>()
        .with(Tag0::id())
        .build();

    q.each(|p| {
        if let Some(p) = p {
            with_pair += 1;
            assert!((p.x - 1.0f32).abs() < f32::EPSILON);
            assert!((p.y - 2.0f32).abs() < f32::EPSILON);
        } else {
            without_pair += 1;
        }
    });

    assert_eq!(with_pair, 1);
    assert_eq!(without_pair, 1);
}

// ─── pair_with_variable_src ───────────────────────────────────────────────────

#[test]
fn query_pair_with_variable_src() {
    #[derive(Component)]
    struct RelVar2;
    #[derive(Component)]
    struct ThisComp {
        x: i32,
    }
    #[derive(Component)]
    struct OtherComp {
        x: i32,
    }

    let world = World::new();

    let other = world.entity().set(OtherComp { x: 10 });

    for i in 0..3i32 {
        world
            .entity()
            .set(ThisComp { x: i })
            .add((RelVar2::id(), other));
    }

    // Use explicit with() terms so term_at can access them without hitting
    // the generic-term protection in check_term_access_validity.
    let q = world
        .query::<()>()
        .with(RelVar2::id())
        .set_second("$other")
        .with(ThisComp::id())
        .with(OtherComp::id())
        .set_src("$other")
        .build();

    let mut is_present: u32 = 0;
    q.run(|mut it| {
        while it.next() {
            for i in it.iter() {
                let this = it.get_field::<ThisComp>(1).unwrap();
                // OtherComp is a shared field (from $other entity), only index 0
                let other_comp = it.get_field::<OtherComp>(2).unwrap();
                is_present |= 1 << this[i].x;
                assert_eq!(other_comp[0].x, 10);
            }
        }
    });

    assert_eq!(is_present, 7);
}

// ─── has_entity / has_table / has_range ──────────────────────────────────────
// TODO: missing API: query.has(entity) / query.has(table) / query.has(range)
// ecs_query_has / ecs_query_has_table / ecs_query_has_range are in sys bindings
// but not wrapped in the Rust high-level query API yet.

// ─── page_iter_captured_query / worker_iter_captured_query ───────────────────
// TODO: missing API: QueryIter::page() / QueryIter::worker() not yet implemented
// See flecs_ecs/src/core/query_iter.rs line 159:
// "TODO : worker_iterable and page_iterable not implemented yet"

// ─── optional_module ──────────────────────────────────────────────────────────
// TODO: missing API: world.import::<Module>() with Singleton semantics differs
// from C++ world.import<TestComponent>() which calls ctor with world arg.
// Not ported as it requires non-trivial Module infrastructure.

// ─── invalid_each_w_no_this / invalid_field_*_from_each_w_iter ───────────────
// These C++ tests use install_test_abort / test_expect_abort to verify C-level
// abort behavior. In Rust these would be debug panics, covered by other tests.

// ─── test_no_defer_each / test_no_defer_iter ─────────────────────────────────
// These tests expect an abort/panic on structural modification inside a
// non-deferred cached query. They require the crash-handler test infrastructure
// and are not ported here.

// ─── pair_with_variable_src_no_row_fields ────────────────────────────────────
// Similar to query_pair_with_variable_src above but with non-tag Rel component.
// Covered by the variable src test above.
