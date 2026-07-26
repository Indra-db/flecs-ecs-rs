//! This example shows one possible way to implement an inventory system
//!  using ECS relationships.

use flecs_ecs::experimental::prelude::*;
use flecs_ecs::prelude::*;

//MARK: Components

// Inventory tags, relationships

/// Base item type
#[derive(Component, Debug)]
struct Item;

/// Container tag
#[derive(Component, Debug)]
struct Container;

/// Inventory tag
#[derive(Component, Debug)]
struct Inventory;

#[derive(Component, Debug)]
struct ContainedBy;

// Item / unit properties

/// Item is active/worn
#[derive(Component, Debug)]
struct Active;

/// Number of items the instance represents
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
struct Amount {
    amount: i32,
}

/// Health of the item
#[derive(Component, Debug, Clone)]
struct Health {
    value: i32,
}

/// Amount of damage an item deals per use
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
struct Attack {
    value: i32,
}

// Items
#[derive(Component, Debug)]
struct Sword;
#[derive(Component, Debug)]
struct Armor;
#[derive(Component, Debug)]
struct Coin;

// Item prefab types

#[derive(Component, Debug)]
struct WoodenSword;
#[derive(Component, Debug)]
struct IronSword;
#[derive(Component, Debug)]
struct WoodenArmor;
#[derive(Component, Debug)]
struct IronArmor;

//MARK: Utility Functions

/// Find the "kind" of an item (e.g., Sword, Armor, Coin) by looking for something
/// that inherits from `Item`.
fn item_kind(item: EntityView<'_>) -> Option<Entity> {
    let world = item.world();
    let mut result_entity: Option<Entity> = None;

    item.each_component(|comp| {
        if comp.is_entity() {
            // If id is a plain entity (component), check if component inherits
            // from Item
            if comp.entity_view().has((id::<flecs::IsA>(), Item)) {
                result_entity = Some(comp.entity_view().id());
            }
        } else if comp.is_pair() {
            // If item has a base entity, check if the base has an attribute
            // that is an Item.
            if comp.first_id() == flecs::IsA::ID
                && let Some(base_kind) = item_kind(comp.second_id())
            {
                result_entity = Some(base_kind);
            }
        }
    });

    result_entity
}

/// Return the "name of the prefab" (e.g., `WoodenSword`) if found,
/// otherwise the more generic kind (e.g., `Sword`).
fn item_name(item: EntityView<'_>) -> Option<String> {
    let world = item.world();
    let mut result_name: Option<String> = None;

    item.each_component(|comp| {
        if comp.is_entity() {
            if comp.entity_view().has((id::<flecs::IsA>(), Item)) {
                result_name = comp.entity_view().get_name();
            }
        } else if comp.is_pair()
            && comp.first_id() == flecs::IsA::ID
            && let Some(base_kind) = item_kind(comp.second_id())
        {
            result_name = comp.second_id().get_name();
        }
    });

    result_name
}

/// If entity is not a Container, get its Inventory target (the actual container).
fn get_container(container: EntityView<'_>) -> Entity {
    let world = container.world();
    if container.has(Container) {
        return container.id();
    }
    container.target(Inventory, 0).unwrap().id()
}

/// Iterate all items in an inventory
fn for_each_item<F>(container: EntityView<'_>, mut func: F)
where
    F: FnMut(flecs_ecs::core::EntityView<'_>, ()),
{
    let world = container.world();
    world
        .query::<()>()
        .with((ContainedBy, container))
        .build()
        .each_entity(func);
}

/// Find item in the inventory of the specified "kind".
/// If `active_required == true`, only return items that have `Active`.
fn find_item_w_kind(
    container: EntityView<'_>,
    kind: Entity,
    active_required: bool,
) -> Option<Entity> {
    let world = container.world();

    let mut result: Option<Entity> = None;

    let container = world.entity_from_id(get_container(container));

    for_each_item(container, |item, _| {
        // Check if we should only return active items. This is useful when
        // searching for an item that needs to be equipped.
        if active_required && !item.has(Active) {
            return;
        }

        if let Some(ik) = item_kind(item)
            && ik == kind
        {
            result = Some(item.id());
        }
    });

    result
}

/// Transfer a single item to a different container.
fn transfer_item(container: EntityView<'_>, item: EntityView<'_>) {
    let world = container.world();

    let amt = item.cloned::<&Amount>().unwrap_or(Amount { amount: 1 });

    #[allow(clippy::redundant_else)]
    if amt.amount > 0 {
        // If item has amount we need to check if the container already has an
        // item of this kind, and increase the value.
        let ik = item_kind(item).unwrap();
        let dst_item = find_item_w_kind(container, ik, false);

        if let Some(dst_item) = dst_item {
            // If a matching item was found, increase its amount
            let mut dst_amt = world
                .entity_from_id(dst_item)
                .get::<&mut Amount>()
                .unwrap();
            dst_amt.amount += amt.amount;
            drop(dst_amt); // release the write guard before the structural destruct
            item.destruct();
            return;
        } else {
            // If no matching item was found, fallthrough which will move the
            // item from the src container to the dst container
        }
    }

    // Move item to target container (replaces previous ContainedBy, if any)
    item.add((ContainedBy, container));
}

/// Move all items from `src` container to `dst` container.
fn transfer_items(dst: EntityView<'_>, src: EntityView<'_>) {
    let world = dst.world();
    println!(">> Transfer items from {} to {}\n", src.name(), dst.name());

    // Defer is recommended in Flecs so you can safely modify ECS while iterating.
    // In flecs-rust you can do `world.defer(|w| { ... })`.
    world.defer(|| {
        let dst_container = world.entity_from_id(get_container(dst));
        let src_container = world.entity_from_id(get_container(src));

        for_each_item(src_container, |item, _| {
            transfer_item(dst_container, item);
        });
    });
}

/// Attack `player` with `weapon`.
fn attack(player: EntityView<'_>, weapon: EntityView<'_>) {
    let world = player.world();

    println!(
        ">> {} is attacked with a {}!",
        player.name(),
        item_name(weapon).unwrap_or("UnknownItem".to_string())
    );

    let atk = weapon.cloned::<&Attack>();

    if atk.is_none() {
        // A weapon without Attack power? Odd.
        println!(" - the weapon is a dud");
        return;
    }

    let mut att_value = atk.unwrap().value;

    // Get armor item, if player has equipped any
    if let Some(armor_e) = find_item_w_kind(player, world.component_id::<Armor>(), true) {
        let armor_e = world.entity_from_id(armor_e);
        let (health,) = armor_e.get::<(Option<&mut Health>,)>().unwrap();
        if let Some(mut armor_health) = health {
            println!(
                " - {} defends with {} ({} health)",
                player.name(),
                item_name(armor_e).unwrap(),
                armor_health.value
            );

            // Subtract attack from armor health. If armor health goes below
            // zero, delete the armor and carry over remaining attack points.
            armor_health.value -= att_value;

            if armor_health.value <= 0 {
                att_value = -armor_health.value;
                let armor_name = item_name(armor_e).unwrap();
                drop(armor_health); // release the guard before the structural destruct
                armor_e.destruct();
                println!(" - {armor_name} is destroyed!");
            } else {
                println!(
                    " - {} has {} health left after taking {} damage",
                    item_name(armor_e).unwrap(),
                    armor_health.value,
                    att_value
                );
                att_value = 0;
            }
        } else {
            // Armor without Defense power? Odd.
            println!(" - the {} armor is a dud", item_name(armor_e).unwrap());
        }
    } else {
        // Brave but stupid
        println!(" - {} fights without armor!", player.name());
    }

    // For each usage of the weapon, subtract one from its health
    let mut weapon_health = weapon.get::<&mut Health>().unwrap();
    if weapon_health.value > 0 {
        weapon_health.value -= 1;
        if weapon_health.value == 0 {
            drop(weapon_health); // release the guard before the structural destruct
            println!(" - {} is destroyed!", item_name(weapon).unwrap());
            weapon.destruct();
        } else {
            println!(
                " - {} has {} uses left",
                item_name(weapon).unwrap(),
                weapon_health.value
            );
        }
    }

    // If armor didn't counter the whole attack, subtract from the player health
    if att_value > 0 {
        let mut player_health = player.get::<&mut Health>().unwrap();
        player_health.value -= att_value;
        if player_health.value <= 0 {
            drop(player_health); // release the guard before the structural destruct
            println!(" - {} died!", player.name());
            player.destruct();
        } else {
            println!(
                " - {} has {} health left after taking {} damage",
                player.name(),
                player_health.value,
                att_value
            );
        }
    }

    println!();
}

/// Print items in a container / inventory.
fn print_items(container: EntityView<'_>) {
    println!("-- {}'s inventory:", container.name());

    let world = container.world();
    let mut count = 0;

    // In case the player entity was provided, make sure we're working
    // with its inventory entity.
    let container = world.entity_from_id(get_container(container));

    for_each_item(container, |item, _| {
        // Items with an Amount component fill up a single inventory slot but
        // represent multiple instances, like coins.
        let amount = item
            .cloned::<&Amount>()
            .unwrap_or(Amount { amount: 1 })
            .amount;
        println!(
            " - {} {} ({})",
            amount,
            item_name(item).unwrap_or("UnknownItem".to_string()),
            world.entity_from_id(item_kind(item).unwrap()).name()
        );

        count += 1;
    });

    if count == 0 {
        println!(" - << empty >>");
    }

    println!();
}

//MARK: ECS Modules

#[derive(Component)]
pub struct InventoryComponentsModule;

#[derive(Component)]
pub struct InventoryModule;

impl Module for InventoryComponentsModule {
    fn module(world: &World) {
        world.module::<InventoryComponentsModule>("inventory::components");

        world.component::<Item>();
        world.component::<Container>();
        world.component::<Inventory>();
        // Item can only be contained by one container
        world
            .component::<ContainedBy>()
            .add_trait::<flecs::Exclusive>();
    }
}

impl Module for InventoryModule {
    fn module(world: &World) {
        world.module::<InventoryModule>("inventory::systems");
        world.import::<InventoryComponentsModule>();
    }
}

#[derive(Component)]
pub struct ItemComponentsModule;

impl Module for ItemComponentsModule {
    fn module(world: &World) {
        world.module::<ItemComponentsModule>("item::components");

        world.component::<Active>();
        world.component::<Amount>();

        //health gets copied to instance, don't share, defaults to Onstantiate Copy,
        world.component::<Health>();

        world
            .component::<Attack>()
            .add_trait::<(flecs::OnInstantiate, flecs::Inherit)>();

        world
            .component::<Sword>()
            .is_a(Item)
            .add_trait::<(flecs::OnInstantiate, flecs::Inherit)>();

        world
            .component::<Armor>()
            .is_a(Item)
            .add_trait::<(flecs::OnInstantiate, flecs::Inherit)>();
        world
            .component::<Coin>()
            .is_a(Item)
            .add_trait::<(flecs::OnInstantiate, flecs::Inherit)>();

        //register item prefabs
        world
            .prefab_type::<WoodenSword>()
            .add(Sword)
            .set(Attack { value: 1 })
            // copy to instance, don't share
            .set(Health { value: 5 });

        world
            .prefab_type::<IronSword>()
            .add(Sword)
            .set(Attack { value: 4 })
            .set(Health { value: 10 });

        world
            .prefab_type::<WoodenArmor>()
            .add(Armor)
            .set(Health { value: 10 });

        world
            .prefab_type::<IronArmor>()
            .add(Armor)
            .set(Health { value: 20 });
    }
}

#[derive(Component)]
pub struct ItemModule;

impl Module for ItemModule {
    fn module(world: &World) {
        world.module::<ItemModule>("item::systems");
        world.import::<ItemComponentsModule>();
    }
}

//MARK: Main

fn main() {
    let mut world = World::new();

    world.import::<InventoryModule>();
    world.import::<ItemModule>();

    // Create a loot box with items
    let loot_box = world
        .entity_named("Chest")
        .add(Container)
        .with_first(ContainedBy, || {
            world.entity().is_a(IronSword);
            world.entity().is_a(WoodenArmor);
            world.entity().add(Coin).set(Amount { amount: 30 });
        });

    // Create a player entity with an inventory
    let player = world.entity_named("Player").set(Health { value: 10 }).add((
        Inventory,
        world.entity().add(Container).with_first(ContainedBy, || {
            world.entity().add(Coin).set(Amount { amount: 20 });
        }),
    ));

    // Print items in loot box
    print_items(loot_box);

    // Print items in player inventory
    print_items(player);

    // Copy items from loot box to player inventory
    transfer_items(player, loot_box);

    // Print items in player inventory after transfer
    print_items(player);

    // Print items in loot box after transfer
    print_items(loot_box);

    // Find armor entity & equip it
    if let Some(armor) = find_item_w_kind(player, world.component_id::<Armor>(), false) {
        world.entity_from_id(armor).add(Active);
    }

    // Create a weapon to attack the player with
    let my_sword = world.entity().is_a(IronSword);

    // Attack player
    attack(player, my_sword);
    attack(player, my_sword);
    attack(player, my_sword);
    attack(player, my_sword);
    attack(player, my_sword);

    // Output:
    // -- Chest's inventory:
    //  - 1 IronSword (Sword)
    //  - 1 WoodenArmor (Armor)
    //  - 30 Coin (Coin)

    // -- Player's inventory:
    //  - 20 Coin (Coin)

    // >> Transfer items from Chest to Player

    // -- Player's inventory:
    //  - 50 Coin (Coin)
    //  - 1 IronSword (Sword)
    //  - 1 WoodenArmor (Armor)

    // -- Chest's inventory:
    //  - << empty >>

    // >> Player is attacked with a IronSword!
    //  - Player defends with WoodenArmor (10 health)
    //  - WoodenArmor has 6 health left after taking 4 damage
    //  - IronSword has 9 uses left

    // >> Player is attacked with a IronSword!
    //  - Player defends with WoodenArmor (6 health)
    //  - WoodenArmor has 2 health left after taking 4 damage
    //  - IronSword has 8 uses left

    // >> Player is attacked with a IronSword!
    //  - Player defends with WoodenArmor (2 health)
    //  - WoodenArmor is destroyed!
    //  - IronSword has 7 uses left
    //  - Player has 8 health left after taking 2 damage

    // >> Player is attacked with a IronSword!
    //  - Player fights without armor!
    //  - IronSword has 6 uses left
    //  - Player has 4 health left after taking 4 damage

    // >> Player is attacked with a IronSword!
    //  - Player fights without armor!
    //  - IronSword has 5 uses left
    //  - Player died!
}
