use core::mem::offset_of;
use flecs_ecs::experimental::prelude::EntityGuardExt;
use flecs_ecs::prelude::*;

#[test]
fn meta_struct_field_order() {
    let world = World::new();

    #[derive(Component, Default)]
    struct Test {
        a: u32,
        b: i64,
        c: i16,
        d: i8,
        e: i64,
    }

    world
        .component::<Test>()
        .member(u32::id(), ("a", Count(0), offset_of!(Test, a)))
        .member(i64::id(), ("b", Count(0), offset_of!(Test, b)))
        .member(i16::id(), ("c", Count(0), offset_of!(Test, c)))
        .member(i8::id(), ("d", Count(0), offset_of!(Test, d)))
        .member(i64::id(), ("e", Count(0), offset_of!(Test, e)));

    let e = world.entity().set(Test {
        a: 10,
        b: 20,
        c: 30,
        d: 40,
        e: 50,
    });

    {
        let ptr = e.get::<&Test>().unwrap();
        assert_eq!(ptr.a, 10);
        assert_eq!(ptr.b, 20);
        let json = world.to_expr(&*ptr);
        assert_eq!(json, "{a: 10, b: 20, c: 30, d: 40, e: 50}"); //if this fails, field re-ordering is not working
    };
}

#[test]
fn test_meta_debug_stringify() {
    #[derive(Debug, flecs_ecs_derive::Component)]
    struct Position {
        x: f32,
        y: f32,
    }

    #[derive(flecs_ecs_derive::Component)]
    struct Velocity {
        x: f32,
        y: f32,
    }

    impl core::fmt::Display for Velocity {
        fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
            write!(f, "Velocity {{ x: {}, y: {} }}", self.x, self.y)
        }
    }

    let world = World::new();

    world
        .component::<Position>()
        .opaque_func(meta_ser_stringify_type_debug::<Position>);

    world
        .component::<Velocity>()
        .opaque_func(meta_ser_stringify_type_display::<Velocity>);

    let ent = world
        .entity_named("bob")
        .set(Position { x: 1.0, y: 2.0 })
        .set(Velocity { x: 3.0, y: 4.0 });

    let json = ent.to_json(None);

    assert_eq!(
        json,
        r#"{"name":"bob", "components":{"flecs.meta_test_rust.test_meta_debug_stringify.Position":"Position { x: 1.0, y: 2.0 }", "flecs.meta_test_rust.test_meta_debug_stringify.Velocity":"Velocity { x: 3, y: 4 }"}}"#
    );
}
