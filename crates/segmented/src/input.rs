use crate::{WidgetrySegmented, WidgetrySegmentedOption, group};
use bevy::input::{ButtonState, keyboard::KeyboardInput};
use bevy::input_focus::FocusedInput;
use bevy::picking::{events::PointerClick, pointer::PointerButton};
use bevy::prelude::*;
use bevy::ui::{Checked, InteractionDisabled};
use bevy::ui_widgets::ValueChange;
use bevy_widgetry_core::exclusive_selection::SelectionHistory;
use bevy_widgetry_internal_log::widgetry_error;

pub(crate) fn on_click(
    mut event: On<PointerClick>,
    options: Query<(&ChildOf, Has<InteractionDisabled>), With<WidgetrySegmentedOption>>,
    groups: Query<(), With<WidgetrySegmented>>,
    mut commands: Commands,
) {
    let Ok((parent, disabled)) = options.get(event.entity) else {
        return;
    };
    event.propagate(false);
    if event.button != PointerButton::Primary || disabled || !groups.contains(parent.parent()) {
        return;
    }
    let root = parent.parent();
    let target = event.entity;
    commands.queue(move |world: &mut World| commit(world, root, target));
}

pub(crate) fn on_key(
    mut event: On<FocusedInput<KeyboardInput>>,
    groups: Query<&Children, (With<WidgetrySegmented>, Without<InteractionDisabled>)>,
    options: Query<(Has<Checked>, Has<InteractionDisabled>), With<WidgetrySegmentedOption>>,
    mut commands: Commands,
) {
    let Ok(children) = groups.get(event.focused_entity) else {
        return;
    };
    if event.input.state != ButtonState::Pressed || event.input.repeat {
        return;
    }
    let backwards = match event.input.key_code {
        KeyCode::ArrowLeft => true,
        KeyCode::ArrowRight => false,
        _ => return,
    };
    event.propagate(false);
    let Some(current) = children
        .iter()
        .position(|child| options.get(child).is_ok_and(|(checked, _)| checked))
    else {
        return;
    };
    let enabled = |index: &usize| {
        options
            .get(children[*index])
            .is_ok_and(|(_, disabled)| !disabled)
    };
    let next = if backwards {
        (0..current).rev().find(enabled)
    } else {
        (current + 1..children.len()).find(enabled)
    };
    if let Some(index) = next {
        let target = children[index];
        let root = event.focused_entity;
        commands.queue(move |world: &mut World| commit(world, root, target));
    }
}

fn commit(world: &mut World, root: Entity, target: Entity) -> Result<(), BevyError> {
    if world.get::<WidgetrySegmented>(root).is_none()
        || world
            .get::<ChildOf>(target)
            .is_none_or(|parent| parent.parent() != root)
        || world.get::<InteractionDisabled>(root).is_some()
        || world.get::<InteractionDisabled>(target).is_some()
    {
        return Ok(());
    }
    let children = world
        .get::<Children>(root)
        .map(|children| children.to_vec())
        .unwrap_or_default();
    let Some(index) = children.iter().position(|child| *child == target) else {
        return Ok(());
    };
    group::selection_target(world, root, index)
        .inspect_err(|error| widgetry_error!(?root, ?target, %error, "Segmented 用户选择失败"))?;
    group::initialize_group(world, root)?;
    if world.get::<Checked>(target).is_some() {
        return Ok(());
    }
    SelectionHistory::select(world, &children, target)
        .inspect_err(|error| widgetry_error!(?root, ?target, %error, "Segmented 用户选择失败"))?;
    world.trigger(ValueChange {
        source: root,
        value: index,
        is_final: true,
    });
    Ok(())
}
