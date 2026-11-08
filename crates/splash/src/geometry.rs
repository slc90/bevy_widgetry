use crate::SplashError;
use bevy::{
    math::I64Vec2,
    prelude::*,
    window::{Monitor, MonitorSelection, WindowPosition},
    winit::{WINIT_WINDOWS, WinitMonitors},
};

#[derive(Clone, Copy)]
pub(crate) struct Placement {
    pub position: IVec2,
    pub base_scale: f32,
    display: Display,
}

#[derive(Clone, Copy)]
struct Display {
    entity: Option<Entity>,
    bounds: IRect,
    scale: f32,
}

fn display(entity: Entity, monitor: &Monitor) -> Display {
    Display {
        entity: Some(entity),
        bounds: IRect::from_corners(
            monitor.physical_position,
            monitor.physical_position
                + UVec2::new(monitor.physical_width, monitor.physical_height).as_ivec2(),
        ),
        scale: monitor.scale_factor as f32,
    }
}

pub(crate) fn placement<'a>(
    main: Entity,
    window: &Window,
    size: UVec2,
    mut monitors: impl Iterator<Item = (Entity, &'a Monitor, bool)>,
    native_monitors: Option<&WinitMonitors>,
    confirmed: Option<Placement>,
) -> Result<Option<Placement>, SplashError> {
    let (minimized, center) = WINIT_WINDOWS.with_borrow(|windows| {
        let Some(window) = windows.get_window(main) else {
            return Ok((false, None));
        };
        if window.is_minimized() == Some(true) {
            return Ok((true, None));
        }
        let position = window
            .outer_position()
            .map_err(|source| SplashError::NativePosition { main, source })?;
        let size = window.outer_size();
        Ok((
            false,
            Some(I64Vec2::new(
                i64::from(position.x) + i64::from(size.width) / 2,
                i64::from(position.y) + i64::from(size.height) / 2,
            )),
        ))
    })?;
    if minimized {
        return retained(main, size, confirmed, monitors);
    }
    let center = center.or_else(|| match window.position {
        WindowPosition::At(position) => {
            Some(position.as_i64vec2() + window.physical_size().as_i64vec2() / 2)
        }
        _ => None,
    });
    let selected = if let Some(center) = center {
        monitors
            .map(|(entity, monitor, _)| display(entity, monitor))
            .min_by_key(|display| {
                let min = display.bounds.min.as_i64vec2();
                let max = display.bounds.max.as_i64vec2();
                let inside = center.cmpge(min).all() && center.cmplt(max).all();
                let distance = center - center.clamp(min, max);
                (
                    !inside,
                    i128::from(distance.x).pow(2) + i128::from(distance.y).pow(2),
                )
            })
    } else {
        match window.position {
            WindowPosition::Centered(MonitorSelection::Entity(entity)) => {
                monitors.find_map(|(candidate, monitor, _)| {
                    (candidate == entity).then(|| display(candidate, monitor))
                })
            }
            WindowPosition::Centered(MonitorSelection::Primary) => monitors
                .find_map(|(entity, monitor, primary)| primary.then(|| display(entity, monitor))),
            WindowPosition::Centered(MonitorSelection::Index(index)) => native_monitors
                .and_then(|monitors| monitors.nth(index))
                .map(|monitor| {
                    let position = monitor.position();
                    let size = monitor.size();
                    let min = IVec2::new(position.x, position.y);
                    Display {
                        entity: None,
                        bounds: IRect::from_corners(
                            min,
                            min + UVec2::new(size.width, size.height).as_ivec2(),
                        ),
                        scale: monitor.scale_factor() as f32,
                    }
                }),
            _ => None,
        }
    };
    selected
        .map(|display| centered(main, size, display))
        .transpose()
}

fn centered(main: Entity, size: UVec2, display: Display) -> Result<Placement, SplashError> {
    if size.cmpgt(display.bounds.size().as_uvec2()).any() {
        return Err(SplashError::TooLarge {
            main,
            size,
            monitor: display.bounds,
        });
    }
    Ok(Placement {
        position: display.bounds.min + (display.bounds.size() - size.as_ivec2()) / 2,
        base_scale: display.scale,
        display,
    })
}

fn retained<'a>(
    main: Entity,
    size: UVec2,
    confirmed: Option<Placement>,
    mut monitors: impl Iterator<Item = (Entity, &'a Monitor, bool)>,
) -> Result<Option<Placement>, SplashError> {
    let Some(confirmed) = confirmed else {
        return Ok(None);
    };
    monitors
        .find_map(|(entity, monitor, _)| {
            let display = display(entity, monitor);
            let matches = confirmed
                .display
                .entity
                .map_or(display.bounds == confirmed.display.bounds, |previous| {
                    previous == entity
                });
            matches.then_some(display)
        })
        .map(|display| centered(main, size, display))
        .transpose()
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::disallowed_macros)]
mod tests {
    use super::*;

    #[test]
    fn minimized_main_retains_confirmed_display_and_waits_without_one() {
        let mut world = World::new();
        let left = world
            .spawn(Monitor {
                name: None,
                physical_width: 1920,
                physical_height: 1080,
                physical_position: IVec2::new(-1920, 0),
                refresh_rate_millihertz: None,
                scale_factor: 1.5,
                video_modes: vec![],
            })
            .id();
        let size = UVec2::new(480, 270);
        let previous = centered(
            Entity::PLACEHOLDER,
            size,
            display(left, world.get::<Monitor>(left).unwrap()),
        )
        .unwrap();
        let mut query = world.query::<(Entity, &Monitor)>();
        let current = retained(
            Entity::PLACEHOLDER,
            size,
            Some(previous),
            query
                .iter(&world)
                .map(|(entity, monitor)| (entity, monitor, false)),
        )
        .unwrap()
        .unwrap();
        assert_eq!(current.position, IVec2::new(-1200, 405));
        assert!(
            retained(
                Entity::PLACEHOLDER,
                size,
                None,
                query
                    .iter(&world)
                    .map(|(entity, monitor)| (entity, monitor, false))
            )
            .unwrap()
            .is_none()
        );
        world.get_mut::<Monitor>(left).unwrap().physical_position = IVec2::new(-3840, 0);
        let moved = retained(
            Entity::PLACEHOLDER,
            size,
            Some(previous),
            query
                .iter(&world)
                .map(|(entity, monitor)| (entity, monitor, false)),
        )
        .unwrap()
        .unwrap();
        assert_eq!(moved.position, IVec2::new(-3120, 405));
        world.despawn(left);
        assert!(
            retained(
                Entity::PLACEHOLDER,
                size,
                Some(previous),
                query
                    .iter(&world)
                    .map(|(entity, monitor)| (entity, monitor, false))
            )
            .unwrap()
            .is_none()
        );
    }
}
