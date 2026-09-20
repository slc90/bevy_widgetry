#![cfg(test)]

use bevy::app::Propagate;
use bevy::input::{
    ButtonState,
    keyboard::{Key, KeyboardInput},
};
use bevy::input_focus::tab_navigation::{TabGroup, TabIndex};
use bevy::input_focus::{
    FocusCause, InputFocus, InputFocusSystems, InputFocusVisible, dispatch_focused_input,
};
use bevy::picking::hover::Hovered;
use bevy::prelude::*;
use bevy::ui::{Checked, InteractionDisabled};
use bevy::ui_widgets::ValueChange;
use bevy::ui_widgets::{RadioButton, RadioGroup};
use bevy::window::PrimaryWindow;
use bevy_widgetry_core::{DARK_THEME, ForegroundColor, LIGHT_THEME, ThemeMode};
use bevy_widgetry_radio_group::{
    WidgetryRadioGroup, WidgetryRadioGroupPlugin, WidgetryRadioOption,
};
use bevy_widgetry_test_utils::{LogCapture, primary_click, scene_app, switch_theme};
use std::panic::{AssertUnwindSafe, catch_unwind};

/// 同时捕获官方与公开通知，验证初始化与程序化操作保持静默。
#[derive(Resource, Default)]
struct Changes {
    /// 官方用户 selection event。
    entities: Vec<(Entity, Entity, bool)>,
    /// Widgetry 的 root index event。
    indices: Vec<(Entity, usize, bool)>,
}

/// 为 Radio 的 integration 测试装配官方输入所需的 resource 与通知记录。
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

/// 使用固定三项内容验证 index 语义，第二项故意预置 Checked。
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

/// 检查整个 Group 的唯一 Checked，避免只验证目标而漏掉旧 selection。
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

// 初始化必须覆盖调用方预先插入的 Checked，静默建立首项唯一选中的 invariant。
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

// 从 option 的 label 触发真实 click，验证官方互斥选择与 root index 通知；重选不得重复通知。
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

// 直接在官方 event 边界输入非 final 通知，Widgetry 必须原样保留 is_final。
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

// 有效 index 更新唯一 Checked；同值、越界及无效 entity 均不写 state，也不发送任何通知。
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

// 与 ComboBox 一样允许 Scene spawn 后立即设置初值，后续初始化不得把显式选择重置为 index 0。
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

// 空 Group、非 Option child、独立或挂在普通 Node 下的 Option 均必须先记录 entity context 再 panic。
#[test]
fn invalid_hierarchy_logs_before_panicking() {
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
        let result = capture.run(|| catch_unwind(AssertUnwindSafe(|| app.update())));
        assert!(result.is_err(), "case {case}");
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

// 初始及运行期 disabled 只镜像到 option root；禁止 click，允许程序化修改，移除后恢复交互。
#[test]
fn disabled_mirrors_options_and_allows_programmatic_selection() {
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
            assert!(app.world().get::<InteractionDisabled>(*child).is_none());
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
    for &option in &options {
        assert!(app.world().get::<InteractionDisabled>(option).is_none());
    }
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

// 调用方在 ancestor 提供 TabGroup，从无 focus 状态通过 Tab 进入 Group，
// 再验证方向键选择与同帧 disabled 镜像，避免手动设置 focus 绕过真实入口。
#[test]
fn keyboard_navigation_respects_disabled_before_dispatch() {
    let mut app = app();
    app.init_resource::<ButtonInput<KeyCode>>();
    app.add_message::<KeyboardInput>().add_systems(
        PreUpdate,
        dispatch_focused_input::<KeyboardInput>.in_set(InputFocusSystems::Dispatch),
    );
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
    assert_eq!(app.world().resource::<InputFocus>().get(), None);
    app.world_mut().write_message(KeyboardInput {
        key_code: KeyCode::Tab,
        logical_key: Key::Tab,
        state: ButtonState::Pressed,
        text: None,
        repeat: false,
        window,
    });
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
        app.world_mut().write_message(KeyboardInput {
            key_code: KeyCode::ArrowRight,
            logical_key: Key::ArrowRight,
            state: ButtonState::Pressed,
            text: None,
            repeat: false,
            window,
        });
        app.update();
        assert_selected(&app, root, expected);
    }
    assert_eq!(
        app.world().resource::<Changes>().indices,
        vec![(root, 1, true), (root, 2, true)]
    );
}

// Group 的 focus 只改变 border，disabled 优先，theme event 立即刷新且移除 state 后恢复。
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
        DARK_THEME.control_background,
        DARK_THEME.control_border,
    );
    app.world_mut()
        .resource_mut::<InputFocus>()
        .set(root, FocusCause::Navigated);
    app.update();
    assert_colors(
        &app,
        DARK_THEME.control_background,
        DARK_THEME.control_border_active,
    );
    app.world_mut().entity_mut(root).insert(InteractionDisabled);
    app.update();
    assert_colors(
        &app,
        DARK_THEME.control_background_disabled,
        DARK_THEME.control_border_disabled,
    );
    switch_theme(&mut app, ThemeMode::Light);
    assert_colors(
        &app,
        LIGHT_THEME.control_background_disabled,
        LIGHT_THEME.control_border_disabled,
    );
    app.world_mut()
        .entity_mut(root)
        .remove::<InteractionDisabled>();
    app.update();
    assert_colors(
        &app,
        LIGHT_THEME.control_background,
        LIGHT_THEME.control_border_active,
    );
    app.world_mut().resource_mut::<InputFocus>().clear();
    app.update();
    assert_colors(
        &app,
        LIGHT_THEME.control_background,
        LIGHT_THEME.control_border,
    );
}

// 两套 theme 的 hover、checked 与 disabled 组合都只给 indicator 配色，用户内容继承 foreground。
#[test]
fn option_styles_follow_state_and_theme_without_styling_user_nodes() {
    let mut app = app();
    let root = app.world_mut().spawn_scene(group_scene()).unwrap().id();
    app.update();
    let option = app.world().get::<Children>(root).unwrap()[1];
    let children = app.world().get::<Children>(option).unwrap();
    let (indicator, label) = (children[0], children[1]);
    let dot = app.world().get::<Children>(indicator).unwrap()[0];
    for mode in [ThemeMode::Dark, ThemeMode::Light] {
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
                colors.control_border_disabled
            } else if hovered {
                colors.control_border_hovered
            } else if checked {
                colors.control_border_active
            } else {
                colors.control_border
            };
            let fill = if !checked {
                Color::NONE
            } else if disabled {
                colors.foreground_disabled
            } else {
                colors.control_border_active
            };
            let foreground = if disabled {
                colors.foreground_disabled
            } else {
                colors.foreground
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
    switch_theme(&mut app, ThemeMode::Dark);
    assert_eq!(
        app.world().get::<BackgroundColor>(dot).unwrap().0,
        DARK_THEME.control_border_active
    );
}

// 横向和 Grid patch 只改变 layout，保留默认 padding、indicator 及 direct child index 行为。
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

// 调用方预先装配依赖时不得重复注册，新增 Widgetry plugin 保留字体策略与 theme。
#[test]
fn plugin_reuses_dependencies_and_preserves_font_policy() {
    let mut app = scene_app();
    app.add_plugins((
        bevy::ui_widgets::RadioGroupPlugin,
        bevy::input_focus::tab_navigation::TabNavigationPlugin,
        bevy_widgetry_core::WidgetryFocusPlugin,
        bevy_widgetry_core::ThemePlugin,
        bevy_widgetry_core::ForegroundColorPlugin,
    ));
    app.insert_resource(ThemeMode::Light)
        .add_plugins(WidgetryRadioGroupPlugin);
    let root = app.world_mut().spawn_scene(group_scene()).unwrap().id();
    app.update();
    assert_selected(&app, root, 0);
    assert_eq!(*app.world().resource::<ThemeMode>(), ThemeMode::Light);
    let option = app.world().get::<Children>(root).unwrap()[0];
    let label = app.world().get::<Children>(option).unwrap()[1];
    assert_eq!(
        app.world().get::<TextFont>(label).unwrap().font,
        bevy::text::FontSource::Monospace
    );
}

// BSN 组合任意嵌套内容时，option 保留内建 indicator，Group 和 option 都挂载官方行为。
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
