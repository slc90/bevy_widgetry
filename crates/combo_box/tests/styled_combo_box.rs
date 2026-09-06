use bevy::{
    app::{App, Startup},
    camera::visibility::Visibility,
    color::Color,
    ecs::{
        hierarchy::Children,
        query::{Has, With, Without},
        system::Commands,
    },
    picking::hover::Hovered,
    ui::{
        BackgroundColor, BorderColor, FlexDirection, InteractionDisabled, Node, PositionType,
        Pressed, Selected, Val, widget::Text,
    },
    ui_widgets::{Button, ListBox, ListItem},
};
use bevy_widgetry_combo_box::{
    ComboBox, SetComboBoxSelected, StyledComboBoxPlugin, spawn_styled_combo_box,
};
use rstest::{fixture, rstest};

const EXPECTED_OPTION_DEFAULT_BG: Color = Color::srgb(0.10, 0.10, 0.12);
const EXPECTED_OPTION_SELECTED_BG: Color = Color::srgb(0.10, 0.32, 0.42);
const EXPECTED_FIELD_HOVERED_BG: Color = Color::srgb(0.16, 0.22, 0.26);
const EXPECTED_FIELD_PRESSED_BG: Color = Color::srgb(0.08, 0.35, 0.45);
const EXPECTED_FIELD_OPEN_BG: Color = Color::srgb(0.12, 0.28, 0.34);
const EXPECTED_FIELD_DEFAULT_BG: Color = Color::srgb(0.12, 0.12, 0.14);
const EXPECTED_POPUP_BORDER: Color = Color::srgb(1.0, 0.35, 0.75);
const EXPECTED_POPUP_BG: Color = Color::srgb(0.10, 0.10, 0.12);
const EXPECTED_OPTION_HOVERED_BG: Color = Color::srgb(0.18, 0.45, 0.65);
const EXPECTED_OPTION_DISABLED_BG: Color = Color::srgb(0.08, 0.08, 0.09);
const EXPECTED_FIELD_DISABLED_BG: Color = Color::srgb(0.08, 0.08, 0.09);

#[fixture]
fn app() -> App {
    let mut app = App::new();

    app.add_plugins(StyledComboBoxPlugin)
        .add_systems(Startup, spawn_test_combo_box);

    app
}

fn spawn_test_combo_box(mut commands: Commands) {
    spawn_styled_combo_box(
        &mut commands,
        vec![
            "Apple".to_string(),
            "Banana".to_string(),
            "Orange".to_string(),
        ],
    );
}

#[rstest]
fn spawned_combo_box_has_visual_structure(mut app: App) {
    app.update();

    let world = app.world_mut();

    // Root
    let combo_box = {
        let mut query =
            world.query_filtered::<(bevy::ecs::entity::Entity, Has<Node>), With<ComboBox>>();

        let (entity, has_node) = query.single(world).unwrap();

        assert!(has_node);

        entity
    };

    let root_children = world.get::<Children>(combo_box).unwrap();

    // Field = Button
    let field = root_children
        .iter()
        .copied()
        .find(|&child| world.get::<Button>(child).is_some())
        .expect("ComboBox should contain a field Button");

    assert!(world.get::<Node>(field).is_some());
    assert!(world.get::<BackgroundColor>(field).is_some());
    assert!(world.get::<BorderColor>(field).is_some());

    // Popup = ListBox
    let popup = root_children
        .iter()
        .copied()
        .find(|&child| world.get::<ListBox>(child).is_some())
        .expect("ComboBox should contain a popup ListBox");

    assert!(world.get::<Node>(popup).is_some());
    assert!(world.get::<BackgroundColor>(popup).is_some());
    assert!(world.get::<BorderColor>(popup).is_some());

    // Options = ListItem
    let popup_children = world.get::<Children>(popup).unwrap();

    let options = popup_children
        .iter()
        .copied()
        .filter(|&child| world.get::<ListItem>(child).is_some())
        .collect::<Vec<_>>();

    assert_eq!(options.len(), 3);

    for option in options {
        assert!(world.get::<Node>(option).is_some());
        assert!(world.get::<BackgroundColor>(option).is_some());
    }
}

#[rstest]
fn spawned_combo_box_field_shows_first_option(mut app: App) {
    app.update();

    let world = app.world_mut();

    let combo_box = {
        let mut query = world.query_filtered::<bevy::ecs::entity::Entity, With<ComboBox>>();

        query.single(world).unwrap()
    };

    let root_children = world.get::<Children>(combo_box).unwrap();

    let field = root_children
        .iter()
        .copied()
        .find(|&child| world.get::<Button>(child).is_some())
        .expect("ComboBox should contain a field Button");

    let field_children = world.get::<Children>(field).unwrap();

    let texts = field_children
        .iter()
        .filter_map(|&child| world.get::<Text>(child))
        .map(|text| text.0.as_str())
        .collect::<Vec<_>>();

    assert!(texts.contains(&"Apple"));
}

#[rstest]
fn programmatic_selection_updates_field_text(mut app: App) {
    app.update();

    let combo_box = {
        let world = app.world_mut();

        let mut query = world.query_filtered::<bevy::ecs::entity::Entity, With<ComboBox>>();

        query.single(world).unwrap()
    };

    // 初始 selection = 0，所以 Field 显示 Apple
    {
        let world = app.world();

        let root_children = world.get::<Children>(combo_box).unwrap();

        let field = root_children
            .iter()
            .copied()
            .find(|&child| world.get::<Button>(child).is_some())
            .expect("ComboBox should contain a field Button");

        let field_children = world.get::<Children>(field).unwrap();

        let texts = field_children
            .iter()
            .filter_map(|&child| world.get::<Text>(child))
            .map(|text| text.0.as_str())
            .collect::<Vec<_>>();

        assert!(texts.contains(&"Apple"));
    }

    // programmatic selection: Apple → Orange
    app.world_mut().trigger(SetComboBoxSelected {
        entity: combo_box,
        selected: 2,
    });

    app.update();

    // Field Text 应该跟随 Selected 状态变化
    {
        let world = app.world();

        let root_children = world.get::<Children>(combo_box).unwrap();

        let field = root_children
            .iter()
            .copied()
            .find(|&child| world.get::<Button>(child).is_some())
            .expect("ComboBox should contain a field Button");

        let field_children = world.get::<Children>(field).unwrap();

        let texts = field_children
            .iter()
            .filter_map(|&child| world.get::<Text>(child))
            .map(|text| text.0.as_str())
            .collect::<Vec<_>>();

        assert!(texts.contains(&"Orange"));
        assert!(!texts.contains(&"Apple"));
    }
}

#[rstest]
fn programmatic_selection_updates_option_styles(mut app: App) {
    app.update();

    let combo_box = {
        let world = app.world_mut();

        let mut query = world.query_filtered::<bevy::ecs::entity::Entity, With<ComboBox>>();

        query.single(world).unwrap()
    };

    // 找 Popup
    let popup = {
        let world = app.world();

        let root_children = world.get::<Children>(combo_box).unwrap();

        root_children
            .iter()
            .copied()
            .find(|&child| world.get::<ListBox>(child).is_some())
            .expect("ComboBox should contain a popup ListBox")
    };

    // 通过每个 ListItem 下面的 Text 找 Apple / Orange
    let (apple, orange) = {
        let world = app.world();

        let popup_children = world.get::<Children>(popup).unwrap();

        let mut apple = None;
        let mut orange = None;

        for &option in popup_children.iter() {
            if world.get::<ListItem>(option).is_none() {
                continue;
            }

            let children = world.get::<Children>(option).unwrap();

            let text = children
                .iter()
                .filter_map(|&child| world.get::<Text>(child))
                .next()
                .unwrap();

            match text.0.as_str() {
                "Apple" => apple = Some(option),
                "Orange" => orange = Some(option),
                _ => {}
            }
        }

        (apple.unwrap(), orange.unwrap())
    };

    // 初始：Apple selected，Orange default
    assert_eq!(
        app.world().get::<BackgroundColor>(apple).unwrap().0,
        EXPECTED_OPTION_SELECTED_BG,
    );

    assert_eq!(
        app.world().get::<BackgroundColor>(orange).unwrap().0,
        EXPECTED_OPTION_DEFAULT_BG,
    );

    // Apple → Orange
    app.world_mut().trigger(SetComboBoxSelected {
        entity: combo_box,
        selected: 2,
    });

    app.update();

    // Apple 恢复 default
    assert_eq!(
        app.world().get::<BackgroundColor>(apple).unwrap().0,
        EXPECTED_OPTION_DEFAULT_BG,
    );

    // Orange 变 selected
    assert_eq!(
        app.world().get::<BackgroundColor>(orange).unwrap().0,
        EXPECTED_OPTION_SELECTED_BG,
    );
}

#[rstest]
fn field_style_priority_is_open_then_pressed_then_hovered(mut app: App) {
    app.update();

    let combo_box = {
        let world = app.world_mut();

        let mut query = world.query_filtered::<bevy::ecs::entity::Entity, With<ComboBox>>();

        query.single(world).unwrap()
    };

    let (field, popup) = {
        let world = app.world();

        let root_children = world.get::<Children>(combo_box).unwrap();

        let field = root_children
            .iter()
            .copied()
            .find(|&child| world.get::<Button>(child).is_some())
            .expect("ComboBox should contain a field Button");

        let popup = root_children
            .iter()
            .copied()
            .find(|&child| world.get::<ListBox>(child).is_some())
            .expect("ComboBox should contain a popup ListBox");

        (field, popup)
    };

    // Hovered
    app.world_mut().entity_mut(field).insert(Hovered(true));

    app.update();

    assert_eq!(
        app.world().get::<BackgroundColor>(field).unwrap().0,
        EXPECTED_FIELD_HOVERED_BG,
    );

    // Hovered + Pressed
    // Pressed 应该覆盖 Hovered
    app.world_mut().entity_mut(field).insert(Pressed);

    app.update();

    assert_eq!(
        app.world().get::<BackgroundColor>(field).unwrap().0,
        EXPECTED_FIELD_PRESSED_BG,
    );

    // Hovered + Pressed + Open
    // Open 应该再覆盖 Pressed
    *app.world_mut().get_mut::<Visibility>(popup).unwrap() = Visibility::Visible;

    app.update();

    assert_eq!(
        app.world().get::<BackgroundColor>(field).unwrap().0,
        EXPECTED_FIELD_OPEN_BG,
    );
}

#[rstest]
fn field_style_falls_back_when_higher_priority_states_are_removed(mut app: App) {
    app.update();

    let combo_box = {
        let world = app.world_mut();

        let mut query = world.query_filtered::<bevy::ecs::entity::Entity, With<ComboBox>>();

        query.single(world).unwrap()
    };

    let (field, popup) = {
        let world = app.world();

        let root_children = world.get::<Children>(combo_box).unwrap();

        let field = root_children
            .iter()
            .copied()
            .find(|&child| world.get::<Button>(child).is_some())
            .expect("ComboBox should contain a field Button");

        let popup = root_children
            .iter()
            .copied()
            .find(|&child| world.get::<ListBox>(child).is_some())
            .expect("ComboBox should contain a popup ListBox");

        (field, popup)
    };

    // Hovered + Pressed + Open
    app.world_mut()
        .entity_mut(field)
        .insert(Hovered(true))
        .insert(Pressed);

    *app.world_mut().get_mut::<Visibility>(popup).unwrap() = Visibility::Visible;

    app.update();

    assert_eq!(
        app.world().get::<BackgroundColor>(field).unwrap().0,
        EXPECTED_FIELD_OPEN_BG,
    );

    // Open 消失 → 回退到 Pressed
    *app.world_mut().get_mut::<Visibility>(popup).unwrap() = Visibility::Hidden;

    app.update();

    assert_eq!(
        app.world().get::<BackgroundColor>(field).unwrap().0,
        EXPECTED_FIELD_PRESSED_BG,
    );

    // Pressed 消失 → 回退到 Hovered
    app.world_mut().entity_mut(field).remove::<Pressed>();

    app.update();

    assert_eq!(
        app.world().get::<BackgroundColor>(field).unwrap().0,
        EXPECTED_FIELD_HOVERED_BG,
    );

    // Hovered 消失 → 回退到 Default
    app.world_mut().entity_mut(field).insert(Hovered(false));

    app.update();

    assert_eq!(
        app.world().get::<BackgroundColor>(field).unwrap().0,
        EXPECTED_FIELD_DEFAULT_BG,
    );
}

#[rstest]
fn popup_has_expected_style(mut app: App) {
    app.update();

    let combo_box = {
        let world = app.world_mut();

        let mut query = world.query_filtered::<bevy::ecs::entity::Entity, With<ComboBox>>();

        query.single(world).unwrap()
    };

    let popup = {
        let world = app.world();

        let root_children = world.get::<Children>(combo_box).unwrap();

        root_children
            .iter()
            .copied()
            .find(|&child| world.get::<ListBox>(child).is_some())
            .expect("ComboBox should contain a popup ListBox")
    };

    let world = app.world();

    assert_eq!(
        world.get::<BackgroundColor>(popup).unwrap().0,
        EXPECTED_POPUP_BG,
    );

    let border = world.get::<BorderColor>(popup).unwrap();

    assert_eq!(border.top, EXPECTED_POPUP_BORDER);
    assert_eq!(border.right, EXPECTED_POPUP_BORDER);
    assert_eq!(border.bottom, EXPECTED_POPUP_BORDER);
    assert_eq!(border.left, EXPECTED_POPUP_BORDER);

    let node = world.get::<Node>(popup).unwrap();

    assert_eq!(node.position_type, PositionType::Absolute);
    assert_eq!(node.left, Val::Px(0.0));
    assert_eq!(node.top, Val::Percent(100.0));
    assert_eq!(node.width, Val::Percent(100.0));
    assert_eq!(node.flex_direction, FlexDirection::Column);
}

#[rstest]
fn option_hovered_overrides_selected(mut app: App) {
    app.update();

    let selected_option = {
        let world = app.world_mut();

        let mut query =
            world.query_filtered::<bevy::ecs::entity::Entity, (With<ListItem>, With<Selected>)>();

        query.single(world).unwrap()
    };

    // 初始是 Selected 样式
    assert_eq!(
        app.world()
            .get::<BackgroundColor>(selected_option)
            .unwrap()
            .0,
        EXPECTED_OPTION_SELECTED_BG,
    );

    // Selected + Hovered
    // Hovered 优先级更高
    app.world_mut()
        .entity_mut(selected_option)
        .insert(Hovered(true));

    app.update();

    assert_eq!(
        app.world()
            .get::<BackgroundColor>(selected_option)
            .unwrap()
            .0,
        EXPECTED_OPTION_HOVERED_BG,
    );

    // Hovered 消失后应该回退到 Selected
    app.world_mut()
        .entity_mut(selected_option)
        .insert(Hovered(false));

    app.update();

    assert_eq!(
        app.world()
            .get::<BackgroundColor>(selected_option)
            .unwrap()
            .0,
        EXPECTED_OPTION_SELECTED_BG,
    );
}

#[rstest]
fn unselected_option_hovered_falls_back_to_default(mut app: App) {
    app.update();

    let option = {
        let world = app.world_mut();

        let mut query = world
            .query_filtered::<bevy::ecs::entity::Entity, (With<ListItem>, Without<Selected>)>();

        query.iter(world).next().unwrap()
    };

    // 初始是 Default
    assert_eq!(
        app.world().get::<BackgroundColor>(option).unwrap().0,
        EXPECTED_OPTION_DEFAULT_BG,
    );

    // Hovered
    app.world_mut().entity_mut(option).insert(Hovered(true));

    app.update();

    assert_eq!(
        app.world().get::<BackgroundColor>(option).unwrap().0,
        EXPECTED_OPTION_HOVERED_BG,
    );

    // Hovered 消失 → Default
    app.world_mut().entity_mut(option).insert(Hovered(false));

    app.update();

    assert_eq!(
        app.world().get::<BackgroundColor>(option).unwrap().0,
        EXPECTED_OPTION_DEFAULT_BG,
    );
}

#[rstest]
fn disabled_option_overrides_hovered_and_selected(mut app: App) {
    app.update();

    let combo_box = {
        let world = app.world_mut();

        let mut query = world.query_filtered::<bevy::ecs::entity::Entity, With<ComboBox>>();

        query.single(world).unwrap()
    };

    let selected_option = {
        let world = app.world_mut();

        let mut query =
            world.query_filtered::<bevy::ecs::entity::Entity, (With<ListItem>, With<Selected>)>();

        query.single(world).unwrap()
    };

    // Selected + Hovered → Hovered
    app.world_mut()
        .entity_mut(selected_option)
        .insert(Hovered(true));

    app.update();

    assert_eq!(
        app.world()
            .get::<BackgroundColor>(selected_option)
            .unwrap()
            .0,
        EXPECTED_OPTION_HOVERED_BG,
    );

    // Disabled 在 ComboBox root 上，
    // 但 Option 应该解析成 Disabled style
    app.world_mut()
        .entity_mut(combo_box)
        .insert(InteractionDisabled);

    app.update();

    assert_eq!(
        app.world()
            .get::<BackgroundColor>(selected_option)
            .unwrap()
            .0,
        EXPECTED_OPTION_DISABLED_BG,
    );

    // Disabled 消失 → 回退到 Hovered
    app.world_mut()
        .entity_mut(combo_box)
        .remove::<InteractionDisabled>();

    app.update();

    assert_eq!(
        app.world()
            .get::<BackgroundColor>(selected_option)
            .unwrap()
            .0,
        EXPECTED_OPTION_HOVERED_BG,
    );

    // Hovered 消失 → 再回退到 Selected
    app.world_mut()
        .entity_mut(selected_option)
        .insert(Hovered(false));

    app.update();

    assert_eq!(
        app.world()
            .get::<BackgroundColor>(selected_option)
            .unwrap()
            .0,
        EXPECTED_OPTION_SELECTED_BG,
    );
}

#[rstest]
fn field_disabled_overrides_other_states_and_falls_back(mut app: App) {
    app.update();

    let combo_box = {
        let world = app.world_mut();

        let mut query = world.query_filtered::<bevy::ecs::entity::Entity, With<ComboBox>>();

        query.single(world).unwrap()
    };

    let (field, popup) = {
        let world = app.world();

        let root_children = world.get::<Children>(combo_box).unwrap();

        let field = root_children
            .iter()
            .copied()
            .find(|&child| world.get::<Button>(child).is_some())
            .expect("ComboBox should contain a field Button");

        let popup = root_children
            .iter()
            .copied()
            .find(|&child| world.get::<ListBox>(child).is_some())
            .expect("ComboBox should contain a popup ListBox");

        (field, popup)
    };

    // Hovered + Pressed + Open → Open
    app.world_mut()
        .entity_mut(field)
        .insert(Hovered(true))
        .insert(Pressed);

    *app.world_mut().get_mut::<Visibility>(popup).unwrap() = Visibility::Visible;

    app.update();

    assert_eq!(
        app.world().get::<BackgroundColor>(field).unwrap().0,
        EXPECTED_FIELD_OPEN_BG,
    );

    // Disabled 优先级最高
    app.world_mut()
        .entity_mut(combo_box)
        .insert(InteractionDisabled);

    app.update();

    assert_eq!(
        app.world().get::<BackgroundColor>(field).unwrap().0,
        EXPECTED_FIELD_DISABLED_BG,
    );

    // Disabled 时 Popup 已经被 Headless 关闭，
    // 所以恢复 enabled 后回退到 Pressed，而不是 Open
    app.world_mut()
        .entity_mut(combo_box)
        .remove::<InteractionDisabled>();

    app.update();

    assert_eq!(
        app.world().get::<BackgroundColor>(field).unwrap().0,
        EXPECTED_FIELD_PRESSED_BG,
    );

    // Pressed 消失 → Hovered
    app.world_mut().entity_mut(field).remove::<Pressed>();

    app.update();

    assert_eq!(
        app.world().get::<BackgroundColor>(field).unwrap().0,
        EXPECTED_FIELD_HOVERED_BG,
    );
}
