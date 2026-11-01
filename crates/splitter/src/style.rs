use crate::{
    SplitterLayoutError as Problem,
    error::{LayoutKey, failure, layout_kind},
};
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
use bevy_widgetry_core::ui::WidgetryUiSystems;
use bevy_widgetry_diagnostics::{
    ErrorFrame,
    bevy::{DiagnosticBatch, FailureState, WidgetryEcsError},
};
use bevy_widgetry_theme::WidgetryThemeMode;
use std::sync::Arc;

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
pub(crate) struct StyleDiagnostics(FailureState<LayoutKey>);
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
    let mut batch = DiagnosticBatch::default();
    for root in dirty {
        if world.get::<WidgetrySplitter>(root).is_none() {
            continue;
        }
        let frame = || ErrorFrame::new("splitter.style").field("root", root);
        match apply(world, root) {
            Ok(()) => {
                if let Some(mut diagnostics) = world.get_mut::<StyleDiagnostics>(root) {
                    diagnostics.0.recover(frame);
                }
            }
            Err(problem) => {
                let key = problem.key();
                let kind = layout_kind(
                    root,
                    world.get::<State>(root),
                    world.get::<Parts>(root),
                    problem,
                );
                let create = || -> Arc<dyn bevy_widgetry_diagnostics::bevy::Diagnostic> {
                    Arc::new(failure(kind, frame()))
                };
                let error = if let Some(mut diagnostics) = world.get_mut::<StyleDiagnostics>(root) {
                    diagnostics.0.failure(key, create)
                } else {
                    WidgetryEcsError::from_shared(create())
                };
                for diagnostic in error.shared_diagnostics() {
                    batch.push_shared(diagnostic);
                }
            }
        }
    }
    batch.into_bevy_error().map_or(Ok(()), Err)
}

fn apply(world: &mut World, root: Entity) -> Result<(), Problem> {
    let state = *world.get::<State>(root).ok_or(Problem::MissingPart {
        entity: root,
        part: "State",
    })?;
    let handle = world
        .get::<Parts>(root)
        .ok_or(Problem::MissingPart {
            entity: root,
            part: "Parts",
        })?
        .handle;
    if world.get::<Handle>(handle).is_none() {
        return Err(Problem::MissingPart {
            entity: handle,
            part: "Handle",
        });
    }
    let parent = world.get::<ChildOf>(handle).map(ChildOf::parent);
    if parent != Some(root) {
        return Err(Problem::InvalidParent {
            entity: handle,
            expected: root,
            actual: parent,
        });
    }
    let line = world
        .get::<HandleVisual>(handle)
        .ok_or(Problem::MissingPart {
            entity: handle,
            part: "HandleVisual",
        })?
        .line;
    let parent = world.get::<ChildOf>(line).map(ChildOf::parent);
    if parent != Some(handle) {
        return Err(Problem::InvalidParent {
            entity: line,
            expected: handle,
            actual: parent,
        });
    }
    let disabled = !state.resizable || world.get::<InteractionDisabled>(handle).is_some();
    let palette = world
        .get::<ColorState>(root)
        .ok_or(Problem::MissingPart {
            entity: root,
            part: "ColorState",
        })?
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
        .ok_or(Problem::MissingPart {
            entity: line,
            part: "BackgroundColor",
        })?
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
