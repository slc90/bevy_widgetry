use bevy::{
    input::{ButtonState, keyboard::KeyboardInput},
    input_focus::{FocusedInput, tab_navigation::TabIndex},
    picking::{
        PickingSystems,
        events::{
            PointerCancel, PointerClick, PointerDrag, PointerDragEnd, PointerDragStart,
            PointerPress, PointerRelease,
        },
        hover::Hovered,
        pointer::{PointerAction, PointerButton, PointerId, PointerInput},
    },
    prelude::*,
    ui::{InteractionDisabled, Pressed},
    ui_widgets::Activate,
};
use bevy_widgetry_core::pointer::WidgetryPointerQuery;

#[derive(SceneComponent, Default, Clone)]
#[require(Node, Hovered, TabIndex(-1))]
pub struct WidgetryButtonHeadless;

#[derive(Component)]
struct PressOwner {
    pointer: PointerId,
    target: bevy::camera::NormalizedRenderTarget,
}

impl WidgetryButtonHeadless {
    fn scene() -> impl Scene {
        bsn! {}
    }
}

pub(crate) fn install(app: &mut App) {
    app.add_observer(press)
        .add_observer(click)
        .add_observer(release)
        .add_observer(cancel)
        .add_observer(drag_start)
        .add_observer(drag)
        .add_observer(drag_end)
        .add_observer(key)
        .add_observer(disabled)
        .add_systems(PreUpdate, finish_press.after(PickingSystems::Last));
}

fn release(
    mut event: On<PointerRelease>,
    buttons: Query<(), With<WidgetryButtonHeadless>>,
    roots: Query<&PressOwner, With<WidgetryButtonHeadless>>,
    mut commands: Commands,
) {
    if !buttons.contains(event.entity) {
        return;
    }
    event.propagate(false);
    if event.button == PointerButton::Primary
        && roots.get(event.entity).is_ok_and(|owner| {
            owner.pointer == event.pointer.id && owner.target == event.pointer.target
        })
    {
        commands
            .entity(event.entity)
            .remove::<(Pressed, PressOwner)>();
    }
}

fn cancel(
    mut event: On<PointerCancel>,
    buttons: Query<(), With<WidgetryButtonHeadless>>,
    roots: Query<&PressOwner, With<WidgetryButtonHeadless>>,
    mut commands: Commands,
) {
    if !buttons.contains(event.entity) {
        return;
    }
    event.propagate(false);
    if roots.get(event.entity).is_ok_and(|owner| {
        owner.pointer == event.pointer.id && owner.target == event.pointer.target
    }) {
        commands
            .entity(event.entity)
            .remove::<(Pressed, PressOwner)>();
    }
}

fn press(
    mut event: On<PointerPress>,
    roots: Query<(Has<InteractionDisabled>, Has<Pressed>), With<WidgetryButtonHeadless>>,
    mut commands: Commands,
) {
    let Ok((disabled, pressed)) = roots.get(event.entity) else {
        return;
    };
    event.propagate(false);
    if event.button != PointerButton::Primary {
        return;
    }
    if !disabled && !pressed {
        commands.entity(event.entity).insert((
            Pressed,
            PressOwner {
                pointer: event.pointer.id,
                target: event.pointer.target.clone(),
            },
        ));
    }
}

fn click(
    mut event: On<PointerClick>,
    buttons: Query<(), With<WidgetryButtonHeadless>>,
    roots: Query<
        (&PressOwner, Has<Pressed>, Has<InteractionDisabled>),
        With<WidgetryButtonHeadless>,
    >,
    mut commands: Commands,
) {
    if !buttons.contains(event.entity) {
        return;
    }
    event.propagate(false);
    if event.button != PointerButton::Primary {
        return;
    }
    if let Ok((owner, pressed, disabled)) = roots.get(event.entity) {
        event.propagate(false);
        if pressed
            && !disabled
            && owner.pointer == event.pointer.id
            && owner.target == event.pointer.target
        {
            commands.trigger(Activate {
                entity: event.entity,
            });
        }
    }
}

fn drag_start(
    mut event: On<PointerDragStart>,
    buttons: Query<(), With<WidgetryButtonHeadless>>,
    roots: Query<&PressOwner, With<WidgetryButtonHeadless>>,
    mut commands: Commands,
) {
    if !buttons.contains(event.entity) {
        return;
    }
    event.propagate(false);
    if event.button == PointerButton::Primary
        && roots.get(event.entity).is_ok_and(|owner| {
            owner.pointer == event.pointer.id && owner.target == event.pointer.target
        })
    {
        commands
            .entity(event.entity)
            .remove::<(Pressed, PressOwner)>();
    }
}

fn drag(mut event: On<PointerDrag>, buttons: Query<(), With<WidgetryButtonHeadless>>) {
    if buttons.contains(event.entity) {
        event.propagate(false);
    }
}

fn drag_end(mut event: On<PointerDragEnd>, buttons: Query<(), With<WidgetryButtonHeadless>>) {
    if buttons.contains(event.entity) {
        event.propagate(false);
    }
}

fn key(
    mut event: On<FocusedInput<KeyboardInput>>,
    roots: Query<Has<InteractionDisabled>, With<WidgetryButtonHeadless>>,
    mut commands: Commands,
) {
    if roots
        .get(event.focused_entity)
        .is_ok_and(|disabled| !disabled)
        && !event.input.repeat
        && event.input.state == ButtonState::Pressed
        && matches!(event.input.key_code, KeyCode::Enter | KeyCode::Space)
    {
        event.propagate(false);
        commands.trigger(Activate {
            entity: event.focused_entity,
        });
    }
}

fn disabled(
    event: On<Insert<bevy_widgetry_core::disabled::WidgetryEffectiveDisabled>>,
    roots: Query<
        &bevy_widgetry_core::disabled::WidgetryEffectiveDisabled,
        (With<WidgetryButtonHeadless>, With<PressOwner>),
    >,
    mut commands: Commands,
) {
    if roots
        .get(event.entity)
        .is_ok_and(|state| state.is_disabled())
    {
        commands
            .entity(event.entity)
            .remove::<(Pressed, PressOwner)>();
    }
}

fn finish_press(
    mut inputs: MessageReader<PointerInput>,
    pointers: WidgetryPointerQuery,
    roots: Query<
        (Entity, &PressOwner, Has<Pressed>, Has<InteractionDisabled>),
        With<WidgetryButtonHeadless>,
    >,
    windows: Query<&Window>,
    mut commands: Commands,
) {
    let inputs: Vec<_> = inputs.read().collect();
    for (entity, owner, pressed, disabled) in &roots {
        let invalid = pointers
            .location(owner.pointer)
            .is_none_or(|location| location.target != owner.target)
            || (owner.pointer == PointerId::Mouse
                && matches!(&owner.target, bevy::camera::NormalizedRenderTarget::Window(window)
                    if !windows.get(window.entity()).is_ok_and(|window| window.focused)));
        let cancelled = inputs.iter().any(|input| {
            input.pointer_id == owner.pointer
                && input.location.target == owner.target
                && matches!(input.action, PointerAction::Cancel)
        });
        let ended = inputs
            .iter()
            .rev()
            .find_map(|input| {
                if input.pointer_id != owner.pointer || input.location.target != owner.target {
                    return None;
                }
                match input.action {
                    PointerAction::Cancel | PointerAction::Release(PointerButton::Primary) => {
                        Some(true)
                    }
                    PointerAction::Press(PointerButton::Primary) => Some(false),
                    _ => None,
                }
            })
            .unwrap_or(false);
        if !pressed || disabled || invalid || cancelled || ended {
            commands.entity(entity).remove::<(Pressed, PressOwner)>();
        }
    }
}
