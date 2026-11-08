use crate::indicator::checkbox_indicator_scene;
use crate::{CheckBoxError, CheckBoxFailure, error::failure};
use accesskit::{Role, Toggled};
use bevy::a11y::AccessibilityNode;
use bevy::input::{
    ButtonState,
    keyboard::{KeyCode, KeyboardInput},
};
use bevy::input_focus::tab_navigation::TabIndex;
use bevy::input_focus::{FocusCause, FocusedInput, InputFocus, InputFocusVisible};
use bevy::picking::events::{
    PointerCancel, PointerClick, PointerDragEnd, PointerPress, PointerRelease,
};
use bevy::picking::hover::Hovered;
use bevy::prelude::*;
use bevy::ui::{BackgroundColor, BorderColor, InteractionDisabled, Pressed};
use bevy::ui_widgets::{ActivateOnPress, ValueChange};
use bevy_widgetry_core::foreground::ResolvedForeground;

#[derive(Component, Default, Clone, Copy, Debug, PartialEq, Eq, Reflect)]
#[component(immutable)]
#[reflect(Component)]
pub enum WidgetryCheckState {
    #[default]
    Unchecked,
    Checked,
    Indeterminate,
}

#[derive(SceneComponent, Default, Clone)]
#[require(Node, WidgetryCheckState, AccessibilityNode(accesskit::Node::new(Role::CheckBox)), Hovered, bevy_widgetry_core::pointer::WidgetryPointerPressed, TabIndex(-1))]
pub struct WidgetryTriStateCheckboxHeadless;

impl WidgetryTriStateCheckboxHeadless {
    fn scene() -> impl Scene {
        bsn! {}
    }

    pub fn set_state(commands: &mut Commands, entity: Entity, state: WidgetryCheckState) {
        commands.queue(move |world: &mut World| {
            write_state(world, entity, Some(state)).map_err(|error| error.into_bevy_error())
        });
    }

    pub fn cycle_state(commands: &mut Commands, entity: Entity) {
        commands.queue(move |world: &mut World| {
            write_state(world, entity, None).map_err(|error| error.into_bevy_error())
        });
    }
}

pub struct WidgetryTriStateCheckboxHeadlessPlugin;

impl Plugin for WidgetryTriStateCheckboxHeadlessPlugin {
    fn build(&self, app: &mut App) {
        if !app.is_plugin_added::<bevy_widgetry_core::ui::WidgetryUiPlugin>() {
            app.add_plugins(bevy_widgetry_core::ui::WidgetryUiPlugin);
        }
        app.register_type::<WidgetryCheckState>()
            .add_observer(on_press)
            .add_observer(on_click)
            .add_observer(on_release)
            .add_observer(on_drag_end)
            .add_observer(on_cancel)
            .add_observer(on_key)
            .add_systems(
                PostUpdate,
                sync_accessibility
                    .after(bevy_widgetry_core::ui::WidgetryUiSystems::Disabled)
                    .before(bevy_widgetry_core::ui::WidgetryUiSystems::Colors),
            );
        bevy_widgetry_internal_log::widgetry_debug!(
            "WidgetryTriStateCheckboxHeadlessPlugin 注册完成"
        );
    }
}

#[derive(SceneComponent, Default, Clone)]
#[scene(crate::WidgetryCheckBoxProps)]
#[require(
    WidgetryTriStateCheckboxHeadless,
    crate::style::StyleDiagnostics,
    crate::colors::ColorState
)]
pub struct WidgetryTriStateCheckbox;

fn next_check_state(state: WidgetryCheckState) -> WidgetryCheckState {
    match state {
        WidgetryCheckState::Unchecked => WidgetryCheckState::Checked,
        WidgetryCheckState::Checked => WidgetryCheckState::Indeterminate,
        WidgetryCheckState::Indeterminate => WidgetryCheckState::Unchecked,
    }
}

fn write_state(
    world: &mut World,
    entity: Entity,
    target: Option<WidgetryCheckState>,
) -> Result<(), CheckBoxFailure> {
    if world
        .get::<WidgetryTriStateCheckboxHeadless>(entity)
        .is_none()
    {
        return Err(failure(
            CheckBoxError::InvalidTarget { root: entity },
            "check_box.write_state",
        ));
    }
    let current = world
        .get::<WidgetryCheckState>(entity)
        .copied()
        .ok_or_else(|| {
            failure(
                CheckBoxError::MissingPart {
                    root: entity,
                    entity,
                    part: "WidgetryCheckState",
                },
                "check_box.write_state",
            )
        })?;
    let next = target.unwrap_or_else(|| next_check_state(current));
    if next != current {
        world.entity_mut(entity).insert(next);
        world.trigger(ValueChange {
            source: entity,
            value: next,
            is_final: true,
        });
    }
    Ok(())
}

fn on_press(
    mut event: On<PointerPress>,
    query: Query<
        (
            &WidgetryCheckState,
            Has<InteractionDisabled>,
            Has<Pressed>,
            Has<ActivateOnPress>,
        ),
        With<WidgetryTriStateCheckboxHeadless>,
    >,
    focus: Option<ResMut<InputFocus>>,
    focus_visible: Option<ResMut<InputFocusVisible>>,
    mut commands: Commands,
) {
    let Ok((_state, disabled, pressed, activate_on_press)) = query.get(event.entity) else {
        return;
    };
    if let Some(mut focus) = focus {
        focus.set(event.entity, FocusCause::Pressed);
    }
    if let Some(mut visible) = focus_visible {
        visible.0 = false;
    }
    event.propagate(false);
    if !disabled && !pressed {
        commands.entity(event.entity).insert(Pressed);
        if activate_on_press {
            WidgetryTriStateCheckboxHeadless::cycle_state(&mut commands, event.entity);
        }
    }
}

fn on_click(
    mut event: On<PointerClick>,
    query: Query<
        (&WidgetryCheckState, Has<InteractionDisabled>),
        (
            With<WidgetryTriStateCheckboxHeadless>,
            Without<ActivateOnPress>,
        ),
    >,
    mut commands: Commands,
) {
    let Ok((_state, disabled)) = query.get(event.entity) else {
        return;
    };
    event.propagate(false);
    if !disabled {
        WidgetryTriStateCheckboxHeadless::cycle_state(&mut commands, event.entity);
    }
}

fn on_release(
    mut event: On<PointerRelease>,
    query: Query<(), With<WidgetryTriStateCheckboxHeadless>>,
    mut commands: Commands,
) {
    if query.contains(event.entity) {
        event.propagate(false);
        commands.entity(event.entity).remove::<Pressed>();
    }
}

fn on_drag_end(
    mut event: On<PointerDragEnd>,
    query: Query<(), With<WidgetryTriStateCheckboxHeadless>>,
    mut commands: Commands,
) {
    if query.contains(event.entity) {
        event.propagate(false);
        commands.entity(event.entity).remove::<Pressed>();
    }
}

fn on_cancel(
    mut event: On<PointerCancel>,
    query: Query<(), With<WidgetryTriStateCheckboxHeadless>>,
    mut commands: Commands,
) {
    if query.contains(event.entity) {
        event.propagate(false);
        commands.entity(event.entity).remove::<Pressed>();
    }
}

fn on_key(
    mut event: On<FocusedInput<KeyboardInput>>,
    query: Query<
        &WidgetryCheckState,
        (
            With<WidgetryTriStateCheckboxHeadless>,
            Without<InteractionDisabled>,
        ),
    >,
    mut commands: Commands,
) {
    let Ok(_state) = query.get(event.focused_entity) else {
        return;
    };
    let input = &event.event().input;
    if input.state == ButtonState::Pressed
        && !input.repeat
        && matches!(input.key_code, KeyCode::Enter | KeyCode::Space)
    {
        event.propagate(false);
        WidgetryTriStateCheckboxHeadless::cycle_state(&mut commands, event.focused_entity);
    }
}

fn sync_accessibility(
    mut query: Query<
        (&WidgetryCheckState, &mut AccessibilityNode),
        (
            With<WidgetryTriStateCheckboxHeadless>,
            Or<(
                Changed<WidgetryCheckState>,
                Added<AccessibilityNode>,
                Added<WidgetryTriStateCheckboxHeadless>,
            )>,
        ),
    >,
) {
    for (state, mut node) in &mut query {
        node.set_toggled(match state {
            WidgetryCheckState::Unchecked => Toggled::False,
            WidgetryCheckState::Checked => Toggled::True,
            WidgetryCheckState::Indeterminate => Toggled::Mixed,
        });
    }
}

impl WidgetryTriStateCheckbox {
    fn scene(props: crate::WidgetryCheckBoxProps) -> impl Scene {
        bsn! {
            template(move |context| props.colors.clone().initial(context.entity.id()).map_err(|error| error.into_bevy_error()))
            @WidgetryTriStateCheckboxHeadless
            template(|_| Ok(bevy::picking::cursor::EntityCursor::System(bevy::window::SystemCursorIcon::Pointer)))
            Node { flex_direction: FlexDirection::Row, align_items: AlignItems::Center, column_gap: px(crate::checkbox::LABEL_GAP), min_height: px(crate::checkbox::ROW_MIN_HEIGHT) }
            BackgroundColor
            BorderColor
            template(|_| Ok(ResolvedForeground::default()))
            Children [@checkbox_indicator_scene()]
        }
    }
}

#[cfg(test)]
#[allow(clippy::disallowed_macros)]
mod tests {
    use super::*;

    #[test]
    fn next_state_follows_three_state_cycle() {
        for (from, to) in [
            (WidgetryCheckState::Unchecked, WidgetryCheckState::Checked),
            (
                WidgetryCheckState::Checked,
                WidgetryCheckState::Indeterminate,
            ),
            (
                WidgetryCheckState::Indeterminate,
                WidgetryCheckState::Unchecked,
            ),
        ] {
            assert_eq!(next_check_state(from), to);
        }
    }
}
