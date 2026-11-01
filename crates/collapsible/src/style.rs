use crate::{
    CollapsibleFailure, CollapsibleStructureError as Problem,
    error::{icon, missing, structural},
};
use crate::{collapsible::Parts, colors::ColorState};
use bevy::prelude::*;
use bevy_widgetry_asset::BuiltinIcon;
use bevy_widgetry_core::icon::WidgetryIcon;
use bevy_widgetry_diagnostics::{
    ErrorFrame,
    bevy::{Diagnostic, FailureState, WidgetryEcsError},
};
use bevy_widgetry_theme::WidgetryThemeMode;
use std::sync::Arc;

pub(crate) const BORDER: f32 = 1.0;
pub(crate) const RADIUS: f32 = 6.0;
pub(crate) const PADDING: f32 = 12.0;
pub(crate) const HEADER_PADDING_Y: f32 = 8.0;
pub(crate) const BUTTON_SIZE: f32 = 32.0;
pub(crate) const ICON_SIZE: u32 = 16;

#[derive(Component, Clone, Default)]
pub(crate) struct Chevron;

pub(crate) fn arrow(expanded: bool) -> BuiltinIcon {
    if expanded {
        BuiltinIcon::ChevronUp
    } else {
        BuiltinIcon::ChevronDown
    }
}

pub(crate) fn set_arrow(
    world: &mut World,
    root: Entity,
    button: Entity,
    expanded: bool,
) -> Result<(), CollapsibleFailure> {
    let mut pending = vec![button];
    while let Some(entity) = pending.pop() {
        if world.get::<Chevron>(entity).is_some() {
            WidgetryIcon::set_svg_in_world(world, entity, arrow(expanded).path()).map_err(
                |error| {
                    icon(
                        error,
                        ErrorFrame::new("collapsible.set_arrow")
                            .field("root", root)
                            .field("button", button)
                            .field("icon", entity)
                            .field("expanded", expanded),
                    )
                },
            )?;
            return Ok(());
        }
        if let Some(children) = world.get::<Children>(entity) {
            pending.extend(children.iter());
        }
    }
    Err(missing(root, button, "Chevron"))
}

pub(crate) fn install(app: &mut App) {
    app.add_systems(
        PostUpdate,
        update.in_set(bevy_widgetry_core::ui::WidgetryUiSystems::Colors),
    );
}

fn update(
    mode: Res<WidgetryThemeMode>,
    roots: Query<(Entity, Ref<ColorState>, Ref<Parts>)>,
    mut commands: Commands,
) {
    for (root, colors, parts) in &roots {
        if mode.is_changed() || colors.is_changed() || parts.is_added() {
            commands.queue(move |world: &mut World| apply(world, root));
        }
    }
}

#[derive(Component, Default)]
pub(crate) struct StyleDiagnostics(FailureState<Problem>);

fn apply(world: &mut World, root: Entity) -> Result {
    if !world.entities().contains(root) {
        return Ok(());
    }
    let frame = || ErrorFrame::new("collapsible.style").field("root", root);
    match apply_colors(world, root) {
        Ok(()) => {
            if let Some(mut state) = world.get_mut::<StyleDiagnostics>(root) {
                state.0.recover(frame);
            }
            Ok(())
        }
        Err(problem) => {
            let key = problem.clone();
            let create = || -> Arc<dyn Diagnostic> { Arc::new(structural(problem, frame())) };
            let error = if let Some(mut state) = world.get_mut::<StyleDiagnostics>(root) {
                state.0.failure(key, create)
            } else {
                WidgetryEcsError::from_shared(create())
            };
            Err(error.into_bevy_error())
        }
    }
}

fn apply_colors(world: &mut World, root: Entity) -> Result<(), Problem> {
    let parts = crate::state::inspect_structure(world, root)?;
    let colors = world
        .get::<ColorState>(root)
        .ok_or(Problem::MissingPart {
            root,
            entity: root,
            part: "ColorState",
        })?
        .0
        .resolve(&world.resource::<WidgetryThemeMode>().colors().collapsible);
    for entity in [root, parts.header, parts.content] {
        if world.get::<BackgroundColor>(entity).is_none() {
            return Err(Problem::MissingPart {
                root,
                entity,
                part: "BackgroundColor",
            });
        }
    }
    for entity in [root, parts.content] {
        if world.get::<BorderColor>(entity).is_none() {
            return Err(Problem::MissingPart {
                root,
                entity,
                part: "BorderColor",
            });
        }
    }
    for (entity, color) in [
        (root, colors.background),
        (parts.header, colors.header_background),
        (parts.content, colors.content_background),
    ] {
        if let Some(mut background) = world.get_mut::<BackgroundColor>(entity) {
            background.set_if_neq(BackgroundColor(color));
        }
    }
    for (entity, color) in [(root, colors.border), (parts.content, colors.separator)] {
        if let Some(mut border) = world.get_mut::<BorderColor>(entity) {
            border.set_if_neq(BorderColor::all(color));
        }
    }
    Ok(())
}
