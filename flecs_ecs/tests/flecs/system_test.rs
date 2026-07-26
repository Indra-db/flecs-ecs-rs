#![allow(clippy::float_cmp)]
#![allow(dead_code)]
#![allow(clippy::std_instead_of_alloc)]
#![allow(clippy::std_instead_of_core)]

use core::ffi::c_void;

use flecs_ecs::core::*;
use flecs_ecs::macros::*;
use timer::TimerAPI;

use crate::common_test::*;

#[derive(Component)]
struct LastVal(i32);

#[test]
fn iter() {
    let mut world = World::new();

    let entity = world
        .entity()
        .set(Position { x: 10, y: 20 })
        .set(Velocity { x: 1, y: 2 })
        .id();

    world
        .system::<(&mut Position, &mut Velocity)>()
        .run(|mut it| {
            while it.next() {
                let mut p = it.field_mut::<Position>(0);
                let v = it.field::<Velocity>(1);
                for i in it.iter() {
                    p[i].x += v[i].x;
                    p[i].y += v[i].y;
                }
            }
        });

    world.progress();

    world.entity_from_id(entity).get::<(&Position, &Velocity)>(|(p, v)| {
        assert_eq!(p.x, 11);
        assert_eq!(p.y, 22);
        assert_eq!(v.x, 1);
        assert_eq!(v.y, 2);
    });
}

#[test]
fn iter_macro() {
    let mut world = World::new();

    let entity = world
        .entity()
        .set(Position { x: 10, y: 20 })
        .set(Velocity { x: 1, y: 2 })
        .id();

    system!(world, &mut Position, &Velocity).run(|mut it| {
        while it.next() {
            let mut p = it.field_mut::<Position>(0);
            let v = it.field::<Velocity>(1);
            for i in it.iter() {
                p[i].x += v[i].x;
                p[i].y += v[i].y;
            }
        }
    });

    world.progress();

    world.entity_from_id(entity).get::<(&Position, &Velocity)>(|(p, v)| {
        assert_eq!(p.x, 11);
        assert_eq!(p.y, 22);
        assert_eq!(v.x, 1);
        assert_eq!(v.y, 2);
    });
}

#[test]
fn iter_const() {
    let mut world = World::new();

    let entity = world
        .entity()
        .set(Position { x: 10, y: 20 })
        .set(Velocity { x: 1, y: 2 })
        .id();

    world.system::<(&mut Position, &Velocity)>().run(|mut it| {
        while it.next() {
            let mut p = it.field_mut::<Position>(0);
            let v = it.field::<Velocity>(1);
            for i in it.iter() {
                p[i].x += v[i].x;
                p[i].y += v[i].y;
            }
        }
    });

    world.progress();

    world.entity_from_id(entity).get::<(&Position, &Velocity)>(|(p, v)| {
        assert_eq!(p.x, 11);
        assert_eq!(p.y, 22);
        assert_eq!(v.x, 1);
        assert_eq!(v.y, 2);
    });
}

#[test]
fn iter_shared() {
    let mut world = World::new();

    world
        .component::<Position>()
        .add_trait::<(flecs::OnInstantiate, flecs::Inherit)>();
    world
        .component::<Velocity>()
        .add_trait::<(flecs::OnInstantiate, flecs::Inherit)>();

    let base = world.entity().set(Velocity { x: 1, y: 2 });

    let e1 = world
        .entity()
        .set(Position { x: 10, y: 20 })
        .add((flecs::IsA::ID, base))
        .id();

    let e2 = world
        .entity()
        .set(Position { x: 10, y: 20 })
        .set(Velocity { x: 3, y: 4 })
        .id();

    world
        .system::<&mut Position>()
        .expr("Velocity(self|up IsA)")
        .run(|mut it| {
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
        })
        .expect("valid system query expression");

    world.progress();

    world.entity_from_id(e1).get::<&Position>(|p| {
        assert_eq!(p.x, 11);
        assert_eq!(p.y, 22);
    });

    world.entity_from_id(e2).get::<&Position>(|p| {
        assert_eq!(p.x, 13);
        assert_eq!(p.y, 24);
    });
}

#[test]
fn iter_optional() {
    let mut world = World::new();
    world.component_named::<Mass>("Mass");

    let e1 = world
        .entity()
        .set(Position { x: 10, y: 20 })
        .set(Velocity { x: 1, y: 2 })
        .set(Mass { value: 1 })
        .id();

    let e2 = world
        .entity()
        .set(Position { x: 30, y: 40 })
        .set(Velocity { x: 3, y: 4 })
        .set(Mass { value: 1 })
        .id();

    let e3 = world.entity().set(Position { x: 50, y: 60 }).id();

    let e4 = world.entity().set(Position { x: 70, y: 80 }).id();

    world
        .system::<(&mut Position, Option<&mut Velocity>, Option<&mut Mass>)>()
        .run(|mut it| {
            while it.next() {
                let mut p = it.field_mut::<Position>(0);

                if it.is_set(1) && it.is_set(2) {
                    let v = it.field::<Velocity>(1);
                    let m = it.field::<Mass>(2);
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

    world.progress();

    world.entity_from_id(e1).get::<&Position>(|p| {
        assert_eq!(p.x, 11);
        assert_eq!(p.y, 22);
    });

    world.entity_from_id(e2).get::<&Position>(|p| {
        assert_eq!(p.x, 33);
        assert_eq!(p.y, 44);
    });

    world.entity_from_id(e3).get::<&Position>(|p| {
        assert_eq!(p.x, 51);
        assert_eq!(p.y, 61);
    });

    world.entity_from_id(e4).get::<&Position>(|p| {
        assert_eq!(p.x, 71);
        assert_eq!(p.y, 81);
    });
}

#[test]
fn each() {
    let mut world = World::new();

    let entity = world
        .entity()
        .set(Position { x: 10, y: 20 })
        .set(Velocity { x: 1, y: 2 })
        .id();

    world
        .system::<(&mut Position, &mut Velocity)>()
        .each_entity(|_e, (p, v)| {
            p.x += v.x;
            p.y += v.y;
        });

    world.progress();

    world.entity_from_id(entity).get::<&Position>(|p| {
        assert_eq!(p.x, 11);
        assert_eq!(p.y, 22);
    });
}

#[test]
fn each_const() {
    let mut world = World::new();

    let entity = world
        .entity()
        .set(Position { x: 10, y: 20 })
        .set(Velocity { x: 1, y: 2 })
        .id();

    world
        .system::<(&mut Position, &Velocity)>()
        .each_entity(|_e, (p, v)| {
            p.x += v.x;
            p.y += v.y;
        });

    world.progress();

    world.entity_from_id(entity).get::<&Position>(|p| {
        assert_eq!(p.x, 11);
        assert_eq!(p.y, 22);
    });
}

#[test]
fn each_shared() {
    let mut world = World::new();

    let base = world.entity().set(Velocity { x: 1, y: 2 });

    let e1 = world
        .entity()
        .set(Position { x: 10, y: 20 })
        .add((flecs::IsA::ID, base))
        .id();

    let e2 = world
        .entity()
        .set(Position { x: 10, y: 20 })
        .set(Velocity { x: 3, y: 4 })
        .id();

    world
        .system::<(&mut Position, &Velocity)>()
        .each_entity(|_e, (p, v)| {
            p.x += v.x;
            p.y += v.y;
        });

    world.progress();

    world.entity_from_id(e1).get::<&Position>(|p| {
        assert_eq!(p.x, 11);
        assert_eq!(p.y, 22);
    });

    world.entity_from_id(e2).get::<&Position>(|p| {
        assert_eq!(p.x, 13);
        assert_eq!(p.y, 24);
    });
}

#[test]
fn each_optional() {
    let mut world = World::new();
    world.component_named::<Mass>("Mass");

    let e1 = world
        .entity()
        .set(Position { x: 10, y: 20 })
        .set(Velocity { x: 1, y: 2 })
        .set(Mass { value: 1 })
        .id();

    let e2 = world
        .entity()
        .set(Position { x: 30, y: 40 })
        .set(Velocity { x: 3, y: 4 })
        .set(Mass { value: 1 })
        .id();

    let e3 = world.entity().set(Position { x: 50, y: 60 }).id();

    let e4 = world.entity().set(Position { x: 70, y: 80 }).id();

    world
        .system::<(&mut Position, Option<&mut Velocity>, Option<&mut Mass>)>()
        .each_entity(|_e, (p, v, m)| {
            if let Some(v) = v
                && let Some(m) = m
            {
                p.x += v.x * m.value;
                p.y += v.y * m.value;
            } else {
                p.x += 1;
                p.y += 1;
            }
        });

    world.progress();

    world.entity_from_id(e1).get::<&Position>(|p| {
        assert_eq!(p.x, 11);
        assert_eq!(p.y, 22);
    });
    world.entity_from_id(e1).get::<&Position>(|p| {
        assert_eq!(p.x, 11);
        assert_eq!(p.y, 22);
    });

    world.entity_from_id(e2).get::<&Position>(|p| {
        assert_eq!(p.x, 33);
        assert_eq!(p.y, 44);
    });

    world.entity_from_id(e3).get::<&Position>(|p| {
        assert_eq!(p.x, 51);
        assert_eq!(p.y, 61);
    });

    world.entity_from_id(e4).get::<&Position>(|p| {
        assert_eq!(p.x, 71);
        assert_eq!(p.y, 81);
    });
}

#[test]
fn signature() {
    let mut world = World::new();

    let entity = world
        .entity()
        .set(Position { x: 10, y: 20 })
        .set(Velocity { x: 1, y: 2 })
        .id();

    world
        .system::<()>()
        .expr("Position, Velocity")
        .run(|mut it| {
            while it.next() {
                let mut p = it.field_mut::<Position>(0);
                let v = it.field::<Velocity>(1);

                for i in it.iter() {
                    p[i].x += v[i].x;
                    p[i].y += v[i].y;
                }
            }
        })
        .expect("valid system query expression");

    world.progress();

    world.entity_from_id(entity).get::<&Position>(|p| {
        assert_eq!(p.x, 11);
        assert_eq!(p.y, 22);
    });

    world.entity_from_id(entity).get::<&Velocity>(|v| {
        assert_eq!(v.x, 1);
        assert_eq!(v.y, 2);
    });
}

#[test]
fn signature_const() {
    let mut world = World::new();

    let entity = world
        .entity()
        .set(Position { x: 10, y: 20 })
        .set(Velocity { x: 1, y: 2 })
        .id();

    world
        .system::<()>()
        .expr("Position, [in] Velocity")
        .run(|mut it| {
            while it.next() {
                let mut p = it.field_mut::<Position>(0);
                let v = it.field::<Velocity>(1);

                for i in it.iter() {
                    p[i].x += v[i].x;
                    p[i].y += v[i].y;
                }
            }
        })
        .expect("valid system query expression");

    world.progress();

    world.entity_from_id(entity).get::<&Position>(|p| {
        assert_eq!(p.x, 11);
        assert_eq!(p.y, 22);
    });

    world.entity_from_id(entity).get::<&Velocity>(|v| {
        assert_eq!(v.x, 1);
        assert_eq!(v.y, 2);
    });
}

#[test]
fn signature_shared() {
    let mut world = World::new();

    world
        .component::<Position>()
        .add_trait::<(flecs::OnInstantiate, flecs::Inherit)>();
    world
        .component::<Velocity>()
        .add_trait::<(flecs::OnInstantiate, flecs::Inherit)>();

    let base = world.entity().set(Velocity { x: 1, y: 2 });

    let e1 = world
        .entity()
        .set(Position { x: 10, y: 20 })
        .add((flecs::IsA::ID, base))
        .id();

    let e2 = world
        .entity()
        .set(Position { x: 10, y: 20 })
        .set(Velocity { x: 3, y: 4 })
        .id();

    world
        .system::<()>()
        .expr("Position, [in] Velocity(self|up IsA)")
        .run(|mut it| {
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
        })
        .expect("valid system query expression");

    world.progress();

    world.entity_from_id(e1).get::<&Position>(|p| {
        assert_eq!(p.x, 11);
        assert_eq!(p.y, 22);
    });

    world.entity_from_id(e2).get::<&Position>(|p| {
        assert_eq!(p.x, 13);
        assert_eq!(p.y, 24);
    });
}

#[test]
fn signature_optional() {
    let mut world = World::new();
    world.component_named::<Mass>("Mass");

    let e1 = world
        .entity()
        .set(Position { x: 10, y: 20 })
        .set(Velocity { x: 1, y: 2 })
        .set(Mass { value: 1 })
        .id();

    let e2 = world
        .entity()
        .set(Position { x: 30, y: 40 })
        .set(Velocity { x: 3, y: 4 })
        .set(Mass { value: 1 })
        .id();

    let e3 = world.entity().set(Position { x: 50, y: 60 }).id();

    let e4 = world.entity().set(Position { x: 70, y: 80 }).id();

    world
        .system::<()>()
        .expr("Position, ?Velocity, ?Mass")
        .run(|mut it| {
            while it.next() {
                let mut p = it.field_mut::<Position>(0);

                if it.is_set(1) && it.is_set(2) {
                    let v = it.field::<Velocity>(1);
                    let m = it.field::<Mass>(2);
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
        })
        .expect("valid system query expression");

    world.progress();

    world.entity_from_id(e1).get::<&Position>(|p| {
        assert_eq!(p.x, 11);
        assert_eq!(p.y, 22);
    });

    world.entity_from_id(e2).get::<&Position>(|p| {
        assert_eq!(p.x, 33);
        assert_eq!(p.y, 44);
    });

    world.entity_from_id(e3).get::<&Position>(|p| {
        assert_eq!(p.x, 51);
        assert_eq!(p.y, 61);
    });

    world.entity_from_id(e4).get::<&Position>(|p| {
        assert_eq!(p.x, 71);
        assert_eq!(p.y, 81);
    });
}

#[test]
fn copy_name_on_create() {
    let world = World::new();

    let mut name = "hello";

    let system_1 = world
        .system_named::<&mut Position>(name)
        .run(|mut it| while it.next() {});

    name = "world";
    let system_2 = world
        .system_named::<&mut Position>(name)
        .run(|mut it| while it.next() {});

    assert_ne!(system_1.id(), system_2.id());
}

#[test]
fn nested_system() {
    let world = World::new();

    let system_1 = world
        .system_named::<&mut Position>("foo::bar")
        .run(|mut it| while it.next() {});

    assert_eq!(system_1.name(), "bar");

    let e = world.lookup("foo");
    assert_ne!(e.id(), 0);
    assert_eq!(e.name(), "foo");

    let se = e.lookup("bar");
    assert_ne!(se.id(), 0);
    assert_eq!(se.name(), "bar");
}

#[test]
fn empty_signature() {
    let mut world = World::new();

    world.set(Count(0));

    world.system::<()>().run(|mut it| {
        let world = it.world();
        while it.next() {
            world.get::<&mut Count>(|c| {
                c.0 += 1;
            });
        }
    });

    world.progress();

    world.get::<&Count>(|c| {
        assert_eq!(c.0, 1);
    });
}

#[test]
fn iter_tag() {
    let mut world = World::new();

    world.set(Count(0));

    world.system::<()>().with(&TagA::id()).run(|mut it| {
        let world = it.world();
        while it.next() {
            world.get::<&mut Count>(|c| {
                c.0 += 1;
            });
        }
    });

    world.entity().add(TagA::id());

    world.progress();

    world.get::<&Count>(|c| {
        assert_eq!(c.0, 1);
    });
}

#[test]
fn each_tag() {
    let mut world = World::new();

    world.set(Count(0));

    world.system::<()>().with(&TagA::id()).run(|mut it| {
        while it.next() {
            for _ in it.iter() {
                let world = it.world();
                world.get::<&mut Count>(|c| {
                    c.0 += 1;
                });
            }
        }
    });

    world.entity().add(TagA::id());

    world.progress();

    world.get::<&Count>(|c| {
        assert_eq!(c.0, 1);
    });
}

#[test]
fn set_interval() {
    let world = World::new();

    let sys = world.system::<()>().kind(0).set_interval(1.0).run(|_it| {});

    let i = sys.interval();
    assert_eq!(i, 1.0_f32);

    let sys = sys.set_interval(2.0);

    let i = sys.interval();
    assert_eq!(i, 2.0_f32);
}

#[test]
fn order_by_type() {
    let world = World::new();

    world.entity().set(Position { x: 3, y: 0 });
    world.entity().set(Position { x: 1, y: 0 });
    world.entity().set(Position { x: 5, y: 0 });
    world.entity().set(Position { x: 2, y: 0 });
    world.entity().set(Position { x: 4, y: 0 });

    world.set(LastVal(0));
    world.set(Count(0));

    let sys = world
        .system::<&Position>()
        .order_by::<Position>(|_e1, p1: &Position, _e2, p2: &Position| -> i32 {
            (p1.x > p2.x) as i32 - (p1.x < p2.x) as i32
        })
        .each_entity(|e, p| {
            let world = e.world();
            world.get::<&mut LastVal>(|last_val| {
                assert!(p.x > last_val.0);
                last_val.0 = p.x;
            });
            world.get::<&mut Count>(|c| {
                c.0 += 1;
            });
        });

    sys.run();

    world.get::<&Count>(|c| {
        assert_eq!(c.0, 5);
    });
}

#[test]
fn order_by_id() {
    let world = World::new();

    let pos = world.component::<Position>();

    world.entity().set(Position { x: 3, y: 0 });
    world.entity().set(Position { x: 1, y: 0 });
    world.entity().set(Position { x: 5, y: 0 });
    world.entity().set(Position { x: 2, y: 0 });
    world.entity().set(Position { x: 4, y: 0 });

    world.set(LastVal(0));
    world.set(Count(0));

    let sys = world
        .system::<&Position>()
        .order_by_id(
            pos,
            |_e1, p1: *const c_void, _e2, p2: *const c_void| -> i32 {
                let p1 = unsafe { &*(p1 as *const Position) };
                let p2 = unsafe { &*(p2 as *const Position) };
                (p1.x > p2.x) as i32 - (p1.x < p2.x) as i32
            },
        )
        .each_entity(|e, p| {
            let world = e.world();
            world.get::<&mut LastVal>(|last_val| {
                assert!(p.x > last_val.0);
                last_val.0 = p.x;
            });
            world.get::<&mut Count>(|c| {
                c.0 += 1;
            });
        });

    sys.run();

    world.get::<&Count>(|c| {
        assert_eq!(c.0, 5);
    });
}

#[test]
fn order_by_type_after_create() {
    let world = World::new();

    world.entity().set(Position { x: 3, y: 0 });
    world.entity().set(Position { x: 1, y: 0 });
    world.entity().set(Position { x: 5, y: 0 });
    world.entity().set(Position { x: 2, y: 0 });
    world.entity().set(Position { x: 4, y: 0 });

    world.set(LastVal(0));
    world.set(Count(0));

    let sys = world
        .system::<&Position>()
        .order_by::<Position>(|_e1, p1: &Position, _e2, p2: &Position| -> i32 {
            (p1.x > p2.x) as i32 - (p1.x < p2.x) as i32
        })
        .each_entity(|e, p| {
            let world = e.world();
            world.get::<&mut LastVal>(|last_val| {
                assert!(p.x > last_val.0);
                last_val.0 = p.x;
            });
            world.get::<&mut Count>(|c| {
                c.0 += 1;
            });
        });

    sys.run();

    world.get::<&Count>(|c| {
        assert_eq!(c.0, 5);
    });
}

// SAFETY_MARKER_ORDER_BY_ID_AFTER_CREATE
#[test]
fn order_by_id_after_create() {
    let world = World::new();

    let pos = world.component::<Position>();

    world.entity().set(Position { x: 3, y: 0 });
    world.entity().set(Position { x: 1, y: 0 });
    world.entity().set(Position { x: 5, y: 0 });
    world.entity().set(Position { x: 2, y: 0 });
    world.entity().set(Position { x: 4, y: 0 });

    world.set(LastVal(0));
    world.set(Count(0));

    let sys = world
        .system::<&Position>()
        .order_by_id(
            pos,
            |_e1, p1: *const c_void, _e2, p2: *const c_void| -> i32 {
                let p1 = unsafe { &*(p1 as *const Position) };
                let p2 = unsafe { &*(p2 as *const Position) };
                (p1.x > p2.x) as i32 - (p1.x < p2.x) as i32
            },
        )
        .each_entity(|e, p| {
            let world = e.world();
            world.get::<&mut LastVal>(|last_val| {
                assert!(p.x > last_val.0);
                last_val.0 = p.x;
            });
            world.get::<&mut Count>(|c| {
                c.0 += 1;
            });
        });

    sys.run();

    world.get::<&Count>(|c| {
        assert_eq!(c.0, 5);
    });
}

#[test]
fn get_query() {
    let world = World::new();

    world.entity().set(Position { x: 0, y: 0 });
    world.entity().set(Position { x: 1, y: 0 });
    world.entity().set(Position { x: 2, y: 0 });

    world.set(Count(0));

    let sys = world.system::<&Position>().each_entity(|_e, _p| {
        // Not used
    });

    let q = sys.query();

    q.run(|mut it| {
        let world = it.world();
        while it.next() {
            let pos = it.field::<Position>(0);
            for i in it.iter() {
                assert_eq!(<FieldIndex as Into<usize>>::into(i) as i32, pos[i].x);
                world.get::<&mut Count>(|c| {
                    c.0 += 1;
                });
            }
        }
    });

    world.get::<&Count>(|c| {
        assert_eq!(c.0, 3);
    });
}

#[test]
fn add_from_each() {
    let mut world = World::new();

    let e1 = world.entity().set(Position { x: 0, y: 0 }).id();
    let e2 = world.entity().set(Position { x: 1, y: 0 }).id();
    let e3 = world.entity().set(Position { x: 2, y: 0 }).id();

    world.system::<&Position>().each_entity(|e, _p| {
        e.add(Velocity::id());
        // Add is deferred
        assert!(!e.has(Velocity::id()));
    });

    world.progress();

    assert!(world.entity_from_id(e1).has(Velocity::id()));
    assert!(world.entity_from_id(e2).has(Velocity::id()));
    assert!(world.entity_from_id(e3).has(Velocity::id()));
}

#[test]
fn delete_from_each() {
    let mut world = World::new();

    let e1 = world.entity().set(Position { x: 0, y: 0 }).id();
    let e2 = world.entity().set(Position { x: 1, y: 0 }).id();
    let e3 = world.entity().set(Position { x: 2, y: 0 }).id();

    world.system::<&Position>().each_entity(|e, _p| {
        e.destruct();
        // Delete is deferred
        assert!(e.is_alive());
    });

    world.progress();

    assert!(!world.entity_from_id(e1).is_alive());
    assert!(!world.entity_from_id(e2).is_alive());
    assert!(!world.entity_from_id(e3).is_alive());
}

#[test]
fn add_from_each_world_handle() {
    let mut world = World::new();

    let e1 = world
        .entity()
        .set(EntityRef {
            value: world.entity().id(),
        })
        .id();
    let e2 = world
        .entity()
        .set(EntityRef {
            value: world.entity().id(),
        })
        .id();
    let e3 = world
        .entity()
        .set(EntityRef {
            value: world.entity().id(),
        })
        .id();

    world.system::<&EntityRef>().each_entity(|e, c| {
        let world = e.world();
        let e = world.entity_from_id(c.value);
        e.mut_stage_of(e).add(Position::id());
    });

    world.progress();

    world.entity_from_id(e1).get::<&EntityRef>(|c| {
        let e = world.entity_from_id(c.value);
        assert!(e.has(Position::id()));
    });

    world.entity_from_id(e2).get::<&EntityRef>(|c| {
        let e = world.entity_from_id(c.value);
        assert!(e.has(Position::id()));
    });

    world.entity_from_id(e3).get::<&EntityRef>(|c| {
        let e = world.entity_from_id(c.value);
        assert!(e.has(Position::id()));
    });
}

#[test]
fn new_from_each() {
    let mut world = World::new();

    let e1 = world.entity().set(Position { x: 0, y: 0 }).id();
    let e2 = world.entity().set(Position { x: 0, y: 0 }).id();
    let e3 = world.entity().set(Position { x: 0, y: 0 }).id();

    world.system::<&Position>().each_entity(|e, _p| {
        e.set(EntityRef {
            value: e.world().entity().add(Velocity::id()).id(),
        });
    });

    world.progress();

    assert!(world.entity_from_id(e1).has(EntityRef::id()));
    assert!(world.entity_from_id(e2).has(EntityRef::id()));
    assert!(world.entity_from_id(e3).has(EntityRef::id()));

    world.entity_from_id(e1).get::<&EntityRef>(|c| {
        assert!(world.entity_from_id(c.value).has(Velocity::id()));
    });

    world.entity_from_id(e2).get::<&EntityRef>(|c| {
        assert!(world.entity_from_id(c.value).has(Velocity::id()));
    });

    world.entity_from_id(e3).get::<&EntityRef>(|c| {
        assert!(world.entity_from_id(c.value).has(Velocity::id()));
    });
}

#[test]
fn add_from_iter() {
    let mut world = World::new();

    let e1 = world.entity().set(Position { x: 0, y: 0 }).id();
    let e2 = world.entity().set(Position { x: 1, y: 0 }).id();
    let e3 = world.entity().set(Position { x: 2, y: 0 }).id();

    world.system::<&Position>().run(|mut it| {
        while it.next() {
            for i in it.iter() {
                it.get_entity(i).unwrap().add(Velocity::id());
                assert!(!it.get_entity(i).unwrap().has(Velocity::id()));
            }
        }
    });

    world.progress();

    assert!(world.entity_from_id(e1).has(Velocity::id()));
    assert!(world.entity_from_id(e2).has(Velocity::id()));
    assert!(world.entity_from_id(e3).has(Velocity::id()));
}

#[test]
fn delete_from_iter() {
    let mut world = World::new();

    let e1 = world.entity().set(Position { x: 0, y: 0 }).id();
    let e2 = world.entity().set(Position { x: 1, y: 0 }).id();
    let e3 = world.entity().set(Position { x: 2, y: 0 }).id();

    world.system::<&Position>().run(|mut it| {
        while it.next() {
            for i in it.iter() {
                it.get_entity(i).unwrap().destruct();
                // Delete is deferred
                assert!(it.get_entity(i).unwrap().is_alive());
            }
        }
    });

    world.progress();

    assert!(!world.entity_from_id(e1).is_alive());
    assert!(!world.entity_from_id(e2).is_alive());
    assert!(!world.entity_from_id(e3).is_alive());
}

#[test]
fn add_from_iter_world_handle() {
    let mut world = World::new();

    let e1 = world
        .entity()
        .set(EntityRef {
            value: world.entity().id(),
        })
        .id();
    let e2 = world
        .entity()
        .set(EntityRef {
            value: world.entity().id(),
        })
        .id();
    let e3 = world
        .entity()
        .set(EntityRef {
            value: world.entity().id(),
        })
        .id();

    world.system::<&EntityRef>().run(|mut it| {
        let world = it.world();
        while it.next() {
            let c = it.field::<EntityRef>(0);
            for i in it.iter() {
                world
                    .entity_from_id(c[i].value)
                    .mut_current_stage(it.world())
                    .add(Position::id());
            }
        }
    });

    world.progress();

    world.entity_from_id(e1).get::<&EntityRef>(|c| {
        let e = world.entity_from_id(c.value);
        assert!(e.has(Position::id()));
    });

    world.entity_from_id(e2).get::<&EntityRef>(|c| {
        let e = world.entity_from_id(c.value);
        assert!(e.has(Position::id()));
    });

    world.entity_from_id(e3).get::<&EntityRef>(|c| {
        let e = world.entity_from_id(c.value);
        assert!(e.has(Position::id()));
    });
}

#[test]
fn new_from_iter() {
    let mut world = World::new();

    let e1 = world.entity().set(Position { x: 0, y: 0 }).id();
    let e2 = world.entity().set(Position { x: 0, y: 0 }).id();
    let e3 = world.entity().set(Position { x: 0, y: 0 }).id();

    world.system::<&Position>().run(|mut it| {
        while it.next() {
            for i in it.iter() {
                it.get_entity(i).unwrap().set(EntityRef {
                    value: it.world().entity().add(Velocity::id()).id(),
                });
            }
        }
    });

    world.progress();

    assert!(world.entity_from_id(e1).has(EntityRef::id()));
    assert!(world.entity_from_id(e2).has(EntityRef::id()));
    assert!(world.entity_from_id(e3).has(EntityRef::id()));

    world.entity_from_id(e1).get::<&EntityRef>(|c| {
        assert!(world.entity_from_id(c.value).has(Velocity::id()));
    });

    world.entity_from_id(e2).get::<&EntityRef>(|c| {
        assert!(world.entity_from_id(c.value).has(Velocity::id()));
    });

    world.entity_from_id(e3).get::<&EntityRef>(|c| {
        assert!(world.entity_from_id(c.value).has(Velocity::id()));
    });
}

#[test]
fn each_w_mut_children_it() {
    let mut world = World::new();

    let parent = world.entity().set(Position { x: 0, y: 0 });
    let e1 = world
        .entity()
        .set(Position { x: 0, y: 0 })
        .child_of(parent)
        .id();
    let e2 = world
        .entity()
        .set(Position { x: 0, y: 0 })
        .child_of(parent)
        .id();
    let e3 = world
        .entity()
        .set(Position { x: 0, y: 0 })
        .child_of(parent)
        .id();

    world.set(Count(0));

    world.system::<&Position>().run(|mut it| {
        let world = it.world();
        while it.next() {
            for i in it.iter() {
                it.get_entity(i).unwrap().each_child(|child| {
                    child.add(Velocity::id());
                    world.get::<&mut Count>(|c| {
                        c.0 += 1;
                    });
                });
            }
        }
    });

    world.progress();

    world.get::<&Count>(|c| {
        assert_eq!(c.0, 3);
    });

    assert!(world.entity_from_id(e1).has(Velocity::id()));
    assert!(world.entity_from_id(e2).has(Velocity::id()));
    assert!(world.entity_from_id(e3).has(Velocity::id()));
}

#[test]
fn readonly_children_iter() {
    let mut world = World::new();

    let parent = world.entity();
    world.entity().set(EntityRef { value: parent.id() });
    world.entity().set(Position { x: 1, y: 0 }).child_of(parent);
    world.entity().set(Position { x: 1, y: 0 }).child_of(parent);
    world.entity().set(Position { x: 1, y: 0 }).child_of(parent);

    world.set(Count(0));

    world.system::<&EntityRef>().run(|mut it| {
        let world = it.world();
        while it.next() {
            let c = it.field::<EntityRef>(0);
            for i in it.iter() {
                world.entity_from_id(c[i].value).each_child(|child| {
                    // Dummy code to ensure we can access the entity
                    child.get::<&Position>(|p| {
                        assert_eq!(p.x, 1);
                        assert_eq!(p.y, 0);
                    });

                    world.get::<&mut Count>(|c| {
                        c.0 += 1;
                    });
                });
            }
        }
    });

    world.progress();

    world.get::<&Count>(|c| {
        assert_eq!(c.0, 3);
    });
}

#[test]
fn rate_filter() {
    let mut world = World::new();

    #[derive(Default, Component)]
    struct Counter {
        root: i32,
        l1_a: i32,
        l1_b: i32,
        l1_c: i32,
        l2_a: i32,
        l2_b: i32,
    }

    world.set(Counter::default());

    let root_mult: i32 = 1;
    let l1_a_mult: i32 = 1;
    let l1_b_mult: i32 = 2;
    let l1_c_mult: i32 = 3;
    let l2_a_mult: i32 = 2;
    let l2_b_mult: i32 = 4;
    let mut frame_count: i32 = 0;

    let root = world.system_named::<()>("root").run(|mut it| {
        let world = it.world();
        while it.next() {
            world.get::<&mut Counter>(|c| {
                c.root += 1;
            });
        }
    });

    let l1_a = world
        .system_named::<()>("l1_a")
        .set_tick_source_rate(root.id(), 1)
        .run(|mut it| {
            let world = it.world();
            while it.next() {
                world.get::<&mut Counter>(|c| {
                    c.l1_a += 1;
                });
            }
        });

    let l1_b = world
        .system_named::<()>("l1_b")
        .set_tick_source_rate(root.id(), 2)
        .run(|mut it| {
            let world = it.world();
            while it.next() {
                world.get::<&mut Counter>(|c| {
                    c.l1_b += 1;
                });
            }
        });

    world
        .system_named::<()>("l1_c")
        .set_tick_source_rate(root.id(), 3)
        .run(|mut it| {
            let world = it.world();
            while it.next() {
                world.get::<&mut Counter>(|c| {
                    c.l1_c += 1;
                });
            }
        });

    world
        .system_named::<()>("l2_a")
        .set_tick_source_rate(l1_a.id(), 2)
        .run(|mut it| {
            let world = it.world();
            while it.next() {
                world.get::<&mut Counter>(|c| {
                    c.l2_a += 1;
                });
            }
        });

    world
        .system_named::<()>("l2_b")
        .set_tick_source_rate(l1_b.id(), 2)
        .run(|mut it| {
            let world = it.world();
            while it.next() {
                world.get::<&mut Counter>(|c| {
                    c.l2_b += 1;
                });
            }
        });

    for _i in 0..30 {
        world.progress();
        frame_count += 1;
        world.get::<&Counter>(|c| {
            assert_eq!(c.root, frame_count / root_mult);
            assert_eq!(c.l1_a, frame_count / l1_a_mult);
            assert_eq!(c.l1_b, frame_count / l1_b_mult);
            assert_eq!(c.l1_c, frame_count / l1_c_mult);
            assert_eq!(c.l2_a, frame_count / l2_a_mult);
            assert_eq!(c.l2_b, frame_count / l2_b_mult);
        });
    }
}

#[test]
fn self_rate_filter() {
    let mut world = World::new();

    world.set(Count(0));

    world
        .system::<&Position>()
        .set_rate(2)
        .each_entity(|e, _p| {
            let world = e.world();
            world.get::<&mut Count>(|c| {
                c.0 += 1;
            });
        });

    world.entity().set(Position { x: 1, y: 2 });

    for _i in 0..10 {
        world.progress();
    }

    world.get::<&Count>(|c| {
        assert_eq!(c.0, 5);
    });
}

#[test]
fn update_rate_filter() {
    let mut world = World::new();

    #[derive(Default, Component)]
    struct Counter {
        root: i32,
        l1: i32,
        l2: i32,
    }

    world.set(Counter::default());

    let root_mult: i32 = 1;
    let mut l1_mult: i32 = 2;
    let mut l2_mult: i32 = 6;
    let mut frame_count: i32 = 0;

    let root = world.system_named::<()>("root").run(|mut it| {
        let world = it.world();
        while it.next() {
            world.get::<&mut Counter>(|c| {
                c.root += 1;
            });
        }
    });

    let l1 = world
        .system_named::<()>("l1")
        .set_tick_source_rate(root.id(), 2)
        .run(|mut it| {
            let world = it.world();
            while it.next() {
                world.get::<&mut Counter>(|c| {
                    c.l1 += 1;
                });
            }
        });

    world
        .system_named::<()>("l2")
        .set_tick_source_rate(l1.id(), 3)
        .run(|mut it| {
            let world = it.world();
            while it.next() {
                world.get::<&mut Counter>(|c| {
                    c.l2 += 1;
                });
            }
        });

    let l1 = l1.id();

    for _i in 0..12 {
        world.progress();
        frame_count += 1;
        world.get::<&Counter>(|c| {
            assert_eq!(c.root, frame_count / root_mult);
            assert_eq!(c.l1, frame_count / l1_mult);
            assert_eq!(c.l2, frame_count / l2_mult);
        });
    }

    let _l1 = System::new_from_existing(world.entity_from_id(l1)).set_rate(4); // Run twice as slow
    l1_mult *= 2;
    l2_mult *= 2;

    frame_count = 0;
    world.set(Counter::default());

    for _i in 0..32 {
        world.progress();
        frame_count += 1;
        world.get::<&Counter>(|c| {
            assert_eq!(c.root, frame_count / root_mult);
            assert_eq!(c.l1, frame_count / l1_mult);
            assert_eq!(c.l2, frame_count / l2_mult);
        });
    }
}

#[test]
fn test_auto_defer_each() {
    let world = World::new();

    let e1 = world.entity().add(Tag).set(Value { value: 10 });
    let e2 = world.entity().add(Tag).set(Value { value: 20 });
    let e3 = world.entity().add(Tag).set(Value { value: 30 });

    let s = world.system::<&mut Value>().with(Tag).each_entity(|e, v| {
        v.value += 1;
        e.remove(Tag);
    });

    s.run();

    assert!(!e1.has(Tag));
    assert!(!e2.has(Tag));
    assert!(!e3.has(Tag));

    assert!(e1.has(Value::id()));
    assert!(e2.has(Value::id()));
    assert!(e3.has(Value::id()));

    e1.get::<&Value>(|v| {
        assert_eq!(v.value, 11);
    });

    e2.get::<&Value>(|v| {
        assert_eq!(v.value, 21);
    });

    e3.get::<&Value>(|v| {
        assert_eq!(v.value, 31);
    });
}

#[test]
fn test_auto_defer_iter() {
    let world = World::new();

    let e1 = world.entity().add(Tag).set(Value { value: 10 });
    let e2 = world.entity().add(Tag).set(Value { value: 20 });
    let e3 = world.entity().add(Tag).set(Value { value: 30 });

    let s = world.system::<&mut Value>().with(Tag).run(|mut it| {
        while it.next() {
            let mut v = it.field_mut::<Value>(0);
            for i in it.iter() {
                v[i].value += 1;
                it.get_entity(i).unwrap().remove(Tag);
            }
        }
    });

    s.run();

    assert!(!e1.has(Tag));
    assert!(!e2.has(Tag));
    assert!(!e3.has(Tag));

    assert!(e1.has(Value::id()));
    assert!(e2.has(Value::id()));
    assert!(e3.has(Value::id()));

    e1.get::<&Value>(|v| {
        assert_eq!(v.value, 11);
    });

    e2.get::<&Value>(|v| {
        assert_eq!(v.value, 21);
    });

    e3.get::<&Value>(|v| {
        assert_eq!(v.value, 31);
    });
}

#[test]
fn custom_pipeline() {
    let mut world = World::new();
    world.set(Count(0));

    let pre_frame = world.entity().add(id::<flecs::pipeline::Phase>());
    let on_frame = world
        .entity()
        .add(id::<flecs::pipeline::Phase>())
        .depends_on(pre_frame);
    let post_frame = world
        .entity()
        .add(id::<flecs::pipeline::Phase>())
        .depends_on(on_frame);
    let tag = world.entity();

    let pip = world
        .pipeline()
        .with(id::<flecs::system::System>())
        .with(id::<flecs::pipeline::Phase>())
        .cascade_id(id::<flecs::DependsOn>())
        .with(tag)
        .build();

    let post_sys = world
        .system_named::<()>("post")
        .kind(post_frame)
        .run(|mut it| {
            while it.next() {
                it.world().get::<&Count>(|c| assert_eq!(c.0, 2));
                it.world().get::<&mut Count>(|c| c.0 += 1);
            }
        });
    world.entity_from_id(post_sys.id()).add(tag);

    let on_sys = world.system_named::<()>("on").kind(on_frame).run(|mut it| {
        while it.next() {
            it.world().get::<&Count>(|c| assert_eq!(c.0, 1));
            it.world().get::<&mut Count>(|c| c.0 += 1);
        }
    });
    world.entity_from_id(on_sys.id()).add(tag);

    let pre_sys = world
        .system_named::<()>("pre")
        .kind(pre_frame)
        .run(|mut it| {
            while it.next() {
                it.world().get::<&Count>(|c| assert_eq!(c.0, 0));
                it.world().get::<&mut Count>(|c| c.0 += 1);
            }
        });
    world.entity_from_id(pre_sys.id()).add(tag);

    world.get::<&Count>(|c| assert_eq!(c.0, 0));

    world.set_pipeline(pip.id());
    world.progress();

    world.get::<&Count>(|c| assert_eq!(c.0, 3));
}

#[test]
fn custom_pipeline_w_kind() {
    let mut world = World::new();

    let tag = world.entity();

    let pip = world
        .pipeline()
        .with(id::<flecs::system::System>())
        .with(tag)
        .build();

    world.set(Count(0));

    world.system::<()>().kind(tag).run(|mut it| {
        while it.next() {
            let world = it.world();
            world.get::<&mut Count>(|c| {
                assert_eq!(c, 0);
                c.0 += 1;
            });
        }
    });

    world.system::<()>().kind(tag).run(|mut it| {
        let world = it.world();
        while it.next() {
            world.get::<&mut Count>(|c| {
                assert_eq!(c.0, 1);
                c.0 += 1;
            });
        }
    });

    world.system::<()>().kind(tag).run(|mut it| {
        let world = it.world();
        while it.next() {
            world.get::<&mut Count>(|c| {
                assert_eq!(c.0, 2);
                c.0 += 1;
            });
        }
    });

    world.get::<&Count>(|c| {
        assert_eq!(c.0, 0);
    });

    world.set_pipeline(pip.id());

    world.progress();

    world.get::<&Count>(|c| {
        assert_eq!(c.0, 3);
    });
}

#[test]
fn instanced_query_w_singleton_each() {
    let world = World::new();

    world
        .component::<Velocity>()
        .add_trait::<flecs::Singleton>();
    world.set(Velocity { x: 1, y: 2 });

    let e1 = world.entity().set(Position { x: 10, y: 20 });
    e1.set(SelfRef { value: e1.id() });
    let e2 = world.entity().set(Position { x: 20, y: 30 });
    e2.set(SelfRef { value: e2.id() });
    let e3 = world.entity().set(Position { x: 30, y: 40 });
    e3.set(SelfRef { value: e3.id() });
    let e4 = world.entity().set(Position { x: 40, y: 50 });
    e4.set(SelfRef { value: e4.id() });
    let e5 = world.entity().set(Position { x: 50, y: 60 });
    e5.set(SelfRef { value: e5.id() });

    e4.add(TagA::id());
    e5.add(TagA::id());

    world.set(Count(0));

    let sys = world
        .system::<(&SelfRef, &mut Position, &Velocity)>()
        .each_entity(|e, (s, p, v)| {
            assert_eq!(e.id(), s.value);
            p.x += v.x;
            p.y += v.y;
            e.world().get::<&mut Count>(|c| {
                c.0 += 1;
            });
        });

    sys.run();

    world.get::<&Count>(|c| assert_eq!(c.0, 5));

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

#[test]
fn instanced_query_w_base_each() {
    let world = World::new();

    let base = world.entity().set(Velocity { x: 1, y: 2 });

    let e1 = world.entity().is_a(base).set(Position { x: 10, y: 20 });
    e1.set(SelfRef { value: e1.id() });
    let e2 = world.entity().is_a(base).set(Position { x: 20, y: 30 });
    e2.set(SelfRef { value: e2.id() });
    let e3 = world.entity().is_a(base).set(Position { x: 30, y: 40 });
    e3.set(SelfRef { value: e3.id() });
    let e4 = world
        .entity()
        .is_a(base)
        .set(Position { x: 40, y: 50 })
        .add(TagA::id());
    e4.set(SelfRef { value: e4.id() });
    let e5 = world
        .entity()
        .is_a(base)
        .set(Position { x: 50, y: 60 })
        .add(TagA::id());
    e5.set(SelfRef { value: e5.id() });
    let e6 = world
        .entity()
        .set(Position { x: 60, y: 70 })
        .set(Velocity { x: 2, y: 3 });
    e6.set(SelfRef { value: e6.id() });
    let e7 = world
        .entity()
        .set(Position { x: 70, y: 80 })
        .set(Velocity { x: 4, y: 5 });
    e7.set(SelfRef { value: e7.id() });

    world.set(Count(0));

    let sys = world
        .system::<(&SelfRef, &mut Position, &Velocity)>()
        .each_entity(|e, (s, p, v)| {
            assert_eq!(e.id(), s.value);
            p.x += v.x;
            p.y += v.y;
            e.world().get::<&mut Count>(|c| {
                c.0 += 1;
            });
        });

    sys.run();

    world.get::<&Count>(|c| assert_eq!(c.0, 7));

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

#[test]
fn instanced_query_w_singleton_iter() {
    let world = World::new();

    world
        .component::<Velocity>()
        .add_trait::<flecs::Singleton>();
    world.set(Velocity { x: 1, y: 2 });

    let e1 = world.entity().set(Position { x: 10, y: 20 });
    e1.set(SelfRef { value: e1.id() });
    let e2 = world.entity().set(Position { x: 20, y: 30 });
    e2.set(SelfRef { value: e2.id() });
    let e3 = world.entity().set(Position { x: 30, y: 40 });
    e3.set(SelfRef { value: e3.id() });
    let e4 = world.entity().set(Position { x: 40, y: 50 });
    e4.set(SelfRef { value: e4.id() });
    let e5 = world.entity().set(Position { x: 50, y: 60 });
    e5.set(SelfRef { value: e5.id() });

    e4.add(TagA::id());
    e5.add(TagA::id());

    world.set(Count(0));

    let sys = world
        .system::<(&SelfRef, &mut Position, &Velocity)>()
        .run(|mut it| {
            while it.next() {
                let s = it.field::<SelfRef>(0);
                let mut p = it.field_mut::<Position>(1);
                let v = it.field::<Velocity>(2);
                assert!(it.count() > 1);
                for i in it.iter() {
                    p[i].x += v[0].x;
                    p[i].y += v[0].y;
                    assert_eq!(it.get_entity(i).unwrap().id(), s[i].value);
                    it.world().get::<&mut Count>(|c| {
                        c.0 += 1;
                    });
                }
            }
        });

    sys.run();

    world.get::<&Count>(|c| assert_eq!(c.0, 5));

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

#[test]
fn instanced_query_w_base_iter() {
    let world = World::new();

    let base = world.entity().set(Velocity { x: 1, y: 2 });

    let e1 = world.entity().is_a(base).set(Position { x: 10, y: 20 });
    e1.set(SelfRef { value: e1.id() });
    let e2 = world.entity().is_a(base).set(Position { x: 20, y: 30 });
    e2.set(SelfRef { value: e2.id() });
    let e3 = world.entity().is_a(base).set(Position { x: 30, y: 40 });
    e3.set(SelfRef { value: e3.id() });
    let e4 = world
        .entity()
        .is_a(base)
        .set(Position { x: 40, y: 50 })
        .add(TagA::id());
    e4.set(SelfRef { value: e4.id() });
    let e5 = world
        .entity()
        .is_a(base)
        .set(Position { x: 50, y: 60 })
        .add(TagA::id());
    e5.set(SelfRef { value: e5.id() });
    let e6 = world
        .entity()
        .set(Position { x: 60, y: 70 })
        .set(Velocity { x: 2, y: 3 });
    e6.set(SelfRef { value: e6.id() });
    let e7 = world
        .entity()
        .set(Position { x: 70, y: 80 })
        .set(Velocity { x: 4, y: 5 });
    e7.set(SelfRef { value: e7.id() });

    world.set(Count(0));

    let sys = world
        .system::<(&SelfRef, &mut Position, &Velocity)>()
        .run(|mut it| {
            while it.next() {
                let s = it.field::<SelfRef>(0);
                let mut p = it.field_mut::<Position>(1);
                let v = it.field::<Velocity>(2);
                assert!(it.count() > 1);
                for i in it.iter() {
                    if it.is_self(2) {
                        p[i].x += v[i].x;
                        p[i].y += v[i].y;
                    } else {
                        p[i].x += v[0].x;
                        p[i].y += v[0].y;
                    }
                    assert_eq!(it.get_entity(i).unwrap().id(), s[i].value);
                    it.world().get::<&mut Count>(|c| {
                        c.0 += 1;
                    });
                }
            }
        });

    sys.run();

    world.get::<&Count>(|c| assert_eq!(c.0, 7));

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

#[test]
fn create_w_no_template_args() {
    let world = World::new();

    let entity = world.entity().set(Position { x: 10, y: 20 });

    let entity_id = entity.id();
    world.set(Count(0));

    let s = world
        .system::<()>()
        .with(Position::id())
        .each_entity(move |e, _| {
            let world = e.world();
            assert!(e == entity_id);
            world.get::<&mut Count>(|c| {
                c.0 += 1;
            });
        });

    s.run();

    world.get::<&Count>(|c| {
        assert_eq!(c.0, 1);
    });
}

#[derive(Component)]
struct PipelineType;
#[derive(Component)]
struct First;
#[derive(Component)]
struct Second;

#[test]
fn system_w_type_kind_type_pipeline() {
    let mut world = World::new();

    world
        .component::<Second>()
        .add(id::<flecs::pipeline::Phase>())
        .depends_on(
            world
                .component::<First>()
                .add(id::<flecs::pipeline::Phase>()),
        );

    world
        .pipeline_type::<PipelineType>()
        .with(id::<flecs::system::System>())
        .with(id::<flecs::pipeline::Phase>())
        .cascade_id(id::<flecs::DependsOn>())
        .build();

    world.set_pipeline(PipelineType::id());

    let entity = world.entity().add(Tag);
    let entity_id = entity.id();

    world.set(Count2 { a: 0, b: 0 });

    world
        .system::<()>()
        .with(&Tag::id())
        .kind(Second::id())
        .run(move |mut it| {
            while it.next() {
                for i in it.iter() {
                    let e = it.get_entity(i).unwrap();
                    let world = e.world();
                    assert!(e == entity_id);
                    world.get::<&mut Count2>(|c| {
                        assert_eq!(c.a, 0);
                        assert_eq!(c.b, 1);
                        c.a += 1;
                    });
                }
            }
        });

    world
        .system::<()>()
        .with(&Tag::id())
        .kind(First::id())
        .run(move |mut it| {
            while it.next() {
                for i in it.iter() {
                    let world = it.world();
                    let e = it.get_entity(i).unwrap();
                    assert!(e == entity_id);
                    world.get::<&mut Count2>(|c| {
                        assert_eq!(c.b, 0);
                        c.b += 1;
                    });
                }
            }
        });

    world.progress();

    world.get::<&Count2>(|c| {
        assert_eq!(c.a, 1);
        assert_eq!(c.b, 1);
    });
}

#[test]
fn default_ctor() {
    let world = World::new();

    world.set(Count(0));

    let sys = world.system::<&Position>().each_entity(|e, p| {
        assert_eq!(p.x, 10);
        assert_eq!(p.y, 20);
        e.world().get::<&mut Count>(|c| {
            c.0 += 1;
        });
    });

    world.entity().set(Position { x: 10, y: 20 });

    let sys_var = world.system_from(unsafe { sys.entity_view(&world) });

    sys_var.run();

    world.get::<&Count>(|c| {
        assert_eq!(c.0, 1);
    });
}

#[test]
fn entity_ctor() {
    let world = World::new();

    world.set(Count(0));

    let sys = world.system::<()>().run(|mut it| {
        let world = it.world();
        while it.next() {
            world.get::<&mut Count>(|c| {
                c.0 += 1;
            });
        }
    });

    let sys_from_id = world.system_from(unsafe { sys.entity_view(&world) });

    sys_from_id.run();
    world.get::<&Count>(|c| {
        assert_eq!(c.0, 1);
    });
}

#[test]
fn ensure_instanced_w_each() {
    let world = World::new();

    let e1 = world.entity().set(Position { x: 10, y: 20 });
    let e1_id = e1.id();

    world.set(Count(0));

    let sys = world.system::<&Position>().each_iter(move |it, row, _| {
        let e = it.get_entity(row).unwrap();
        assert!(e == e1_id);
        it.world().get::<&mut Count>(|count| {
            count.0 += 1;
        });
    });

    let _q = sys.query();

    assert_eq!(world.get::<&Count>(|c| c.0), 0);
    sys.run();
    assert_eq!(world.get::<&Count>(|c| c.0), 1);
}

#[test]
fn multithread_system_w_query_each() {
    let mut world = World::new();

    world.set_threads(2);

    let e1 = world
        .entity()
        .set(Position { x: 10, y: 20 })
        .set(Velocity { x: 1, y: 2 })
        .id();

    let q = world.new_query::<&Velocity>().handle();

    world
        .system::<&mut Position>()
        .par_each_entity(move |e, p| {
            let world = e.world();
            q.iter_stage(world).each(|v| {
                p.x += v.x;
                p.y += v.y;
            });
        });

    world.progress();

    world.entity_from_id(e1).get::<&Position>(|p| {
        assert_eq!(p.x, 11);
        assert_eq!(p.y, 22);
    });
}

#[test]
fn multithread_system_w_query_each_w_iter() {
    let mut world = World::new();

    world.set_threads(2);

    let e = world
        .entity()
        .set(Position { x: 10, y: 20 })
        .set(Velocity { x: 1, y: 2 })
        .id();

    let q = world.new_query::<&Velocity>().handle();

    world.system::<&mut Position>().par_run(move |mut it| {
        while it.next() {
            let mut p = it.field_mut::<Position>(0);
            for i in it.iter() {
                let p = &mut p[i];
                q.iter_stage(it.world()).each(|v| {
                    p.x += v.x;
                    p.y += v.y;
                });
            }
        }
    });

    world.progress();

    world.entity_from_id(e).get::<&Position>(|p| {
        assert_eq!(p.x, 11);
        assert_eq!(p.y, 22);
    });
}

#[test]
fn multithread_system_w_query_each_w_world() {
    let mut world = World::new();

    world.set_threads(2);

    let e = world
        .entity()
        .set(Position { x: 10, y: 20 })
        .set(Velocity { x: 1, y: 2 })
        .id();

    let q = world.new_query::<&Velocity>().handle();
    world.system::<&mut Position>().par_run(move |mut it| {
        let world = it.world();
        while it.next() {
            let mut p = it.field_mut::<Position>(0);
            for i in it.iter() {
                let p = &mut p[i];
                q.iter_stage(world).each(|v| {
                    p.x += v.x;
                    p.y += v.y;
                });
            }
        }
    });

    world.progress();

    world.entity_from_id(e).get::<&Position>(|p| {
        assert_eq!(p.x, 11);
        assert_eq!(p.y, 22);
    });
}

#[test]
fn multithread_system_w_query_iter() {
    let mut world = World::new();

    world.set_threads(2);

    let e = world
        .entity()
        .set(Position { x: 10, y: 20 })
        .set(Velocity { x: 1, y: 2 })
        .id();

    let q = world.new_query::<&Velocity>().handle();

    world
        .system::<&mut Position>()
        .par_each_entity(move |e, p| {
            q.iter_stage(e).run(|mut it| {
                while it.next() {
                    let v = it.field::<Velocity>(0);

                    for i in it.iter() {
                        p.x += v[i].x;
                        p.y += v[i].y;
                    }
                }
            });
        });

    world.progress();

    world.entity_from_id(e).get::<&Position>(|p| {
        assert_eq!(p.x, 11);
        assert_eq!(p.y, 22);
    });
}

#[test]
fn multithread_system_w_query_iter_w_iter() {
    let mut world = World::new();

    world.set_threads(2);

    let e = world
        .entity()
        .set(Position { x: 10, y: 20 })
        .set(Velocity { x: 1, y: 2 })
        .id();

    let q = world.new_query::<&Velocity>().handle();

    world.system::<&mut Position>().par_run(move |mut it| {
        let world = it.world();
        while it.next() {
            let mut p = it.field_mut::<Position>(0);
            for i in it.iter() {
                let p = &mut p[i];
                q.iter_stage(world).run(|mut it| {
                    while it.next() {
                        let v = it.field::<Velocity>(0);
                        for i in it.iter() {
                            p.x += v[i].x;
                            p.y += v[i].y;
                        }
                    }
                });
            }
        }
    });

    world.progress();

    world.entity_from_id(e).get::<&Position>(|p| {
        assert_eq!(p.x, 11);
        assert_eq!(p.y, 22);
    });
}

#[test]
fn multithread_system_w_query_iter_w_world() {
    let mut world = World::new();

    world.set_threads(2);

    let e = world
        .entity()
        .set(Position { x: 10, y: 20 })
        .set(Velocity { x: 1, y: 2 })
        .id();

    let q = world.new_query::<&Velocity>().handle();

    world.system::<&mut Position>().par_run(move |mut it| {
        let world = it.world();
        while it.next() {
            let mut p = it.field_mut::<Position>(0);
            for i in it.iter() {
                let p = &mut p[i];
                q.iter_stage(world).run(|mut it| {
                    while it.next() {
                        let v = it.field::<Velocity>(0);
                        for i in it.iter() {
                            p.x += v[i].x;
                            p.y += v[i].y;
                        }
                    }
                });
            }
        }
    });

    world.progress();

    world.entity_from_id(e).get::<&Position>(|p| {
        assert_eq!(p.x, 11);
        assert_eq!(p.y, 22);
    });
}

#[test]
fn multithread_system_w_get_var() {
    let mut world = World::new();
    world.set_threads(4);

    let bob = world.entity_named("bob").add(Position::id());
    let alice = world.entity_named("alice").add(Position::id());
    let bob_id = bob.id();
    let alice_id = alice.id();

    bob.add((Rel::id(), alice));

    world.set(Count(0));

    world
        .system::<&Position>()
        .with((Rel::id(), "$other"))
        .term_at(0)
        .set_src("$other")
        .par_each_iter(move |it, _row, _pos| {
            let e = it.get_entity(_row).unwrap();
            let other = it.get_var_by_name("other");
            assert!(e == bob_id);
            assert!(other == alice_id);
            it.world().get::<&mut Count>(|count| {
                count.0 += 1;
            });
        });

    world.progress();

    world.get::<&Count>(|c| assert_eq!(c.0, 1));
}

#[test]
fn run_callback() {
    let mut world = World::new();

    let entity = world
        .entity()
        .set(Position { x: 10, y: 20 })
        .set(Velocity { x: 1, y: 2 })
        .id();

    world.system::<(&mut Position, &Velocity)>().run_each(
        |mut it| {
            while it.next() {
                it.each();
            }
        },
        |(p, v)| {
            p.x += v.x;
            p.y += v.y;
        },
    );

    world.progress();

    world.entity_from_id(entity).get::<&Position>(|p| {
        assert_eq!(p.x, 11);
        assert_eq!(p.y, 22);
    });

    world.entity_from_id(entity).get::<&Velocity>(|v| {
        assert_eq!(v.x, 1);
        assert_eq!(v.y, 2);
    });
}

#[test]
fn startup_system() {
    let mut world = World::new();

    world.set(Count2 { a: 0, b: 0 });

    world
        .system::<()>()
        .kind(flecs::pipeline::OnStart::ID)
        .run(|mut it| {
            let world = it.world();
            while it.next() {
                assert!(it.delta_time() == 0.0);
                world.get::<&mut Count2>(|c| {
                    c.a += 1;
                });
            }
        });

    world
        .system::<()>()
        .kind(flecs::pipeline::OnUpdate::ID)
        .run(|mut it| {
            let world = it.world();
            while it.next() {
                assert_ne!(it.delta_time(), 0.0);
                world.get::<&mut Count2>(|c| {
                    c.b += 1;
                });
            }
        });

    world.progress();
    world.get::<&Count2>(|c| {
        assert_eq!(c.a, 1);
        assert_eq!(c.b, 1);
    });
    world.progress();
    world.get::<&Count2>(|c| {
        assert_eq!(c.a, 1);
        assert_eq!(c.b, 2);
    });
}

#[test]
fn interval_tick_source() {
    let mut world = World::new();

    let t = world.timer().set_interval(2.1);

    t.get::<&mut flecs::timer::Timer>(|timer| {
        timer.time = 0.0;
    });

    world.set(Count2 { a: 0, b: 0 });

    world.system::<()>().set_tick_source(t).run(|mut it| {
        let world = it.world();
        while it.next() {
            world.get::<&mut Count2>(|c| {
                c.a += 1;
            });
        }
    });

    world.system::<()>().set_tick_source(t).run(|mut it| {
        let world = it.world();
        while it.next() {
            world.get::<&mut Count2>(|c| {
                c.b += 1;
            });
        }
    });

    world.progress_time(1.0);
    let c = world.cloned::<&Count2>();
    assert_eq!(c.a, 0);
    assert_eq!(c.b, 0);

    world.progress_time(1.0);
    let c = world.cloned::<&Count2>();
    assert_eq!(c.a, 0);
    assert_eq!(c.b, 0);

    world.progress_time(1.0);
    let c = world.cloned::<&Count2>();
    assert_eq!(c.a, 1);
    assert_eq!(c.b, 1);
}

#[test]
fn rate_tick_source() {
    let mut world = World::new();

    let t = world.timer().set_rate(3);

    world.set(Count2 { a: 0, b: 0 });

    world.system::<()>().set_tick_source(t).run(|mut it| {
        let world = it.world();
        while it.next() {
            world.get::<&mut Count2>(|c| {
                c.a += 1;
            });
        }
    });

    world.system::<()>().set_tick_source(t).run(|mut it| {
        let world = it.world();
        while it.next() {
            world.get::<&mut Count2>(|c| {
                c.b += 1;
            });
        }
    });

    world.progress_time(1.0);
    let c = world.cloned::<&Count2>();
    assert_eq!(0, c.a);
    assert_eq!(0, c.b);

    world.progress_time(1.0);
    let c = world.cloned::<&Count2>();
    assert_eq!(0, c.a);
    assert_eq!(0, c.b);

    world.progress_time(1.0);
    let c = world.cloned::<&Count2>();
    assert_eq!(1, c.a);
    assert_eq!(1, c.b);
}

#[test]
fn nested_rate_tick_source() {
    let mut world = World::new();
    world.set(Count(0));
    world.set(Count2 { a: 0, b: 0 });

    // t3 ticks every 3 frames; t6 ticks every 2 t3 ticks = every 6 frames
    let t3 = world.timer().set_rate(3);
    let t6 = world.timer().set_rate_with_tick_source(2, t3.id());

    world
        .system_named::<()>("sys_a")
        .set_tick_source(t3.id())
        .run(|mut it| {
            while it.next() {
                it.world().get::<&mut Count>(|c| c.0 += 1);
            }
        });

    world
        .system_named::<()>("sys_b")
        .set_tick_source(t6.id())
        .run(|mut it| {
            while it.next() {
                it.world().get::<&mut Count2>(|c| c.a += 1);
            }
        });

    // frames 1–2: neither fires
    for _ in 0..2 {
        world.progress();
    }
    world.get::<&Count>(|c| assert_eq!(c.0, 0));
    world.get::<&Count2>(|c| assert_eq!(c.a, 0));

    // frame 3: t3 ticks → sys_a fires (count=1), t6 doesn't (needs 2 t3 ticks)
    world.progress();
    world.get::<&Count>(|c| assert_eq!(c.0, 1));
    world.get::<&Count2>(|c| assert_eq!(c.a, 0));

    // frames 4–5: neither fires
    for _ in 0..2 {
        world.progress();
    }
    world.get::<&Count>(|c| assert_eq!(c.0, 1));
    world.get::<&Count2>(|c| assert_eq!(c.a, 0));

    // frame 6: t3 ticks again (count=2) AND t6 ticks (sys_b fires, a=1)
    world.progress();
    world.get::<&Count>(|c| assert_eq!(c.0, 2));
    world.get::<&Count2>(|c| assert_eq!(c.a, 1));
}

#[test]
fn table_get() {
    let world = World::new();

    let e1 = world.entity().set(Position { x: 10, y: 20 });
    let e2 = world.entity().set(Position { x: 20, y: 30 });

    let e1_id = e1.id();
    let e2_id = e2.id();

    let sys = world
        .system::<()>()
        .with(Position::id())
        .each_iter(move |it, index, _| {
            let e = it.get_entity(index).unwrap();
            let mut table = it.table().unwrap();
            let pos = table.get_mut::<Position>().unwrap();
            let i: usize = index.into();
            let p = &pos[i];
            assert!(e == e1_id || e == e2_id);
            if e == e1_id {
                assert_eq!(p.x, 10);
                assert_eq!(p.y, 20);
            } else if e == e2_id {
                assert_eq!(p.x, 20);
                assert_eq!(p.y, 30);
            }
        });

    sys.run();
}

#[test]
fn range_get() {
    let world = World::new();

    let e1 = world.entity().set(Position { x: 10, y: 20 });
    let e2 = world.entity().set(Position { x: 20, y: 30 });

    let e1_id = e1.id();
    let e2_id = e2.id();

    let sys = world
        .system::<()>()
        .with(Position::id())
        .each_iter(move |it, index, _| {
            let e = it.get_entity(index).unwrap();
            let mut range = it.range().unwrap();
            let pos = range.get_mut::<Position>().unwrap();
            let i: usize = index.into();
            let p = &pos[i];
            assert!(e == e1_id || e == e2_id);
            if e == e1_id {
                assert_eq!(p.x, 10);
                assert_eq!(p.y, 20);
            } else if e == e2_id {
                assert_eq!(p.x, 20);
                assert_eq!(p.y, 30);
            }
        });

    sys.run();
}

#[test]
fn randomize_timers() {
    let world = World::new();

    // on musl builds, `rand` call always returns 0 until seeded, so we need to
    // call srand to seed the random number generator
    // SAFETY: libc::time and libc::srand are plain C library calls with no
    // preconditions here; null is a valid argument to time() per its contract.
    unsafe {
        let seed = libc::time(core::ptr::null_mut()) as u32;
        libc::srand(seed);
    }

    let s1 = world
        .system::<()>()
        .set_interval(1.0)
        .run(|mut it| while it.next() {});

    {
        let t = s1.try_cloned::<&flecs::timer::Timer>();
        assert!(t.is_some());
        assert_eq!(t.unwrap().time, 0.0);
    }

    world.randomize_timers();

    let s2 = world
        .system::<()>()
        .set_interval(1.0)
        .run(|mut it| while it.next() {});

    {
        let t = s1.try_cloned::<&flecs::timer::Timer>();
        assert!(t.is_some());
        assert_ne!(t.unwrap().time, 0.0);
    }

    {
        let t = s2.try_cloned::<&flecs::timer::Timer>();
        assert!(t.is_some());
        assert_ne!(t.unwrap().time, 0.0);
    }
}

#[test]
fn optional_pair_term() {
    thread_local! {
        static WITH_PAIR: std::cell::Cell<i32> = const { std::cell::Cell::new(0) };
        static WITHOUT_PAIR: std::cell::Cell<i32> = const { std::cell::Cell::new(0) };
    }
    WITH_PAIR.set(0);
    WITHOUT_PAIR.set(0);

    let mut world = World::new();

    world
        .entity()
        .add(TagA::id())
        .set_pair::<Position, Tag>(Position { x: 1, y: 2 });
    world.entity().add(TagA::id());

    world
        .system::<Option<&(Position, Tag)>>()
        .with(TagA::id())
        .each(|p| {
            if let Some(p) = p {
                assert_eq!(p.x, 1);
                assert_eq!(p.y, 2);
                WITH_PAIR.set(WITH_PAIR.get() + 1);
            } else {
                WITHOUT_PAIR.set(WITHOUT_PAIR.get() + 1);
            }
        });

    world.progress_time(1.0);
    assert_eq!(WITH_PAIR.get(), 1);
    assert_eq!(WITHOUT_PAIR.get(), 1);
}

#[test]
fn singleton_tick_source() {
    let mut world = World::new();

    world.timer_from::<TagA>().set_timeout(1.5);

    world.set(Count(0));

    world
        .system::<()>()
        .set_tick_source(TagA::id())
        .run(|mut it| {
            while it.next() {
                it.world().get::<&mut Count>(|count| {
                    count.0 += 1;
                });
            }
        });

    world.progress_time(1.0);
    world.get::<&Count>(|c| assert_eq!(c.0, 0));

    world.progress_time(1.0);
    world.get::<&Count>(|c| assert_eq!(c.0, 1));

    world.progress_time(2.0);
    world.get::<&Count>(|c| assert_eq!(c.0, 1));
}

#[test]
fn pipeline_step_with_kind_enum() {
    let mut world = World::new();

    let custom_step = world
        .entity()
        .add(flecs::Phase::ID)
        .depends_on(flecs::pipeline::OnStart::ID);

    world.set(Count(0));

    world.system::<()>().kind(custom_step).run(move |mut it| {
        while it.next() {
            it.world().get::<&mut Count>(|c| c.0 += 1);
        }
    });

    world.progress();
    world.get::<&Count>(|c| assert!(c.0 > 0));
}

#[test]
fn pipeline_step_depends_on_pipeline_step_with_enum() {
    let mut world = World::new();

    let custom_step = world
        .entity()
        .add(flecs::Phase::ID)
        .depends_on(flecs::pipeline::OnStart::ID);

    let custom_step2 = world.entity().add(flecs::Phase::ID).depends_on(custom_step);

    world.set(Count(0));

    world.system::<()>().kind(custom_step2).run(move |mut it| {
        while it.next() {
            it.world().get::<&mut Count>(|c| c.0 += 1);
        }
    });

    world.progress();
    world.get::<&Count>(|c| assert!(c.0 > 0));
}

#[test]
fn register_twice_w_each() {
    let world = World::new();

    world.set(Count2 { a: 0, b: 0 });

    let sys = world.system_named::<()>("Test").run(|mut it| {
        while it.next() {
            it.world().get::<&mut Count2>(|count| {
                count.a += 1;
            });
        }
    });
    sys.run();

    world.get::<&mut Count2>(|count| {
        assert_eq!(count.a, 1);
    });

    let sys = sys.update::<()>().run(|mut it| {
        while it.next() {
            it.world().get::<&mut Count2>(|count| {
                count.b += 1;
            });
        }
    });
    sys.run();

    world.get::<&mut Count2>(|count| {
        assert_eq!(count.b, 1);
    });
}

#[test]
fn register_twice_w_run() {
    let world = World::new();

    world.set(Count2 { a: 0, b: 0 });

    let sys = world.system_named::<()>("Test").run(|mut it| {
        while it.next() {
            it.world().get::<&mut Count2>(|count| {
                count.a += 1;
            });
        }
    });
    sys.run();

    world.get::<&Count2>(|count| {
        assert_eq!(count.a, 1);
    });

    let sys = sys.update::<()>().run(|mut it| {
        while it.next() {
            it.world().get::<&mut Count2>(|count| {
                count.b += 1;
            });
        }
    });
    sys.run();

    world.get::<&Count2>(|count| {
        assert_eq!(count.b, 1);
    });
}

#[test]
fn register_twice_w_run_each() {
    let world = World::new();

    world.set(Count2 { a: 0, b: 0 });

    let sys = world.system_named::<()>("Test").run(|mut it| {
        while it.next() {
            it.world().get::<&mut Count2>(|count| {
                count.a += 1;
            });
        }
    });
    sys.run();

    world.get::<&Count2>(|count| {
        assert_eq!(count.a, 1);
    });

    let sys = sys.update::<()>().run(|mut it| {
        while it.next() {
            it.world().get::<&mut Count2>(|count| {
                count.b += 1;
            });
        }
    });
    sys.run();

    world.get::<&Count2>(|count| {
        assert_eq!(count.b, 1);
    });
}

#[test]
fn register_twice_w_each_run() {
    let world = World::new();

    world.set(Count2 { a: 0, b: 0 });

    let sys = world.system_named::<()>("Test").run(|mut it| {
        while it.next() {
            it.world().get::<&mut Count2>(|count| {
                count.a += 1;
            });
        }
    });
    sys.run();

    world.get::<&Count2>(|count| {
        assert_eq!(count.a, 1);
    });

    let sys = sys.update::<()>().run(|mut it| {
        while it.next() {
            it.world().get::<&mut Count2>(|count| {
                count.b += 1;
            });
        }
    });
    sys.run();

    world.get::<&Count2>(|count| {
        assert_eq!(count.b, 1);
    });
}

#[test]
fn lookup_and_update_each() {
    let world = World::new();

    world.set(Count2 { a: 0, b: 0 });

    world
        .system_named::<()>("Test")
        .run(|mut it| {
            while it.next() {
                it.world().get::<&mut Count2>(|count| {
                    count.a += 1;
                });
            }
        })
        .run();

    world.get::<&Count2>(|count| {
        assert_eq!(count.a, 1);
    });

    let e = world.lookup("Test");
    assert!(*e.id() != 0);

    let sys = world.system_from(e);
    sys.update::<()>()
        .run(|mut it| {
            while it.next() {
                it.world().get::<&mut Count2>(|count| {
                    count.b += 1;
                });
            }
        })
        .run();

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
        .system_named::<()>("Test")
        .run(|mut it| {
            while it.next() {
                it.world().get::<&mut Count2>(|count| {
                    count.a += 1;
                });
            }
        })
        .run();

    world.get::<&Count2>(|count| {
        assert_eq!(count.a, 1);
    });

    let e = world.lookup("Test");
    assert!(*e.id() != 0);

    let sys = world.system_from(e);
    sys.update::<()>()
        .run(|mut it| {
            while it.next() {
                it.world().get::<&mut Count2>(|count| {
                    count.b += 1;
                });
            }
        })
        .run();

    world.get::<&Count2>(|count| {
        assert_eq!(count.a, 1);
        assert_eq!(count.b, 1);
    });
}

#[test]
fn lookup_and_update_ctx() {
    let world = World::new();

    world
        .system_named::<()>("Test")
        .run(|mut it| while it.next() {});

    let e = world.lookup("Test");
    assert!(*e.id() != 0);

    let mut sys = world.system_from(e);
    assert!(sys.context().is_null());

    let mut my_ctx: i32 = 42;
    sys.set_context(&mut my_ctx as *mut i32 as *mut c_void);
    assert!(sys.context() == &mut my_ctx as *mut i32 as *mut c_void);
}

#[test]
fn set_group() {
    #[derive(Component)]
    struct GroupRel;
    #[derive(Component)]
    struct GroupTgtA;
    #[derive(Component)]
    struct GroupTgtB;
    #[derive(Component)]
    struct GroupTgtC;

    let world = World::new();

    let e1 = world.entity().add((GroupRel::id(), GroupTgtA::id()));
    let e2 = world.entity().add((GroupRel::id(), GroupTgtB::id()));
    world.entity().add((GroupRel::id(), GroupTgtC::id()));

    let e4 = world
        .entity()
        .add((GroupRel::id(), GroupTgtA::id()))
        .add(TagA::id());
    let e5 = world
        .entity()
        .add((GroupRel::id(), GroupTgtB::id()))
        .add(TagA::id());
    world
        .entity()
        .add((GroupRel::id(), GroupTgtC::id()))
        .add(TagA::id());

    let e1_id = e1.id();
    let e2_id = e2.id();
    let e4_id = e4.id();
    let e5_id = e5.id();
    let tgt_b_id = world.component::<GroupTgtB>().id();

    world.set(Count(0));

    unsafe extern "C-unwind" fn group_by_grp_rel(
        world: *mut flecs_ecs::sys::ecs_world_t,
        table: *mut flecs_ecs::sys::ecs_table_t,
        id: flecs_ecs::sys::ecs_id_t,
        _ctx: *mut c_void,
    ) -> u64 {
        let mut match_id: flecs_ecs::sys::ecs_id_t = 0;
        // SAFETY: world and table pointers are valid, non-null pointers passed in by
        // flecs from within a group_by callback invocation; ecs_search is a valid C API call.
        unsafe {
            if flecs_ecs::sys::ecs_search(
                world,
                table,
                flecs_ecs::sys::ecs_make_pair(id, flecs_ecs::sys::EcsWildcard),
                &mut match_id,
            ) != -1
            {
                // ECS_PAIR_SECOND: low 32 bits of the id hold the second element
                (match_id & 0xFFFF_FFFF) as u64
            } else {
                0
            }
        }
    }

    let sys = world
        .system::<()>()
        .with((GroupRel::id(), flecs::Wildcard::ID))
        .group_by_fn(GroupRel::id(), Some(group_by_grp_rel))
        .run(move |mut it| {
            while it.next() {
                for i in it.iter() {
                    let e = it.get_entity(i).unwrap();
                    it.world().get::<&mut Count>(|count| {
                        if e == e1_id || e == e4_id || e == e2_id || e == e5_id {
                            count.0 += 1;
                        }
                    });
                }
            }
        });

    // Run with TgtB group
    sys.query()
        .with_group(tgt_b_id)
        .run(|mut it| while it.next() {});

    sys.run();

    world.get::<&Count>(|c| assert!(c.0 > 0));
}

#[test]
fn run_w_0_src_query() {
    let mut world = World::new();

    world.set(Count(0));

    world.system::<()>().write(Position::id()).run(|it| {
        let world = it.world();
        world.get::<&mut Count>(|c| {
            c.0 += 1;
        });
    });

    world.progress();
    world.get::<&Count>(|c| {
        assert_eq!(c.0, 1);
    });
}

#[test]
fn reuse_system_builder() {
    let mut world = World::new();

    world.entity().set(Position { x: 10, y: 20 });
    world
        .entity()
        .set(Position { x: 10, y: 20 })
        .set(Velocity { x: 1, y: 2 });

    let count_1 = std::rc::Rc::new(core::cell::Cell::new(0));
    let count_2 = std::rc::Rc::new(core::cell::Cell::new(0));

    let mut sb = world.system::<&Position>();

    let count_1_c = count_1.clone();
    let s1 = sb.each(move |_p| {
        count_1_c.set(count_1_c.get() + 1);
    });

    let count_2_c = count_2.clone();
    let s2 = sb.with(Velocity::id()).each(move |_p| {
        count_2_c.set(count_2_c.get() + 1);
    });

    assert!(s1.id() != s2.id());

    world.progress();

    assert_eq!(count_1.get(), 2);
    assert_eq!(count_2.get(), 1);
}

#[test]
fn kind_on_shared_builder() {
    let world = World::new();

    let mut sb = world.system::<&Position>();

    let s1 = sb.each(|_p| {});

    let s2 = sb.kind(id::<flecs::pipeline::PostUpdate>()).each(|_p| {});

    assert!(s1.id() != s2.id());

    let s1 = unsafe { s1.entity_view(&world) };
    let s2 = unsafe { s2.entity_view(&world) };

    assert!(s1.has((flecs::DependsOn::ID, flecs::pipeline::OnUpdate::ID)));
    assert!(s1.has(flecs::pipeline::OnUpdate::ID));

    assert!(s2.has((flecs::DependsOn::ID, flecs::pipeline::PostUpdate::ID)));
    assert!(s2.has(flecs::pipeline::PostUpdate::ID));
    assert!(!s2.has((flecs::DependsOn::ID, flecs::pipeline::OnUpdate::ID)));
    assert!(!s2.has(flecs::pipeline::OnUpdate::ID));
}

#[test]
fn custom_pipeline_w_name() {
    let mut world = World::new();

    let tag = world.entity();

    let pip = world
        .pipeline_named("MyPipeline")
        .with(id::<flecs::system::System>())
        .with(tag)
        .build();

    assert_eq!(unsafe { pip.entity_view(&world) }.name(), "MyPipeline");
    assert_eq!(world.lookup("MyPipeline"), unsafe { pip.entity_view(&world) });

    let count = std::rc::Rc::new(core::cell::Cell::new(0));
    let count_c = count.clone();

    world.system::<()>().kind(tag).run(move |mut it| {
        while it.next() {
            count_c.set(count_c.get() + 1);
        }
    });

    assert_eq!(count.get(), 0);

    world.set_pipeline(unsafe { pip.entity_view(&world) });

    world.progress();

    assert_eq!(count.get(), 1);
}
