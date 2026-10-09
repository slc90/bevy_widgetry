use bevy_widgetry_theme::{WidgetryThemeChanged, WidgetryThemeMode};

use crate::checkbox::WidgetryCheckBox;
use crate::indicator::{CheckBoxIndicator, CheckBoxMark};
use crate::tri_state::{WidgetryCheckState, WidgetryTriStateCheckbox};
use bevy::picking::hover::Hovered;
use bevy::prelude::*;
use bevy::ui::{BorderColor, Checked, InteractionDisabled, Pressed};
use bevy_widgetry_asset::BuiltinIcon;
use bevy_widgetry_core::diagnostics::FailureState;
use bevy_widgetry_core::foreground::ResolvedForeground;
use bevy_widgetry_core::icon::WidgetryIcon;
use bevy_widgetry_log::{widgetry_error, widgetry_info};
#[derive(Component, Default)]
pub(crate) struct StyleDiagnostics {
    mark: FailureState,
    colors: FailureState,
}

type RootStyleData = (
    Entity,
    &'static crate::colors::ColorState,
    &'static Hovered,
    Has<Pressed>,
    Has<InteractionDisabled>,
    Has<Checked>,
    Option<&'static WidgetryCheckState>,
    &'static Children,
    &'static mut ResolvedForeground,
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

fn visual_state(checked: bool, tri_state: Option<&WidgetryCheckState>) -> CheckBoxVisualState {
    match tri_state.copied() {
        Some(WidgetryCheckState::Indeterminate) => CheckBoxVisualState::Indeterminate,
        Some(WidgetryCheckState::Checked) => CheckBoxVisualState::Checked,
        Some(WidgetryCheckState::Unchecked) => CheckBoxVisualState::Unchecked,
        None if checked => CheckBoxVisualState::Checked,
        None => CheckBoxVisualState::Unchecked,
    }
}

pub(crate) fn update_mark_geometry(
    changed: Query<
        Entity,
        Or<(
            Added<WidgetryCheckBox>,
            Added<WidgetryTriStateCheckbox>,
            Added<Checked>,
            Changed<WidgetryCheckState>,
        )>,
    >,
    mut removed: RemovedComponents<Checked>,
    mut roots: Query<
        (
            &Children,
            Has<Checked>,
            Option<&WidgetryCheckState>,
            &mut StyleDiagnostics,
        ),
        Or<(With<WidgetryCheckBox>, With<WidgetryTriStateCheckbox>)>,
    >,
    indicators: Query<&Children, With<CheckBoxIndicator>>,
    mut marks: Query<(&mut CheckBoxMark, &mut Visibility), With<WidgetryIcon>>,
    mut commands: Commands,
) -> Result<(), BevyError> {
    let mut failure = None;
    for root in changed.iter().chain(removed.read()) {
        let Ok((children, checked, tri_state, mut diagnostics)) = roots.get_mut(root) else {
            continue;
        };
        let result = (|| -> Result<(), BevyError> {
            let indicator = children
                .iter()
                .find(|child| indicators.contains(*child))
                .ok_or_else(|| BevyError::error("CheckBox missing indicator"))?;
            let mark_entity = indicators
                .get(indicator)?
                .iter()
                .find(|child| marks.contains(*child))
                .ok_or_else(|| BevyError::error("CheckBox indicator missing mark"))?;
            let (mut mark, mut visibility) = marks.get_mut(mark_entity)?;
            let desired = match visual_state(checked, tri_state) {
                CheckBoxVisualState::Unchecked => None,
                CheckBoxVisualState::Checked => Some(BuiltinIcon::CheckboxCheck),
                CheckBoxVisualState::Indeterminate => Some(BuiltinIcon::CheckboxIndeterminate),
            };
            visibility.set_if_neq(if desired.is_some() {
                Visibility::Inherited
            } else {
                Visibility::Hidden
            });
            if let Some(icon) = desired
                && mark.icon != Some(icon)
            {
                WidgetryIcon::set_svg(&mut commands, mark_entity, icon.path());
                mark.icon = Some(icon);
            }
            Ok(())
        })();
        if let Err(error) = diagnostics.mark.observe(
            result,
            |error| widgetry_error!(?root, %error, "CheckBox mark 内部结构失效"),
            || widgetry_info!(?root, "CheckBox mark 恢复正常"),
        ) && failure.is_none()
        {
            failure = Some(error);
        }
    }
    failure.map_or(Ok(()), Err)
}

fn resolve_style(
    colors: &bevy_widgetry_theme::WidgetryCheckBoxColors,
    state: CheckBoxVisualState,
    hovered: bool,
    pressed: bool,
    disabled: bool,
) -> CheckBoxStyle {
    let colors = match state {
        CheckBoxVisualState::Unchecked => colors.unchecked,
        CheckBoxVisualState::Checked => colors.checked,
        CheckBoxVisualState::Indeterminate => colors.indeterminate,
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
    colors: &bevy_widgetry_theme::WidgetryCheckBoxColors,
    (
        root,
        overrides,
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
    marks: &mut Query<&mut CheckBoxMark, With<WidgetryIcon>>,
    commands: &mut Commands,
) -> Result<(), BevyError> {
    let result = (|| -> Result<(), BevyError> {
        let state = visual_state(checked, tri_state);
        let colors = overrides.0.resolve(colors);
        let style = resolve_style(&colors, state, hovered.0, pressed, disabled);
        if *foreground != ResolvedForeground(style.foreground) {
            foreground.set_if_neq(ResolvedForeground(style.foreground));
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
        let Ok(mut mark) = marks.get_mut(mark_entity) else {
            return Err(BevyError::error("CheckBox mark missing style"));
        };
        if background.0 != style.background {
            background.0 = style.background;
        }
        if *border != BorderColor::all(style.border) {
            *border = BorderColor::all(style.border);
        }
        if state != CheckBoxVisualState::Unchecked && mark.color != Some(style.mark) {
            commands
                .entity(mark_entity)
                .insert(ResolvedForeground(style.mark));
            mark.color = Some(style.mark);
        }
        Ok(())
    })();
    diagnostics.colors.observe(
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
            Without<bevy_widgetry_core::color::WidgetryStyleOwner<WidgetryCheckBox>>,
            Or<(
                Added<WidgetryCheckBox>,
                Added<WidgetryTriStateCheckbox>,
                Changed<Hovered>,
                Changed<crate::colors::ColorState>,
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
    mut marks: Query<&mut CheckBoxMark, With<WidgetryIcon>>,
    mut commands: Commands,
) -> Result<(), BevyError> {
    let mut failure = None;
    for item in &mut roots {
        if let Err(error) = apply_style(
            &mode.colors().check_box,
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
    mut roots: Query<
        RootStyleData,
        (
            Or<(With<WidgetryCheckBox>, With<WidgetryTriStateCheckbox>)>,
            Without<bevy_widgetry_core::color::WidgetryStyleOwner<WidgetryCheckBox>>,
        ),
    >,
    mut indicators: Query<
        (&Children, &mut BackgroundColor, &mut BorderColor),
        With<CheckBoxIndicator>,
    >,
    mut marks: Query<&mut CheckBoxMark, With<WidgetryIcon>>,
    mut commands: Commands,
) -> Result<(), BevyError> {
    let mut failure = None;
    for entity in pressed.read().chain(checked.read()).chain(disabled.read()) {
        if let Ok(item) = roots.get_mut(entity)
            && let Err(error) = apply_style(
                &mode.colors().check_box,
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

pub(crate) fn refresh_theme(_event: On<WidgetryThemeChanged>, mut commands: Commands) {
    commands.queue(|world: &mut World| -> Result<(), BevyError> {
        let theme = world.resource::<WidgetryThemeMode>().colors().check_box;
        let roots = world
            .query_filtered::<Entity, (
                Or<(With<WidgetryCheckBox>, With<WidgetryTriStateCheckbox>)>,
                Without<bevy_widgetry_core::color::WidgetryStyleOwner<WidgetryCheckBox>>,
            )>()
            .iter(world)
            .collect::<Vec<_>>();
        let mut failure = None;
        for root in roots {
            let colors = crate::WidgetryCheckBoxColorOverrides::get(world, root)?.resolve(&theme);
            if let Err(error) = apply_owned_checkbox_colors(world, root, &colors)
                && failure.is_none()
            {
                failure = Some(error);
            }
        }
        failure.map_or(Ok(()), Err)
    });
}

pub fn apply_owned_checkbox_colors(
    world: &mut World,
    root: Entity,
    colors: &bevy_widgetry_theme::WidgetryCheckBoxColors,
) -> Result<(), BevyError> {
    let mut state = bevy::ecs::system::SystemState::<(
        Query<RootStyleData>,
        Query<(&Children, &mut BackgroundColor, &mut BorderColor), With<CheckBoxIndicator>>,
        Query<&mut CheckBoxMark, With<WidgetryIcon>>,
        Commands,
    )>::new(world);
    let (mut roots, mut indicators, mut marks, mut commands) =
        state.get_mut(world).map_err(|error| {
            widgetry_error!(?root, %error, "CheckBox 颜色查询失败");
            BevyError::error(error.to_string())
        })?;
    let result = roots
        .get_mut(root)
        .map_err(|_| {
            widgetry_error!(?root, "CheckBox 缺失托管颜色主体");
            BevyError::error("CheckBox 缺失托管颜色主体")
        })
        .and_then(|item| apply_style(colors, item, &mut indicators, &mut marks, &mut commands));
    state.apply(world);
    result
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
            let style = resolve_style(&colors.check_box, state, hover, press, disabled);
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
        app.edit_schedule(PostUpdate, |schedule| {
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
