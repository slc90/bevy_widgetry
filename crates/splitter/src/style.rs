use crate::{
    WidgetrySplitter,
    colors::ColorState,
    input::Handle,
    splitter::{Parts, State},
};
use bevy::{
    ecs::entity::EntityHashSet,
    picking::{cursor::EntityCursor, hover::Hovered},
    prelude::*,
    ui::{InteractionDisabled, Pressed},
    window::SystemCursorIcon,
};
use bevy_widgetry_core::{diagnostics::FailureState, ui::WidgetryUiSystems};
use bevy_widgetry_internal_log::{widgetry_error, widgetry_info};
use bevy_widgetry_theme::WidgetryThemeMode;

#[derive(Component, FromTemplate)]
struct HandleVisual {
    line: Entity,
}
#[derive(Component, Default, Clone)]
struct Line;

pub(crate) fn handle_line(horizontal: bool, thickness: f32) -> impl Scene {
    bsn! {
        HandleVisual { line:#SplitterLine }
        Children [
            #SplitterLine Line Pickable::IGNORE
            Node {
                width: {if horizontal {px(thickness)} else {percent(100)}},
                height: {if horizontal {percent(100)} else {px(thickness)}},
                flex_shrink: 0.0,
            }
            BackgroundColor(Color::NONE)
        ]
    }
}
#[derive(Component, Default)]
pub(crate) struct StyleDiagnostics(FailureState);
#[derive(Resource, Default)]
struct Changes(EntityHashSet);

pub(crate) fn install(app: &mut App) {
    app.init_resource::<Changes>()
        .add_observer(removed)
        .add_systems(
            PostUpdate,
            (collect, update).chain().in_set(WidgetryUiSystems::Colors),
        );
}

fn root_of(
    mut entity: Entity,
    parents: &Query<&ChildOf>,
    roots: &Query<Entity, With<WidgetrySplitter>>,
) -> Option<Entity> {
    loop {
        if roots.contains(entity) {
            return Some(entity);
        }
        entity = parents.get(entity).ok()?.parent();
    }
}

fn removed(
    event: On<
        Remove<(
            Hovered,
            Pressed,
            InteractionDisabled,
            ColorState,
            HandleVisual,
            BackgroundColor,
            ChildOf,
        )>,
    >,
    parents: Query<&ChildOf>,
    roots: Query<Entity, With<WidgetrySplitter>>,
    managed: Query<(), Or<(With<WidgetrySplitter>, With<Handle>, With<Line>)>>,
    mut changes: ResMut<Changes>,
) {
    if !managed.contains(event.entity) {
        return;
    }
    if let Some(root) = root_of(event.entity, &parents, &roots) {
        changes.0.insert(root);
    }
}

fn collect(
    mode: Res<WidgetryThemeMode>,
    roots: Query<Entity, With<WidgetrySplitter>>,
    inputs: Query<
        Entity,
        (
            Or<(With<WidgetrySplitter>, With<Handle>, With<Line>)>,
            Or<(
                Added<WidgetrySplitter>,
                Changed<State>,
                Changed<ColorState>,
                Changed<Hovered>,
                Added<Pressed>,
                Added<InteractionDisabled>,
                Added<HandleVisual>,
                Added<BackgroundColor>,
                Changed<ChildOf>,
            )>,
        ),
    >,
    parents: Query<&ChildOf>,
    mut changes: ResMut<Changes>,
) {
    if mode.is_changed() {
        changes.0.extend(roots.iter());
    }
    for entity in &inputs {
        if let Some(root) = root_of(entity, &parents, &roots) {
            changes.0.insert(root);
        }
    }
}

fn update(world: &mut World) -> Result {
    let dirty = std::mem::take(&mut world.resource_mut::<Changes>().0);
    let mut failure = None;
    for root in dirty {
        if world.get::<WidgetrySplitter>(root).is_none() {
            continue;
        }
        let result = apply(world, root);
        let result = if let Some(mut diagnostics) = world.get_mut::<StyleDiagnostics>(root) {
            diagnostics.0.observe(
                result,
                |error| widgetry_error!(?root,%error,"Splitter 样式内部结构失效"),
                || widgetry_info!(?root, "Splitter 样式内部结构恢复正常"),
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
    failure.map_or(Ok(()), Err)
}

fn apply(world: &mut World, root: Entity) -> Result {
    let state = *world
        .get::<State>(root)
        .ok_or_else(|| BevyError::error("Splitter runtime state is missing"))?;
    let handle = world
        .get::<Parts>(root)
        .ok_or_else(|| BevyError::error("Splitter parts are missing"))?
        .handle;
    if world
        .get::<ChildOf>(handle)
        .is_none_or(|p| p.parent() != root)
        || world.get::<Handle>(handle).is_none()
    {
        return Err(BevyError::error("Splitter Handle owner is invalid"));
    }
    let line = world
        .get::<HandleVisual>(handle)
        .ok_or_else(|| BevyError::error("Splitter Handle visual is missing"))?
        .line;
    if world
        .get::<ChildOf>(line)
        .is_none_or(|p| p.parent() != handle)
    {
        return Err(BevyError::error("Splitter line owner is invalid"));
    }
    let disabled = !state.resizable || world.get::<InteractionDisabled>(handle).is_some();
    let palette = world
        .get::<ColorState>(root)
        .ok_or_else(|| BevyError::error("Splitter colors are missing"))?
        .0
        .resolve(&world.resource::<WidgetryThemeMode>().colors().splitter);
    let color = if disabled {
        palette.disabled
    } else if world.get::<Pressed>(handle).is_some() {
        palette.dragging
    } else if world.get::<Hovered>(handle).is_some_and(|h| h.0) {
        palette.hovered
    } else {
        palette.normal
    };
    world
        .get_mut::<BackgroundColor>(line)
        .ok_or_else(|| BevyError::error("Splitter line background is missing"))?
        .set_if_neq(BackgroundColor(color));
    if disabled {
        if world.get::<EntityCursor>(handle).is_some() {
            world.entity_mut(handle).remove::<EntityCursor>();
        }
    } else {
        let cursor = EntityCursor::System(match state.orientation {
            crate::WidgetrySplitterOrientation::Horizontal => SystemCursorIcon::EwResize,
            crate::WidgetrySplitterOrientation::Vertical => SystemCursorIcon::NsResize,
        });
        if world.get::<EntityCursor>(handle) != Some(&cursor) {
            world.entity_mut(handle).insert(cursor);
        }
    }
    Ok(())
}
