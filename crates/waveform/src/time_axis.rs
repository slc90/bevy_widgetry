use bevy::prelude::*;
use bevy_widgetry_core::diagnostics::FailureState;
use bevy_widgetry_core::scene::{logged_error, spawn_scene};
use bevy_widgetry_core::text::WidgetryText;
use bevy_widgetry_internal_log::{widgetry_error, widgetry_info};

#[derive(Component, Default)]
pub(crate) struct AxisState {
    ticks: Vec<TimeTick>,
    parts: Vec<TickParts>,
    label_spacing: f32,
    revision: u64,
    failure: FailureState,
    layout_failure: FailureState,
}

#[derive(Component, FromTemplate, Clone, Copy)]
struct TickParts {
    mark: Entity,
    label: Entity,
}

pub(crate) fn prepare(world: &mut World) -> Result {
    let axes: Vec<_> = world
        .query_filtered::<(Entity, &ChildOf), With<AxisState>>()
        .iter(world)
        .map(|(axis, parent)| (axis, parent.parent()))
        .collect();
    let mut failure = None;
    for (axis, root) in axes {
        let result = prepare_one(world, axis, root);
        let result = world
            .get_mut::<AxisState>(axis)
            .ok_or_else(|| logged_error("Waveform axis state missing"))?
            .failure
            .observe(
                result,
                |error| widgetry_error!(?root, %error, "Waveform 时间标尺更新失败"),
                || widgetry_info!(?root, "Waveform 时间标尺恢复正常"),
            );
        if let Err(error) = result
            && failure.is_none()
        {
            failure = Some(error);
        }
    }
    failure.map_or(Ok(()), Err)
}

fn prepare_one(world: &mut World, axis: Entity, root: Entity) -> Result {
    let runtime = world
        .get::<crate::WaveformRuntime>(root)
        .ok_or_else(|| logged_error("Waveform axis runtime missing"))?;
    let range = runtime.viewport_range();
    let rate = runtime.metadata().map(|metadata| metadata.sample_rate);
    let node = world
        .get::<ComputedNode>(axis)
        .ok_or_else(|| logged_error("Waveform axis layout missing"))?;
    let width = node.size().x * node.inverse_scale_factor();
    let state = world
        .get::<AxisState>(axis)
        .ok_or_else(|| logged_error("Waveform axis state missing"))?;
    let values = rate.map_or_else(Vec::new, |rate| {
        let start = u128::from(range.start) * 1_000_000_000 / u128::from(rate);
        let span = u128::from(range.end - range.start) * 1_000_000_000 / u128::from(rate);
        let origin = u128::from(range.start) * 1_000_000_000;
        let extent = u128::from(range.end - range.start) * 1_000_000_000;
        ticks(start, span, width, state.label_spacing)
            .into_iter()
            .filter_map(|mut tick| {
                let at = tick.nanos * u128::from(rate);
                if at < origin || at > origin + extent {
                    return None;
                }
                tick.fraction = ((at - origin) as f64 / extent as f64) as f32;
                Some(tick)
            })
            .collect()
    });
    let mut parts = state.parts.clone();
    while parts.len() < values.len() {
        let entity = spawn_scene(world, bsn! {
            TickParts { mark: #TimeMark, label: #TimeLabel }
            template(move |_| Ok(ChildOf(axis)))
            Node { position_type: PositionType::Absolute, width: percent(100), height: percent(100) }
            Pickable::IGNORE
            Children [
                #TimeMark
                Name::new("WaveformTimeMark")
                Node { position_type: PositionType::Absolute, width: px(1), height: px(6) }
                BackgroundColor::default()
                Pickable::IGNORE
            --
                #TimeLabel
                Name::new("WaveformTimeLabel")
                Text::default()
                TextFont { font_size: bevy::text::FontSize::Px(12.0) }
                TextLayout::no_wrap()
                WidgetryText
                Node { position_type: PositionType::Absolute, top: px(7), flex_shrink: 0.0 }
                UiTransform::from_translation(Val2::percent(-50, 0))
                Pickable::IGNORE
            ]
        }).map_err(|error| logged_error(error.to_string()))?;
        parts.push(
            *world
                .get::<TickParts>(entity)
                .ok_or_else(|| logged_error("Waveform tick parts missing"))?,
        );
        world
            .get_mut::<AxisState>(axis)
            .ok_or_else(|| logged_error("Waveform axis state missing"))?
            .parts = parts.clone();
    }
    for (index, part) in parts.iter().enumerate() {
        let tick = values.get(index);
        let label_width = world
            .get::<ComputedNode>(part.label)
            .map_or(0.0, |node| node.size().x * node.inverse_scale_factor());
        let mut mark = world
            .get_mut::<Node>(part.mark)
            .ok_or_else(|| logged_error("Waveform tick mark missing"))?;
        mark.display = if tick.is_some() {
            Display::Flex
        } else {
            Display::None
        };
        if let Some(tick) = tick {
            mark.left = percent(tick.fraction * 100.0);
            mark.height = px(if tick.major { 6 } else { 3 });
        }
        let mut label = world
            .get_mut::<Node>(part.label)
            .ok_or_else(|| logged_error("Waveform tick label missing"))?;
        label.display = if tick.is_some_and(|tick| tick.major) {
            Display::Flex
        } else {
            Display::None
        };
        if let Some(tick) = tick {
            let edge = (label_width * 0.5 / width.max(1.0)).min(0.5);
            label.left = percent(tick.fraction.clamp(edge, 1.0 - edge) * 100.0);
            world
                .get_mut::<Text>(part.label)
                .ok_or_else(|| logged_error("Waveform tick text missing"))?
                .set_if_neq(Text(tick.label.clone()));
        }
    }
    let mut state = world
        .get_mut::<AxisState>(axis)
        .ok_or_else(|| logged_error("Waveform axis state missing"))?;
    if state.ticks != values {
        state.ticks = values;
        state.revision = state.revision.wrapping_add(1);
    }
    Ok(())
}

pub(crate) fn finish(
    mut axes: Query<(Entity, &mut AxisState, &ComputedNode, &UiGlobalTransform)>,
    mut labels: Query<(&ComputedNode, &UiGlobalTransform, &mut Visibility), Without<AxisState>>,
    mut marks: Query<&mut BackgroundColor>,
    theme: Res<bevy_widgetry_theme::WidgetryThemeMode>,
) -> Result {
    let mut failure = None;
    for (axis, mut state, node, transform) in &mut axes {
        let result = (|| -> Result<f32, BevyError> {
            let mut last_right = f32::NEG_INFINITY;
            let mut measured = 0.0_f32;
            let width = node.size().x;
            let left = transform.translation.x - width * 0.5;
            for (index, part) in state.parts.iter().enumerate() {
                let (label_node, label_transform, mut visibility) = labels
                    .get_mut(part.label)
                    .map_err(|error| logged_error(error.to_string()))?;
                // NoWrap node sizes already include this frame's font measurement.
                // TextLayoutInfo is refreshed later in PostLayout, which must follow visibility and menu focus.
                let label_width = label_node.size().x;
                let label_left = label_transform.translation.x - label_width * 0.5;
                let label_right = label_left + label_width;
                let major = state.ticks.get(index).is_some_and(|tick| tick.major);
                let visible = major
                    && label_width > 0.0
                    && label_left >= left - 0.5
                    && label_right <= left + width + 0.5
                    && label_left >= last_right + 8.0 / node.inverse_scale_factor();
                *visibility = if visible {
                    Visibility::Inherited
                } else {
                    Visibility::Hidden
                };
                if major {
                    measured = measured.max(label_width * node.inverse_scale_factor() + 16.0);
                }
                if visible {
                    last_right = label_right;
                }
                marks
                    .get_mut(part.mark)
                    .map_err(|error| logged_error(error.to_string()))?
                    .0 = theme.colors().text.normal.foreground.with_alpha(0.4);
            }
            Ok(measured.max(90.0))
        })();
        let result = state.layout_failure.observe(
            result,
            |error| widgetry_error!(?axis, %error, "Waveform 时间标尺布局检查失败"),
            || widgetry_info!(?axis, "Waveform 时间标尺布局检查恢复正常"),
        );
        match result {
            Ok(spacing) => state.label_spacing = spacing,
            Err(error) if failure.is_none() => failure = Some(error),
            Err(_) => {}
        }
    }
    failure.map_or(Ok(()), Err)
}

pub(crate) fn grid_snapshot(world: &World, root: Entity) -> Result<(u64, Vec<f32>), BevyError> {
    let children = world
        .get::<Children>(root)
        .ok_or_else(|| logged_error("Waveform children missing"))?;
    let axis = children
        .iter()
        .find_map(|child| world.get::<AxisState>(child))
        .ok_or_else(|| logged_error("Waveform axis missing"))?;
    Ok((
        axis.revision,
        axis.ticks
            .iter()
            .filter(|tick| tick.major)
            .map(|tick| tick.fraction)
            .collect(),
    ))
}

#[derive(Component, Clone, Copy)]
pub(crate) struct GridState(pub bool);

#[derive(EntityEvent, Clone, Copy, Debug)]
pub struct WaveformGridChanged {
    pub entity: Entity,
    pub visible: bool,
}

#[derive(Clone, Debug, PartialEq)]
struct TimeTick {
    nanos: u128,
    fraction: f32,
    major: bool,
    label: String,
}

fn ticks(start: u128, span: u128, width: f32, label_spacing: f32) -> Vec<TimeTick> {
    if span == 0 || !width.is_finite() || width <= 0.0 {
        return Vec::new();
    }
    let desired = (span as f64 * f64::from(label_spacing.max(90.0)) / f64::from(width.max(1.0)))
        .ceil() as u128;
    let mut power = 1_u128;
    let major = loop {
        if let Some(step) = [1, 2, 5]
            .into_iter()
            .map(|factor| factor * power)
            .find(|&step| step >= desired)
        {
            break step;
        }
        power *= 10;
    };
    let minor = [5, 2]
        .into_iter()
        .find_map(|divisor| {
            let step = major / divisor;
            (major.is_multiple_of(divisor) && step as f64 / span as f64 * f64::from(width) >= 20.0)
                .then_some(step)
        })
        .unwrap_or(major);
    let precision = 9_u32.saturating_sub(major.ilog10()) as usize;
    let mut at = start.div_ceil(minor) * minor;
    let mut result = Vec::new();
    while at <= start + span {
        let is_major = at.is_multiple_of(major);
        let label = if is_major {
            let seconds = at / 1_000_000_000;
            if precision == 0 {
                format!("{seconds} s")
            } else {
                let fraction = (at % 1_000_000_000) / 10_u128.pow(9 - precision as u32);
                format!("{seconds}.{fraction:0precision$} s")
            }
        } else {
            String::new()
        };
        result.push(TimeTick {
            nanos: at,
            fraction: (at - start) as f32 / span as f32,
            major: is_major,
            label,
        });
        at += minor;
    }
    result
}

#[cfg(test)]
#[allow(clippy::disallowed_macros, clippy::unwrap_used)]
mod tests {
    use super::*;

    const SECOND: u128 = 1_000_000_000;

    #[test]
    fn menu_text_input_and_axis_can_share_the_real_ui_schedule() {
        let mut app = bevy_widgetry_test_utils::benchmark::ui_app().unwrap();
        app.add_message::<bevy::window::Ime>()
            .init_resource::<ButtonInput<bevy::input::keyboard::Key>>();
        app.add_plugins((
            TransformPlugin,
            bevy::ui_widgets::TextInputPlugin,
            bevy::ui_widgets::MenuPlugin,
            crate::WaveformRenderPlugin,
        ));
        app.update();
    }

    #[test]
    fn absolute_ticks_anchor_to_zero_and_move_continuously() {
        let current = ticks(25 * SECOND, 10 * SECOND, 500.0, 90.0);
        let majors: Vec<_> = current.iter().filter(|tick| tick.major).collect();
        assert_eq!(
            majors
                .iter()
                .map(|tick| tick.label.as_str())
                .collect::<Vec<_>>(),
            ["26 s", "28 s", "30 s", "32 s", "34 s"]
        );
        let moved = ticks(25 * SECOND + SECOND / 10, 10 * SECOND, 500.0, 90.0);
        for tick in majors {
            let next = moved.iter().find(|next| next.nanos == tick.nanos).unwrap();
            assert!((tick.fraction - next.fraction - 0.01).abs() < 0.00001);
        }
    }

    #[test]
    fn fractional_labels_density_and_long_timestamps_remain_distinct() {
        let short = ticks(0, SECOND, 500.0, 90.0);
        assert!(short.iter().any(|tick| tick.label == "0.2 s"));
        for (width, spacing) in [(30.0, 90.0), (500.0, 140.0), (1600.0, 90.0)] {
            let values = ticks(0, 30 * SECOND, width, spacing);
            let majors: Vec<_> = values.iter().filter(|tick| tick.major).collect();
            assert!(
                majors
                    .windows(2)
                    .all(|pair| (pair[1].fraction - pair[0].fraction) * width >= spacing - 0.01)
            );
            assert!(
                values
                    .windows(2)
                    .all(|pair| (pair[1].fraction - pair[0].fraction) * width >= 20.0 - 0.01)
            );
        }
        let far = u128::from(u64::MAX) * SECOND;
        let values = ticks(far, SECOND, 500.0, 90.0);
        assert!(values.iter().filter(|tick| tick.major).count() >= 5);
        assert!(values.windows(2).all(|pair| pair[0].nanos < pair[1].nanos));
    }
}
