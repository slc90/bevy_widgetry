use crate::{WidgetryUiScale, WindowScaleExcluded};
use bevy::{
    camera::CameraUpdateSystems,
    math::DVec2,
    prelude::*,
    window::{WindowResizeConstraints, WindowResized, WindowScaleFactorChanged},
};
use bevy_widgetry_internal_log::widgetry_debug;

pub struct WidgetryUiScalePlugin;

#[derive(SystemSet, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum WidgetryUiScaleSystems {
    Sync,
}

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
struct WindowSize {
    logical: DVec2,
    expected: UVec2,
    scale: f32,
    constraints: WindowResizeConstraints,
}

struct WindowChanges {
    resized: Option<WindowResized>,
    rescaled: Option<WindowScaleFactorChanged>,
}

impl WindowSize {
    fn new(window: &Window) -> Self {
        Self {
            logical: window.physical_size().as_dvec2() / f64::from(window.scale_factor()),
            expected: window.physical_size(),
            scale: window.scale_factor(),
            constraints: window.resize_constraints,
        }
    }

    fn apply(&mut self, entity: Entity, window: &mut Window, scale: f32) -> WindowChanges {
        let previous_size = window.physical_size();
        let previous_scale = window.scale_factor();
        self.constraints = window.resize_constraints;
        if window.physical_size() != self.expected {
            self.logical = window.physical_size().as_dvec2() / f64::from(self.scale);
        }
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
            .max(DVec2::ONE)
            .as_uvec2();
        window.resolution.set_scale_factor_override(Some(scale));
        window
            .resolution
            .set_physical_resolution(physical.x, physical.y);
        self.expected = physical;
        self.scale = scale;
        // winit reports requested changes in Last, after camera extraction data was prepared.
        // Publish them now so explicit viewports fit the new output texture in this frame.
        WindowChanges {
            resized: (previous_size != physical).then(|| WindowResized {
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

fn initialize(world: &mut World, entity: Entity) {
    let scale = world.resource::<WidgetryUiScale>().get();
    if world.get::<WindowScaleExcluded>(entity).is_some()
        || world.get::<WindowSize>(entity).is_some()
    {
        return;
    }
    let Some(mut window) = world.get_mut::<Window>(entity) else {
        return;
    };
    let mut size = WindowSize::new(&window);
    let changes = size.apply(entity, &mut window, scale);
    world.entity_mut(entity).insert(size);
    world.write_message_batch(changes.resized);
    world.write_message_batch(changes.rescaled);
}

fn window_added(event: On<Add<Window>>, mut commands: Commands) {
    let entity = event.entity;
    commands.queue(move |world: &mut World| initialize(world, entity));
}

fn window_removed(event: On<Remove<Window>>, mut commands: Commands) {
    commands.entity(event.entity).try_remove::<WindowSize>();
}

fn exclusion_added(event: On<Add<WindowScaleExcluded>>, mut commands: Commands) {
    commands.entity(event.entity).try_remove::<WindowSize>();
}

fn exclusion_removed(event: On<Remove<WindowScaleExcluded>>, mut commands: Commands) {
    let entity = event.entity;
    commands.queue(move |world: &mut World| initialize(world, entity));
}

fn sync(
    scale: Res<WidgetryUiScale>,
    mut windows: Query<
        (Entity, &mut Window, Option<&mut WindowSize>),
        Without<WindowScaleExcluded>,
    >,
    mut commands: Commands,
    mut resized: MessageWriter<WindowResized>,
    mut rescaled: MessageWriter<WindowScaleFactorChanged>,
) {
    for (entity, mut window, size) in &mut windows {
        let Some(mut size) = size else {
            let mut size = WindowSize::new(&window);
            let changes = size.apply(entity, &mut window, scale.get());
            resized.write_batch(changes.resized);
            rescaled.write_batch(changes.rescaled);
            commands.entity(entity).insert(size);
            continue;
        };
        if size.scale != scale.get()
            || size.constraints != window.resize_constraints
            || window.physical_size() != size.expected
            || window.resolution.scale_factor_override() != Some(scale.get())
        {
            let changes = size.apply(entity, &mut window, scale.get());
            resized.write_batch(changes.resized);
            rescaled.write_batch(changes.rescaled);
        }
    }
}
