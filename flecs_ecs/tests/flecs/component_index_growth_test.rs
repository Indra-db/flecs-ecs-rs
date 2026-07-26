use flecs_ecs::experimental::prelude::EntityGuardExt;
use flecs_ecs::prelude::*;
use seq_macro::seq;

seq!(N in 0..1100 {
    #[repr(C)]
    #[derive(Component)]
    pub enum ManyVariantsEnum {
        #(Variant~N,)*
    }
});

#[derive(Component)]
struct HighIndexComponent {
    value: i32,
}

/// Registering a type whose global type index is far beyond a fresh world's
/// component-id cache length must grow the cache up to that index instead of
/// panicking with an out-of-bounds index.
#[test]
fn component_array_growth_covers_high_type_index() {
    let world_a = World::new();
    world_a.component::<ManyVariantsEnum>();

    let world_b = World::new();
    let comp = world_b.component::<HighIndexComponent>();
    assert_ne!(comp.id(), 0);

    let e = world_b.entity().set(HighIndexComponent { value: 7 });
    {
        let c = e.get_ref::<&HighIndexComponent>().unwrap();
        assert_eq!(c.value, 7);
    };
}
