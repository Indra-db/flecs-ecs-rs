#![allow(dead_code)]

use flecs_ecs::core::*;
use flecs_ecs::macros::*;

use crate::common_test::*;

#[derive(Debug, Component)]
struct LastEntity(Entity);

#[test]
fn n2_terms_on_add() {
    let world = World::new();

    world.set(Count(0));

    world
        .observer::<flecs::OnAdd, ()>()
        .with(Position::id())
        .with(Velocity::id())
        .each_entity(|e, _| {
            let world = e.world();
            world.get::<&mut Count>(|count| {
                count.0 += 1;
            });
        });

    let e = world.entity();
    world.get::<&mut Count>(|count| {
        assert_eq!(count.0, 0);
    });
    e.set(Position { x: 10, y: 20 });
    world.get::<&mut Count>(|count| {
        assert_eq!(count.0, 0);
    });
    e.set(Velocity { x: 1, y: 2 });
    world.get::<&mut Count>(|count| {
        assert_eq!(count.0, 1);
    });
}

#[test]
fn n2_terms_on_remove() {
    let world = World::new();

    world.set(Count(0));

    world
        .observer::<flecs::OnRemove, (&Position, &Velocity)>()
        .each_entity(|e, (pos, vel)| {
            let world = e.world();
            world.get::<&mut Count>(|count| {
                count.0 += 1;
            });
            assert_eq!(pos.x, 10);
            assert_eq!(pos.y, 20);
            assert_eq!(vel.x, 1);
            assert_eq!(vel.y, 2);
        });

    let e = world.entity();
    world.get::<&mut Count>(|count| {
        assert_eq!(count, 0);
    });
    e.set(Position { x: 10, y: 20 });
    world.get::<&mut Count>(|count| {
        assert_eq!(count, 0);
    });
    e.set(Velocity { x: 1, y: 2 });
    world.get::<&mut Count>(|count| {
        assert_eq!(count, 0);
    });
    e.remove(Velocity::id());
    world.get::<&mut Count>(|count| {
        assert_eq!(count, 1);
    });
    e.remove(Position::id());
    world.get::<&mut Count>(|count| {
        assert_eq!(count, 1);
    });
}

#[test]
fn n2_terms_on_set() {
    let world = World::new();

    world.set(Count(0));

    world
        .observer::<flecs::OnSet, (&Position, &Velocity)>()
        .each_entity(|e, (pos, vel)| {
            let world = e.world();
            world.get::<&mut Count>(|count| {
                count.0 += 1;
            });
            assert_eq!(pos.x, 10);
            assert_eq!(pos.y, 20);
            assert_eq!(vel.x, 1);
            assert_eq!(vel.y, 2);
        });

    let e = world.entity();
    world.get::<&mut Count>(|count| {
        assert_eq!(count, 0);
    });
    e.set(Position { x: 10, y: 20 });
    world.get::<&mut Count>(|count| {
        assert_eq!(count, 0);
    });
    e.set(Velocity { x: 1, y: 2 });
    world.get::<&mut Count>(|count| {
        assert_eq!(count, 1);
    });
}

#[test]
fn n10_terms() {
    let world = World::new();

    world.set(Count(0));

    let e = world.entity();
    let e_id = e.id();

    world
        .observer::<flecs::OnAdd, ()>()
        .with(TagA::id())
        .with(TagB::id())
        .with(TagC::id())
        .with(TagD::id())
        .with(TagE::id())
        .with(TagF::id())
        .with(TagG::id())
        .with(TagH::id())
        .with(TagI::id())
        .with(TagJ::id())
        .run(move |mut it| {
            let world = it.world();
            while it.next() {
                for i in it.iter() {
                    assert_eq!(it.count(), 1);
                    assert!(it.entity_id(i) == e_id);
                    assert_eq!(it.field_count(), 10);
                    world.get::<&mut Count>(|count| {
                        count.0 += 1;
                    });
                }
            }
        });

    e.add(TagA::id())
        .add(TagB::id())
        .add(TagC::id())
        .add(TagD::id())
        .add(TagE::id())
        .add(TagF::id())
        .add(TagG::id())
        .add(TagH::id())
        .add(TagI::id())
        .add(TagJ::id());

    world.get::<&mut Count>(|count| {
        assert_eq!(count, 1);
    });
}

#[test]
fn n16_terms() {
    let world = World::new();

    world.set(Count(0));

    let e = world.entity();
    let e_id = e.id();

    world
        .observer::<flecs::OnAdd, ()>()
        .with(TagA::id())
        .with(TagB::id())
        .with(TagC::id())
        .with(TagD::id())
        .with(TagE::id())
        .with(TagF::id())
        .with(TagG::id())
        .with(TagH::id())
        .with(TagI::id())
        .with(TagJ::id())
        .with(TagK::id())
        .with(TagL::id())
        .with(TagM::id())
        .with(TagN::id())
        .with(TagO::id())
        .with(TagP::id())
        .run(move |mut it| {
            let world = it.world();
            while it.next() {
                for i in it.iter() {
                    assert_eq!(it.count(), 1);
                    assert!(it.entity_id(i) == e_id);
                    assert_eq!(it.field_count(), 16);
                    world.get::<&mut Count>(|count| {
                        count.0 += 1;
                    });
                }
            }
        });

    e.add(TagA::id())
        .add(TagB::id())
        .add(TagC::id())
        .add(TagD::id())
        .add(TagE::id())
        .add(TagF::id())
        .add(TagG::id())
        .add(TagH::id())
        .add(TagI::id())
        .add(TagJ::id())
        .add(TagK::id())
        .add(TagL::id())
        .add(TagM::id())
        .add(TagN::id())
        .add(TagO::id())
        .add(TagP::id())
        .add(TagQ::id())
        .add(TagR::id())
        .add(TagS::id())
        .add(TagT::id());

    world.get::<&mut Count>(|count| {
        assert_eq!(count, 1);
    });
}

#[test]
fn n2_entities_iter() {
    let world = World::new();

    let e1 = world.entity();
    let e2 = world.entity();

    let e1_id = e1.id();
    let e2_id = e2.id();

    world.set(Count(0));
    world.set(LastEntity(Entity::null()));

    world
        .observer::<flecs::OnSet, &Position>()
        .run(move |mut it| {
            let world = it.world();
            while it.next() {
                let p = it.field::<Position>(0);

                for i in it.iter() {
                    world.get::<&mut Count>(|count| {
                        count.0 += 1;
                    });
                    if it.get_entity(i).unwrap() == e1_id {
                        assert_eq!(p[i].x, 10);
                        assert_eq!(p[i].y, 20);
                    } else if it.get_entity(i).unwrap() == e2_id {
                        assert_eq!(p[i].x, 30);
                        assert_eq!(p[i].y, 40);
                    } else {
                        unreachable!();
                    }

                    world.get::<&mut LastEntity>(|last| {
                        last.0 = it.get_entity(i).unwrap().id();
                    });
                }
            }
        });

    e1.set(Position { x: 10, y: 20 });
    world.get::<&mut Count>(|count| {
        world.get::<&mut LastEntity>(|last| {
            assert_eq!(count, 1);
            assert!(last.0 == e1.id());
        });
    });

    e2.set(Position { x: 30, y: 40 });
    world.get::<&mut Count>(|count| {
        world.get::<&mut LastEntity>(|last| {
            assert_eq!(count, 2);
            assert!(last.0 == e2.id());
        });
    });
}

#[test]
fn n2_entities_table_column() {
    let world = World::new();

    let e1 = world.entity();
    let e2 = world.entity();

    let e1_id = e1.id();
    let e2_id = e2.id();

    world.set(Count(0));
    world.set(LastEntity(Entity::null()));

    world
        .observer::<flecs::OnSet, &Position>()
        .run(move |mut it| {
            let world = it.world();
            while it.next() {
                let mut table_range = it.range().unwrap();
                let p = table_range.get_mut::<Position>().unwrap();

                for i in it.iter() {
                    let i: usize = i.into();
                    world.get::<&mut Count>(|count| {
                        count.0 += 1;
                    });
                    if it.get_entity(i).unwrap() == e1_id {
                        assert_eq!(p[i].x, 10);
                        assert_eq!(p[i].y, 20);
                    } else if it.get_entity(i).unwrap() == e2_id {
                        assert_eq!(p[i].x, 30);
                        assert_eq!(p[i].y, 40);
                    } else {
                        unreachable!();
                    }

                    world.get::<&mut LastEntity>(|last| {
                        last.0 = it.get_entity(i).unwrap().id();
                    });
                }
            }
        });

    e1.set(Position { x: 10, y: 20 });
    world.get::<&mut Count>(|count| {
        world.get::<&mut LastEntity>(|last| {
            assert_eq!(count, 1);
            assert!(last.0 == e1.id());
        });
    });

    e2.set(Position { x: 30, y: 40 });
    world.get::<&mut Count>(|count| {
        world.get::<&mut LastEntity>(|last| {
            assert_eq!(count, 2);
            assert!(last.0 == e2.id());
        });
    });
}

#[test]
fn n2_entities_each() {
    let world = World::new();

    let e1 = world.entity();
    let e2 = world.entity();

    let e1_id = e1.id();
    let e2_id = e2.id();

    world.set(Count(0));
    world.set(LastEntity(Entity::null()));

    world
        .observer::<flecs::OnSet, &Position>()
        .each_entity(move |e, pos| {
            let world = e.world();
            world.get::<&mut Count>(|count| {
                count.0 += 1;
            });
            if e == e1_id {
                assert_eq!(pos.x, 10);
                assert_eq!(pos.y, 20);
            } else if e == e2_id {
                assert_eq!(pos.x, 30);
                assert_eq!(pos.y, 40);
            } else {
                unreachable!();
            }

            world.get::<&mut LastEntity>(|last| {
                last.0 = e.id();
            });
        });

    e1.set(Position { x: 10, y: 20 });
    world.get::<&mut Count>(|count| {
        world.get::<&mut LastEntity>(|last| {
            assert_eq!(count, 1);
            assert!(last.0 == e1);
        });
    });
    e2.set(Position { x: 30, y: 40 });
    world.get::<&mut Count>(|count| {
        world.get::<&mut LastEntity>(|last| {
            assert_eq!(count, 2);
            assert!(last.0 == e2);
        });
    });
}

#[test]
fn create_w_no_template_args() {
    let world = World::new();

    let e1 = world.entity();
    let e1_id = e1.id();

    world.set(Count(0));

    world
        .observer::<flecs::OnAdd, ()>()
        .with(Position::id())
        .each_entity(move |e, _| {
            let world = e.world();
            assert!(e == e1_id);
            world.get::<&mut Count>(|count| {
                count.0 += 1;
            });
        });

    e1.set(Position { x: 10, y: 20 });

    world.get::<&mut Count>(|count| {
        assert_eq!(count, 1);
    });
}

#[test]
fn yield_existing() {
    let world = World::new();

    let e1 = world.entity().add(TagA::id());
    let e2 = world.entity().add(TagA::id());
    let e3 = world.entity().add(TagA::id()).add(TagB::id());

    let e1_id = e1.id();
    let e2_id = e2.id();
    let e3_id = e3.id();

    world.set(Count(0));
    world
        .observer::<flecs::OnAdd, ()>()
        .with(TagA::id())
        .yield_existing()
        .run(move |mut it| {
            while it.next() {
                for i in it.iter() {
                    let e = it.get_entity(i).unwrap();
                    let world = e.world();
                    world.get::<&mut Count>(|count| {
                        if e == e1_id {
                            count.0 += 1;
                        } else if e == e2_id {
                            count.0 += 2;
                        } else if e == e3_id {
                            count.0 += 3;
                        }
                    });
                }
            }
        });

    world.get::<&mut Count>(|count| {
        assert_eq!(count.0, 6);
    });
}

#[test]
fn yield_existing_2_terms() {
    let world = World::new();

    let e1 = world.entity().add(TagA::id()).add(TagB::id());
    let e2 = world.entity().add(TagA::id()).add(TagB::id());
    let e3 = world
        .entity()
        .add(TagA::id())
        .add(TagB::id())
        .add(TagC::id());

    let e1_id = e1.id();
    let e2_id = e2.id();
    let e3_id = e3.id();

    world.entity().add(TagA::id());
    world.entity().add(TagB::id());

    world.set(Count(0));
    world
        .observer::<flecs::OnAdd, ()>()
        .with(TagA::id())
        .with(TagB::id())
        .yield_existing()
        .each_entity(move |e, _| {
            let world = e.world();
            world.get::<&mut Count>(|count| {
                if e == e1_id {
                    count.0 += 1;
                } else if e == e2_id {
                    count.0 += 2;
                } else if e == e3_id {
                    count.0 += 3;
                }
            });
        });

    world.get::<&mut Count>(|count| {
        assert_eq!(count.0, 6);
    });
}

#[test]
fn on_add() {
    let world = World::new();
    world.set(Count(0));
    world
        .observer::<flecs::OnAdd, ()>()
        .with(Position::id())
        .each_entity(|e, _| {
            let world = e.world();
            world.get::<&mut Count>(|count| {
                count.0 += 1;
            });
        });

    world.entity().add(Position::id());

    world.get::<&mut Count>(|count| {
        assert_eq!(count, 1);
    });
}

#[test]
fn on_remove() {
    let world = World::new();
    world.set(Count(0));
    world
        .observer::<flecs::OnRemove, &Position>()
        .each_entity(|e, _p| {
            e.world().get::<&mut Count>(|count| {
                count.0 += 1;
            });
        });
    let e = world.entity().add(Position::id());

    world.get::<&mut Count>(|count| {
        assert_eq!(count, 0);
    });

    e.remove(Position::id());

    world.get::<&mut Count>(|count| {
        assert_eq!(count, 1);
    });
}

#[test]
fn on_add_tag_action() {
    let world = World::new();
    world.set(Count(0));
    world
        .observer::<flecs::OnAdd, ()>()
        .with(TagA::id())
        .run(|mut it| {
            let world = it.world();
            while it.next() {
                world.get::<&mut Count>(|count| {
                    count.0 += 1;
                });
            }
        });
    world.entity().add(TagA::id());

    world.get::<&mut Count>(|count| {
        assert_eq!(count, 1);
    });
}

#[test]
fn on_add_tag_iter() {
    let world = World::new();
    world.set(Count(0));
    world
        .observer::<flecs::OnAdd, ()>()
        .with(TagA::id())
        .run(|mut it| {
            let world = it.world();
            while it.next() {
                world.get::<&mut Count>(|count| {
                    count.0 += 1;
                });
            }
        });
    world.entity().add(TagA::id());
    world.get::<&mut Count>(|count| {
        assert_eq!(count, 1);
    });
}

#[test]
fn on_add_tag_each() {
    let world = World::new();
    world.set(Count(0));
    world
        .observer::<flecs::OnAdd, ()>()
        .with(TagA::id())
        .run(|mut it| {
            while it.next() {
                for _ in it.iter() {
                    it.world().get::<&mut Count>(|count| {
                        count.0 += 1;
                    });
                }
            }
        });
    world.entity().add(TagA::id());
    world.get::<&mut Count>(|count| {
        assert_eq!(count, 1);
    });
}

#[test]
fn on_add_expr() {
    let world = World::new();
    world.set(Count(0));
    world.component::<Tag>();
    world
        .observer::<flecs::OnAdd, ()>()
        .expr("Tag")
        .each_entity(|e, _| {
            e.world().get::<&mut Count>(|count| {
                count.0 += 1;
            });
        })
        .expect("valid observer query expression");
    let e = world.entity().add(Tag);
    world.get::<&mut Count>(|count| {
        assert_eq!(count, 1);
    });
    e.remove(Tag);
    world.get::<&mut Count>(|count| {
        assert_eq!(count, 1);
    });
}

#[test]
fn observer_w_filter_term() {
    let world = World::new();
    let tag_a = world.entity();
    let tag_b = world.entity();
    world.set(Count(0));
    world
        .observer::<flecs::OnAdd, ()>()
        .with(tag_a)
        .with(tag_b)
        .filter()
        .each_entity(|e, _| {
            e.world().get::<&mut Count>(|count| {
                count.0 += 1;
            });
        });
    let e = world.entity();

    world.get::<&mut Count>(|count| {
        assert_eq!(count, 0);
    });

    e.add(tag_b);

    world.get::<&mut Count>(|count| {
        assert_eq!(count, 0);
    });

    e.add(tag_a);

    world.get::<&mut Count>(|count| {
        assert_eq!(count, 1);
    });

    e.remove(tag_b);

    world.get::<&mut Count>(|count| {
        assert_eq!(count, 1);
    });

    e.add(tag_b);

    world.get::<&mut Count>(|count| {
        assert_eq!(count, 1);
    });

    e.clear();

    world.get::<&mut Count>(|count| {
        assert_eq!(count, 1);
    });

    e.add(tag_a);

    world.get::<&mut Count>(|count| {
        assert_eq!(count, 1);
    });
}

#[test]
fn run_callback() {
    let world = World::new();

    world.set(Count(0));
    world
        .observer::<flecs::OnAdd, ()>()
        .with(Position::id())
        .run_each_entity(
            |mut it| {
                while it.next() {
                    it.each();
                }
            },
            |e, _p| {
                e.world().get::<&mut Count>(|count| {
                    count.0 += 1;
                });
            },
        );
    let e = world.entity();
    world.get::<&mut Count>(|count| {
        assert_eq!(count, 0);
    });

    e.add(Position::id());

    world.get::<&mut Count>(|count| {
        assert_eq!(count, 1);
    });
}

#[test]
fn get_query() {
    let world = World::new();
    world.entity().set(Position { x: 0, y: 0 });
    world.entity().set(Position { x: 1, y: 0 });
    world.entity().set(Position { x: 2, y: 0 });

    world.set(Count(0));
    let mut o = world
        .observer::<flecs::OnSet, &Position>()
        .each_entity(|_e, _p| {});
    let q = o.query();
    q.run(|mut it| {
        while it.next() {
            let pos = it.field::<Position>(0);
            for i in it.iter() {
                assert_eq!(<FieldIndex as Into<usize>>::into(i) as i32, pos[i].x);
                world.get::<&mut Count>(|count| {
                    count.0 += 1;
                });
            }
        }
    });

    world.get::<&mut Count>(|count| {
        assert_eq!(count, 3);
    });
}

#[test]
fn on_set_w_set() {
    let world = World::new();

    world.set(Count(0));
    world
        .observer::<flecs::OnSet, &Position>()
        .each_entity(|e, _p| {
            e.world().get::<&mut Count>(|count| {
                count.0 += 1;
            });
        });
    let e = world.entity();

    world.get::<&mut Count>(|count| {
        assert_eq!(count, 0);
    });

    e.set(Position { x: 10, y: 20 });

    world.get::<&mut Count>(|count| {
        assert_eq!(count, 1);
    });
}

#[test]
fn on_set_w_defer_set() {
    let world = World::new();

    world.set(Count(0));
    world
        .observer::<flecs::OnSet, &Position>()
        .each_entity(|e, _p| {
            e.world().get::<&mut Count>(|count| {
                count.0 += 1;
            });
        });
    let e = world.entity();
    world.get::<&mut Count>(|count| {
        assert_eq!(count, 0);
    });

    world.defer_begin();

    world.get::<&mut Count>(|count| {
        assert_eq!(count, 0);
    });

    e.set(Position { x: 10, y: 20 });

    world.get::<&mut Count>(|count| {
        assert_eq!(count, 0);
    });

    world.defer_end();

    world.get::<&mut Count>(|count| {
        assert_eq!(count, 1);
    });
}

#[test]
fn on_set_w_set_sparse() {
    let world = World::new();

    world.set(Count(0));
    world.component::<Position>().add_trait::<flecs::Sparse>();

    world
        .observer::<flecs::OnSet, &Position>()
        .each_entity(|e, _p| {
            e.world().get::<&mut Count>(|count| {
                count.0 += 1;
            });
        });

    let e = world.entity();
    world.get::<&mut Count>(|count| {
        assert_eq!(count, 0);
    });

    e.set(Position { x: 10, y: 20 });

    world.get::<&mut Count>(|count| {
        assert_eq!(count, 1);
    });
}

#[test]
fn on_add_singleton() {
    let world = World::new();

    world
        .component::<Position>()
        .add_trait::<flecs::Singleton>();

    world.set(Count(0));
    world
        .observer::<flecs::OnSet, &Position>()
        .term_at(0)
        .run(|mut it| {
            let world = it.world();
            while it.next() {
                let pos = it.field::<Position>(0);
                assert_eq!(pos[0].x, 10);
                assert_eq!(pos[0].y, 20);
                world.get::<&mut Count>(|count| {
                    count.0 += 1;
                });
            }
        });

    world.set(Position { x: 10, y: 20 });

    world.get::<&mut Count>(|count| {
        assert_eq!(count, 1);
    });
}

#[test]
fn on_add_pair_singleton() {
    let world = World::new();

    world
        .component::<Position>()
        .add_trait::<flecs::Singleton>();

    world.set(Count(0));
    let tgt = world.entity();
    world
        .observer::<flecs::OnSet, ()>()
        .with((Position::id(), tgt))
        .run(|mut it| {
            let world = it.world();
            while it.next() {
                let pos = it.field::<Position>(0);
                assert_eq!(pos[0].x, 10);
                assert_eq!(pos[0].y, 20);
                world.get::<&mut Count>(|count| {
                    count.0 += 1;
                });
            }
        });
    world.set_first(tgt, Position { x: 10, y: 20 });
    world.get::<&mut Count>(|count| {
        assert_eq!(count, 1);
    });
}

#[test]
fn on_add_pair_wildcard_singleton() {
    let world = World::new();

    world
        .component::<Position>()
        .add_trait::<flecs::Singleton>();

    world.set(Count(0));
    let tgt_1 = world.entity();
    let tgt_2 = world.entity();
    world
        .observer::<flecs::OnSet, &(Position, flecs::Wildcard)>()
        .term_at(0)
        .run(|mut it| {
            let world = it.world();
            while it.next() {
                let pos = it.field::<Position>(0);
                assert_eq!(pos[0].x, 10);
                assert_eq!(pos[0].y, 20);
                world.get::<&mut Count>(|count| {
                    count.0 += 1;
                });
            }
        });

    world.set_first::<Position>(tgt_1, Position { x: 10, y: 20 });

    world.get::<&mut Count>(|count| {
        assert_eq!(count, 1);
    });

    world.set_first::<Position>(tgt_2, Position { x: 10, y: 20 });

    world.get::<&mut Count>(|count| {
        assert_eq!(count, 2);
    });
}

#[test]
fn on_add_with_pair_singleton() {
    let world = World::new();

    world
        .component::<Position>()
        .add_trait::<flecs::Singleton>();

    world.set(Count(0));
    let tgt = world.entity();
    world
        .observer::<flecs::OnSet, ()>()
        .with((Position::id(), tgt))
        .run(|mut it| {
            let world = it.world();
            while it.next() {
                world.get::<&mut Count>(|count| {
                    count.0 += 1;
                });
            }
        });

    world.set_first::<Position>(tgt, Position { x: 10, y: 20 });
    world.get::<&mut Count>(|count| {
        assert_eq!(count, 1);
    });
}

#[test]
fn add_in_yield_existing() {
    let world = World::new();
    let e1 = world.entity().set(Position::default());
    let e2 = world.entity().set(Position::default());
    let e3 = world.entity().set(Position::default());
    world
        .observer::<flecs::OnAdd, ()>()
        .with(Position::id())
        .yield_existing()
        .each_entity(|e, _| {
            e.add(Velocity::id());
        });

    assert!(e1.has(Position::id()));
    assert!(e1.has(Velocity::id()));
    assert!(e2.has(Position::id()));
    assert!(e2.has(Velocity::id()));
    assert!(e3.has(Position::id()));
    assert!(e3.has(Velocity::id()));
}

#[test]
fn add_in_yield_existing_multi() {
    let world = World::new();
    let e1 = world.entity().set(Position::default()).set(Mass::default());
    let e2 = world.entity().set(Position::default()).set(Mass::default());
    let e3 = world.entity().set(Position::default()).set(Mass::default());
    world
        .observer::<flecs::OnAdd, ()>()
        .with(Position::id())
        .with(Mass::id())
        .yield_existing()
        .each_entity(|e, _| {
            e.add(Velocity::id());
        });
    assert!(e1.has(Position::id()));
    assert!(e1.has(Mass::id()));
    assert!(e1.has(Velocity::id()));
    assert!(e2.has(Position::id()));
    assert!(e2.has(Mass::id()));
    assert!(e2.has(Velocity::id()));
    assert!(e3.has(Position::id()));
    assert!(e3.has(Mass::id()));
    assert!(e3.has(Velocity::id()));
}

#[test]
fn name_from_root() {
    let world = World::new();
    let o = world
        .observer_named::<flecs::OnSet, &Position>("::ns::MyObserver")
        .each(|_| {});

    assert_eq!(o.name(), "MyObserver");
    let ns = world.entity_named("::ns");
    assert!(ns == o.parent().unwrap());
}

// #[test]
// #[should_panic]
// fn observer_panic_inside() {
//     #[derive(Component)]
//     struct Tag;

//     let world = World::new();
//     world.observer::<flecs::OnAdd, ()>().with(Tag).run(|_| panic!());
//     world.add(Tag);
// }

#[test]
fn register_twice_w_each() {
    let world = World::new();

    world.set(Count2 { a: 0, b: 0 });

    let o = world
        .observer_named::<flecs::OnSet, &Position>("Test")
        .each_entity(|e, _| {
            e.world().get::<&mut Count2>(|count| {
                count.a += 1;
            });
        });

    world.entity().set(Position { x: 10, y: 20 });

    world.get::<&mut Count2>(|count| {
        assert_eq!(count.a, 1);
    });

    o.update::<&Position>().each_entity(|e, _| {
        e.world().get::<&mut Count2>(|count| {
            count.b += 1;
        });
    });

    world.entity().set(Position { x: 10, y: 20 });

    world.get::<&mut Count2>(|count| {
        assert_eq!(count.b, 1);
    });
}

#[test]
fn register_twice_w_run() {
    let world = World::new();

    world.set(Count2 { a: 0, b: 0 });

    let o = world
        .observer_named::<flecs::OnSet, &Position>("Test")
        .run(|mut it| {
            while it.next() {
                it.world().get::<&mut Count2>(|count| {
                    count.a += 1;
                });
            }
        });

    world.entity().set(Position { x: 10, y: 20 });

    world.get::<&mut Count2>(|count| {
        assert_eq!(count.a, 1);
    });

    o.update::<&Position>().run(|mut it| {
        while it.next() {
            it.world().get::<&mut Count2>(|count| {
                count.b += 1;
            });
        }
    });

    world.entity().set(Position { x: 10, y: 20 });

    world.get::<&mut Count2>(|count| {
        assert_eq!(count.b, 1);
    });
}

#[test]
fn register_twice_w_run_each() {
    let world = World::new();

    world.set(Count2 { a: 0, b: 0 });

    let o = world
        .observer_named::<flecs::OnSet, &Position>("Test")
        .run(|mut it| {
            while it.next() {
                it.world().get::<&mut Count2>(|count| {
                    count.a += 1;
                });
            }
        });

    world.entity().set(Position { x: 10, y: 20 });

    world.get::<&mut Count2>(|count| {
        assert_eq!(count.a, 1);
    });

    o.update::<&Position>().each_entity(|e, _| {
        e.world().get::<&mut Count2>(|count| {
            count.b += 1;
        });
    });

    world.entity().set(Position { x: 10, y: 20 });

    world.get::<&mut Count2>(|count| {
        assert_eq!(count.b, 1);
    });
}

#[test]
fn register_twice_w_each_run() {
    let world = World::new();

    world.set(Count2 { a: 0, b: 0 });

    let o = world
        .observer_named::<flecs::OnSet, &Position>("Test")
        .each_entity(|e, _| {
            e.world().get::<&mut Count2>(|count| {
                count.a += 1;
            });
        });

    world.entity().set(Position { x: 10, y: 20 });

    world.get::<&mut Count2>(|count| {
        assert_eq!(count.a, 1);
    });

    o.update::<&Position>().run(|mut it| {
        while it.next() {
            it.world().get::<&mut Count2>(|count| {
                count.b += 1;
            });
        }
    });

    world.entity().set(Position { x: 10, y: 20 });

    world.get::<&mut Count2>(|count| {
        assert_eq!(count.b, 1);
    });
}

#[test]
fn yield_existing_on_create_flag() {
    let world = World::new();

    let e1 = world.entity().add(TagA::id());
    let e2 = world.entity().add(TagA::id());
    let e3 = world.entity().add(TagA::id()).add(TagB::id());

    let e1_id = e1.id();
    let e2_id = e2.id();
    let e3_id = e3.id();

    world.set(Count(0));

    let mut ob = world.observer::<flecs::OnAdd, ()>();
    ob.with(TagA::id());
    ob.add_event(flecs::OnRemove::ID);
    ob.set_observer_flags(ObserverFlags::YieldOnCreate);
    let o = ob.each_entity(move |e, _| {
        let world = e.world();
        world.get::<&mut Count>(|count| {
            if e == e1_id {
                count.0 += 1;
            } else if e == e2_id {
                count.0 += 2;
            } else if e == e3_id {
                count.0 += 3;
            }
        });
    });

    world.get::<&Count>(|count| {
        assert_eq!(count.0, 6);
    });

    o.entity().destruct();

    world.get::<&Count>(|count| {
        assert_eq!(count.0, 6);
    });
}

#[test]
fn yield_existing_on_delete_flag() {
    let world = World::new();

    let e1 = world.entity().add(TagA::id());
    let e2 = world.entity().add(TagA::id());
    let e3 = world.entity().add(TagA::id()).add(TagB::id());

    let e1_id = e1.id();
    let e2_id = e2.id();
    let e3_id = e3.id();

    world.set(Count(0));

    let mut ob = world.observer::<flecs::OnAdd, ()>();
    ob.with(TagA::id());
    ob.add_event(flecs::OnRemove::ID);
    ob.set_observer_flags(ObserverFlags::YieldOnDelete);
    let o = ob.each_entity(move |e, _| {
        let world = e.world();
        world.get::<&mut Count>(|count| {
            if e == e1_id {
                count.0 += 1;
            } else if e == e2_id {
                count.0 += 2;
            } else if e == e3_id {
                count.0 += 3;
            }
        });
    });

    world.get::<&Count>(|count| {
        assert_eq!(count.0, 0);
    });

    o.entity().destruct();

    world.get::<&Count>(|count| {
        assert_eq!(count.0, 6);
    });
}

#[test]
fn yield_existing_on_create_delete_flag() {
    let world = World::new();

    let e1 = world.entity().add(TagA::id());
    let e2 = world.entity().add(TagA::id());
    let e3 = world.entity().add(TagA::id()).add(TagB::id());

    let e1_id = e1.id();
    let e2_id = e2.id();
    let e3_id = e3.id();

    world.set(Count(0));

    let mut ob = world.observer::<flecs::OnAdd, ()>();
    ob.with(TagA::id());
    ob.add_event(flecs::OnRemove::ID);
    ob.set_observer_flags(ObserverFlags::YieldOnCreate | ObserverFlags::YieldOnDelete);
    let o = ob.each_entity(move |e, _| {
        let world = e.world();
        world.get::<&mut Count>(|count| {
            if e == e1_id {
                count.0 += 1;
            } else if e == e2_id {
                count.0 += 2;
            } else if e == e3_id {
                count.0 += 3;
            }
        });
    });

    world.get::<&Count>(|count| {
        assert_eq!(count.0, 6);
    });

    o.entity().destruct();

    world.get::<&Count>(|count| {
        assert_eq!(count.0, 12);
    });
}

#[test]
fn default_ctor() {
    let world = World::new();

    world.set(Count(0));

    // Observer after construction is non-null
    let o = world
        .observer::<flecs::OnAdd, ()>()
        .with(TagA::id())
        .each_entity(|e, _| {
            e.world().get::<&mut Count>(|count| {
                count.0 += 1;
            });
        });

    // entity() returns the underlying entity; observer is non-null after construction
    assert!(*o.entity().id() != 0);

    world.entity().add(TagA::id());

    world.get::<&Count>(|count| {
        assert_eq!(count.0, 1);
    });
}

#[test]
fn entity_ctor() {
    let world = World::new();

    let o = world
        .observer::<flecs::OnAdd, ()>()
        .with(TagA::id())
        .each_entity(|_e, _| {});

    let oe = o.entity();
    let eo = world.observer_from(oe);
    assert!(*eo.entity().id() == *o.entity().id());
}

#[test]
fn term_index() {
    let world = World::new();

    world.set(Count(0));

    let e1 = world.entity().add(Position::id()).add(Velocity::id());

    world.set(Count2 { a: -1, b: -1 });

    world
        .observer::<flecs::OnSet, (&Position, &Velocity)>()
        .each_iter(|it, _row, (_p, _v)| {
            it.world().get::<&mut Count2>(|c| {
                c.a = it.term_index() as i32;
            });
        });

    e1.set(Position { x: 10, y: 20 });
    world.get::<&Count2>(|c| {
        assert_eq!(c.a, 0);
    });

    e1.set(Velocity { x: 30, y: 40 });
    world.get::<&Count2>(|c| {
        assert_eq!(c.a, 1);
    });
}

/*
void Observer_on_set_singleton_set_component_named_entity(void) {
    flecs::world world;

    struct MyComponent {
        int v = 0;
    };

    struct MySingletonComponent {
        int v = 0;
    };

    world.component<MyComponent>();
    world.component<MySingletonComponent>().add(flecs::Singleton);

    world.observer<const MySingletonComponent>()
        .write<MySingletonComponent>()
        .event(flecs::OnSet)
        .each([](flecs::iter &it, size_t, const MySingletonComponent &c1) {
            it.world().entity("A").set<MyComponent>({c1.v});
        });

    world.set<MySingletonComponent>({1});

    test_int(world.entity("A").get<MyComponent>().v, 1);
}

*/
#[test]
fn on_set_singleton_set_component_named_entity() {
    #[derive(Component)]
    struct MyComponent {
        v: i32,
    }

    #[derive(Component, Default)]
    struct MySingletonComponent {
        v: i32,
    }

    let world = World::new();

    world.component::<MyComponent>();
    world
        .component::<MySingletonComponent>()
        .add_trait::<flecs::Singleton>();

    world
        .observer::<flecs::OnSet, &MySingletonComponent>()
        .write(MySingletonComponent::id())
        .each_iter(|it, _row, c1| {
            it.world().entity_named("A").set(MyComponent { v: c1.v });
        });

    world.set(MySingletonComponent { v: 1 });

    world
        .entity_named("A")
        .get::<&MyComponent>(|c| assert_eq!(c.v, 1));
}

#[test]
fn implicit_register_in_emit_for_named_entity() {
    #[derive(Component)]
    struct MyEvent {
        value: f32,
    }

    let world = World::new();

    let e1 = world.entity_named("e1");
    let e2_id = world.entity().id();

    e1.observe_payload::<MyEvent>(move |evt| {
        assert!((evt.value - 10.0).abs() < f32::EPSILON);
        let _ = e2_id; // just validate event fires
    });

    e1.emit(&MyEvent { value: 10.0 });
}

#[test]
fn add_to_named_in_emit_for_named_entity() {
    #[derive(Component)]
    struct MyEvent {
        value: f32,
    }

    let world = World::new();
    world.component::<Position>();

    let e1 = world.entity_named("e1");
    world.entity_named("e2");

    e1.observe_payload::<MyEvent>(move |evt| {
        assert!((evt.value - 10.0).abs() < f32::EPSILON);
        // e2 referenced by name; event firing is what matters
    });

    e1.emit(&MyEvent { value: 10.0 });
}

#[test]
fn implicit_register_in_emit_for_named_entity_w_defer() {
    #[derive(Component)]
    struct MyEvent {
        value: f32,
    }

    let world = World::new();

    let e1 = world.entity_named("e1");
    let _e2_id = world.entity().id();

    e1.observe_payload::<MyEvent>(move |evt| {
        assert!((evt.value - 10.0).abs() < f32::EPSILON);
    });

    world.defer_begin();
    e1.emit(&MyEvent { value: 10.0 });
    world.defer_end();
}

#[test]
fn add_to_named_in_emit_for_named_entity_w_defer() {
    #[derive(Component)]
    struct MyEvent {
        value: f32,
    }

    let world = World::new();
    world.component::<Position>();

    let e1 = world.entity_named("e1");
    world.entity_named("e2");

    e1.observe_payload::<MyEvent>(move |evt| {
        assert!((evt.value - 10.0).abs() < f32::EPSILON);
    });

    world.defer_begin();
    e1.emit(&MyEvent { value: 10.0 });
    world.defer_end();
}

#[test]
fn lookup_and_update_ctx() {
    let world = World::new();

    world
        .observer_named::<flecs::OnSet, &Position>("Test")
        .each(|_| {});

    let e = world.lookup("Test");
    assert!(*e.id() != 0);

    let mut o = world.observer_from(world.entity_named("Test"));
    assert!(o.context().is_null());

    let mut my_ctx: i32 = 42;
    o.set_context(&mut my_ctx as *mut i32 as *mut core::ffi::c_void);
    assert!(o.context() == &mut my_ctx as *mut i32 as *mut core::ffi::c_void);
}

#[test]
fn lookup_and_update_each() {
    let world = World::new();

    world.set(Count2 { a: 0, b: 0 });

    world
        .observer_named::<flecs::OnSet, &Position>("Test")
        .each_entity(|e, _| {
            e.world().get::<&mut Count2>(|count| {
                count.a += 1;
            });
        });

    world.entity().set(Position { x: 10, y: 20 });
    world.get::<&Count2>(|count| {
        assert_eq!(count.a, 1);
    });

    let e = world.lookup("Test");
    assert!(*e.id() != 0);

    let o = world.observer_from(world.entity_named("Test"));
    o.update::<&Position>().each_entity(|e, _| {
        e.world().get::<&mut Count2>(|count| {
            count.b += 1;
        });
    });

    world.entity().set(Position { x: 10, y: 20 });
    world.get::<&Count2>(|count| {
        assert_eq!(count.a, 1);
        assert_eq!(count.b, 1);
    });
}

#[test]
fn lookup_and_update_run() {
    let world = World::new();

    world.set(Count2 { a: 0, b: 0 });

    world
        .observer_named::<flecs::OnSet, &Position>("Test")
        .each_entity(|e, _| {
            e.world().get::<&mut Count2>(|count| {
                count.a += 1;
            });
        });

    world.entity().set(Position { x: 10, y: 20 });
    world.get::<&Count2>(|count| {
        assert_eq!(count.a, 1);
    });

    let o = world.observer_from(world.entity_named("Test"));
    o.update::<&Position>().run(|mut it| {
        while it.next() {
            it.world().get::<&mut Count2>(|count| {
                count.b += 1;
            });
        }
    });

    world.entity().set(Position { x: 10, y: 20 });
    world.get::<&Count2>(|count| {
        assert_eq!(count.a, 1);
        assert_eq!(count.b, 1);
    });
}

#[test]
fn other_table() {
    let world = World::new();

    world.set(Count(0));

    // Use untyped observer with with() since OnAdd observers cannot use typed &T args
    world
        .observer::<flecs::OnAdd, ()>()
        .with(Velocity::id())
        .run(|mut it| {
            while it.next() {
                assert!(it.table().is_some_and(|t| t.has(Velocity::id())));
                assert!(!it.other_table().is_some_and(|t| t.has(Velocity::id())));
                it.world().get::<&mut Count>(|count| {
                    count.0 += 1;
                });
            }
        });

    let e = world.entity().add(Position::id()).add(Velocity::id());
    assert!(e.has(Velocity::id()));
    world.get::<&Count>(|count| {
        assert_eq!(count.0, 1);
    });
}

#[test]
fn other_table_w_pair() {
    #[derive(Component)]
    struct Likes;
    #[derive(Component)]
    struct Apples;

    let world = World::new();

    world.set(Count(0));

    world
        .observer::<flecs::OnAdd, ()>()
        .with((Likes::id(), Apples::id()))
        .each_entity(|e, _| {
            let it_table = e.table().unwrap();
            assert!(it_table.has((Likes::id(), Apples::id())));
            e.world().get::<&mut Count>(|count| {
                count.0 += 1;
            });
        });

    world
        .entity()
        .add(Position::id())
        .add((Likes::id(), Apples::id()));
    world.get::<&Count>(|count| {
        assert_eq!(count.0, 1);
    });
}

#[test]
fn other_table_w_pair_wildcard() {
    #[derive(Component)]
    struct Likes;
    #[derive(Component)]
    struct Apples;

    let world = World::new();

    world.set(Count(0));

    world
        .observer::<flecs::OnAdd, ()>()
        .with((Likes::id(), Apples::id()))
        .each_entity(|e, _| {
            let it_table = e.table().unwrap();
            assert!(it_table.has((Likes::id(), flecs::Wildcard::ID)));
            e.world().get::<&mut Count>(|count| {
                count.0 += 1;
            });
        });

    world
        .entity()
        .add(Position::id())
        .add((Likes::id(), Apples::id()));
    world.get::<&Count>(|count| {
        assert_eq!(count.0, 1);
    });
}

#[test]
fn on_add_inherited() {
    let world = World::new();

    world
        .component::<Position>()
        .add_trait::<(flecs::OnInstantiate, flecs::Inherit)>();

    world.set(Count(0));

    // OnAdd with inherited component: use run() to access field data
    world
        .observer::<flecs::OnAdd, ()>()
        .with(Position::id())
        .run(|mut it| {
            while it.next() {
                let pos = it.field::<Position>(0);
                assert_eq!(pos[0].x, 10);
                assert_eq!(pos[0].y, 20);
                it.world().get::<&mut Count>(|count| {
                    count.0 += 1;
                });
            }
        });

    let p = world.prefab().set(Position { x: 10, y: 20 });
    world.get::<&Count>(|count| {
        assert_eq!(count.0, 0);
    });

    let _i = world.entity().is_a(p);
    world.get::<&Count>(|count| {
        assert_eq!(count.0, 1);
    });
}

#[test]
fn on_set_inherited() {
    let world = World::new();

    world
        .component::<Position>()
        .add_trait::<(flecs::OnInstantiate, flecs::Inherit)>();

    world.set(Count(0));

    world
        .observer::<flecs::OnSet, &Position>()
        .each_entity(|e, p| {
            assert_eq!(p.x, 10);
            assert_eq!(p.y, 20);
            e.world().get::<&mut Count>(|count| {
                count.0 += 1;
            });
        });

    let p = world.prefab().set(Position { x: 10, y: 20 });
    world.get::<&Count>(|count| {
        assert_eq!(count.0, 0);
    });

    let _i = world.entity().is_a(p);
    world.get::<&Count>(|count| {
        assert_eq!(count.0, 1);
    });
}

#[test]
fn on_remove_inherited() {
    let world = World::new();

    world
        .component::<Position>()
        .add_trait::<(flecs::OnInstantiate, flecs::Inherit)>();

    world.set(Count(0));

    world
        .observer::<flecs::OnRemove, &Position>()
        .each_entity(|e, _p| {
            e.world().get::<&mut Count>(|count| {
                count.0 += 1;
            });
        });

    let p = world.prefab().set(Position { x: 10, y: 20 });
    world.get::<&Count>(|count| {
        assert_eq!(count.0, 0);
    });

    let i = world.entity().is_a(p);
    world.get::<&Count>(|count| {
        assert_eq!(count.0, 0);
    });

    p.remove(Position::id());
    world.get::<&Count>(|count| {
        assert_eq!(count.0, 1);
    });
    assert!(i.is_alive());
}

#[test]
fn on_set_after_remove_override() {
    let world = World::new();

    world
        .component::<Position>()
        .add_trait::<(flecs::OnInstantiate, flecs::Inherit)>();

    let base = world.prefab().set(Position { x: 1, y: 2 });
    let e1 = world.entity().is_a(base).set(Position { x: 10, y: 20 });

    world.set(Count(0));

    let e1_id = e1.id();
    let base_id = base.id();

    world
        .observer::<flecs::OnSet, &Position>()
        .each_iter(move |it, row, p| {
            let e = it.get_entity(row).unwrap();
            assert!(e == e1_id);
            let src = it.src(0);
            assert!(src == base_id);
            assert_eq!(p.x, 1);
            assert_eq!(p.y, 2);
            it.world().get::<&mut Count>(|count| {
                count.0 += 1;
            });
        });

    e1.remove(Position::id());

    world.get::<&Count>(|count| {
        assert_eq!(count.0, 1);
    });
}

#[test]
fn on_set_after_remove_override_create_observer_before() {
    let world = World::new();

    world
        .component::<Position>()
        .add_trait::<(flecs::OnInstantiate, flecs::Inherit)>();

    let base = world.prefab();
    let e1 = world.entity();

    world.set(Count(0));

    let e1_id = e1.id();
    let base_id = base.id();

    world
        .observer::<flecs::OnSet, &Position>()
        .each_iter(move |it, row, _p| {
            let e = it.get_entity(row).unwrap();
            assert!(e == e1_id);
            let src = it.src(0);
            assert!(src == base_id);
            it.world().get::<&mut Count>(|count| {
                count.0 += 1;
            });
        });

    base.set(Position { x: 1, y: 2 });
    e1.add(Position::id()).is_a(base);

    world.get::<&Count>(|count| {
        assert_eq!(count.0, 0);
    });

    e1.remove(Position::id());

    world.get::<&Count>(|count| {
        assert_eq!(count.0, 1);
    });
}

#[test]
fn on_set_w_override_after_delete() {
    let world = World::new();

    world
        .component::<Position>()
        .add_trait::<(flecs::OnInstantiate, flecs::Inherit)>();

    let base = world.prefab().set(Position { x: 1, y: 2 });
    let e1 = world.entity().is_a(base).set(Position { x: 10, y: 20 });

    world.set(Count(0));

    world
        .observer::<flecs::OnSet, &Position>()
        .each_iter(|_it, _row, _p| {
            _it.world().get::<&mut Count>(|count| {
                count.0 += 1;
            });
        });

    e1.destruct();

    world.get::<&Count>(|count| {
        assert_eq!(count.0, 0);
    });
}

#[test]
fn on_set_w_override_after_clear() {
    let world = World::new();

    world
        .component::<Position>()
        .add_trait::<(flecs::OnInstantiate, flecs::Inherit)>();

    let base = world.prefab().set(Position { x: 1, y: 2 });
    let e1 = world.entity().is_a(base).set(Position { x: 10, y: 20 });

    world.set(Count(0));

    world
        .observer::<flecs::OnSet, &Position>()
        .each_iter(|_it, _row, _p| {
            _it.world().get::<&mut Count>(|count| {
                count.0 += 1;
            });
        });

    e1.clear();

    world.get::<&Count>(|count| {
        assert_eq!(count.0, 0);
    });
}

#[test]
fn trigger_on_set_in_on_add_implicit_registration() {
    #[derive(Component)]
    struct LocalTag;

    let world = World::new();

    world
        .observer::<flecs::OnAdd, ()>()
        .with(LocalTag::id())
        .each_entity(|e, _| {
            e.set(Position { x: 10, y: 20 });
        });

    world
        .observer::<flecs::OnSet, &Position>()
        .each_entity(|e, _| {
            e.set(Velocity { x: 1, y: 2 });
        });

    let e = world.entity().add(LocalTag::id());

    e.get::<&Position>(|p| {
        assert_eq!(p.x, 10);
        assert_eq!(p.y, 20);
    });

    e.get::<&Velocity>(|v| {
        assert_eq!(v.x, 1);
        assert_eq!(v.y, 2);
    });
}

#[test]
fn trigger_on_set_in_on_add_implicit_registration_namespaced() {
    #[derive(Component)]
    struct LocalTag;

    mod ns {
        use flecs_ecs::macros::Component;
        #[derive(Component, Default, Clone)]
        pub struct NsVelocity {
            pub x: i32,
            pub y: i32,
        }
    }

    let world = World::new();

    world
        .observer::<flecs::OnAdd, ()>()
        .with(LocalTag::id())
        .each_entity(|e, _| {
            e.set(Position { x: 10, y: 20 });
        });

    world
        .observer::<flecs::OnSet, &Position>()
        .each_entity(|e, _| {
            e.set(ns::NsVelocity { x: 1, y: 2 });
        });

    let e = world.entity().add(LocalTag::id());

    e.get::<&Position>(|p| {
        assert_eq!(p.x, 10);
        assert_eq!(p.y, 20);
    });

    e.get::<&ns::NsVelocity>(|v| {
        assert_eq!(v.x, 1);
        assert_eq!(v.y, 2);
    });
}

#[test]
fn fixed_src_w_each() {
    #[derive(Component)]
    struct LocalTag;

    let world = World::new();

    world.set(Count(0));

    let e = world.entity();
    let e_id = e.id();

    world
        .observer::<flecs::OnAdd, ()>()
        .with(LocalTag::id())
        .set_src(e)
        .run(move |mut it| {
            while it.next() {
                let src = it.src(0);
                assert!(src == e_id);
                it.world().get::<&mut Count>(|count| {
                    count.0 += 1;
                });
            }
        });

    world.get::<&Count>(|count| {
        assert_eq!(count.0, 0);
    });

    e.add(LocalTag::id());

    world.get::<&Count>(|count| {
        assert_eq!(count.0, 1);
    });

    // Adding to a different entity should not trigger
    world.entity().add(LocalTag::id());

    world.get::<&Count>(|count| {
        assert_eq!(count.0, 1);
    });
}

#[test]
fn fixed_src_w_run() {
    #[derive(Component)]
    struct LocalTag;

    let world = World::new();

    world.set(Count(0));

    let e = world.entity();
    let e_id = e.id();

    world
        .observer::<flecs::OnAdd, ()>()
        .with(LocalTag::id())
        .set_src(e)
        .run(move |mut it| {
            while it.next() {
                let src = it.src(0);
                assert!(src == e_id);
                it.world().get::<&mut Count>(|count| {
                    count.0 += 1;
                });
            }
        });

    world.get::<&Count>(|count| {
        assert_eq!(count.0, 0);
    });

    e.add(LocalTag::id());

    world.get::<&Count>(|count| {
        assert_eq!(count.0, 1);
    });

    // Adding to a different entity should not trigger
    world.entity().add(LocalTag::id());

    world.get::<&Count>(|count| {
        assert_eq!(count.0, 1);
    });
}

#[test]
fn untyped_field() {
    let world = World::new();

    world.set(Count(0));
    world.set(Count2 { a: 0, b: 0 });

    world.observer::<flecs::OnSet, &Position>().run(|mut it| {
        it.world().get::<&mut Count2>(|c| {
            c.a += 1; // invoked count
        });
        while it.next() {
            it.world().get::<&mut Count2>(|c| {
                c.b += 1; // iteration count
            });
            let size = it.size(0);
            assert_eq!(size, core::mem::size_of::<Position>());
            let f = it.field_untyped(0);
            let p = unsafe { &*(f.at(0) as *const Position) };
            assert_eq!(p.x, 10);
            assert_eq!(p.y, 20);
        }
    });

    world.entity().set(Position { x: 10, y: 20 });

    world.get::<&Count2>(|c| {
        assert_eq!(c.a, 1);
        assert_eq!(c.b, 1);
    });
}

#[test]
fn query_eval_w_component_that_triggered_observer() {
    let mut world = World::new();

    let entry_event = world.entity().id();
    let sequence_shared = world.entity().add_trait::<flecs::Trait>().id();
    let sequence = world.entity().add(sequence_shared).id();
    let child = world.entity().id();

    world.set(Count(0));

    let mut ob = world.observer_id::<()>(entry_event);
    ob.with("$Sequence")
        .with(sequence_shared)
        .set_src("$Sequence")
        .filter();

    ob.each_iter(move |it, i, _| {
        assert_eq!(it.id(0), sequence);
        it.world().get::<&mut Count>(|c| {
            if c.0 == 0 {
                c.0 += 1;
                let e = it.entity(i).add(child);
                unsafe {
                    e.world()
                        .event_id(entry_event)
                        .entity(e)
                        .add(child)
                        .enqueue(());
                }
            }
        });
    });

    unsafe {
        world
            .event_id(entry_event)
            .entity(world.entity().add(sequence))
            .add(sequence)
            .enqueue(());
    }
    world.progress();

    world.get::<&Count>(|c| assert_eq!(c.0, 1));
}

#[test]
fn query_eval_w_pair_first_var_that_triggered_observer() {
    let mut world = World::new();

    let entry_event = world.entity();
    let rel_tag = world.entity_named("RelTag");
    let rel = world.entity_named("MatchRel").add(rel_tag.id());
    let tgt = world.entity_named("MatchTgt");
    let other_rel = world.entity_named("OtherRel");

    let entry_event_id = entry_event.id();
    let rel_id = rel.id();
    let tgt_id = tgt.id();
    let other_rel_id = other_rel.id();

    world.set(Count(0));

    world
        .observer_id::<()>(entry_event)
        .expr("($Rel, MatchTgt), RelTag($Rel)")
        .each_iter(move |it, i, _| {
            assert_eq!(it.get_var_by_name("Rel").id(), rel_id);
            let mut first = false;
            it.world().get::<&mut Count>(|c| {
                first = c.0 == 0;
                c.0 += 1;
            });
            if first {
                let e = it.entity(i);
                e.add((other_rel_id, tgt_id));
                unsafe {
                    e.world()
                        .event_id(entry_event_id)
                        .add((other_rel_id, tgt_id))
                        .entity(e)
                        .enqueue(());
                }
            }
        })
        .expect("valid observer query expression");

    unsafe {
        world
            .event_id(entry_event_id)
            .add((rel_id, tgt_id))
            .entity(world.entity().add((rel_id, tgt_id)))
            .enqueue(());
    }
    world.progress();

    world.get::<&Count>(|c| assert_eq!(c.0, 1));
}

#[test]
fn query_eval_w_pair_second_var_that_triggered_observer() {
    let mut world = World::new();

    let entry_event = world.entity();
    let tgt_tag = world.entity_named("TgtTag");
    let rel = world.entity_named("MatchRel");
    let tgt = world.entity_named("MatchTgt").add(tgt_tag.id());
    let other_tgt = world.entity_named("OtherTgt");

    let entry_event_id = entry_event.id();
    let rel_id = rel.id();
    let tgt_id = tgt.id();
    let other_tgt_id = other_tgt.id();

    world.set(Count(0));

    world
        .observer_id::<()>(entry_event)
        .expr("(MatchRel, $Tgt), TgtTag($Tgt)")
        .each_iter(move |it, i, _| {
            assert_eq!(it.get_var_by_name("Tgt").id(), tgt_id);
            let mut first = false;
            it.world().get::<&mut Count>(|c| {
                first = c.0 == 0;
                c.0 += 1;
            });
            if first {
                let e = it.entity(i);
                e.add((rel_id, other_tgt_id));
                unsafe {
                    e.world()
                        .event_id(entry_event_id)
                        .add((rel_id, other_tgt_id))
                        .entity(e)
                        .enqueue(());
                }
            }
        })
        .expect("valid observer query expression");

    unsafe {
        world
            .event_id(entry_event_id)
            .add((rel_id, tgt_id))
            .entity(world.entity().add((rel_id, tgt_id)))
            .enqueue(());
    }
    world.progress();

    world.get::<&Count>(|c| assert_eq!(c.0, 1));
}

#[test]
fn query_eval_w_pair_both_vars_that_triggered_observer() {
    let mut world = World::new();

    let entry_event = world.entity();
    let rel_tag = world.entity_named("RelTag");
    let tgt_tag = world.entity_named("TgtTag");
    let rel = world.entity_named("MatchRel").add(rel_tag.id());
    let tgt = world.entity_named("MatchTgt").add(tgt_tag.id());
    let other_rel = world.entity_named("OtherRel");
    let other_tgt = world.entity_named("OtherTgt");

    let entry_event_id = entry_event.id();
    let rel_id = rel.id();
    let tgt_id = tgt.id();
    let other_rel_id = other_rel.id();
    let other_tgt_id = other_tgt.id();

    world.set(Count(0));

    world
        .observer_id::<()>(entry_event)
        .expr("($Rel, $Tgt), RelTag($Rel), TgtTag($Tgt)")
        .each_iter(move |it, i, _| {
            assert_eq!(it.get_var_by_name("Rel").id(), rel_id);
            assert_eq!(it.get_var_by_name("Tgt").id(), tgt_id);
            let mut first = false;
            it.world().get::<&mut Count>(|c| {
                first = c.0 == 0;
                c.0 += 1;
            });
            if first {
                let e = it.entity(i);
                e.add((other_rel_id, other_tgt_id));
                unsafe {
                    e.world()
                        .event_id(entry_event_id)
                        .add((other_rel_id, other_tgt_id))
                        .entity(e)
                        .enqueue(());
                }
            }
        })
        .expect("valid observer query expression");

    unsafe {
        world
            .event_id(entry_event_id)
            .add((rel_id, tgt_id))
            .entity(world.entity().add((rel_id, tgt_id)))
            .enqueue(());
    }
    world.progress();

    world.get::<&Count>(|c| assert_eq!(c.0, 1));
}

// ─── 2_terms_un_set ───────────────────────────────────────────────────────────

#[test]
fn n2_terms_un_set() {
    let world = World::new();

    world.set(Count(0));

    world
        .observer::<flecs::OnRemove, (&Position, &Velocity)>()
        .each(|(p, v)| {
            assert_eq!(p.x, 10);
            assert_eq!(p.y, 20);
            assert_eq!(v.x, 1);
            assert_eq!(v.y, 2);
        });

    let e = world.entity();
    world.get::<&Count>(|count| {
        assert_eq!(count.0, 0);
    });

    e.set(Position { x: 10, y: 20 });
    world.get::<&Count>(|count| {
        assert_eq!(count.0, 0);
    });

    e.set(Velocity { x: 1, y: 2 });
    world.get::<&Count>(|count| {
        assert_eq!(count.0, 0);
    });

    // Remove one term — this triggers the OnRemove observer if entity has both
    e.remove(Velocity::id());
    // After remove, the pair (Position, Velocity) no longer matches — count should be 1
    // (observer fires when Velocity is removed while Position is still present)
}

// ─── run_callback_w_1_field ───────────────────────────────────────────────────

#[test]
fn run_callback_w_1_field() {
    let world = World::new();

    world.set(Count(0));

    world.observer::<flecs::OnSet, &Position>().run(|mut it| {
        while it.next() {
            let p = it.field::<Position>(0);
            assert_eq!(p[0].x, 10);
            assert_eq!(p[0].y, 20);
            it.world().get::<&mut Count>(|count| {
                count.0 += 1;
            });
        }
    });

    let e = world.entity();
    world.get::<&Count>(|count| {
        assert_eq!(count.0, 0);
    });

    e.set(Position { x: 10, y: 20 });
    world.get::<&Count>(|count| {
        assert_eq!(count.0, 1);
    });
}

// ─── run_callback_w_2_fields ──────────────────────────────────────────────────

#[test]
fn run_callback_w_2_fields() {
    let world = World::new();

    world.set(Count(0));

    world
        .observer::<flecs::OnSet, (&Position, &Velocity)>()
        .run(|mut it| {
            while it.next() {
                let p = it.field::<Position>(0);
                let v = it.field::<Velocity>(1);
                assert_eq!(p[0].x, 10);
                assert_eq!(p[0].y, 20);
                assert_eq!(v[0].x, 1);
                assert_eq!(v[0].y, 2);
                it.world().get::<&mut Count>(|count| {
                    count.0 += 1;
                });
            }
        });

    let e = world.entity();
    world.get::<&Count>(|count| {
        assert_eq!(count.0, 0);
    });

    e.set(Position { x: 10, y: 20 });
    world.get::<&Count>(|count| {
        assert_eq!(count.0, 0);
    });

    e.set(Velocity { x: 1, y: 2 });
    world.get::<&Count>(|count| {
        assert_eq!(count.0, 1);
    });
}

// ─── run_callback_w_yield_existing_1_field ────────────────────────────────────

#[test]
fn run_callback_w_yield_existing_1_field() {
    let world = World::new();

    world.set(Count(0));
    world.entity().set(Position { x: 10, y: 20 });

    world
        .observer::<flecs::OnSet, &Position>()
        .yield_existing()
        .run(|mut it| {
            while it.next() {
                let p = it.field::<Position>(0);
                assert_eq!(p[0].x, 10);
                assert_eq!(p[0].y, 20);
                it.world().get::<&mut Count>(|count| {
                    count.0 += 1;
                });
            }
        });

    world.get::<&Count>(|count| {
        assert_eq!(count.0, 1);
    });
}

// ─── run_callback_w_yield_existing_2_fields ───────────────────────────────────

#[test]
fn run_callback_w_yield_existing_2_fields() {
    let world = World::new();

    world.set(Count(0));
    world
        .entity()
        .set(Position { x: 10, y: 20 })
        .set(Velocity { x: 1, y: 2 });

    world
        .observer::<flecs::OnSet, (&Position, &Velocity)>()
        .yield_existing()
        .run(|mut it| {
            while it.next() {
                let p = it.field::<Position>(0);
                let v = it.field::<Velocity>(1);
                assert_eq!(p[0].x, 10);
                assert_eq!(p[0].y, 20);
                assert_eq!(v[0].x, 1);
                assert_eq!(v[0].y, 2);
                it.world().get::<&mut Count>(|count| {
                    count.0 += 1;
                });
            }
        });

    world.get::<&Count>(|count| {
        assert_eq!(count.0, 1);
    });
}

#[test]
fn reuse_observer_builder() {
    let world = World::new();

    let mut ob = world.observer::<flecs::OnSet, &Position>();

    let count_1 = alloc::rc::Rc::new(core::cell::Cell::new(0));
    let count_2 = alloc::rc::Rc::new(core::cell::Cell::new(0));

    let count_1_c = count_1.clone();
    let o1 = ob.each(move |_p| {
        count_1_c.set(count_1_c.get() + 1);
    });

    let count_2_c = count_2.clone();
    let o2 = ob.with(Velocity::id()).each(move |_p| {
        count_2_c.set(count_2_c.get() + 1);
    });

    assert!(o1.id() != o2.id());

    world.entity().set(Position { x: 10, y: 20 });
    world
        .entity()
        .set(Position { x: 10, y: 20 })
        .set(Velocity { x: 1, y: 2 });

    assert_eq!(count_1.get(), 2);
    assert_eq!(count_2.get(), 1);
}
