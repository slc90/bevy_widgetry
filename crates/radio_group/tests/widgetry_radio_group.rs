//! State：固定非空 options、selected index、root enabled/focus。
//! 不支持动态 option mutation。
//! Stimuli：初始化、label click、keyboard、set_selected queue、disabled 和 theme。
//! Guards：有效 root/direct child、合法 index、同值。
//! 组内恰好一个 Checked，程序化及 theme 静默。
//! Coverage Map：本文件负责 initialization/selection/interaction/style/composition，组内互斥由 assert_selected 统一检查。
//! 多组场景检查来源与隔离。
//! 局部 style/diagnostics 留在源码 module。

// 测试断言需要在 contract 不满足时立即失败。
// 生产代码的 panic lint 会拒绝这些表达式，因此仅在本测试 scope 允许所列 lint。
#![allow(clippy::disallowed_macros, clippy::expect_used, clippy::unwrap_used)]
#![cfg(test)]

use bevy::app::Propagate;
use bevy::input::{
    ButtonState,
    keyboard::{Key, KeyboardInput},
};
use bevy::input_focus::tab_navigation::{TabGroup, TabIndex};
use bevy::input_focus::{FocusCause, InputFocus, InputFocusVisible};
use bevy::picking::hover::Hovered;
use bevy::prelude::*;
use bevy::ui::{Checked, InteractionDisabled};
use bevy::ui_widgets::ValueChange;
use bevy::ui_widgets::{RadioButton, RadioGroup};
use bevy::window::PrimaryWindow;
use bevy_widgetry_core::ForegroundColor;
use bevy_widgetry_radio_group::{
    WidgetryRadioGroup, WidgetryRadioGroupPlugin, WidgetryRadioOption,
};
use bevy_widgetry_test_utils::{
    ErrorCapture, LogCapture, add_keyboard_dispatch, press_key, primary_click, queue_key,
    scene_app, switch_theme,
};
use bevy_widgetry_theme::{WIDGETRY_DARK_THEME, WIDGETRY_LIGHT_THEME, WidgetryThemeMode};

#[derive(Resource, Default)]
struct Changes {
    entities: Vec<(Entity, Entity, bool)>,
    indices: Vec<(Entity, usize, bool)>,
}

fn app() -> App {
    let mut app = scene_app();
    app.world_mut().register_component::<Window>();
    app.add_plugins(WidgetryRadioGroupPlugin)
        .init_resource::<InputFocusVisible>()
        .init_resource::<Changes>()
        .add_observer(
            |event: On<ValueChange<Entity>>, mut changes: ResMut<Changes>| {
                changes
                    .entities
                    .push((event.source, event.value, event.is_final));
            },
        )
        .add_observer(
            |event: On<ValueChange<usize>>, mut changes: ResMut<Changes>| {
                changes
                    .indices
                    .push((event.source, event.value, event.is_final));
            },
        );
    app
}

fn group_scene() -> impl Scene {
    bsn! {
        @WidgetryRadioGroup
        Children [
            (@WidgetryRadioOption Children [Text("Low")]),
            (@WidgetryRadioOption Checked Children [Text("Medium")]),
            (@WidgetryRadioOption Children [Text("High")]),
        ]
    }
}

#[test]
fn option_request_is_independent_of_group_request() {
    let mut app = app();
    let root = app.world_mut().spawn_scene(group_scene()).unwrap().id();
    app.update();
    let option = app.world().get::<Children>(root).unwrap()[0];
    app.world_mut().entity_mut(root).insert(InteractionDisabled);
    app.world_mut()
        .entity_mut(option)
        .insert(InteractionDisabled);
    app.world_mut()
        .entity_mut(root)
        .remove::<InteractionDisabled>();
    app.update();
    assert!(app.world().get::<InteractionDisabled>(option).is_some());
}

fn assert_selected(app: &App, root: Entity, index: usize) {
    for (position, option) in app
        .world()
        .get::<Children>(root)
        .unwrap()
        .iter()
        .enumerate()
    {
        assert_eq!(
            app.world().get::<Checked>(option).is_some(),
            position == index
        );
    }
}

#[test]
fn initializes_first_option_silently() {
    let mut app = app();
    let root = app.world_mut().spawn_scene(group_scene()).unwrap().id();
    app.update();
    assert_selected(&app, root, 0);
    let changes = app.world().resource::<Changes>();
    assert!(changes.entities.is_empty());
    assert!(changes.indices.is_empty());
}

#[test]
fn damaged_option_does_not_block_other_options_theme_update() {
    let mut app = app();
    let root = app.world_mut().spawn_scene(group_scene()).unwrap().id();
    app.update();
    let options = app.world().get::<Children>(root).unwrap().to_vec();
    let indicator = app.world().get::<Children>(options[0]).unwrap()[0];
    app.world_mut().despawn(indicator);
    app.set_error_handler(ErrorCapture::handler());
    let errors = ErrorCapture::default();
    let logs = LogCapture::default();
    errors.run(|| logs.run(|| switch_theme(&mut app, WidgetryThemeMode::Light)));
    let errors = errors.take();
    assert_eq!(errors.len(), 1);
    assert_eq!(errors[0].severity(), bevy::ecs::error::Severity::Error);
    for option in &options[1..] {
        let indicator = app.world().get::<Children>(*option).unwrap()[0];
        assert_eq!(
            *app.world().get::<BorderColor>(indicator).unwrap(),
            BorderColor::all(WIDGETRY_LIGHT_THEME.radio_group.container.normal.border)
        );
        assert_eq!(
            app.world()
                .get::<Propagate<ForegroundColor>>(*option)
                .unwrap()
                .0,
            ForegroundColor(WIDGETRY_LIGHT_THEME.radio_group.container.normal.foreground)
        );
    }
    assert_eq!(
        logs.records()
            .iter()
            .filter(|record| record.level == bevy::log::Level::ERROR)
            .count(),
        1
    );
}

#[test]
fn user_click_selects_once_and_converts_index() {
    let mut app = app();
    let root = app.world_mut().spawn_scene(group_scene()).unwrap().id();
    app.update();
    let option = app.world().get::<Children>(root).unwrap()[2];
    let label = app.world().get::<Children>(option).unwrap()[1];
    for _ in 0..2 {
        app.world_mut().trigger(primary_click(label));
        app.world_mut().flush();
        assert_selected(&app, root, 2);
        let changes = app.world().resource::<Changes>();
        assert_eq!(changes.entities, vec![(root, option, true)]);
        assert_eq!(changes.indices, vec![(root, 2, true)]);
    }
}

#[test]
fn forwards_non_final_value_change() {
    let mut app = app();
    let root = app.world_mut().spawn_scene(group_scene()).unwrap().id();
    app.update();
    let option = app.world().get::<Children>(root).unwrap()[1];
    app.world_mut().trigger(ValueChange {
        source: root,
        value: option,
        is_final: false,
    });
    app.world_mut().flush();
    assert_eq!(
        app.world().resource::<Changes>().indices,
        vec![(root, 1, false)]
    );
    assert_selected(&app, root, 1);
}

#[test]
fn programmatic_selection_is_silent_and_ignores_invalid_input() {
    let mut app = app();
    let root = app.world_mut().spawn_scene(group_scene()).unwrap().id();
    app.update();
    WidgetryRadioGroup::set_selected(&mut app.world_mut().commands(), root, 2);
    app.world_mut().flush();
    assert_selected(&app, root, 2);
    let option = app.world().get::<Children>(root).unwrap()[2];
    let tick = app
        .world()
        .entity(option)
        .get_ref::<Checked>()
        .unwrap()
        .last_changed();
    let unrelated = app.world_mut().spawn_empty().id();
    let deleted = app.world_mut().spawn_empty().id();
    app.world_mut().despawn(deleted);
    for (entity, index) in [
        (root, 2),
        (root, 3),
        (root, usize::MAX),
        (unrelated, 0),
        (deleted, 0),
        (Entity::PLACEHOLDER, 0),
    ] {
        WidgetryRadioGroup::set_selected(&mut app.world_mut().commands(), entity, index);
        app.world_mut().flush();
        app.update();
        assert_selected(&app, root, 2);
        assert_eq!(
            app.world()
                .entity(option)
                .get_ref::<Checked>()
                .unwrap()
                .last_changed(),
            tick
        );
    }
    assert!(app.world().resource::<Changes>().entities.is_empty());
    assert!(app.world().resource::<Changes>().indices.is_empty());
}

#[test]
fn selection_before_first_update_is_preserved() {
    let mut app = app();
    let root = app.world_mut().commands().spawn_scene(group_scene()).id();
    WidgetryRadioGroup::set_selected(&mut app.world_mut().commands(), root, 2);
    app.world_mut().flush();
    app.update();
    assert_selected(&app, root, 2);
    assert!(app.world().resource::<Changes>().entities.is_empty());
    assert!(app.world().resource::<Changes>().indices.is_empty());
}

#[test]
fn invalid_hierarchy_returns_error_and_logs() {
    for case in 0..4 {
        let capture = LogCapture::default();
        let mut app = app();
        let root = match case {
            0 => app
                .world_mut()
                .spawn_scene(bsn! { @WidgetryRadioGroup })
                .unwrap()
                .id(),
            1 => app
                .world_mut()
                .spawn_scene(bsn! { @WidgetryRadioGroup Children [Node] })
                .unwrap()
                .id(),
            2 => app
                .world_mut()
                .spawn_scene(bsn! { @WidgetryRadioOption })
                .unwrap()
                .id(),
            _ => app
                .world_mut()
                .spawn_scene(bsn! { Node Children [(@WidgetryRadioOption)] })
                .unwrap()
                .id(),
        };
        app.edit_schedule(PreUpdate, |schedule| {
            schedule.set_executor(bevy::ecs::schedule::SingleThreadedExecutor::new());
        });
        app.set_error_handler(ErrorCapture::handler());
        let errors = ErrorCapture::default();
        errors.run(|| capture.run(|| app.world_mut().run_schedule(PreUpdate)));
        let errors = errors.take();
        assert!(!errors.is_empty(), "case {case}");
        assert!(
            errors
                .iter()
                .all(|error| error.severity() == bevy::ecs::error::Severity::Error)
        );
        let records = capture.records();
        let error = records
            .iter()
            .find(|record| record.level == bevy::log::Level::ERROR)
            .unwrap();
        let field = if case == 2 { "child" } else { "root" };
        assert_eq!(error.fields.get(field), Some(&format!("{root:?}")));
        if case == 1 || case == 3 {
            let child = app.world().get::<Children>(root).unwrap()[0];
            assert_eq!(error.fields.get("child"), Some(&format!("{child:?}")));
        }
    }
}

#[test]
fn invalid_programmatic_initialization_reaches_command_handler() {
    let mut app = app();
    app.set_error_handler(ErrorCapture::handler());
    let root = app
        .world_mut()
        .spawn_scene(bsn! { @WidgetryRadioGroup Children [Node] })
        .unwrap()
        .id();
    let child = app.world().get::<Children>(root).unwrap()[0];
    WidgetryRadioGroup::set_selected(&mut app.world_mut().commands(), root, 0);
    let errors = ErrorCapture::default();
    let logs = LogCapture::default();
    errors.run(|| logs.run(|| app.world_mut().flush()));
    let errors = errors.take();
    assert_eq!(errors.len(), 1);
    assert_eq!(errors[0].severity(), bevy::ecs::error::Severity::Error);
    assert!(app.world().get::<Checked>(child).is_none());
    assert!(
        logs.records()
            .iter()
            .any(|record| record.level == bevy::log::Level::ERROR
                && record.fields.get("root") == Some(&format!("{root:?}")))
    );
}

#[test]
fn disabled_inherits_into_options_and_content_and_allows_programmatic_selection() {
    let mut app = app();
    let root = app
        .world_mut()
        .spawn_scene(bsn! { group_scene() InteractionDisabled })
        .unwrap()
        .id();
    app.update();
    let options = app.world().get::<Children>(root).unwrap().to_vec();
    for option in &options {
        assert!(app.world().get::<InteractionDisabled>(*option).is_some());
        for child in app.world().get::<Children>(*option).unwrap() {
            assert!(app.world().get::<InteractionDisabled>(*child).is_some());
        }
    }
    app.world_mut().trigger(primary_click(options[2]));
    app.world_mut().flush();
    assert_selected(&app, root, 0);
    WidgetryRadioGroup::set_selected(&mut app.world_mut().commands(), root, 1);
    app.world_mut().flush();
    assert_selected(&app, root, 1);
    assert!(app.world().resource::<Changes>().indices.is_empty());
    assert!(app.world().resource::<Changes>().entities.is_empty());
    app.world_mut()
        .entity_mut(root)
        .remove::<InteractionDisabled>();
    app.update();
    for (index, &option) in options.iter().enumerate() {
        assert!(app.world().get::<InteractionDisabled>(option).is_none());
        let label = app.world().get::<Children>(option).unwrap()[1];
        assert!(app.world().get::<InteractionDisabled>(label).is_none());
        assert_eq!(
            app.world().get::<Text>(label).unwrap().0,
            ["Low", "Medium", "High"][index]
        );
    }
    assert_selected(&app, root, 1);
    assert!(app.world().resource::<Changes>().indices.is_empty());
    app.world_mut().trigger(primary_click(options[2]));
    app.world_mut().flush();
    assert_selected(&app, root, 2);
    app.world_mut().entity_mut(root).insert(InteractionDisabled);
    app.update();
    app.world_mut().trigger(primary_click(options[0]));
    app.world_mut().flush();
    assert_selected(&app, root, 2);
    assert_eq!(app.world().get::<TabIndex>(root).unwrap().0, 0);
}

#[test]
fn keyboard_navigation_respects_disabled_before_dispatch() {
    let mut app = app();
    app.init_resource::<ButtonInput<KeyCode>>();
    add_keyboard_dispatch(&mut app);
    let window = app
        .world_mut()
        .spawn((Window::default(), PrimaryWindow))
        .id();
    let ui_root = app
        .world_mut()
        .spawn((Node::default(), TabGroup::default()))
        .id();
    let root = app.world_mut().spawn_scene(group_scene()).unwrap().id();
    app.world_mut().entity_mut(ui_root).add_child(root);
    app.update();
    // InputFocusPlugin 的 Startup 会把初始 focus 设为 primary window。
    // 先运行 Startup 再清空 focus，避免默认 focus 干扰本次 transition。
    app.world_mut().resource_mut::<InputFocus>().clear();
    assert_eq!(app.world().resource::<InputFocus>().get(), None);
    queue_key(
        &mut app,
        KeyboardInput {
            key_code: KeyCode::Tab,
            logical_key: Key::Tab,
            state: ButtonState::Pressed,
            text: None,
            repeat: false,
            window,
        },
    );
    app.update();
    assert_eq!(app.world().resource::<InputFocus>().get(), Some(root));
    assert!(app.world().resource::<InputFocusVisible>().0);
    assert_selected(&app, root, 0);
    assert!(app.world().resource::<Changes>().indices.is_empty());
    for (disabled, expected) in [(false, 1), (true, 1), (false, 2)] {
        if disabled {
            app.world_mut().entity_mut(root).insert(InteractionDisabled);
        } else {
            app.world_mut()
                .entity_mut(root)
                .remove::<InteractionDisabled>();
        }
        queue_key(
            &mut app,
            KeyboardInput {
                key_code: KeyCode::ArrowRight,
                logical_key: Key::ArrowRight,
                state: ButtonState::Pressed,
                text: None,
                repeat: false,
                window,
            },
        );
        app.update();
        assert_selected(&app, root, expected);
    }
    assert_eq!(
        app.world().resource::<Changes>().indices,
        vec![(root, 1, true), (root, 2, true)]
    );
}

#[test]
fn group_style_tracks_focus_disabled_and_theme() {
    let mut app = app();
    let root = app.world_mut().spawn_scene(group_scene()).unwrap().id();
    app.update();
    let assert_colors = |app: &App, background, border| {
        assert_eq!(
            app.world().get::<BackgroundColor>(root).unwrap().0,
            background
        );
        assert_eq!(
            *app.world().get::<BorderColor>(root).unwrap(),
            BorderColor::all(border)
        );
    };
    assert_colors(
        &app,
        WIDGETRY_DARK_THEME.radio_group.container.normal.background,
        WIDGETRY_DARK_THEME.radio_group.container.normal.border,
    );
    app.world_mut()
        .resource_mut::<InputFocus>()
        .set(root, FocusCause::Navigated);
    app.update();
    assert_colors(
        &app,
        WIDGETRY_DARK_THEME.radio_group.container.normal.background,
        WIDGETRY_DARK_THEME.radio_group.container.focused.border,
    );
    app.world_mut().entity_mut(root).insert(InteractionDisabled);
    app.update();
    assert_colors(
        &app,
        WIDGETRY_DARK_THEME
            .radio_group
            .container
            .disabled
            .background,
        WIDGETRY_DARK_THEME.radio_group.container.disabled.border,
    );
    switch_theme(&mut app, WidgetryThemeMode::Light);
    assert_colors(
        &app,
        WIDGETRY_LIGHT_THEME
            .radio_group
            .container
            .disabled
            .background,
        WIDGETRY_LIGHT_THEME.radio_group.container.disabled.border,
    );
    app.world_mut()
        .entity_mut(root)
        .remove::<InteractionDisabled>();
    app.update();
    assert_colors(
        &app,
        WIDGETRY_LIGHT_THEME.radio_group.container.normal.background,
        WIDGETRY_LIGHT_THEME.radio_group.container.focused.border,
    );
    app.world_mut().resource_mut::<InputFocus>().clear();
    app.update();
    assert_colors(
        &app,
        WIDGETRY_LIGHT_THEME.radio_group.container.normal.background,
        WIDGETRY_LIGHT_THEME.radio_group.container.normal.border,
    );
}

#[test]
fn option_styles_follow_state_and_theme_without_styling_user_nodes() {
    let mut app = app();
    let root = app.world_mut().spawn_scene(group_scene()).unwrap().id();
    app.update();
    let option = app.world().get::<Children>(root).unwrap()[1];
    let children = app.world().get::<Children>(option).unwrap();
    let (indicator, label) = (children[0], children[1]);
    let dot = app.world().get::<Children>(indicator).unwrap()[0];
    for mode in [WidgetryThemeMode::Dark, WidgetryThemeMode::Light] {
        switch_theme(&mut app, mode);
        for (checked, hovered, disabled) in [
            (false, false, false),
            (false, true, false),
            (true, false, false),
            (true, true, false),
            (true, true, true),
            (false, true, true),
            (false, false, false),
        ] {
            WidgetryRadioGroup::set_selected(
                &mut app.world_mut().commands(),
                root,
                usize::from(checked),
            );
            app.world_mut().flush();
            app.world_mut().entity_mut(option).insert(Hovered(hovered));
            if disabled {
                app.world_mut().entity_mut(root).insert(InteractionDisabled);
            } else {
                app.world_mut()
                    .entity_mut(root)
                    .remove::<InteractionDisabled>();
            }
            app.update();
            let colors = mode.colors();
            let border = if disabled {
                colors.radio_group.container.disabled.border
            } else if hovered {
                colors.radio_group.option.unchecked.hovered.border
            } else if checked {
                colors.radio_group.option.checked.normal.border
            } else {
                colors.radio_group.container.normal.border
            };
            let fill = if !checked {
                Color::NONE
            } else if disabled {
                colors.radio_group.container.disabled.foreground
            } else {
                if hovered {
                    colors.radio_group.option.checked.hovered.dot
                } else {
                    colors.radio_group.option.checked.normal.dot
                }
            };
            let foreground = if disabled {
                colors.radio_group.container.disabled.foreground
            } else {
                colors.radio_group.container.normal.foreground
            };
            assert_eq!(
                *app.world().get::<BorderColor>(indicator).unwrap(),
                BorderColor::all(border)
            );
            assert_eq!(app.world().get::<BackgroundColor>(dot).unwrap().0, fill);
            assert_eq!(
                app.world()
                    .get::<Propagate<ForegroundColor>>(option)
                    .unwrap()
                    .0
                    .0,
                foreground
            );
            assert_eq!(app.world().get::<TextColor>(label).unwrap().0, foreground);
            assert_eq!(
                app.world().get::<BackgroundColor>(option).unwrap().0,
                Color::NONE
            );
            assert_eq!(
                *app.world().get::<BorderColor>(option).unwrap(),
                BorderColor::default()
            );
        }
    }
    WidgetryRadioGroup::set_selected(&mut app.world_mut().commands(), root, 1);
    app.world_mut().flush();
    app.update();
    switch_theme(&mut app, WidgetryThemeMode::Dark);
    assert_eq!(
        app.world().get::<BackgroundColor>(dot).unwrap().0,
        WIDGETRY_DARK_THEME.radio_group.option.checked.normal.dot
    );
}

#[test]
fn layout_patches_preserve_selection_semantics() {
    let mut app = app();
    let horizontal = app
        .world_mut()
        .spawn_scene(bsn! {
            group_scene() Node { flex_direction: FlexDirection::Row, column_gap: px(20) }
        })
        .unwrap()
        .id();
    let grid = app.world_mut().spawn_scene(bsn! {
        group_scene() Node { display: Display::Grid, grid_template_columns: {vec![RepeatedGridTrack::auto(2)]} }
    }).unwrap().id();
    app.update();
    assert_eq!(
        app.world().get::<Node>(horizontal).unwrap().flex_direction,
        FlexDirection::Row
    );
    assert_eq!(
        app.world().get::<Node>(grid).unwrap().display,
        Display::Grid
    );
    assert_eq!(
        app.world().get::<Node>(grid).unwrap().grid_template_columns,
        vec![RepeatedGridTrack::auto(2)]
    );
    for root in [horizontal, grid] {
        assert_eq!(
            app.world().get::<Node>(root).unwrap().padding,
            UiRect::all(px(8))
        );
        assert_selected(&app, root, 0);
        let option = app.world().get::<Children>(root).unwrap()[2];
        app.world_mut().trigger(primary_click(option));
        app.world_mut().flush();
        assert_selected(&app, root, 2);
    }
}

#[test]
fn plugin_reuses_dependencies_and_preserves_font_policy() {
    let mut app = scene_app();
    app.add_plugins((
        bevy::ui_widgets::RadioGroupPlugin,
        bevy::input_focus::tab_navigation::TabNavigationPlugin,
    ));
    app.insert_resource(WidgetryThemeMode::Light)
        .add_plugins(WidgetryRadioGroupPlugin);
    let root = app.world_mut().spawn_scene(group_scene()).unwrap().id();
    app.update();
    assert_selected(&app, root, 0);
    assert_eq!(
        *app.world().resource::<WidgetryThemeMode>(),
        WidgetryThemeMode::Light
    );
    let option = app.world().get::<Children>(root).unwrap()[0];
    let label = app.world().get::<Children>(option).unwrap()[1];
    assert_eq!(
        app.world().get::<TextFont>(label).unwrap().font,
        bevy::text::FontSource::Monospace
    );
}

#[test]
fn scene_composes_indicator_and_user_content() {
    let mut app = scene_app();
    app.add_plugins(WidgetryRadioGroupPlugin);
    let root = app
        .world_mut()
        .spawn_scene(bsn! {
            @WidgetryRadioGroup
            Children [
                (@WidgetryRadioOption Children [(Node Children [Text("Low")]), Text("Extra")]),
                (@WidgetryRadioOption Children [Text("High")]),
            ]
        })
        .unwrap()
        .id();
    app.update();
    assert!(app.world().get::<RadioGroup>(root).is_some());
    assert_eq!(app.world().get::<TabIndex>(root).unwrap().0, 0);
    let options = app.world().get::<Children>(root).unwrap();
    assert_eq!(options.len(), 2);
    let first = options[0];
    assert!(app.world().get::<RadioButton>(first).is_some());
    let contents = app.world().get::<Children>(first).unwrap();
    assert_eq!(contents.len(), 3);
    let indicator = contents[0];
    assert_eq!(app.world().get::<Node>(indicator).unwrap().width, px(16));
    let dot = app.world().get::<Children>(indicator).unwrap()[0];
    assert_eq!(app.world().get::<Node>(dot).unwrap().width, px(8));
    assert_eq!(app.world().get::<Text>(contents[2]).unwrap().0, "Extra");
}

#[test]
fn single_option_is_stable_for_repeated_selection() {
    let mut app = app();
    add_keyboard_dispatch(&mut app);
    app.init_resource::<ButtonInput<KeyCode>>();
    let window = app
        .world_mut()
        .spawn((Window::default(), PrimaryWindow))
        .id();
    let root = app
        .world_mut()
        .spawn_scene(
            bsn! { @WidgetryRadioGroup Children [@WidgetryRadioOption Children [Text("Only")]] },
        )
        .unwrap()
        .id();
    app.update();
    let option = app.world().get::<Children>(root).unwrap()[0];
    let label = app.world().get::<Children>(option).unwrap()[1];
    for _ in 0..2 {
        app.world_mut().trigger(primary_click(label));
        WidgetryRadioGroup::set_selected(&mut app.world_mut().commands(), root, 0);
        app.world_mut().flush();
        assert_selected(&app, root, 0);
    }
    app.world_mut()
        .resource_mut::<InputFocus>()
        .set(root, FocusCause::Navigated);
    press_key(&mut app, window, KeyCode::ArrowRight);
    assert_selected(&app, root, 0);
    assert!(app.world().resource::<Changes>().indices.is_empty());
    assert!(app.world().resource::<Changes>().entities.is_empty());
}

#[test]
fn independent_groups_isolate_selection_and_notifications() {
    let mut app = app();
    let a = app.world_mut().spawn_scene(group_scene()).unwrap().id();
    let b = app.world_mut().spawn_scene(group_scene()).unwrap().id();
    app.update();
    assert_selected(&app, a, 0);
    assert_selected(&app, b, 0);
    let a_option = app.world().get::<Children>(a).unwrap()[2];
    let b_option = app.world().get::<Children>(b).unwrap()[1];
    app.world_mut().trigger(primary_click(a_option));
    app.world_mut().flush();
    assert_selected(&app, a, 2);
    assert_selected(&app, b, 0);
    WidgetryRadioGroup::set_selected(&mut app.world_mut().commands(), b, 2);
    app.world_mut().flush();
    assert_selected(&app, a, 2);
    assert_selected(&app, b, 2);
    app.world_mut().trigger(primary_click(b_option));
    app.world_mut().flush();
    assert_selected(&app, a, 2);
    assert_selected(&app, b, 1);
    WidgetryRadioGroup::set_selected(&mut app.world_mut().commands(), a, 0);
    app.world_mut().flush();
    assert_selected(&app, a, 0);
    assert_selected(&app, b, 1);
    switch_theme(&mut app, WidgetryThemeMode::Light);
    assert_eq!(
        app.world().resource::<Changes>().indices,
        vec![(a, 2, true), (b, 1, true)]
    );
    assert_eq!(
        app.world().resource::<Changes>().entities,
        vec![(a, a_option, true), (b, b_option, true)]
    );
}

#[test]
fn queued_selection_preserves_last_valid_value_and_other_groups() {
    let mut app = app();
    let a = app.world_mut().spawn_scene(group_scene()).unwrap().id();
    let b = app.world_mut().spawn_scene(group_scene()).unwrap().id();
    app.update();
    for index in [1, 2, usize::MAX] {
        WidgetryRadioGroup::set_selected(&mut app.world_mut().commands(), a, index);
    }
    assert_selected(&app, a, 0);
    app.world_mut().flush();
    assert_selected(&app, a, 2);
    assert_selected(&app, b, 0);
    WidgetryRadioGroup::set_selected(&mut app.world_mut().commands(), a, 1);
    app.world_mut().despawn(a);
    app.world_mut().flush();
    assert!(app.world().get_entity(a).is_err());
    assert_selected(&app, b, 0);
    assert!(app.world().resource::<Changes>().indices.is_empty());
    assert!(app.world().resource::<Changes>().entities.is_empty());
}

#[test]
fn invalid_group_does_not_block_healthy_group_and_recovers() {
    let mut app = app();
    app.set_error_handler(ErrorCapture::handler());
    app.edit_schedule(PreUpdate, |schedule| {
        schedule.set_executor(bevy::ecs::schedule::SingleThreadedExecutor::new());
    });
    let invalid = app
        .world_mut()
        .spawn_scene(bsn! { @WidgetryRadioGroup })
        .unwrap()
        .id();
    let healthy = app.world_mut().spawn_scene(group_scene()).unwrap().id();
    let errors = ErrorCapture::default();
    let logs = LogCapture::default();
    for _ in 0..2 {
        errors.run(|| logs.run(|| app.update()));
        assert_selected(&app, healthy, 0);
    }
    assert_eq!(errors.take().len(), 2);
    assert_eq!(
        logs.records()
            .iter()
            .filter(|r| r.level == bevy::log::Level::ERROR)
            .count(),
        1
    );
    app.world_mut()
        .entity_mut(invalid)
        .apply_scene(bsn! { Children [(@WidgetryRadioOption)] })
        .unwrap();
    errors.run(|| logs.run(|| app.update()));
    assert!(errors.take().is_empty());
    assert_selected(&app, invalid, 0);
    assert_eq!(
        logs.records()
            .iter()
            .filter(|r| r.fields.get("message").is_some_and(|m| m.contains("恢复")))
            .count(),
        1
    );
}

#[test]
fn orphan_option_failure_is_reported_once_and_parent_repair_recovers() {
    let mut app = app();
    app.set_error_handler(ErrorCapture::handler());
    app.edit_schedule(PreUpdate, |schedule| {
        schedule.set_executor(bevy::ecs::schedule::SingleThreadedExecutor::new());
    });
    let orphan = app
        .world_mut()
        .spawn_scene(bsn! { @WidgetryRadioOption })
        .unwrap()
        .id();
    let healthy = app.world_mut().spawn_scene(group_scene()).unwrap().id();
    let errors = ErrorCapture::default();
    let logs = LogCapture::default();
    for _ in 0..2 {
        errors.run(|| logs.run(|| app.update()));
        assert_selected(&app, healthy, 0);
    }
    assert_eq!(errors.take().len(), 2);
    assert_eq!(
        logs.records()
            .iter()
            .filter(|r| r.level == bevy::log::Level::ERROR)
            .count(),
        1
    );
    let repaired = app
        .world_mut()
        .spawn_scene(bsn! { @WidgetryRadioGroup })
        .unwrap()
        .id();
    app.world_mut().entity_mut(repaired).add_child(orphan);
    errors.run(|| logs.run(|| app.update()));
    assert!(errors.take().is_empty());
    assert_selected(&app, repaired, 0);
    assert_eq!(
        logs.records()
            .iter()
            .filter(|r| r.fields.get("message").is_some_and(|m| m.contains("恢复")))
            .count(),
        1
    );
}
