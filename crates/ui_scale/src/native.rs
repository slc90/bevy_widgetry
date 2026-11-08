use bevy::{
    math::I64Vec2,
    prelude::*,
    window::{Monitor, MonitorSelection, WindowMode, WindowPosition},
    winit::{WINIT_WINDOWS, WinitMonitors},
};

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Desktop {
    pub work: IRect,
    pub base_scale: f32,
    pub chrome: UVec2,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(crate) struct Environment {
    pub desktop: Option<Desktop>,
    pub suspended: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct WorkAreaFault {
    pub point: IVec2,
    pub source: winsafe::co::ERROR,
}

pub(crate) fn environment<'a>(
    entity: Entity,
    window: &Window,
    monitors: impl Iterator<Item = (Entity, &'a Monitor, bool)>,
    native_monitors: Option<&WinitMonitors>,
    prefer_requested_position: bool,
) -> Result<Environment, WorkAreaFault> {
    let (current, suspended, chrome) = WINIT_WINDOWS.with_borrow(|native| {
        let Some(native) = native.get_window(entity) else {
            return (None, false, UVec2::ZERO);
        };
        let current = native.current_monitor().map(|monitor| {
            let position = monitor.position();
            let size = monitor.size();
            (
                IVec2::new(
                    position.x + (size.width / 2) as i32,
                    position.y + (size.height / 2) as i32,
                ),
                monitor.scale_factor() as f32,
            )
        });
        let outer = native.outer_size();
        let inner = native.inner_size();
        (
            current,
            native.is_maximized() || native.is_minimized() == Some(true),
            UVec2::new(
                outer.width.saturating_sub(inner.width),
                outer.height.saturating_sub(inner.height),
            ),
        )
    });
    let requested = || {
        if let WindowPosition::Centered(MonitorSelection::Index(index)) = window.position {
            return native_monitors
                .and_then(|monitors| monitors.nth(index))
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
                });
        }
        selected_monitor(window, monitors).map(|monitor| {
            (
                monitor.physical_position
                    + IVec2::new(
                        (monitor.physical_width / 2) as i32,
                        (monitor.physical_height / 2) as i32,
                    ),
                monitor.scale_factor as f32,
            )
        })
    };
    // Honor a new host position request before winit moves the native window in Last.
    let geometry = if prefer_requested_position
        && !matches!(
            window.position,
            WindowPosition::Automatic | WindowPosition::Centered(MonitorSelection::Current)
        ) {
        requested().or(current)
    } else {
        current.or_else(requested)
    };
    let desktop = geometry
        .map(|(point, base_scale)| {
            let info = winsafe::HMONITOR::MonitorFromPoint(
                winsafe::POINT {
                    x: point.x,
                    y: point.y,
                },
                winsafe::co::MONITOR::DEFAULTTONEAREST,
            )
            .GetMonitorInfo()
            .map_err(|source| WorkAreaFault { point, source })?;
            Ok(Desktop {
                work: IRect::from_corners(
                    IVec2::new(info.rcWork.left, info.rcWork.top),
                    IVec2::new(info.rcWork.right, info.rcWork.bottom),
                ),
                base_scale,
                chrome,
            })
        })
        .transpose()?;
    Ok(Environment {
        desktop,
        suspended: suspended || window.mode != WindowMode::Windowed,
    })
}

fn selected_monitor<'a>(
    window: &Window,
    monitors: impl Iterator<Item = (Entity, &'a Monitor, bool)>,
) -> Option<&'a Monitor> {
    match window.position {
        WindowPosition::Centered(MonitorSelection::Entity(entity)) => monitors
            .into_iter()
            .find(|(candidate, _, _)| *candidate == entity)
            .map(|(_, monitor, _)| monitor),
        WindowPosition::Centered(MonitorSelection::Index(_)) => None,
        WindowPosition::At(position) => monitors
            .into_iter()
            .max_by_key(|(_, monitor, _)| monitor_score(position, window.physical_size(), monitor))
            .map(|(_, monitor, _)| monitor),
        _ => monitors
            .into_iter()
            .find(|(_, _, primary)| *primary)
            .map(|(_, monitor, _)| monitor),
    }
}

fn monitor_score(position: IVec2, size: UVec2, monitor: &Monitor) -> (i128, i128) {
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
    (
        i128::from(overlap.x) * i128::from(overlap.y),
        -(i128::from(distance.x).pow(2) + i128::from(distance.y).pow(2)),
    )
}

#[cfg(test)]
#[allow(clippy::disallowed_macros, clippy::unwrap_used)]
mod tests {
    use super::*;
    use bevy::window::PrimaryMonitor;

    const DESIGN: UVec2 = UVec2::new(640, 400);
    const MONITOR_SIZE: UVec2 = UVec2::new(1920, 1080);

    fn monitor(x: i32) -> Monitor {
        Monitor {
            name: None,
            physical_width: MONITOR_SIZE.x,
            physical_height: MONITOR_SIZE.y,
            physical_position: IVec2::new(x, 0),
            refresh_rate_millihertz: None,
            scale_factor: 1.0,
            video_modes: vec![],
        }
    }

    #[test]
    fn before_creation_selection_keeps_negative_coordinates_and_overlap_priority() {
        let mut world = World::new();
        world.spawn((monitor(0), PrimaryMonitor));
        let left = world.spawn(monitor(-1920)).id();
        let mut query = world.query::<(Entity, &Monitor, Has<PrimaryMonitor>)>();
        let mut window = Window {
            resolution: DESIGN.into(),
            position: WindowPosition::At(IVec2::new(-500, 100)),
            ..default()
        };
        assert_eq!(
            selected_monitor(&window, query.iter(&world))
                .unwrap()
                .physical_position
                .x,
            -1920
        );
        window.position = WindowPosition::At(IVec2::new(-200, 100));
        assert_eq!(
            selected_monitor(&window, query.iter(&world))
                .unwrap()
                .physical_position
                .x,
            0
        );
        window.position = WindowPosition::Centered(MonitorSelection::Entity(left));
        assert_eq!(
            selected_monitor(&window, query.iter(&world))
                .unwrap()
                .physical_position
                .x,
            -1920
        );
        window.position = WindowPosition::Centered(MonitorSelection::Index(0));
        assert!(selected_monitor(&window, query.iter(&world)).is_none());
    }

    #[test]
    fn far_positions_use_distance_without_overflow() {
        let left = monitor(-1920);
        let right = monitor(0);
        assert!(
            monitor_score(IVec2::splat(i32::MAX), DESIGN, &right)
                > monitor_score(IVec2::splat(i32::MAX), DESIGN, &left)
        );
    }

    #[test]
    fn missing_monitors_are_a_normal_wait_and_fullscreen_suspends_geometry() {
        let window = Window {
            mode: WindowMode::BorderlessFullscreen(MonitorSelection::Primary),
            ..default()
        };
        let environment = environment(
            Entity::PLACEHOLDER,
            &window,
            std::iter::empty(),
            None,
            false,
        )
        .unwrap();
        assert!(environment.desktop.is_none());
        assert!(environment.suspended);
    }
}
