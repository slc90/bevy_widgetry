use bevy_widgetry_theme::{
    WidgetryTheme, WidgetryThemeChanged, WidgetryThemeMode, WidgetryThemePlugin,
};

use bevy::{
    app::{App, Plugin, PostUpdate},
    ecs::{
        change_detection::{DetectChanges, DetectChangesMut},
        entity::Entity,
        lifecycle::RemovedComponents,
        observer::On,
        query::{Added, Changed, Has, Or, With, Without},
        schedule::IntoScheduleConfigs,
        system::{Query, Res},
    },
    input_focus::{AcquireFocus, InputFocus, tab_navigation::TabNavigationPlugin},
    picking::hover::Hovered,
    prelude::{Component, Scene, SceneComponent, bsn, template},
    text::{EditableText, EditableTextSystems, TextColor, TextCursorStyle, TextEdit},
    ui::{BackgroundColor, BorderColor, BorderRadius, InteractionDisabled, Node, UiRect, px},
};
use bevy_widgetry_log::widgetry_info;

#[derive(SceneComponent, Default, Clone)]
#[scene(crate::WidgetryTextFieldProps)]
#[require(crate::colors::ColorState)]
pub struct WidgetryTextField;

#[derive(SceneComponent, Default, Clone)]
#[scene(crate::WidgetryTextFieldProps)]
#[require(crate::colors::ColorState)]
pub struct WidgetryReadOnlyTextField;

#[derive(Component, Default, Clone)]
struct TextFieldBase;

#[derive(Component, Default, Clone)]
struct ReadOnly;

type TextFieldStyleData = (
    Entity,
    &'static crate::colors::ColorState,
    Has<ReadOnly>,
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
        Without<bevy_widgetry_core::color::WidgetryStyleOwner<WidgetryTextField>>,
        Or<(
            Added<TextFieldBase>,
            Changed<Hovered>,
            Changed<crate::colors::ColorState>,
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

fn apply_text_field_style(
    colors: &WidgetryTheme,
    focused_entity: Option<Entity>,
    (
        entity,
        overrides,
        read_only,
        hovered,
        disabled,
        background,
        border,
        text,
        cursor,
    ): <TextFieldStyleData as bevy::ecs::query::QueryData>::Item<'_, '_>,
) {
    let focused = focused_entity == Some(entity);

    let colors = overrides.0.resolve(&colors.text_field);
    let colors = if read_only {
        colors.read_only
    } else {
        colors.editable
    };
    let state = if disabled {
        colors.disabled
    } else if focused {
        colors.focused
    } else if hovered.0 {
        colors.hovered
    } else {
        colors.normal
    };
    apply_outputs(state, (background, border, text, cursor));
}

fn apply_outputs(
    state: bevy_widgetry_theme::WidgetryTextFieldStateColors,
    (mut background, mut border, mut text, mut cursor): (
        bevy::prelude::Mut<BackgroundColor>,
        bevy::prelude::Mut<BorderColor>,
        bevy::prelude::Mut<TextColor>,
        bevy::prelude::Mut<TextCursorStyle>,
    ),
) {
    background.set_if_neq(BackgroundColor(state.background));
    border.set_if_neq(BorderColor::all(state.border));
    text.set_if_neq(TextColor(state.foreground));
    if cursor.color != state.caret
        || cursor.selection_color != state.selection_background
        || cursor.unfocused_selection_color != state.unfocused_selection_background
        || cursor.selected_text_color != Some(state.selection_foreground)
    {
        cursor.color = state.caret;
        cursor.selection_color = state.selection_background;
        cursor.unfocused_selection_color = state.unfocused_selection_background;
        cursor.selected_text_color = Some(state.selection_foreground);
    }
}

fn update_widgetry_text_field_style_changed(
    mode: Res<WidgetryThemeMode>,
    input_focus: Res<InputFocus>,
    mut query: ChangedTextFieldStyleQuery<'_, '_>,
) {
    let focused = input_focus.get();

    for item in &mut query {
        apply_text_field_style(mode.colors(), focused, item);
    }
}

fn update_widgetry_text_field_style_focus_changed(
    mode: Res<WidgetryThemeMode>,
    input_focus: Res<InputFocus>,
    mut query: Query<
        TextFieldStyleData,
        (
            With<TextFieldBase>,
            Without<bevy_widgetry_core::color::WidgetryStyleOwner<WidgetryTextField>>,
        ),
    >,
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
    mode: Res<WidgetryThemeMode>,
    input_focus: Res<InputFocus>,
    mut removed_disabled: RemovedComponents<InteractionDisabled>,
    mut query: Query<
        TextFieldStyleData,
        (
            With<TextFieldBase>,
            Without<bevy_widgetry_core::color::WidgetryStyleOwner<WidgetryTextField>>,
        ),
    >,
) {
    let focused = input_focus.get();

    for entity in removed_disabled.read() {
        if let Ok(item) = query.get_mut(entity) {
            apply_text_field_style(mode.colors(), focused, item);
        }
    }
}

fn refresh_text_field_theme(
    _event: On<WidgetryThemeChanged>,
    mut commands: bevy::prelude::Commands,
) {
    commands.queue(
        |world: &mut bevy::prelude::World| -> Result<(), bevy::prelude::BevyError> {
            let theme = world.resource::<WidgetryThemeMode>().colors().text_field;
            let roots = world
                .query_filtered::<Entity, (
                    With<TextFieldBase>,
                    Without<bevy_widgetry_core::color::WidgetryStyleOwner<WidgetryTextField>>,
                )>()
                .iter(world)
                .collect::<Vec<_>>();
            for root in roots {
                let colors =
                    crate::WidgetryTextFieldColorOverrides::get(world, root)?.resolve(&theme);
                apply_owned_text_field_colors(world, root, &colors)?;
            }
            Ok(())
        },
    );
}

impl WidgetryTextField {
    fn scene(props: crate::WidgetryTextFieldProps) -> impl Scene {
        bsn! { text_field_base_scene() template(move |_| props.colors.clone().initial()) }
    }
}

impl WidgetryReadOnlyTextField {
    fn scene(props: crate::WidgetryTextFieldProps) -> impl Scene {
        bsn! { text_field_base_scene() ReadOnly template(move |_| props.colors.clone().initial()) }
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
        if !app.is_plugin_added::<bevy_widgetry_core::ui::WidgetryUiPlugin>() {
            app.add_plugins(bevy_widgetry_core::ui::WidgetryUiPlugin);
        }
        if !app.is_plugin_added::<TabNavigationPlugin>() {
            app.add_plugins(TabNavigationPlugin);
        }
        if !app.is_plugin_added::<WidgetryThemePlugin>() {
            app.add_plugins(WidgetryThemePlugin);
        }

        app.add_observer(refresh_text_field_theme);
        app.add_observer(retain_text_field_focus_on_acquire);
        app.add_systems(
            PostUpdate,
            (
                block_disabled_text_field_edits,
                block_read_only_text_field_edits,
            )
                .after(bevy_widgetry_core::ui::WidgetryUiSystems::Disabled)
                .before(EditableTextSystems),
        );

        app.add_systems(
            PostUpdate,
            (
                update_widgetry_text_field_style_changed,
                update_widgetry_text_field_style_focus_changed,
                update_widgetry_text_field_style_removed,
            )
                .in_set(bevy_widgetry_core::ui::WidgetryUiSystems::Colors),
        );
        widgetry_info!("WidgetryTextFieldPlugin 注册完成");
    }
}

pub fn apply_owned_text_field_colors(
    world: &mut bevy::prelude::World,
    entity: Entity,
    colors: &bevy_widgetry_theme::WidgetryTextFieldColors,
) -> Result<(), bevy::prelude::BevyError> {
    use bevy::prelude::*;
    let colors = if world.get::<ReadOnly>(entity).is_some() {
        colors.read_only
    } else {
        colors.editable
    };
    let focused = world.get_resource::<InputFocus>().and_then(InputFocus::get) == Some(entity);
    let state = if world.get::<InteractionDisabled>(entity).is_some() {
        colors.disabled
    } else if focused {
        colors.focused
    } else if world.get::<Hovered>(entity).is_some_and(|h| h.0) {
        colors.hovered
    } else {
        colors.normal
    };
    let invalid = || {
        bevy_widgetry_log::widgetry_error!(?entity, "TextField 缺失托管颜色输出");
        BevyError::error("TextField 缺失托管颜色输出")
    };
    let mut query = world.query::<(
        &mut BackgroundColor,
        &mut BorderColor,
        &mut TextColor,
        &mut TextCursorStyle,
    )>();
    let outputs = query.get_mut(world, entity).map_err(|_| invalid())?;
    apply_outputs(state, outputs);
    Ok(())
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
