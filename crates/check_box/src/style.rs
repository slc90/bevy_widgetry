use bevy_widgetry_theme::{WidgetryTheme, WidgetryThemeChanged, WidgetryThemeMode};

use crate::checkbox::WidgetryCheckBox;
use crate::indicator::{CheckBoxIndicator, CheckBoxMark};
use crate::tri_state::{WidgetryCheckState, WidgetryTriStateCheckbox};
use bevy::app::Propagate;
use bevy::picking::hover::Hovered;
use bevy::prelude::*;
use bevy::ui::{BorderColor, Checked, InteractionDisabled, Pressed};
use bevy_widgetry_asset::BuiltinIcon;
use bevy_widgetry_core::ForegroundColor;
use bevy_widgetry_core::diagnostics::FailureState;
use bevy_widgetry_core::icon::WidgetryIcon;
use bevy_widgetry_log::{widgetry_error, widgetry_info};
#[derive(Component, Default)]
pub(crate) struct StyleDiagnostics(FailureState);

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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum CheckBoxVisualState {
    Unchecked,
    Checked,
    Indeterminate,
}

struct CheckBoxStyle {
    background: Color,
    border: Color,
    foreground: Color,
    mark: Color,
}

fn resolve_style(
    colors: &WidgetryTheme,
    state: CheckBoxVisualState,
    hovered: bool,
    pressed: bool,
    disabled: bool,
) -> CheckBoxStyle {
    let colors = match state {
        CheckBoxVisualState::Unchecked => colors.check_box.unchecked,
        CheckBoxVisualState::Checked => colors.check_box.checked,
        CheckBoxVisualState::Indeterminate => colors.check_box.indeterminate,
    };
    let colors = if disabled {
        colors.disabled
    } else if pressed {
        colors.pressed
    } else if hovered {
        colors.hovered
    } else {
        colors.normal
    };
    CheckBoxStyle {
        background: colors.background,
        border: colors.border,
        foreground: colors.foreground,
        mark: colors.mark,
    }
}

fn apply_style(
    colors: &WidgetryTheme,
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

pub(crate) fn update_changed(
    mode: Res<WidgetryThemeMode>,
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

pub(crate) fn update_removed(
    mode: Res<WidgetryThemeMode>,
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

pub(crate) fn refresh_theme(
    event: On<WidgetryThemeChanged>,
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

// 测试断言需要在 contract 不满足时立即失败。
// 生产代码的 panic lint 会拒绝这些表达式，因此仅在本测试 scope 允许所列 lint。
#[cfg(test)]
#[allow(clippy::disallowed_macros)]
mod tests {
    use super::*;
    use crate::{WidgetryCheckBoxPlugin, WidgetryTriStateCheckbox};
    use bevy_widgetry_test_utils::{LogCapture, scene_app};

    #[test]
    fn style_priority() {
        let colors = WidgetryThemeMode::Dark.colors();
        for (state, hover, press, disabled, background, border) in [
            (
                CheckBoxVisualState::Checked,
                true,
                true,
                true,
                colors.check_box.unchecked.disabled.background,
                colors.check_box.unchecked.disabled.border,
            ),
            (
                CheckBoxVisualState::Indeterminate,
                true,
                true,
                false,
                colors.check_box.indeterminate.pressed.background,
                colors.check_box.indeterminate.pressed.border,
            ),
            (
                CheckBoxVisualState::Checked,
                true,
                false,
                false,
                colors.check_box.checked.hovered.background,
                colors.check_box.checked.hovered.border,
            ),
            (
                CheckBoxVisualState::Indeterminate,
                false,
                false,
                false,
                colors.check_box.checked.normal.background,
                colors.check_box.checked.normal.border,
            ),
            (
                CheckBoxVisualState::Unchecked,
                false,
                false,
                false,
                colors.check_box.unchecked.normal.background,
                colors.check_box.unchecked.normal.border,
            ),
        ] {
            let style = resolve_style(colors, state, hover, press, disabled);
            assert_eq!(style.background, background);
            assert_eq!(style.border, border);
            assert_eq!(
                style.foreground,
                if disabled {
                    colors.check_box.unchecked.disabled.foreground
                } else {
                    colors.check_box.unchecked.normal.foreground
                }
            );
            assert_eq!(
                style.mark,
                if state == CheckBoxVisualState::Unchecked {
                    Color::NONE
                } else if disabled {
                    colors.check_box.checked.disabled.mark
                } else {
                    colors.check_box.checked.normal.mark
                }
            );
        }
    }

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
