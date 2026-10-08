use crate::indicator::checkbox_indicator_scene;
use accesskit::{Role, Toggled};
use bevy::a11y::AccessibilityNode;
use bevy::app::Propagate;
use bevy::input::{
    ButtonState,
    keyboard::{KeyCode, KeyboardInput},
};
use bevy::input_focus::tab_navigation::TabIndex;
use bevy::input_focus::{FocusCause, FocusedInput, InputFocus, InputFocusVisible};
use bevy::picking::events::{Cancel, Click, DragEnd, Pointer, Press, Release};
use bevy::picking::hover::Hovered;
use bevy::prelude::*;
use bevy::ui::{BackgroundColor, BorderColor, InteractionDisabled, Pressed};
use bevy::ui_widgets::{ActivateOnPress, ValueChange};
use bevy_widgetry_core::ForegroundColor;
use bevy_widgetry_log::widgetry_error;

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
#[require(crate::style::StyleDiagnostics)]
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
) -> Result<(), BevyError> {
    if world.get::<WidgetryTriStateCheckbox>(entity).is_none() {
        widgetry_error!(?entity, "三态 CheckBox 更新目标不存在或不是三态 Widget");
        return Err(BevyError::error(
            "三态 CheckBox 更新目标不存在或不是三态 Widget",
        ));
    }
    let Some(current) = world.get::<WidgetryCheckState>(entity).copied() else {
        widgetry_error!(?entity, "三态 CheckBox 缺失必需 state");
        return Err(BevyError::error("三态 CheckBox 缺失必需 state"));
    };
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

pub(crate) fn on_press(
    mut event: On<Pointer<Press>>,
    query: Query<
        (
            &WidgetryCheckState,
            Has<InteractionDisabled>,
            Has<Pressed>,
            Has<ActivateOnPress>,
        ),
        With<WidgetryTriStateCheckbox>,
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
            WidgetryTriStateCheckbox::cycle_state(&mut commands, event.entity);
        }
    }
}

pub(crate) fn on_click(
    mut event: On<Pointer<Click>>,
    query: Query<
        (&WidgetryCheckState, Has<InteractionDisabled>),
        (With<WidgetryTriStateCheckbox>, Without<ActivateOnPress>),
    >,
    mut commands: Commands,
) {
    let Ok((_state, disabled)) = query.get(event.entity) else {
        return;
    };
    event.propagate(false);
    if !disabled {
        WidgetryTriStateCheckbox::cycle_state(&mut commands, event.entity);
    }
}

pub(crate) fn on_release(
    mut event: On<Pointer<Release>>,
    query: Query<(), With<WidgetryTriStateCheckbox>>,
    mut commands: Commands,
) {
    if query.contains(event.entity) {
        event.propagate(false);
        commands.entity(event.entity).remove::<Pressed>();
    }
}

pub(crate) fn on_drag_end(
    mut event: On<Pointer<DragEnd>>,
    query: Query<(), With<WidgetryTriStateCheckbox>>,
    mut commands: Commands,
) {
    if query.contains(event.entity) {
        event.propagate(false);
        commands.entity(event.entity).remove::<Pressed>();
    }
}

pub(crate) fn on_cancel(
    mut event: On<Pointer<Cancel>>,
    query: Query<(), With<WidgetryTriStateCheckbox>>,
    mut commands: Commands,
) {
    if query.contains(event.entity) {
        event.propagate(false);
        commands.entity(event.entity).remove::<Pressed>();
    }
}

pub(crate) fn on_key(
    mut event: On<FocusedInput<KeyboardInput>>,
    query: Query<
        &WidgetryCheckState,
        (With<WidgetryTriStateCheckbox>, Without<InteractionDisabled>),
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
        WidgetryTriStateCheckbox::cycle_state(&mut commands, event.focused_entity);
    }
}

pub(crate) fn sync_accessibility(
    mut query: Query<(&WidgetryCheckState, &mut AccessibilityNode), Changed<WidgetryCheckState>>,
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
    fn scene() -> impl Scene {
        bsn! {
            template(|_| Ok(WidgetryCheckState::Unchecked))
            template(|_| Ok(AccessibilityNode(accesskit::Node::new(Role::CheckBox))))
            Hovered(false)
            bevy_widgetry_core::pointer::WidgetryPointerPressed
            TabIndex(-1)
            Node { flex_direction: FlexDirection::Row, align_items: AlignItems::Center, column_gap: px(6), min_height: px(24) }
            BackgroundColor
            BorderColor
            template(|_| Ok(Propagate(ForegroundColor::default())))
            Children [checkbox_indicator_scene()]
        }
    }

    pub fn set_state(commands: &mut Commands, entity: Entity, state: WidgetryCheckState) {
        commands.queue(move |world: &mut World| write_state(world, entity, Some(state)));
    }

    pub fn cycle_state(commands: &mut Commands, entity: Entity) {
        commands.queue(move |world: &mut World| write_state(world, entity, None));
    }
}

// 测试断言需要在 contract 不满足时立即失败。
// 生产代码的 panic lint 会拒绝这些表达式，因此仅在本测试 scope 允许所列 lint。
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
