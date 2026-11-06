use crate::{ProgressBarError, error::failure};
use crate::{
    WidgetryProgressBar, WidgetryProgressBarProps, animation::Animation, colors::ColorState,
    progress::ProgressState,
};
use bevy::prelude::*;
use bevy_widgetry_core::ui::WidgetryUiSystems;
use bevy_widgetry_diagnostics::{
    ErrorFrame,
    bevy::{DiagnosticBatch, FailureState},
};
use bevy_widgetry_theme::WidgetryTheme;
use std::sync::Arc;

#[derive(Component, FromTemplate)]
struct Parts {
    track: Entity,
    fill: Entity,
    clip: Entity,
}

#[derive(Component, Default)]
struct Diagnostics(FailureState<([Entity; 3], Entity, u8)>);

// Bevy overflow clips rectangles, so crop a full-track rounded canvas instead of shrinking its caps.
// Bound the reciprocal for subpixel fractions so valid subnormal progress never creates infinite layout values.
fn canvas_width(fraction: f32) -> f32 {
    100.0 / fraction.max(f32::EPSILON)
}

pub(crate) fn scene(props: WidgetryProgressBarProps) -> impl Scene {
    let value = props.state;
    let height = props.height;
    let colors = props.colors;
    let (left, fraction) = Animation::new(value).window();
    bsn! {
        template(move |ctx| {
            value.validate().map_err(|error| error.with_context(ErrorFrame::new("progress_bar.initialize").field("root", ctx.entity.id())).into_bevy_error())?;
            if !height.is_finite() || height <= 0.0 {
                return Err(failure(ProgressBarError::InvalidHeight { height }, ErrorFrame::new("progress_bar.initialize").field("root", ctx.entity.id())).into_bevy_error());
            }
            Ok(ProgressState(value))
        })
        template(move |ctx| colors.clone().initial(ctx.entity.id()).map_err(|error| error.into_bevy_error()))
        template(|_| Ok(Diagnostics::default()))
        template(move |_| Ok(Animation::new(value)))
        Parts { track: #ProgressTrack, fill: #ProgressFill, clip: #ProgressClip }
        Pickable::IGNORE
        Node {
            width: percent(100), height: px(height), min_width: px(0),
            border: UiRect::all(px(1)), border_radius: BorderRadius::MAX,
            flex_shrink: 0.0,
        }
        BorderColor::all(Color::NONE)
        Children [
            #ProgressTrack Pickable::IGNORE
            Node { width: percent(100), height: percent(100), min_width: px(0), min_height: px(0), border_radius: BorderRadius::MAX, overflow: Overflow::clip() }
            BackgroundColor(Color::NONE)
            Children [
                #ProgressClip Pickable::IGNORE
                Node { position_type: PositionType::Absolute, left: {percent(left * 100.0)}, top: px(0), width: {percent(fraction * 100.0)}, height: percent(100), overflow: Overflow::clip() }
                BackgroundColor(Color::NONE)
                Children [
                    #ProgressFill Pickable::IGNORE
                    Node { position_type: PositionType::Absolute, left: {percent(-left * canvas_width(fraction))}, top: px(0), width: {percent(canvas_width(fraction))}, height: percent(100), border_radius: BorderRadius::MAX }
                    BackgroundColor(Color::NONE)
                ]
            ]
        ]
    }
}

pub(crate) fn install(app: &mut App) {
    app.add_message::<bevy::window::RequestRedraw>()
        .add_systems(PostUpdate, update.in_set(WidgetryUiSystems::Colors));
}

fn update(
    mode: Res<WidgetryTheme>,
    time: Res<Time<Real>>,
    mut redraw: MessageWriter<bevy::window::RequestRedraw>,
    mut roots: Query<
        (
            Entity,
            Ref<ProgressState>,
            &Parts,
            Ref<ColorState>,
            &mut BorderColor,
            &mut Diagnostics,
            &mut Animation,
        ),
        With<WidgetryProgressBar>,
    >,
    mut visuals: Query<(&mut Node, &mut BackgroundColor)>,
) -> Result {
    let mut failures = DiagnosticBatch::default();
    for (root, state, parts, local, mut border, mut diagnostics, mut animation) in &mut roots {
        let was_moving = animation.active();
        let retargeted = state.is_changed() && animation.retarget(state.0);
        let moving = animation.active();
        // Skip idle time only when starting a transition, so per-frame targets still make progress.
        if moving && (!retargeted || was_moving) {
            animation.advance(time.delta());
        }
        if animation.active() {
            redraw.write(bevy::window::RequestRedraw);
        }
        if !mode.is_changed() && !state.is_changed() && !local.is_changed() && !moving {
            continue;
        }
        let frame = || ErrorFrame::new("progress_bar.style").field("root", root);
        let entities = [parts.track, parts.fill, parts.clip];
        let result = (|| -> Result<(), bevy::ecs::query::QueryEntityError> {
            let colors = local.0.resolve(&mode.progress_bar);
            let [
                (_, mut track),
                (mut fill_node, mut fill),
                (mut clip_node, _),
            ] = visuals.get_many_mut(entities)?;
            border.set_if_neq(BorderColor::all(colors.border));
            track.set_if_neq(BackgroundColor(colors.track));
            fill.set_if_neq(BackgroundColor(colors.fill));
            let (left, fraction) = animation.window();
            let width = percent(fraction * 100.0);
            if clip_node.width != width {
                clip_node.width = width;
            }
            if clip_node.left != percent(left * 100.0) {
                clip_node.left = percent(left * 100.0);
            }
            let offset = percent(-left * canvas_width(fraction));
            if fill_node.left != offset {
                fill_node.left = offset;
            }
            let canvas = percent(canvas_width(fraction));
            if fill_node.width != canvas {
                fill_node.width = canvas;
            }
            Ok(())
        })();
        match result {
            Ok(()) => {
                diagnostics.0.recover(frame);
            }
            Err(source) => {
                let (entity, category) = match &source {
                    bevy::ecs::query::QueryEntityError::QueryDoesNotMatch(entity, _) => {
                        (*entity, 0)
                    }
                    bevy::ecs::query::QueryEntityError::NotSpawned(error) => (error.entity(), 1),
                    bevy::ecs::query::QueryEntityError::AliasedMutability(entity) => (*entity, 2),
                };
                let error = diagnostics.0.failure((entities, entity, category), || {
                    Arc::new(failure(
                        ProgressBarError::VisualStructure {
                            root,
                            track: parts.track,
                            fill: parts.fill,
                            clip: parts.clip,
                            source,
                        },
                        frame(),
                    ))
                });
                for diagnostic in error.shared_diagnostics() {
                    failures.push_shared(diagnostic);
                }
            }
        }
    }
    failures.into_bevy_error().map_or(Ok(()), Err)
}

#[cfg(test)]
#[allow(clippy::disallowed_macros)]
mod tests {
    use super::*;

    #[test]
    fn clipped_canvas_is_finite_at_zero_and_subnormal_progress() {
        for fraction in [0.0, f32::from_bits(1), f32::EPSILON, 0.01, 0.65, 1.0] {
            assert!(canvas_width(fraction).is_finite());
            if fraction >= f32::EPSILON {
                assert!((canvas_width(fraction) * fraction - 100.0).abs() < 0.001);
            }
        }
    }
}
