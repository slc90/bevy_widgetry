use crate::config::AppConfig;
use bevy::ecs::system::NonSendMarker;
use bevy::math::I64Vec2;
use bevy::prelude::*;
use bevy::window::{
    Monitor, MonitorSelection, PrimaryMonitor, PrimaryWindow, WindowMode, WindowPosition,
    WindowResizeConstraints,
};
use bevy::winit::WINIT_WINDOWS;

#[derive(Component)]
pub(crate) struct PopupSize {
    logical: Vec2,
    expected: UVec2,
    scale: f32,
    base_scale: f32,
    constraints: WindowResizeConstraints,
    desktop: Option<Desktop>,
    suspended: bool,
}

#[derive(Clone, Copy, PartialEq)]
struct Desktop {
    work: IRect,
    base_scale: f32,
}

pub(crate) fn install(app: &mut App) {
    app.add_observer(initialize_window)
        .add_systems(Update, sync_popup_sizes);
}

fn initialize_window(
    event: On<Add<Window>>,
    _main_thread: NonSendMarker,
    config: Res<AppConfig>,
    mut windows: Query<(&mut Window, Has<PrimaryWindow>)>,
    monitors: Query<(Entity, &Monitor, Has<PrimaryMonitor>)>,
    mut commands: Commands,
) -> Result {
    let (mut window, primary) = windows
        .get_mut(event.entity)
        .map_err(crate::diagnostics::error)?;
    if primary {
        window
            .resolution
            .set_scale_factor_override(Some(config.scale_factor));
        return Ok(());
    }
    let mut size = PopupSize {
        logical: window.resolution.size(),
        expected: window.physical_size(),
        scale: window.scale_factor(),
        base_scale: window.resolution.base_scale_factor(),
        constraints: window.resize_constraints,
        desktop: None,
        suspended: false,
    };
    let desktop = desktop_for(event.entity, &window, &monitors)?;
    resize_popup(&mut window, &mut size, config.scale_factor, desktop, false);
    commands.entity(event.entity).insert(size);
    Ok(())
}

pub(crate) fn apply(
    entity: Entity,
    window: &mut Window,
    size: Option<&mut PopupSize>,
    scale: f32,
    monitors: &Query<(Entity, &Monitor, Has<PrimaryMonitor>)>,
) -> Result {
    if let Some(size) = size {
        let desktop = desktop_for(entity, window, monitors)?;
        resize_popup(window, size, scale, desktop, suspended(entity, window));
    } else {
        window.resolution.set_scale_factor_override(Some(scale));
    }
    Ok(())
}

fn sync_popup_sizes(
    _main_thread: NonSendMarker,
    config: Res<AppConfig>,
    mut windows: Query<(Entity, &mut Window, &mut PopupSize), Changed<Window>>,
    monitors: Query<(Entity, &Monitor, Has<PrimaryMonitor>)>,
) -> Result {
    for (entity, mut window, mut size) in &mut windows {
        let desktop = desktop_for(entity, &window, &monitors)?;
        let suspended = suspended(entity, &window);
        if size.scale != config.scale_factor
            || size.base_scale != window.resolution.base_scale_factor()
            || (!suspended && size.expected != window.physical_size())
            || size.desktop != desktop
            || size.suspended != suspended
        {
            resize_popup(
                &mut window,
                &mut size,
                config.scale_factor,
                desktop,
                suspended,
            );
        }
    }
    Ok(())
}

fn suspended(entity: Entity, window: &Window) -> bool {
    window.mode != WindowMode::Windowed
        || WINIT_WINDOWS.with_borrow(|native| {
            native
                .get_window(entity)
                .is_some_and(|window| window.is_maximized() || window.is_minimized() == Some(true))
        })
}

fn desktop_for(
    entity: Entity,
    window: &Window,
    monitors: &Query<(Entity, &Monitor, Has<PrimaryMonitor>)>,
) -> Result<Option<Desktop>> {
    let current = WINIT_WINDOWS.with_borrow(|native| {
        native
            .get_window(entity)
            .and_then(|window| window.current_monitor())
            .map(|monitor| {
                let position = monitor.position();
                let size = monitor.size();
                (
                    IVec2::new(
                        position.x + (size.width / 2) as i32,
                        position.y + (size.height / 2) as i32,
                    ),
                    monitor.scale_factor() as f32,
                )
            })
    });
    let geometry = current.or_else(|| {
        let selected = match window.position {
            WindowPosition::Centered(MonitorSelection::Entity(entity)) => monitors.get(entity).ok(),
            WindowPosition::Centered(MonitorSelection::Index(index)) => monitors.iter().nth(index),
            WindowPosition::At(position) => monitors.iter().max_by_key(|(_, monitor, _)| {
                monitor_score(position, window.physical_size(), monitor)
            }),
            _ => monitors.iter().find(|(_, _, primary)| *primary),
        };
        selected.map(|(_, monitor, _)| {
            (
                monitor.physical_position
                    + IVec2::new(
                        (monitor.physical_width / 2) as i32,
                        (monitor.physical_height / 2) as i32,
                    ),
                monitor.scale_factor as f32,
            )
        })
    });
    let Some((point, base_scale)) = geometry else {
        return Ok(None);
    };
    let info = winsafe::HMONITOR::MonitorFromPoint(
        winsafe::POINT {
            x: point.x,
            y: point.y,
        },
        winsafe::co::MONITOR::DEFAULTTONEAREST,
    )
    .GetMonitorInfo()
    .map_err(|error| {
        crate::diagnostics::at(
            error,
            bevy_widgetry::diagnostics::ErrorFrame::new("gallery.window_scale.work_area"),
        )
    })?;
    Ok(Some(Desktop {
        work: IRect::from_corners(
            IVec2::new(info.rcWork.left, info.rcWork.top),
            IVec2::new(info.rcWork.right, info.rcWork.bottom),
        ),
        base_scale,
    }))
}

fn monitor_score(position: IVec2, size: UVec2, monitor: &Monitor) -> (i64, i64) {
    let min = monitor.physical_position.as_i64vec2();
    let max = min
        + I64Vec2::new(
            i64::from(monitor.physical_width),
            i64::from(monitor.physical_height),
        );
    let window_min = position.as_i64vec2();
    let window_max = window_min + size.as_i64vec2();
    let overlap = (window_max.min(max) - window_min.max(min)).max(I64Vec2::ZERO);
    let center = (window_min + window_max) / 2;
    let distance = center - center.clamp(min, max);
    (overlap.x * overlap.y, -distance.length_squared())
}

fn resize_popup(
    window: &mut Window,
    size: &mut PopupSize,
    scale: f32,
    desktop: Option<Desktop>,
    suspended: bool,
) {
    // WM_DPICHANGED can resize the native window without a user Resize gesture.
    // Monitor metadata and the native resize may arrive in different updates.
    let same_base_scale = size.base_scale == window.resolution.base_scale_factor()
        && size.desktop.map(|desktop| desktop.base_scale)
            == desktop.map(|desktop| desktop.base_scale)
        && desktop
            .is_none_or(|desktop| window.resolution.base_scale_factor() == desktop.base_scale);
    if !suspended && !size.suspended && same_base_scale && window.physical_size() != size.expected {
        size.logical = window.physical_size().as_vec2() / size.scale;
    }
    window.resolution.set_scale_factor_override(Some(scale));
    if !suspended {
        let constraints = size.constraints.check_constraints();
        let min = Vec2::new(constraints.min_width, constraints.min_height);
        let max = Vec2::new(constraints.max_width, constraints.max_height);
        let physical = (size.logical.clamp(min, max) * scale)
            .round()
            .max(Vec2::ONE);
        let physical = desktop.map_or(physical, |desktop| {
            physical.min(desktop.work.size().as_vec2())
        });
        let requested = physical.as_uvec2();
        window
            .resolution
            .set_physical_resolution(requested.x, requested.y);
        size.expected = requested;
        if let Some(desktop) = desktop {
            // winit constraints use OS logical pixels, which may differ from the Gallery UI scale.
            let ratio = scale / desktop.base_scale;
            let work_size = desktop.work.size().as_vec2();
            window.resize_constraints = WindowResizeConstraints {
                min_width: (constraints.min_width * ratio).min(work_size.x / desktop.base_scale),
                min_height: (constraints.min_height * ratio).min(work_size.y / desktop.base_scale),
                max_width: (constraints.max_width * ratio).min(work_size.x / desktop.base_scale),
                max_height: (constraints.max_height * ratio).min(work_size.y / desktop.base_scale),
            };
            let max_position = desktop.work.max - requested.as_ivec2();
            let position = match window.position {
                WindowPosition::At(position) => position.clamp(desktop.work.min, max_position),
                _ => desktop.work.min + (desktop.work.size() - requested.as_ivec2()) / 2,
            };
            window.position = WindowPosition::At(position);
        }
    }
    size.scale = scale;
    size.base_scale = window.resolution.base_scale_factor();
    size.desktop = desktop;
    size.suspended = suspended;
}

#[cfg(test)]
#[allow(clippy::disallowed_macros, clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn system_dpi_resizes_do_not_replace_the_popup_logical_size() {
        for metadata_arrives_first in [false, true] {
            let (mut window, mut size) = fixture();
            let old = Desktop {
                work: IRect::new(0, 0, 2560, 1600),
                base_scale: 1.5,
            };
            let new = Desktop {
                work: IRect::new(0, 0, 3840, 2160),
                base_scale: 2.0,
            };
            window.resolution.set_scale_factor(1.5);
            resize_popup(&mut window, &mut size, 1.0, Some(old), false);
            if metadata_arrives_first {
                resize_popup(&mut window, &mut size, 1.0, Some(new), false);
            }
            window.resolution.set_physical_resolution(853, 533);
            window.resolution.set_scale_factor(2.0);
            resize_popup(
                &mut window,
                &mut size,
                1.0,
                Some(if metadata_arrives_first { new } else { old }),
                false,
            );
            window.resolution.set_scale_factor(2.0);
            resize_popup(&mut window, &mut size, 1.0, Some(new), false);
            assert_eq!(window.physical_size(), UVec2::new(640, 400));
            window.resolution.set_physical_resolution(800, 500);
            resize_popup(&mut window, &mut size, 1.0, Some(new), false);
            assert_eq!(window.physical_size(), UVec2::new(800, 500));
            resize_popup(&mut window, &mut size, 2.0, Some(new), false);
            assert_eq!(window.physical_size(), UVec2::new(1600, 1000));
        }
    }

    #[test]
    fn pre_creation_monitor_selection_uses_the_largest_overlap_then_nearest_monitor() {
        let monitor = |x| Monitor {
            name: None,
            physical_width: 1920,
            physical_height: 1080,
            physical_position: IVec2::new(x, 0),
            refresh_rate_millihertz: None,
            scale_factor: 1.0,
            video_modes: vec![],
        };
        let left = monitor(0);
        let right = monitor(1920);
        let size = UVec2::new(640, 400);
        assert!(
            monitor_score(IVec2::new(1800, 100), size, &right)
                > monitor_score(IVec2::new(1800, 100), size, &left)
        );
        assert!(
            monitor_score(IVec2::new(-1000, 100), size, &left)
                > monitor_score(IVec2::new(-1000, 100), size, &right)
        );
        let negative = monitor(-1920);
        assert!(
            monitor_score(IVec2::new(-1000, 100), size, &negative)
                > monitor_score(IVec2::new(-1000, 100), size, &left)
        );
    }

    fn fixture() -> (Window, PopupSize) {
        let window = Window {
            resolution: (640, 400).into(),
            ..default()
        };
        let size = PopupSize {
            logical: Vec2::new(640.0, 400.0),
            expected: window.physical_size(),
            scale: 1.0,
            base_scale: window.resolution.base_scale_factor(),
            constraints: window.resize_constraints,
            desktop: None,
            suspended: false,
        };
        (window, size)
    }

    #[test]
    fn scaling_preserves_content_space_and_user_resizes() {
        let (mut window, mut size) = fixture();
        for scale in [1.25, 2.0, 1.0] {
            resize_popup(&mut window, &mut size, scale, None, false);
            assert_eq!(window.resolution.size(), Vec2::new(640.0, 400.0));
        }
        window.resolution.set_physical_resolution(800, 500);
        resize_popup(&mut window, &mut size, 1.5, None, false);
        assert_eq!(window.physical_size(), UVec2::new(1200, 750));
    }

    #[test]
    fn constrained_popup_fits_the_work_area_without_losing_its_design_size() {
        let (mut window, mut size) = fixture();
        let desktop = Some(Desktop {
            work: IRect::new(100, 50, 1000, 650),
            base_scale: 1.5,
        });
        resize_popup(&mut window, &mut size, 2.0, desktop, false);
        assert_eq!(window.physical_size(), UVec2::new(900, 600));
        assert_eq!(window.position, WindowPosition::At(IVec2::new(100, 50)));
        assert_eq!(window.resize_constraints.max_width, 600.0);
        resize_popup(&mut window, &mut size, 1.0, desktop, false);
        assert_eq!(window.physical_size(), UVec2::new(640, 400));
    }

    #[test]
    fn scaling_leaves_maximized_geometry_to_the_os_and_restores_normal_size() {
        let (mut window, mut size) = fixture();
        window.resolution.set_physical_resolution(1920, 1080);
        resize_popup(&mut window, &mut size, 2.0, None, true);
        assert_eq!(window.physical_size(), UVec2::new(1920, 1080));
        window.resolution.set_physical_resolution(640, 400);
        resize_popup(&mut window, &mut size, 2.0, None, false);
        assert_eq!(window.physical_size(), UVec2::new(1280, 800));
    }
}
