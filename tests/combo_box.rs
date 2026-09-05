use std::time::Duration;

use bevy::{
    app::{App, Startup},
    camera::{NormalizedRenderTarget, visibility::Visibility},
    ecs::{entity::Entity, hierarchy::Children, resource::Resource, system::Commands},
    math::Vec2,
    picking::{
        backend::HitData,
        events::{Click, Pointer, Press},
        pointer::{Location, PointerButton, PointerId},
    },
    ui_widgets::{Button, ButtonPlugin, ListBox},
};

use bevy_widgetry::combo_box::{ComboBoxPlugin, spawn_headless_combo_box};

#[derive(Resource)]
struct TestComboBoxes {
    a: Entity,
    b: Entity,
}

fn spawn_two_combo_boxes(mut commands: Commands) {
    let a = spawn_headless_combo_box(&mut commands, 3, Some(0));
    let b = spawn_headless_combo_box(&mut commands, 3, Some(1));

    commands.insert_resource(TestComboBoxes { a, b });
}

fn primary_press(entity: Entity) -> Pointer<Press> {
    Pointer::new(
        PointerId::Mouse,
        Location {
            target: NormalizedRenderTarget::None {
                width: 1,
                height: 1,
            },
            position: Vec2::ZERO,
        },
        Press {
            button: PointerButton::Primary,
            hit: HitData::new(Entity::PLACEHOLDER, 0.0, None, None),
            count: 1,
        },
        entity,
    )
}

fn primary_click(entity: Entity) -> Pointer<Click> {
    Pointer::new(
        PointerId::Mouse,
        Location {
            target: NormalizedRenderTarget::None {
                width: 1,
                height: 1,
            },
            position: Vec2::ZERO,
        },
        Click {
            button: PointerButton::Primary,
            hit: HitData::new(Entity::PLACEHOLDER, 0.0, None, None),
            duration: Duration::ZERO,
            count: 1,
        },
        entity,
    )
}

fn find_child_with<T: bevy::ecs::component::Component>(
    world: &bevy::ecs::world::World,
    parent: Entity,
) -> Entity {
    let children = world.get::<Children>(parent).unwrap();

    children
        .iter()
        .copied()
        .find(|&child| world.get::<T>(child).is_some())
        .unwrap()
}

#[test]
fn clicking_second_combo_box_should_close_first_and_open_second() {
    let mut app = App::new();

    app.add_plugins((ButtonPlugin, ComboBoxPlugin))
        .add_systems(Startup, spawn_two_combo_boxes);

    app.update();

    let world = app.world_mut();

    let (combo_a, combo_b) = {
        let combo_boxes = world.resource::<TestComboBoxes>();
        (combo_boxes.a, combo_boxes.b)
    };

    let field_b = find_child_with::<Button>(world, combo_b);
    let popup_a = find_child_with::<ListBox>(world, combo_a);
    let popup_b = find_child_with::<ListBox>(world, combo_b);

    // 先模拟 A 已经处于打开状态。
    *world.get_mut::<Visibility>(popup_a).unwrap() = Visibility::Visible;

    assert_eq!(
        *world.get::<Visibility>(popup_a).unwrap(),
        Visibility::Visible
    );

    assert_eq!(
        *world.get::<Visibility>(popup_b).unwrap(),
        Visibility::Hidden
    );

    // 真正走 Button 的 pointer -> Activate 路径。
    world.trigger(primary_press(field_b));
    world.flush();

    world.trigger(primary_click(field_b));
    world.flush();

    // 点击 B：
    // - 对 A 来说是 outside click，所以 A 关闭；
    // - B 的 Button 产生 Activate，所以 B 打开。
    assert_eq!(
        *world.get::<Visibility>(popup_a).unwrap(),
        Visibility::Hidden
    );

    assert_eq!(
        *world.get::<Visibility>(popup_b).unwrap(),
        Visibility::Visible
    );
}
