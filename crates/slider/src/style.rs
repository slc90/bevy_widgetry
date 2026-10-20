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
use bevy_widgetry_core::{diagnostics::FailureState, ui::WidgetryUiSystems};
use bevy_widgetry_internal_log::{widgetry_error, widgetry_info};
use bevy_widgetry_theme::WidgetryThemeMode;

#[derive(Component, FromTemplate)]
pub(crate) struct Parts {
    pub(crate) track: Entity,
    fill: Entity,
    pub(crate) thumb: Entity,
}

#[derive(Component, Default)]
struct StyleDiagnostics(FailureState);

fn initial(props: &WidgetrySliderProps) -> Result<SliderState, BevyError> {
    for size in [props.track_thickness, props.thumb_diameter] {
        if !size.is_finite() || size <= 0.0 {
            widgetry_error!(size, "Slider 尺寸必须为有限正数");
            return Err(bevy_widgetry_core::scene::logged_error(
                "Slider dimensions must be finite and positive",
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
        template(move |_| initial(&props))
        template(move |_| colors.clone().initial())
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
    let mut failure = None;
    for root in dirty {
        let Ok((_, state, parts, local, hovered, pressed, disabled, mut diagnostics)) =
            roots.get_mut(root)
        else {
            continue;
        };
        let result = (|| -> Result {
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
            ] = nodes
                .get_many_mut([parts.track, parts.fill, parts.thumb])
                .map_err(|_| BevyError::error("Slider visual subtree is missing"))?;
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
            let mut outline =
                outline.ok_or_else(|| BevyError::error("Slider thumb focus outline is missing"))?;
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
        let result = diagnostics.0.observe(
            result,
            |error| widgetry_error!(?root,%error,"Slider 样式内部结构失效"),
            || widgetry_info!(?root, "Slider 样式内部结构恢复正常"),
        );
        if let Err(error) = result
            && failure.is_none()
        {
            failure = Some(error);
        }
    }
    failure.map_or(Ok(()), Err)
}
