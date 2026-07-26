#![allow(clippy::float_cmp)]
use core::ffi::CStr;

use core::mem::offset_of;
use flecs_ecs::prelude::meta::*;
use flecs_ecs::sys;
use flecs_ecs::experimental::prelude::EntityGuardExt;
use flecs_ecs::{component, prelude::*};
use flecs_ecs_sys::ecs_world_t;

fn std_string_support(world: WorldRef) -> Opaque<String> {
    let mut ts = Opaque::<String>::new(world);

    // Let reflection framework know what kind of type this is
    ts.as_type(flecs::meta::String);

    // Forward std::string value to (JSON/...) serializer
    ts.serialize(|s: &Serializer, data: &String| unsafe {
        s.value_id(
            flecs::meta::String,
            &data.as_ptr() as *const *const u8 as *const core::ffi::c_void,
        )
    });

    // Serialize string into std::string
    ts.assign_string(|data: &mut String, value: *const core::ffi::c_char| {
        *data = unsafe { CStr::from_ptr(value).to_string_lossy().into_owned() }
    });

    ts
}

fn std_vector_support<T: Default>(world: WorldRef) -> Opaque<Vec<T>, T> {
    let id = id!(&world, Vec<T>);
    let mut ts = Opaque::<Vec<T>, T>::new_id(world, id);

    // Let reflection framework know what kind of type this is
    ts.as_type(world.vector::<T>());

    // Forward std::vector value to (JSON/...) serializer
    ts.serialize(|s: &Serializer, data: &Vec<T>| {
        let world = unsafe { WorldRef::from_ptr(s.world as *mut ecs_world_t) };
        let id = id!(world, T);
        for el in data.iter() {
            unsafe {
                s.value_id(id, el as *const T as *const core::ffi::c_void);
            }
        }
        0
    });

    // Return vector size
    ts.count(|data: &mut Vec<T>| data.len());

    fn ensure_generic_element<T: Default>(data: &mut Vec<T>, elem: usize) -> &mut T {
        if data.len() <= elem {
            data.resize_with(elem + 1, || T::default());
        }
        &mut data[elem]
    }

    fn resize_generic_vec<T: Default>(data: &mut Vec<T>, elem: usize) {
        data.resize_with(elem + 1, || T::default());
    }

    // Ensure element exists, return
    ts.ensure_element(ensure_generic_element::<T>);

    // Resize contents of vector
    ts.resize(resize_generic_vec::<T>);

    ts
}

#[test]
fn meta_struct() {
    let world = World::new();

    #[derive(Component)]
    struct Test {
        a: i32,
        b: f32,
    }

    let c = component!(&world, Test { a: i32, b: f32 });

    assert_ne!(c.id(), 0);

    unsafe {
        let m = sys::ecs_struct_get_member(world.ptr_mut(), *c.id(), c"a".as_ptr());
        assert!(!m.is_null());
        assert_eq!((*m).type_, flecs::meta::I32);

        let m = sys::ecs_struct_get_member(world.ptr_mut(), *c.id(), c"b".as_ptr());
        assert!(!m.is_null());
        assert_eq!((*m).type_, flecs::meta::F32);
    }
}

#[test]
fn meta_nested_struct() {
    let world = World::new();

    #[derive(Component)]
    struct Test {
        x: i32,
    }

    #[derive(Component)]
    struct Nested {
        a: Test,
    }

    let t = component!(&world, Test { x: i32 });

    let n = component!(&world, Nested { a: Test });

    assert_ne!(n.id(), 0);

    unsafe {
        let m = sys::ecs_struct_get_member(world.ptr_mut(), *n.id(), c"a".as_ptr());
        assert!(!m.is_null());
        assert_eq!((*m).type_, t.id());
    }
}

#[test]
fn meta_struct_w_portable_type() {
    let world = World::new();

    #[derive(Component)]
    struct Test {
        a: usize,
        b: usize,
        c: Entity,
        d: Entity,
    }

    let t = component!(
        &world,
        Test {
            a: usize,
            b: usize,
            c: Entity,
            d: Entity
        }
    );

    assert_ne!(t.id(), 0);

    unsafe {
        let m = sys::ecs_struct_get_member(world.ptr_mut(), *t.id(), c"a".as_ptr());
        assert!(!m.is_null());
        assert_eq!((*m).type_, flecs::meta::UPtr);

        let m = sys::ecs_struct_get_member(world.ptr_mut(), *t.id(), c"b".as_ptr());
        assert!(!m.is_null());
        assert_eq!((*m).type_, flecs::meta::UPtr);
    }

    // b.get::<&flecs::meta::Member>(|mem| {
    //     assert_eq!(mem.type_, flecs::meta::UPtr);
    // });

    // let c = t.lookup("c");
    // assert_ne!(c.id(), 0);
    // assert!(c.has(id::<flecs::meta::Member>()));

    // c.get::<&flecs::meta::Member>(|mem| {
    //     assert_eq!(mem.type_, flecs::meta::Entity);
    // });

    // let d = t.lookup("d");
    // assert_!(d.id(), 0);
    // assert!(d.has(id::<flecs::meta::Member>()));

    // d.get::<&flecs::meta::Member>(|mem| {
    //     assert_eq!(mem.type_, flecs::meta::Entity);
    // });
}

//TODO meta_units -- units addon is not yet implemented in Rust
//TODO Meta_unit_w_quantity -- units addon is not yet implemented in Rust
//TODO Meta_unit_w_prefix -- units addon is not yet implemented in Rust
//TODO Meta_unit_w_over -- units addon is not yet implemented in Rust

#[test]
fn meta_partial_struct() {
    let world = World::new();

    #[derive(Component)]
    struct Position {
        x: f32,
    }

    let c = component!(&world, Position { x: f32 });

    assert_ne!(c.id(), 0);

    {
        let ptr = c.get_ref::<&flecs::Component>().unwrap();
        assert_eq!(ptr.size, 4);
        assert_eq!(ptr.alignment, 4);
    };

    unsafe {
        let m = sys::ecs_struct_get_member(world.ptr_mut(), *c.id(), c"x".as_ptr());
        assert!(!m.is_null());
        assert_eq!((*m).type_, flecs::meta::F32);
        assert_eq!((*m).offset, 0);
    }
}

#[test]
fn meta_partial_struct_custom_offset() {
    let world = World::new();

    #[derive(Component)]
    struct Position {
        x: f32,
        y: f32,
    }

    let c = component!(&world, Position { y: f32 });

    assert_ne!(c.id(), 0);

    {
        let ptr = c.get_ref::<&flecs::Component>().unwrap();
        assert_eq!(ptr.size, 8);
        assert_eq!(ptr.alignment, 4);
    };

    unsafe {
        let m = sys::ecs_struct_get_member(world.ptr_mut(), *c.id(), c"y".as_ptr());
        assert!(!m.is_null());
        assert_eq!((*m).type_, flecs::meta::F32);
        assert_eq!((*m).offset, 4);
    }
}

#[test]
fn meta_bitmask() {
    let world = World::new();

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

    world
        .component::<Toppings>()
        .bit("bacon", Toppings::BACON)
        .bit("lettuce", Toppings::LETTUCE)
        .bit("tomato", Toppings::TOMATO);

    component!(&world, Sandwich { toppings: Toppings });

    // Create entity with Sandwich as usual
    let e = world.entity().set(Sandwich {
        toppings: Toppings {
            value: Toppings::BACON | Toppings::LETTUCE,
        },
    });

    // Convert Sandwidth component to flecs expression string
    {
        let val = e.get_ref::<&Sandwich>().unwrap();
        assert_eq!(world.to_expr(&*val), "{toppings: lettuce|bacon}");
    };
}

#[test]
fn meta_world_ser_deser_flecs_entity() {
    #[derive(Component)]
    struct RustEntity {
        entity: Entity,
    }

    let world = World::new();

    component!(&world, RustEntity { entity: Entity });

    let e1 = world.entity_named("ent1");
    let e2 = world
        .entity_named("ent2")
        .set(RustEntity { entity: e1.id() });

    {
        let ptr = e2.get_ref::<&RustEntity>();
        assert!(ptr.is_some());
        let ptr = ptr.unwrap();
        assert_eq!(world.to_json::<RustEntity>(&*ptr), "{\"entity\":\"ent1\"}");
    };

    let json = world.to_json_world(None);

    let world = World::new();

    component!(&world, RustEntity { entity: Entity });

    world.from_json_world(json.as_str(), None);

    assert!(e1.is_alive());
    assert!(e2.is_alive());

    {
        let ptr = e2.get_ref::<&RustEntity>();
        assert!(ptr.is_some());
        let ptr = ptr.unwrap();
        assert_eq!(world.to_json::<RustEntity>(&*ptr), "{\"entity\":\"ent1\"}");
    };
}

#[test]
fn meta_new_world_ser_deser_flecs_entity() {
    #[derive(Component)]
    struct RustEntity {
        entity: Entity,
    }

    let world = World::new();

    component!(&world, RustEntity { entity: Entity });

    let e1 = world.entity_named("ent1");
    let e2 = world
        .entity_named("ent2")
        .set(RustEntity { entity: e1.id() });

    {
        let ptr = e2.get_ref::<&RustEntity>();
        assert!(ptr.is_some());
        let ptr = ptr.unwrap();
        assert_eq!(world.to_json::<RustEntity>(&*ptr), "{\"entity\":\"ent1\"}");
    };

    let json = world.to_json_world(None);

    let world = World::new();

    component!(&world, RustEntity { entity: Entity });

    world.from_json_world(json.as_str(), None);

    let e1 = world.lookup("ent1");
    let e2 = world.lookup("ent2");

    assert_ne!(e1.id(), 0);
    assert_ne!(e2.id(), 0);

    assert!(e1.is_alive());
    assert!(e2.is_alive());

    {
        let ptr = e2.get_ref::<&RustEntity>();
        assert!(ptr.is_some());
        let ptr = ptr.unwrap();
        assert_eq!(world.to_json::<RustEntity>(&*ptr), "{\"entity\":\"ent1\"}");
    };
}

#[test]
fn meta_new_world_ser_deser_empty_flecs_entity() {
    #[derive(Component)]
    struct RustEntity {
        entity: Entity,
    }

    let world = World::new();

    component!(&world, RustEntity { entity: Entity });

    let e1 = Entity::null();
    let e2 = world.entity_named("ent2").set(RustEntity { entity: e1 });

    {
        let ptr = e2.get_ref::<&RustEntity>();
        assert!(ptr.is_some());
        let ptr = ptr.unwrap();
        assert_eq!(world.to_json::<RustEntity>(&*ptr), "{\"entity\":\"#0\"}");
    };

    let json = world.to_json_world(None);

    let world = World::new();

    component!(&world, RustEntity { entity: Entity });

    world.from_json_world(json.as_str(), None);

    let e2 = world.lookup("ent2");

    assert_ne!(e2.id(), 0);

    assert!(e2.is_alive());

    {
        let ptr = e2.get_ref::<&RustEntity>();
        assert!(ptr.is_some());
        let ptr = ptr.unwrap();
        assert_eq!(world.to_json::<RustEntity>(&*ptr), "{\"entity\":\"#0\"}");
    };
}

#[repr(C)]
#[derive(Component)]
#[allow(clippy::enum_clike_unportable_variant)]
enum EnumWithBits {
    BitA = 0,
    BitB = 1 << 0,
    BitAll = 0xffffffff,
}

#[derive(Component)]
struct EnumWithBitsStruct {
    bits: EnumWithBits,
}

impl Default for EnumWithBitsStruct {
    fn default() -> Self {
        EnumWithBitsStruct {
            bits: EnumWithBits::BitAll,
        }
    }
}

#[test]
fn meta_struct_member_ptr() {
    let world = World::new();

    #[derive(Component)]
    struct Test {
        x: i32,
    }

    #[derive(Component)]
    struct Test2 {
        y: f64,
    }

    #[derive(Component)]
    struct Nested {
        a: Test, //what offset should this be? Rust returns 16, but CPP gives 0 lol
        pad: i32,
        b: [Test2; 2],
    }

    let t = component!(&world, Test { x: i32 });

    let t2 = component!(&world, Test2 { y: f64 });

    let n = component!(
        &world,
        Nested {
            a: Test,
            b: [Test2; 2]
        }
    );

    //validate Test #1
    assert_ne!(t.id(), 0);

    unsafe {
        let m = sys::ecs_struct_get_member(world.ptr_mut(), *t.id(), c"x".as_ptr());
        assert!(!m.is_null());
        assert_eq!((*m).type_, flecs::meta::I32);
        assert_eq!((*m).offset, offset_of!(Test, x) as i32);
    }

    //validate Test2 #2
    assert_ne!(t2.id(), 0);

    unsafe {
        let m = sys::ecs_struct_get_member(world.ptr_mut(), *t2.id(), c"y".as_ptr());
        assert!(!m.is_null());
        assert_eq!((*m).type_, flecs::meta::F64);
        assert_eq!((*m).offset, offset_of!(Test2, y) as i32);
    }

    // Validate Nested
    assert_ne!(n.id(), 0);

    unsafe {
        let m = sys::ecs_struct_get_member(world.ptr_mut(), *n.id(), c"a".as_ptr());
        assert!(!m.is_null());
        assert_eq!((*m).type_, t.id());
        assert_eq!((*m).offset, offset_of!(Nested, a) as i32);

        let m = sys::ecs_struct_get_member(world.ptr_mut(), *n.id(), c"b".as_ptr());
        assert!(!m.is_null());
        assert_eq!((*m).type_, t2.id());
        assert_eq!((*m).offset, offset_of!(Nested, b) as i32);
        assert_eq!((*m).count, 2);
    }
}

#[test]
fn meta_struct_field_order() {
    let world = World::new();

    #[derive(Component)]
    struct Test {
        a: u32,
        b: i32,
    }

    let t = component!(&world, Test { a: u32, b: i32 });

    assert_ne!(t.id(), 0);
    unsafe {
        let m = sys::ecs_struct_get_member(world.ptr_mut(), *t.id(), c"a".as_ptr());
        assert!(!m.is_null());
        assert_eq!((*m).type_, flecs::meta::U32);
        assert_eq!((*m).offset, offset_of!(Test, a) as i32);

        let m = sys::ecs_struct_get_member(world.ptr_mut(), *t.id(), c"b".as_ptr());
        assert!(!m.is_null());
        assert_eq!((*m).type_, flecs::meta::I32);
        assert_eq!((*m).offset, offset_of!(Test, b) as i32);
    }

    let e = world.entity().set(Test { a: 10, b: 20 });

    {
        let ptr = e.get_ref::<&Test>().unwrap();
        assert_eq!(ptr.a, 10);
        assert_eq!(ptr.b, 20);
        let json = world.to_expr(&*ptr);
        assert_eq!(json, "{a: 10, b: 20}"); //if this fails, field re-ordering is not working
    };
}

#[test]
fn meta_ser_deser_option() {
    let world = World::new();

    #[derive(Component, Default, Debug, PartialEq)]
    struct OptComponent(Option<u32>);

    component!(&world, #[auto] Option<u32>);
    component!(&world, OptComponent(Option<u32>));

    {
        let mut v = OptComponent::default();
        let json = "{\"0\":{\"None\":false}}".to_string();
        world.from_json::<OptComponent>(&mut v, &json, None);
        assert_eq!(v, OptComponent(None));
        let json = world.to_json::<OptComponent>(&v);
        assert_eq!(json, "{\"0\":{\"None\":false}}");
    }

    {
        let mut v = OptComponent::default();
        let json = "{\"0\":{\"Some\":42}}".to_string();
        world.from_json::<OptComponent>(&mut v, &json, None);
        assert_eq!(v, OptComponent(Some(42)));
        let json = world.to_json::<OptComponent>(&v);
        assert_eq!(json, "{\"0\":{\"Some\":42}}");
    }
}

#[test]
fn meta_ser_deser_option_none_multi_threaded() {
    let handles: Vec<_> = (0..4)
        .map(|_| {
            std::thread::spawn(|| {
                let world = World::new();

                #[derive(Component, Default, Debug, PartialEq)]
                struct OptComponent(Option<u32>);

                component!(&world, #[auto] Option<u32>);
                component!(&world, OptComponent(Option<u32>));

                for _ in 0..100 {
                    let mut v = OptComponent(Some(7));
                    let json = "{\"0\":{\"None\":false}}".to_string();
                    world.from_json::<OptComponent>(&mut v, &json, None);
                    assert_eq!(v, OptComponent(None));
                    let json = world.to_json::<OptComponent>(&v);
                    assert_eq!(json, "{\"0\":{\"None\":false}}");
                }
            })
        })
        .collect();

    for handle in handles {
        handle.join().unwrap();
    }
}

#[test]
fn meta_macro_numeric_field_indices_for_large_tuple_struct() {
    let world = World::new();

    #[derive(Component, Default)]
    struct Big(u8, u8, u8, u8, u8, u8, u8, u8, u8, u8, u8, u8, u8);

    component!(
        &world,
        Big {
            0: u8,
            1: u8,
            2: u8,
            3: u8,
            4: u8,
            5: u8,
            6: u8,
            7: u8,
            8: u8,
            9: u8,
            10: u8,
            11: u8,
            12: u8
        }
    );

    let v = Big(1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13);
    let json = world.to_json::<Big>(&v);
    assert!(json.contains("\"12\":13"), "json was: {json}");
}
