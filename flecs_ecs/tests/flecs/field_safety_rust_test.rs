use crate::common_test::*;

#[test]
#[should_panic]
fn field_index_shared_field_above_zero_panics() {
    let world = World::new();

    let src = world.entity().set(Velocity { x: 5, y: 5 });
    world.entity().set(Position { x: 1, y: 1 });
    world.entity().set(Position { x: 2, y: 2 });

    let query = world
        .query::<(&Position, &Velocity)>()
        .term_at(1)
        .set_src(src)
        .build();

    query.run(|mut it| {
        while it.next() {
            let vel = it.field::<Velocity>(1);
            assert!(vel.is_shared());
            assert!(it.count() > 1);
            for i in it.iter() {
                core::hint::black_box(vel[i].x);
            }
        }
    });
}

/// The field type/id check must run in release builds too: reading a field as
/// the wrong component type would reinterpret its bytes. This panics regardless
/// of `debug_assertions`.
#[test]
#[should_panic(expected = "id mismatch")]
fn field_wrong_type_panics_in_release() {
    let world = World::new();
    world.entity().set(Position { x: 1, y: 1 });

    let query = world.new_query::<&Position>();
    query.run(|mut it| {
        while it.next() {
            let wrong = it.field::<Velocity>(0);
            core::hint::black_box(wrong[0].x);
        }
    });
}

#[test]
#[should_panic]
fn field_index_out_of_bounds_panics() {
    let world = World::new();
    world.entity().set(Position { x: 1, y: 1 });

    let query = world.new_query::<&Position>();
    query.run(|mut it| {
        while it.next() {
            let pos = it.field::<Position>(0);
            let oob = unsafe { FieldIndex::new(pos.len()) };
            core::hint::black_box(pos[oob].x);
        }
    });
}

#[test]
#[should_panic]
fn field_at_untyped_row_out_of_bounds_panics() {
    let world = World::new();
    world.entity().set(Position { x: 1, y: 1 });

    let query = world.new_query::<&Position>();
    query.run(|mut it| {
        while it.next() {
            core::hint::black_box(it.field_at_untyped(0, it.count() + 10));
        }
    });
}

#[test]
#[should_panic]
fn field_untyped_at_out_of_bounds_panics() {
    let world = World::new();
    world.entity().set(Position { x: 1, y: 1 });

    let query = world.new_query::<&Position>();
    query.run(|mut it| {
        while it.next() {
            let field = it.field_untyped(0);
            core::hint::black_box(field.at(it.count() + 10));
        }
    });
}

#[test]
fn try_field_ok() {
    let world = World::new();
    world.entity().set(Position { x: 3, y: 4 });

    let query = world.new_query::<&Position>();
    let mut count = 0;
    query.run(|mut it| {
        while it.next() {
            let pos = it.try_field::<Position>(0).unwrap();
            for i in it.iter() {
                assert_eq!(pos[i].x, 3);
                assert_eq!(pos[i].y, 4);
                count += 1;
            }
        }
    });
    assert_eq!(count, 1);
}

#[test]
fn try_field_unset_optional_returns_error() {
    let world = World::new();
    world.entity().set(Position { x: 1, y: 1 });

    let query = world.new_query::<(&Position, Option<&Velocity>)>();
    let mut checked = false;
    query.run(|mut it| {
        while it.next() {
            assert_eq!(
                it.try_field::<Velocity>(1).unwrap_err(),
                FieldError::NoMatchesCount0
            );
            checked = true;
        }
    });
    assert!(checked);
}

#[cfg(feature = "flecs_safety_locks")]
mod safety_locks {
    use super::*;

    #[test]
    #[should_panic]
    fn field_at_mut_dense_conflicts_with_field_mut() {
        let world = World::new();
        world.entity().set(Position { x: 1, y: 1 });

        let query = world.new_query::<&mut Position>();
        query.run(|mut it| {
            while it.next() {
                let pos = it.field_mut::<Position>(0);
                let pos_at = it.field_at_mut::<Position>(0, 0usize);
                core::hint::black_box((pos[0].x, pos_at.x));
            }
        });
    }

    #[test]
    #[should_panic]
    fn field_at_mut_dense_conflicts_with_field_at_mut() {
        let world = World::new();
        world.entity().set(Position { x: 1, y: 1 });
        world.entity().set(Position { x: 2, y: 2 });

        let query = world.new_query::<&mut Position>();
        query.run(|mut it| {
            while it.next() {
                let a = it.field_at_mut::<Position>(0, 0usize);
                let b = it.field_at_mut::<Position>(0, 1usize);
                core::hint::black_box((a.x, b.x));
            }
        });
    }

    #[test]
    fn field_at_dense_read_read_ok() {
        let world = World::new();
        world.entity().set(Position { x: 1, y: 1 });
        world.entity().set(Position { x: 2, y: 2 });

        let query = world.new_query::<&Position>();
        let mut count = 0;
        query.run(|mut it| {
            while it.next() {
                let a = it.field_at::<Position>(0, 0usize);
                let b = it.field_at::<Position>(0, 1usize);
                count += a.x + b.x;
            }
        });
        assert_eq!(count, 3);
    }

    #[test]
    fn try_field_locked_returns_error() {
        let world = World::new();
        world.entity().set(Position { x: 1, y: 1 });

        let query = world.new_query::<&mut Position>();
        let mut checked = false;
        query.run(|mut it| {
            while it.next() {
                let pos = it.field_mut::<Position>(0);
                assert_eq!(it.try_field::<Position>(0).unwrap_err(), FieldError::Locked);
                assert_eq!(
                    it.try_field_mut::<Position>(0).unwrap_err(),
                    FieldError::Locked
                );
                core::hint::black_box(pos[0].x);
                checked = true;
            }
        });
        assert!(checked);
    }
}
