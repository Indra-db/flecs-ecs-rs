use core::hint::black_box;
pub use core::time::Duration;
pub use criterion::{Criterion, criterion_group, criterion_main};
pub use flecs_ecs::experimental::prelude::EntityGuardExt;
pub use flecs_ecs::macros::*;
pub use flecs_ecs::prelude::*;
use flecs_ecs::sys;
pub use seq_macro::seq;
pub use std::time::Instant;

pub const ENTITY_COUNT: u32 = 1000;
pub const QUERY_ENTITY_COUNT: u32 = 65536;

seq!(P in 1..=64 {
    #[derive(Debug, Default, Clone, Component)]
    pub struct C~P(pub u32);

    #[derive(Debug, Default, Clone, Component)]
    pub struct T~P;
});

#[derive(Debug, Default, Component, Clone)]
pub struct Position {
    pub x: f32,
    pub y: f32,
}

#[derive(Debug, Default, Component, Clone)]
pub struct Velocity {
    pub x: f32,
    pub y: f32,
}

pub fn flip_coin() -> bool {
    unsafe { libc::rand() % 2 == 0 }
    //rand::random::<bool>()
}

#[allow(unused)]
fn criterion_config() -> Criterion {
    Criterion::default()
        .warm_up_time(Duration::from_secs_f32(0.5))
        .measurement_time(Duration::from_secs(1))
        .sample_size(50)
}

pub fn reset_srand() {
    unsafe {
        libc::srand(0);
    }
}

pub fn create_entities(world: &World, count: usize) -> Vec<EntityView<'_>> {
    let mut vec = Vec::<EntityView>::with_capacity(count);
    for _ in 0..count {
        let entity = world.entity();
        vec.push(entity);
    }
    vec
}

pub fn create_ids<'a>(
    world: impl WorldProvider<'a>,
    count: usize,
    size: usize,
    low: bool,
    sparse: bool,
    fragment: bool,
) -> Vec<u64> {
    let world = world.world().ptr_mut();

    if count > 0 {
        let mut vec = Vec::<sys::ecs_entity_t>::with_capacity(count);
        unsafe {
            for i in 0..count {
                if low {
                    vec.push(sys::ecs_new_low_id(world));
                } else {
                    vec.push(sys::ecs_new(world));
                }
                if size > 0 {
                    sys::ecs_set_id(
                        world,
                        vec[i],
                        sys::FLECS_IDEcsComponentID_,
                        core::mem::size_of::<EcsComponent>(),
                        &EcsComponent {
                            size: size as i32,
                            alignment: 4,
                        } as *const EcsComponent
                            as *const core::ffi::c_void,
                    );
                }
                if sparse {
                    sys::ecs_add_id(world, vec[i], sys::EcsSparse);
                }
                if !fragment {
                    sys::ecs_add_id(world, vec[i], sys::EcsDontFragment);
                }
            }
        }
        vec
    } else {
        Vec::new()
    }
}

pub fn setup_component<T: ComponentId>(world: &World, sparse: bool, fragment: bool) -> u64 {
    let comp = world.component::<T>();
    if sparse {
        comp.add_trait::<flecs::Sparse>();
    }
    if !fragment {
        comp.add_trait::<flecs::DontFragment>();
    }
    *comp.id()
}

#[macro_export]
macro_rules! get_component_ids {
    ($world:expr, $count:expr, $component:expr, $sparse:expr, $fragment:expr) => {{
        let world = $world.world();
        let mut vec = Vec::<u64>::with_capacity($count as usize);


        if $component {
                    seq!(P in 1..=64 {
            if P < $count {
                let id = setup_component::<C~P>(&world, $sparse, $fragment);
                vec.push(id);
            }
        });
        } else {
            seq!(P in 1..=64 {
            if P < $count {
                let id = setup_component::<T~P>(&world, $sparse, $fragment);
                vec.push(id);
            }
        });
        }
        vec
    }
}
}

#[macro_export]
macro_rules! create_typed_query {
    ($world:expr, $count:expr, $is_component:expr) => {{

    // if $is_component {
        seq!(P in 1..=$count {
            type F = (
                #(
                &'static C~P,
                )*
            );
        });

        $world.query::<F>()
    // }
    // type F = ();


    // let q = $world.query::<F>();

    //  seq!(P in 1..=$count {
    //     q
    //     #(
    //         .with(T~P::id()).self_().set_in()
    //     )*;
    // });
    // q
    }};
}

/// this is a helper function that performs a reset on previously registered component data that is cached for the world.
/// this is useful for benchmarks because in each iteration the world is reset to a clean state, but the static
/// component data is not reset which would cause problems in single world mode.
/// This function is called at the end of each iteration to do this reset.
#[inline(always)]
pub fn reset_world_arrays(world: &World) {
    world.reset_cached_component_ids();
}

macro_rules! vec_of_ids {
        ($world:expr, $component:ty, $start:expr, $end:expr) => {{
            let mut vec = Vec::<Entity>::with_capacity(($end - $start + 1) as usize);
            seq!(P in $start..=$end {
                vec.push(*$world.entity_from::<$component~P>());
            });
            vec
        }};
    }

#[allow(unused)]
macro_rules! add_query_entities_w_rnd_range {
        ($world:expr, $component:ty, $count:expr) => {{
        reset_srand();
        for _ in 0..QUERY_ENTITY_COUNT {
            let entity = $world.entity();
            seq!(P in 1..=$count {
                if flip_coin() {
                    entity.add::<$component~P>();
                }
            });
            entity.add::<T0>();
        }};
    }}

macro_rules! add_component_range {
        ($world:expr, $entity:expr, $component:ty, $start:expr, $end:expr) => {{
            seq!(P in $start..=$end {
                $entity.add($component~P::id());
        });
        }};
    }

macro_rules! add_component_range_cmd {
    ($world:expr, $entity:expr, $component:ty, $start:expr, $end:expr) => {{
        $world.defer_begin();
        add_component_range!($world, $entity, $component, $start, $end);
        $world.defer_end();
    }};
}

macro_rules! enable_disable_component_range {
        ($world:expr, $entity:expr, $component:ty, $start:expr, $end:expr) => {{
            seq!(P in $start..=$end {
                $entity.enable($component~P::id());
                $entity.disable($component~P::id());
        });
        }};
    }

macro_rules! add_relationship_targets {
        ($entity:expr, $amount:expr) => {{
            seq!(P in 1..=$amount {
                $entity.set_pair::<C1, T~P>(C1(0.0));
        });
        }};
    }

macro_rules! add_component_on_add_hook {
        ($world:expr, $amount:expr) => {{
            seq!(P in 1..=$amount {
                $world.component::<C~P>().on_add(|_, _| {});
        });
        }};
    }

macro_rules! add_component_on_remove_hook {
        ($world:expr, $amount:expr) => {{
            seq!(P in 1..=$amount {
                $world.component::<C~P>().on_remove(|_, _| {});
        });
        }};
    }

macro_rules! set_components_inheritable {
    ($world:expr, $component:ty, $start:expr, $end:expr) => {{
        seq!(P in $start..=$end {
            $world.component::<$component~P>().add_trait::<(flecs::OnInstantiate, flecs::Inherit)>();
        });
    }};
}

macro_rules! set_components_toggleable {
    ($world:expr, $component:ty, $start:expr, $end:expr) => {{
        seq!(P in $start..=$end {
            $world.component::<$component~P>().add_trait::<flecs::CanToggle>();
        });
    }};
}

macro_rules! set_components_sparse {
    ($world:expr, $component:ty, $start:expr, $end:expr) => {{
        seq!(P in $start..=$end {
            $world.component::<$component~P>().add_trait::<flecs::Sparse>();
        });
    }};
}

macro_rules! set_components_dont_fragment {
    ($world:expr, $component:ty, $start:expr, $end:expr) => {{
        seq!(P in $start..=$end {
            $world.component::<$component~P>().add_trait::<flecs::DontFragment>();
        });
    }};
}

macro_rules! set_component_range {
        ($world:expr, $entity:expr, $component:ty, $start:expr, $end:expr) => {{
            seq!(P in $start..=$end {
                $entity.set::<$component~P>($component~P(0));
        });
        }};
    }

macro_rules! set_component_range_cmd {
    ($world:expr, $entity:expr, $component:ty, $start:expr, $end:expr) => {{
        $world.defer_begin();
        set_component_range!($world, $entity, $component, $start, $end);
        $world.defer_end();
    }};
}

macro_rules! remove_component_range {
        ($world:expr, $entity:expr, $component:ty, $start:expr, $end:expr) => {{
            seq!(P in $start..=$end {
                $entity.remove($component~P::id());
        });
        }};
    }

macro_rules! remove_component_range_cmd {
    ($world:expr, $entity:expr, $component:ty, $start:expr, $end:expr) => {{
        $world.defer_begin();
        remove_component_range!($world, $entity, $component, $start, $end);
        $world.defer_end();
    }};
}

macro_rules! has_component_range {
        ($world:expr, $entity:expr, $component:ty, $start:expr, $end:expr) => {{
            seq!(P in $start..=$end {
                let ok = $entity.has($component~P::id());
                core::hint::black_box(ok);
        });
        }};
    }

macro_rules! owns_component_range {
        ($world:expr, $entity:expr, $component:ty, $start:expr, $end:expr) => {{
            seq!(P in $start..=$end {
                let ok = $entity.owns($component~P::id());
                core::hint::black_box(ok);
        });
        }};
    }

macro_rules! get_component_range {
        ($world:expr, $entity:expr, $component:ty, $start:expr, $end:expr) => {{
            seq!(P in $start..=$end {
                core::hint::black_box($entity.get::<(#(&$component~P,)*)>());
            });
        }}
    }

macro_rules! get_mut_component_range {
        ($world:expr, $entity:expr, $component:ty, $start:expr, $end:expr) => {{
            seq!(P in $start..=$end {
                core::hint::black_box($entity.get::<(#(&mut $component~P,)*)>());
            });
        }};
    }

#[allow(unused)]
macro_rules! get_mut_component_range_cmd {
    ($world:expr, $entity:expr, $component:ty, $start:expr, $end:expr) => {{
        $world.defer_begin();
        get_mut_component_range!($world, $entity, $component, $start, $end);
        $world.defer_end();
    }};
}

macro_rules! register_component_range {
        ($world:expr, $component:ty, $start:expr, $end:expr) => {{
            seq!(P in $start..=$end {
                $world.component::<$component~P>();
            });
        }};
    }

macro_rules! bench_loop_entities {

        ($group:expr,$name:literal,$entity_count:expr
        ; $(($registration:ident,($r_type:ty, $r_start:expr, $r_end:expr))),*
        ; $(($preparation:ident,($p_type:ty, $p_start:expr, $p_end:expr))),*
        ; $(($benchmark:ident,($b_type:ty, $b_start:expr, $b_end:expr))),*
        )
        => {{
            $group.bench_function($name, |bencher| {
                let world = World::new();
                let mut entities = Vec::with_capacity($entity_count as usize);
                $(
                    $registration!(world, $r_type, $r_start, $r_end);
                )*

                for _ in 0..$entity_count {
                    let entity = world.entity();
                    $(
                        $preparation!(world, entity, $p_type, $p_start, $p_end);
                    )*
                    entities.push(entity);
                }

                bencher.iter_custom(|iters| {
                    let start = Instant::now();
                    for _ in 0..iters {
                        for e in &mut entities {
                            $(
                                $benchmark!(&world, e, $b_type, $b_start, $b_end);
                            )*
                        }
                    }
                    let elapsed = start.elapsed();
                    let mut operations : u32 = 0;
                    let mut count : u32 = 0;
                     $(
                        {

                            count += 1;
                            operations += ($b_end - $b_start + 1);
                        };
                    )*
                    elapsed / (operations * $entity_count * count) //time average per entity operation
                });
                reset_world_arrays(&world);
            });
        }};
    }

macro_rules! bench_create_delete_entity {

        ($group:expr,$name:literal,$entity_count:expr
        ,$ttype:ty, $start:expr, $end:expr
        ,$benchmark:ident)
        => {{
            $group.bench_function(format!("create_delete_entities_{}", $name), |bencher| {
                let world = World::new();

                register_component_range!(world, $ttype, $start, $end);

                bencher.iter_custom(|iters| {
                    let start = Instant::now();
                    for _ in 0..iters {
                        for _ in 0..$entity_count {
                            let entity = world.entity();
                            $benchmark!(&world, entity, $ttype, $start, $end);
                            entity.destruct();
                        }
                    }
                    let elapsed = start.elapsed();
                    elapsed / (2 + ($end - $start + 1) * $entity_count) //time average per entity operation
                });
            });
        }};
    }

macro_rules! bench_create_delete_entity_cmd {

        ($group:expr,$name:literal,$entity_count:expr
        ,$ttype:ty, $start:expr, $end:expr
        ,$benchmark:ident)
        => {{
            $group.bench_function(format!("create_delete_entities_cmd_{}", $name), |bencher| {
                let world = World::new();

                register_component_range!(world, $ttype, $start, $end);

                bencher.iter_custom(|iters| {
                    let start = Instant::now();
                    for _ in 0..iters {
                        world.defer_begin();
                        for _ in 0..$entity_count {
                            let entity = world.entity();
                            $benchmark!(&world, entity, $ttype, $start, $end);
                            //add_component_range!(world, entity, $ttype, $start, $end);
                            entity.destruct();
                        }
                        world.defer_end();
                    }
                    let elapsed = start.elapsed();
                    elapsed / (2 + ($end - $start + 1) * $entity_count) //time average per entity operation
                });
            });
        }};
    }

macro_rules! bench_get_relationship_target {
    ($group:expr,$name:literal,$entity_count:expr,$target_count:expr) => {{
        $group.bench_function($name, |bencher| {
            let world = World::new();
            let mut entities = Vec::<EntityView>::with_capacity($entity_count as usize);

            for _ in 0..$entity_count {
                let entity = world.entity();
                add_relationship_targets!(entity, $target_count);
                entities.push(entity);
            }
            bencher.iter_custom(|iters| {
                let start = Instant::now();
                for _ in 0..iters {
                    for entity in &entities {
                        for i in 0..$target_count {
                            let _ = entity.target::<C1>(i);
                        }
                    }
                }
                let elapsed = start.elapsed();
                elapsed / ($entity_count * $target_count) as u32
            });

            reset_component_range!(T, 1, $target_count);
            reset_component_range!(C, 1, 1);
        });
    }};
}

macro_rules! bench_add_remove_override {
    ($group:expr,$name:literal,$amount:expr) => {{
        $group.bench_function(format!("add_remove_override_{}", $name), |bencher| {
            let world = World::new();
            let entities = create_entities(&world, ENTITY_COUNT as usize);

            let base = world.entity();
            for _ in 0..$amount {
                set_components_inheritable!(&world, C, 1, $amount);
                set_component_range!(&world, base, C, 1, $amount);
            }

            for entity in &entities {
                entity.is_a(base);
            }

            bencher.iter_custom(|iters| {
                let start = Instant::now();
                for _ in 0..iters {
                    for entity in &entities {
                        set_component_range!(&world, entity, C, 1, $amount);
                        remove_component_range!(&world, entity, C, 1, $amount);
                    }
                }
                let elapsed = start.elapsed();
                elapsed / (2 * ENTITY_COUNT * $amount) as u32
            });
        });
    }};
}

macro_rules! bench_add_remove_hooks {
    ($group:expr,$name:literal,$amount:expr) => {{
        $group.bench_function(format!("add_remove_hooks_{}", $name), |bencher| {
            let world = World::new();
            let entities = create_entities(&world, ENTITY_COUNT as usize);

            add_component_on_add_hook!(world, $amount);
            add_component_on_remove_hook!(world, $amount);

            bencher.iter_custom(|iters| {
                let start = Instant::now();
                for _ in 0..iters {
                    for entity in &entities {
                        set_component_range!(&world, entity, C, 1, $amount);
                        remove_component_range!(&world, entity, C, 1, $amount);
                    }
                }
                let elapsed = start.elapsed();
                elapsed / (2 * ENTITY_COUNT * $amount) as u32
            });
        });
    }};
}

pub(crate) use add_component_on_add_hook;
pub(crate) use add_component_on_remove_hook;
pub(crate) use add_component_range;
pub(crate) use add_component_range_cmd;
#[allow(unused)]
pub(crate) use add_query_entities_w_rnd_range;
pub(crate) use add_relationship_targets;
pub(crate) use bench_add_remove_hooks;
pub(crate) use bench_add_remove_override;
pub(crate) use bench_create_delete_entity;
pub(crate) use bench_create_delete_entity_cmd;
pub(crate) use bench_get_relationship_target;
pub(crate) use bench_loop_entities;
pub(crate) use enable_disable_component_range;
pub(crate) use get_component_ids;
pub(crate) use get_component_range;
pub(crate) use get_mut_component_range;
#[allow(unused)]
pub(crate) use get_mut_component_range_cmd;
pub(crate) use has_component_range;
pub(crate) use owns_component_range;
pub(crate) use register_component_range;
pub(crate) use remove_component_range;
pub(crate) use remove_component_range_cmd;
#[allow(unused)]
pub(crate) use set_component_range;
pub(crate) use set_component_range_cmd;
pub(crate) use set_components_dont_fragment;
pub(crate) use set_components_inheritable;
pub(crate) use set_components_sparse;
pub(crate) use set_components_toggleable;
pub(crate) use vec_of_ids;
