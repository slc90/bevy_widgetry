use crate::virtualization::ListRuntime;
use crate::{WidgetryListView, WidgetryListViewItem, WidgetryListViewState};
use bevy::app::Propagate;
use bevy::input_focus::InputFocus;
use bevy::picking::hover::Hovered;
use bevy::prelude::*;
use bevy::ui::{InteractionDisabled, Pressed, Selected};
use bevy_widgetry_core::{ColorTheme, ForegroundColor, ThemeChanged, ThemeMode};

type RootStyleData = (
    Entity,
    Has<InteractionDisabled>,
    &'static WidgetryListViewState,
    Option<&'static ListRuntime>,
    &'static mut BorderColor,
);

type RowStyleData = (
    &'static WidgetryListViewItem,
    Option<&'static Hovered>,
    Has<Pressed>,
    Has<Selected>,
    Has<InteractionDisabled>,
    &'static mut BackgroundColor,
    &'static mut BorderColor,
    &'static mut Propagate<ForegroundColor>,
);

#[derive(Debug, PartialEq)]
struct RowStyle {
    background: Color,
    border: Color,
    foreground: Color,
}

fn root_border(colors: &ColorTheme, disabled: bool, focused: bool) -> Color {
    if disabled {
        colors.control_border_disabled
    } else if focused {
        colors.control_border_active
    } else {
        colors.control_border
    }
}

fn resolve_row(
    colors: &ColorTheme,
    disabled: bool,
    pressed: bool,
    hovered: bool,
    selected: bool,
    active: bool,
) -> RowStyle {
    RowStyle {
        background: if disabled {
            Color::NONE
        } else if pressed {
            colors.control_background_pressed
        } else if hovered {
            colors.item_background_hovered
        } else if selected {
            colors.item_background_selected
        } else {
            Color::NONE
        },
        border: if active {
            colors.control_border_active
        } else {
            Color::NONE
        },
        foreground: if disabled {
            colors.foreground_disabled
        } else {
            colors.foreground
        },
    }
}

fn apply<T: Send + Sync + 'static>(
    colors: &ColorTheme,
    focus: Option<Entity>,
    roots: &mut Query<RootStyleData, (With<WidgetryListView<T>>, Without<WidgetryListViewItem>)>,
    rows: &mut Query<RowStyleData, (With<WidgetryListViewItem>, Without<WidgetryListView<T>>)>,
) {
    for (root, disabled, state, runtime, mut border) in roots.iter_mut() {
        let focused = focus == Some(root);
        border.set_if_neq(BorderColor::all(root_border(colors, disabled, focused)));
        let Some(runtime) = runtime else { continue };
        for &row in &runtime.rows {
            let Ok((
                item,
                hovered,
                pressed,
                selected,
                row_disabled,
                mut background,
                mut border,
                mut foreground,
            )) = rows.get_mut(row)
            else {
                continue;
            };
            let style = resolve_row(
                colors,
                disabled || row_disabled,
                pressed,
                hovered.is_some_and(|hovered| hovered.0),
                selected,
                focused && !disabled && state.active == Some(item.id),
            );
            background.set_if_neq(BackgroundColor(style.background));
            border.set_if_neq(BorderColor::all(style.border));
            if foreground.0 != ForegroundColor(style.foreground) {
                foreground.0 = ForegroundColor(style.foreground);
            }
        }
    }
}

pub(crate) fn update<T: Send + Sync + 'static>(
    mode: Res<ThemeMode>,
    focus: Res<InputFocus>,
    mut roots: Query<RootStyleData, (With<WidgetryListView<T>>, Without<WidgetryListViewItem>)>,
    mut rows: Query<RowStyleData, (With<WidgetryListViewItem>, Without<WidgetryListView<T>>)>,
) {
    apply(mode.colors(), focus.get(), &mut roots, &mut rows);
}

pub(crate) fn refresh_theme<T: Send + Sync + 'static>(
    event: On<ThemeChanged>,
    focus: Res<InputFocus>,
    mut roots: Query<RootStyleData, (With<WidgetryListView<T>>, Without<WidgetryListViewItem>)>,
    mut rows: Query<RowStyleData, (With<WidgetryListViewItem>, Without<WidgetryListView<T>>)>,
) {
    apply(event.mode.colors(), focus.get(), &mut roots, &mut rows);
}

// 测试断言需要在 contract 不满足时立即失败；生产代码的 panic lint 会拒绝这些表达式，因此仅在本测试 scope 允许所列 lint。
#[cfg(test)]
#[allow(clippy::disallowed_macros)]
mod tests {
    use super::*;
    use bevy_widgetry_core::DARK_THEME;

    fn colors() -> ColorTheme {
        ColorTheme {
            control_border: Color::srgb_u8(1, 0, 0),
            control_border_active: Color::srgb_u8(2, 0, 0),
            control_border_disabled: Color::srgb_u8(3, 0, 0),
            control_background_pressed: Color::srgb_u8(4, 0, 0),
            item_background_hovered: Color::srgb_u8(5, 0, 0),
            item_background_selected: Color::srgb_u8(6, 0, 0),
            foreground: Color::srgb_u8(7, 0, 0),
            foreground_disabled: Color::srgb_u8(8, 0, 0),
            ..DARK_THEME
        }
    }

    #[test]
    fn row_priority_and_active_border_are_orthogonal() {
        let colors = colors();
        for mask in 0..32 {
            let disabled = mask & 1 != 0;
            let pressed = mask & 2 != 0;
            let hovered = mask & 4 != 0;
            let selected = mask & 8 != 0;
            let active = mask & 16 != 0;
            let style = resolve_row(&colors, disabled, pressed, hovered, selected, active);
            assert_eq!(
                style.background,
                if disabled {
                    Color::NONE
                } else if pressed {
                    colors.control_background_pressed
                } else if hovered {
                    colors.item_background_hovered
                } else if selected {
                    colors.item_background_selected
                } else {
                    Color::NONE
                },
                "mask={mask}"
            );
            assert_eq!(
                style.foreground,
                if disabled {
                    colors.foreground_disabled
                } else {
                    colors.foreground
                },
                "mask={mask}"
            );
            assert_eq!(
                style.border,
                if active {
                    colors.control_border_active
                } else {
                    Color::NONE
                },
                "mask={mask}"
            );
        }
    }

    #[test]
    fn root_disabled_overrides_focus() {
        let colors = colors();
        assert_eq!(root_border(&colors, false, false), colors.control_border);
        assert_eq!(
            root_border(&colors, false, true),
            colors.control_border_active
        );
        assert_eq!(
            root_border(&colors, true, false),
            colors.control_border_disabled
        );
        assert_eq!(
            root_border(&colors, true, true),
            colors.control_border_disabled
        );
    }
}
