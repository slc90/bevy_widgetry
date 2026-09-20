use bevy::{
    app::{App, Plugin, Propagate, Update},
    color::Color,
    ecs::{
        lifecycle::RemovedComponents,
        observer::On,
        query::{Added, Changed, Has, Or, With},
        system::{Query, Res},
    },
    input_focus::tab_navigation::TabIndex,
    picking::hover::Hovered,
    prelude::{Scene, SceneComponent, bsn, template},
    ui::{
        BackgroundColor, BorderColor, BorderRadius, InteractionDisabled, Node, Pressed, UiRect, px,
    },
    ui_widgets::{Button, ButtonPlugin},
};
use bevy_widgetry_core::{
    ColorTheme, ForegroundColor, ForegroundColorPlugin, ThemeChanged, ThemeMode, ThemePlugin,
};
use bevy_widgetry_log::widgetry_info;

/// 使用 Widgetry 默认视觉的官方 Bevy Button；需注册 WidgetryButtonPlugin。
/// 通过 BSN 的 @WidgetryButton 构造完整外壳，内容与 layout 由调用方组合和 patch。
/// 默认不参与 Tab navigation；视觉优先级为 disabled、pressed、hover、普通，不提供 focus style。
/// 背景、border 和传播的 foreground color 是运行期 theme 输出，直接 patch 颜色会在 state 或 theme 更新时被覆盖。
#[derive(SceneComponent, Default, Clone)]
pub struct WidgetryButton;

/// 一次 state 解析得到的完整 Button 配色，供初始化和增量刷新共用。
#[derive(Debug, PartialEq)]
struct ButtonStyle {
    /// state 解析完成后要写入 node 的 background color。
    background: Color,
    /// state 解析完成后要写入 node 的 border color。
    border: Color,
    /// 普通 state 下文本与 icon 使用的 foreground color。
    foreground: Color,
}

type ButtonStyleData = (
    &'static Hovered,
    Has<Pressed>,
    Has<InteractionDisabled>,
    &'static mut BackgroundColor,
    &'static mut BorderColor,
    &'static mut Propagate<ForegroundColor>,
);

/// 注册官方 Button 行为、Button style 和 theme 刷新，并装配共享 foreground color 传播 plugin。
/// 内容由调用方通过 children 提供，文本字体由调用方配置。
pub struct WidgetryButtonPlugin;

/// 仅访问需要重新解析 style 的 Widget，保持 change filter 条件集中。
type ChangedButtonStyleQuery<'w, 's> = Query<
    'w,
    's,
    ButtonStyleData,
    (
        With<WidgetryButton>,
        Or<(
            Added<WidgetryButton>,
            Changed<Hovered>,
            Added<Pressed>,
            Added<InteractionDisabled>,
        )>,
    ),
>;

/// 按 disabled、pressed、hover、普通的顺序选择完整配色。
fn resolve_button_style(
    colors: &ColorTheme,
    hovered: bool,
    pressed: bool,
    disabled: bool,
) -> ButtonStyle {
    let (background, border) = if disabled {
        (
            colors.control_background_disabled,
            colors.control_border_disabled,
        )
    } else if pressed {
        (
            colors.control_background_pressed,
            colors.control_border_pressed,
        )
    } else if hovered {
        (
            colors.control_background_hovered,
            colors.control_border_hovered,
        )
    } else {
        (colors.control_background, colors.control_border)
    };
    ButtonStyle {
        background,
        border,
        foreground: if disabled {
            colors.foreground_disabled
        } else {
            colors.foreground
        },
    }
}

/// 把解析结果写入背景、border 与传播的 foreground color，避免三者来自不同 state。
fn apply_button_style(
    colors: &ColorTheme,
    (hovered, pressed, disabled, mut background, mut border, mut foreground): <ButtonStyleData as bevy::ecs::query::QueryData>::Item<'_, '_>,
) {
    let style = resolve_button_style(colors, hovered.0, pressed, disabled);
    background.0 = style.background;
    *border = BorderColor::all(style.border);
    foreground.0 = ForegroundColor(style.foreground);
}

/// 响应新增 style 及 interaction component 变更，首次挂载也读取已有 state。
fn update_widgetry_button_style_changed(
    mode: Res<ThemeMode>,
    mut query: ChangedButtonStyleQuery<'_, '_>,
) {
    for item in &mut query {
        apply_button_style(mode.colors(), item);
    }
}

/// 移除 pressed 或 disabled state 后重新解析剩余 state 的配色。
fn update_widgetry_button_style_removed(
    mode: Res<ThemeMode>,
    mut removed_pressed: RemovedComponents<Pressed>,
    mut removed_disabled: RemovedComponents<InteractionDisabled>,
    mut query: Query<ButtonStyleData, With<WidgetryButton>>,
) {
    for entity in removed_pressed.read().chain(removed_disabled.read()) {
        if let Ok(item) = query.get_mut(entity) {
            apply_button_style(mode.colors(), item);
        }
    }
}

/// 收到 theme 通知时立即刷新全部 Button，避免等待 interaction state 再次变化。
fn refresh_button_theme(
    event: On<ThemeChanged>,
    mut query: Query<ButtonStyleData, With<WidgetryButton>>,
) {
    for item in &mut query {
        apply_button_style(event.mode.colors(), item);
    }
}

impl WidgetryButton {
    /// 一次性展开默认外壳，保留 BSN 对几何 field 的局部覆盖能力。
    fn scene() -> impl Scene {
        bsn! {
            Button
            Hovered(false)
            TabIndex(-1)
            Node {
                min_height: px(32),
                padding: UiRect::axes(px(12), px(6)),
                border: UiRect::all(px(1)),
                border_radius: BorderRadius::all(px(4)),
            }
            BackgroundColor
            BorderColor
            template(|_| Ok(Propagate(ForegroundColor::default())))
        }
    }
}

impl Plugin for WidgetryButtonPlugin {
    fn build(&self, app: &mut App) {
        if !app.is_plugin_added::<ButtonPlugin>() {
            app.add_plugins(ButtonPlugin);
        }
        if !app.is_plugin_added::<ForegroundColorPlugin>() {
            app.add_plugins(ForegroundColorPlugin);
        }
        if !app.is_plugin_added::<ThemePlugin>() {
            app.add_plugins(ThemePlugin);
        }
        app.add_observer(refresh_button_theme);
        app.add_systems(
            Update,
            (
                update_widgetry_button_style_changed,
                update_widgetry_button_style_removed,
            ),
        );
        widgetry_info!("WidgetryButtonPlugin 注册完成");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const TEST_THEME: ColorTheme = ColorTheme {
        window_background: Color::BLACK,
        window_border: Color::WHITE,
        title_bar_border: Color::WHITE,
        foreground: Color::srgb_u8(1, 0, 0),
        foreground_disabled: Color::srgb_u8(2, 0, 0),
        control_background: Color::srgb_u8(3, 0, 0),
        control_background_hovered: Color::srgb_u8(4, 0, 0),
        control_background_pressed: Color::srgb_u8(5, 0, 0),
        control_background_active: Color::srgb_u8(6, 0, 0),
        control_background_disabled: Color::srgb_u8(7, 0, 0),
        control_border: Color::srgb_u8(8, 0, 0),
        control_border_hovered: Color::srgb_u8(9, 0, 0),
        control_border_pressed: Color::srgb_u8(10, 0, 0),
        control_border_active: Color::srgb_u8(11, 0, 0),
        control_border_disabled: Color::srgb_u8(12, 0, 0),
        popup_background: Color::srgb_u8(13, 0, 0),
        popup_border: Color::srgb_u8(14, 0, 0),
        item_background_hovered: Color::srgb_u8(15, 0, 0),
        item_background_selected: Color::srgb_u8(16, 0, 0),
        text_selection: Color::srgb_u8(52, 92, 140),
        text_selection_unfocused: Color::srgb_u8(65, 70, 78),
    };

    // 用互不相同的测试颜色组合 interaction state，验证完整配色与 disabled、pressed、hover 优先级。
    #[test]
    fn resolves_complete_style_with_state_priority() {
        let c = &TEST_THEME;
        for (hovered, pressed, disabled, background, border, foreground) in [
            (
                false,
                false,
                false,
                c.control_background,
                c.control_border,
                c.foreground,
            ),
            (
                true,
                false,
                false,
                c.control_background_hovered,
                c.control_border_hovered,
                c.foreground,
            ),
            (
                true,
                true,
                false,
                c.control_background_pressed,
                c.control_border_pressed,
                c.foreground,
            ),
            (
                true,
                true,
                true,
                c.control_background_disabled,
                c.control_border_disabled,
                c.foreground_disabled,
            ),
        ] {
            assert_eq!(
                resolve_button_style(c, hovered, pressed, disabled),
                ButtonStyle {
                    background,
                    border,
                    foreground
                }
            );
        }
    }
}
