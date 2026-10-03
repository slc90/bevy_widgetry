//! Coverage Map：本文件负责 Scene/layout/font policy、完整 style 与 theme 保留文本/selection。
//! disabled.rs 负责全部编辑阻止及恢复。
//! read_only.rs 负责 mutation 分界、selection 消费与 focus。
//! State：构造类型 normal/readonly、enabled、focus/hover、文本/选区。
//! stimuli 为输入、程序化内容、theme/state。
//! Invariants：theme/style 不改变文本、选区或 entity identity。
//! queue 只作为特定过滤阶段证据。

// 测试断言需要在 contract 不满足时立即失败。
// 生产代码的 panic lint 会拒绝这些表达式，因此仅在本测试 scope 允许所列 lint。
#![allow(clippy::disallowed_macros, clippy::expect_used, clippy::unwrap_used)]
#![cfg(test)]

mod support;

use bevy::prelude::*;
use bevy::ui::{Node, Val};
use bevy::{
    app::App,
    color::Color,
    ecs::entity::Entity,
    input_focus::{FocusCause, InputFocus},
    picking::hover::Hovered,
    text::{TextColor, TextCursorStyle, TextEdit},
    ui::{BackgroundColor, BorderColor, InteractionDisabled},
};
use bevy::{
    input_focus::tab_navigation::TabIndex,
    text::{EditableText, FontSource, LineHeight, TextLayout},
    ui::{BorderRadius, UiRect},
};
use bevy_widgetry_core::WidgetryFontPlugin;
use bevy_widgetry_core::{DARK_THEME, LIGHT_THEME, ThemeMode};
use bevy_widgetry_test_utils::{scene_app, switch_theme};
use bevy_widgetry_text_field::{
    WidgetryReadOnlyTextField, WidgetryTextField, WidgetryTextFieldPlugin,
};
use rstest::fixture;
use support::editing_app;

#[test]
fn scene_preserves_official_configuration_and_app_font_policy() {
    let mut app = App::new();
    app.add_plugins((
        MinimalPlugins,
        AssetPlugin::default(),
        bevy::scene::ScenePlugin,
        bevy::input_focus::InputFocusPlugin,
    ));
    app.add_plugins(WidgetryTextFieldPlugin);
    assert!(app.is_plugin_added::<bevy::input_focus::tab_navigation::TabNavigationPlugin>());
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

#[fixture]
fn app() -> App {
    let mut app = scene_app();

    app.add_plugins(WidgetryTextFieldPlugin);

    app
}

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

fn focus(app: &mut App, entity: Entity) {
    app.world_mut()
        .resource_mut::<InputFocus>()
        .set(entity, FocusCause::Navigated);
}

fn clear_focus(app: &mut App) {
    app.world_mut().resource_mut::<InputFocus>().clear();
}

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

    assert_style(
        &app,
        entity,
        DARK_THEME.control_background_active,
        DARK_THEME.control_border_active,
        DARK_THEME.foreground,
    );
}

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

    assert_eq!(app.world().resource::<InputFocus>().get(), Some(focused));

    assert!(app.world().get::<InteractionDisabled>(disabled).is_some());
}

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

#[test]
fn theme_and_state_preserve_consumed_text_and_selection() {
    let mut app = editing_app();
    app.add_plugins(WidgetryTextFieldPlugin);
    let normal = app
        .world_mut()
        .spawn_scene(bsn! { @WidgetryTextField template_value(EditableText::new("content")) })
        .unwrap()
        .id();
    let read_only = app
        .world_mut()
        .spawn_scene(
            bsn! { @WidgetryReadOnlyTextField template_value(EditableText::new("content")) },
        )
        .unwrap()
        .id();
    app.update();
    for root in [normal, read_only] {
        app.world_mut()
            .get_mut::<EditableText>(root)
            .unwrap()
            .queue_edit(TextEdit::SelectAll);
    }
    app.update();
    for mode in [ThemeMode::Light, ThemeMode::Dark] {
        switch_theme(&mut app, mode);
        for disabled in [false, true, false] {
            for root in [normal, read_only] {
                if disabled {
                    app.world_mut().entity_mut(root).insert(InteractionDisabled);
                } else {
                    app.world_mut()
                        .entity_mut(root)
                        .remove::<InteractionDisabled>();
                }
            }
            app.update();
            let c = mode.colors();
            for root in [normal, read_only] {
                let edit = app.world().get::<EditableText>(root).unwrap();
                assert_eq!(edit.value().to_string(), "content");
                assert_eq!(edit.editor().raw_selection().text_range(), 0..7);
                assert_style(
                    &app,
                    root,
                    if disabled {
                        c.control_background_disabled
                    } else {
                        c.control_background
                    },
                    if disabled {
                        c.control_border_disabled
                    } else {
                        c.control_border
                    },
                    if disabled {
                        c.foreground_disabled
                    } else {
                        c.foreground
                    },
                );
                let cursor = app.world().get::<TextCursorStyle>(root).unwrap();
                assert_eq!(cursor.selection_color, c.text_selection);
                assert_eq!(cursor.unfocused_selection_color, c.text_selection_unfocused);
            }
            assert!(app.world().get::<WidgetryTextField>(normal).is_some());
            assert!(
                app.world()
                    .get::<WidgetryReadOnlyTextField>(read_only)
                    .is_some()
            );
        }
    }
}
