#![cfg(test)]

use bevy::prelude::*;
use bevy::ui::{Node, Val};
use bevy::{
    app::App,
    color::Color,
    ecs::entity::Entity,
    input_focus::{FocusCause, InputFocus},
    picking::hover::Hovered,
    text::{TextColor, TextCursorStyle},
    ui::{BackgroundColor, BorderColor, InteractionDisabled},
};
use bevy::{
    input_focus::tab_navigation::TabIndex,
    text::{EditableText, FontSource, LineHeight, TextLayout},
    ui::{BorderRadius, UiRect},
};
use bevy_widgetry_core::{DARK_THEME, LIGHT_THEME, ThemeMode};
use bevy_widgetry_core::{WidgetryFocusPlugin, WidgetryFontPlugin};
use bevy_widgetry_test_utils::{scene_app, switch_theme};
use bevy_widgetry_text_field::{
    WidgetryReadOnlyTextField, WidgetryTextField, WidgetryTextFieldPlugin,
};
use rstest::fixture;

// BSN 外壳不覆盖官方 typesetting 默认值、调用方 multiline 配置，也不隐式安装字体或输入 plugin。
#[test]
fn scene_preserves_official_configuration_and_app_font_policy() {
    let mut app = App::new();
    app.add_plugins((
        MinimalPlugins,
        AssetPlugin::default(),
        bevy::scene::ScenePlugin,
    ));
    app.add_plugins(WidgetryTextFieldPlugin);
    assert!(app.is_plugin_added::<WidgetryFocusPlugin>());
    assert!(!app.is_plugin_added::<WidgetryFontPlugin>());
    assert!(!app.is_plugin_added::<bevy::ui_widgets::EditableTextInputPlugin>());
    let entity = app.world_mut().spawn_scene(bsn! {
        @WidgetryTextField
        template_value(EditableText::new("First\nSecond"))
        EditableText { visible_lines: {Some(4.0)}, allow_newlines: true, visible_width: {Some(20.0)}, max_characters: {Some(80)} }
    }).unwrap().id();
    app.update();
    let editable = app.world().get::<EditableText>(entity).unwrap();
    assert_eq!(editable.visible_lines, Some(4.0));
    assert!(editable.allow_newlines);
    assert_eq!(editable.visible_width, Some(20.0));
    assert_eq!(editable.max_characters, Some(80));
    assert_eq!(
        editable.value().into_iter().collect::<String>(),
        "First\nSecond"
    );
    assert!(app.world().get::<WidgetryTextField>(entity).is_some());
    assert!(app.world().get::<TabIndex>(entity).is_none());
    assert!(app.world().get::<Children>(entity).is_none());
    assert_eq!(
        app.world().get::<TextLayout>(entity).unwrap().linebreak,
        TextLayout::default().linebreak
    );
    assert_eq!(
        *app.world().get::<LineHeight>(entity).unwrap(),
        LineHeight::default()
    );
    assert_eq!(
        app.world().get::<TextFont>(entity).unwrap().font,
        FontSource::default()
    );
}

// 默认 layout 允许官方 visible_lines 决定高度，不再固定 Widget 宽高。
#[test]
fn default_layout_does_not_fix_width_or_height() {
    let mut app = app();
    let entity = app
        .world_mut()
        .spawn_scene(bsn! { @WidgetryTextField })
        .unwrap()
        .id();
    let node = app.world().get::<Node>(entity).unwrap();
    assert_eq!(node.width, Val::Auto);
    assert_eq!(node.height, Val::Auto);
    assert_eq!(node.min_height, Val::Auto);
    assert_eq!(node.padding, UiRect::axes(px(10), px(6)));
    assert_eq!(node.border, UiRect::all(px(1)));
    assert_eq!(node.border_radius, BorderRadius::all(px(4)));
}

/// 复用 headless Scene 设施，装配待测 theme TextField。
#[fixture]
fn app() -> App {
    let mut app = scene_app();

    app.add_plugins(WidgetryTextFieldPlugin);

    app
}

/// 检查同一 Widget 的背景、border、foreground 与 cursor 颜色一致。
fn assert_style(app: &App, entity: Entity, background: Color, border: Color, foreground: Color) {
    assert_eq!(
        app.world().get::<BackgroundColor>(entity).unwrap().0,
        background
    );

    assert_eq!(
        *app.world().get::<BorderColor>(entity).unwrap(),
        BorderColor::all(border)
    );

    assert_eq!(app.world().get::<TextColor>(entity).unwrap().0, foreground);

    assert_eq!(
        app.world().get::<TextCursorStyle>(entity).unwrap().color,
        foreground
    );
}

/// 模拟调用方程序化设置 focus。
fn focus(app: &mut App, entity: Entity) {
    app.world_mut()
        .resource_mut::<InputFocus>()
        .set(entity, FocusCause::Navigated);
}

/// 模拟调用方程序化清空 focus。
fn clear_focus(app: &mut App) {
    app.world_mut().resource_mut::<InputFocus>().clear();
}

// 初始化没有 focus 或 hover 的 TextField，验证完整默认颜色及 cursor style。
#[test]
fn spawned_text_field_uses_normal_style() {
    let mut app = app();

    let entity = app
        .world_mut()
        .spawn_scene(bsn! { @WidgetryTextField })
        .unwrap()
        .id();

    app.update();

    assert_style(
        &app,
        entity,
        DARK_THEME.control_background,
        DARK_THEME.control_border,
        DARK_THEME.foreground,
    );
}

// 在同一 TextField 上切换 hover 和 focus，验证 focus 优先且失去 focus 后正确回退。
#[test]
fn hover_and_focus_follow_expected_priority() {
    let mut app = app();

    let entity = app
        .world_mut()
        .spawn_scene(bsn! { @WidgetryTextField })
        .unwrap()
        .id();

    app.update();

    app.world_mut().entity_mut(entity).insert(Hovered(true));

    app.update();

    assert_style(
        &app,
        entity,
        DARK_THEME.control_background_hovered,
        DARK_THEME.control_border_hovered,
        DARK_THEME.foreground,
    );

    focus(&mut app, entity);

    app.update();

    assert_style(
        &app,
        entity,
        DARK_THEME.control_background_active,
        DARK_THEME.control_border_active,
        DARK_THEME.foreground,
    );

    clear_focus(&mut app);

    app.update();

    assert_style(
        &app,
        entity,
        DARK_THEME.control_background_hovered,
        DARK_THEME.control_border_hovered,
        DARK_THEME.foreground,
    );
}

// 保留 focus 时禁用再启用 TextField，验证 disabled 覆盖后能够恢复 focus style。
#[test]
fn disabled_has_priority_and_removal_restores_focus() {
    let mut app = app();

    let entity = app
        .world_mut()
        .spawn_scene(bsn! { @WidgetryTextField Hovered(true) })
        .unwrap()
        .id();

    focus(&mut app, entity);

    app.world_mut()
        .entity_mut(entity)
        .insert(InteractionDisabled);

    app.update();

    assert_style(
        &app,
        entity,
        DARK_THEME.control_background_disabled,
        DARK_THEME.control_border_disabled,
        DARK_THEME.foreground_disabled,
    );

    app.world_mut()
        .entity_mut(entity)
        .remove::<InteractionDisabled>();

    app.update();

    // Focus 仍然存在，因此恢复 Focused，而不是 Hovered。
    assert_style(
        &app,
        entity,
        DARK_THEME.control_background_active,
        DARK_THEME.control_border_active,
        DARK_THEME.foreground,
    );
}

// 没有 focus 时解除 disabled，必须按当前 hover state 恢复 Hovered 或 Normal。
#[test]
fn removing_disabled_without_focus_restores_hover_or_normal() {
    for hovered in [false, true] {
        let mut app = app();
        let entity = app
            .world_mut()
            .spawn_scene(bsn! {
                @WidgetryTextField Hovered({hovered}) InteractionDisabled
            })
            .unwrap()
            .id();
        app.update();
        app.world_mut()
            .entity_mut(entity)
            .remove::<InteractionDisabled>();
        app.update();
        assert_style(
            &app,
            entity,
            if hovered {
                DARK_THEME.control_background_hovered
            } else {
                DARK_THEME.control_background
            },
            if hovered {
                DARK_THEME.control_border_hovered
            } else {
                DARK_THEME.control_border
            },
            DARK_THEME.foreground,
        );
    }
}

// 多种 TextField state 下切换 theme，验证配色变化不破坏文本和 interaction state。
#[test]
fn theme_switch_preserves_current_widget_states() {
    let mut app = app();

    let focused = app
        .world_mut()
        .spawn_scene(bsn! { @WidgetryTextField })
        .unwrap()
        .id();

    let disabled = app
        .world_mut()
        .spawn_scene(bsn! { @WidgetryTextField InteractionDisabled })
        .unwrap()
        .id();

    focus(&mut app, focused);

    app.update();

    switch_theme(&mut app, ThemeMode::Light);

    assert_style(
        &app,
        focused,
        LIGHT_THEME.control_background_active,
        LIGHT_THEME.control_border_active,
        LIGHT_THEME.foreground,
    );

    assert_style(
        &app,
        disabled,
        LIGHT_THEME.control_background_disabled,
        LIGHT_THEME.control_border_disabled,
        LIGHT_THEME.foreground_disabled,
    );

    // state 本身不能因为换 Theme 被破坏。
    assert_eq!(app.world().resource::<InputFocus>().get(), Some(focused));

    assert!(app.world().get::<InteractionDisabled>(disabled).is_some());
}

// 切换 theme 并检查 selection 与失去 focus 后的 selection 颜色，验证两种 selection state 均更新。
#[test]
fn selection_colors_follow_theme() {
    let mut app = app();

    let entity = app
        .world_mut()
        .spawn_scene(bsn! { @WidgetryTextField })
        .unwrap()
        .id();
    app.update();

    let cursor = app.world().get::<TextCursorStyle>(entity).unwrap();

    assert_eq!(cursor.selected_text_color, None);
    assert_eq!(cursor.selection_color, DARK_THEME.text_selection);
    assert_eq!(
        cursor.unfocused_selection_color,
        DARK_THEME.text_selection_unfocused
    );

    switch_theme(&mut app, ThemeMode::Light);

    let cursor = app.world().get::<TextCursorStyle>(entity).unwrap();

    assert_eq!(cursor.selected_text_color, None);
    assert_eq!(cursor.selection_color, LIGHT_THEME.text_selection);
    assert_eq!(
        cursor.unfocused_selection_color,
        LIGHT_THEME.text_selection_unfocused
    );
}

// ReadOnly 与普通 TextField 在各 interaction state 和 theme 下共享完整 style。
#[test]
fn read_only_style_matches_text_field_in_each_state() {
    let mut app = app();
    let normal = app
        .world_mut()
        .spawn_scene(bsn! { @WidgetryReadOnlyTextField })
        .unwrap()
        .id();
    let hovered = app
        .world_mut()
        .spawn_scene(bsn! { @WidgetryReadOnlyTextField Hovered(true) })
        .unwrap()
        .id();
    let focused = app
        .world_mut()
        .spawn_scene(bsn! { @WidgetryReadOnlyTextField })
        .unwrap()
        .id();
    let disabled = app
        .world_mut()
        .spawn_scene(bsn! { @WidgetryReadOnlyTextField Hovered(true) InteractionDisabled })
        .unwrap()
        .id();
    focus(&mut app, focused);
    app.update();
    for (colors, mode) in [
        (DARK_THEME, ThemeMode::Dark),
        (LIGHT_THEME, ThemeMode::Light),
    ] {
        switch_theme(&mut app, mode);
        assert_style(
            &app,
            normal,
            colors.control_background,
            colors.control_border,
            colors.foreground,
        );
        assert_style(
            &app,
            hovered,
            colors.control_background_hovered,
            colors.control_border_hovered,
            colors.foreground,
        );
        assert_style(
            &app,
            focused,
            colors.control_background_active,
            colors.control_border_active,
            colors.foreground,
        );
        assert_style(
            &app,
            disabled,
            colors.control_background_disabled,
            colors.control_border_disabled,
            colors.foreground_disabled,
        );
        for entity in [normal, hovered, focused, disabled] {
            let cursor = app.world().get::<TextCursorStyle>(entity).unwrap();
            assert_eq!(cursor.selection_color, colors.text_selection);
            assert_eq!(
                cursor.unfocused_selection_color,
                colors.text_selection_unfocused
            );
            assert_eq!(cursor.selected_text_color, None);
        }
    }
}

// ReadOnly 沿用官方 EditableText 配置及单 entity layout，允许调用方在 BSN 中 patch。
#[test]
fn read_only_scene_accepts_official_configuration() {
    let mut app = app();
    let entity = app.world_mut().spawn_scene(bsn! {
        @WidgetryReadOnlyTextField
        template_value(EditableText::new("First\nSecond"))
        EditableText { visible_lines: {Some(4.0)}, allow_newlines: true, visible_width: {Some(20.0)}, max_characters: {Some(80)} }
    }).unwrap().id();
    app.update();
    let editable = app.world().get::<EditableText>(entity).unwrap();
    assert_eq!(editable.value().to_string(), "First\nSecond");
    assert_eq!(editable.visible_lines, Some(4.0));
    assert!(editable.allow_newlines);
    assert_eq!(editable.visible_width, Some(20.0));
    assert_eq!(editable.max_characters, Some(80));
    assert!(app.world().get::<Children>(entity).is_none());
    let node = app.world().get::<Node>(entity).unwrap();
    assert_eq!(node.padding, UiRect::axes(px(10), px(6)));
    assert_eq!(node.border, UiRect::all(px(1)));
    assert_eq!(node.border_radius, BorderRadius::all(px(4)));
}
