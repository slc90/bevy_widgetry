use crate::WidgetrySegmentedOptionHeadless;
use crate::{
    SegmentedError, SegmentedFailure,
    error::{collect, core, failure},
};
use bevy::ecs::entity::EntityHashSet;
use bevy::input_focus::tab_navigation::TabIndex;
use bevy::prelude::*;
use bevy::ui::Checked;
use bevy_widgetry_core::exclusive_selection::{InitialSelection, SelectionHistory as Initialized};
use bevy_widgetry_diagnostics::{
    ErrorFrame,
    bevy::{DiagnosticBatch, FailureState, WidgetryEcsError},
};
use std::sync::Arc;

#[derive(SceneComponent, Default, Clone)]
#[scene(crate::WidgetrySegmentedHeadlessProps)]
#[require(Node, TabIndex(-1), GroupDiagnostics, crate::OrientationState)]
pub struct WidgetrySegmentedHeadless;

impl WidgetrySegmentedHeadless {
    fn scene(props: crate::WidgetrySegmentedHeadlessProps) -> impl Scene {
        bsn! {
            template(move |_| Ok(crate::OrientationState(props.orientation)))
            template(|_| Ok(bevy::a11y::AccessibilityNode(accesskit::Node::new(accesskit::Role::RadioGroup))))
        }
    }
}

#[derive(SceneComponent, Default, Clone)]
#[scene(crate::WidgetrySegmentedProps)]
#[require(
    WidgetrySegmentedHeadless,
    crate::colors::ColorState,
    crate::style::StyleDiagnostics
)]
pub struct WidgetrySegmented;

#[derive(Component, Default)]
struct GroupDiagnostics(FailureState<GroupFailureKey>);
#[derive(Clone, PartialEq, Eq)]
enum GroupFailureKey {
    Empty,
    Child(Entity),
    InitialChecked(Vec<Entity>),
}

#[derive(Component, Default)]
pub(crate) struct OwnershipDiagnostics(FailureState<Option<Entity>>);

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
        .add_observer(on_parent_discarded)
        .add_observer(on_initial_checked_inserted)
        .add_observer(on_initial_checked_discarded);
    let world = app.world_mut();
    let groups = world
        .query_filtered::<Entity, With<WidgetrySegmentedHeadless>>()
        .iter(world)
        .collect::<EntityHashSet>();
    let options = world
        .query_filtered::<Entity, With<WidgetrySegmentedOptionHeadless>>()
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

fn on_initial_checked_inserted(
    event: On<Insert<Checked>>,
    options: Query<&ChildOf, With<WidgetrySegmentedOptionHeadless>>,
    initialized: Query<(), With<Initialized>>,
    mut changes: ResMut<StructureChanges>,
) {
    if let Ok(parent) = options.get(event.entity)
        && !initialized.contains(parent.parent())
    {
        changes.groups.insert(parent.parent());
    }
}

fn on_initial_checked_discarded(
    event: On<Discard<Checked>>,
    options: Query<&ChildOf, With<WidgetrySegmentedOptionHeadless>>,
    initialized: Query<(), With<Initialized>>,
    mut changes: ResMut<StructureChanges>,
) {
    if let Ok(parent) = options.get(event.entity)
        && !initialized.contains(parent.parent())
    {
        changes.groups.insert(parent.parent());
    }
}

fn on_group_inserted(
    event: On<Insert<WidgetrySegmentedHeadless>>,
    children: Query<&Children>,
    mut changes: ResMut<StructureChanges>,
) {
    invalidate_group(event.entity, &children, &mut changes);
}

fn on_group_removed(
    event: On<Remove<WidgetrySegmentedHeadless>>,
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
    event: On<Insert<WidgetrySegmentedOptionHeadless>>,
    parents: Query<&ChildOf>,
    mut changes: ResMut<StructureChanges>,
) {
    invalidate_option(event.entity, &parents, &mut changes);
}

fn on_option_removed(
    event: On<Remove<WidgetrySegmentedOptionHeadless>>,
    parents: Query<&ChildOf>,
    mut changes: ResMut<StructureChanges>,
) {
    invalidate_option(event.entity, &parents, &mut changes);
}

fn invalidate_parent(
    entity: Entity,
    parents: &Query<(Has<WidgetrySegmentedOptionHeadless>, &ChildOf)>,
    groups: &Query<(), With<WidgetrySegmentedHeadless>>,
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
    parents: Query<(Has<WidgetrySegmentedOptionHeadless>, &ChildOf)>,
    groups: Query<(), With<WidgetrySegmentedHeadless>>,
    mut changes: ResMut<StructureChanges>,
) {
    invalidate_parent(event.entity, &parents, &groups, &mut changes);
}

fn on_parent_discarded(
    event: On<Discard<ChildOf>>,
    parents: Query<(Has<WidgetrySegmentedOptionHeadless>, &ChildOf)>,
    groups: Query<(), With<WidgetrySegmentedHeadless>>,
    checked: Query<(), With<Checked>>,
    mut initialized: Query<&mut Initialized>,
    mut changes: ResMut<StructureChanges>,
) {
    // Discard still exposes the old parent on both replacement and removal.
    if checked.contains(event.entity)
        && let Ok((_, parent)) = parents.get(event.entity)
        && let Ok(mut history) = initialized.get_mut(parent.parent())
    {
        history.remember_removed(event.entity);
    }
    invalidate_parent(event.entity, &parents, &groups, &mut changes);
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
    let mut batch = DiagnosticBatch::default();
    for root in roots {
        if world.get::<WidgetrySegmentedHeadless>(root).is_none() {
            continue;
        }
        #[cfg(test)]
        if let Some(mut changes) = world.get_resource_mut::<StructureChanges>() {
            changes.groups_checked += 1;
        }
        collect(&mut batch, initialize_group(world, root));
    }
    for option in options {
        if world
            .get::<WidgetrySegmentedOptionHeadless>(option)
            .is_none()
        {
            continue;
        }
        let parent = world.get::<ChildOf>(option).map(ChildOf::parent);
        #[cfg(test)]
        if let Some(mut changes) = world.get_resource_mut::<StructureChanges>() {
            changes.options_checked += 1;
        }
        let frame = || {
            ErrorFrame::new("segmented_option.ownership")
                .field("option", option)
                .field("parent", format_args!("{parent:?}"))
        };
        if parent.is_some_and(|parent| world.get::<WidgetrySegmentedHeadless>(parent).is_some()) {
            if let Some(mut diagnostics) = world.get_mut::<OwnershipDiagnostics>(option) {
                diagnostics.0.recover(frame);
            }
        } else {
            let create = || -> Arc<dyn bevy_widgetry_diagnostics::bevy::Diagnostic> {
                Arc::new(failure(
                    SegmentedError::InvalidParent { option, parent },
                    frame(),
                ))
            };
            let error = if let Some(mut diagnostics) = world.get_mut::<OwnershipDiagnostics>(option)
            {
                diagnostics.0.failure(parent, create)
            } else {
                WidgetryEcsError::from_shared(create())
            };
            collect(&mut batch, Err(error));
        }
    }
    batch.into_bevy_error().map_or(Ok(()), Err)
}

pub(crate) fn initialize_group(world: &mut World, root: Entity) -> Result<(), WidgetryEcsError> {
    let children = world
        .get::<Children>(root)
        .map(|children| children.to_vec())
        .unwrap_or_default();
    let invalid = if children.is_empty() {
        Initialized::reset(world, root).map_err(|error| {
            WidgetryEcsError::new(core(
                error,
                ErrorFrame::new("segmented.reset_selection").field("group", root),
            ))
        })?;
        Some((
            GroupFailureKey::Empty,
            SegmentedError::EmptyGroup { group: root },
        ))
    } else {
        children
            .iter()
            .find(|child| {
                world
                    .get::<WidgetrySegmentedOptionHeadless>(**child)
                    .is_none()
            })
            .map(|child| {
                (
                    GroupFailureKey::Child(*child),
                    SegmentedError::InvalidChild {
                        group: root,
                        option: *child,
                    },
                )
            })
    };
    if let Some((key, kind)) = invalid {
        let create = || -> Arc<dyn bevy_widgetry_diagnostics::bevy::Diagnostic> {
            Arc::new(failure(kind, ErrorFrame::new("segmented.initialize")))
        };
        return Err(
            if let Some(mut diagnostics) = world.get_mut::<GroupDiagnostics>(root) {
                diagnostics.0.failure(key, create)
            } else {
                WidgetryEcsError::from_shared(create())
            },
        );
    }
    if world.get::<Initialized>(root).is_none() {
        let checked: Vec<_> = children
            .iter()
            .copied()
            .filter(|child| world.get::<Checked>(*child).is_some())
            .collect();
        if checked.len() > 1 {
            let key = GroupFailureKey::InitialChecked(checked.clone());
            let create = || -> Arc<dyn bevy_widgetry_diagnostics::bevy::Diagnostic> {
                Arc::new(failure(
                    SegmentedError::InitialCheckedConflict {
                        group: root,
                        options: checked,
                    },
                    ErrorFrame::new("segmented.initialize_checked"),
                ))
            };
            let error = if let Some(mut diagnostics) = world.get_mut::<GroupDiagnostics>(root) {
                diagnostics.0.failure(key, create)
            } else {
                WidgetryEcsError::from_shared(create())
            };
            return Err(error);
        }
    }
    Initialized::repair(world, root, &children, InitialSelection::Checked).map_err(|error| {
        WidgetryEcsError::new(core(
            error,
            ErrorFrame::new("segmented.repair_selection").field("group", root),
        ))
    })?;
    if let Some(mut diagnostics) = world.get_mut::<GroupDiagnostics>(root) {
        diagnostics
            .0
            .recover(|| ErrorFrame::new("segmented.initialize").field("group", root));
    }
    Ok(())
}

impl WidgetrySegmentedHeadless {
    pub fn set_selected(commands: &mut Commands, entity: Entity, selected: usize) {
        commands.queue(move |world: &mut World| -> Result<(), BevyError> {
            let (children, target) = selection_target(world, entity, selected)
                .map_err(|error| error.into_bevy_error())?;
            initialize_group(world, entity).map_err(|error| error.into_bevy_error())?;
            if world.get::<Checked>(target).is_some() {
                return Ok(());
            }
            Initialized::select(world, &children, target).map_err(|error| {
                core(
                    error,
                    ErrorFrame::new("segmented.select")
                        .field("group", entity)
                        .field("index", selected),
                )
                .into_bevy_error()
            })?;
            Ok(())
        });
    }
}

impl WidgetrySegmented {
    fn scene(props: crate::WidgetrySegmentedProps) -> impl Scene {
        let horizontal = props.orientation == crate::SegmentedOrientation::Horizontal;
        let equal_width = horizontal && props.equal_width;
        bsn! {
            template(move |context| props.colors.clone().initial(context.entity.id()).map_err(|error| error.into_bevy_error()))
            @WidgetrySegmentedHeadless { @orientation: {props.orientation} }
            Node {
                display: {if equal_width { Display::Grid } else { Display::Flex }},
                grid_auto_flow: GridAutoFlow::Column,
                grid_auto_columns: {if equal_width { vec![GridTrack::fr(1.0)] } else { vec![] }},
                flex_direction: {if horizontal { FlexDirection::Row } else { FlexDirection::Column }},
                min_height: px(32),
                padding: UiRect::all(px(2)),
                border: UiRect::all(px(1)),
                border_radius: BorderRadius::all(px(6)),
                align_items: AlignItems::Stretch,
            }
            BackgroundColor
            BorderColor
        }
    }
}

pub(crate) fn selection_target(
    world: &World,
    root: Entity,
    index: usize,
) -> Result<(Vec<Entity>, Entity), SegmentedFailure> {
    let invalid = |kind| {
        failure(
            kind,
            ErrorFrame::new("segmented.select")
                .field("group", root)
                .field("index", index),
        )
    };
    if world.get::<WidgetrySegmentedHeadless>(root).is_none() {
        return Err(invalid(SegmentedError::InvalidTarget {
            entity: root,
            widget: "Segmented",
        }));
    }
    let children = world
        .get::<Children>(root)
        .filter(|children| !children.is_empty())
        .ok_or_else(|| invalid(SegmentedError::EmptyGroup { group: root }))?;
    for &option in children {
        if world
            .get::<WidgetrySegmentedOptionHeadless>(option)
            .is_none()
        {
            return Err(invalid(SegmentedError::InvalidChild {
                group: root,
                option,
            }));
        }
    }
    let target = children.get(index).copied().ok_or_else(|| {
        invalid(SegmentedError::IndexOutOfBounds {
            group: root,
            index,
            len: children.len(),
        })
    })?;
    Ok((children.to_vec(), target))
}

#[cfg(test)]
#[allow(clippy::disallowed_macros)]
mod tests {
    use super::*;
    use crate::WidgetrySegmentedOption;
    use bevy_widgetry_test_utils::{ErrorCapture, scene_app};

    #[test]
    fn idle_groups_do_not_repeat_structure_checks() {
        let mut app = scene_app();
        app.add_plugins(crate::WidgetrySegmentedPlugin);
        for _ in 0..2 {
            app.world_mut()
                .spawn_scene(bsn! {
                    @WidgetrySegmented
                    Children [@WidgetrySegmentedOption-- @WidgetrySegmentedOption-- @WidgetrySegmentedOption]
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
        app.add_plugins(crate::WidgetrySegmentedPlugin);
        let mut roots = Vec::new();
        for _ in 0..2 {
            roots.push(
                app.world_mut()
                    .spawn_scene(bsn! {
                        @WidgetrySegmented
                        Children [@WidgetrySegmentedOption-- @WidgetrySegmentedOption-- @WidgetrySegmentedOption]
                    })
                    .unwrap()
                    .id(),
            );
        }
        initialize(app.world_mut()).unwrap();
        let root = roots[0];
        WidgetrySegmentedHeadless::set_selected(&mut app.world_mut().commands(), root, 1);
        app.world_mut().flush();
        std::mem::take(&mut *app.world_mut().resource_mut::<StructureChanges>());
        app.world_mut()
            .entity_mut(root)
            .remove::<WidgetrySegmentedHeadless>();
        assert_eq!(
            initialize(app.world_mut()).unwrap_err().severity(),
            bevy::ecs::error::Severity::Error
        );
        let checked = std::mem::take(&mut *app.world_mut().resource_mut::<StructureChanges>());
        assert_eq!((checked.groups_checked, checked.options_checked), (0, 3));
        app.world_mut()
            .entity_mut(root)
            .insert(WidgetrySegmentedHeadless);
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
            .spawn_scene(bsn! { @WidgetrySegmented Children [@WidgetrySegmentedOption] })
            .unwrap()
            .id();
        app.add_plugins(crate::WidgetrySegmentedPlugin);
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
                @WidgetrySegmented Children [@WidgetrySegmentedOption-- @WidgetrySegmentedOption Checked]
            })
            .unwrap()
            .id();
            let children = app.world().get::<Children>(root).unwrap().to_vec();
            if invalid_structure {
                app.world_mut()
                    .entity_mut(children[0])
                    .remove::<WidgetrySegmentedOptionHeadless>();
            }
            WidgetrySegmentedHeadless::set_selected(
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
