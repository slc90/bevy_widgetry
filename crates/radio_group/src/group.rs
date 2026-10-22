use crate::WidgetryRadioOption;
use bevy::ecs::entity::EntityHashSet;
use bevy::input_focus::tab_navigation::TabIndex;
use bevy::prelude::*;
use bevy::ui::Checked;
use bevy::ui_widgets::{RadioGroup, ValueChange};
use bevy_widgetry_core::diagnostics::FailureState;
use bevy_widgetry_internal_log::{widgetry_error, widgetry_info};

#[derive(SceneComponent, Default, Clone)]
#[scene(crate::WidgetryRadioGroupProps)]
#[require(GroupDiagnostics, crate::colors::ColorState)]
pub struct WidgetryRadioGroup;

#[derive(Component, Default)]
struct GroupDiagnostics(FailureState);

#[derive(Component, Default)]
pub(crate) struct OwnershipDiagnostics(FailureState);

#[derive(Component)]
struct Initialized {
    order: Vec<Entity>,
    removed_selection: Option<Entity>,
}

#[derive(Resource, Default)]
struct StructureChanges {
    groups: EntityHashSet,
    options: EntityHashSet,
    #[cfg(test)]
    groups_checked: usize,
    #[cfg(test)]
    options_checked: usize,
}

pub(crate) fn install(app: &mut App) {
    app.init_resource::<StructureChanges>()
        .add_observer(on_group_inserted)
        .add_observer(on_group_removed)
        .add_observer(on_option_inserted)
        .add_observer(on_option_removed)
        .add_observer(on_parent_inserted)
        .add_observer(on_parent_discarded);
    let world = app.world_mut();
    let groups = world
        .query_filtered::<Entity, With<WidgetryRadioGroup>>()
        .iter(world)
        .collect::<EntityHashSet>();
    let options = world
        .query_filtered::<Entity, With<WidgetryRadioOption>>()
        .iter(world)
        .collect::<EntityHashSet>();
    let mut changes = world.resource_mut::<StructureChanges>();
    changes.groups.extend(groups);
    changes.options.extend(options);
}

fn invalidate_group(entity: Entity, children: &Query<&Children>, changes: &mut StructureChanges) {
    changes.groups.insert(entity);
    if let Ok(children) = children.get(entity) {
        changes.options.extend(children.iter());
    }
}

fn on_group_inserted(
    event: On<Insert<WidgetryRadioGroup>>,
    children: Query<&Children>,
    mut changes: ResMut<StructureChanges>,
) {
    invalidate_group(event.entity, &children, &mut changes);
}

fn on_group_removed(
    event: On<Remove<WidgetryRadioGroup>>,
    children: Query<&Children>,
    mut changes: ResMut<StructureChanges>,
) {
    // ChildOf does not change when the parent loses its Group marker.
    invalidate_group(event.entity, &children, &mut changes);
}

fn invalidate_option(entity: Entity, parents: &Query<&ChildOf>, changes: &mut StructureChanges) {
    changes.options.insert(entity);
    if let Ok(parent) = parents.get(entity) {
        changes.groups.insert(parent.parent());
    }
}

fn on_option_inserted(
    event: On<Insert<WidgetryRadioOption>>,
    parents: Query<&ChildOf>,
    mut changes: ResMut<StructureChanges>,
) {
    invalidate_option(event.entity, &parents, &mut changes);
}

fn on_option_removed(
    event: On<Remove<WidgetryRadioOption>>,
    parents: Query<&ChildOf>,
    mut changes: ResMut<StructureChanges>,
) {
    invalidate_option(event.entity, &parents, &mut changes);
}

fn invalidate_parent(
    entity: Entity,
    parents: &Query<(Has<WidgetryRadioOption>, &ChildOf)>,
    groups: &Query<(), With<WidgetryRadioGroup>>,
    changes: &mut StructureChanges,
) {
    if let Ok((option, parent)) = parents.get(entity) {
        if option {
            changes.options.insert(entity);
        }
        if groups.contains(parent.parent()) {
            changes.groups.insert(parent.parent());
        }
    }
}

fn on_parent_inserted(
    event: On<Insert<ChildOf>>,
    parents: Query<(Has<WidgetryRadioOption>, &ChildOf)>,
    groups: Query<(), With<WidgetryRadioGroup>>,
    mut changes: ResMut<StructureChanges>,
) {
    invalidate_parent(event.entity, &parents, &groups, &mut changes);
}

fn on_parent_discarded(
    event: On<Discard<ChildOf>>,
    parents: Query<(Has<WidgetryRadioOption>, &ChildOf)>,
    groups: Query<(), With<WidgetryRadioGroup>>,
    checked: Query<(), With<Checked>>,
    mut initialized: Query<&mut Initialized>,
    mut changes: ResMut<StructureChanges>,
) {
    // Discard still exposes the old parent on both replacement and removal.
    if checked.contains(event.entity)
        && let Ok((_, parent)) = parents.get(event.entity)
        && let Ok(mut history) = initialized.get_mut(parent.parent())
        && history.order.contains(&event.entity)
    {
        history.removed_selection = Some(event.entity);
    }
    invalidate_parent(event.entity, &parents, &groups, &mut changes);
}

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
    // Scene observers can run before Children are attached.
    // Drain once at the input boundary after deferred construction has completed.
    let (roots, options) = {
        let mut changes = world.resource_mut::<StructureChanges>();
        (
            std::mem::take(&mut changes.groups),
            std::mem::take(&mut changes.options),
        )
    };
    let mut failure = None;
    for root in roots {
        if world.get::<WidgetryRadioGroup>(root).is_none() {
            continue;
        }
        #[cfg(test)]
        if let Some(mut changes) = world.get_resource_mut::<StructureChanges>() {
            changes.groups_checked += 1;
        }
        if let Err(error) = initialize_group(world, root)
            && failure.is_none()
        {
            failure = Some(error);
        }
    }
    for child in options {
        if world.get::<WidgetryRadioOption>(child).is_none() {
            continue;
        }
        let root = world.get::<ChildOf>(child).map(ChildOf::parent);
        #[cfg(test)]
        if let Some(mut changes) = world.get_resource_mut::<StructureChanges>() {
            changes.options_checked += 1;
        }
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
    let children = world
        .get::<Children>(root)
        .map(|children| children.to_vec())
        .unwrap_or_default();
    if children.is_empty() {
        world.entity_mut(root).remove::<Initialized>();
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
    let target = match world.get::<Initialized>(root) {
        None => children[0],
        Some(history) => history
            .order
            .iter()
            .copied()
            .find(|child| children.contains(child) && world.get::<Checked>(*child).is_some())
            .or_else(|| {
                let index = history
                    .order
                    .iter()
                    .position(|child| Some(*child) == history.removed_selection)?;
                history.order[index + 1..]
                    .iter()
                    .chain(history.order[..index].iter().rev())
                    .copied()
                    .find(|child| children.contains(child))
            })
            .unwrap_or(children[0]),
    };
    select(world, &children, target);
    world.entity_mut(root).insert(Initialized {
        order: children,
        removed_selection: None,
    });
    Ok(())
}

fn select(world: &mut World, children: &[Entity], target: Entity) {
    for &child in children {
        if child == target && world.get::<Checked>(child).is_none() {
            world.entity_mut(child).insert(Checked);
        } else if child != target {
            world.entity_mut(child).remove::<Checked>();
        }
    }
}

impl WidgetryRadioGroup {
    pub fn set_selected(commands: &mut Commands, entity: Entity, selected: usize) {
        commands.queue(move |world: &mut World| -> Result<(), BevyError> {
            let (children, target) = selection_target(world, entity, selected).inspect_err(|error| {
                widgetry_error!(root = ?entity, index = selected, %error, "RadioGroup 程序化选择失败");
            })?;
            initialize_group(world, entity)?;
            if world.get::<Checked>(target).is_some() {
                return Ok(());
            }
            select(world, &children, target);
            Ok(())
        });
    }

    fn scene(props: crate::WidgetryRadioGroupProps) -> impl Scene {
        bsn! {
            template(move |_| props.colors.clone().initial())
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
            bevy_widgetry_core::foreground::ResolvedForeground
            BackgroundColor
            BorderColor
        }
    }
}

fn selection_target(
    world: &World,
    root: Entity,
    index: usize,
) -> Result<(Vec<Entity>, Entity), BevyError> {
    if world.get::<WidgetryRadioGroup>(root).is_none() {
        return Err(BevyError::error(format!(
            "WidgetryRadioGroup root is missing or invalid: {root:?}"
        )));
    }
    let children = world
        .get::<Children>(root)
        .filter(|children| !children.is_empty())
        .ok_or_else(|| BevyError::error("WidgetryRadioGroup requires at least one option"))?;
    for child in children {
        if world.get::<WidgetryRadioOption>(*child).is_none() {
            return Err(BevyError::error(format!(
                "WidgetryRadioGroup requires WidgetryRadioOption children: {child:?}"
            )));
        }
    }
    let target = children.get(index).copied().ok_or_else(|| {
        BevyError::error(format!("WidgetryRadioGroup index out of bounds: {index}"))
    })?;
    Ok((children.to_vec(), target))
}

#[cfg(test)]
#[allow(clippy::disallowed_macros)]
mod tests {
    use super::*;
    use bevy_widgetry_test_utils::{ErrorCapture, scene_app};

    #[test]
    fn idle_groups_do_not_repeat_structure_checks() {
        let mut app = scene_app();
        app.add_plugins(crate::WidgetryRadioGroupPlugin);
        for _ in 0..2 {
            app.world_mut()
                .spawn_scene(bsn! {
                    @WidgetryRadioGroup
                    Children [@WidgetryRadioOption-- @WidgetryRadioOption-- @WidgetryRadioOption]
                })
                .unwrap();
        }
        initialize(app.world_mut()).unwrap();
        let checked = std::mem::take(&mut *app.world_mut().resource_mut::<StructureChanges>());
        assert_eq!((checked.groups_checked, checked.options_checked), (2, 6));
        for _ in 0..3 {
            initialize(app.world_mut()).unwrap();
        }
        let checked = app.world().resource::<StructureChanges>();
        assert_eq!((checked.groups_checked, checked.options_checked), (0, 0));
    }

    #[test]
    fn parent_marker_changes_check_only_the_affected_group() {
        let mut app = scene_app();
        app.add_plugins(crate::WidgetryRadioGroupPlugin);
        let mut roots = Vec::new();
        for _ in 0..2 {
            roots.push(
                app.world_mut()
                    .spawn_scene(bsn! {
                        @WidgetryRadioGroup
                        Children [@WidgetryRadioOption-- @WidgetryRadioOption-- @WidgetryRadioOption]
                    })
                    .unwrap()
                    .id(),
            );
        }
        initialize(app.world_mut()).unwrap();
        let root = roots[0];
        WidgetryRadioGroup::set_selected(&mut app.world_mut().commands(), root, 1);
        app.world_mut().flush();
        std::mem::take(&mut *app.world_mut().resource_mut::<StructureChanges>());
        app.world_mut()
            .entity_mut(root)
            .remove::<WidgetryRadioGroup>();
        assert_eq!(
            initialize(app.world_mut()).unwrap_err().severity(),
            bevy::ecs::error::Severity::Error
        );
        let checked = std::mem::take(&mut *app.world_mut().resource_mut::<StructureChanges>());
        assert_eq!((checked.groups_checked, checked.options_checked), (0, 3));
        app.world_mut().entity_mut(root).insert(WidgetryRadioGroup);
        initialize(app.world_mut()).unwrap();
        let checked = std::mem::take(&mut *app.world_mut().resource_mut::<StructureChanges>());
        assert_eq!((checked.groups_checked, checked.options_checked), (1, 3));
        let children = app.world().get::<Children>(root).unwrap();
        assert!(app.world().get::<Checked>(children[1]).is_some());
        assert!(app.world().get::<Checked>(children[0]).is_none());
        app.world_mut().despawn(root);
        initialize(app.world_mut()).unwrap();
        let checked = app.world().resource::<StructureChanges>();
        assert_eq!((checked.groups_checked, checked.options_checked), (0, 0));
        assert!(checked.groups.is_empty());
        assert!(checked.options.is_empty());
    }

    #[test]
    fn late_plugin_registration_checks_existing_scenes_once() {
        let mut app = scene_app();
        let root = app
            .world_mut()
            .spawn_scene(bsn! { @WidgetryRadioGroup Children [@WidgetryRadioOption] })
            .unwrap()
            .id();
        app.add_plugins(crate::WidgetryRadioGroupPlugin);
        initialize(app.world_mut()).unwrap();
        let checked = std::mem::take(&mut *app.world_mut().resource_mut::<StructureChanges>());
        assert_eq!((checked.groups_checked, checked.options_checked), (1, 1));
        assert!(app.world().get::<Initialized>(root).is_some());
        initialize(app.world_mut()).unwrap();
        let checked = app.world().resource::<StructureChanges>();
        assert_eq!((checked.groups_checked, checked.options_checked), (0, 0));
    }

    #[test]
    fn rejected_request_does_not_initialize_group() {
        for invalid_structure in [false, true] {
            let mut app = scene_app();
            app.set_error_handler(ErrorCapture::handler());
            let root = app
            .world_mut()
            .spawn_scene(bsn! {
                @WidgetryRadioGroup Children [@WidgetryRadioOption-- @WidgetryRadioOption Checked]
            })
            .unwrap()
            .id();
            let children = app.world().get::<Children>(root).unwrap().to_vec();
            if invalid_structure {
                app.world_mut()
                    .entity_mut(children[0])
                    .remove::<WidgetryRadioOption>();
            }
            WidgetryRadioGroup::set_selected(
                &mut app.world_mut().commands(),
                root,
                if invalid_structure { 1 } else { 2 },
            );
            let errors = ErrorCapture::default();
            errors.run(|| app.world_mut().flush());
            assert_eq!(errors.take().len(), 1);
            assert!(app.world().get::<Initialized>(root).is_none());
            assert!(app.world().get::<Checked>(children[0]).is_none());
            assert!(app.world().get::<Checked>(children[1]).is_some());
        }
    }
}
