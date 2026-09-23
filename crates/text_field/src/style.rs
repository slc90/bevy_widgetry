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
    input_focus::{AcquireFocus, InputFocus},
    picking::hover::Hovered,
    prelude::{Component, Scene, SceneComponent, bsn},
    text::{EditableText, EditableTextSystems, TextColor, TextCursorStyle, TextEdit},
    ui::{BackgroundColor, BorderColor, BorderRadius, InteractionDisabled, Node, UiRect, px},
};
use bevy_widgetry_core::{ColorTheme, ThemeChanged, ThemeMode, ThemePlugin, WidgetryFocusPlugin};
use bevy_widgetry_log::widgetry_info;

/// 基于官方 EditableText 的 theme TextField，通过 BSN 的 @WidgetryTextField 构造。
/// 需注册 WidgetryTextFieldPlugin；应用负责提供 Bevy EditableTextInputPlugin。
/// 文本、换行、可见行数及字体由调用方 patch 官方 component，不默认参与 Tab navigation。
/// 高度由官方 visible_lines 测量，不设置固定高度或最小高度。
/// 颜色由 theme 管理，state 优先级为 disabled、focus、hover、普通。
#[derive(SceneComponent, Default, Clone)]
pub struct WidgetryTextField;

/// 保留官方 selection、navigation 和复制能力，同时禁止用户修改内容的 TextField。
/// 需注册 WidgetryTextFieldPlugin；调用方仍可 patch EditableText 并程序化修改文本。
/// ReadOnly 身份在构造时确定，不提供运行时切换 API。
#[derive(SceneComponent, Default, Clone)]
pub struct WidgetryReadOnlyTextField;

/// 标识两种 TextField 共用的 style 与 disabled 行为范围。
#[derive(Component, Default, Clone)]
struct TextFieldBase;

/// 标识需要在官方编辑阶段前过滤 mutation 的 TextField。
#[derive(Component, Default, Clone)]
struct ReadOnly;

/// 合并 disabled、focus 与 hover 优先级后的 TextField 配色。
#[derive(Debug, PartialEq)]
struct TextFieldStyle {
    /// state 解析完成后要写入 node 的 background color。
    background: Color,
    /// state 解析完成后要写入 node 的 border color。
    border: Color,
    /// 普通 state 下文本与 icon 使用的 foreground color。
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

/// 装配 TextField 基础行为与 theme style，跟踪 focus、disabled state 和 selection 颜色。
/// 自动装配 theme 和 pointer 清除 focus 的策略，不安装字体 fallback 或官方文本输入 plugin。
pub struct WidgetryTextFieldPlugin;

/// 集中表达 TextField 的 style 变更 filter 条件。
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

/// 在官方编辑处理前清除 disabled Widget 的用户操作，保留程序化 set_text 的结果。
fn block_disabled_text_field_edits(
    mut query: Query<&mut EditableText, (With<TextFieldBase>, With<InteractionDisabled>)>,
) {
    for mut editable_text in &mut query {
        editable_text.pending_edits.clear();
        editable_text.pending_paste = None;
    }
}

/// 在官方编辑阶段前丢弃 ReadOnly 的 mutation，保留其余 navigation 与 selection command。
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

/// 已获得 focus 的 TextField 接住后续 AcquireFocus，避免它继续冒泡到 window 清除 focus。
fn retain_text_field_focus_on_acquire(
    mut event: On<AcquireFocus>,
    text_fields: Query<(), (With<TextFieldBase>, Without<InteractionDisabled>)>,
    focus: Res<InputFocus>,
) {
    if text_fields.contains(event.focused_entity) && focus.get() == Some(event.focused_entity) {
        event.propagate(false);
    }
}

/// 按 disabled、focus、hover、普通的优先级选择 TextField 颜色。
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

/// 同步背景、border、文本、cursor 及 selection，保持同一 theme 下的完整外观。
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

/// 在新增 Widget 或 interaction state 变化时读取当前 focus 并应用完整 style。
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

/// focus resource 变化时重新解析各 TextField，覆盖获得和失去 focus 两条路径。
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

/// disabled state 移除后恢复当前 focus 或 hover 对应的 style。
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

/// theme event 到达后立即刷新所有 TextField 而不修改其编辑 state。
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
    /// 单 entity 外壳仅提供 layout 和 theme 输出 component，编辑默认值沿用官方定义。
    fn scene() -> impl Scene {
        text_field_base_scene()
    }
}

impl WidgetryReadOnlyTextField {
    /// 复用同一外壳，并附加仅用于输入过滤的内部身份。
    fn scene() -> impl Scene {
        bsn! { text_field_base_scene() ReadOnly }
    }
}

/// 两种 TextField 共用同一单 entity layout 与默认官方编辑配置。
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
        if !app.is_plugin_added::<WidgetryFocusPlugin>() {
            app.add_plugins(WidgetryFocusPlugin);
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

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::scene::WorldSceneExt;
    use bevy_widgetry_test_utils::scene_app;

    // 两种 Scene 同时存在时，普通 TextField 不得获得 ReadOnly 身份或丢弃输入。
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
