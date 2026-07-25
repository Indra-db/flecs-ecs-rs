use super::*;

mod wildcard_into_id {
    use super::*;

    mod entity_view {
        use super::*;

        #[test]
        fn sparse_wildcard_read_concrete_write_panics() {
            let world = World::new();
            world.component::<Foo>().add_trait::<flecs::Sparse>();
            let bar_id = world.component::<Bar>().id();
            let entity = world.entity().set_first(Foo(0), bar_id);

            let result = std::panic::catch_unwind(core::panic::AssertUnwindSafe(|| {
                entity.get::<&(Foo, flecs::Wildcard)>(|_| {
                    entity.get::<&mut (Foo, Bar)>(|_| {});
                });
            }));

            assert!(result.is_err());
        }

        #[test]
        fn read_read() {
            let world = World::new();
            let bar_id = world.component::<Bar>().id();
            let entity = world.entity().set_first(Foo(0), bar_id);
            entity.get::<&(Foo, flecs::Wildcard)>(|_| {
                entity.get::<&(Foo, Bar)>(|_| {});
            });
        }

        #[test]
        #[should_panic]
        fn read_write() {
            let world = World::new();
            let bar_id = world.component::<Bar>().id();
            let entity = world.entity().set_first(Foo(0), bar_id);
            entity.get::<&(Foo, flecs::Wildcard)>(|_| {
                entity.get::<&mut (Foo, Bar)>(|_| {});
            });
        }

        #[test]
        #[should_panic]
        fn write_read() {
            let world = World::new();
            let bar_id = world.component::<Bar>().id();
            let entity = world.entity().set_first(Foo(0), bar_id);
            entity.get::<&mut (Foo, flecs::Wildcard)>(|_| {
                entity.get::<&(Foo, Bar)>(|_| {});
            });
        }

        #[test]
        #[should_panic]
        fn write_cloned() {
            let world = World::new();
            let bar_id = world.component::<Bar>().id();
            let entity = world.entity().set_first(Foo(0), bar_id);
            entity.get::<&mut (Foo, flecs::Wildcard)>(|_| {
                let _ = entity.cloned::<&(Foo, Bar)>();
            });
        }

        #[test]
        #[should_panic]
        fn write_write() {
            let world = World::new();
            let bar_id = world.component::<Bar>().id();
            let entity = world.entity().set_first(Foo(0), bar_id);
            entity.get::<&mut (Foo, flecs::Wildcard)>(|_| {
                entity.get::<&mut (Foo, Bar)>(|_| {});
            });
        }
    }

    mod query_in_query {
        use super::*;

        mod read_read {
            use super::*;

            #[test]
            fn run() {
                let world = World::new();
                let bar_id = world.component::<Bar>().id();
                world.entity().set_first(Foo(0), bar_id);
                let q0 = query!(world, &(Foo, flecs::Wildcard)).build();
                let q1 = query!(world, &(Foo, Bar)).build();
                q0.run(|iter| {
                    q1.run(|iter| {
                        iter.fini();
                    });
                    iter.fini();
                });
            }

            #[test]
            fn each() {
                let world = World::new();
                let bar_id = world.component::<Bar>().id();
                world.entity().set_first(Foo(0), bar_id);
                let q0 = query!(world, &(Foo, flecs::Wildcard)).build();
                let q1 = query!(world, &(Foo, Bar)).build();
                q0.each(|_| {
                    q1.each(|_| {});
                });
            }

            #[test]
            fn each_entity() {
                let world = World::new();
                let bar_id = world.component::<Bar>().id();
                world.entity().set_first(Foo(0), bar_id);
                let q0 = query!(world, &(Foo, flecs::Wildcard)).build();
                let q1 = query!(world, &(Foo, Bar)).build();
                q0.each_entity(|_, _| {
                    q1.each_entity(|_, _| {});
                });
            }

            #[test]
            fn each_iter() {
                let world = World::new();
                let bar_id = world.component::<Bar>().id();
                world.entity().set_first(Foo(0), bar_id);
                let q0 = query!(world, &(Foo, flecs::Wildcard)).build();
                let q1 = query!(world, &(Foo, Bar)).build();
                q0.each_iter(|_, _, _| {
                    q1.each_iter(|_, _, _| {});
                });
            }
        }

        mod read_write {
            use super::*;

            #[test]
            #[should_panic]
            fn run() {
                let world = World::new();
                let bar_id = world.component::<Bar>().id();
                world.entity().set_first(Foo(0), bar_id);
                let q0 = query!(world, &(Foo, flecs::Wildcard)).build();
                let q1 = query!(world, &mut (Foo, Bar)).build();
                q0.run(|mut iter| {
                    while iter.next() {
                        let _x = iter.field::<Foo>(0);
                        q1.run(|mut iter| {
                            while iter.next() {
                                let _y = iter.field_mut::<Foo>(0);
                            }
                        });
                    }
                });
            }

            #[test]
            #[should_panic]
            fn each() {
                let world = World::new();
                let bar_id = world.component::<Bar>().id();
                world.entity().set_first(Foo(0), bar_id);
                let q0 = query!(world, &(Foo, flecs::Wildcard)).build();
                let q1 = query!(world, &mut (Foo, Bar)).build();
                q0.each(|_| {
                    q1.each(|_| {});
                });
            }

            #[test]
            #[should_panic]
            fn each_entity() {
                let world = World::new();
                let bar_id = world.component::<Bar>().id();
                world.entity().set_first(Foo(0), bar_id);
                let q0 = query!(world, &(Foo, flecs::Wildcard)).build();
                let q1 = query!(world, &mut (Foo, Bar)).build();
                q0.each_entity(|_, _| {
                    q1.each_entity(|_, _| {});
                });
            }

            #[test]
            #[should_panic]
            fn each_iter() {
                let world = World::new();
                let bar_id = world.component::<Bar>().id();
                world.entity().set_first(Foo(0), bar_id);
                let q0 = query!(world, &(Foo, flecs::Wildcard)).build();
                let q1 = query!(world, &mut (Foo, Bar)).build();
                q0.each_iter(|_, _, _| {
                    q1.each_iter(|_, _, _| {});
                });
            }
        }

        mod write_read {
            use super::*;

            #[test]
            #[should_panic]
            fn run() {
                let world = World::new();
                let bar_id = world.component::<Bar>().id();
                world.entity().set_first(Foo(0), bar_id);
                let q0 = query!(world, &mut (Foo, flecs::Wildcard)).build();
                let q1 = query!(world, &(Foo, Bar)).build();
                q0.run(|mut iter| {
                    while iter.next() {
                        let _x = iter.field_mut::<Foo>(0);
                        q1.run(|mut iter| {
                            while iter.next() {
                                let _y = iter.field::<Foo>(0);
                            }
                        });
                    }
                });
            }

            #[test]
            #[should_panic]
            fn each() {
                let world = World::new();
                let bar_id = world.component::<Bar>().id();
                world.entity().set_first(Foo(0), bar_id);
                let q0 = query!(world, &mut (Foo, flecs::Wildcard)).build();
                let q1 = query!(world, &(Foo, Bar)).build();
                q0.each(|_| {
                    q1.each(|_| {});
                });
            }

            #[test]
            #[should_panic]
            fn each_entity() {
                let world = World::new();
                let bar_id = world.component::<Bar>().id();
                world.entity().set_first(Foo(0), bar_id);
                let q0 = query!(world, &mut (Foo, flecs::Wildcard)).build();
                let q1 = query!(world, &(Foo, Bar)).build();
                q0.each_entity(|_, _| {
                    q1.each_entity(|_, _| {});
                });
            }

            #[test]
            #[should_panic]
            fn each_iter() {
                let world = World::new();
                let bar_id = world.component::<Bar>().id();
                world.entity().set_first(Foo(0), bar_id);
                let q0 = query!(world, &mut (Foo, flecs::Wildcard)).build();
                let q1 = query!(world, &(Foo, Bar)).build();
                q0.each_iter(|_, _, _| {
                    q1.each_iter(|_, _, _| {});
                });
            }
        }

        mod write_write {
            use super::*;

            #[test]
            #[should_panic]
            fn run() {
                let world = World::new();
                let bar_id = world.component::<Bar>().id();
                world.entity().set_first(Foo(0), bar_id);
                let q0 = query!(world, &mut (Foo, flecs::Wildcard)).build();
                let q1 = query!(world, &mut (Foo, Bar)).build();
                q0.run(|mut iter| {
                    while iter.next() {
                        let _x = iter.field_mut::<Foo>(0);
                        q1.run(|mut iter| {
                            while iter.next() {
                                let _y = iter.field_mut::<Foo>(0);
                            }
                        });
                    }
                });
            }

            #[test]
            #[should_panic]
            fn each() {
                let world = World::new();
                let bar_id = world.component::<Bar>().id();
                world.entity().set_first(Foo(0), bar_id);
                let q0 = query!(world, &mut (Foo, flecs::Wildcard)).build();
                let q1 = query!(world, &mut (Foo, Bar)).build();
                q0.each(|_| {
                    q1.each(|_| {});
                });
            }

            #[test]
            #[should_panic]
            fn each_entity() {
                let world = World::new();
                let bar_id = world.component::<Bar>().id();
                world.entity().set_first(Foo(0), bar_id);
                let q0 = query!(world, &mut (Foo, flecs::Wildcard)).build();
                let q1 = query!(world, &mut (Foo, Bar)).build();
                q0.each_entity(|_, _| {
                    q1.each_entity(|_, _| {});
                });
            }

            #[test]
            #[should_panic]
            fn each_iter() {
                let world = World::new();
                let bar_id = world.component::<Bar>().id();
                world.entity().set_first(Foo(0), bar_id);
                let q0 = query!(world, &mut (Foo, flecs::Wildcard)).build();
                let q1 = query!(world, &mut (Foo, Bar)).build();
                q0.each_iter(|_, _, _| {
                    q1.each_iter(|_, _, _| {});
                });
            }
        }
    }

    mod query_entity_view {
        use super::*;

        #[test]
        fn read_read() {
            let world = World::new();
            let bar_id = world.component::<Bar>().id();
            world.entity().set_first(Foo(0), bar_id);
            query!(world, &(Foo, flecs::Wildcard))
                .build()
                .each_entity(|entity, _| {
                    entity.get::<&(Foo, Bar)>(|_| {});
                });
        }

        #[test]
        #[should_panic]
        fn read_write() {
            let world = World::new();
            let bar_id = world.component::<Bar>().id();
            world.entity().set_first(Foo(0), bar_id);
            query!(world, &(Foo, flecs::Wildcard))
                .build()
                .each_entity(|entity, _| {
                    entity.get::<&mut (Foo, Bar)>(|_| {});
                });
        }

        #[test]
        #[should_panic]
        fn write_read() {
            let world = World::new();
            let bar_id = world.component::<Bar>().id();
            world.entity().set_first(Foo(0), bar_id);
            query!(world, &mut (Foo, flecs::Wildcard))
                .build()
                .each_entity(|entity, _| {
                    entity.get::<&(Foo, Bar)>(|_| {});
                });
        }

        #[test]
        #[should_panic]
        fn write_write() {
            let world = World::new();
            let bar_id = world.component::<Bar>().id();
            world.entity().set_first(Foo(0), bar_id);
            query!(world, &mut (Foo, flecs::Wildcard))
                .build()
                .each_entity(|entity, _| {
                    entity.get::<&mut (Foo, Bar)>(|_| {});
                });
        }
    }
}

mod id_into_wildcard {
    use super::*;

    mod entity_view {
        use super::*;

        fn read_read() {
            let world = World::new();
            let bar_id = world.component::<Bar>().id();
            let entity = world.entity().set_first(Foo(0), bar_id);
            entity.get::<&(Foo, Bar)>(|_| {
                entity.get::<&(Foo, flecs::Wildcard)>(|_| {});
            });
        }

        fn read_write() {
            let world = World::new();
            let bar_id = world.component::<Bar>().id();
            let entity = world.entity().set_first(Foo(0), bar_id);
            entity.get::<&(Foo, Bar)>(|_| {
                entity.get::<&mut (Foo, flecs::Wildcard)>(|_| {});
            });
        }

        fn write_read() {
            let world = World::new();
            let bar_id = world.component::<Bar>().id();
            let entity = world.entity().set_first(Foo(0), bar_id);
            entity.get::<&mut (Foo, Bar)>(|_| {
                entity.get::<&(Foo, flecs::Wildcard)>(|_| {});
            });
        }

        fn write_cloned() {
            let world = World::new();
            let bar_id = world.component::<Bar>().id();
            let entity = world.entity().set_first(Foo(0), bar_id);
            entity.get::<&mut (Foo, Bar)>(|_| {
                let _ = entity.cloned::<&(Foo, flecs::Wildcard)>();
            });
        }

        fn write_write() {
            let world = World::new();
            let bar_id = world.component::<Bar>().id();
            let entity = world.entity().set_first(Foo(0), bar_id);
            entity.get::<&mut (Foo, Bar)>(|_| {
                entity.get::<&mut (Foo, flecs::Wildcard)>(|_| {});
            });
        }
    }

    mod query_in_query {
        use super::*;

        mod read_read {
            use super::*;

            #[test]
            fn run() {
                let world = World::new();
                let bar_id = world.component::<Bar>().id();
                world.entity().set_first(Foo(0), bar_id);
                let q0 = query!(world, &(Foo, Bar)).build();
                let q1 = query!(world, &(Foo, flecs::Wildcard)).build();
                q0.run(|mut iter| {
                    while iter.next() {
                        let _x = iter.field::<Foo>(0);
                        q1.run(|mut iter| {
                            while iter.next() {
                                let _y = iter.field::<Foo>(0);
                            }
                        });
                    }
                });
            }

            #[test]
            fn each() {
                let world = World::new();
                let bar_id = world.component::<Bar>().id();
                world.entity().set_first(Foo(0), bar_id);
                let q0 = query!(world, &(Foo, Bar)).build();
                let q1 = query!(world, &(Foo, flecs::Wildcard)).build();
                q0.each(|_| {
                    q1.each(|_| {});
                });
            }

            #[test]
            fn each_entity() {
                let world = World::new();
                let bar_id = world.component::<Bar>().id();
                world.entity().set_first(Foo(0), bar_id);
                let q0 = query!(world, &(Foo, Bar)).build();
                let q1 = query!(world, &(Foo, flecs::Wildcard)).build();
                q0.each_entity(|_, _| {
                    q1.each_entity(|_, _| {});
                });
            }

            #[test]
            fn each_iter() {
                let world = World::new();
                let bar_id = world.component::<Bar>().id();
                world.entity().set_first(Foo(0), bar_id);
                let q0 = query!(world, &(Foo, Bar)).build();
                let q1 = query!(world, &(Foo, flecs::Wildcard)).build();
                q0.each_iter(|_, _, _| {
                    q1.each_iter(|_, _, _| {});
                });
            }
        }

        mod read_write {
            use super::*;

            #[test]
            #[should_panic]
            fn run() {
                let world = World::new();
                let bar_id = world.component::<Bar>().id();
                world.entity().set_first(Foo(0), bar_id);
                let q0 = query!(world, &(Foo, Bar)).build();
                let q1 = query!(world, &mut (Foo, flecs::Wildcard)).build();
                q0.run(|mut iter| {
                    while iter.next() {
                        let _x = iter.field::<Foo>(0);
                        q1.run(|mut iter| {
                            while iter.next() {
                                let _y = iter.field_mut::<Foo>(0);
                            }
                        });
                    }
                });
            }

            #[test]
            #[should_panic]
            fn each() {
                let world = World::new();
                let bar_id = world.component::<Bar>().id();
                world.entity().set_first(Foo(0), bar_id);
                let q0 = query!(world, &(Foo, Bar)).build();
                let q1 = query!(world, &mut (Foo, flecs::Wildcard)).build();
                q0.each(|_| {
                    q1.each(|_| {});
                });
            }

            #[test]
            #[should_panic]
            fn each_entity() {
                let world = World::new();
                let bar_id = world.component::<Bar>().id();
                world.entity().set_first(Foo(0), bar_id);
                let q0 = query!(world, &(Foo, Bar)).build();
                let q1 = query!(world, &mut (Foo, flecs::Wildcard)).build();
                q0.each_entity(|_, _| {
                    q1.each_entity(|_, _| {});
                });
            }

            #[test]
            #[should_panic]
            fn each_iter() {
                let world = World::new();
                let bar_id = world.component::<Bar>().id();
                world.entity().set_first(Foo(0), bar_id);
                let q0 = query!(world, &(Foo, Bar)).build();
                let q1 = query!(world, &mut (Foo, flecs::Wildcard)).build();
                q0.each_iter(|_, _, _| {
                    q1.each_iter(|_, _, _| {});
                });
            }
        }

        mod write_read {
            use super::*;

            #[test]
            #[should_panic]
            fn run() {
                let world = World::new();
                let bar_id = world.component::<Bar>().id();
                world.entity().set_first(Foo(0), bar_id);
                let q0 = query!(world, &mut (Foo, Bar)).build();
                let q1 = query!(world, &(Foo, flecs::Wildcard)).build();
                q0.run(|mut iter| {
                    while iter.next() {
                        let _x = iter.field_mut::<Foo>(0);
                        q1.run(|mut iter| {
                            while iter.next() {
                                let _y = iter.field::<Foo>(0);
                            }
                        });
                    }
                });
            }

            #[test]
            #[should_panic]
            fn each() {
                let world = World::new();
                let bar_id = world.component::<Bar>().id();
                world.entity().set_first(Foo(0), bar_id);
                let q0 = query!(world, &mut (Foo, Bar)).build();
                let q1 = query!(world, &(Foo, flecs::Wildcard)).build();
                q0.each(|_| {
                    q1.each(|_| {});
                });
            }

            #[test]
            #[should_panic]
            fn each_entity() {
                let world = World::new();
                let bar_id = world.component::<Bar>().id();
                world.entity().set_first(Foo(0), bar_id);
                let q0 = query!(world, &mut (Foo, Bar)).build();
                let q1 = query!(world, &(Foo, flecs::Wildcard)).build();
                q0.each_entity(|_, _| {
                    q1.each_entity(|_, _| {});
                });
            }

            #[test]
            #[should_panic]
            fn each_iter() {
                let world = World::new();
                let bar_id = world.component::<Bar>().id();
                world.entity().set_first(Foo(0), bar_id);
                let q0 = query!(world, &mut (Foo, Bar)).build();
                let q1 = query!(world, &(Foo, flecs::Wildcard)).build();
                q0.each_iter(|_, _, _| {
                    q1.each_iter(|_, _, _| {});
                });
            }
        }

        mod write_write {
            use super::*;

            #[test]
            #[should_panic]
            fn run() {
                let world = World::new();
                let bar_id = world.component::<Bar>().id();
                world.entity().set_first(Foo(0), bar_id);
                let q0 = query!(world, &mut (Foo, Bar)).build();
                let q1 = query!(world, &mut (Foo, flecs::Wildcard)).build();
                q0.run(|mut iter| {
                    while iter.next() {
                        let _x = iter.field_mut::<Foo>(0);
                        q1.run(|mut iter| {
                            while iter.next() {
                                let _y = iter.field_mut::<Foo>(0);
                            }
                        });
                    }
                });
            }

            #[test]
            #[should_panic]
            fn each() {
                let world = World::new();
                let bar_id = world.component::<Bar>().id();
                world.entity().set_first(Foo(0), bar_id);
                let q0 = query!(world, &mut (Foo, Bar)).build();
                let q1 = query!(world, &mut (Foo, flecs::Wildcard)).build();
                q0.each(|_| {
                    q1.each(|_| {});
                });
            }

            #[test]
            #[should_panic]
            fn each_entity() {
                let world = World::new();
                let bar_id = world.component::<Bar>().id();
                world.entity().set_first(Foo(0), bar_id);
                let q0 = query!(world, &mut (Foo, Bar)).build();
                let q1 = query!(world, &mut (Foo, flecs::Wildcard)).build();
                q0.each_entity(|_, _| {
                    q1.each_entity(|_, _| {});
                });
            }

            #[test]
            #[should_panic]
            fn each_iter() {
                let world = World::new();
                let bar_id = world.component::<Bar>().id();
                world.entity().set_first(Foo(0), bar_id);
                let q0 = query!(world, &mut (Foo, Bar)).build();
                let q1 = query!(world, &mut (Foo, flecs::Wildcard)).build();
                q0.each_iter(|_, _, _| {
                    q1.each_iter(|_, _, _| {});
                });
            }
        }
    }

    mod query_entity_view {
        use super::*;

        #[test]
        fn read_read() {
            let world = World::new();
            let bar_id = world.component::<Bar>().id();
            world.entity().set_first(Foo(0), bar_id);
            query!(world, &(Foo, Bar)).build().each_entity(|entity, _| {
                entity.get::<&(Foo, flecs::Wildcard)>(|_| {});
            });
        }

        #[test]
        #[should_panic]
        fn read_write() {
            let world = World::new();
            let bar_id = world.component::<Bar>().id();
            world.entity().set_first(Foo(0), bar_id);
            query!(world, &(Foo, Bar)).build().each_entity(|entity, _| {
                entity.get::<&mut (Foo, flecs::Wildcard)>(|_| {});
            });
        }

        #[test]
        #[should_panic]
        fn write_read() {
            let world = World::new();
            let bar_id = world.component::<Bar>().id();
            world.entity().set_first(Foo(0), bar_id);
            query!(world, &mut (Foo, Bar))
                .build()
                .each_entity(|entity, _| {
                    entity.get::<&(Foo, flecs::Wildcard)>(|_| {});
                });
        }

        #[test]
        #[should_panic]
        fn write_write() {
            let world = World::new();
            let bar_id = world.component::<Bar>().id();
            world.entity().set_first(Foo(0), bar_id);
            query!(world, &mut (Foo, Bar))
                .build()
                .each_entity(|entity, _| {
                    entity.get::<&mut (Foo, flecs::Wildcard)>(|_| {});
                });
        }
    }
}
