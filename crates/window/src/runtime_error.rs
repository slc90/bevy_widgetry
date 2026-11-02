use crate::{
    WindowError, WindowFailure, WindowFault,
    window_root::{OwnedWindow, WindowRoot},
};
use bevy::prelude::*;
use bevy_widgetry_diagnostics::{
    ErrorFrame,
    bevy::{DiagnosticBatch, FailureState, WidgetryEcsError},
};
use std::{collections::HashMap, sync::Arc};

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum Phase {
    Title,
    Style,
}

#[derive(Resource, Default)]
pub(crate) struct WindowDiagnostics(HashMap<(Entity, Phase), FailureState<WindowFault>>);

pub(crate) fn frame(world: &World, root: Entity, operation: &'static str) -> ErrorFrame {
    let native = world.get::<WindowRoot>(root).map(|root| root.target_window);
    let camera = world.get::<UiTargetCamera>(root).map(|camera| camera.0);
    ErrorFrame::new(operation)
        .field("root", root)
        .field("native", format_args!("{native:?}"))
        .field("camera", format_args!("{camera:?}"))
        .field(
            "owned",
            format_args!(
                "{:?}",
                world
                    .get::<OwnedWindow>(root)
                    .map(|owned| (owned.window, owned.camera))
            ),
        )
        .field(
            "native_properties",
            format_args!(
                "{:?}",
                native
                    .and_then(|native| world.get::<Window>(native))
                    .map(|window| (
                        window.transparent,
                        window.decorations,
                        window.composite_alpha_mode
                    ))
            ),
        )
        .field(
            "camera_target",
            format_args!(
                "{:?}",
                camera.and_then(|camera| world.get::<bevy::camera::RenderTarget>(camera))
            ),
        )
}

#[track_caller]
pub(crate) fn failure(
    world: &World,
    root: Entity,
    fault: WindowFault,
    operation: &'static str,
) -> WindowFailure {
    let frame = frame(world, root, operation).field("failure", format_args!("{fault:?}"));
    WindowFailure::new(WindowError::Structure { fault }, frame)
}

pub(crate) fn observe(
    world: &mut World,
    root: Entity,
    phase: Phase,
    result: Result<(), WindowFault>,
    operation: &'static str,
) -> Result<(), WidgetryEcsError> {
    world.resource_scope(|world, mut diagnostics: Mut<WindowDiagnostics>| {
        let key = (root, phase);
        match result {
            Err(fault) => Err(diagnostics
                .0
                .entry(key)
                .or_default()
                .failure(fault.clone(), || {
                    Arc::new(failure(world, root, fault, operation))
                })),
            Ok(()) => {
                if let Some(mut state) = diagnostics.0.remove(&key) {
                    state.recover(|| frame(world, root, operation));
                }
                Ok(())
            }
        }
    })
}

pub(crate) fn removed(event: On<Remove<WindowRoot>>, mut diagnostics: ResMut<WindowDiagnostics>) {
    diagnostics.0.retain(|(root, _), _| *root != event.entity);
}

pub(crate) fn finish(errors: impl IntoIterator<Item = WidgetryEcsError>) -> Result {
    let mut batch = DiagnosticBatch::default();
    for error in errors {
        for diagnostic in error.shared_diagnostics() {
            batch.push_shared(diagnostic);
        }
    }
    batch.into_bevy_error().map_or(Ok(()), Err)
}

#[cfg(test)]
#[allow(clippy::disallowed_macros, clippy::unwrap_used)]
mod tests {
    use super::*;
    use crate::{
        WidgetryWindowBackground, WidgetryWindowControlsConfig, WidgetryWindowPlugin,
        WidgetryWindowTitle, owned_widgetry_window, style::WindowStyle, title::TitleState,
    };
    use bevy::ecs::schedule::SingleThreadedExecutor;
    use bevy_widgetry_test_utils::{ErrorCapture, scene_app};

    #[test]
    fn missing_derived_states_keep_diagnostics_outside_the_lost_components() {
        let mut app = scene_app();
        app.add_plugins(WidgetryWindowPlugin);
        app.set_error_handler(ErrorCapture::handler());
        app.edit_schedule(PostUpdate, |schedule| {
            schedule.set_executor(SingleThreadedExecutor::new());
        });
        let root = app.world_mut().spawn_scene(bsn! {
            @owned_widgetry_window(Window::default(), WidgetryWindowControlsConfig::default(), WidgetryWindowBackground::Theme, Default::default(), bsn_list! { WidgetryWindowTitle }, bsn_list! {})
        }).unwrap().id();
        app.update();
        app.world_mut()
            .entity_mut(root)
            .remove::<(TitleState, WindowStyle)>();
        let errors = ErrorCapture::default();
        let mut origins = std::collections::HashSet::new();
        for _ in 0..2 {
            errors.run(|| app.update());
            let records = errors.take_records();
            let diagnostics = records
                .iter()
                .flat_map(|record| record.managed().unwrap().diagnostics())
                .collect::<Vec<_>>();
            assert_eq!(diagnostics.len(), 2);
            for diagnostic in diagnostics {
                origins.insert(diagnostic.origin().id().clone());
            }
        }
        assert_eq!(origins.len(), 2);
        app.world_mut()
            .entity_mut(root)
            .insert((TitleState::default(), WindowStyle::default()));
        errors.run(|| app.update());
        assert!(errors.take_records().is_empty());
        assert!(app.world().resource::<WindowDiagnostics>().0.is_empty());
        app.world_mut().entity_mut(root).remove::<TitleState>();
        errors.run(|| app.update());
        assert_eq!(errors.take_records().len(), 1);
        app.world_mut().despawn(root);
        assert!(app.world().resource::<WindowDiagnostics>().0.is_empty());
    }
}
