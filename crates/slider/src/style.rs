use crate::{
    SliderError, SliderFailure,
    error::{StyleKey, StyleProblem, failure},
};
use crate::{
    WidgetrySlider, WidgetrySliderOrientation, WidgetrySliderProps, colors::ColorState,
    input::DragState, value::SliderState,
};
use bevy::{
    ecs::entity::EntityHashSet,
    input_focus::{InputFocus, InputFocusVisible},
    picking::hover::Hovered,
    prelude::*,
    ui::{InteractionDisabled, Pressed, UiTransform, Val2},
};
use bevy_widgetry_core::ui::WidgetryUiSystems;
use bevy_widgetry_diagnostics::{
    ErrorFrame,
    bevy::{DiagnosticBatch, FailureState},
};
use bevy_widgetry_theme::WidgetryThemeMode;
use std::sync::Arc;

#[derive(Component, FromTemplate)]
pub(crate) struct Parts {
    pub(crate) track: Entity,
    fill: Entity,
    pub(crate) thumb: Entity,
}

#[derive(Component, Default)]
struct StyleDiagnostics(FailureState<StyleKey>);

fn initial(props: &WidgetrySliderProps) -> Result<SliderState, SliderFailure> {
    for (field, size) in [
        ("track_thickness", props.track_thickness),
        ("thumb_diameter", props.thumb_diameter),
    ] {
        if !size.is_finite() || size <= 0.0 {
            return Err(failure(
                SliderError::InvalidDimensions {
                    track_thickness: props.track_thickness,
                    thumb_diameter: props.thumb_diameter,
                    field,
                },
                ErrorFrame::new("slider.initialize_dimensions"),
            ));
        }
    }
    SliderState::new(props)
}

fn ratio(state: &SliderState) -> f32 {
    ((f64::from(state.value) - f64::from(state.min))
        / (f64::from(state.max) - f64::from(state.min))) as f32
}

pub(crate) fn scene(props: WidgetrySliderProps) -> impl Scene {
    let vertical = props.orientation == WidgetrySliderOrientation::Vertical;
    let thickness = props.track_thickness;
    let diameter = props.thumb_diameter;
    let cross = 28.0_f32.max(diameter).max(thickness);
    let initial_ratio = ((f64::from(props.value.unwrap_or(props.min)) - f64::from(props.min))
        / (f64::from(props.max) - f64::from(props.min))) as f32;
    let colors = props.colors.clone();
    let palette = props
        .colors
        .resolve(&WidgetryThemeMode::default().colors().slider)
        .normal;
    bsn! {
        template(move |context| initial(&props).map_err(|error| error.with_context(ErrorFrame::new("slider.initialize").field("root", context.entity.id())).into_bevy_error()))
        template(move |context| colors.clone().initial(context.entity.id()).map_err(|error| error.into_bevy_error()))
        template(|_| Ok(DragState::default()))
        template(|_| Ok(StyleDiagnostics::default()))
        Parts { track: #SliderTrack, fill: #SliderFill, thumb: #SliderThumb }
        bevy::input_focus::tab_navigation::TabIndex(0) Hovered(false)
        Node {
            width: px(if vertical {cross} else {200.0}), height: px(if vertical {200.0} else {cross}),
            min_width: px(if vertical {cross} else {0.0}), min_height: px(if vertical {0.0} else {cross}),
            flex_shrink: 0.0, align_items: AlignItems::Center, justify_content: JustifyContent::Center,
            padding: {if vertical {UiRect::vertical(px(diameter*0.5))} else {UiRect::horizontal(px(diameter*0.5))}},
        }
        Children [
            #SliderTrack Pickable::IGNORE
            Node { width: {if vertical {px(thickness)} else {percent(100)}}, height: {if vertical {percent(100)} else {px(thickness)}}, flex_shrink: 0.0, border_radius: BorderRadius::MAX }
            BackgroundColor({palette.track})
            Children [
                #SliderFill Pickable::IGNORE
                Node {
                    position_type: PositionType::Absolute, left: px(0), bottom: px(0),
                    width: {if vertical {percent(100)} else {percent(initial_ratio*100.0)}},
                    height: {if vertical {percent(initial_ratio*100.0)} else {percent(100)}},
                    border_radius: BorderRadius::MAX,
                }
                BackgroundColor({palette.fill})--
                #SliderThumb Pickable::IGNORE
                Node {
                    position_type: PositionType::Absolute, width: px(diameter), height: px(diameter),
                    left: {if vertical {percent(50)} else {percent(initial_ratio*100.0)}},
                    top: {if vertical {percent((1.0-initial_ratio)*100.0)} else {percent(50)}},
                    border_radius: BorderRadius::MAX,
                }
                UiTransform { translation: {Val2::percent(-50,-50)} }
                BackgroundColor({palette.thumb})
                Outline { width: px(2), offset: px(3), color: Color::NONE }
            ]
        ]
    }
}

pub(crate) fn install(app: &mut App) {
    app.register_type::<SliderState>()
        .register_type::<WidgetrySliderOrientation>()
        .add_systems(PostUpdate, update.in_set(WidgetryUiSystems::Colors));
}

fn update(
    mode: Res<WidgetryThemeMode>,
    focus: Res<InputFocus>,
    focus_visible: Res<InputFocusVisible>,
    changed: Query<
        Entity,
        (
            With<WidgetrySlider>,
            Or<(
                Changed<SliderState>,
                Changed<ColorState>,
                Changed<Hovered>,
                Changed<DragState>,
                Changed<Pressed>,
                Changed<InteractionDisabled>,
            )>,
        ),
    >,
    mut removed_disabled: RemovedComponents<InteractionDisabled>,
    mut removed_pressed: RemovedComponents<Pressed>,
    mut removed_hovered: RemovedComponents<Hovered>,
    mut roots: Query<
        (
            Entity,
            &SliderState,
            &Parts,
            &ColorState,
            Option<&Hovered>,
            Has<Pressed>,
            Has<InteractionDisabled>,
            &mut StyleDiagnostics,
        ),
        With<WidgetrySlider>,
    >,
    mut nodes: Query<(&mut Node, &mut BackgroundColor, Option<&mut Outline>)>,
) -> Result {
    let mut dirty: EntityHashSet =
        if mode.is_changed() || focus.is_changed() || focus_visible.is_changed() {
            roots.iter().map(|row| row.0).collect()
        } else {
            changed.iter().collect()
        };
    dirty.extend(removed_disabled.read());
    dirty.extend(removed_pressed.read());
    dirty.extend(removed_hovered.read());
    let mut failures = DiagnosticBatch::default();
    for root in dirty {
        let Ok((_, state, parts, local, hovered, pressed, disabled, mut diagnostics)) =
            roots.get_mut(root)
        else {
            continue;
        };
        let entities = [parts.track, parts.fill, parts.thumb];
        let result = (|| -> Result<(), StyleProblem> {
            let colors = local.0.resolve(&mode.colors().slider);
            let color = if disabled {
                colors.disabled
            } else if pressed {
                colors.pressed
            } else if hovered.is_some_and(|hovered| hovered.0) {
                colors.hovered
            } else {
                colors.normal
            };
            let [
                (_, mut track, _),
                (mut fill_node, mut fill, _),
                (mut thumb_node, mut thumb, outline),
            ] = nodes.get_many_mut(entities).map_err(StyleProblem::Query)?;
            track.set_if_neq(BackgroundColor(color.track));
            fill.set_if_neq(BackgroundColor(color.fill));
            thumb.set_if_neq(BackgroundColor(color.thumb));
            let percentage = ratio(state) * 100.0;
            let vertical = state.orientation == WidgetrySliderOrientation::Vertical;
            let fill_length = if vertical {
                fill_node.height
            } else {
                fill_node.width
            };
            if fill_length != percent(percentage) {
                if vertical {
                    fill_node.height = percent(percentage);
                } else {
                    fill_node.width = percent(percentage);
                }
            }
            let thumb_position = if vertical {
                thumb_node.top
            } else {
                thumb_node.left
            };
            let position = percent(if vertical {
                100.0 - percentage
            } else {
                percentage
            });
            if thumb_position != position {
                if vertical {
                    thumb_node.top = position;
                } else {
                    thumb_node.left = position;
                }
            }
            let mut outline = outline.ok_or(StyleProblem::Outline(parts.thumb))?;
            let next = if focus.get() == Some(root) && focus_visible.0 && !disabled {
                color.focus_ring
            } else {
                Color::NONE
            };
            if outline.color != next {
                outline.color = next;
            }
            Ok(())
        })();
        let frame = || ErrorFrame::new("slider.style").field("root", root);
        match result {
            Ok(()) => {
                diagnostics.0.recover(frame);
            }
            Err(problem) => {
                let key = problem.key(entities);
                let error = diagnostics.0.failure(key, || {
                    Arc::new(problem.into_failure(root, entities, frame()))
                });
                for diagnostic in error.shared_diagnostics() {
                    failures.push_shared(diagnostic);
                }
            }
        }
    }
    failures.into_bevy_error().map_or(Ok(()), Err)
}
