use bevy::{
    app::{App, Plugin, PostUpdate, Update},
    color::Color,
    ecs::{
        change_detection::DetectChanges,
        entity::Entity,
        lifecycle::RemovedComponents,
        observer::On,
        query::{Added, Changed, Has, Or, With, Without},
        schedule::IntoScheduleConfigs,
        system::{Query, Res},
    },
    input_focus::{AcquireFocus, InputFocus, tab_navigation::TabNavigationPlugin},
    picking::hover::Hovered,
    prelude::{Component, Scene, SceneComponent, bsn},
    text::{EditableText, EditableTextSystems, TextColor, TextCursorStyle, TextEdit},
    ui::{BackgroundColor, BorderColor, BorderRadius, InteractionDisabled, Node, UiRect, px},
};
use bevy_widgetry_core::{ColorTheme, ThemeChanged, ThemeMode, ThemePlugin};
use bevy_widgetry_log::widgetry_info;

#[derive(SceneComponent, Default, Clone)]
pub struct WidgetryTextField;

#[derive(SceneComponent, Default, Clone)]
pub struct WidgetryReadOnlyTextField;

#[derive(Component, Default, Clone)]
struct TextFieldBase;

#[derive(Component, Default, Clone)]
struct ReadOnly;

#[derive(Debug, PartialEq)]
struct TextFieldStyle {
    background: Color,
    border: Color,
    foreground: Color,
}

type TextFieldStyleData = (
    Entity,
    &'static Hovered,
    Has<InteractionDisabled>,
    &'static mut BackgroundColor,
    &'static mut BorderColor,
    &'static mut TextColor,
    &'static mut TextCursorStyle,
);

pub struct WidgetryTextFieldPlugin;

type ChangedTextFieldStyleQuery<'w, 's> = Query<
    'w,
    's,
    TextFieldStyleData,
    (
        With<TextFieldBase>,
        Or<(
            Added<TextFieldBase>,
            Changed<Hovered>,
            Added<InteractionDisabled>,
        )>,
    ),
>;

fn block_disabled_text_field_edits(
    mut query: Query<&mut EditableText, (With<TextFieldBase>, With<InteractionDisabled>)>,
) {
    for mut editable_text in &mut query {
        editable_text.pending_edits.clear();
        editable_text.pending_paste = None;
    }
}

fn block_read_only_text_field_edits(
    mut query: Query<&mut EditableText, (With<TextFieldBase>, With<ReadOnly>)>,
) {
    for mut editable_text in &mut query {
        editable_text.pending_edits.retain(|edit| {
            !matches!(
                edit,
                TextEdit::Cut
                    | TextEdit::Paste
                    | TextEdit::Insert(_)
                    | TextEdit::Backspace
                    | TextEdit::BackspaceWord
                    | TextEdit::Delete
                    | TextEdit::DeleteWord
                    | TextEdit::ImeSetCompose { .. }
                    | TextEdit::ImeCommit { .. }
            )
        });
        editable_text.pending_paste = None;
    }
}

// 没有 TabIndex 的 TextField 经 pointer 获得 focus 后，AcquireFocus 仍会传播到 window 并清除 focus。
// 在已取得 focus 的 TextField 停止 propagation，保留本次输入目标。
fn retain_text_field_focus_on_acquire(
    mut event: On<AcquireFocus>,
    text_fields: Query<(), (With<TextFieldBase>, Without<InteractionDisabled>)>,
    focus: Res<InputFocus>,
) {
    if text_fields.contains(event.focused_entity) && focus.get() == Some(event.focused_entity) {
        event.propagate(false);
    }
}

fn resolve_text_field_style(
    colors: &ColorTheme,
    hovered: bool,
    focused: bool,
    disabled: bool,
) -> TextFieldStyle {
    let (background, border) = if disabled {
        (
            colors.control_background_disabled,
            colors.control_border_disabled,
        )
    } else if focused {
        (
            colors.control_background_active,
            colors.control_border_active,
        )
    } else if hovered {
        (
            colors.control_background_hovered,
            colors.control_border_hovered,
        )
    } else {
        (colors.control_background, colors.control_border)
    };

    TextFieldStyle {
        background,
        border,
        foreground: if disabled {
            colors.foreground_disabled
        } else {
            colors.foreground
        },
    }
}

fn apply_text_field_style(
    colors: &ColorTheme,
    focused_entity: Option<Entity>,
    (
        entity,
        hovered,
        disabled,
        mut background,
        mut border,
        mut text,
        mut cursor,
    ): <TextFieldStyleData as bevy::ecs::query::QueryData>::Item<'_, '_>,
) {
    let focused = focused_entity == Some(entity);

    let style = resolve_text_field_style(colors, hovered.0, focused, disabled);

    background.0 = style.background;
    *border = BorderColor::all(style.border);
    text.0 = style.foreground;
    cursor.color = style.foreground;
    cursor.selection_color = colors.text_selection;
    cursor.unfocused_selection_color = colors.text_selection_unfocused;
    cursor.selected_text_color = None;
}

fn update_widgetry_text_field_style_changed(
    mode: Res<ThemeMode>,
    input_focus: Res<InputFocus>,
    mut query: ChangedTextFieldStyleQuery<'_, '_>,
) {
    let focused = input_focus.get();

    for item in &mut query {
        apply_text_field_style(mode.colors(), focused, item);
    }
}

fn update_widgetry_text_field_style_focus_changed(
    mode: Res<ThemeMode>,
    input_focus: Res<InputFocus>,
    mut query: Query<TextFieldStyleData, With<TextFieldBase>>,
) {
    if !input_focus.is_changed() {
        return;
    }

    let focused = input_focus.get();

    for item in &mut query {
        apply_text_field_style(mode.colors(), focused, item);
    }
}

fn update_widgetry_text_field_style_removed(
    mode: Res<ThemeMode>,
    input_focus: Res<InputFocus>,
    mut removed_disabled: RemovedComponents<InteractionDisabled>,
    mut query: Query<TextFieldStyleData, With<TextFieldBase>>,
) {
    let focused = input_focus.get();

    for entity in removed_disabled.read() {
        if let Ok(item) = query.get_mut(entity) {
            apply_text_field_style(mode.colors(), focused, item);
        }
    }
}

fn refresh_text_field_theme(
    event: On<ThemeChanged>,
    input_focus: Res<InputFocus>,
    mut query: Query<TextFieldStyleData, With<TextFieldBase>>,
) {
    let focused = input_focus.get();

    for item in &mut query {
        apply_text_field_style(event.mode.colors(), focused, item);
    }
}

impl WidgetryTextField {
    fn scene() -> impl Scene {
        text_field_base_scene()
    }
}

impl WidgetryReadOnlyTextField {
    fn scene() -> impl Scene {
        bsn! { text_field_base_scene() ReadOnly }
    }
}

fn text_field_base_scene() -> impl Scene {
    bsn! {
        TextFieldBase
        EditableText
        Hovered(false)
        Node {
            padding: UiRect::axes(px(10), px(6)),
            border: UiRect::all(px(1)),
            border_radius: BorderRadius::all(px(4)),
        }
        BackgroundColor
        BorderColor
        TextCursorStyle
    }
}

impl Plugin for WidgetryTextFieldPlugin {
    fn build(&self, app: &mut App) {
        if !app.is_plugin_added::<bevy_widgetry_core::pointer::WidgetryPointerPlugin>() {
            app.add_plugins(bevy_widgetry_core::pointer::WidgetryPointerPlugin);
        }
        if !app.is_plugin_added::<TabNavigationPlugin>() {
            app.add_plugins(TabNavigationPlugin);
        }
        if !app.is_plugin_added::<ThemePlugin>() {
            app.add_plugins(ThemePlugin);
        }

        app.add_observer(refresh_text_field_theme);
        app.add_observer(retain_text_field_focus_on_acquire);
        app.add_systems(
            PostUpdate,
            (
                block_disabled_text_field_edits,
                block_read_only_text_field_edits,
            )
                .before(EditableTextSystems),
        );

        app.add_systems(
            Update,
            (
                update_widgetry_text_field_style_changed,
                update_widgetry_text_field_style_focus_changed,
                update_widgetry_text_field_style_removed,
            ),
        );
        widgetry_info!("WidgetryTextFieldPlugin 注册完成");
    }
}

// 测试断言需要在 contract 不满足时立即失败。
// 生产代码的 panic lint 会拒绝这些表达式，因此仅在本测试 scope 允许所列 lint。
#[cfg(test)]
#[allow(clippy::disallowed_macros)]
mod tests {
    use super::*;
    use bevy::scene::WorldSceneExt;
    use bevy_widgetry_test_utils::scene_app;

    #[test]
    fn read_only_marker_is_exclusive_to_read_only_scene() {
        let mut app = scene_app();
        app.add_plugins(WidgetryTextFieldPlugin);
        let normal = app
            .world_mut()
            .spawn_scene(bsn! { @WidgetryTextField })
            .unwrap()
            .id();
        let read_only = app
            .world_mut()
            .spawn_scene(bsn! { @WidgetryReadOnlyTextField })
            .unwrap()
            .id();
        assert!(app.world().get::<TextFieldBase>(normal).is_some());
        assert!(app.world().get::<ReadOnly>(normal).is_none());
        assert!(app.world().get::<TextFieldBase>(read_only).is_some());
        assert!(app.world().get::<ReadOnly>(read_only).is_some());
        app.world_mut()
            .get_mut::<EditableText>(normal)
            .unwrap()
            .queue_edit(TextEdit::Insert("X".into()));
        app.update();
        assert!(
            app.world()
                .get::<EditableText>(normal)
                .unwrap()
                .pending_edits
                .contains(&TextEdit::Insert("X".into()))
        );
    }
}
