use crate::z_ignore_test_common::*;

use core::ffi::c_void;
use flecs_ecs::experimental::prelude::*;
use flecs_ecs::prelude::*;
use flecs_ecs::sys;

#[derive(Debug, Component)]
pub struct Position {
    pub x: f32,
    pub y: f32,
}

#[derive(Component)]
pub struct Tag;

#[derive(Component)]
pub struct First;

#[derive(Component)]
pub struct Second;

#[derive(Component)]
pub struct Third;

#[derive(Component)]
pub struct Group;

#[extern_abi]
fn callback_group_by_relationship(
    world: *mut sys::ecs_world_t,
    table: *mut sys::ecs_table_t,
    id: u64,
    _group_by_ctx: *mut c_void,
) -> u64 {
    // Use sys::ecs_search to find the target for the relationship in the table
    let mut match_id: sys::ecs_id_t = Default::default();
    let world = unsafe { WorldRef::from_ptr(world) };
    let id = IdView::new_from_id(world, (id, flecs::Wildcard::ID)).id();
    if unsafe { sys::ecs_search(world.world_ptr_mut(), table, *id, &mut match_id) } != -1 {
        *IdView::new_from_id(world, match_id).second_id().id() // First, Second or Third
    } else {
        0
    }
}

fn main() {
    let mut world = World::new();

    // Register components in order so that id for First is lower than Third
    world.component::<First>();
    world.component::<Second>();
    world.component::<Third>();

    // Grouped query
    let query = world
        .query::<&Position>()
        .group_by_fn(Group, Some(callback_group_by_relationship))
        .build();

    // Create entities in 6 different tables with 3 group ids
    world
        .entity()
        .add((Group, Third))
        .set(Position { x: 1.0, y: 1.0 });
    world
        .entity()
        .add((Group, Second))
        .set(Position { x: 2.0, y: 2.0 });
    world
        .entity()
        .add((Group, First))
        .set(Position { x: 3.0, y: 3.0 });

    world
        .entity()
        .add((Group, Third))
        .set(Position { x: 4.0, y: 4.0 })
        .add(Tag);
    world
        .entity()
        .add((Group, Second))
        .set(Position { x: 5.0, y: 5.0 })
        .add(Tag);
    world
        .entity()
        .add((Group, First))
        .set(Position { x: 6.0, y: 6.0 })
        .add(Tag);

    println!();

    // The query cache now looks like this:
    //  - group First:
    //     - table [Position, (Group, First)]
    //     - table [Position, Tag, (Group, First)]
    //
    //  - group Second:
    //     - table [Position, (Group, Second)]
    //     - table [Position, Tag, (Group, Second)]
    //
    //  - group Third:
    //     - table [Position, (Group, Third)]
    //     - table [Position, Tag, (Group, Third)]
    //

    query.run_exclusive(&mut world, |mut it| {
        while it.next() {
            let world = it.world();
            let group = world.entity_from_id(it.group_id());
            let pos = it.field::<Position>(0);

            println!(
                "Group: {:?} - Table: [{:?}]",
                group.path().unwrap(),
                it.archetype()
            );

            for i in it.iter() {
                println!(" [{:?}]", pos[i]);
            }

            println!();
        }
    });

    // Output:
    //  Group: "::First" - Table: [Position, (Group,First)]
    //  [Position { x: 3.0, y: 3.0 }]
    //
    //  Group: "::First" - Table: [Position, Tag, (Group,First)]
    //  [Position { x: 6.0, y: 6.0 }]
    //
    //  Group: "::Second" - Table: [Position, (Group,Second)]
    //  [Position { x: 2.0, y: 2.0 }]
    //
    //  Group: "::Second" - Table: [Position, Tag, (Group,Second)]
    //  [Position { x: 5.0, y: 5.0 }]
    //
    //  Group: "::Third" - Table: [Position, (Group,Third)]
    //  [Position { x: 1.0, y: 1.0 }]
    //
    //  Group: "::Third" - Table: [Position, Tag, (Group,Third)]
    //  [Position { x: 4.0, y: 4.0 }]
}

#[cfg(feature = "flecs_nightly_tests")]
#[test]
fn test() {
    let output_capture = OutputCapture::capture().unwrap();
    main();
    output_capture.test("query_group_by_custom".to_string());
}
