const DIRECTION_DEADZONE_SQUARED: f32 = 4.0;
const TRACK_HIT_MARGIN: f32 = 18.0;
const THUMB_HIT_RADIUS: f32 = 16.0;

const TICK_INTERVALS: i32 = 48;
const TICK_ORIGIN: f32 = 98.0;

const TICK_RADIUS: f32 = 3.0;

const TICK_DIAMETER: f32 = 6.0;
const CONTROL_DIAMETER: f32 = 200.0;
const CONTROL_BORDER_WIDTH: f32 = 2.0;
const THUMB_DIAMETER: f32 = 22.0;
const INITIAL_THUMB_LEFT: f32 = 100.0;
const INITIAL_THUMB_TOP: f32 = 30.0;

const DETAIL_GAP: f32 = 8.0;
const CONTROL_GAP: f32 = 8.0;
const LABEL_FONT_SIZE: f32 = 22.0;

use crate::i18n::{Language, SliderKey, TextKey, localized, tr_args};
use bevy::{
    input::keyboard::KeyboardInput,
    input_focus::{FocusCause, InputFocus, tab_navigation::TabIndex},
    prelude::*,
    ui::{
        ComputedUiRenderTargetInfo, InteractionDisabled, Pressed, UiGlobalTransform, UiTransform,
        Val2,
    },
    ui_widgets::{Activate, ValueChange},
};
use bevy_widgetry::{
    button::WidgetryButton,
    slider::{
        SliderDirection, SliderDragEnded, SliderDragOutcome, SliderDragStrategy, SliderFailure,
        SliderInteractionStart, SliderInteractionStrategy, SliderPointerInput, SliderProjection,
        WidgetrySliderHeadless, WidgetrySliderInteraction, WidgetrySliderState,
    },
    text::WidgetryText,
    theme::WidgetryTheme,
};

const START: f64 = std::f64::consts::PI * 0.75;
const SPAN: f64 = std::f64::consts::PI * 1.5;
const RADIUS: f32 = 70.0;

struct ArcStrategy;
struct ArcDrag {
    previous: f64,
    angle: f64,
    offset: f64,
}

fn local_point(world: &World, input: SliderPointerInput) -> Option<Vec2> {
    let transform = world.get::<UiGlobalTransform>(input.entity)?;
    let node = world.get::<ComputedNode>(input.entity)?;
    if node.size().min_element() <= 0.0 {
        return None;
    }
    let target = world.get::<ComputedUiRenderTargetInfo>(input.entity)?;
    let camera = world
        .get::<bevy::ui::ComputedUiTargetCamera>(input.entity)?
        .get()?;
    let origin = world
        .get::<Camera>(camera)?
        .physical_viewport_rect()?
        .min
        .as_vec2();
    let scale = world.get_resource::<UiScale>().map_or(1.0, |scale| scale.0);
    let local = transform
        .try_inverse()?
        .transform_point2(input.position * target.scale_factor() / scale - origin);
    let local = local * (CONTROL_DIAMETER / node.size().x);
    local.is_finite().then_some(local)
}

fn position(angle: f64, state: &WidgetrySliderState) -> f64 {
    f64::from(state.min())
        + (angle - START) / SPAN * (f64::from(state.max()) - f64::from(state.min()))
}

impl SliderInteractionStrategy for ArcStrategy {
    fn begin(
        &self,
        world: &World,
        input: SliderPointerInput,
    ) -> Result<Option<SliderInteractionStart>, SliderFailure> {
        let Some(local) = local_point(world, input) else {
            return Ok(None);
        };
        Ok(
            ArcDrag::begin(local, &input.state).map(|(position, drag)| SliderInteractionStart {
                position,
                drag: Box::new(drag),
            }),
        )
    }

    fn key(
        &self,
        _: &World,
        _: Entity,
        input: &KeyboardInput,
    ) -> Result<Option<SliderDirection>, SliderFailure> {
        Ok(match input.key_code {
            KeyCode::KeyW => Some(SliderDirection::Increase),
            KeyCode::KeyS => Some(SliderDirection::Decrease),
            _ => None,
        })
    }
}
impl SliderDragStrategy for ArcDrag {
    fn project(
        &mut self,
        world: &World,
        input: SliderPointerInput,
    ) -> Result<SliderProjection, SliderFailure> {
        let Some(local) = local_point(world, input) else {
            return Ok(SliderProjection::Wait);
        };
        if local.length_squared() < DIRECTION_DEADZONE_SQUARED {
            return Ok(SliderProjection::Wait);
        }
        let raw = f64::from(local.y.atan2(local.x));
        Ok(SliderProjection::Position(self.advance(raw, &input.state)))
    }
}
impl ArcDrag {
    fn begin(local: Vec2, state: &WidgetrySliderState) -> Option<(f64, Self)> {
        if !(RADIUS - TRACK_HIT_MARGIN..=RADIUS + TRACK_HIT_MARGIN).contains(&local.length()) {
            return None;
        }
        let raw = f64::from(local.y.atan2(local.x));
        let ratio = (state.value() - state.min()) / (state.max() - state.min());
        let knob_angle = START + f64::from(ratio) * SPAN;
        let knob = Vec2::new(knob_angle.cos() as f32, knob_angle.sin() as f32) * RADIUS;
        let on_knob = local.distance_squared(knob) <= THUMB_HIT_RADIUS.powi(2);
        let angle = if on_knob {
            knob_angle + (raw - knob_angle + std::f64::consts::PI).rem_euclid(std::f64::consts::TAU)
                - std::f64::consts::PI
        } else {
            START + (raw - START).rem_euclid(std::f64::consts::TAU)
        };
        if !on_knob && angle > START + SPAN {
            return None;
        }

        let projected = position(angle, state);
        let offset = if on_knob {
            projected - f64::from(state.value())
        } else {
            0.0
        };
        Some((
            if on_knob {
                f64::from(state.value())
            } else {
                projected
            },
            ArcDrag {
                previous: raw,
                angle,
                offset,
            },
        ))
    }

    fn advance(&mut self, raw: f64, state: &WidgetrySliderState) -> f64 {
        self.angle += (raw - self.previous + std::f64::consts::PI)
            .rem_euclid(std::f64::consts::TAU)
            - std::f64::consts::PI;
        self.previous = raw;
        position(self.angle, state) - self.offset
    }
}

#[derive(Component, Reflect, Default)]
#[reflect(Component)]
struct ArcReadout {
    changes: usize,
    completed: usize,
    cancelled: usize,
}
#[derive(Component)]
struct ArcDot(f32);
#[derive(Component, Default, Clone)]
struct ArcKnob;
#[derive(Component, Default, Clone)]
struct ArcLabel;
#[derive(Component, Clone, Copy, Debug)]
enum Action {
    Focus,
    Disabled,
    Preset,
}

fn dots() -> impl SceneList {
    (0..=TICK_INTERVALS).map(|index| {
        let ratio = index as f32 / TICK_INTERVALS as f32;
        let angle = START + f64::from(ratio)*SPAN;
        let x = TICK_ORIGIN + angle.cos() as f32 * RADIUS;
        let y = TICK_ORIGIN + angle.sin() as f32 * RADIUS;
        bsn! {
            template(move |_|Ok(ArcDot(ratio))) Pickable::IGNORE BackgroundColor
            Node { position_type:PositionType::Absolute,left:px(x-TICK_RADIUS),top:px(y-TICK_RADIUS),width:px(TICK_DIAMETER),height:px(TICK_DIAMETER),border_radius:BorderRadius::MAX }
        }
    }).collect::<Vec<_>>()
}

pub(super) fn scene() -> impl Scene {
    bsn! {
        Node {flex_direction:FlexDirection::Column,align_items:AlignItems::Start,row_gap:px(DETAIL_GAP),flex_shrink:0.0}
        Children [
            @localized(TextKey::Slider(SliderKey::ArcDescription)) WidgetryText--
            @WidgetrySliderHeadless { @value:{Some(50.0)} }
            template(|_|Ok(WidgetrySliderInteraction::new(ArcStrategy)))
            template(|_|Ok(ArcReadout::default()))
            Name("SliderArc") TabIndex(0) on(changed) on(ended)
            Node {width:px(CONTROL_DIAMETER),height:px(CONTROL_DIAMETER),flex_shrink:0.0,border:UiRect::all(px(CONTROL_BORDER_WIDTH)),border_radius:BorderRadius::MAX}
            BorderColor BackgroundColor
            Children [
                {dots()}--
                ArcKnob Name("SliderArcKnob") Pickable::IGNORE BackgroundColor
                Node {position_type:PositionType::Absolute,width:px(THUMB_DIAMETER),height:px(THUMB_DIAMETER),left:px(INITIAL_THUMB_LEFT),top:px(INITIAL_THUMB_TOP),border_radius:BorderRadius::MAX}
                UiTransform {translation:Val2::percent(-50,-50)}--
                ArcLabel Text("") WidgetryText Pickable::IGNORE
                TextFont {font_size:FontSize::Px(LABEL_FONT_SIZE)}
                Node {position_type:PositionType::Absolute,left:percent(50),top:percent(50)}
                UiTransform {translation:Val2::percent(-50,-50)}
            ]--
            Node {column_gap:px(CONTROL_GAP),row_gap:px(DETAIL_GAP),flex_wrap:FlexWrap::Wrap}
            Children [
                @control(Action::Focus,SliderKey::ArcFocus,"SliderArcFocus")--
                @control(Action::Disabled,SliderKey::ArcDisabled,"SliderArcDisabled")--
                @control(Action::Preset,SliderKey::ArcPreset,"SliderArcPreset")
            ]
        ]
    }
}
fn control(action: Action, key: SliderKey, name: &'static str) -> impl Scene {
    bsn! {
        @WidgetryButton { @children:{Some(Box::new(bsn_list!{@localized(TextKey::Slider(key)) WidgetryText}) as Box<dyn SceneList>)} }
        template(move |_|Ok(action)) Name(name) on(operate)
    }
}
fn changed(event: On<ValueChange<f32>>, mut readouts: Query<&mut ArcReadout>) {
    if let Ok(mut readout) = readouts.get_mut(event.source) {
        readout.changes += 1;
    }
}
fn ended(event: On<SliderDragEnded>, mut readouts: Query<&mut ArcReadout>) {
    if let Ok(mut readout) = readouts.get_mut(event.entity) {
        match event.outcome {
            SliderDragOutcome::Completed => readout.completed += 1,
            SliderDragOutcome::Cancelled => readout.cancelled += 1,
        }
        info!(entity=?event.entity,value=event.value,outcome=?event.outcome,"弧形 Slider 操作已结束");
    }
}
fn operate(
    event: On<Activate>,
    actions: Query<&Action>,
    roots: Query<(Entity, Has<InteractionDisabled>), With<ArcReadout>>,
    mut focus: ResMut<InputFocus>,
    mut commands: Commands,
) {
    let Ok(action) = actions.get(event.entity) else {
        return;
    };
    for (root, disabled) in &roots {
        match action {
            Action::Focus => focus.set(root, FocusCause::Navigated),
            Action::Disabled => {
                if disabled {
                    commands.entity(root).remove::<InteractionDisabled>();
                } else {
                    commands.entity(root).insert(InteractionDisabled);
                }
            }
            Action::Preset => WidgetrySliderHeadless::set_value(&mut commands, root, 50.0),
        }
        info!(?root, ?action, "操作弧形 Slider");
    }
}
pub(super) fn install(app: &mut App) {
    app.register_type::<ArcReadout>().add_systems(
        PostUpdate,
        update
            .before(bevy::text::detect_text_needs_rerender)
            .before(bevy::ui::UiSystems::Content)
            .run_if(in_state(crate::gallery::GalleryPage::Slider)),
    );
}
fn update(
    language: Res<Language>,
    theme: Res<WidgetryTheme>,
    roots: Query<
        (
            &WidgetrySliderState,
            Has<InteractionDisabled>,
            Has<Pressed>,
            &Children,
        ),
        With<ArcReadout>,
    >,
    mut borders: Query<&mut BorderColor, With<ArcReadout>>,
    mut backgrounds: Query<&mut BackgroundColor>,
    mut dots: Query<&ArcDot>,
    mut knobs: Query<&mut Node, With<ArcKnob>>,
    mut labels: Query<&mut Text, With<ArcLabel>>,
) {
    for (state, disabled, pressed, children) in &roots {
        let ratio = (state.value() - state.min()) / (state.max() - state.min());
        let color = if disabled {
            Color::srgb_u8(110, 120, 135)
        } else if pressed {
            Color::srgb_u8(245, 180, 70)
        } else {
            Color::srgb_u8(40, 200, 180)
        };
        for child in children {
            if let Ok(dot) = dots.get_mut(*child)
                && let Ok(mut background) = backgrounds.get_mut(*child)
            {
                background.set_if_neq(BackgroundColor(if dot.0 <= ratio {
                    color
                } else {
                    theme.slider.normal.track
                }));
            }
            if let Ok(mut node) = knobs.get_mut(*child) {
                let angle = START + f64::from(ratio) * SPAN;
                node.left = px(TICK_ORIGIN + angle.cos() as f32 * RADIUS);
                node.top = px(TICK_ORIGIN + angle.sin() as f32 * RADIUS);
                if let Ok(mut background) = backgrounds.get_mut(*child) {
                    background.set_if_neq(BackgroundColor(color));
                }
            }
            if let Ok(mut text) = labels.get_mut(*child) {
                text.set_if_neq(Text(tr_args(
                    *language,
                    TextKey::Slider(SliderKey::ArcValue),
                    &[("value", format!("{:.0}", state.value()))],
                )));
            }
        }
    }
    for mut border in &mut borders {
        border.set_if_neq(BorderColor::all(theme.slider.normal.track));
    }
}

#[cfg(test)]
#[allow(clippy::disallowed_macros, clippy::unwrap_used)]
mod tests {
    use super::*;
    use bevy_widgetry_core::scene::spawn_scene;
    #[test]
    fn external_arc_session_crosses_angle_cut_both_ways_and_keeps_grab_offset() {
        let comparison_tolerance = 1e-9;

        let mut app = bevy_widgetry_test_utils::scene_app();
        let root = spawn_scene(
            app.world_mut(),
            bsn! {@WidgetrySliderHeadless { @value:{Some(50.0)} }},
        )
        .unwrap();
        let state = *WidgetrySliderHeadless::state(app.world(), root).unwrap();
        let angle = 179.0_f64.to_radians();
        let mut drag = ArcDrag {
            previous: angle,
            angle,
            offset: position(angle, &state) - 50.0,
        };
        assert!((drag.advance(angle, &state) - 50.0).abs() < comparison_tolerance);
        let after = drag.advance((-179.0_f64).to_radians(), &state);
        assert!((after - (50.0 + 2.0 / 270.0 * 100.0)).abs() < comparison_tolerance);
        assert!((drag.advance(angle, &state) - 50.0).abs() < comparison_tolerance);
    }
    #[test]
    fn endpoint_thumb_can_be_grabbed_on_the_gap_side_without_jumping() {
        let comparison_tolerance = 1e-8;

        let mut app = bevy_widgetry_test_utils::scene_app();
        for (value, angle) in [
            (0.0, 130.0_f64.to_radians()),
            (100.0, 410.0_f64.to_radians()),
        ] {
            let root = spawn_scene(
                app.world_mut(),
                bsn! {@WidgetrySliderHeadless { @value:{Some(value)} }},
            )
            .unwrap();
            let state = *WidgetrySliderHeadless::state(app.world(), root).unwrap();
            let local = Vec2::new(angle.cos() as f32, angle.sin() as f32) * RADIUS;
            let (initial, mut drag) = ArcDrag::begin(local, &state).unwrap();
            assert_eq!(initial, f64::from(value));
            let raw = f64::from(local.y.atan2(local.x));
            assert!((drag.advance(raw, &state) - f64::from(value)).abs() < comparison_tolerance);
            let inward = if value == 0.0 { 0.02 } else { -0.02 };
            let next = drag.advance(raw + inward, &state);
            assert!(
                (next - (f64::from(value) + inward / SPAN * 100.0)).abs() < comparison_tolerance
            );
        }
        let root = spawn_scene(app.world_mut(), bsn! {@WidgetrySliderHeadless}).unwrap();
        let state = *WidgetrySliderHeadless::state(app.world(), root).unwrap();
        let gap = Vec2::new(0.0, 1.0) * RADIUS;
        assert!(ArcDrag::begin(gap, &state).is_none());
    }
}
