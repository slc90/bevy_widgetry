use crate::WidgetryRadioOption;
use bevy::input_focus::tab_navigation::TabIndex;
use bevy::prelude::*;
use bevy::ui::Checked;
use bevy::ui_widgets::{RadioGroup, ValueChange};
use bevy_widgetry_core::diagnostics::FailureState;
use bevy_widgetry_log::{widgetry_error, widgetry_info};

#[derive(SceneComponent, Default, Clone)]
#[require(GroupDiagnostics)]
pub struct WidgetryRadioGroup;

#[derive(Component, Default)]
struct GroupDiagnostics(FailureState);

#[derive(Component, Default)]
pub(crate) struct OwnershipDiagnostics(FailureState);

#[derive(Component)]
struct Initialized;

pub(crate) fn handle_value_change(
    event: On<ValueChange<Entity>>,
    groups: Query<&Children, With<WidgetryRadioGroup>>,
    mut commands: Commands,
) {
    let Ok(children) = groups.get(event.source) else {
        return;
    };
    if let Some(index) = children.iter().position(|child| child == event.value) {
        commands.trigger(ValueChange {
            source: event.source,
            value: index,
            is_final: event.is_final,
        });
    }
}

pub(crate) fn initialize(world: &mut World) -> Result<(), BevyError> {
    let roots = world
        .query_filtered::<Entity, (With<WidgetryRadioGroup>, Without<Initialized>)>()
        .iter(world)
        .collect::<Vec<_>>();
    let mut failure = None;
    for root in roots {
        if let Err(error) = initialize_group(world, root)
            && failure.is_none()
        {
            failure = Some(error);
        }
    }
    let options = world
        .query_filtered::<(Entity, Option<&ChildOf>), With<WidgetryRadioOption>>()
        .iter(world)
        .map(|(child, parent)| (child, parent.map(ChildOf::parent)))
        .collect::<Vec<_>>();
    for (child, root) in options {
        let result = if root.is_some_and(|root| world.get::<WidgetryRadioGroup>(root).is_some()) {
            Ok(())
        } else {
            Err(BevyError::error(
                "WidgetryRadioOption requires a direct WidgetryRadioGroup parent",
            ))
        };
        let result = if let Some(mut diagnostics) = world.get_mut::<OwnershipDiagnostics>(child) {
            diagnostics.0.observe(
                result,
                |error| match root {
                    Some(root) => {
                        widgetry_error!(?root, ?child, %error, "RadioOption parent 校验失败")
                    }
                    None => widgetry_error!(?child, %error, "RadioOption parent 校验失败"),
                },
                || widgetry_info!(?root, ?child, "RadioOption parent 恢复正常"),
            )
        } else {
            result
        };
        if let Err(error) = result
            && failure.is_none()
        {
            failure = Some(error);
        }
    }
    match failure {
        Some(error) => Err(error),
        None => Ok(()),
    }
}

fn initialize_group(world: &mut World, root: Entity) -> Result<(), BevyError> {
    let result = initialize_group_inner(world, root);
    let invalid_child = world.get::<Children>(root).and_then(|children| {
        children
            .iter()
            .find(|child| world.get::<WidgetryRadioOption>(*child).is_none())
    });
    if let Some(mut diagnostics) = world.get_mut::<GroupDiagnostics>(root) {
        diagnostics.0.observe(
            result,
            |error| match invalid_child {
                Some(child) => widgetry_error!(?root, ?child, %error, "RadioGroup 初始化失败"),
                None => widgetry_error!(?root, %error, "RadioGroup 初始化失败"),
            },
            || widgetry_info!(?root, "RadioGroup 初始化恢复正常"),
        )
    } else {
        result
    }
}

fn initialize_group_inner(world: &mut World, root: Entity) -> Result<(), BevyError> {
    if world.get::<Initialized>(root).is_some() {
        return Ok(());
    }
    let children = world
        .get::<Children>(root)
        .map(|children| children.to_vec())
        .unwrap_or_default();
    if children.is_empty() {
        return Err(BevyError::error(
            "WidgetryRadioGroup requires at least one option",
        ));
    }
    for &child in &children {
        if world.get::<WidgetryRadioOption>(child).is_none() {
            return Err(BevyError::error(format!(
                "WidgetryRadioGroup requires WidgetryRadioOption children: {child:?}"
            )));
        }
    }
    select(world, &children, children[0]);
    world.entity_mut(root).insert(Initialized);
    Ok(())
}

fn select(world: &mut World, children: &[Entity], target: Entity) {
    for &child in children {
        if child == target {
            world.entity_mut(child).insert(Checked);
        } else {
            world.entity_mut(child).remove::<Checked>();
        }
    }
}

impl WidgetryRadioGroup {
    pub fn set_selected(commands: &mut Commands, entity: Entity, selected: usize) {
        commands.queue(move |world: &mut World| -> Result<(), BevyError> {
            if world.get::<WidgetryRadioGroup>(entity).is_none() {
                return Ok(());
            }
            initialize_group(world, entity)?;
            let Some(children) = world
                .get::<Children>(entity)
                .map(|children| children.to_vec())
            else {
                return Ok(());
            };
            let Some(&target) = children.get(selected) else {
                return Ok(());
            };
            if world.get::<Checked>(target).is_some() {
                return Ok(());
            }
            select(world, &children, target);
            Ok(())
        });
    }

    fn scene() -> impl Scene {
        bsn! {
            RadioGroup
            TabIndex::default()
            Node {
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Stretch,
                row_gap: px(6),
                padding: UiRect::all(px(8)),
                border: UiRect::all(px(1)),
                border_radius: BorderRadius::all(px(4)),
            }
            BackgroundColor
            BorderColor
        }
    }
}
