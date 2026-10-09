use bevy_widgetry_theme::{WidgetryThemeChanged, WidgetryThemeMode};

use crate::virtualization::ListRuntime;
use crate::{WidgetryListView, WidgetryListViewItem, WidgetryListViewState};
use bevy::input_focus::InputFocus;
use bevy::picking::hover::Hovered;
use bevy::prelude::*;
use bevy::ui::{InteractionDisabled, Pressed, Selected};
use bevy_widgetry_core::foreground::ResolvedForeground;
#[derive(Debug, PartialEq)]
struct RowStyle {
    background: Color,
    border: Color,
    foreground: Color,
}

fn root_border(
    colors: &bevy_widgetry_theme::WidgetryListViewColors,
    disabled: bool,
    focused: bool,
) -> Color {
    if disabled {
        colors.container.disabled.border
    } else if focused {
        colors.container.focused.border
    } else {
        colors.container.normal.border
    }
}

fn resolve_row(
    colors: &bevy_widgetry_theme::WidgetryListViewColors,
    disabled: bool,
    pressed: bool,
    hovered: bool,
    selected: bool,
    active: bool,
) -> RowStyle {
    let colors = &colors.item;
    let state = if disabled {
        colors.disabled
    } else if pressed {
        colors.pressed
    } else if hovered {
        colors.hovered
    } else if selected {
        colors.selected
    } else {
        colors.normal
    };
    RowStyle {
        background: state.background,
        border: if active {
            if disabled {
                colors.disabled_active_border
            } else {
                colors.active_border
            }
        } else {
            state.border
        },
        foreground: state.foreground,
    }
}

pub fn apply_owned_list_colors<T: Send + Sync + 'static>(
    world: &mut World,
    root: Entity,
    colors: &bevy_widgetry_theme::WidgetryListViewColors,
) -> Result<(), BevyError> {
    let disabled = world.get::<InteractionDisabled>(root).is_some();
    let focused = world.get_resource::<InputFocus>().and_then(InputFocus::get) == Some(root);
    let state = *world
        .get::<WidgetryListViewState>(root)
        .ok_or_else(|| invalid_structure(root))?;
    let container = if disabled {
        colors.container.disabled
    } else if focused {
        colors.container.focused
    } else {
        colors.container.normal
    };
    world
        .get_mut::<BackgroundColor>(root)
        .ok_or_else(|| invalid_structure(root))?
        .set_if_neq(BackgroundColor(container.background));
    world
        .get_mut::<BorderColor>(root)
        .ok_or_else(|| invalid_structure(root))?
        .set_if_neq(BorderColor::all(root_border(colors, disabled, focused)));
    world
        .get_mut::<ResolvedForeground>(root)
        .ok_or_else(|| invalid_structure(root))?
        .set_if_neq(ResolvedForeground(container.foreground));
    let rows = world
        .get::<ListRuntime>(root)
        .map(|runtime| runtime.rows.clone())
        .unwrap_or_default();
    for row in rows {
        let item = *world
            .get::<WidgetryListViewItem>(row)
            .ok_or_else(|| invalid_structure(row))?;
        let row_disabled = disabled || world.get::<InteractionDisabled>(row).is_some();
        let style = resolve_row(
            colors,
            row_disabled,
            world.get::<Pressed>(row).is_some(),
            world.get::<Hovered>(row).is_some_and(|h| h.0),
            world.get::<Selected>(row).is_some(),
            focused && state.active == Some(item.id),
        );
        world
            .get_mut::<BackgroundColor>(row)
            .ok_or_else(|| invalid_structure(row))?
            .set_if_neq(BackgroundColor(style.background));
        world
            .get_mut::<BorderColor>(row)
            .ok_or_else(|| invalid_structure(row))?
            .set_if_neq(BorderColor::all(style.border));
        world
            .get_mut::<ResolvedForeground>(row)
            .ok_or_else(|| invalid_structure(row))?
            .set_if_neq(ResolvedForeground(style.foreground));
    }
    Ok(())
}
fn invalid_structure(entity: Entity) -> BevyError {
    bevy_widgetry_log::widgetry_error!(?entity, "ListView 缺失颜色主体或必需输出");
    BevyError::error("ListView 缺失颜色主体或必需输出")
}
pub(crate) fn update<T: Send + Sync + 'static>(world: &mut World) -> Result<(), BevyError> {
    let colors = world.resource::<WidgetryThemeMode>().colors().list_view;
    let roots = world
        .query_filtered::<Entity, (
            With<WidgetryListView<T>>,
            Without<bevy_widgetry_core::color::WidgetryStyleOwner<WidgetryListView<T>>>,
        )>()
        .iter(world)
        .collect::<Vec<_>>();
    for root in roots {
        let overrides = world
            .get::<crate::colors::ColorState>(root)
            .ok_or_else(|| invalid_structure(root))?
            .0
            .resolve(&colors);
        apply_owned_list_colors::<T>(world, root, &overrides)?;
    }
    Ok(())
}
pub(crate) fn refresh_theme<T: Send + Sync + 'static>(
    _event: On<WidgetryThemeChanged>,
    mut commands: Commands,
) {
    commands.queue(update::<T>);
}

// 测试断言需要在 contract 不满足时立即失败。
// 生产代码的 panic lint 会拒绝这些表达式，因此仅在本测试 scope 允许所列 lint。
#[cfg(test)]
#[allow(clippy::disallowed_macros)]
mod tests {
    use super::*;
    use bevy_widgetry_theme::WIDGETRY_DARK_THEME;

    fn colors() -> bevy_widgetry_theme::WidgetryListViewColors {
        let mut colors = WIDGETRY_DARK_THEME.list_view;
        colors.container.normal.border = Color::srgb_u8(1, 0, 0);
        colors.container.focused.border = Color::srgb_u8(2, 0, 0);
        colors.container.disabled.border = Color::srgb_u8(3, 0, 0);
        colors.item.pressed.background = Color::srgb_u8(4, 0, 0);
        colors.item.hovered.background = Color::srgb_u8(5, 0, 0);
        colors.item.selected.background = Color::srgb_u8(6, 0, 0);
        colors.item.normal.foreground = Color::srgb_u8(7, 0, 0);
        colors.item.disabled.foreground = Color::srgb_u8(8, 0, 0);
        colors.item.active_border = Color::srgb_u8(9, 0, 0);
        colors
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
                    colors.item.pressed.background
                } else if hovered {
                    colors.item.hovered.background
                } else if selected {
                    colors.item.selected.background
                } else {
                    Color::NONE
                },
                "mask={mask}"
            );
            let row = if disabled {
                colors.item.disabled
            } else if pressed {
                colors.item.pressed
            } else if hovered {
                colors.item.hovered
            } else if selected {
                colors.item.selected
            } else {
                colors.item.normal
            };
            assert_eq!(style.foreground, row.foreground, "mask={mask}");
            assert_eq!(
                style.border,
                if active {
                    if disabled {
                        colors.item.disabled_active_border
                    } else {
                        colors.item.active_border
                    }
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
        assert_eq!(
            root_border(&colors, false, false),
            colors.container.normal.border
        );
        assert_eq!(
            root_border(&colors, false, true),
            colors.container.focused.border
        );
        assert_eq!(
            root_border(&colors, true, false),
            colors.container.disabled.border
        );
        assert_eq!(
            root_border(&colors, true, true),
            colors.container.disabled.border
        );
    }
}
