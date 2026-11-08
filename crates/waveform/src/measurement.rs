const VOLTS_TO_MILLIVOLTS: f64 = 1e3;
const VOLTS_TO_MICROVOLTS: f64 = 1e6;
const SELECTION_FILL_ALPHA: f32 = 0.08;
const LABEL_BACKGROUND_ALPHA: f32 = 0.9;

use crate::view_error::ViewResult;
use bevy_widgetry_job::JobFailure;
use bevy_widgetry_toggle_button::WidgetryToggleButtonHeadless;
mod gesture;
mod input;

use crate::view::{MeasurementOwner, MeasurementParts};
use crate::view_error::{ViewErrors, ViewFailureState};
use crate::{Waveform, WaveformChannelLayout, WaveformRuntime, WaveformUnit};
use bevy::prelude::*;
use bevy_widgetry_core::ui::WidgetryUiSystems;
use bevy_widgetry_toggle_button::{
    WidgetryToggleButton, WidgetryToggleButtonChanged, WidgetryToggleButtonPlugin,
};

/// Screen scale differences, unrelated to channel/sample selection.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct WaveformMeasurement {
    /// Plot-local logical pixels, retained independently of the moving time axis.
    pub rectangle: Rect,
    pub delta_seconds: f64,
    pub delta_volts: f64,
    pub display_unit: WaveformUnit,
}

#[derive(EntityEvent, Clone, Copy, Debug)]
pub struct WaveformMeasurementChanged {
    pub entity: Entity,
    pub measurement: Option<WaveformMeasurement>,
}

#[derive(Component, Default)]
pub(crate) struct MeasurementState {
    rectangle: Option<Rect>,
    generation: Option<u64>,
    drag: Option<input::Drag>,
    published: Option<WaveformMeasurement>,
    failure: ViewFailureState,
}

fn parts(world: &World, root: Entity) -> Result<&MeasurementParts, JobFailure> {
    Waveform::validate_target(world, root)?;
    world.get::<MeasurementParts>(root).ok_or_else(|| {
        crate::view_error::missing(crate::WaveformPart::MeasurementParts)
            .into_failure(Some(root), "waveform.measurement.parts")
    })
}

impl Waveform {
    pub fn measurement_toggle(world: &World, root: Entity) -> Result<Entity, JobFailure> {
        Ok(parts(world, root)?.toggle)
    }
    pub fn measurement_enabled(world: &World, root: Entity) -> Result<bool, JobFailure> {
        let toggle = Self::measurement_toggle(world, root)?;
        WidgetryToggleButtonHeadless::selected(world, toggle).map_err(|error| {
            JobFailure::from(error).with_context(
                bevy_widgetry_diagnostics::ErrorFrame::new("waveform.measurement.enabled")
                    .field("root", root)
                    .field("toggle", toggle),
            )
        })
    }
    pub fn set_measurement_enabled_in_world(
        world: &mut World,
        root: Entity,
        enabled: bool,
    ) -> Result<bool, JobFailure> {
        let toggle = Self::measurement_toggle(world, root)?;
        let changed = WidgetryToggleButtonHeadless::set_selected_in_world(world, toggle, enabled)
            .map_err(|error| {
            JobFailure::from(error).with_context(
                bevy_widgetry_diagnostics::ErrorFrame::new("waveform.measurement.set")
                    .field("root", root)
                    .field("toggle", toggle),
            )
        })?;
        if !enabled {
            Self::clear_measurement_in_world(world, root)?;
        }
        Ok(changed)
    }
    pub fn set_measurement_enabled(commands: &mut Commands, root: Entity, enabled: bool) {
        commands.queue(move |world: &mut World| {
            Self::set_measurement_enabled_in_world(world, root, enabled)
                .map(|_| ())
                .map_err(bevy_widgetry_job::JobFailure::into_bevy_error)
        });
    }
    pub fn clear_measurement_in_world(world: &mut World, root: Entity) -> Result<bool, JobFailure> {
        Self::validate_target(world, root)?;
        clear(world, root)
            .map_err(|error| error.into_failure(Some(root), "waveform.measurement.clear"))
    }
    pub fn clear_measurement(commands: &mut Commands, root: Entity) {
        commands.queue(move |world: &mut World| {
            Self::clear_measurement_in_world(world, root)
                .map(|_| ())
                .map_err(bevy_widgetry_job::JobFailure::into_bevy_error)
        });
    }
    pub fn measurement(
        world: &World,
        root: Entity,
    ) -> Result<Option<WaveformMeasurement>, JobFailure> {
        Self::validate_target(world, root)?;
        snapshot(world, root)
            .map_err(|error| error.into_failure(Some(root), "waveform.measurement"))
    }
}

fn snapshot(world: &World, root: Entity) -> ViewResult<Option<WaveformMeasurement>> {
    let parts = world.get::<MeasurementParts>(root).ok_or_else(|| {
        crate::view_error::missing(crate::WaveformPart::MeasurementParts).at(root)
    })?;
    if !WidgetryToggleButtonHeadless::selected(world, parts.toggle).map_err(|error| {
        JobFailure::from(error).with_context(
            bevy_widgetry_diagnostics::ErrorFrame::new("waveform.measurement.query")
                .field("root", root)
                .field("toggle", parts.toggle),
        )
    })? {
        return Ok(None);
    }
    let Some(rectangle) = world
        .get::<MeasurementState>(root)
        .and_then(|state| state.rectangle)
    else {
        return Ok(None);
    };
    let Some(size) = size(world, root) else {
        return Ok(None);
    };
    let layout = world
        .get::<WaveformChannelLayout>(root)
        .ok_or_else(|| crate::view_error::missing(crate::WaveformPart::ChannelLayout).at(root))?;
    let runtime = world
        .get::<WaveformRuntime>(root)
        .ok_or_else(|| crate::view_error::missing(crate::WaveformPart::Runtime).at(root))?;
    let Some(metadata) = runtime.metadata() else {
        return Ok(None);
    };
    if layout.row_height <= 0.0 || size.min_element() <= 0.0 {
        return Ok(None);
    }
    let rectangle = clamp_rectangle(rectangle, size);
    let range = runtime.viewport_range();
    let duration = (range.end - range.start) as f64 / f64::from(metadata.sample_rate);
    let amplitude = runtime.config().amplitude_range;
    Ok(Some(WaveformMeasurement {
        rectangle,
        delta_seconds: f64::from(rectangle.width()) / f64::from(size.x) * duration,
        delta_volts: (f64::from(rectangle.height()) / f64::from(layout.row_height))
            * 2.0
            * amplitude.volts(),
        display_unit: amplitude.unit,
    }))
}
fn clear(world: &mut World, root: Entity) -> ViewResult<bool> {
    world.get::<MeasurementParts>(root).ok_or_else(|| {
        crate::view_error::missing(crate::WaveformPart::MeasurementParts).at(root)
    })?;
    input::end(world, root);
    let generation = world
        .get::<WaveformRuntime>(root)
        .map(WaveformRuntime::session_generation);
    let mut state = world
        .get_mut::<MeasurementState>(root)
        .ok_or_else(|| crate::view_error::missing(crate::WaveformPart::MeasurementState))?;
    state.generation = generation;
    let changed = state.rectangle.take().is_some();
    if changed {
        publish(world, root)?;
    }
    Ok(changed)
}

fn size(world: &World, root: Entity) -> Option<Vec2> {
    let parts = world.get::<MeasurementParts>(root)?;
    let node = world.get::<ComputedNode>(parts.surface)?;
    let size = node.size() * node.inverse_scale_factor();
    (size.is_finite() && size.min_element() > 0.0).then_some(size)
}
fn clamp_rectangle(rectangle: Rect, size: Vec2) -> Rect {
    let extent = rectangle.size().min(size);
    let min = rectangle
        .min
        .clamp(Vec2::ZERO, (size - extent).max(Vec2::ZERO));
    Rect::from_corners(min, min + extent)
}

fn publish(world: &mut World, root: Entity) -> ViewResult {
    let measurement = snapshot(world, root)?;
    let mut state = world.get_mut::<MeasurementState>(root).ok_or_else(|| {
        crate::view_error::missing(crate::WaveformPart::MeasurementState).at(root)
    })?;
    if state.published != measurement {
        state.published = measurement;
        world.trigger(WaveformMeasurementChanged {
            entity: root,
            measurement,
        });
    }
    Ok(())
}

fn toggle_changed(
    event: On<WidgetryToggleButtonChanged>,
    roots: Query<(Entity, &MeasurementParts)>,
    mut commands: Commands,
) {
    if !event.selected
        && let Some((root, _)) = roots.iter().find(|(_, parts)| parts.toggle == event.entity)
    {
        commands.queue(move |world: &mut World| {
            Waveform::clear_measurement_in_world(world, root)
                .map(|_| ())
                .map_err(JobFailure::into_bevy_error)
        });
    }
}

pub(crate) fn install(app: &mut App) {
    if !app.is_plugin_added::<WidgetryToggleButtonPlugin>() {
        app.add_plugins(WidgetryToggleButtonPlugin);
    }
    input::install(app);
    app.add_observer(toggle_changed)
        .add_systems(PostUpdate, prepare.in_set(WidgetryUiSystems::Build))
        .add_systems(
            PostUpdate,
            prepare
                .after(crate::channels::finish)
                .before(crate::renderer::render),
        );
}

fn prepare(world: &mut World) -> Result {
    let roots = world
        .query_filtered::<Entity, With<MeasurementParts>>()
        .iter(world)
        .collect::<Vec<_>>();
    let mut failure = ViewErrors::default();
    for root in roots {
        let result = prepare_one(world, root);
        let result = world
            .get_mut::<MeasurementState>(root)
            .ok_or_else(|| {
                crate::view_error::missing(crate::WaveformPart::MeasurementState)
                    .into_bevy_error(None, "waveform.prepare")
            })?
            .failure
            .observe(result, root, "waveform.measurement.prepare");
        if let Err(error) = result {
            failure.push(error);
        }
    }
    failure.finish(world.commands())
}

fn prepare_one(world: &mut World, root: Entity) -> ViewResult {
    let generation = world
        .get::<WaveformRuntime>(root)
        .map(WaveformRuntime::session_generation);
    if world
        .get::<MeasurementState>(root)
        .is_some_and(|state| state.generation.is_some() && state.generation != generation)
    {
        clear(world, root)?;
    }
    world
        .get_mut::<MeasurementState>(root)
        .ok_or_else(|| crate::view_error::missing(crate::WaveformPart::MeasurementState).at(root))?
        .generation = generation;
    let toggle = world
        .get::<MeasurementParts>(root)
        .ok_or_else(|| crate::view_error::missing(crate::WaveformPart::MeasurementParts).at(root))?
        .toggle;
    if world.get::<WidgetryToggleButton>(toggle).is_none() {
        return Err(crate::view_error::missing(crate::WaveformPart::MeasurementToggle).at(toggle));
    }
    let enabled = WidgetryToggleButtonHeadless::selected(world, toggle)?;
    if !enabled {
        clear(world, root)?;
    }
    if let Some(size) = size(world, root) {
        let mut state = world.get_mut::<MeasurementState>(root).ok_or_else(|| {
            crate::view_error::missing(crate::WaveformPart::MeasurementState).at(root)
        })?;
        if let Some(rect) = state.rectangle {
            state.rectangle = Some(clamp_rectangle(rect, size));
        }
    }
    publish(world, root)?;
    let measurement = snapshot(world, root)?;
    let parts = parts(world, root)?;
    let (surface, outline, readout) = (parts.surface, parts.outline, parts.readout);
    world
        .entity_mut(surface)
        .insert_if_new(MeasurementOwner { root });
    world
        .get_mut::<Pickable>(surface)
        .ok_or_else(|| {
            crate::view_error::missing(crate::WaveformPart::MeasurementPickable).at(surface)
        })?
        .set_if_neq(if enabled {
            Pickable::default()
        } else {
            Pickable::IGNORE
        });
    let colors = world.resource::<bevy_widgetry_theme::WidgetryTheme>();
    let border = colors.check_box.checked.normal.border;
    let background = colors.waveform.normal.background;
    world
        .get_mut::<BorderColor>(outline)
        .ok_or_else(|| {
            crate::view_error::missing(crate::WaveformPart::MeasurementBorder).at(outline)
        })?
        .set_if_neq(BorderColor::all(border));
    world
        .get_mut::<BackgroundColor>(outline)
        .ok_or_else(|| {
            crate::view_error::missing(crate::WaveformPart::MeasurementBackground).at(outline)
        })?
        .set_if_neq(BackgroundColor(border.with_alpha(SELECTION_FILL_ALPHA)));
    world
        .get_mut::<BackgroundColor>(readout)
        .ok_or_else(|| {
            crate::view_error::missing(crate::WaveformPart::MeasurementReadoutBackground)
                .at(readout)
        })?
        .set_if_neq(BackgroundColor(
            background.with_alpha(LABEL_BACKGROUND_ALPHA),
        ));
    let mut node = world.get_mut::<Node>(outline).ok_or_else(|| {
        crate::view_error::missing(crate::WaveformPart::MeasurementRectangleNode).at(outline)
    })?;
    let mut next = node.clone();
    next.display = if measurement.is_some() {
        Display::Flex
    } else {
        Display::None
    };
    if let Some(value) = measurement {
        next.left = px(value.rectangle.min.x);
        next.top = px(value.rectangle.min.y);
        next.width = px(value.rectangle.width());
        next.height = px(value.rectangle.height());
    }
    let changed = node.set_if_neq(next);
    let mut node = world.get_mut::<Node>(readout).ok_or_else(|| {
        crate::view_error::missing(crate::WaveformPart::MeasurementReadoutNode).at(readout)
    })?;
    let display = if measurement.is_some() {
        Display::Flex
    } else {
        Display::None
    };
    let display_changed = node.display != display;
    if display_changed {
        node.display = display;
    }
    if let Some(value) = measurement {
        let (scale, unit) = match value.display_unit {
            WaveformUnit::Volts => (1.0, "V"),
            WaveformUnit::Millivolts => (VOLTS_TO_MILLIVOLTS, "mV"),
            WaveformUnit::Microvolts => (VOLTS_TO_MICROVOLTS, "µV"),
        };
        let label = format!(
            "Δt = {:.3} s   ΔV = {:.3} {unit}",
            value.delta_seconds,
            value.delta_volts * scale
        );
        world
            .get_mut::<Text>(readout)
            .ok_or_else(|| {
                crate::view_error::missing(crate::WaveformPart::MeasurementReadoutText).at(readout)
            })?
            .set_if_neq(Text(label));
    }
    if changed || display_changed {
        world.write_message(bevy::window::RequestRedraw);
    }
    Ok(())
}
