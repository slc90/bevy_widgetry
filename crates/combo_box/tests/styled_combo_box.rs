#![cfg(test)]

use bevy::ecs::{entity::Entity, world::World};
use bevy::{
    app::{App, Propagate, Startup},
    camera::visibility::Visibility,
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
use bevy_widgetry_core::{DARK_THEME, ForegroundColor, LIGHT_THEME, ThemeMode};
use bevy_widgetry_test_utils::switch_theme;
use rstest::{fixture, rstest};

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

fn combo_box_root(world: &mut World) -> Entity {
    world
        .query_filtered::<Entity, With<ComboBox>>()
        .single(world)
        .unwrap()
}

fn combo_box_field(world: &World, root: Entity) -> Entity {
    world
        .get::<Children>(root)
        .unwrap()
        .iter()
        .copied()
        .find(|&child| world.get::<Button>(child).is_some())
        .expect("ComboBox field")
}

fn combo_box_popup(world: &World, root: Entity) -> Entity {
    world
        .get::<Children>(root)
        .unwrap()
        .iter()
        .copied()
        .find(|&child| world.get::<ListBox>(child).is_some())
        .expect("ComboBox popup")
}

fn combo_box_field_texts(world: &World, field: Entity) -> Vec<&str> {
    world
        .get::<Children>(field)
        .unwrap()
        .iter()
        .filter_map(|&child| world.get::<Text>(child))
        .map(|text| text.0.as_str())
        .collect()
}

fn combo_box_option(world: &World, popup: Entity, label: &str) -> Entity {
    world
        .get::<Children>(popup)
        .unwrap()
        .iter()
        .copied()
        .find(|&option| {
            world.get::<ListItem>(option).is_some()
                && world
                    .get::<Children>(option)
                    .unwrap()
                    .iter()
                    .any(|&child| world.get::<Text>(child).is_some_and(|text| text.0 == label))
        })
        .expect("ComboBox option label")
}

// 初始化真实 ECS 子树，验证输入区、弹层和选项具备对应可视组件。
#[rstest]
fn spawned_combo_box_has_visual_structure(mut app: App) {
    app.update();

    let world = app.world_mut();

    let combo_box = {
        let mut query =
            world.query_filtered::<(bevy::ecs::entity::Entity, Has<Node>), With<ComboBox>>();

        let (entity, has_node) = query.single(world).unwrap();

        assert!(has_node);

        entity
    };

    let field = combo_box_field(world, combo_box);

    assert!(world.get::<Node>(field).is_some());
    assert!(world.get::<BackgroundColor>(field).is_some());
    assert!(world.get::<BorderColor>(field).is_some());

    let popup = combo_box_popup(world, combo_box);

    assert!(world.get::<Node>(popup).is_some());
    assert!(world.get::<BackgroundColor>(popup).is_some());
    assert!(world.get::<BorderColor>(popup).is_some());

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

// 没有用户事件的首次更新，应将默认选项标签显示到输入区域。
#[rstest]
fn spawned_combo_box_field_shows_first_option(mut app: App) {
    app.update();

    let combo_box = combo_box_root(app.world_mut());

    let world = app.world_mut();

    let field = combo_box_field(world, combo_box);

    let texts = combo_box_field_texts(world, field);

    assert!(texts.contains(&"Apple"));
}

// 程序化更改选项，验证显示文本从 Selected 派生而不依赖用户事件。
#[rstest]
fn programmatic_selection_updates_field_text(mut app: App) {
    app.update();

    let combo_box = combo_box_root(app.world_mut());

    // 初始 selection = 0，所以 Field 显示 Apple
    {
        let world = app.world();

        let field = combo_box_field(world, combo_box);

        let texts = combo_box_field_texts(world, field);

        assert!(texts.contains(&"Apple"));
    }

    app.world_mut().trigger(SetComboBoxSelected {
        entity: combo_box,
        selected: 2,
    });

    app.update();

    // Field Text 应该跟随 Selected 状态变化
    {
        let world = app.world();

        let field = combo_box_field(world, combo_box);

        let texts = combo_box_field_texts(world, field);

        assert!(texts.contains(&"Orange"));
        assert!(!texts.contains(&"Apple"));
    }
}

// 在两个选项之间切换，验证旧选项恢复默认色、新选项采用选中色。
#[rstest]
fn programmatic_selection_updates_option_styles(mut app: App) {
    app.update();

    let combo_box = combo_box_root(app.world_mut());

    // 找 Popup
    let popup = {
        let world = app.world();

        combo_box_popup(world, combo_box)
    };

    // 通过每个 ListItem 下面的 Text 找 Apple / Orange
    let apple = combo_box_option(app.world(), popup, "Apple");
    let orange = combo_box_option(app.world(), popup, "Orange");

    // 初始：Apple selected，Orange default
    assert_eq!(
        app.world().get::<BackgroundColor>(apple).unwrap().0,
        DARK_THEME.item_background_selected,
    );

    assert_eq!(
        app.world().get::<BackgroundColor>(orange).unwrap().0,
        DARK_THEME.popup_background,
    );

    app.world_mut().trigger(SetComboBoxSelected {
        entity: combo_box,
        selected: 2,
    });

    app.update();

    // Apple 恢复 default
    assert_eq!(
        app.world().get::<BackgroundColor>(apple).unwrap().0,
        DARK_THEME.popup_background,
    );

    // Orange 变 selected
    assert_eq!(
        app.world().get::<BackgroundColor>(orange).unwrap().0,
        DARK_THEME.item_background_selected,
    );
}

// 依次叠加悬停、按压和打开状态，验证每次都采用最高优先级配色。
#[rstest]
fn field_style_priority_is_open_then_pressed_then_hovered(mut app: App) {
    app.update();

    let combo_box = combo_box_root(app.world_mut());

    let (field, popup) = {
        let world = app.world();

        let field = combo_box_field(world, combo_box);

        let popup = combo_box_popup(world, combo_box);

        (field, popup)
    };

    app.world_mut().entity_mut(field).insert(Hovered(true));

    app.update();

    assert_eq!(
        app.world().get::<BackgroundColor>(field).unwrap().0,
        DARK_THEME.control_background_hovered,
    );

    // Pressed 应该覆盖 Hovered
    app.world_mut().entity_mut(field).insert(Pressed);

    app.update();

    assert_eq!(
        app.world().get::<BackgroundColor>(field).unwrap().0,
        DARK_THEME.control_background_pressed,
    );

    // Open 应该再覆盖 Pressed
    *app.world_mut().get_mut::<Visibility>(popup).unwrap() = Visibility::Visible;

    app.update();

    assert_eq!(
        app.world().get::<BackgroundColor>(field).unwrap().0,
        DARK_THEME.control_background_active,
    );
}

// 逐个移除较高优先级状态，验证背景逐级恢复到剩余状态。
#[rstest]
fn field_style_falls_back_when_higher_priority_states_are_removed(mut app: App) {
    app.update();

    let combo_box = combo_box_root(app.world_mut());

    let (field, popup) = {
        let world = app.world();

        let field = combo_box_field(world, combo_box);

        let popup = combo_box_popup(world, combo_box);

        (field, popup)
    };

    app.world_mut()
        .entity_mut(field)
        .insert(Hovered(true))
        .insert(Pressed);

    *app.world_mut().get_mut::<Visibility>(popup).unwrap() = Visibility::Visible;

    app.update();

    assert_eq!(
        app.world().get::<BackgroundColor>(field).unwrap().0,
        DARK_THEME.control_background_active,
    );

    // Open 消失 → 回退到 Pressed
    *app.world_mut().get_mut::<Visibility>(popup).unwrap() = Visibility::Hidden;

    app.update();

    assert_eq!(
        app.world().get::<BackgroundColor>(field).unwrap().0,
        DARK_THEME.control_background_pressed,
    );

    // Pressed 消失 → 回退到 Hovered
    app.world_mut().entity_mut(field).remove::<Pressed>();

    app.update();

    assert_eq!(
        app.world().get::<BackgroundColor>(field).unwrap().0,
        DARK_THEME.control_background_hovered,
    );

    // Hovered 消失 → 回退到 Default
    app.world_mut().entity_mut(field).insert(Hovered(false));

    app.update();

    assert_eq!(
        app.world().get::<BackgroundColor>(field).unwrap().0,
        DARK_THEME.control_background,
    );
}

// 首次创建弹层，验证背景、边框和布局共同满足样式契约。
#[rstest]
fn popup_has_expected_style(mut app: App) {
    app.update();

    let combo_box = combo_box_root(app.world_mut());

    let popup = {
        let world = app.world();

        combo_box_popup(world, combo_box)
    };

    let world = app.world();

    assert_eq!(
        world.get::<BackgroundColor>(popup).unwrap().0,
        DARK_THEME.popup_background,
    );

    let border = world.get::<BorderColor>(popup).unwrap();

    assert_eq!(border.top, DARK_THEME.popup_border);
    assert_eq!(border.right, DARK_THEME.popup_border);
    assert_eq!(border.bottom, DARK_THEME.popup_border);
    assert_eq!(border.left, DARK_THEME.popup_border);

    let node = world.get::<Node>(popup).unwrap();

    assert_eq!(node.position_type, PositionType::Absolute);
    assert_eq!(node.left, Val::Px(0.0));
    assert_eq!(node.top, Val::Percent(100.0));
    assert_eq!(node.width, Val::Percent(100.0));
    assert_eq!(node.flex_direction, FlexDirection::Column);
}

// 选中项进入再退出悬停，验证悬停临时覆盖但不删除选择状态。
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
        DARK_THEME.item_background_selected,
    );

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
        DARK_THEME.item_background_hovered,
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
        DARK_THEME.item_background_selected,
    );
}

// 未选中项进入再退出悬停，验证退出后不误用选中色。
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
        DARK_THEME.popup_background,
    );

    app.world_mut().entity_mut(option).insert(Hovered(true));

    app.update();

    assert_eq!(
        app.world().get::<BackgroundColor>(option).unwrap().0,
        DARK_THEME.item_background_hovered,
    );

    // Hovered 消失 → Default
    app.world_mut().entity_mut(option).insert(Hovered(false));

    app.update();

    assert_eq!(
        app.world().get::<BackgroundColor>(option).unwrap().0,
        DARK_THEME.popup_background,
    );
}

// 在根控件禁用时保留选项状态，验证禁用覆盖及恢复后的状态优先级。
#[rstest]
fn disabled_option_overrides_hovered_and_selected(mut app: App) {
    app.update();

    let combo_box = combo_box_root(app.world_mut());

    let selected_option = {
        let world = app.world_mut();

        let mut query =
            world.query_filtered::<bevy::ecs::entity::Entity, (With<ListItem>, With<Selected>)>();

        query.single(world).unwrap()
    };

    app.world_mut()
        .entity_mut(selected_option)
        .insert(Hovered(true));

    app.update();

    assert_eq!(
        app.world()
            .get::<BackgroundColor>(selected_option)
            .unwrap()
            .0,
        DARK_THEME.item_background_hovered,
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
        DARK_THEME.control_background_disabled,
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
        DARK_THEME.item_background_hovered,
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
        DARK_THEME.item_background_selected,
    );
}

// 禁用已打开的控件再恢复，验证弹层关闭后只恢复仍有效的交互样式。
#[rstest]
fn field_disabled_overrides_other_states_and_falls_back(mut app: App) {
    app.update();

    let combo_box = combo_box_root(app.world_mut());

    let (field, popup) = {
        let world = app.world();

        let field = combo_box_field(world, combo_box);

        let popup = combo_box_popup(world, combo_box);

        (field, popup)
    };

    app.world_mut()
        .entity_mut(field)
        .insert(Hovered(true))
        .insert(Pressed);

    *app.world_mut().get_mut::<Visibility>(popup).unwrap() = Visibility::Visible;

    app.update();

    assert_eq!(
        app.world().get::<BackgroundColor>(field).unwrap().0,
        DARK_THEME.control_background_active,
    );

    // Disabled 优先级最高
    app.world_mut()
        .entity_mut(combo_box)
        .insert(InteractionDisabled);

    app.update();

    assert_eq!(
        app.world().get::<BackgroundColor>(field).unwrap().0,
        DARK_THEME.control_background_disabled,
    );

    // Disabled 时 Popup 已经被 Headless 关闭，
    // 所以恢复 enabled 后回退到 Pressed，而不是 Open
    app.world_mut()
        .entity_mut(combo_box)
        .remove::<InteractionDisabled>();

    app.update();

    assert_eq!(
        app.world().get::<BackgroundColor>(field).unwrap().0,
        DARK_THEME.control_background_pressed,
    );

    // Pressed 消失 → Hovered
    app.world_mut().entity_mut(field).remove::<Pressed>();

    app.update();

    assert_eq!(
        app.world().get::<BackgroundColor>(field).unwrap().0,
        DARK_THEME.control_background_hovered,
    );
}

fn assert_foreground(app: &App, entity: Entity, color: bevy::color::Color) {
    assert_eq!(
        app.world()
            .get::<Propagate<ForegroundColor>>(entity)
            .unwrap()
            .0
            .0,
        color
    );
}

// 先设置主题再创建控件，验证输入区、弹层和选项初始化均读取当前配色。
#[rstest]
fn new_combo_box_uses_current_theme(mut app: App) {
    switch_theme(&mut app, ThemeMode::Light);
    app.update();
    let root = combo_box_root(app.world_mut());
    let field = combo_box_field(app.world(), root);
    let popup = combo_box_popup(app.world(), root);
    let selected = combo_box_option(app.world(), popup, "Apple");
    let unselected = combo_box_option(app.world(), popup, "Orange");
    assert_eq!(
        app.world().get::<BackgroundColor>(field).unwrap().0,
        LIGHT_THEME.control_background
    );
    assert_eq!(
        *app.world().get::<BorderColor>(field).unwrap(),
        BorderColor::all(LIGHT_THEME.control_border)
    );
    assert_eq!(
        app.world().get::<BackgroundColor>(popup).unwrap().0,
        LIGHT_THEME.popup_background
    );
    assert_eq!(
        *app.world().get::<BorderColor>(popup).unwrap(),
        BorderColor::all(LIGHT_THEME.popup_border)
    );
    assert_eq!(
        app.world().get::<BackgroundColor>(selected).unwrap().0,
        LIGHT_THEME.item_background_selected
    );
    assert_eq!(
        app.world().get::<BackgroundColor>(unselected).unwrap().0,
        LIGHT_THEME.popup_background
    );
    for entity in [field, selected, unselected] {
        assert_foreground(&app, entity, LIGHT_THEME.foreground);
    }
}

// 打开弹层并保留选中和悬停状态，验证主题事件立即刷新整棵可视子树。
#[rstest]
fn theme_switch_immediately_refreshes_open_field_options_and_popup(mut app: App) {
    app.update();
    let root = combo_box_root(app.world_mut());
    let field = combo_box_field(app.world(), root);
    let popup = combo_box_popup(app.world(), root);
    let selected = combo_box_option(app.world(), popup, "Apple");
    let hovered = combo_box_option(app.world(), popup, "Orange");
    app.world_mut()
        .entity_mut(field)
        .insert((Hovered(true), Pressed));
    app.world_mut().entity_mut(hovered).insert(Hovered(true));
    *app.world_mut().get_mut::<Visibility>(popup).unwrap() = Visibility::Visible;
    app.update();
    assert_eq!(
        app.world().get::<BackgroundColor>(field).unwrap().0,
        DARK_THEME.control_background_active
    );
    assert_eq!(
        app.world().get::<BackgroundColor>(selected).unwrap().0,
        DARK_THEME.item_background_selected
    );
    assert_eq!(
        app.world().get::<BackgroundColor>(popup).unwrap().0,
        DARK_THEME.popup_background
    );
    for mode in [ThemeMode::Light, ThemeMode::Dark] {
        switch_theme(&mut app, mode);
        let c = mode.colors();
        assert_eq!(
            app.world().get::<BackgroundColor>(field).unwrap().0,
            c.control_background_active
        );
        assert_eq!(
            *app.world().get::<BorderColor>(field).unwrap(),
            BorderColor::all(c.control_border_active)
        );
        assert_eq!(
            app.world().get::<BackgroundColor>(selected).unwrap().0,
            c.item_background_selected
        );
        assert_eq!(
            app.world().get::<BackgroundColor>(hovered).unwrap().0,
            c.item_background_hovered
        );
        assert_eq!(
            app.world().get::<BackgroundColor>(popup).unwrap().0,
            c.popup_background
        );
        assert_eq!(
            *app.world().get::<BorderColor>(popup).unwrap(),
            BorderColor::all(c.popup_border)
        );
        for entity in [field, selected, hovered] {
            assert_foreground(&app, entity, c.foreground);
        }
        assert_eq!(
            *app.world().get::<Visibility>(popup).unwrap(),
            Visibility::Visible
        );
        assert!(app.world().get::<Selected>(selected).is_some());
        assert!(app.world().get::<Pressed>(field).is_some());
        assert!(app.world().get::<Hovered>(field).unwrap().0);
        assert!(app.world().get::<Hovered>(hovered).unwrap().0);
    }
}

// 禁用状态下切换主题，验证颜色更新但弹层仍关闭、状态仍禁用。
#[rstest]
fn theme_switch_preserves_disabled_combo_box(mut app: App) {
    app.update();
    let root = combo_box_root(app.world_mut());
    let field = combo_box_field(app.world(), root);
    let popup = combo_box_popup(app.world(), root);
    let option = combo_box_option(app.world(), popup, "Apple");
    app.world_mut().entity_mut(root).insert(InteractionDisabled);
    app.update();
    switch_theme(&mut app, ThemeMode::Light);
    for entity in [field, option] {
        assert_eq!(
            app.world().get::<BackgroundColor>(entity).unwrap().0,
            LIGHT_THEME.control_background_disabled
        );
        assert_foreground(&app, entity, LIGHT_THEME.foreground_disabled);
    }
    assert_eq!(
        *app.world().get::<BorderColor>(field).unwrap(),
        BorderColor::all(LIGHT_THEME.control_border_disabled)
    );
    assert!(app.world().get::<InteractionDisabled>(root).is_some());
    assert!(app.world().get::<Selected>(option).is_some());
    assert_eq!(
        *app.world().get::<Visibility>(popup).unwrap(),
        Visibility::Hidden
    );
}
