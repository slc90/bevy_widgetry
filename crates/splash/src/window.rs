use crate::{
    SplashError, SplashFailure,
    error::failure,
    geometry::{self, Placement},
};
use bevy::{
    camera::CameraUpdateSystems,
    ecs::system::NonSendMarker,
    prelude::*,
    window::{
        Monitor, PrimaryMonitor, WindowMode, WindowPosition, WindowResizeConstraints,
        WindowResized, WindowScaleFactorChanged,
    },
    winit::WinitMonitors,
};
use bevy_widgetry_core::window_scale::WindowScaleExcluded;
use bevy_widgetry_diagnostics::{
    ErrorFrame,
    bevy::{DiagnosticBatch, FailureState},
};
use bevy_widgetry_internal_log::widgetry_debug;
use std::sync::Arc;

const PHYSICAL_PIXEL_SCALE: f32 = 1.0;

pub struct SplashPlugin;

#[derive(Resource, Default)]
struct Installed;

impl Plugin for SplashPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Installed>()
            .add_message::<WindowResized>()
            .add_message::<WindowScaleFactorChanged>()
            .configure_sets(
                PostUpdate,
                SplashSystems::Geometry.before(CameraUpdateSystems),
            )
            .add_systems(PostUpdate, sync.in_set(SplashSystems::Geometry));
        widgetry_debug!("SplashPlugin 注册完成");
    }
}

#[derive(SystemSet, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SplashSystems {
    Geometry,
}

#[derive(Component, Default)]
struct GeometryState {
    failure: FailureState<String>,
    confirmed: Option<Placement>,
}

#[derive(Component)]
#[require(WindowScaleExcluded, GeometryState)]
pub struct SplashWindow {
    main: Entity,
    physical_size: UVec2,
    positioned: bool,
}

impl SplashWindow {
    pub fn main_window(&self) -> Entity {
        self.main
    }

    pub fn physical_size(&self) -> UVec2 {
        self.physical_size
    }

    pub fn is_positioned(&self) -> bool {
        self.positioned
    }
}

pub struct Splash;

impl Splash {
    pub fn start(
        world: &mut World,
        main: Entity,
        mut window: Window,
    ) -> Result<Entity, SplashFailure> {
        if !world.contains_resource::<Installed>() {
            return Err(failure(SplashError::PluginMissing, "splash.start"));
        }
        let size = window.physical_size();
        if size.x == 0 || size.y == 0 {
            return Err(failure(SplashError::InvalidSize { size }, "splash.start"));
        }
        let main_window = world
            .get::<Window>(main)
            .ok_or_else(|| failure(SplashError::MainWindowMissing { main }, "splash.start"))?
            .clone();
        if world.get::<SplashWindow>(main).is_some() {
            return Err(failure(
                SplashError::InvalidMainWindow { main },
                "splash.start",
            ));
        }
        let mut monitors = world.query::<(Entity, &Monitor, Has<PrimaryMonitor>)>();
        let placement = geometry::placement(
            main,
            &main_window,
            size,
            monitors.iter(world),
            world.get_resource::<WinitMonitors>(),
            None,
        )
        .map_err(|kind| failure(kind, "splash.start"))?;
        let state = SplashWindow {
            main,
            physical_size: size,
            positioned: placement.is_some(),
        };
        window.visible = false;
        window.position = WindowPosition::Automatic;
        apply(&mut window, &state, placement);
        Ok(world
            .spawn((
                state,
                WindowScaleExcluded,
                GeometryState {
                    confirmed: placement,
                    ..default()
                },
                window,
            ))
            .id())
    }
}

fn constraints(size: UVec2, placement: Placement) -> WindowResizeConstraints {
    let size = size.as_vec2() / placement.base_scale;
    WindowResizeConstraints {
        min_width: size.x,
        min_height: size.y,
        max_width: size.x,
        max_height: size.y,
    }
}

fn needs_update(window: &Window, state: &SplashWindow, placement: Option<Placement>) -> bool {
    window.decorations
        || window.resizable
        || window.mode != WindowMode::Windowed
        || window.enabled_buttons.maximize
        || window.resolution.scale_factor_override() != Some(PHYSICAL_PIXEL_SCALE)
        || window.physical_size() != state.physical_size
        || placement.is_some_and(|placement| {
            window.position != WindowPosition::At(placement.position)
                || window.resize_constraints != constraints(state.physical_size, placement)
        })
}

fn apply(window: &mut Window, state: &SplashWindow, placement: Option<Placement>) {
    window.decorations = false;
    window.resizable = false;
    window.mode = WindowMode::Windowed;
    window.enabled_buttons.maximize = false;
    window
        .resolution
        .set_scale_factor_override(Some(PHYSICAL_PIXEL_SCALE));
    window
        .resolution
        .set_physical_resolution(state.physical_size.x, state.physical_size.y);
    if let Some(placement) = placement {
        window.position = WindowPosition::At(placement.position);
        window.resize_constraints = constraints(state.physical_size, placement);
    }
}

fn sync(
    _main_thread: NonSendMarker,
    mains: Query<&Window, Without<SplashWindow>>,
    monitors: Query<(Entity, &Monitor, Has<PrimaryMonitor>)>,
    native_monitors: Option<Res<WinitMonitors>>,
    mut splashes: Query<(Entity, &mut Window, &mut SplashWindow, &mut GeometryState)>,
    mut resized: MessageWriter<WindowResized>,
    mut rescaled: MessageWriter<WindowScaleFactorChanged>,
) -> Result {
    let mut failures = DiagnosticBatch::default();
    for (entity, mut window, mut state, mut geometry) in &mut splashes {
        let result = match mains.get(state.main) {
            Ok(main) => geometry::placement(
                state.main,
                main,
                state.physical_size,
                monitors.iter(),
                native_monitors.as_deref(),
                geometry.confirmed,
            ),
            Err(_) => Err(SplashError::MainWindowMissing { main: state.main }),
        };
        let placement =
            match result {
                Ok(placement) => placement,
                Err(kind) => {
                    state.positioned = false;
                    geometry.confirmed = None;
                    let previous = geometry
                        .failure
                        .active()
                        .map(|failure| failure.origin().id().clone());
                    let key = kind.to_string();
                    let error = geometry.failure.failure(key, || {
                        Arc::new(SplashFailure::new(
                            kind,
                            ErrorFrame::new("splash.geometry")
                                .field("splash", entity)
                                .field("main", state.main),
                        ))
                    });
                    if error.diagnostics().next().is_some_and(|diagnostic| {
                        previous.as_ref() != Some(diagnostic.origin().id())
                    }) {
                        for diagnostic in error.shared_diagnostics() {
                            failures.push_shared(diagnostic);
                        }
                    }
                    continue;
                }
            };
        if placement.is_some() {
            geometry.confirmed = placement;
            geometry
                .failure
                .recover(|| ErrorFrame::new("splash.geometry.recovered").field("window", entity));
        }
        if state.positioned != placement.is_some() {
            state.positioned = placement.is_some();
        }
        if needs_update(&window, &state, placement) {
            let previous_size = window.physical_size();
            let previous_scale = window.scale_factor();
            apply(&mut window, &state, placement);
            if previous_size != window.physical_size() {
                resized.write(WindowResized {
                    window: entity,
                    width: window.width(),
                    height: window.height(),
                });
            }
            if previous_scale != window.scale_factor() {
                rescaled.write(WindowScaleFactorChanged {
                    window: entity,
                    scale_factor: f64::from(window.scale_factor()),
                });
            }
        }
    }
    match failures.into_bevy_error() {
        Some(error) => Err(error),
        None => Ok(()),
    }
}
