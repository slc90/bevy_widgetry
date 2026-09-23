use crate::checkbox::WidgetryCheckBox;
use crate::indicator::{CheckBoxIndicator, CheckBoxMark};
use crate::tri_state::{WidgetryCheckState, WidgetryTriStateCheckbox};
use bevy::app::Propagate;
use bevy::picking::hover::Hovered;
use bevy::prelude::*;
use bevy::ui::{BorderColor, Checked, InteractionDisabled, Pressed};
use bevy_widgetry_asset::BuiltinIcon;
use bevy_widgetry_core::icon::WidgetryIcon;
use bevy_widgetry_core::{ColorTheme, ForegroundColor, ThemeMode};

/// 统一二态与三态的视觉输入，颜色只区分是否 active。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum CheckBoxVisualState {
    Unchecked,
    Checked,
    Indeterminate,
}

/// 一次解析得到 indicator 与 mark 的完整配色。
struct CheckBoxStyle {
    /// Indicator 的背景色。
    background: Color,
    /// Indicator 的 border 色。
    border: Color,
    /// Root 向 label 传播的 foreground 色。
    foreground: Color,
    /// 可见 mark 的显式颜色。
    mark: Color,
}

/// 按 disabled、pressed、hovered、active、normal 选择配色。
fn resolve_style(
    colors: &ColorTheme,
    state: CheckBoxVisualState,
    hovered: bool,
    pressed: bool,
    disabled: bool,
) -> CheckBoxStyle {
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
    } else if state != CheckBoxVisualState::Unchecked {
        (
            colors.control_background_active,
            colors.control_border_active,
        )
    } else {
        (colors.control_background, colors.control_border)
    };
    CheckBoxStyle {
        background,
        border,
        foreground: if disabled {
            colors.foreground_disabled
        } else {
            colors.foreground
        },
        mark: if disabled {
            colors.foreground_disabled
        } else {
            colors.control_border_active
        },
    }
}

/// 从真实 state 同步 root foreground、indicator 配色与唯一 mark 的 SVG。
pub(crate) fn update_style(
    mode: Res<ThemeMode>,
    roots: Query<
        (
            &Hovered,
            Has<Pressed>,
            Has<InteractionDisabled>,
            Has<Checked>,
            Option<&WidgetryCheckState>,
            &Children,
            &mut Propagate<ForegroundColor>,
        ),
        Or<(With<WidgetryCheckBox>, With<WidgetryTriStateCheckbox>)>,
    >,
    mut indicators: Query<
        (&Children, &mut BackgroundColor, &mut BorderColor),
        With<CheckBoxIndicator>,
    >,
    mut marks: Query<(&mut CheckBoxMark, &mut Visibility, &mut WidgetryIcon)>,
    server: Res<AssetServer>,
) {
    for (hovered, pressed, disabled, checked, tri_state, children, mut foreground) in roots {
        let state = match tri_state.copied() {
            Some(WidgetryCheckState::Indeterminate) => CheckBoxVisualState::Indeterminate,
            Some(WidgetryCheckState::Checked) => CheckBoxVisualState::Checked,
            Some(WidgetryCheckState::Unchecked) => CheckBoxVisualState::Unchecked,
            None if checked => CheckBoxVisualState::Checked,
            None => CheckBoxVisualState::Unchecked,
        };
        let style = resolve_style(mode.colors(), state, hovered.0, pressed, disabled);
        if foreground.0 != ForegroundColor(style.foreground) {
            foreground.0 = ForegroundColor(style.foreground);
        }
        for child in children.iter() {
            let Ok((mark_children, mut background, mut border)) = indicators.get_mut(child) else {
                continue;
            };
            if background.0 != style.background {
                background.0 = style.background;
            }
            if *border != BorderColor::all(style.border) {
                *border = BorderColor::all(style.border);
            }
            for mark_entity in mark_children.iter() {
                let Ok((mut mark, mut visibility, mut icon)) = marks.get_mut(mark_entity) else {
                    continue;
                };
                let desired = match state {
                    CheckBoxVisualState::Unchecked => None,
                    CheckBoxVisualState::Checked => Some(BuiltinIcon::CheckboxCheck),
                    CheckBoxVisualState::Indeterminate => Some(BuiltinIcon::CheckboxIndeterminate),
                };
                let target_visibility = if desired.is_some() {
                    Visibility::Inherited
                } else {
                    Visibility::Hidden
                };
                if *visibility != target_visibility {
                    *visibility = target_visibility;
                }
                if let Some(icon_id) = desired {
                    if mark.icon != Some(icon_id) {
                        icon.set_svg(&server, icon_id.path());
                        mark.icon = Some(icon_id);
                    }
                    if mark.color != Some(style.mark) {
                        icon.set_color(style.mark);
                        mark.color = Some(style.mark);
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{WidgetryCheckBoxPlugin, WidgetryTriStateCheckbox};
    use bevy_widgetry_test_utils::scene_app;

    /// 相同 theme 中通过 state 优先级选择颜色，disabled 覆盖 pressed、hovered 和 active。
    #[test]
    fn style_priority() {
        let colors = ThemeMode::Dark.colors();
        assert_eq!(
            resolve_style(colors, CheckBoxVisualState::Checked, true, true, true).background,
            colors.control_background_disabled
        );
        assert_eq!(
            resolve_style(
                colors,
                CheckBoxVisualState::Indeterminate,
                true,
                true,
                false
            )
            .background,
            colors.control_background_pressed
        );
        assert_eq!(
            resolve_style(colors, CheckBoxVisualState::Checked, true, false, false).background,
            colors.control_background_hovered
        );
        assert_eq!(
            resolve_style(
                colors,
                CheckBoxVisualState::Indeterminate,
                false,
                false,
                false
            )
            .background,
            colors.control_background_active
        );
        assert_eq!(
            resolve_style(colors, CheckBoxVisualState::Unchecked, false, false, false).background,
            colors.control_background
        );
    }

    /// 三态在同一个 mark entity 上切换内建 SVG，恢复 Unchecked 时仅隐藏 mark。
    #[test]
    fn mark_svg_switches_without_replacing_entity() {
        let mut app = scene_app();
        app.add_plugins(WidgetryCheckBoxPlugin);
        let root = app
            .world_mut()
            .spawn_scene(bsn! { @WidgetryTriStateCheckbox })
            .expect("test entity exists")
            .id();
        app.update();
        let indicator = app
            .world()
            .get::<Children>(root)
            .expect("test entity exists")[0];
        let mark = app
            .world()
            .get::<Children>(indicator)
            .expect("test entity exists")[0];
        for (state, icon) in [
            (WidgetryCheckState::Checked, BuiltinIcon::CheckboxCheck),
            (
                WidgetryCheckState::Indeterminate,
                BuiltinIcon::CheckboxIndeterminate,
            ),
        ] {
            WidgetryTriStateCheckbox::set_state(&mut app.world_mut().commands(), root, state);
            app.update();
            assert_eq!(
                app.world()
                    .get::<CheckBoxMark>(mark)
                    .expect("test entity exists")
                    .icon,
                Some(icon)
            );
            assert_eq!(
                app.world()
                    .get::<Children>(indicator)
                    .expect("test entity exists")[0],
                mark
            );
        }
        WidgetryTriStateCheckbox::set_state(
            &mut app.world_mut().commands(),
            root,
            WidgetryCheckState::Unchecked,
        );
        app.update();
        assert_eq!(
            app.world().get::<Visibility>(mark),
            Some(&Visibility::Hidden)
        );
        assert_eq!(
            app.world()
                .get::<Children>(indicator)
                .expect("test entity exists")[0],
            mark
        );
    }
}
