use crate::{
    UiScaleError, UiScaleFailure, WidgetryUiScale, WindowScaleExcluded,
    geometry::WindowSize,
    native::{self, Environment, WorkAreaFault},
};
use bevy::{
    camera::CameraUpdateSystems,
    ecs::system::NonSendMarker,
    prelude::*,
    window::{Monitor, PrimaryMonitor, WindowResized, WindowScaleFactorChanged},
    winit::WinitMonitors,
};
use bevy_widgetry_core::window_scale::WidgetryUiScaleSystems;
use bevy_widgetry_diagnostics::{
    ErrorFrame,
    bevy::{DiagnosticBatch, FailureState, WidgetryEcsError},
};
use bevy_widgetry_internal_log::widgetry_debug;
use std::sync::Arc;

pub struct WidgetryUiScalePlugin;

impl Plugin for WidgetryUiScalePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<WidgetryUiScale>()
            .add_message::<WindowResized>()
            .add_message::<WindowScaleFactorChanged>()
            .register_type::<WindowScaleExcluded>()
            .add_observer(window_added)
            .add_observer(window_removed)
            .add_observer(exclusion_added)
            .add_observer(exclusion_removed)
            .configure_sets(
                PostUpdate,
                WidgetryUiScaleSystems::Sync.before(CameraUpdateSystems),
            )
            .add_systems(PostUpdate, sync.in_set(WidgetryUiScaleSystems::Sync));
        widgetry_debug!(plugin = "WidgetryUiScalePlugin", "插件注册完成");
    }
}

#[derive(Component)]
struct ManagedWindow {
    size: WindowSize,
    active: bool,
    failure: FailureState<WorkAreaFault>,
}

impl ManagedWindow {
    fn new(window: &Window) -> Self {
        Self {
            size: WindowSize::new(window),
            active: true,
            failure: default(),
        }
    }

    fn prepare(
        &mut self,
        entity: Entity,
        window: &Window,
        scale: f32,
        environment: Result<Environment, WorkAreaFault>,
    ) -> Result<Option<Environment>, WidgetryEcsError> {
        let environment =
            match environment {
                Ok(environment) => environment,
                Err(fault) => {
                    let previous = self
                        .failure
                        .active()
                        .map(|diagnostic| diagnostic.origin().id().clone());
                    let error = self.failure.failure(fault, || {
                        Arc::new(UiScaleFailure::new(
                            UiScaleError::WorkArea {
                                window: entity,
                                point: fault.point,
                                source: fault.source,
                            },
                            ErrorFrame::new("ui_scale.work_area")
                                .field("window", entity)
                                .field("point", format_args!("{:?}", fault.point)),
                        ))
                    });
                    if error.diagnostics().next().is_some_and(|diagnostic| {
                        previous.as_ref() != Some(diagnostic.origin().id())
                    }) {
                        return Err(error);
                    }
                    return Ok(None);
                }
            };
        if environment.desktop.is_some() {
            self.failure.recover(|| {
                ErrorFrame::new("ui_scale.work_area.recovered").field("window", entity)
            });
        } else if self.failure.active().is_some() {
            return Ok(None);
        }
        if !self.active {
            self.size.resume(window);
            self.active = true;
        }
        if self.size.needs_update(window, scale, environment) {
            Ok(Some(environment))
        } else {
            Ok(None)
        }
    }
}

fn initialize(world: &mut World, entity: Entity) -> Result {
    if world.get::<WindowScaleExcluded>(entity).is_some() {
        return Ok(());
    }
    let Some(window) = world.get::<Window>(entity).cloned() else {
        return Ok(());
    };
    let scale = world.resource::<WidgetryUiScale>().get();
    let mut monitors = world.query::<(Entity, &Monitor, Has<PrimaryMonitor>)>();
    let requested_position = world
        .get::<ManagedWindow>(entity)
        .is_none_or(|managed| managed.size.position_changed(&window));
    let environment = native::environment(
        entity,
        &window,
        monitors.iter(world),
        world.get_resource::<WinitMonitors>(),
        requested_position,
    );
    let mut managed = world
        .entity_mut(entity)
        .take::<ManagedWindow>()
        .unwrap_or_else(|| ManagedWindow::new(&window));
    let outcome = {
        let Some(mut window) = world.get_mut::<Window>(entity) else {
            return Ok(());
        };
        managed
            .prepare(entity, &window, scale, environment)
            .map(|environment| {
                environment
                    .map(|environment| managed.size.apply(entity, &mut window, scale, environment))
            })
    };
    world.entity_mut(entity).insert(managed);
    if let Some(changes) = outcome.map_err(WidgetryEcsError::into_bevy_error)? {
        world.write_message_batch(changes.resized);
        world.write_message_batch(changes.rescaled);
    }
    Ok(())
}

fn window_added(event: On<Add<Window>>, mut commands: Commands) {
    let entity = event.entity;
    commands.queue(move |world: &mut World| initialize(world, entity));
}

fn window_removed(event: On<Remove<Window>>, mut commands: Commands) {
    commands.entity(event.entity).try_remove::<ManagedWindow>();
}

fn exclusion_added(event: On<Add<WindowScaleExcluded>>, mut commands: Commands) {
    let entity = event.entity;
    commands.queue(move |world: &mut World| {
        if let Some(mut managed) = world.get_mut::<ManagedWindow>(entity) {
            managed.active = false;
        }
    });
}

fn exclusion_removed(event: On<Remove<WindowScaleExcluded>>, mut commands: Commands) {
    let entity = event.entity;
    commands.queue(move |world: &mut World| initialize(world, entity));
}

fn sync(
    _main_thread: NonSendMarker,
    scale: Res<WidgetryUiScale>,
    monitors: Query<(Entity, &Monitor, Has<PrimaryMonitor>)>,
    native_monitors: Option<Res<WinitMonitors>>,
    mut windows: Query<
        (Entity, &mut Window, Option<&mut ManagedWindow>),
        Without<WindowScaleExcluded>,
    >,
    mut commands: Commands,
    mut resized: MessageWriter<WindowResized>,
    mut rescaled: MessageWriter<WindowScaleFactorChanged>,
) -> Result {
    let mut failures = DiagnosticBatch::default();
    for (entity, mut window, managed) in &mut windows {
        let requested_position = managed
            .as_ref()
            .is_none_or(|managed| managed.size.position_changed(&window));
        let environment = native::environment(
            entity,
            &window,
            monitors.iter(),
            native_monitors.as_deref(),
            requested_position,
        );
        let mut new = None;
        let managed = match managed {
            Some(managed) => managed.into_inner(),
            None => new.insert(ManagedWindow::new(&window)),
        };
        match managed.prepare(entity, &window, scale.get(), environment) {
            Ok(Some(environment)) => {
                let changes = managed
                    .size
                    .apply(entity, &mut window, scale.get(), environment);
                resized.write_batch(changes.resized);
                rescaled.write_batch(changes.rescaled);
            }
            Ok(None) => {}
            Err(error) => {
                for diagnostic in error.shared_diagnostics() {
                    failures.push_shared(diagnostic);
                }
            }
        }
        if let Some(managed) = new {
            commands.entity(entity).insert(managed);
        }
    }
    match failures.into_bevy_error() {
        Some(error) => Err(error),
        None => Ok(()),
    }
}

#[cfg(test)]
#[allow(clippy::disallowed_macros, clippy::unwrap_used)]
mod tests {
    use super::*;
    use bevy_widgetry_test_utils::{DiagnosticPhase, LogCapture};

    #[test]
    fn work_area_fault_retains_source_and_identity_until_recovery() {
        let logs = LogCapture::default();
        logs.run(|| {
            let window = Window::default();
            let initial_size = window.physical_size();
            let mut managed = ManagedWindow::new(&window);
            let fault = WorkAreaFault { point: IVec2::new(-1920, 100), source: winsafe::co::ERROR::INVALID_HANDLE };
            let first = managed.prepare(Entity::PLACEHOLDER, &window, 2.0, Err(fault)).err().unwrap();
            let diagnostic = first.diagnostics().next().unwrap();
            let origin = Arc::clone(diagnostic.origin());
            let failure = (diagnostic as &dyn std::error::Error).downcast_ref::<UiScaleFailure>().unwrap();
            assert!(matches!(failure.kind(), UiScaleError::WorkArea { window: entity, point, source } if *entity == Entity::PLACEHOLDER && *point == fault.point && *source == fault.source));
            assert_eq!(window.physical_size(), initial_size);
            assert!(managed.prepare(Entity::PLACEHOLDER, &window, 2.0, Err(fault)).unwrap().is_none());
            assert!(Arc::ptr_eq(&origin, managed.failure.active().unwrap().origin()));
            assert_eq!(logs.diagnostic_records(origin.id(), DiagnosticPhase::Source).len(), 1);
            assert!(logs.diagnostic_records(origin.id(), DiagnosticPhase::Handled).is_empty());
            managed.prepare(Entity::PLACEHOLDER, &window, 2.0, Ok(Environment::default())).unwrap();
            assert!(logs.records_for_error(origin.id()).iter().all(|record| !record.matches_field("phase", "recovered")));
            assert!(managed.prepare(Entity::PLACEHOLDER, &window, 2.0, Err(fault)).unwrap().is_none());
            assert!(Arc::ptr_eq(&origin, managed.failure.active().unwrap().origin()));
            assert_eq!(logs.diagnostic_records(origin.id(), DiagnosticPhase::Source).len(), 1);
            assert!(logs.diagnostic_records(origin.id(), DiagnosticPhase::Handled).is_empty());
            let recovered = Environment {
                desktop: Some(native::Desktop {
                    work: IRect::new(0, 0, 3840, 2160),
                    base_scale: 2.0,
                    chrome: UVec2::ZERO,
                }),
                suspended: false,
            };
            assert!(managed.prepare(Entity::PLACEHOLDER, &window, 2.0, Ok(recovered)).unwrap().is_some());
            assert_eq!(logs.records_for_error(origin.id()).iter().filter(|record| record.matches_field("phase", "recovered")).count(), 1);
            let next = managed.prepare(Entity::PLACEHOLDER, &window, 2.0, Err(fault)).err().unwrap();
            assert_ne!(next.diagnostics().next().unwrap().origin().id(), origin.id());
        });
    }
}
