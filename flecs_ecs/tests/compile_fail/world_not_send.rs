//! `World` must not be `Send`: it holds unsynchronized state (`WorldCtx`) and
//! may own `!Send` components, so it cannot be moved to another thread.

use flecs_ecs::prelude::*;

fn main() {
    let world = World::new();
    std::thread::spawn(move || {
        world.entity();
    });
}
