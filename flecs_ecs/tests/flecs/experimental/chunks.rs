//! Deliverable 3: chunk cursor + `each!` macro.

use super::{Position, Velocity};
use flecs_ecs::core::*;
use flecs_ecs::each;
use flecs_ecs::experimental::prelude::*;
use flecs_ecs::macros::query;

fn seed(world: &World, n: i32) {
    for i in 0..n {
        world
            .entity()
            .set(Position { x: i, y: 0 })
            .set(Velocity { x: 2, y: 3 });
    }
}

#[test]
fn chunks_cursor_yields_column_slices() {
    let mut world = World::new();
    seed(&world, 6);
    let q = query!(world, &mut Position, &Velocity).build();

    let mut rows = 0;
    let mut cursor = q.chunks(&mut world);
    while let Some((pos, vel)) = cursor.next() {
        assert_eq!(pos.len(), vel.len());
        for i in 0..pos.len() {
            pos[i].x += vel[i].x;
            rows += 1;
        }
    }
    assert_eq!(rows, 6);
}

#[test]
fn chunks_for_each_terminal() {
    let mut world = World::new();
    seed(&world, 4);
    let q = query!(world, &mut Position, &Velocity).build();

    q.chunks(&mut world).for_each(|(pos, vel)| {
        for i in 0..pos.len() {
            pos[i].y += vel[i].y;
        }
    });

    let mut ys = Vec::new();
    q.each(|(p, _)| ys.push(p.y));
    assert_eq!(ys, vec![3, 3, 3, 3]);
}

#[test]
fn each_macro_tuple_body() {
    let mut world = World::new();
    seed(&world, 5);
    let q = query!(world, &mut Position, &Velocity).build();

    each!((pos, vel) in q.chunks(&mut world) {
        pos.x += vel.x;
    });

    let mut xs = Vec::new();
    q.each(|(p, _)| xs.push(p.x));
    xs.sort_unstable();
    assert_eq!(xs, vec![2, 3, 4, 5, 6]);
}

#[test]
fn each_macro_single_body() {
    let mut world = World::new();
    seed(&world, 3);
    let q = query!(world, &mut Position).build();

    each!(pos in q.chunks(&mut world) {
        pos.y = 99;
    });

    let mut ys = Vec::new();
    q.each(|p| ys.push(p.y));
    assert_eq!(ys, vec![99, 99, 99]);
}

#[test]
fn each_macro_break_continue() {
    let mut world = World::new();
    seed(&world, 10);
    let q = query!(world, &mut Position, &Velocity).build();

    let mut touched = 0;
    each!((pos, vel) in q.chunks(&mut world) {
        if pos.x % 2 == 0 {
            continue;
        }
        if touched >= 3 {
            break;
        }
        pos.x += vel.x;
        touched += 1;
    });
    assert!(touched <= 3);
}

#[test]
fn each_macro_question_mark() {
    fn run(world: &mut World) -> Result<i32, &'static str> {
        let q = query!(world, &mut Position, &Velocity).build();
        let mut sum = 0;
        each!((pos, vel) in q.chunks(world) {
            let step: Result<i32, &'static str> =
                if vel.x >= 0 { Ok(vel.x) } else { Err("negative") };
            pos.x += step?;
            sum += pos.x;
        });
        Ok(sum)
    }

    let mut world = World::new();
    seed(&world, 4);
    assert!(run(&mut world).is_ok());
}

#[test]
fn each_macro_matches_locked_each() {
    let mut world = World::new();
    seed(&world, 7);
    let q = query!(world, &mut Position, &Velocity).build();

    // macro path
    each!((pos, vel) in q.chunks(&mut world) {
        pos.x += vel.x;
    });
    let mut via_macro = Vec::new();
    q.each(|(p, _)| via_macro.push(p.x));
    via_macro.sort_unstable();

    // reference: closure each doing the same on a fresh world
    let world2 = World::new();
    seed(&world2, 7);
    let q2 = query!(world2, &mut Position, &Velocity).build();
    q2.each(|(p, v)| p.x += v.x);
    let mut via_each = Vec::new();
    q2.each(|(p, _)| via_each.push(p.x));
    via_each.sort_unstable();

    assert_eq!(via_macro, via_each);
}

#[test]
fn each_macro_body_panic_does_not_wedge() {
    let mut world = World::new();
    seed(&world, 4);
    let q = query!(world, &mut Position, &Velocity).build();

    let result = std::panic::catch_unwind(core::panic::AssertUnwindSafe(|| {
        each!((pos, vel) in q.chunks(&mut world) {
            pos.x += vel.x;
            panic!("boom mid-iteration");
        });
    }));
    assert!(result.is_err());

    // The iterator was finalized on unwind and no lock leaked: a fresh locked
    // each still works.
    let mut count = 0;
    q.each(|(_, _)| count += 1);
    assert_eq!(count, 4);
}

#[test]
#[should_panic]
fn chunks_rejects_non_disjoint_query() {
    // Same component read and written via different sources cannot be proven
    // disjoint, so chunks() must refuse to hand out aliasing slices.
    let mut world = World::new();
    world.entity().set(Position::default());
    // (&mut Position, &Position): duplicate concrete id, one mutable.
    let q = query!(world, &mut Position, &Position).build();
    let _cursor = q.chunks(&mut world);
}

#[test]
#[should_panic(expected = "chunks() requires the query's own world")]
fn chunks_foreign_world_panics() {
    let world_a = World::new();
    world_a
        .entity()
        .set(Position::default())
        .set(Velocity::default());
    let q = query!(world_a, &mut Position, &Velocity).build();
    let mut world_b = World::new();
    let _ = q.chunks(&mut world_b);
}
