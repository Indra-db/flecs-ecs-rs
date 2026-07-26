use crate::z_ignore_test_common::*;

use flecs_ecs::experimental::prelude::*;
use flecs_ecs::prelude::*;

// This example demonstrates how to iterate over all active groups of a
// grouped query using each_group(), and then iterate the entities within each
// group with set_group(). This is useful when an application needs to do
// per-group logic before/after processing the entities in that group.

#[derive(Debug, Component)]
pub struct Position {
    pub x: f64,
    pub y: f64,
}

fn main() {
    let mut world = World::new();

    let asset_a = world.entity_named("Asset_A");
    let asset_b = world.entity_named("Asset_B");
    let asset_c = world.entity_named("Asset_C");

    world
        .entity()
        .is_a(asset_a)
        .set(Position { x: 1.0, y: 1.0 });
    world
        .entity()
        .is_a(asset_a)
        .set(Position { x: 2.0, y: 2.0 });
    world
        .entity()
        .is_a(asset_b)
        .set(Position { x: 3.0, y: 3.0 });
    world
        .entity()
        .is_a(asset_c)
        .set(Position { x: 4.0, y: 4.0 });
    world
        .entity()
        .is_a(asset_c)
        .set(Position { x: 5.0, y: 5.0 });
    world
        .entity()
        .is_a(asset_c)
        .set(Position { x: 6.0, y: 6.0 });

    // Group entities by their IsA target. The default group_by callback uses
    // the relationship's target as the group ID.
    let query = world.query::<&Position>().group_by(flecs::IsA).build();

    // Iterate all active groups, then iterate entities for each group with
    // set_group(). The group ID here is the asset entity (the target of IsA).
    let mut groups: Vec<Entity> = Vec::new();
    query.each_group(|group_id, _group_ctx| {
        groups.push(group_id);
    });

    for group_id in groups {
        let asset = world.entity_from_id(group_id);
        println!("Group {}:", asset.path().unwrap());

        query
            .with_group(group_id)
            .each_entity_exclusive(&mut world, |e, p| {
                println!(" - {}: {{{}, {}}}", e.path().unwrap(), p.x, p.y);
            });
    }

    // Output (group iteration order is unspecified):
    //  Group ::Asset_B:
    //   - #501: {3, 3}
    //  Group ::Asset_A:
    //   - #499: {1, 1}
    //   - #500: {2, 2}
    //  Group ::Asset_C:
    //   - #502: {4, 4}
    //   - #503: {5, 5}
    //   - #504: {6, 6}
}

#[cfg(feature = "flecs_nightly_tests")]
#[test]
fn test() {
    let output_capture = OutputCapture::capture().unwrap();
    main();
    output_capture.test("query_group_iter_all".to_string());
}
