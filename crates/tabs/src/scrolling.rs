use crate::WidgetryTabs;
use crate::tabs::{Parts, State, TabOwner};
use bevy::ecs::entity::EntityHashSet;
use bevy::prelude::*;
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

pub(crate) fn initialize(world: &mut World, bar: Entity, root: Entity) -> Result<(), BevyError> {
    let viewport = viewport(world, bar)?;
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

fn viewport(world: &World, bar: Entity) -> Result<Entity, BevyError> {
    if world.get::<WidgetryScrollArea>(bar).is_none() {
        return Err(crate::selection::invalid("Tabs bar ScrollArea is missing"));
    }
    let viewport = world
        .get::<Children>(bar)
        .and_then(|children| {
            children
                .iter()
                .find(|entity| world.get::<WidgetryScrollAreaViewport>(*entity).is_some())
        })
        .ok_or_else(|| crate::selection::invalid("Tabs bar viewport is missing"))?;
    if world.get::<Node>(viewport).is_none()
        || world.get::<ComputedNode>(viewport).is_none()
        || world.get::<UiGlobalTransform>(viewport).is_none()
        || world.get::<bevy::ui::ScrollPosition>(viewport).is_none()
    {
        return Err(crate::selection::invalid(
            "Tabs bar viewport structure is invalid",
        ));
    }
    Ok(viewport)
}

pub(crate) fn strip(world: &World, bar: Entity) -> Result<Entity, BevyError> {
    let viewport = viewport(world, bar)?;
    let content = world
        .get::<Children>(viewport)
        .and_then(|children| {
            children
                .iter()
                .find(|entity| world.get::<WidgetryScrollAreaContent>(*entity).is_some())
        })
        .ok_or_else(|| crate::selection::invalid("Tabs bar content is missing"))?;
    if world.get::<Node>(content).is_none() {
        return Err(crate::selection::invalid(
            "Tabs bar content structure is invalid",
        ));
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

fn locate(world: &mut World) -> Result<(), BevyError> {
    let roots = std::mem::take(&mut world.resource_mut::<Pending>().roots);
    let mut failure = None;
    for root in roots {
        if world.get::<WidgetryTabs>(root).is_none() {
            continue;
        }
        if world.resource::<Pending>().layout_changed.contains(&root) {
            world.resource_mut::<Pending>().roots.insert(root);
            world.write_message(bevy::window::RequestRedraw);
            continue;
        }
        if let Err(error) = locate_one(world, root)
            && failure.is_none()
        {
            failure = Some(error);
        }
    }
    failure.map_or(Ok(()), Err)
}

fn locate_one(world: &mut World, root: Entity) -> Result<(), BevyError> {
    let Some(state) = world.get::<State>(root) else {
        return Ok(());
    };
    let Some(pair) = state.pairs.get(state.selected) else {
        return Ok(());
    };
    let header = pair.header;
    if world.get::<ComputedNode>(header).is_none()
        || world.get::<UiGlobalTransform>(header).is_none()
    {
        return Err(crate::selection::invalid(
            "Tabs selected header geometry is missing",
        ));
    }
    let bar = world
        .get::<Parts>(root)
        .ok_or_else(|| crate::selection::invalid("Tabs parts are missing"))?
        .bar;
    let view = viewport(world, bar)?;
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
