use crate::WidgetryCollapsible;
use crate::collapsible::{Expanded, Parts};
use crate::{
    CollapsibleFailure, CollapsibleStructureError as Problem,
    error::{missing, structural},
};
use bevy::prelude::*;
use bevy_widgetry_diagnostics::{ErrorFrame, bevy::DiagnosticBatch};

#[derive(EntityEvent, Debug, Clone, Copy)]
pub struct WidgetryCollapsibleChanged {
    pub entity: Entity,
    pub expanded: bool,
}

pub(crate) fn structure(world: &World, root: Entity) -> Result<Parts, CollapsibleFailure> {
    inspect_structure(world, root)
        .map_err(|problem| structural(problem, ErrorFrame::new("collapsible.structure")))
}

pub(crate) fn inspect_structure(world: &World, root: Entity) -> Result<Parts, Problem> {
    if world.get::<WidgetryCollapsible>(root).is_none() {
        return Err(Problem::InvalidRoot { root });
    }
    for (present, part) in [
        (world.get::<Expanded>(root).is_some(), "Expanded"),
        (
            world.get::<crate::disabled::HeaderDisabled>(root).is_some(),
            "HeaderDisabled",
        ),
    ] {
        if !present {
            return Err(Problem::MissingPart {
                root,
                entity: root,
                part,
            });
        }
    }
    let parts = *world.get::<Parts>(root).ok_or(Problem::MissingPart {
        root,
        entity: root,
        part: "Parts",
    })?;
    for (child, parent, part, node, visibility) in [
        (
            parts.header,
            root,
            "header",
            "header.Node",
            "header.Visibility",
        ),
        (
            parts.content,
            root,
            "content",
            "content.Node",
            "content.Visibility",
        ),
        (
            parts.header_content,
            parts.header,
            "header_content",
            "header_content.Node",
            "header_content.Visibility",
        ),
        (
            parts.button,
            parts.header,
            "button",
            "button.Node",
            "button.Visibility",
        ),
    ] {
        if world.get::<Node>(child).is_none() {
            return Err(Problem::MissingPart {
                root,
                entity: child,
                part: node,
            });
        }
        if world.get::<Visibility>(child).is_none() {
            return Err(Problem::MissingPart {
                root,
                entity: child,
                part: visibility,
            });
        }
        let actual = world.get::<ChildOf>(child).map(ChildOf::parent);
        if actual != Some(parent) {
            return Err(Problem::InvalidParent {
                root,
                entity: child,
                part,
                expected: parent,
                actual,
            });
        }
    }
    if world
        .get::<bevy_widgetry_button::WidgetryButton>(parts.button)
        .is_none()
    {
        return Err(Problem::InvalidButton {
            root,
            button: parts.button,
        });
    }
    Ok(parts)
}

pub(crate) fn install(app: &mut App) {
    app.init_resource::<Pending>()
        .add_observer(|event: On<Insert<Parts>>, mut pending: ResMut<Pending>| {
            pending.0.push(event.entity);
        })
        .add_systems(
            PreUpdate,
            initialize
                .before(bevy::input_focus::InputFocusSystems::Dispatch)
                .before(bevy::picking::PickingSystems::Hover),
        )
        .add_systems(
            PostUpdate,
            initialize.in_set(bevy_widgetry_core::ui::WidgetryUiSystems::Build),
        );
    let world = app.world_mut();
    let roots = world
        .query_filtered::<Entity, With<WidgetryCollapsible>>()
        .iter(world)
        .collect::<Vec<_>>();
    world.resource_mut::<Pending>().0.extend(roots);
}

#[derive(Resource, Default)]
struct Pending(Vec<Entity>);

fn initialize(world: &mut World) -> Result {
    let roots = std::mem::take(&mut world.resource_mut::<Pending>().0);
    let mut batch = DiagnosticBatch::default();
    for root in roots {
        if world.get::<WidgetryCollapsible>(root).is_none() {
            continue;
        }
        let result = (|| -> Result<(), CollapsibleFailure> {
            let parts = structure(world, root)?;
            crate::disabled::project(world, root, parts.header)?;
            let expanded = world
                .get::<Expanded>(root)
                .ok_or_else(|| missing(root, root, "Expanded"))?
                .0;
            project(world, parts.content, expanded);
            Ok(())
        })();
        if let Err(error) = result {
            batch.push(
                error.with_context(ErrorFrame::new("collapsible.initialize").field("root", root)),
            );
        }
    }
    batch.into_bevy_error().map_or(Ok(()), Err)
}

fn project(world: &mut World, content: Entity, expanded: bool) {
    world.entity_mut(content).insert(if expanded {
        Visibility::Inherited
    } else {
        Visibility::Hidden
    });
    if let Some(mut node) = world.get_mut::<Node>(content) {
        node.display = if expanded {
            Display::Flex
        } else {
            Display::None
        };
    }
    // Focus navigation can precede PostUpdate visibility propagation after a setter.
    let mut ancestor = world.get::<ChildOf>(content).map(ChildOf::parent);
    let mut parent_visible = true;
    while let Some(entity) = ancestor {
        match world.get::<Visibility>(entity) {
            Some(Visibility::Hidden) => {
                parent_visible = false;
                break;
            }
            Some(Visibility::Inherited) => {
                ancestor = world.get::<ChildOf>(entity).map(ChildOf::parent)
            }
            _ => break,
        }
    }
    let mut pending = vec![(content, parent_visible)];
    while let Some((entity, parent_visible)) = pending.pop() {
        let visible = match world.get::<Visibility>(entity) {
            Some(Visibility::Hidden) => false,
            Some(Visibility::Visible) => true,
            Some(Visibility::Inherited) => parent_visible,
            None => continue,
        };
        if let Some(mut inherited) = world.get_mut::<InheritedVisibility>(entity) {
            inherited.set_if_neq(if visible {
                InheritedVisibility::VISIBLE
            } else {
                InheritedVisibility::HIDDEN
            });
        }
        if let Some(children) = world.get::<Children>(entity) {
            pending.extend(children.iter().map(|child| (child, visible)));
        }
    }
}

impl WidgetryCollapsible {
    pub fn expanded(world: &World, root: Entity) -> Result<bool, CollapsibleFailure> {
        structure(world, root)?;
        Ok(world
            .get::<Expanded>(root)
            .ok_or_else(|| missing(root, root, "Expanded"))?
            .0)
    }

    pub fn set_expanded(commands: &mut Commands, root: Entity, expanded: bool) {
        commands.queue(move |world: &mut World| {
            Self::set_expanded_in_world(world, root, expanded)
                .map(|_| ())
                .map_err(|error| error.into_bevy_error())
        });
    }

    pub fn set_expanded_in_world(
        world: &mut World,
        root: Entity,
        expanded: bool,
    ) -> Result<bool, CollapsibleFailure> {
        let frame = || {
            ErrorFrame::new("collapsible.set_expanded")
                .field("root", root)
                .field("expanded", expanded)
        };
        let parts = structure(world, root).map_err(|error| error.with_context(frame()))?;
        crate::disabled::project(world, root, parts.header)
            .map_err(|error| error.with_context(frame()))?;
        let current = world
            .get::<Expanded>(root)
            .ok_or_else(|| missing(root, root, "Expanded"))?;
        if current.0 == expanded {
            return Ok(false);
        }
        crate::style::set_arrow(world, root, parts.button, expanded)
            .map_err(|error| error.with_context(frame()))?;
        world
            .get_mut::<Expanded>(root)
            .ok_or_else(|| missing(root, root, "Expanded"))?
            .0 = expanded;
        if !expanded {
            crate::input::collapse_focus(world, parts);
        }
        project(world, parts.content, expanded);
        world.trigger(WidgetryCollapsibleChanged {
            entity: root,
            expanded,
        });
        Ok(true)
    }
}

#[cfg(test)]
#[allow(clippy::disallowed_macros, clippy::unwrap_used)]
mod tests {
    use super::*;
    use crate::{CollapsibleError, disabled::HeaderDisabled};
    use bevy_widgetry_test_utils::{DiagnosticPhase, LogCapture, scene_app};
    use std::sync::Arc;

    #[test]
    fn expanded_header_disabled_and_parts_are_distinct_missing_state_failures() {
        let mut app = scene_app();
        app.add_plugins(crate::WidgetryCollapsiblePlugin);
        let root = app
            .world_mut()
            .spawn_scene(bsn! { @WidgetryCollapsible })
            .unwrap()
            .id();
        let expanded = app.world_mut().entity_mut(root).take::<Expanded>().unwrap();
        let failure = WidgetryCollapsible::expanded(app.world(), root).unwrap_err();
        assert!(
            matches!(failure.kind(), CollapsibleError::Structure { source: Problem::MissingPart { root: actual, entity, part: "Expanded" } } if *actual == root && *entity == root)
        );
        app.world_mut().entity_mut(root).insert(expanded);
        let disabled = app
            .world_mut()
            .entity_mut(root)
            .take::<HeaderDisabled>()
            .unwrap();
        let failure = WidgetryCollapsible::header_disabled(app.world(), root).unwrap_err();
        assert!(
            matches!(failure.kind(), CollapsibleError::Structure { source: Problem::MissingPart { root: actual, entity, part: "HeaderDisabled" } } if *actual == root && *entity == root)
        );
        app.world_mut().entity_mut(root).insert(disabled);
        app.world_mut().entity_mut(root).remove::<Parts>();
        let failure = WidgetryCollapsible::expanded(app.world(), root).unwrap_err();
        assert!(
            matches!(failure.kind(), CollapsibleError::Structure { source: Problem::MissingPart { root: actual, entity, part: "Parts" } } if *actual == root && *entity == root)
        );
    }

    #[test]
    fn disabled_core_failure_preserves_origin_request_and_existing_partial_commit_order() {
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, AssetPlugin::default()))
            .init_asset::<bevy::scene::ScenePatch>();
        // Omit the UI plugin to exercise a real missing DisabledRuntime at the composed boundary.
        let root = app
            .world_mut()
            .spawn_scene(bsn! {
                WidgetryCollapsible
                template(|_| Ok(Expanded(true)))
                template(|_| Ok(HeaderDisabled(false)))
                Node
                Children [
                    Node Visibility::Inherited Children [
                        Node Visibility::Inherited --
                        bevy_widgetry_button::WidgetryButton Node Visibility::Inherited
                    ] --
                    Node Visibility::Inherited
                ]
            })
            .unwrap()
            .id();
        let children = app.world().get::<Children>(root).unwrap();
        let (header, content) = (children[0], children[1]);
        let children = app.world().get::<Children>(header).unwrap();
        let parts = Parts {
            header,
            content,
            header_content: children[0],
            button: children[1],
        };
        app.world_mut().entity_mut(root).insert(parts);
        let logs = LogCapture::default();
        logs.run(|| {
            let failure = WidgetryCollapsible::set_expanded_in_world(app.world_mut(), root, false).unwrap_err();
            assert!(matches!(failure.kind(), CollapsibleError::Core { source: bevy_widgetry_core::diagnostics::CoreError::MissingResource { entity, resource: "DisabledRuntime" } } if *entity == header), "{:?}", failure.kind());
            assert!(failure.contexts().iter().any(|frame| frame.operation() == "collapsible.set_expanded"));
            let origin = failure.origin().clone();
            let boundary = failure.into_bevy_error();
            let diagnostic = boundary.downcast_ref::<bevy_widgetry_diagnostics::bevy::WidgetryEcsError>().unwrap().diagnostics().next().unwrap();
            assert!(Arc::ptr_eq(&origin, diagnostic.origin()));
            assert_eq!(logs.diagnostic_records(origin.id(), DiagnosticPhase::Source).len(), 1);
            assert!(WidgetryCollapsible::expanded(app.world(), root).unwrap());
            let failure = WidgetryCollapsible::set_header_disabled_in_world(app.world_mut(), root, true).unwrap_err();
            assert!(matches!(failure.kind(), CollapsibleError::Core { source: bevy_widgetry_core::diagnostics::CoreError::MissingResource { entity, resource: "DisabledRuntime" } } if *entity == header), "{:?}", failure.kind());
            assert!(failure.contexts().iter().any(|frame| frame.operation() == "collapsible.set_header_disabled"));
            assert!(WidgetryCollapsible::header_disabled(app.world(), root).unwrap());
            assert!(WidgetryCollapsible::expanded(app.world(), root).unwrap());
        });
    }
}
