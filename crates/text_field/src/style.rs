use crate::{TextField, TextFieldPlugin};
use bevy::{
    app::{App, Plugin, Update},
    color::Color,
    ecs::{
        change_detection::DetectChanges,
        component::Component,
        entity::Entity,
        lifecycle::RemovedComponents,
        observer::On,
        query::{Added, Changed, Has, Or, With},
        system::{Query, Res},
    },
    input_focus::InputFocus,
    picking::hover::Hovered,
    text::{TextColor, TextCursorStyle},
    ui::{BackgroundColor, BorderColor, InteractionDisabled, Node, UiRect, px},
    utils::default,
};
use bevy_widgetry_core::{ColorTheme, ThemeChanged, ThemeMode, ThemePlugin, WidgetryFontPlugin};
use bevy_widgetry_log::{widgetry_error, widgetry_info};

/// 由控件自身保存必需组件异常的边沿，销毁时不报告恢复。
#[derive(Component, Default, Debug)]
struct StyledTextFieldDiagnostics {
    /// 上一帧是否已报告必需组件缺失。
    failed: bool,
}

/// 带主题配色的 TextField；需注册 StyledTextFieldPlugin，禁用状态优先于焦点。
#[derive(Component, Default)]
#[require(StyledTextFieldDiagnostics)]
#[require(
    TextField,
    Hovered,
    Node = styled_text_field_node(),
    BackgroundColor,
    BorderColor,
)]
pub struct StyledTextField;

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
/// 自动提供 App 默认字体；使用内建字体时须先注册 Bevy 资产与文本插件（通常为 DefaultPlugins）。
pub struct StyledTextFieldPlugin;

/// 集中表达文本输入框的样式变更过滤条件。
type ChangedTextFieldStyleQuery<'w, 's> = Query<
    'w,
    's,
    TextFieldStyleData,
    (
        With<StyledTextField>,
        Or<(
            Added<StyledTextField>,
            Changed<Hovered>,
            Added<InteractionDisabled>,
        )>,
    ),
>;

/// 提供文本框默认尺寸、内边距与边框布局。
fn styled_text_field_node() -> Node {
    Node {
        width: px(240),
        height: px(40),
        padding: UiRect::axes(px(10), px(6)),
        border: UiRect::all(px(1)),
        ..default()
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
fn update_styled_text_field_style_changed(
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
fn update_styled_text_field_style_focus_changed(
    mode: Res<ThemeMode>,
    input_focus: Res<InputFocus>,
    mut query: Query<TextFieldStyleData, With<StyledTextField>>,
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
fn update_styled_text_field_style_removed(
    mode: Res<ThemeMode>,
    input_focus: Res<InputFocus>,
    mut removed_disabled: RemovedComponents<InteractionDisabled>,
    mut query: Query<TextFieldStyleData, With<StyledTextField>>,
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
    mut query: Query<TextFieldStyleData, With<StyledTextField>>,
) {
    let focused = input_focus.get();

    for item in &mut query {
        apply_text_field_style(event.mode.colors(), focused, item);
    }
}

/// 在帧末检查必需组件，避免业务查询过滤掉损坏的控件。
fn diagnose_required_components(
    mut query: Query<
        (
            bevy::prelude::Entity,
            &mut StyledTextFieldDiagnostics,
            bevy::ecs::query::Has<TextField>,
            bevy::ecs::query::Has<Hovered>,
            bevy::ecs::query::Has<Node>,
            bevy::ecs::query::Has<BackgroundColor>,
            bevy::ecs::query::Has<BorderColor>,
            bevy::ecs::query::Has<TextColor>,
            bevy::ecs::query::Has<TextCursorStyle>,
        ),
        With<StyledTextField>,
    >,
) {
    for (
        entity,
        mut state,
        has_text_field,
        has_hovered,
        has_node,
        has_background_color,
        has_border_color,
        has_text_color,
        has_text_cursor_style,
    ) in &mut query
    {
        let failed = !(has_text_field
            && has_hovered
            && has_node
            && has_background_color
            && has_border_color
            && has_text_color
            && has_text_cursor_style);
        if failed != state.failed {
            if failed {
                widgetry_error!(
                    ?entity,
                    has_text_field,
                    has_hovered,
                    has_node,
                    has_background_color,
                    has_border_color,
                    has_text_color,
                    has_text_cursor_style,
                    "StyledTextField 必需组件缺失"
                );
            } else {
                widgetry_info!(?entity, "StyledTextField 必需组件恢复正常");
            }
            state.failed = failed;
        }
    }
}

impl Plugin for StyledTextFieldPlugin {
    fn build(&self, app: &mut App) {
        if !app.is_plugin_added::<TextFieldPlugin>() {
            app.add_plugins(TextFieldPlugin);
        }

        if !app.is_plugin_added::<WidgetryFontPlugin>() {
            app.add_plugins(WidgetryFontPlugin);
        }
        if !app.is_plugin_added::<ThemePlugin>() {
            app.add_plugins(ThemePlugin);
        }

        app.add_observer(refresh_text_field_theme);

        app.add_systems(
            Update,
            (
                update_styled_text_field_style_changed,
                update_styled_text_field_style_focus_changed,
                update_styled_text_field_style_removed,
            ),
        );
        app.add_systems(bevy::app::PostUpdate, diagnose_required_components);
        widgetry_info!("StyledTextFieldPlugin 注册完成");
    }
}
