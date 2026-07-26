#![allow(dead_code)]
use flecs_ecs::core::*;
use flecs_ecs::macros::*;
use flecs_ecs::experimental::QuerySharedExt;

mod batch_locks;
mod cached_ref;
mod get_scope;
mod multithreaded;
mod pairs;
mod scheduler_threadid;
mod panic_cleanup;

#[derive(Clone, Component)]
struct Foo(u8);

#[derive(Component)]
struct Bar;

#[derive(Component)]
struct EventA;

#[derive(Component)]
struct EventB;

#[test]
fn building_conflicting_queries_no_violations() {
    let world = World::new();
    // building queries that overlap without running them is ok
    let _read = query!(world, &Foo).build();
    let _write0 = query!(world, &mut Foo).build();
    let _write1 = query!(world, &mut Foo).build();
}

#[test]
fn running_conflicting_queries_no_violations() {
    let world = World::new();
    let read = query!(world, &Foo).build();
    let write0 = query!(world, &mut Foo).build();
    let write1 = query!(world, &mut Foo).build();

    // running queries that overlap individually is ok
    read.run_shared(&world, |iter| {
        iter.fini();
    });
    write0.run_shared(&world, |iter| {
        iter.fini();
    });
    write1.run_shared(&world, |iter| {
        iter.fini();
    });

    read.each_shared(&world, |_| {});
    write0.each_shared(&world, |_| {});
    write1.each_shared(&world, |_| {});

    read.each_entity_shared(&world, |_, _| {});
    write0.each_entity_shared(&world, |_, _| {});
    write1.each_entity_shared(&world, |_, _| {});

    read.each_iter(|_, _, _| {});
    write0.each_iter(|_, _, _| {});
    write1.each_iter(|_, _, _| {});
}

mod entity_view {
    use super::*;

    mod get {
        use super::*;
        #[test]
        #[should_panic]
        fn read_write() {
            let world = World::new();
            let entity = world.entity().set(Foo(0));
            entity.get::<&Foo>(|_| {
                entity.get::<&mut Foo>(|_| {});
            });
        }

        #[test]
        #[should_panic]
        fn write_read() {
            let world = World::new();
            let entity = world.entity().set(Foo(0));
            entity.get::<&mut Foo>(|_| {
                entity.get::<&Foo>(|_| {});
            });
        }

        #[test]
        #[should_panic]
        fn write_cloned() {
            let world = World::new();
            let entity = world.entity().set(Foo(0));
            entity.get::<&mut Foo>(|_| {
                let _ = entity.cloned::<&Foo>();
            });
        }

        #[test]
        #[should_panic]
        fn write_write() {
            let world = World::new();
            let entity = world.entity().set(Foo(0));
            entity.get::<&mut Foo>(|_| {
                entity.get::<&mut Foo>(|_| {});
            });
        }
    }

    mod try_get {
        use super::*;
        #[test]
        #[should_panic]
        fn read_write() {
            let world = World::new();
            let entity = world.entity().set(Foo(0));
            entity.try_get::<&Foo>(|_| {
                entity.try_get::<&mut Foo>(|_| {});
            });
        }

        #[test]
        #[should_panic]
        fn write_read() {
            let world = World::new();
            let entity = world.entity().set(Foo(0));
            entity.try_get::<&mut Foo>(|_| {
                entity.try_get::<&Foo>(|_| {});
            });
        }

        #[test]
        #[should_panic]
        fn write_cloned() {
            let world = World::new();
            let entity = world.entity().set(Foo(0));
            entity.try_get::<&mut Foo>(|_| {
                let _ = entity.cloned::<&Foo>();
            });
        }

        #[test]
        #[should_panic]
        fn write_write() {
            let world = World::new();
            let entity = world.entity().set(Foo(0));
            entity.try_get::<&mut Foo>(|_| {
                entity.try_get::<&mut Foo>(|_| {});
            });
        }
    }

    mod from_query {
        use super::*;

        #[test]
        #[should_panic]
        fn query_write_view_clone() {
            let world = World::new();
            world.entity().set(Foo(0));
            query!(world, &mut Foo).build().each_entity_shared(&world, |entity, _| {
                let _ = entity.cloned::<&Foo>();
            });
        }

        mod get {
            use super::*;

            #[test]
            #[should_panic]
            fn query_read_view_write() {
                let world = World::new();
                world.entity().set(Foo(0));
                query!(world, &Foo).build().each_entity_shared(&world, |entity, _| {
                    entity.get::<&mut Foo>(|_| {});
                });
            }

            #[test]
            #[should_panic]
            fn query_write_view_read() {
                let world = World::new();
                world.entity().set(Foo(0));
                query!(world, &mut Foo).build().each_entity_shared(&world, |entity, _| {
                    entity.get::<&Foo>(|_| {});
                });
            }

            #[test]
            #[should_panic]
            fn query_write_view_write() {
                let world = World::new();
                world.entity().set(Foo(0));
                query!(world, &mut Foo).build().each_entity_shared(&world, |entity, _| {
                    entity.get::<&mut Foo>(|_| {});
                });
            }
        }

        mod try_get {
            use super::*;

            #[test]
            #[should_panic]
            fn query_read_view_write() {
                let world = World::new();
                world.entity().set(Foo(0));
                query!(world, &Foo).build().each_entity_shared(&world, |entity, _| {
                    entity.try_get::<&mut Foo>(|_| {});
                });
            }

            #[test]
            #[should_panic]
            fn query_write_view_read() {
                let world = World::new();
                world.entity().set(Foo(0));
                query!(world, &mut Foo).build().each_entity_shared(&world, |entity, _| {
                    entity.try_get::<&Foo>(|_| {});
                });
            }

            #[test]
            #[should_panic]
            fn query_write_view_write() {
                let world = World::new();
                world.entity().set(Foo(0));
                query!(world, &mut Foo).build().each_entity_shared(&world, |entity, _| {
                    entity.try_get::<&mut Foo>(|_| {});
                });
            }
        }
    }

    mod from_system {
        use super::*;

        #[test]
        #[should_panic]
        fn system_write_view_clone() {
            let mut world = World::new();
            world.entity().set(Foo(0));
            system!(world, &mut Foo).each_entity(|entity, _| {
                let _ = entity.cloned::<&Foo>();
            });
            world.progress();
        }

        mod get {
            use super::*;

            #[test]
            #[should_panic]
            fn system_read_view_write() {
                let mut world = World::new();
                world.entity().set(Foo(0));
                system!(world, &Foo).each_entity(|entity, _| {
                    entity.get::<&mut Foo>(|_| {});
                });
                world.progress();
            }

            #[test]
            #[should_panic]
            fn system_write_view_read() {
                let mut world = World::new();
                world.entity().set(Foo(0));
                system!(world, &mut Foo).each_entity(|entity, _| {
                    entity.get::<&Foo>(|_| {});
                });
                world.progress();
            }

            #[test]
            #[should_panic]
            fn system_write_view_write() {
                let mut world = World::new();
                world.entity().set(Foo(0));
                system!(world, &mut Foo).each_entity(|entity, _| {
                    entity.get::<&mut Foo>(|_| {});
                });
                world.progress();
            }
        }

        mod try_get {
            use super::*;

            #[test]
            #[should_panic]
            fn system_read_view_write() {
                let mut world = World::new();
                world.entity().set(Foo(0));
                system!(world, &Foo).each_entity(|entity, _| {
                    entity.try_get::<&mut Foo>(|_| {});
                });
                world.progress();
            }

            #[test]
            #[should_panic]
            fn system_write_view_read() {
                let mut world = World::new();
                world.entity().set(Foo(0));
                system!(world, &mut Foo).each_entity(|entity, _| {
                    entity.try_get::<&Foo>(|_| {});
                });
                world.progress();
            }

            #[test]
            #[should_panic]
            fn system_write_view_write() {
                let mut world = World::new();
                world.entity().set(Foo(0));
                system!(world, &mut Foo).each_entity(|entity, _| {
                    entity.try_get::<&mut Foo>(|_| {});
                });
                world.progress();
            }
        }
    }

    mod from_observer {
        use super::*;

        #[test]
        #[should_panic]
        fn observer_write_view_clone() {
            let world = World::new();
            observer!(world, flecs::OnSet, &mut Foo).each_entity(|entity, _| {
                let _ = entity.cloned::<&Foo>();
            });
            world.entity().set(Foo(0));
        }

        mod get {
            use super::*;

            #[test]
            #[should_panic]
            fn observer_read_view_write() {
                let world = World::new();
                observer!(world, flecs::OnSet, &Foo).each_entity(|entity, _| {
                    entity.get::<&mut Foo>(|_| {});
                });
                world.entity().set(Foo(0));
            }

            #[test]
            #[should_panic]
            fn observer_write_view_read() {
                let world = World::new();
                observer!(world, flecs::OnSet, &mut Foo).each_entity(|entity, _| {
                    entity.get::<&Foo>(|_| {});
                });
                world.entity().set(Foo(0));
            }

            #[test]
            #[should_panic]
            fn observer_write_view_write() {
                let world = World::new();
                observer!(world, flecs::OnSet, &mut Foo).each_entity(|entity, _| {
                    entity.get::<&mut Foo>(|_| {});
                });
                world.entity().set(Foo(0));
            }
        }

        mod try_get {
            use super::*;

            #[test]
            #[should_panic]
            fn observer_read_view_write() {
                let world = World::new();
                observer!(world, flecs::OnSet, &Foo).each_entity(|entity, _| {
                    entity.try_get::<&mut Foo>(|_| {});
                });
                world.entity().set(Foo(0));
            }

            #[test]
            #[should_panic]
            fn observer_write_view_read() {
                let world = World::new();
                observer!(world, flecs::OnSet, &mut Foo).each_entity(|entity, _| {
                    entity.try_get::<&Foo>(|_| {});
                });
                world.entity().set(Foo(0));
            }

            #[test]
            #[should_panic]
            fn observer_write_view_write() {
                let world = World::new();
                observer!(world, flecs::OnSet, &mut Foo).each_entity(|entity, _| {
                    entity.try_get::<&mut Foo>(|_| {});
                });
                world.entity().set(Foo(0));
            }
        }
    }
}

mod table_iter {
    use super::*;

    mod field {
        use super::*;

        #[test]
        fn field() {
            let world = World::new();
            world.entity().set(Foo(0));
            query!(world, Foo).build().run_shared(&world, |mut iter| {
                while iter.next() {
                    let _ = iter.field::<Foo>(0);
                }
            });
        }

        #[test]
        #[should_panic]
        fn double_field() {
            let world = World::new();
            world.entity().set(Foo(0));
            query!(world, Foo).build().run_shared(&world, |mut iter| {
                while iter.next() {
                    let _x = iter.field_mut::<Foo>(0);
                    let _y = iter.field_mut::<Foo>(0);
                }
            });
        }

        #[test]
        #[should_panic]
        fn query_read_field() {
            let world = World::new();
            world.entity().set(Foo(0));
            query!(world, &Foo).build().run_shared(&world, |mut iter| {
                while iter.next() {
                    let _x = iter.field::<Foo>(0);
                    let _y = iter.field_mut::<Foo>(0);
                }
            });
        }

        #[test]
        fn query_write_field() {
            let world = World::new();
            world.entity().set(Foo(0));
            query!(world, &mut Foo).build().run_shared(&world, |mut iter| {
                while iter.next() {
                    let _ = iter.field_mut::<Foo>(0);
                }
            });
        }
    }

    mod field_at {
        use super::*;

        #[test]
        fn filter_sparse() {
            let world = World::new();

            world.component::<Foo>().add_trait::<flecs::Sparse>();
            world.entity().set(Foo(0));
            query!(world, Foo).build().run_shared(&world, |mut iter| {
                while iter.next() {
                    for _ in iter.iter() {
                        iter.field_at::<Foo>(0, 0usize);
                    }
                }
            });
        }

        #[test]
        fn filter_non_fragmenting() {
            let world = World::new();

            world.component::<Foo>().add_trait::<flecs::DontFragment>();
            world.entity().set(Foo(0));
            query!(world, Foo).build().run_shared(&world, |mut iter| {
                while iter.next() {
                    for _ in iter.iter() {
                        iter.field_at::<Foo>(0, 0usize);
                    }
                }
            });
        }

        #[test]
        fn query_read_sparse() {
            let world = World::new();
            world.component::<Foo>().add_trait::<flecs::Sparse>();
            world.entity().set(Foo(0));
            query!(world, &Foo).build().run_shared(&world, |mut iter| {
                while iter.next() {
                    for _ in iter.iter() {
                        let _x1 = iter.field_at::<Foo>(0, 0usize);
                        let _x2 = iter.field_at::<Foo>(0, 0usize);
                    }
                }
            });
        }

        #[test]
        fn query_read_non_fragmenting() {
            let world = World::new();
            world.component::<Foo>().add_trait::<flecs::DontFragment>();
            world.entity().set(Foo(0));
            query!(world, &Foo).build().run_shared(&world, |mut iter| {
                while iter.next() {
                    for _ in iter.iter() {
                        let _x1 = iter.field_at::<Foo>(0, 0usize);
                        let _x2 = iter.field_at::<Foo>(0, 0usize);
                    }
                }
            });
        }

        #[test]
        #[should_panic]
        fn query_read_write_sparse() {
            let world = World::new();
            world.component::<Foo>().add_trait::<flecs::Sparse>();
            world.entity().set(Foo(0));
            query!(world, &mut Foo).build().run_shared(&world, |mut iter| {
                while iter.next() {
                    for _ in iter.iter() {
                        let _x = iter.field_at::<Foo>(0, 0usize);
                        let _y = iter.field_at_mut::<Foo>(0, 0usize);
                    }
                }
            });
        }

        #[test]
        #[should_panic]
        fn query_read_write_non_fragmenting() {
            let world = World::new();
            world.component::<Foo>().add_trait::<flecs::DontFragment>();
            world.entity().set(Foo(0));
            query!(world, &mut Foo).build().run_shared(&world, |mut iter| {
                while iter.next() {
                    for _ in iter.iter() {
                        let _x = iter.field_at::<Foo>(0, 0usize);
                        let _y = iter.field_at_mut::<Foo>(0, 0usize);
                    }
                }
            });
        }
    }

    mod field_at_mut {
        use super::*;

        #[test]
        fn filter_sparse() {
            let world = World::new();
            world.component::<Foo>().add_trait::<flecs::Sparse>();
            world.entity().set(Foo(0));
            query!(world, Foo).build().run_shared(&world, |mut iter| {
                while iter.next() {
                    for _ in iter.iter() {
                        iter.field_at_mut::<Foo>(0, 0usize);
                    }
                }
            });
        }

        #[test]
        fn filter_non_fragmenting() {
            let world = World::new();
            world.component::<Foo>().add_trait::<flecs::DontFragment>();
            world.entity().set(Foo(0));
            query!(world, Foo).build().run_shared(&world, |mut iter| {
                while iter.next() {
                    for _ in iter.iter() {
                        iter.field_at_mut::<Foo>(0, 0usize);
                    }
                }
            });
        }

        #[test]
        #[should_panic]
        fn filter_double_field_at_mut_sparse() {
            let world = World::new();
            world.component::<Foo>().add_trait::<flecs::Sparse>();
            world.entity().set(Foo(0));
            query!(world, Foo).build().run_shared(&world, |mut iter| {
                while iter.next() {
                    for _ in iter.iter() {
                        let _x = iter.field_at_mut::<Foo>(0, 0usize);
                        let _y = iter.field_at_mut::<Foo>(0, 0usize);
                    }
                }
            });
        }

        #[test]
        #[should_panic]
        fn filter_double_field_at_mut_non_fragmenting() {
            let world = World::new();
            world.component::<Foo>().add_trait::<flecs::DontFragment>();
            world.entity().set(Foo(0));
            query!(world, Foo).build().run_shared(&world, |mut iter| {
                while iter.next() {
                    for _ in iter.iter() {
                        let _x = iter.field_at_mut::<Foo>(0, 0usize);
                        let _y = iter.field_at_mut::<Foo>(0, 0usize);
                    }
                }
            });
        }

        #[test]
        #[should_panic]
        fn query_read_sparse() {
            let world = World::new();
            world.component::<Foo>().add_trait::<flecs::Sparse>();
            world.entity().set(Foo(0));
            query!(world, &Foo).build().run_shared(&world, |mut iter| {
                while iter.next() {
                    for _ in iter.iter() {
                        let _x = iter.field_at_mut::<Foo>(0, 0usize);
                        let _y = iter.field_at_mut::<Foo>(0, 0usize);
                    }
                }
            });
        }

        #[test]
        #[should_panic]
        fn query_read_non_fragmenting() {
            let world = World::new();
            world.component::<Foo>().add_trait::<flecs::DontFragment>();
            world.entity().set(Foo(0));
            query!(world, &Foo).build().run_shared(&world, |mut iter| {
                while iter.next() {
                    for _ in iter.iter() {
                        let _x = iter.field_at_mut::<Foo>(0, 0usize);
                        let _y = iter.field_at_mut::<Foo>(0, 0usize);
                    }
                }
            });
        }

        #[test]
        #[should_panic]
        fn query_read_write_sparse() {
            let world = World::new();
            world.component::<Foo>().add_trait::<flecs::Sparse>();
            world.entity().set(Foo(0));
            query!(world, &mut Foo).build().run_shared(&world, |mut iter| {
                while iter.next() {
                    for _ in iter.iter() {
                        let _x = iter.field_at::<Foo>(0, 0usize);
                        let _y = iter.field_at_mut::<Foo>(0, 0usize);
                    }
                }
            });
        }

        #[test]
        #[should_panic]
        fn query_read_write_non_fragmenting() {
            let world = World::new();
            world.component::<Foo>().add_trait::<flecs::DontFragment>();
            world.entity().set(Foo(0));
            query!(world, &mut Foo).build().run_shared(&world, |mut iter| {
                while iter.next() {
                    for _ in iter.iter() {
                        let _x = iter.field_at::<Foo>(0, 0usize);
                        let _y = iter.field_at_mut::<Foo>(0, 0usize);
                    }
                }
            });
        }
    }
}

mod query_in_query {
    use super::*;

    mod read_write {
        use super::*;

        #[test]
        fn run_no_fields_ok() {
            let world = World::new();
            world.entity().set(Foo(0));
            let query0 = query!(world, &Foo).build();
            let query1 = query!(world, &mut Foo).build();
            query0.run_shared(&world, |iter| {
                query1.run_shared(&world, |iter| {
                    iter.fini();
                });
                iter.fini();
            });
        }

        #[test]
        #[should_panic]
        fn each_violation() {
            let world = World::new();
            world.entity().set(Foo(0));
            let query0 = query!(world, &Foo).build();
            let query1 = query!(world, &mut Foo).build();
            query0.each_shared(&world, |_| {
                query1.each_shared(&world, |_| {});
            });
        }

        #[test]
        #[should_panic]
        fn each_entity_violation() {
            let world = World::new();
            world.entity().set(Foo(0));
            let query0 = query!(world, &Foo).build();
            let query1 = query!(world, &mut Foo).build();
            query0.each_entity_shared(&world, |_, _| {
                query1.each_entity_shared(&world, |_, _| {});
            });
        }

        #[test]
        #[should_panic]
        fn each_iter_query_violation() {
            let world = World::new();
            world.entity().set(Foo(0));
            let query0 = query!(world, &Foo).build();
            let query1 = query!(world, &mut Foo).build();
            query0.each_iter(|_, _, _| {
                query1.each_iter(|_, _, _| {});
            });
        }
    }

    mod write_read {
        use super::*;

        #[test]
        fn run_no_fields_ok() {
            let world = World::new();
            world.entity().set(Foo(0));
            let query0 = query!(world, &mut Foo).build();
            let query1 = query!(world, &Foo).build();
            query0.run_shared(&world, |iter| {
                query1.run_shared(&world, |iter| {
                    iter.fini();
                });
                iter.fini();
            });
        }

        #[test]
        #[should_panic]
        fn each_violation() {
            let world = World::new();
            world.entity().set(Foo(0));
            let query0 = query!(world, &mut Foo).build();
            let query1 = query!(world, &Foo).build();
            query0.each_shared(&world, |_| {
                query1.each_shared(&world, |_| {});
            });
        }

        #[test]
        #[should_panic]
        fn each_entity_violation() {
            let world = World::new();
            world.entity().set(Foo(0));
            let query0 = query!(world, &mut Foo).build();
            let query1 = query!(world, &Foo).build();
            query0.each_entity_shared(&world, |_, _| {
                query1.each_entity_shared(&world, |_, _| {});
            });
        }

        #[test]
        #[should_panic]
        fn each_iter_violation() {
            let world = World::new();
            world.entity().set(Foo(0));
            let query0 = query!(world, &mut Foo).build();
            let query1 = query!(world, &Foo).build();
            query0.each_iter(|_, _, _| {
                query1.each_iter(|_, _, _| {});
            });
        }
    }

    mod write_write {
        use super::*;

        #[test]
        fn run_no_fields_ok() {
            let world = World::new();
            world.entity().set(Foo(0));
            let query0 = query!(world, &mut Foo).build();
            let query1 = query!(world, &mut Foo).build();
            query0.run_shared(&world, |iter| {
                query1.run_shared(&world, |iter| {
                    iter.fini();
                });
                iter.fini();
            });
        }

        #[test]
        #[should_panic]
        fn each_violation() {
            let world = World::new();
            world.entity().set(Foo(0));
            let query0 = query!(world, &mut Foo).build();
            let query1 = query!(world, &mut Foo).build();
            query0.each_shared(&world, |_| {
                query1.each_shared(&world, |_| {});
            });
        }

        #[test]
        #[should_panic]
        fn each_entity_violation() {
            let world = World::new();
            world.entity().set(Foo(0));
            let query0 = query!(world, &mut Foo).build();
            let query1 = query!(world, &mut Foo).build();
            query0.each_entity_shared(&world, |_, _| {
                query1.each_entity_shared(&world, |_, _| {});
            });
        }

        #[test]
        #[should_panic]
        fn each_iter_violation() {
            let world = World::new();
            world.entity().set(Foo(0));
            let query0 = query!(world, &mut Foo).build();
            let query1 = query!(world, &mut Foo).build();
            query0.each_iter(|_, _, _| {
                query1.each_iter(|_, _, _| {});
            });
        }
    }
}

mod observer_in_observer {
    use super::*;

    mod read_write {
        use super::*;

        #[test]
        fn run_no_fields_ok() {
            let world = World::new();
            let e = world.entity().set(Foo(0)).id();
            observer!(world, EventA, &Foo).run(|iter| {
                iter.fini();
            });
            observer!(world, EventB, &mut Foo).run(move |iter| {
                iter.world()
                    .event()
                    .add(id::<Foo>())
                    .entity(e)
                    .emit(&EventA);
                iter.fini();
            });
            world.event().add(id::<Foo>()).entity(e).emit(&EventB);
        }

        #[test]
        #[should_panic]
        fn each_entity_violation() {
            let world = World::new();
            let e = world.entity().set(Foo(0)).id();
            observer!(world, EventA, &Foo).each_entity(|_, _| {});
            observer!(world, EventB, &mut Foo).each_entity(move |entity, _| {
                entity
                    .world()
                    .event()
                    .add(id::<Foo>())
                    .entity(e)
                    .emit(&EventA);
            });
            world.event().add(id::<Foo>()).entity(e).emit(&EventB);
        }

        #[test]
        #[should_panic]
        fn each_iter_violation() {
            let world = World::new();
            let e = world.entity().set(Foo(0)).id();
            observer!(world, EventA, &Foo).each_iter(|_, _, _| {});
            observer!(world, EventB, &mut Foo).each_iter(move |iter, _, _| {
                iter.world()
                    .event()
                    .add(id::<Foo>())
                    .entity(e)
                    .emit(&EventA);
            });
            world.event().add(id::<Foo>()).entity(e).emit(&EventB);
        }
    }

    mod write_read {
        use super::*;

        #[test]
        fn run_no_fields_ok() {
            let world = World::new();
            let e = world.entity().set(Foo(0)).id();
            observer!(world, EventA, &mut Foo).run(|iter| {
                iter.fini();
            });
            observer!(world, EventB, &Foo).run(move |iter| {
                iter.world()
                    .event()
                    .add(id::<Foo>())
                    .entity(e)
                    .emit(&EventA);
                iter.fini();
            });
            world.event().add(id::<Foo>()).entity(e).emit(&EventB);
        }

        #[test]
        #[should_panic]
        fn each_entity_violation() {
            let world = World::new();
            let e = world.entity().set(Foo(0)).id();
            observer!(world, EventA, &mut Foo).each_entity(|_, _| {});
            observer!(world, EventB, &Foo).each_entity(move |entity, _| {
                entity
                    .world()
                    .event()
                    .add(id::<Foo>())
                    .entity(e)
                    .emit(&EventA);
            });
            world.event().add(id::<Foo>()).entity(e).emit(&EventB);
        }

        #[test]
        #[should_panic]
        fn each_iter_violation() {
            let world = World::new();
            let e = world.entity().set(Foo(0)).id();
            observer!(world, EventA, &mut Foo).each_iter(|_, _, _| {});
            observer!(world, EventB, &Foo).each_iter(move |iter, _, _| {
                iter.world()
                    .event()
                    .add(id::<Foo>())
                    .entity(e)
                    .emit(&EventA);
            });
            world.event().add(id::<Foo>()).entity(e).emit(&EventB);
        }
    }

    mod write_write {
        use super::*;

        #[test]
        fn run_no_fields_ok() {
            let world = World::new();
            let e = world.entity().set(Foo(0)).id();
            observer!(world, EventA, &mut Foo).run(|iter| {
                iter.fini();
            });
            observer!(world, EventB, &mut Foo).run(move |iter| {
                iter.world()
                    .event()
                    .add(id::<Foo>())
                    .entity(e)
                    .emit(&EventA);
                iter.fini();
            });
            world.event().add(id::<Foo>()).entity(e).emit(&EventB);
        }

        #[test]
        #[should_panic]
        fn each_entity_violation() {
            let world = World::new();
            let e = world.entity().set(Foo(0)).id();
            observer!(world, EventA, &mut Foo).each_entity(|_, _| {});
            observer!(world, EventB, &mut Foo).each_entity(move |entity, _| {
                entity
                    .world()
                    .event()
                    .add(id::<Foo>())
                    .entity(e)
                    .emit(&EventA);
            });
            world.event().add(id::<Foo>()).entity(e).emit(&EventB);
        }

        #[test]
        #[should_panic]
        fn each_iter_violation() {
            let world = World::new();
            let e = world.entity().set(Foo(0)).id();
            observer!(world, EventA, &mut Foo).each_iter(|_, _, _| {});
            observer!(world, EventB, &mut Foo).each_iter(move |iter, _, _| {
                iter.world()
                    .event()
                    .add(id::<Foo>())
                    .entity(e)
                    .emit(&EventA);
            });
            world.event().add(id::<Foo>()).entity(e).emit(&EventB);
        }
    }
}

mod query_in_observer {
    use super::*;

    mod read_write {
        use super::*;

        #[test]
        fn run_no_fields_ok() {
            let world = World::new();
            let query = query!(world, &mut Foo).build();
            observer!(world, flecs::OnSet, &Foo).run(move |iter| {
                query.run(|iter| {
                    iter.fini();
                });
                iter.fini();
            });
            world.entity().set(Foo(0));
        }

        #[test]
        #[should_panic]
        fn each_violation() {
            let world = World::new();
            let query = query!(world, &mut Foo).build();
            observer!(world, flecs::OnSet, &Foo).each(move |_| {
                query.each(|_| {});
            });
            world.entity().set(Foo(0));
        }

        #[test]
        #[should_panic]
        fn each_entity_violation() {
            let world = World::new();
            let query = query!(world, &mut Foo).build();
            observer!(world, flecs::OnSet, &Foo).each_entity(move |_, _| {
                query.each_entity(|_, _| {});
            });
            world.entity().set(Foo(0));
        }

        #[test]
        #[should_panic]
        fn each_iter_violation() {
            let world = World::new();
            let query = query!(world, &mut Foo).build();
            observer!(world, flecs::OnSet, &Foo).each_iter(move |_, _, _| {
                query.each_iter(|_, _, _| {});
            });
            world.entity().set(Foo(0));
        }
    }

    mod write_read {
        use super::*;

        #[test]
        fn run_no_fields_ok() {
            let world = World::new();
            let query = query!(world, &Foo).build();
            observer!(world, flecs::OnSet, &mut Foo).run(move |iter| {
                query.run(|iter| {
                    iter.fini();
                });
                iter.fini();
            });
            world.entity().set(Foo(0));
        }

        #[test]
        #[should_panic]
        fn each_violation() {
            let world = World::new();
            let query = query!(world, &Foo).build();
            observer!(world, flecs::OnSet, &mut Foo).each(move |_| {
                query.each(|_| {});
            });
            world.entity().set(Foo(0));
        }

        #[test]
        #[should_panic]
        fn each_entity_violation() {
            let world = World::new();
            let query = query!(world, &Foo).build();
            observer!(world, flecs::OnSet, &mut Foo).each_entity(move |_, _| {
                query.each_entity(|_, _| {});
            });
            world.entity().set(Foo(0));
        }

        #[test]
        #[should_panic]
        fn each_iter_violation() {
            let world = World::new();
            let query = query!(world, &Foo).build();
            observer!(world, flecs::OnSet, &mut Foo).each_iter(move |_, _, _| {
                query.each_iter(|_, _, _| {});
            });
            world.entity().set(Foo(0));
        }
    }

    mod write_write {
        use super::*;

        #[test]
        fn run_no_fields_ok() {
            let world = World::new();
            let query = query!(world, &mut Foo).build();
            observer!(world, flecs::OnSet, &mut Foo).run(move |iter| {
                query.run(|iter| {
                    iter.fini();
                });
                iter.fini();
            });
            world.entity().set(Foo(0));
        }

        #[test]
        #[should_panic]
        fn each_violation() {
            let world = World::new();
            let query = query!(world, &mut Foo).build();
            observer!(world, flecs::OnSet, &mut Foo).each(move |_| {
                query.each(|_| {});
            });
            world.entity().set(Foo(0));
        }

        #[test]
        #[should_panic]
        fn each_entity_violation() {
            let world = World::new();
            let query = query!(world, &mut Foo).build();
            observer!(world, flecs::OnSet, &mut Foo).each_entity(move |_, _| {
                query.each_entity(|_, _| {});
            });
            world.entity().set(Foo(0));
        }

        #[test]
        #[should_panic]
        fn each_iter_violation() {
            let world = World::new();
            let query = query!(world, &mut Foo).build();
            observer!(world, flecs::OnSet, &mut Foo).each_iter(move |_, _, _| {
                query.each_iter(|_, _, _| {});
            });
            world.entity().set(Foo(0));
        }
    }
}

mod query_in_system {
    use super::*;

    mod read_write {
        use super::*;

        #[test]
        fn run_no_fields_ok() {
            let mut world = World::new();
            world.entity().set(Foo(0));
            let query = query!(world, &mut Foo).build();
            system!(world, &Foo).run(move |iter| {
                query.run(|iter| {
                    iter.fini();
                });
                iter.fini();
            });
            world.progress();
        }

        #[test]
        #[should_panic]
        fn each_violation() {
            let mut world = World::new();
            world.entity().set(Foo(0));
            let query = query!(world, &mut Foo).build();
            system!(world, &Foo).each(move |_| {
                query.each(|_| {});
            });
            world.progress();
        }

        #[test]
        #[should_panic]
        fn each_entity_violation() {
            let mut world = World::new();
            world.entity().set(Foo(0));
            let query = query!(world, &mut Foo).build();
            system!(world, &Foo).each_entity(move |_, _| {
                query.each_entity(|_, _| {});
            });
            world.progress();
        }

        #[test]
        #[should_panic]
        fn each_iter_violation() {
            let mut world = World::new();
            world.entity().set(Foo(0));
            let query = query!(world, &mut Foo).build();
            system!(world, &Foo).each_iter(move |_, _, _| {
                query.each_iter(|_, _, _| {});
            });
            world.progress();
        }
    }

    mod write_read {
        use super::*;

        #[test]
        fn run_no_fields_ok() {
            let mut world = World::new();
            world.entity().set(Foo(0));
            let query = query!(world, &Foo).build();
            system!(world, &mut Foo).run(move |iter| {
                query.run(|iter| {
                    iter.fini();
                });
                iter.fini();
            });
            world.progress();
        }

        #[test]
        #[should_panic]
        fn each_violation() {
            let mut world = World::new();
            world.entity().set(Foo(0));
            let query = query!(world, &Foo).build();
            system!(world, &mut Foo).each(move |_| {
                query.each(|_| {});
            });
            world.progress();
        }

        #[test]
        #[should_panic]
        fn each_entity_violation() {
            let mut world = World::new();
            world.entity().set(Foo(0));
            let query = query!(world, &Foo).build();
            system!(world, &mut Foo).each_entity(move |_, _| {
                query.each_entity(|_, _| {});
            });
            world.progress();
        }

        #[test]
        #[should_panic]
        fn each_iter_violation() {
            let mut world = World::new();
            world.entity().set(Foo(0));
            let query = query!(world, &Foo).build();
            system!(world, &mut Foo).each_iter(move |_, _, _| {
                query.each_iter(|_, _, _| {});
            });
            world.progress();
        }
    }

    mod write_write {
        use super::*;

        #[test]
        fn run_no_fields_ok() {
            let mut world = World::new();
            world.entity().set(Foo(0));
            let query = query!(world, &mut Foo).build();
            system!(world, &mut Foo).run(move |iter| {
                query.run(|iter| {
                    iter.fini();
                });
                iter.fini();
            });
            world.progress();
        }

        #[test]
        #[should_panic]
        fn each_violation() {
            let mut world = World::new();
            world.entity().set(Foo(0));
            let query = query!(world, &mut Foo).build();
            system!(world, &mut Foo).each(move |_| {
                query.each(|_| {});
            });
            world.progress();
        }

        #[test]
        #[should_panic]
        fn each_entity_violation() {
            let mut world = World::new();
            world.entity().set(Foo(0));
            let query = query!(world, &mut Foo).build();
            system!(world, &mut Foo).each_entity(move |_, _| {
                query.each_entity(|_, _| {});
            });
            world.progress();
        }

        #[test]
        #[should_panic]
        fn each_iter_violation() {
            let mut world = World::new();
            world.entity().set(Foo(0));
            let query = query!(world, &mut Foo).build();
            system!(world, &mut Foo).each_iter(move |_, _, _| {
                query.each_iter(|_, _, _| {});
            });
            world.progress();
        }
    }
}

mod observer_in_system {
    use super::*;

    mod read_write {
        use super::*;

        #[test]
        fn run_no_fields_ok() {
            let mut world = World::new();
            world.entity().set(Foo(0));
            observer!(world, EventA, &mut Foo).each(|_| {});
            system!(world, &Foo).run(move |mut iter| {
                iter.next();
                iter.entity(0usize).emit(&EventA);
                iter.fini();
            });
            world.progress();
        }

        #[test]
        #[should_panic]
        fn each_entity_violation() {
            let mut world = World::new();
            world.entity().set(Foo(0));
            observer!(world, EventA, &mut Foo).each(|_| {});
            system!(world, &Foo)
                .immediate(true)
                .each_entity(move |entity, _| {
                    let world = entity.world();
                    world.defer_suspend();
                    world.event().add(id::<Foo>()).entity(entity).emit(&EventA);
                    world.defer_resume();
                });
            world.progress();
        }

        #[test]
        #[should_panic]
        fn each_iter_violation() {
            let mut world = World::new();
            world.entity().set(Foo(0));
            observer!(world, EventA, &mut Foo).each(|_| {});
            system!(world, &Foo)
                .immediate(true)
                .each_iter(move |iter, _, _| {
                    let world = iter.world();
                    world.defer_suspend();
                    world
                        .event()
                        .add(id::<Foo>())
                        .entity(iter.entity(0usize))
                        .emit(&EventA);
                    world.defer_resume();
                });
            world.progress();
        }
    }

    mod write_read {
        use super::*;

        #[test]
        fn run_no_fields_ok() {
            let mut world = World::new();
            world.entity().set(Foo(0));
            observer!(world, EventA, &Foo).each(|_| {});
            system!(world, &mut Foo)
                .immediate(true)
                .run(move |mut iter| {
                    iter.next();
                    iter.entity(0usize).emit(&EventA);
                    iter.fini();
                });
            world.progress();
        }

        #[test]
        #[should_panic]
        fn each_entity_violation() {
            let mut world = World::new();
            world.entity().set(Foo(0));
            observer!(world, EventA, &Foo).each(|_| {});
            system!(world, &mut Foo)
                .immediate(true)
                .each_entity(move |entity, _| {
                    let world = entity.world();
                    world.defer_suspend();
                    world.event().add(id::<Foo>()).entity(entity).emit(&EventA);
                    world.defer_resume();
                });
            world.progress();
        }

        #[test]
        #[should_panic]
        fn each_iter_violation() {
            let mut world = World::new();
            world.entity().set(Foo(0));
            observer!(world, EventA, &Foo).each(|_| {});
            system!(world, &mut Foo)
                .immediate(true)
                .each_iter(move |iter, _, _| {
                    let world = iter.world();
                    world.defer_suspend();
                    world
                        .event()
                        .add(id::<Foo>())
                        .entity(iter.entity(0usize))
                        .emit(&EventA);
                    world.defer_resume();
                });
            world.progress();
        }
    }

    mod write_write {
        use super::*;

        #[test]
        fn run_no_fields_ok() {
            let mut world = World::new();
            world.entity().set(Foo(0));
            observer!(world, EventA, &mut Foo).each(|_| {});
            system!(world, &mut Foo).run(move |mut iter| {
                iter.next();
                iter.entity(0usize).emit(&EventA);
                iter.fini();
            });
            world.progress();
        }

        #[test]
        #[should_panic]
        fn each_entity_violation() {
            let mut world = World::new();
            world.entity().set(Foo(0));
            observer!(world, EventA, &mut Foo).each(|_| {});
            system!(world, &mut Foo)
                .immediate(true)
                .each_entity(move |entity, _| {
                    let world = entity.world();
                    world.defer_suspend();
                    world.event().add(id::<Foo>()).entity(entity).emit(&EventA);
                    world.defer_resume();
                });
            world.progress();
        }

        #[test]
        #[should_panic]
        fn each_iter_violation() {
            let mut world = World::new();
            world.entity().set(Foo(0));
            observer!(world, EventA, &mut Foo).each(|_| {});
            system!(world, &mut Foo)
                .immediate(true)
                .each_iter(move |iter, _, _| {
                    let world = iter.world();
                    world.defer_suspend();
                    world
                        .event()
                        .add(id::<Foo>())
                        .entity(iter.entity(0usize))
                        .emit(&EventA);
                    world.defer_resume();
                });
            world.progress();
        }
    }
}

#[test]
fn filter_does_not_panic() {
    let world = World::new();

    #[derive(Component, Clone)]
    struct Foo(pub i32);

    world.entity().set(Foo(0));

    query!(world, Foo).build().each_entity_shared(&world, |entity, _| {
        let _ = entity.cloned::<&Foo>();
    });
}

mod dense_field_at_locks {
    use super::*;

    #[test]
    fn filter_dense_read_read() {
        let world = World::new();
        world.entity().set(Foo(0));
        query!(world, Foo).build().run_shared(&world, |mut iter| {
            while iter.next() {
                for _ in iter.iter() {
                    let _x1 = iter.field_at::<Foo>(0, 0usize);
                    let _x2 = iter.field_at::<Foo>(0, 0usize);
                }
            }
        });
    }

    #[test]
    #[should_panic]
    fn filter_double_field_at_mut_dense() {
        let world = World::new();
        world.entity().set(Foo(0));
        query!(world, Foo).build().run_shared(&world, |mut iter| {
            while iter.next() {
                for _ in iter.iter() {
                    let _x = iter.field_at_mut::<Foo>(0, 0usize);
                    let _y = iter.field_at_mut::<Foo>(0, 0usize);
                }
            }
        });
    }

    #[test]
    #[should_panic]
    fn filter_field_at_read_write_dense() {
        let world = World::new();
        world.entity().set(Foo(0));
        query!(world, Foo).build().run_shared(&world, |mut iter| {
            while iter.next() {
                for _ in iter.iter() {
                    let _x = iter.field_at::<Foo>(0, 0usize);
                    let _y = iter.field_at_mut::<Foo>(0, 0usize);
                }
            }
        });
    }

    #[test]
    #[should_panic]
    fn each_iter_field_at_mut_aliases_tuple_dense() {
        let world = World::new();
        world.entity().set(Foo(0));
        query!(world, &mut Foo)
            .build()
            .each_iter(|iter, index, _foo| {
                let _aliased = iter.field_at_mut::<Foo>(0, index);
            });
    }

    #[test]
    #[should_panic]
    fn each_iter_field_mut_aliases_tuple_dense() {
        let world = World::new();
        world.entity().set(Foo(0));
        query!(world, &mut Foo)
            .build()
            .each_iter(|iter, _index, _foo| {
                let _aliased = iter.field_mut::<Foo>(0);
            });
    }
}
