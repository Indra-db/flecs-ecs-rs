use crate::common_test::*;
use flecs_ecs::experimental::prelude::EntityGuardExt;

fn cloned_tuple_is_nameable_outside_the_crate<T: flecs_ecs::core::ClonedTuple>() {}

#[test]
#[should_panic(expected = "same component data")]
fn query_duplicate_mut_mut_terms_panics() {
    let world = World::new();
    world.entity().set(Position { x: 1, y: 2 });
    world
        .query::<(&mut Position, &mut Position)>()
        .build()
        .each(|(_a, _b)| {});
}

#[test]
#[should_panic(expected = "same component data")]
fn query_duplicate_mut_read_terms_panics() {
    let world = World::new();
    world.entity().set(Position { x: 1, y: 2 });
    world
        .query::<(&mut Position, &Position)>()
        .build()
        .each(|(_a, _b)| {});
}

#[test]
fn query_duplicate_read_read_terms_allowed() {
    let world = World::new();
    world.entity().set(Position { x: 1, y: 2 });
    let mut count = 0;
    world
        .query::<(&Position, &Position)>()
        .build()
        .each(|(a, b)| {
            assert_eq!(a.x, b.x);
            count += 1;
        });
    assert_eq!(count, 1);
}

#[test]
fn query_multi_src_read_read_same_entity_allowed() {
    let world = World::new();
    let e = world.entity().set(Position { x: 1, y: 2 });
    let query = world
        .query::<(&Position, &Position)>()
        .term_at(1)
        .set_src(e)
        .build();
    let mut count = 0;
    query.each(|(this_pos, src_pos)| {
        assert_eq!(this_pos.x, src_pos.x);
        count += 1;
    });
    assert_eq!(count, 1);
}

#[test]
#[should_panic(expected = "same component data")]
fn query_multi_src_mut_read_same_entity_panics() {
    let world = World::new();
    let e = world.entity().set(Position { x: 1, y: 2 });
    world
        .query::<(&mut Position, &Position)>()
        .term_at(1)
        .set_src(e)
        .build()
        .each(|(_a, _b)| {});
}

#[test]
fn query_multi_src_mut_mut_different_entities_allowed() {
    let world = World::new();
    let e1 = world.entity().set(Position { x: 1, y: 2 }).add(Tag::id());
    let e2 = world.entity().set(Position { x: 10, y: 20 });
    let _ = e2;
    let query = world
        .query::<(&mut Position, &mut Position)>()
        .with(Tag::id())
        .term_at(1)
        .set_src(e2)
        .build();
    let mut count = 0;
    query.each(|(this_pos, src_pos)| {
        this_pos.x += src_pos.x;
        src_pos.y += 1;
        count += 1;
    });
    assert_eq!(count, 1);
    {
        let p = e1.get::<&Position>().unwrap();
        assert_eq!(p.x, 11);
    };
    {
        let p = e2.get::<&Position>().unwrap();
        assert_eq!(p.y, 21);
    };
}

#[test]
fn query_same_component_different_source_allowed() {
    let world = World::new();
    let parent = world.entity().set(Position { x: 1, y: 2 });
    world.entity().child_of(parent).set(Position { x: 3, y: 4 });
    let query = world
        .query::<(&Position, &mut Position)>()
        .term_at(1)
        .parent()
        .cascade()
        .build();
    let mut count = 0;
    query.each(|(child_pos, parent_pos)| {
        assert_eq!(child_pos.x, 3);
        assert_eq!(parent_pos.x, 1);
        count += 1;
    });
    assert_eq!(count, 1);
}

#[test]
#[should_panic(expected = "same sparse component")]
fn query_duplicate_mut_sparse_terms_panics() {
    let world = World::new();
    world.component::<Position>().add_trait::<flecs::Sparse>();
    world.entity().set(Position { x: 1, y: 2 });
    world
        .query::<(&mut Position, &mut Position)>()
        .build()
        .each(|(_a, _b)| {});
}

#[test]
#[should_panic(expected = "more than once with at least one mutable reference")]
fn entity_get_duplicate_mut_mut_panics() {
    let world = World::new();
    let entity = world.entity().set(Position { x: 1, y: 2 });
    {
        let (_a, _b) = entity
            .get::<(&mut Position, &mut Position)>()
            .unwrap();
    };
}

#[test]
#[should_panic(expected = "more than once with at least one mutable reference")]
fn entity_try_get_duplicate_mut_read_panics() {
    let world = World::new();
    let entity = world.entity().set(Position { x: 1, y: 2 });
    {
        let (_a, _b) = entity.get::<(&mut Position, &Position)>().unwrap();
    };
}

#[test]
fn entity_get_duplicate_read_read_allowed() {
    let world = World::new();
    let entity = world.entity().set(Position { x: 1, y: 2 });
    let mut count = 0;
    {
        let (a, b) = entity.get::<(&Position, &Position)>().unwrap();
        assert_eq!(a.x, b.x);
        count += 1;
    };
    assert_eq!(count, 1);
}

#[test]
#[should_panic(expected = "more than once with at least one mutable reference")]
fn world_get_duplicate_mut_read_panics() {
    let world = World::new();
    world.set(Position { x: 1, y: 2 });
    world
        .entity_from_id(Position::entity_id(&world))
        .get::<(&mut Position, &Position)>();
}
