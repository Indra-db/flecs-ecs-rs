use crate::z_ignore_test_common::*;

use flecs_ecs::experimental::prelude::*;
use flecs_ecs::prelude::*;

#[derive(Component)]
struct Toppings {
    value: u32,
}

impl Toppings {
    const BACON: u32 = 0x1;
    const LETTUCE: u32 = 0x2;
    const TOMATO: u32 = 0x4;

    fn new() -> Self {
        Toppings { value: 0 }
    }

    fn add(&mut self, topping: u32) {
        self.value |= topping;
    }

    fn remove(&mut self, topping: u32) {
        self.value &= !topping;
    }

    fn has(&self, topping: u32) -> bool {
        self.value & topping != 0
    }
}

#[derive(Component)]
struct Sandwich {
    toppings: Toppings,
}

impl Sandwich {
    fn new() -> Self {
        Sandwich {
            toppings: Toppings::new(),
        }
    }

    fn add_topping(&mut self, topping: u32) {
        self.toppings.add(topping);
    }

    fn remove_topping(&mut self, topping: u32) {
        self.toppings.remove(topping);
    }

    fn has_topping(&self, topping: u32) -> bool {
        self.toppings.has(topping)
    }
}

fn main() {
    let world = World::new();

    world
        .component::<Toppings>()
        .bit("bacon", Toppings::BACON)
        .bit("lettuce", Toppings::LETTUCE)
        .bit("tomato", Toppings::TOMATO);

    world
        .component::<Sandwich>()
        .member(Toppings::id(), "toppings");

    // Create entity with Sandwich
    let e = world.entity().set(Sandwich {
        toppings: Toppings {
            value: Toppings::BACON | Toppings::LETTUCE,
        },
    });

    // Convert Sandwidth component to flecs expression string
    let val = e.get_ref::<&Sandwich>().unwrap();
    println!("{}", world.to_expr(&*val));

    // Output:
    //  {toppings: lettuce|bacon}
}

#[cfg(feature = "flecs_nightly_tests")]
#[test]
fn test() {
    let output_capture = OutputCapture::capture().unwrap();
    main();
    output_capture.test("reflection_bitmask".to_string());
}
