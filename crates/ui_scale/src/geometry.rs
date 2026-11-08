use crate::native::{Desktop, Environment};
use bevy::{
    math::DVec2,
    prelude::*,
    window::{WindowPosition, WindowResizeConstraints, WindowResized, WindowScaleFactorChanged},
};

pub(crate) struct WindowSize {
    logical: DVec2,
    expected: UVec2,
    scale: f32,
    base_scale: f32,
    constraints: WindowResizeConstraints,
    applied_constraints: WindowResizeConstraints,
    desktop: Option<Desktop>,
    suspended: bool,
    position: WindowPosition,
}

pub(crate) struct WindowChanges {
    pub resized: Option<WindowResized>,
    pub rescaled: Option<WindowScaleFactorChanged>,
}

impl WindowSize {
    pub fn new(window: &Window) -> Self {
        Self {
            logical: window.physical_size().as_dvec2() / f64::from(window.scale_factor()),
            expected: window.physical_size(),
            scale: window.scale_factor(),
            base_scale: window.resolution.base_scale_factor(),
            constraints: window.resize_constraints,
            applied_constraints: window.resize_constraints,
            desktop: None,
            suspended: false,
            position: window.position,
        }
    }

    pub fn resume(&mut self, window: &Window) {
        self.logical = window.physical_size().as_dvec2() / f64::from(window.scale_factor());
        self.expected = window.physical_size();
        self.scale = window.scale_factor();
        self.base_scale = window.resolution.base_scale_factor();
        self.desktop = None;
        self.suspended = false;
        self.position = window.position;
    }

    pub fn position_changed(&self, window: &Window) -> bool {
        self.position != window.position
    }

    pub fn needs_update(&self, window: &Window, scale: f32, environment: Environment) -> bool {
        self.scale != scale
            || self.applied_constraints != window.resize_constraints
            || self.base_scale != window.resolution.base_scale_factor()
            || self.desktop != environment.desktop
            || self.suspended != environment.suspended
            || (!environment.suspended && self.position_changed(window))
            || (!environment.suspended && self.expected != window.physical_size())
            || window.resolution.scale_factor_override() != Some(scale)
    }

    pub fn apply(
        &mut self,
        entity: Entity,
        window: &mut Window,
        scale: f32,
        environment: Environment,
    ) -> WindowChanges {
        let previous_size = window.physical_size();
        let previous_scale = window.scale_factor();
        let requested = window.resize_constraints;
        if requested.min_width != self.applied_constraints.min_width {
            self.constraints.min_width = requested.min_width;
        }
        if requested.min_height != self.applied_constraints.min_height {
            self.constraints.min_height = requested.min_height;
        }
        if requested.max_width != self.applied_constraints.max_width {
            self.constraints.max_width = requested.max_width;
        }
        if requested.max_height != self.applied_constraints.max_height {
            self.constraints.max_height = requested.max_height;
        }
        // Metadata and WM_DPICHANGED resize notifications can arrive in either order.
        let same_dpi = self.base_scale == window.resolution.base_scale_factor()
            && self.desktop.map(|desktop| desktop.base_scale)
                == environment.desktop.map(|desktop| desktop.base_scale)
            && environment
                .desktop
                .is_none_or(|desktop| desktop.base_scale == window.resolution.base_scale_factor());
        if !self.suspended
            && !environment.suspended
            && same_dpi
            && self.desktop == environment.desktop
            && window.physical_size() != self.expected
        {
            self.logical = window.physical_size().as_dvec2() / f64::from(self.scale);
        }
        window.resolution.set_scale_factor_override(Some(scale));
        if !environment.suspended {
            let constraints = self.constraints.check_constraints();
            let min = DVec2::new(
                f64::from(constraints.min_width),
                f64::from(constraints.min_height),
            );
            let max = DVec2::new(
                f64::from(constraints.max_width),
                f64::from(constraints.max_height),
            );
            let physical = (self.logical.clamp(min, max) * f64::from(scale))
                .round()
                .max(DVec2::ONE);
            let available = environment.desktop.map(|desktop| {
                desktop
                    .work
                    .size()
                    .as_uvec2()
                    .saturating_sub(desktop.chrome)
                    .max(UVec2::ONE)
            });
            let physical = available
                .map_or(physical, |available| physical.min(available.as_dvec2()))
                .as_uvec2();
            window
                .resolution
                .set_physical_resolution(physical.x, physical.y);
            self.expected = physical;
            if let (Some(desktop), Some(available)) = (environment.desktop, available) {
                // winit constraints use OS logical pixels, not the application's logical UI units.
                let ratio = f64::from(scale) / f64::from(desktop.base_scale);
                let limits = available.as_dvec2() / f64::from(desktop.base_scale);
                let convert = |value: f32, limit: f64| (f64::from(value) * ratio).min(limit) as f32;
                window.resize_constraints = WindowResizeConstraints {
                    min_width: convert(constraints.min_width, limits.x),
                    min_height: convert(constraints.min_height, limits.y),
                    max_width: convert(constraints.max_width, limits.x),
                    max_height: convert(constraints.max_height, limits.y),
                };
                let outer = physical.saturating_add(desktop.chrome).as_ivec2();
                let max_position = (desktop.work.max - outer).max(desktop.work.min);
                window.position = WindowPosition::At(match window.position {
                    WindowPosition::At(position) => position.clamp(desktop.work.min, max_position),
                    _ => desktop.work.min + (max_position - desktop.work.min) / 2,
                });
            }
        }
        self.scale = scale;
        self.base_scale = window.resolution.base_scale_factor();
        self.desktop = environment.desktop;
        self.suspended = environment.suspended;
        self.applied_constraints = window.resize_constraints;
        self.position = window.position;
        // winit reports requested changes in Last, after camera extraction data was prepared.
        // Publish them now so explicit viewports fit the new output texture in this frame.
        WindowChanges {
            resized: (previous_size != window.physical_size()).then(|| WindowResized {
                window: entity,
                width: window.width(),
                height: window.height(),
            }),
            rescaled: (previous_scale != scale).then_some(WindowScaleFactorChanged {
                window: entity,
                scale_factor: f64::from(scale),
            }),
        }
    }
}

#[cfg(test)]
#[allow(clippy::disallowed_macros, clippy::unwrap_used)]
mod tests {
    use super::*;

    const DESIGN: UVec2 = UVec2::new(640, 400);

    fn fixture() -> (Window, WindowSize) {
        let window = Window {
            resolution: DESIGN.into(),
            ..default()
        };
        let size = WindowSize::new(&window);
        (window, size)
    }

    #[test]
    fn system_dpi_resize_preserves_the_design_size() {
        let (mut window, mut size) = fixture();
        window.resolution.set_scale_factor(1.5);
        size.apply(
            Entity::PLACEHOLDER,
            &mut window,
            1.0,
            Environment::default(),
        );
        window.resolution.set_scale_factor(2.0);
        window.resolution.set_physical_resolution(853, 533);
        size.apply(
            Entity::PLACEHOLDER,
            &mut window,
            1.0,
            Environment::default(),
        );
        assert_eq!(window.physical_size(), DESIGN);
    }

    #[test]
    fn temporary_work_area_limit_recovers_the_design_and_preserves_explicit_position() {
        let (mut window, mut size) = fixture();
        window.position = bevy::window::WindowPosition::At(IVec2::new(-850, 100));
        let environment = Environment {
            desktop: Some(Desktop {
                work: IRect::new(-1000, 50, -100, 650),
                base_scale: 1.5,
                chrome: UVec2::ZERO,
            }),
            suspended: false,
        };
        window.resolution.set_scale_factor(1.5);
        size.apply(Entity::PLACEHOLDER, &mut window, 2.0, environment);
        assert_eq!(window.physical_size(), UVec2::new(900, 600));
        assert_eq!(
            window.position,
            bevy::window::WindowPosition::At(IVec2::new(-1000, 50))
        );
        size.apply(Entity::PLACEHOLDER, &mut window, 1.0, environment);
        assert_eq!(window.physical_size(), DESIGN);
    }

    #[test]
    fn suspended_geometry_does_not_replace_the_restored_design() {
        let (mut window, mut size) = fixture();
        let native_maximized_size = UVec2::new(1920, 1080);
        window
            .resolution
            .set_physical_resolution(native_maximized_size.x, native_maximized_size.y);
        size.apply(
            Entity::PLACEHOLDER,
            &mut window,
            2.0,
            Environment {
                suspended: true,
                ..default()
            },
        );
        assert_eq!(window.physical_size(), native_maximized_size);
        window
            .resolution
            .set_physical_resolution(DESIGN.x, DESIGN.y);
        size.apply(
            Entity::PLACEHOLDER,
            &mut window,
            2.0,
            Environment::default(),
        );
        assert_eq!(window.physical_size(), DESIGN * 2);
    }

    #[test]
    fn changing_one_constraint_keeps_other_constraints_in_application_units() {
        let (mut window, mut size) = fixture();
        window.resolution.set_scale_factor(2.0);
        let environment = Environment {
            desktop: Some(Desktop {
                work: IRect::new(0, 0, 3840, 2160),
                base_scale: 2.0,
                chrome: UVec2::ZERO,
            }),
            suspended: false,
        };
        let initial_min = window.resize_constraints.min_width;
        size.apply(Entity::PLACEHOLDER, &mut window, 1.0, environment);
        assert_eq!(window.resize_constraints.min_width, initial_min / 2.0);
        window.resize_constraints.max_width = 500.0;
        size.apply(Entity::PLACEHOLDER, &mut window, 2.0, environment);
        assert_eq!(window.resize_constraints.min_width, initial_min);
        assert_eq!(window.physical_width(), 1000);
    }

    #[test]
    fn both_dpi_event_orders_preserve_design_and_accept_later_user_resize() {
        for metadata_first in [false, true] {
            let (mut window, mut size) = fixture();
            let old = Environment {
                desktop: Some(Desktop {
                    work: IRect::new(0, 0, 2560, 1600),
                    base_scale: 1.5,
                    chrome: UVec2::ZERO,
                }),
                suspended: false,
            };
            let new = Environment {
                desktop: Some(Desktop {
                    work: IRect::new(0, 0, 3840, 2160),
                    base_scale: 2.0,
                    chrome: UVec2::ZERO,
                }),
                suspended: false,
            };
            window.resolution.set_scale_factor(1.5);
            size.apply(Entity::PLACEHOLDER, &mut window, 1.0, old);
            if metadata_first {
                size.apply(Entity::PLACEHOLDER, &mut window, 1.0, new);
            }
            window.resolution.set_scale_factor(2.0);
            window.resolution.set_physical_resolution(853, 533);
            size.apply(
                Entity::PLACEHOLDER,
                &mut window,
                1.0,
                if metadata_first { new } else { old },
            );
            size.apply(Entity::PLACEHOLDER, &mut window, 1.0, new);
            assert_eq!(window.physical_size(), DESIGN);
            assert!(!size.needs_update(&window, 1.0, new));
            window.resolution.set_physical_resolution(800, 500);
            size.apply(Entity::PLACEHOLDER, &mut window, 1.0, new);
            size.apply(Entity::PLACEHOLDER, &mut window, 2.0, new);
            assert_eq!(window.physical_size(), UVec2::new(1600, 1000));
        }
    }

    #[test]
    fn work_area_accounts_for_native_chrome_and_recovers_after_expansion() {
        let (mut window, mut size) = fixture();
        window.resolution.set_scale_factor(1.5);
        let small = Environment {
            desktop: Some(Desktop {
                work: IRect::new(100, 50, 1000, 650),
                base_scale: 1.5,
                chrome: UVec2::new(16, 40),
            }),
            suspended: false,
        };
        size.apply(Entity::PLACEHOLDER, &mut window, 2.0, small);
        assert_eq!(window.physical_size(), UVec2::new(884, 560));
        assert_eq!(window.resize_constraints.max_width, 884.0 / 1.5);
        let large = Environment {
            desktop: Some(Desktop {
                work: IRect::new(100, 50, 3940, 2210),
                ..small.desktop.unwrap()
            }),
            suspended: false,
        };
        size.apply(Entity::PLACEHOLDER, &mut window, 2.0, large);
        assert_eq!(window.physical_size(), DESIGN * 2);
        assert!(!size.needs_update(&window, 2.0, large));
    }

    #[test]
    fn a_new_out_of_bounds_position_is_clamped_without_recentering_a_valid_position() {
        let (mut window, mut size) = fixture();
        let environment = Environment {
            desktop: Some(Desktop {
                work: IRect::new(-1920, 0, 0, 1040),
                base_scale: 1.0,
                chrome: UVec2::ZERO,
            }),
            suspended: false,
        };
        window.position = WindowPosition::At(IVec2::new(-1800, 100));
        size.apply(Entity::PLACEHOLDER, &mut window, 1.0, environment);
        assert_eq!(window.position, WindowPosition::At(IVec2::new(-1800, 100)));
        window.position = WindowPosition::At(IVec2::new(-100, 100));
        assert!(size.needs_update(&window, 1.0, environment));
        size.apply(Entity::PLACEHOLDER, &mut window, 1.0, environment);
        assert_eq!(window.position, WindowPosition::At(IVec2::new(-640, 100)));
    }
}
