use crate::checkbox::WidgetryCheckBox;
use crate::indicator::{CheckBoxIndicator, CheckBoxMark};
use crate::tri_state::{WidgetryCheckState, WidgetryTriStateCheckbox};
use bevy::app::Propagate;
use bevy::picking::hover::Hovered;
use bevy::prelude::*;
use bevy::ui::{BorderColor, Checked, InteractionDisabled, Pressed};
use bevy_widgetry_asset::BuiltinIcon;
use bevy_widgetry_core::diagnostics::FailureState;
use bevy_widgetry_core::icon::WidgetryIcon;
use bevy_widgetry_core::{ColorTheme, ForegroundColor, ThemeChanged, ThemeMode};
use bevy_widgetry_log::{widgetry_error, widgetry_info};

/// 此控件 style 负责异常边界，销毁时不会产生恢复日志。
#[derive(Component, Default)]
pub(crate) struct StyleDiagnostics(FailureState);

/// CheckBox root 的 state、内部结构入口和 foreground 输出。
type RootStyleData = (
    Entity,
    &'static Hovered,
    Has<Pressed>,
    Has<InteractionDisabled>,
    Has<Checked>,
    Option<&'static WidgetryCheckState>,
    &'static Children,
    &'static mut Propagate<ForegroundColor>,
    &'static mut StyleDiagnostics,
);

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

/// 从真实 state 同步单个 root 的 foreground、indicator 配色与唯一 mark 的 SVG。
fn apply_style(
    colors: &ColorTheme,
    (
        root,
        hovered,
        pressed,
        disabled,
        checked,
        tri_state,
        children,
        mut foreground,
        mut diagnostics,
    ): <RootStyleData as bevy::ecs::query::QueryData>::Item<'_, '_>,
    indicators: &mut Query<
        (&Children, &mut BackgroundColor, &mut BorderColor),
        With<CheckBoxIndicator>,
    >,
    marks: &mut Query<(&mut CheckBoxMark, &mut Visibility), With<WidgetryIcon>>,
    commands: &mut Commands,
) -> Result<(), BevyError> {
    let result = (|| -> Result<(), BevyError> {
        let state = match tri_state.copied() {
            Some(WidgetryCheckState::Indeterminate) => CheckBoxVisualState::Indeterminate,
            Some(WidgetryCheckState::Checked) => CheckBoxVisualState::Checked,
            Some(WidgetryCheckState::Unchecked) => CheckBoxVisualState::Unchecked,
            None if checked => CheckBoxVisualState::Checked,
            None => CheckBoxVisualState::Unchecked,
        };
        let style = resolve_style(colors, state, hovered.0, pressed, disabled);
        if foreground.0 != ForegroundColor(style.foreground) {
            foreground.0 = ForegroundColor(style.foreground);
        }
        let Some(indicator) = children.iter().find(|&child| indicators.contains(child)) else {
            return Err(BevyError::error("CheckBox missing indicator"));
        };
        let Ok((mark_children, mut background, mut border)) = indicators.get_mut(indicator) else {
            return Err(BevyError::error("CheckBox indicator missing style"));
        };
        let Some(mark_entity) = mark_children.iter().find(|&child| marks.contains(child)) else {
            return Err(BevyError::error("CheckBox indicator missing mark"));
        };
        let Ok((mut mark, mut visibility)) = marks.get_mut(mark_entity) else {
            return Err(BevyError::error("CheckBox mark missing style"));
        };
        if background.0 != style.background {
            background.0 = style.background;
        }
        if *border != BorderColor::all(style.border) {
            *border = BorderColor::all(style.border);
        }
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
                WidgetryIcon::set_svg(commands, mark_entity, icon_id.path());
                mark.icon = Some(icon_id);
            }
            if mark.color != Some(style.mark) {
                WidgetryIcon::set_color(commands, mark_entity, style.mark);
                mark.color = Some(style.mark);
            }
        }
        Ok(())
    })();
    diagnostics.0.observe(
        result,
        |error| widgetry_error!(?root, %error, "CheckBox style 内部结构失效"),
        || widgetry_info!(?root, "CheckBox style 恢复正常"),
    )
}

/// 新增 CheckBox 或视觉输入变化时重新解析完整配色。
pub(crate) fn update_changed(
    mode: Res<ThemeMode>,
    mut roots: Query<
        RootStyleData,
        (
            Or<(With<WidgetryCheckBox>, With<WidgetryTriStateCheckbox>)>,
            Or<(
                Added<WidgetryCheckBox>,
                Added<WidgetryTriStateCheckbox>,
                Changed<Hovered>,
                Added<Pressed>,
                Added<Checked>,
                Added<InteractionDisabled>,
                Changed<WidgetryCheckState>,
            )>,
        ),
    >,
    mut indicators: Query<
        (&Children, &mut BackgroundColor, &mut BorderColor),
        With<CheckBoxIndicator>,
    >,
    mut marks: Query<(&mut CheckBoxMark, &mut Visibility), With<WidgetryIcon>>,
    mut commands: Commands,
) -> Result<(), BevyError> {
    let mut failure = None;
    for item in &mut roots {
        if let Err(error) = apply_style(
            mode.colors(),
            item,
            &mut indicators,
            &mut marks,
            &mut commands,
        ) && failure.is_none()
        {
            failure = Some(error);
        }
    }
    failure.map_or(Ok(()), Err)
}

/// Pressed、Checked 或 disabled 移除后，按剩余 state 重新解析样式。
pub(crate) fn update_removed(
    mode: Res<ThemeMode>,
    mut pressed: RemovedComponents<Pressed>,
    mut checked: RemovedComponents<Checked>,
    mut disabled: RemovedComponents<InteractionDisabled>,
    mut roots: Query<RootStyleData, Or<(With<WidgetryCheckBox>, With<WidgetryTriStateCheckbox>)>>,
    mut indicators: Query<
        (&Children, &mut BackgroundColor, &mut BorderColor),
        With<CheckBoxIndicator>,
    >,
    mut marks: Query<(&mut CheckBoxMark, &mut Visibility), With<WidgetryIcon>>,
    mut commands: Commands,
) -> Result<(), BevyError> {
    let mut failure = None;
    for entity in pressed.read().chain(checked.read()).chain(disabled.read()) {
        if let Ok(item) = roots.get_mut(entity)
            && let Err(error) = apply_style(
                mode.colors(),
                item,
                &mut indicators,
                &mut marks,
                &mut commands,
            )
            && failure.is_none()
        {
            failure = Some(error);
        }
    }
    failure.map_or(Ok(()), Err)
}

/// ThemeChanged 后立即刷新 CheckBox，不等待下一次 Update。
pub(crate) fn refresh_theme(
    event: On<ThemeChanged>,
    mut roots: Query<RootStyleData, Or<(With<WidgetryCheckBox>, With<WidgetryTriStateCheckbox>)>>,
    mut indicators: Query<
        (&Children, &mut BackgroundColor, &mut BorderColor),
        With<CheckBoxIndicator>,
    >,
    mut marks: Query<(&mut CheckBoxMark, &mut Visibility), With<WidgetryIcon>>,
    mut commands: Commands,
) -> Result<(), BevyError> {
    let mut failure = None;
    for item in &mut roots {
        if let Err(error) = apply_style(
            event.mode.colors(),
            item,
            &mut indicators,
            &mut marks,
            &mut commands,
        ) && failure.is_none()
        {
            failure = Some(error);
        }
    }
    failure.map_or(Ok(()), Err)
}

// 测试 module 中的断言用于验证 contract，生产代码仍禁止。
#[cfg(test)]
#[allow(clippy::disallowed_macros)]
mod tests {
    use super::*;
    use crate::{WidgetryCheckBoxPlugin, WidgetryTriStateCheckbox};
    use bevy_widgetry_test_utils::{LogCapture, scene_app};

    /// 相同 theme 中通过 state 优先级选择颜色，disabled 覆盖 pressed、hovered 和 active。
    #[test]
    fn style_priority() {
        let colors = ThemeMode::Dark.colors();
        for (state, hover, press, disabled, background, border) in [
            (
                CheckBoxVisualState::Checked,
                true,
                true,
                true,
                colors.control_background_disabled,
                colors.control_border_disabled,
            ),
            (
                CheckBoxVisualState::Indeterminate,
                true,
                true,
                false,
                colors.control_background_pressed,
                colors.control_border_pressed,
            ),
            (
                CheckBoxVisualState::Checked,
                true,
                false,
                false,
                colors.control_background_hovered,
                colors.control_border_hovered,
            ),
            (
                CheckBoxVisualState::Indeterminate,
                false,
                false,
                false,
                colors.control_background_active,
                colors.control_border_active,
            ),
            (
                CheckBoxVisualState::Unchecked,
                false,
                false,
                false,
                colors.control_background,
                colors.control_border,
            ),
        ] {
            let style = resolve_style(colors, state, hover, press, disabled);
            assert_eq!(style.background, background);
            assert_eq!(style.border, border);
            assert_eq!(
                style.foreground,
                if disabled {
                    colors.foreground_disabled
                } else {
                    colors.foreground
                }
            );
            assert_eq!(
                style.mark,
                if disabled {
                    colors.foreground_disabled
                } else {
                    colors.control_border_active
                }
            );
        }
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

    /// 仅在样式输入变化时诊断损坏的 indicator 和 mark，空闲 Update 不重复记录。
    #[test]
    fn broken_internal_structure_is_logged() {
        let capture = LogCapture::default();
        let mut app = scene_app();
        app.add_plugins(WidgetryCheckBoxPlugin);
        let root = app
            .world_mut()
            .spawn_scene(bsn! { @WidgetryTriStateCheckbox })
            .unwrap()
            .id();
        app.edit_schedule(Update, |schedule| {
            schedule.set_executor(bevy::ecs::schedule::SingleThreadedExecutor::new());
        });
        app.update();
        let indicator = app.world().get::<Children>(root).unwrap()[0];
        let mark = app.world().get::<Children>(indicator).unwrap()[0];

        app.world_mut()
            .entity_mut(indicator)
            .remove::<CheckBoxIndicator>();
        app.world_mut().entity_mut(root).insert(Hovered(true));
        capture.run(|| app.update());
        capture.run(|| app.update());
        app.world_mut()
            .entity_mut(indicator)
            .insert(CheckBoxIndicator);
        app.world_mut().entity_mut(mark).remove::<CheckBoxMark>();
        app.world_mut().entity_mut(root).insert(Hovered(false));
        capture.run(|| app.update());

        let records = capture.records();
        let errors: Vec<_> = records
            .iter()
            .filter(|record| record.level == bevy::log::tracing::Level::ERROR)
            .collect();
        assert_eq!(errors.len(), 2);
        assert!(errors[0].fields.get("error").unwrap().contains("indicator"));
        assert!(errors[0].fields.contains_key("root"));
        assert!(errors[1].fields.get("error").unwrap().contains("mark"));
        assert!(errors[1].fields.contains_key("root"));
    }
}
