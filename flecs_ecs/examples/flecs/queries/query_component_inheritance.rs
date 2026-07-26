use crate::z_ignore_test_common::*;

use flecs_ecs::experimental::prelude::*;
use flecs_ecs::prelude::*;
// This example shows how queries can be used to match simple inheritance trees.

#[derive(Component)]
struct Unit;

#[derive(Component)]
struct CombatUnit;

#[derive(Component)]
struct MeleeUnit;

#[derive(Component)]
struct RangedUnit;

#[derive(Component)]
struct Warrior;

#[derive(Component)]
struct Wizard;

#[derive(Component)]
struct Marksman;

#[derive(Component)]
struct BuilderX;

fn main() {
    let mut world = World::new();

    // Make the ECS aware of the inheritance relationships. Note that IsA
    // relationship used here is the same as in the prefab example.
    world.component::<CombatUnit>().is_a(Unit);
    world.component::<MeleeUnit>().is_a(CombatUnit);
    world.component::<RangedUnit>().is_a(CombatUnit);

    world.component::<Warrior>().is_a(MeleeUnit);
    world.component::<Wizard>().is_a(RangedUnit);
    world.component::<Marksman>().is_a(RangedUnit);
    world.component::<BuilderX>().is_a(Unit);

    // Create a few units
    world.entity_named("warrior_1").add(Warrior);
    world.entity_named("warrior_2").add(Warrior);

    world.entity_named("marksman_1").add(Marksman);
    world.entity_named("marksman_2").add(Marksman);

    world.entity_named("wizard_1").add(Wizard);
    world.entity_named("wizard_2").add(Wizard);

    world.entity_named("builder_1").add(BuilderX);
    world.entity_named("builder_2").add(BuilderX);

    // Create a rule to find all ranged units
    let r = world.query::<()>().with(RangedUnit).build();

    // Iterate the rule
    r.each_entity_exclusive(&mut world, |e, rangedunit| {
        println!("Unit {} found", e.name());
    });

    // Output:
    //  Unit wizard_1 found
    //  Unit wizard_2 found
    //  Unit marksman_1 found
    //  Unit marksman_2 found
}

#[cfg(feature = "flecs_nightly_tests")]
#[test]
fn test() {
    let output_capture = OutputCapture::capture().unwrap();
    main();
    output_capture.test("query_component_inheritance".to_string());
}
