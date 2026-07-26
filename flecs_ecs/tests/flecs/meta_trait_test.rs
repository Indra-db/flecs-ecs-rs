use flecs_ecs::experimental::prelude::EntityGuardExt;
use flecs_ecs::prelude::meta::*;
use flecs_ecs::prelude::*;

#[derive(Component)]
#[flecs(meta)]
struct Position {
    x: f32,
    y: f32,
}

#[derive(Component)]
#[flecs(meta)]
struct PositionSkipY {
    x: f32,
    #[flecs_skip]
    y: f32,
}

#[derive(Component)]
#[flecs(meta)]
struct PositionSkipX {
    #[flecs_skip]
    x: f32,
    y: f32,
}

#[test]
fn test_pos() {
    let world = World::new();

    // Create a new entity
    let e = world.entity().set(Position { x: 2.0, y: 4.0 });

    // Convert position component to flecs expression string
    {
        let p = e.get::<&Position>().unwrap();
        let expr: String = world.to_expr(&*p);
        assert_eq!(expr, "{x: 2, y: 4}");
    };
}

#[test]
fn test_pos_skip_y() {
    let world = World::new();

    // Create a new entity
    let e = world.entity().set(PositionSkipY { x: 2.0, y: 4.0 });

    // Convert position component to flecs expression string
    {
        let p = e.get::<&PositionSkipY>().unwrap();
        let expr: String = world.to_expr(&*p);
        assert_eq!(expr, "{x: 2}");
    };
}

#[test]
fn test_pos_skip_x() {
    let world = World::new();

    // Create a new entity
    let e = world.entity().set(PositionSkipX { x: 2.0, y: 4.0 });

    // Convert position component to flecs expression string
    {
        let p = e.get::<&PositionSkipX>().unwrap();
        let expr: String = world.to_expr(&*p);
        assert_eq!(expr, "{y: 4}");
    };
}

#[derive(Debug, Component)]
#[repr(C)]
#[flecs(meta)]
pub enum Color {
    Red,
    Green,
    Blue,
}

#[derive(Debug, Component)]
#[flecs(meta)]
pub struct TypeWithEnum {
    pub color: Color,
}

#[test]
fn test_enum() {
    let world = World::new();

    assert!(
        world
            .component::<TypeWithEnum>()
            .has(id::<flecs::meta::Type>())
    );

    // Create a new entity
    let e = world
        .entity()
        .add_enum(Color::Green)
        .set(TypeWithEnum { color: Color::Blue });

    // Convert TypeWithEnum component to flecs expression string. The enum read
    // goes through the guard surface now that pin revalidation resolves an
    // `add_enum` relationship target correctly.
    {
        let (color, type_enum) = e.get::<(&Color, &TypeWithEnum)>().unwrap();
        let expr: String = world.to_expr(&*color);
        assert_eq!(expr, "Green");
        let expr = world.to_expr(&*type_enum);
        assert_eq!(expr, "{color: Blue}");
    }
}

#[derive(Debug, Component)]
#[flecs(meta)]
pub struct TypeWithString {
    pub name: String,
}

#[test]
fn test_type_w_string() {
    let world = World::new();

    assert!(
        world
            .component::<TypeWithString>()
            .has(id::<flecs::meta::Type>())
    );

    // Create a new entity
    let e = world.entity().set(TypeWithString {
        name: "hello".to_string(),
    });

    // Convert TypeWithEnum component to flecs expression string
    {
        let str = e.get::<&TypeWithString>().unwrap();
        let json: String = world.to_json::<TypeWithString>(&str);
        assert_eq!(json, "{\"name\":\"hello\"}");
    };
}

#[derive(Debug, Component)]
#[flecs(meta)]
pub struct TypeWithVecString {
    pub names: Vec<String>,
}

#[test]
fn test_type_w_vec_string() {
    let world = World::new();

    assert!(
        world
            .component::<TypeWithVecString>()
            .has(id::<flecs::meta::Type>())
    );

    // Create a new entity
    let e = world.entity().set(TypeWithVecString {
        names: vec!["hello".to_string(), "world".to_string()],
    });

    // Convert TypeWithVecString component to flecs json string
    {
        let str = e.get::<&TypeWithVecString>().unwrap();
        let json: String = world.to_json::<TypeWithVecString>(&str);
        assert_eq!(json, "{\"names\":[\"hello\", \"world\"]}");
    };
}
