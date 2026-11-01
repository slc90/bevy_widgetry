use crate::WidgetryTabs;
use crate::tabs::{Parts, State, TabOwner};
use crate::{TabsFailure, error::missing};
use bevy::ecs::entity::EntityHashSet;
use bevy::prelude::*;
use bevy_widgetry_diagnostics::{ErrorFrame, bevy::DiagnosticBatch};
use bevy_widgetry_scroll_area::{
    WidgetryScrollArea, WidgetryScrollAreaContent, WidgetryScrollAreaViewport,
    WidgetryScrollIntoView,
};

#[derive(Resource, Default)]
struct Pending {
    roots: EntityHashSet,
    layout_changed: EntityHashSet,
}

#[derive(Component)]
struct BarViewport {
    root: Entity,
    geometry: Option<(Vec2, Vec2, f32)>,
}

#[derive(Component, Default)]
pub(crate) struct HeaderGeometry(Option<(Vec2, f32)>);

pub(crate) fn initialize(world: &mut World, bar: Entity, root: Entity) -> Result<(), TabsFailure> {
    let viewport = viewport(world, root, bar)?;
    world.entity_mut(viewport).insert(BarViewport {
        root,
        geometry: None,
    });
    Ok(())
}

pub(crate) fn install(app: &mut App) {
    app.init_resource::<Pending>().add_systems(
        PostUpdate,
        (collect, locate)
            .chain()
            .after(bevy::ui::UiSystems::PostLayout),
    );
}

fn viewport(world: &World, root: Entity, bar: Entity) -> Result<Entity, TabsFailure> {
    if world.get::<WidgetryScrollArea>(bar).is_none() {
        return Err(missing(root, bar, "WidgetryScrollArea"));
    }
    let viewport = world
        .get::<Children>(bar)
        .and_then(|children| {
            children
                .iter()
                .find(|entity| world.get::<WidgetryScrollAreaViewport>(*entity).is_some())
        })
        .ok_or_else(|| missing(root, bar, "WidgetryScrollAreaViewport"))?;
    for (present, part) in [
        (world.get::<Node>(viewport).is_some(), "Node"),
        (
            world.get::<ComputedNode>(viewport).is_some(),
            "ComputedNode",
        ),
        (
            world.get::<UiGlobalTransform>(viewport).is_some(),
            "UiGlobalTransform",
        ),
        (
            world.get::<bevy::ui::ScrollPosition>(viewport).is_some(),
            "ScrollPosition",
        ),
    ] {
        if !present {
            return Err(missing(root, viewport, part));
        }
    }
    Ok(viewport)
}

pub(crate) fn strip(world: &World, root: Entity, bar: Entity) -> Result<Entity, TabsFailure> {
    let viewport = viewport(world, root, bar)?;
    let content = world
        .get::<Children>(viewport)
        .and_then(|children| {
            children
                .iter()
                .find(|entity| world.get::<WidgetryScrollAreaContent>(*entity).is_some())
        })
        .ok_or_else(|| missing(root, viewport, "WidgetryScrollAreaContent"))?;
    if world.get::<Node>(content).is_none() {
        return Err(missing(root, content, "Node"));
    }
    Ok(content)
}

fn collect(
    changed: Query<Entity, (With<WidgetryTabs>, Changed<State>)>,
    mut headers: Query<(&TabOwner, &ComputedNode, &mut HeaderGeometry), Changed<ComputedNode>>,
    mut viewports: Query<(&ComputedNode, &mut BarViewport), Changed<ComputedNode>>,
    mut pending: ResMut<Pending>,
) {
    pending.layout_changed.clear();
    pending.roots.extend(changed.iter());
    for (owner, node, mut previous) in &mut headers {
        let geometry = (node.size(), node.inverse_scale_factor);
        if previous.0 != Some(geometry) {
            previous.0 = Some(geometry);
            pending.roots.insert(owner.root);
            pending.layout_changed.insert(owner.root);
        }
    }
    for (node, mut owner) in &mut viewports {
        let geometry = (node.size(), node.content_size(), node.inverse_scale_factor);
        if owner.geometry != Some(geometry) {
            owner.geometry = Some(geometry);
            pending.roots.insert(owner.root);
            pending.layout_changed.insert(owner.root);
        }
    }
}

fn locate(world: &mut World) -> Result {
    let roots = std::mem::take(&mut world.resource_mut::<Pending>().roots);
    let mut batch = DiagnosticBatch::default();
    for root in roots {
        if world.get::<WidgetryTabs>(root).is_none() {
            continue;
        }
        if world.resource::<Pending>().layout_changed.contains(&root) {
            world.resource_mut::<Pending>().roots.insert(root);
            world.write_message(bevy::window::RequestRedraw);
            continue;
        }
        if let Err(error) = locate_one(world, root) {
            batch.push(
                error.with_context(ErrorFrame::new("tabs.locate_selected").field("root", root)),
            );
        }
    }
    batch.into_bevy_error().map_or(Ok(()), Err)
}

fn locate_one(world: &mut World, root: Entity) -> Result<(), TabsFailure> {
    let Some(state) = world.get::<State>(root) else {
        return Ok(());
    };
    let Some(pair) = state.pairs.get(state.selected) else {
        return Ok(());
    };
    let header = pair.header;
    for (present, part) in [
        (world.get::<ComputedNode>(header).is_some(), "ComputedNode"),
        (
            world.get::<UiGlobalTransform>(header).is_some(),
            "UiGlobalTransform",
        ),
    ] {
        if !present {
            return Err(missing(root, header, part));
        }
    }
    let bar = world
        .get::<Parts>(root)
        .ok_or_else(|| missing(root, root, "Parts"))?
        .bar;
    let view = viewport(world, root, bar)?;
    if [header, view].iter().any(|entity| {
        world
            .get::<ComputedNode>(*entity)
            .is_none_or(|node| node.is_empty())
    }) {
        world.resource_mut::<Pending>().roots.insert(root);
        return Ok(());
    }
    let before = world
        .get::<bevy::ui::ScrollPosition>(view)
        .map(|position| position.0);
    // Bevy clamps the rendered offset after a resize but retains the raw request.
    // IntoView uses that request as its geometry origin, so reconcile it first.
    if let Some(node) = world.get::<ComputedNode>(view) {
        let max = (node.content_size() - node.size()).max(Vec2::ZERO) * node.inverse_scale_factor;
        if let Some(mut position) = world.get_mut::<bevy::ui::ScrollPosition>(view) {
            let next = position.0.clamp(Vec2::ZERO, max);
            if position.0 != next {
                position.0 = next;
            }
        }
    }
    world.trigger(WidgetryScrollIntoView { entity: header });
    if world
        .get::<bevy::ui::ScrollPosition>(view)
        .map(|position| position.0)
        != before
    {
        world.write_message(bevy::window::RequestRedraw);
    }
    Ok(())
}

#[cfg(test)]
#[allow(clippy::disallowed_macros, clippy::unwrap_used)]
mod tests {
    use super::*;
    use crate::{TabsError, TabsStructureError, WidgetryTab, WidgetryTabsPlugin};
    use bevy_widgetry_test_utils::{ErrorCapture, scene_app};

    #[test]
    fn locating_two_broken_headers_reports_both_owners_at_the_system_boundary() {
        let mut app = scene_app();
        app.add_plugins(WidgetryTabsPlugin);
        app.add_systems(Update, locate);
        app.set_error_handler(ErrorCapture::handler());
        let roots: Vec<_> = (0..2).map(|_| app.world_mut().spawn_scene(bsn! {
            @WidgetryTabs { @children: bsn! { @WidgetryTab { @header: bsn! { Node }, @content: bsn! { Node } } } }
        }).unwrap().id()).collect();
        app.update();
        let headers: Vec<_> = roots
            .iter()
            .map(|root| WidgetryTabs::header(app.world(), *root, 0).unwrap())
            .collect();
        for header in &headers {
            app.world_mut().entity_mut(*header).remove::<ComputedNode>();
        }
        let mut pending = app.world_mut().resource_mut::<Pending>();
        pending.layout_changed.clear();
        pending.roots.extend(roots.iter().copied());
        let errors = ErrorCapture::default();
        errors.run(|| app.world_mut().run_schedule(Update));
        let records = errors.take_records();
        assert_eq!(records.len(), 1);
        assert_eq!(
            records[0].error.severity(),
            bevy::ecs::error::Severity::Error
        );
        let diagnostics: Vec<_> = records[0].managed().unwrap().diagnostics().collect();
        assert_eq!(diagnostics.len(), 2);
        for diagnostic in diagnostics {
            let kind = diagnostic
                .source()
                .unwrap()
                .downcast_ref::<TabsError>()
                .unwrap();
            assert!(
                matches!(kind, TabsError::Structure { source: TabsStructureError::MissingPart { root, entity, part: "ComputedNode" } } if roots.iter().zip(&headers).any(|(r, h)| r == root && h == entity))
            );
            assert!(
                diagnostic
                    .contexts()
                    .iter()
                    .any(|frame| frame.operation() == "tabs.locate_selected")
            );
        }
    }
}
