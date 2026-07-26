use crate::z_ignore_test_common::*;

use core::ffi::c_void;
use flecs_ecs::experimental::prelude::*;
use flecs_ecs::prelude::*;
use flecs_ecs::sys;
use std::sync::Mutex;

#[derive(Debug, Component)]
pub struct Position {
    pub x: f32,
    pub y: f32,
}

#[derive(Component)]
pub struct Tag;

static GROUP_COUNTER: Mutex<i32> = Mutex::new(0);

struct GroupCtx {
    counter: i32,
}

#[derive(Component)]
pub struct First;

#[derive(Component)]
pub struct Second;

#[derive(Component)]
pub struct Third;

#[derive(Component)]
pub struct Group;

#[extern_abi]
fn callback_group_create(
    world: *mut sys::ecs_world_t,
    group_id: u64,
    _group_by_ctx: *mut c_void,
) -> *mut c_void {
    let world_ref = unsafe { WorldRef::from_ptr(world) };
    println!(
        "Group created: {:?}",
        world_ref.world().entity_from_id(group_id).name()
    );

    println!();

    let mut counter = GROUP_COUNTER.lock().unwrap();
    *counter += 1;

    // Return data that will be associated with the group
    let ctx = Box::new(GroupCtx { counter: *counter });

    Box::into_raw(ctx) as *mut core::ffi::c_void // Cast to make sure function type matches
}

#[extern_abi]
fn callback_group_delete(
    world: *mut sys::ecs_world_t,
    group_id: u64,
    _ctx: *mut c_void,
    _group_by_ctx: *mut c_void,
) {
    let world_ref = unsafe { WorldRef::from_ptr(world) };
    println!(
        "Group deleted: {:?}",
        world_ref.world().entity_from_id(group_id).name()
    );

    // if you have any data associated with the group, you need to free it
    // or use the callback group_by_ctx where you pass a context to the callback
}

fn main() {
    let mut world = World::new();

    // Register components in order so that id for First is lower than Third
    world.component::<First>();
    world.component::<Second>();
    world.component::<Third>();

    // Grouped query
    let query = world
        .query::<(&Position,)>()
        .group_by(Group)
        // Callback invoked when a new group is created
        .on_group_create(Some(callback_group_create))
        // Callback invoked when a group is deleted
        .on_group_delete(Some(callback_group_delete))
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

            let ctx = unsafe { &*(query.group_context(group) as *mut GroupCtx) };
            println!(
                "Group: {:?} - Table: [{:?}] - Counter: {}",
                group.path().unwrap(),
                it.archetype(),
                ctx.counter
            );

            for i in it.iter() {
                println!(" [{:?}]", pos[i]);
            }

            println!();
        }
    });

    // Deleting the query will call the on_group_deleted callback
    query.destruct();

    // Output:
    //  Group created: "Third"
    //  Group created: "Second"
    //  Group created: "First"
    //
    //  Group: "::First" - Table: [Position, (Group,First)] - Counter: 3
    //   [Position { x: 3.0, y: 3.0 }]
    //
    //  Group: "::First" - Table: [Position, Tag, (Group,First)] - Counter: 3
    //   [Position { x: 6.0, y: 6.0 }]
    //
    //  Group: "::Second" - Table: [Position, (Group,Second)] - Counter: 2
    //   [Position { x: 2.0, y: 2.0 }]
    //
    //  Group: "::Second" - Table: [Position, Tag, (Group,Second)] - Counter: 2
    //   [Position { x: 5.0, y: 5.0 }]
    //
    //  Group: "::Third" - Table: [Position, (Group,Third)] - Counter: 1
    //   [Position { x: 1.0, y: 1.0 }]
    //
    //  Group: "::Third" - Table: [Position, Tag, (Group,Third)] - Counter: 1
    //   [Position { x: 4.0, y: 4.0 }]
    //
    //  Group deleted: "Second"
    //  Group deleted: "First"
    //  Group deleted: "Third"
}

#[cfg(feature = "flecs_nightly_tests")]
#[test]
fn test() {
    let output_capture = OutputCapture::capture().unwrap();
    main();
    output_capture.test("query_group_by_callbacks".to_string());
}
