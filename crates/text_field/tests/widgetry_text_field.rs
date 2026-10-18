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
use bevy_widgetry_theme::{WIDGETRY_DARK_THEME, WIDGETRY_LIGHT_THEME, WidgetryThemeMode};

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
    assert!(!app.is_plugin_added::<bevy::ui_widgets::TextInputPlugin>());
    let entity = app.world_mut().spawn_scene(bsn! {
        @WidgetryTextField { @value: {"First\nSecond".to_owned()}, @visible_lines: {Some(4.0)}, @allow_newlines: true, @visible_width: {Some(20.0)} }
    }).unwrap().id();
    app.update();
    let editable = support::editor(app.world(), entity).unwrap();
    assert_eq!(editable.visible_lines, Some(4.0));
    assert!(editable.allow_newlines);
    assert_eq!(editable.visible_width, Some(20.0));
    assert_eq!(editable.max_characters, None);
    assert_eq!(
        editable.value().into_iter().collect::<String>(),
        "First\nSecond"
    );
    assert!(app.world().get::<WidgetryTextField>(entity).is_some());
    assert!(app.world().get::<TabIndex>(entity).is_none());
    assert_eq!(app.world().get::<Children>(entity).unwrap().len(), 1);
    assert_eq!(
        app.world()
            .get::<TextLayout>(support::editor_entity(app.world(), entity))
            .unwrap()
            .linebreak,
        TextLayout::default().linebreak
    );
    assert_eq!(
        *app.world()
            .get::<LineHeight>(support::editor_entity(app.world(), entity))
            .unwrap(),
        LineHeight::default()
    );
    assert_eq!(
        app.world()
            .get::<TextFont>(support::editor_entity(app.world(), entity))
            .unwrap()
            .font,
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
    let node = app
        .world()
        .get::<Node>(support::frame_entity(app.world(), entity))
        .unwrap();
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
        app.world()
            .get::<BackgroundColor>(support::frame_entity(app.world(), entity))
            .unwrap()
            .0,
        background
    );

    assert_eq!(
        *app.world()
            .get::<BorderColor>(support::frame_entity(app.world(), entity))
            .unwrap(),
        BorderColor::all(border)
    );

    assert_eq!(
        app.world()
            .get::<TextColor>(support::editor_entity(app.world(), entity))
            .unwrap()
            .0,
        foreground
    );

    assert_eq!(
        app.world()
            .get::<TextCursorStyle>(support::editor_entity(app.world(), entity))
            .unwrap()
            .color,
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
        WIDGETRY_DARK_THEME.text_field.editable.normal.background,
        WIDGETRY_DARK_THEME.text_field.editable.normal.border,
        WIDGETRY_DARK_THEME.text_field.editable.normal.foreground,
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
        WIDGETRY_DARK_THEME.text_field.editable.hovered.background,
        WIDGETRY_DARK_THEME.text_field.editable.hovered.border,
        WIDGETRY_DARK_THEME.text_field.editable.normal.foreground,
    );

    focus(&mut app, entity);

    app.update();

    assert_style(
        &app,
        entity,
        WIDGETRY_DARK_THEME.text_field.editable.focused.background,
        WIDGETRY_DARK_THEME.text_field.editable.focused.border,
        WIDGETRY_DARK_THEME.text_field.editable.normal.foreground,
    );

    clear_focus(&mut app);

    app.update();

    assert_style(
        &app,
        entity,
        WIDGETRY_DARK_THEME.text_field.editable.hovered.background,
        WIDGETRY_DARK_THEME.text_field.editable.hovered.border,
        WIDGETRY_DARK_THEME.text_field.editable.normal.foreground,
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
        WIDGETRY_DARK_THEME.text_field.editable.disabled.background,
        WIDGETRY_DARK_THEME.text_field.editable.disabled.border,
        WIDGETRY_DARK_THEME.text_field.editable.disabled.foreground,
    );

    app.world_mut()
        .entity_mut(entity)
        .remove::<InteractionDisabled>();

    app.update();

    assert_style(
        &app,
        entity,
        WIDGETRY_DARK_THEME.text_field.editable.focused.background,
        WIDGETRY_DARK_THEME.text_field.editable.focused.border,
        WIDGETRY_DARK_THEME.text_field.editable.normal.foreground,
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
                WIDGETRY_DARK_THEME.text_field.editable.hovered.background
            } else {
                WIDGETRY_DARK_THEME.text_field.editable.normal.background
            },
            if hovered {
                WIDGETRY_DARK_THEME.text_field.editable.hovered.border
            } else {
                WIDGETRY_DARK_THEME.text_field.editable.normal.border
            },
            WIDGETRY_DARK_THEME.text_field.editable.normal.foreground,
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

    switch_theme(&mut app, WidgetryThemeMode::Light);

    assert_style(
        &app,
        focused,
        WIDGETRY_LIGHT_THEME.text_field.editable.focused.background,
        WIDGETRY_LIGHT_THEME.text_field.editable.focused.border,
        WIDGETRY_LIGHT_THEME.text_field.editable.normal.foreground,
    );

    assert_style(
        &app,
        disabled,
        WIDGETRY_LIGHT_THEME.text_field.editable.disabled.background,
        WIDGETRY_LIGHT_THEME.text_field.editable.disabled.border,
        WIDGETRY_LIGHT_THEME.text_field.editable.disabled.foreground,
    );

    assert_eq!(
        app.world().resource::<InputFocus>().get(),
        Some(support::editor_entity(app.world(), focused))
    );

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

    let cursor = app
        .world()
        .get::<TextCursorStyle>(support::editor_entity(app.world(), entity))
        .unwrap();

    assert_eq!(
        cursor.selected_text_color,
        Some(
            WIDGETRY_DARK_THEME
                .text_field
                .editable
                .normal
                .selection_foreground
        )
    );
    assert_eq!(
        cursor.selection_color,
        WIDGETRY_DARK_THEME
            .text_field
            .editable
            .normal
            .selection_background
    );
    assert_eq!(
        cursor.unfocused_selection_color,
        WIDGETRY_DARK_THEME
            .text_field
            .editable
            .normal
            .unfocused_selection_background
    );

    switch_theme(&mut app, WidgetryThemeMode::Light);

    let cursor = app
        .world()
        .get::<TextCursorStyle>(support::editor_entity(app.world(), entity))
        .unwrap();

    assert_eq!(cursor.selected_text_color, Some(cursor.color));
    assert_eq!(
        cursor.selection_color,
        WIDGETRY_LIGHT_THEME
            .text_field
            .editable
            .normal
            .selection_background
    );
    assert_eq!(
        cursor.unfocused_selection_color,
        WIDGETRY_LIGHT_THEME
            .text_field
            .editable
            .normal
            .unfocused_selection_background
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
        (WIDGETRY_DARK_THEME, WidgetryThemeMode::Dark),
        (WIDGETRY_LIGHT_THEME, WidgetryThemeMode::Light),
    ] {
        switch_theme(&mut app, mode);
        assert_style(
            &app,
            normal,
            colors.text_field.editable.normal.background,
            colors.text_field.editable.normal.border,
            colors.text_field.editable.normal.foreground,
        );
        assert_style(
            &app,
            hovered,
            colors.text_field.editable.hovered.background,
            colors.text_field.editable.hovered.border,
            colors.text_field.editable.normal.foreground,
        );
        assert_style(
            &app,
            focused,
            colors.text_field.editable.focused.background,
            colors.text_field.editable.focused.border,
            colors.text_field.editable.normal.foreground,
        );
        assert_style(
            &app,
            disabled,
            colors.text_field.editable.disabled.background,
            colors.text_field.editable.disabled.border,
            colors.text_field.editable.disabled.foreground,
        );
        for (entity, state) in [
            (normal, colors.text_field.read_only.normal),
            (hovered, colors.text_field.read_only.hovered),
            (focused, colors.text_field.read_only.focused),
            (disabled, colors.text_field.read_only.disabled),
        ] {
            let cursor = app
                .world()
                .get::<TextCursorStyle>(support::editor_entity(app.world(), entity))
                .unwrap();
            assert_eq!(cursor.selection_color, state.selection_background);
            assert_eq!(
                cursor.unfocused_selection_color,
                state.unfocused_selection_background
            );
            assert_eq!(cursor.selected_text_color, Some(state.selection_foreground));
        }
    }
}

#[test]
fn read_only_scene_accepts_official_configuration() {
    let mut app = app();
    let entity = app.world_mut().spawn_scene(bsn! {
        @WidgetryReadOnlyTextField { @value: {"First\nSecond".to_owned()}, @visible_lines: {Some(4.0)}, @allow_newlines: true, @visible_width: {Some(20.0)} }
    }).unwrap().id();
    app.update();
    let editable = support::editor(app.world(), entity).unwrap();
    assert_eq!(editable.value().to_string(), "First\nSecond");
    assert_eq!(editable.visible_lines, Some(4.0));
    assert!(editable.allow_newlines);
    assert_eq!(editable.visible_width, Some(20.0));
    assert_eq!(editable.max_characters, None);
    assert_eq!(app.world().get::<Children>(entity).unwrap().len(), 1);
    let node = app
        .world()
        .get::<Node>(support::frame_entity(app.world(), entity))
        .unwrap();
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
        .spawn_scene(bsn! { @WidgetryTextField { @value: {("content").to_owned()} } })
        .unwrap()
        .id();
    let read_only = app
        .world_mut()
        .spawn_scene(bsn! { @WidgetryReadOnlyTextField { @value: {("content").to_owned()} } })
        .unwrap()
        .id();
    app.update();
    for root in [normal, read_only] {
        {
            let editor = support::editor_entity(app.world(), root);
            app.world_mut().get_mut::<EditableText>(editor)
        }
        .unwrap()
        .queue_edit(TextEdit::SelectAll);
    }
    app.update();
    for mode in [WidgetryThemeMode::Light, WidgetryThemeMode::Dark] {
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
                let edit = support::editor(app.world(), root).unwrap();
                assert_eq!(edit.value().to_string(), "content");
                assert_eq!(edit.editor().raw_selection().text_range(), 0..7);
                assert_style(
                    &app,
                    root,
                    if disabled {
                        c.text_field.editable.disabled.background
                    } else {
                        c.text_field.editable.normal.background
                    },
                    if disabled {
                        c.text_field.editable.disabled.border
                    } else {
                        c.text_field.editable.normal.border
                    },
                    if disabled {
                        c.text_field.editable.disabled.foreground
                    } else {
                        c.text_field.editable.normal.foreground
                    },
                );
                let cursor = app
                    .world()
                    .get::<TextCursorStyle>(support::editor_entity(app.world(), root))
                    .unwrap();
                assert_eq!(
                    cursor.selection_color,
                    c.text_field.editable.normal.selection_background
                );
                assert_eq!(
                    cursor.unfocused_selection_color,
                    c.text_field.editable.normal.unfocused_selection_background
                );
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
