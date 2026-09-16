use bevy::{
    app::{App, Plugin, PostUpdate, Update},
    color::Color,
    ecs::{
        change_detection::DetectChanges,
        entity::Entity,
        lifecycle::RemovedComponents,
        observer::On,
        query::{Added, Changed, Has, Or, With},
        schedule::IntoScheduleConfigs,
        system::{Query, Res},
    },
    input_focus::InputFocus,
    picking::hover::Hovered,
    prelude::{Scene, SceneComponent, bsn},
    text::{EditableText, EditableTextSystems, TextColor, TextCursorStyle},
    ui::{BackgroundColor, BorderColor, BorderRadius, InteractionDisabled, Node, UiRect, px},
};
use bevy_widgetry_core::{ColorTheme, ThemeChanged, ThemeMode, ThemePlugin, WidgetryFocusPlugin};
use bevy_widgetry_log::widgetry_info;

/// 基于官方 EditableText 的主题输入框，通过 BSN 的 `@WidgetryTextField` 构造。
/// 需注册 WidgetryTextFieldPlugin；应用负责提供 Bevy EditableTextInputPlugin。
/// 文本、换行、可见行数及字体由调用方 patch 官方组件，不默认参与 Tab 导航。
/// 高度由官方 visible_lines 测量，不设置固定高度或最小高度。
/// 颜色由主题管理，状态优先级为禁用、焦点、悬停、普通。
#[derive(SceneComponent, Default, Clone)]
pub struct WidgetryTextField;

/// 合并禁用、焦点与悬停优先级后的文本框配色。
#[derive(Debug, PartialEq)]
struct TextFieldStyle {
    /// 状态解析完成后要写入节点的背景色。
    background: Color,
    /// 状态解析完成后要写入节点的边框色。
    border: Color,
    /// 普通状态下文本与图标使用的前景色。
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

/// 装配文本框基础行为与主题样式，跟踪焦点、禁用状态和选区颜色。
/// 自动装配主题和指针清焦策略，不安装字体 fallback 或官方文本输入插件。
pub struct WidgetryTextFieldPlugin;

/// 集中表达文本输入框的样式变更过滤条件。
type ChangedTextFieldStyleQuery<'w, 's> = Query<
    'w,
    's,
    TextFieldStyleData,
    (
        With<WidgetryTextField>,
        Or<(
            Added<WidgetryTextField>,
            Changed<Hovered>,
            Added<InteractionDisabled>,
        )>,
    ),
>;

/// 在官方编辑处理前清除禁用控件的用户操作，保留程序化 set_text 的结果。
fn block_disabled_text_field_edits(
    mut query: Query<&mut EditableText, (With<WidgetryTextField>, With<InteractionDisabled>)>,
) {
    for mut editable_text in &mut query {
        editable_text.pending_edits.clear();
        editable_text.pending_paste = None;
    }
}

/// 按禁用、焦点、悬停、普通的优先级选择文本框颜色。
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

/// 同步背景、边框、文字、光标及选区，保持同一主题下的完整外观。
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

/// 在新增控件或交互状态变化时读取当前焦点并应用完整样式。
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

/// 焦点资源变化时重新解析各输入框，覆盖获得和失去焦点两条路径。
fn update_widgetry_text_field_style_focus_changed(
    mode: Res<ThemeMode>,
    input_focus: Res<InputFocus>,
    mut query: Query<TextFieldStyleData, With<WidgetryTextField>>,
) {
    if !input_focus.is_changed() {
        return;
    }

    let focused = input_focus.get();

    for item in &mut query {
        apply_text_field_style(mode.colors(), focused, item);
    }
}

/// 禁用状态移除后恢复当前焦点或悬停对应的样式。
fn update_widgetry_text_field_style_removed(
    mode: Res<ThemeMode>,
    input_focus: Res<InputFocus>,
    mut removed_disabled: RemovedComponents<InteractionDisabled>,
    mut query: Query<TextFieldStyleData, With<WidgetryTextField>>,
) {
    let focused = input_focus.get();

    for entity in removed_disabled.read() {
        if let Ok(item) = query.get_mut(entity) {
            apply_text_field_style(mode.colors(), focused, item);
        }
    }
}

/// 主题事件到达后立即刷新所有输入框而不修改其编辑状态。
fn refresh_text_field_theme(
    event: On<ThemeChanged>,
    input_focus: Res<InputFocus>,
    mut query: Query<TextFieldStyleData, With<WidgetryTextField>>,
) {
    let focused = input_focus.get();

    for item in &mut query {
        apply_text_field_style(event.mode.colors(), focused, item);
    }
}

impl WidgetryTextField {
    /// 单实体外壳仅提供布局和主题输出组件，编辑默认值沿用官方定义。
    fn scene() -> impl Scene {
        bsn! {
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
        app.add_systems(
            PostUpdate,
            block_disabled_text_field_edits.before(EditableTextSystems),
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
