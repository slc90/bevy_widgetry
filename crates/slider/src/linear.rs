use crate::{
    SliderDirection, SliderDragStrategy, SliderFailure, SliderInteractionStart,
    SliderInteractionStrategy, SliderPointerInput, SliderProjection, WidgetrySliderInteraction,
    WidgetrySliderOrientation, WidgetrySliderState,
};
use bevy::{
    input::keyboard::KeyboardInput,
    prelude::*,
    ui::{ComputedUiRenderTargetInfo, UiGlobalTransform},
};

#[derive(Component, FromTemplate, Clone, Copy)]
#[require(WidgetrySliderInteraction = WidgetrySliderInteraction::linear())]
pub struct WidgetrySliderLinear {
    pub track: Entity,
    pub thumb: Entity,
    pub orientation: WidgetrySliderOrientation,
}

pub(crate) struct LinearStrategy;
struct LinearDrag {
    offset: f64,
}

fn position(
    world: &World,
    input: SliderPointerInput,
) -> Result<Option<(f64, Vec2, f32, f32)>, SliderFailure> {
    let binding = world
        .get::<WidgetrySliderLinear>(input.entity)
        .ok_or_else(|| crate::error::missing(input.entity, input.entity, "WidgetrySliderLinear"))?;
    for part in [binding.track, binding.thumb] {
        if world.get::<Node>(part).is_none() {
            return Err(crate::error::missing(input.entity, part, "Node"));
        }
    }
    Ok(project_position(
        world,
        input.entity,
        input.position,
        &input.state,
        binding.orientation,
        binding,
    ))
}

impl SliderInteractionStrategy for LinearStrategy {
    fn begin(
        &self,
        world: &World,
        input: SliderPointerInput,
    ) -> Result<Option<SliderInteractionStart>, SliderFailure> {
        let Some((pointer_position, local, thumb_size, travel)) = position(world, input)? else {
            return Ok(None);
        };
        let binding = world
            .get::<WidgetrySliderLinear>(input.entity)
            .ok_or_else(|| {
                crate::error::missing(input.entity, input.entity, "WidgetrySliderLinear")
            })?;
        let normalized =
            (input.state.value() - input.state.min()) / (input.state.max() - input.state.min());
        let center = (normalized - 0.5) * travel;
        let distance = if binding.orientation == WidgetrySliderOrientation::Vertical {
            Vec2::new(local.x, -local.y - center)
        } else {
            Vec2::new(local.x - center, local.y)
        };
        let on_thumb = distance.length_squared() <= (thumb_size * 0.5).powi(2);
        let offset = if on_thumb {
            pointer_position - f64::from(input.state.value())
        } else {
            0.0
        };
        Ok(Some(SliderInteractionStart {
            position: if on_thumb {
                f64::from(input.state.value())
            } else {
                pointer_position
            },
            drag: Box::new(LinearDrag { offset }),
        }))
    }
    fn key(
        &self,
        world: &World,
        entity: Entity,
        input: &KeyboardInput,
    ) -> Result<Option<SliderDirection>, SliderFailure> {
        let binding = world
            .get::<WidgetrySliderLinear>(entity)
            .ok_or_else(|| crate::error::missing(entity, entity, "WidgetrySliderLinear"))?;
        Ok(match (binding.orientation, input.key_code) {
            (WidgetrySliderOrientation::Horizontal, KeyCode::ArrowRight)
            | (WidgetrySliderOrientation::Vertical, KeyCode::ArrowUp) => {
                Some(SliderDirection::Increase)
            }
            (WidgetrySliderOrientation::Horizontal, KeyCode::ArrowLeft)
            | (WidgetrySliderOrientation::Vertical, KeyCode::ArrowDown) => {
                Some(SliderDirection::Decrease)
            }
            _ => None,
        })
    }
}
impl SliderDragStrategy for LinearDrag {
    fn project(
        &mut self,
        world: &World,
        input: SliderPointerInput,
    ) -> Result<SliderProjection, SliderFailure> {
        Ok(match position(world, input)? {
            Some((position, _, _, _)) => SliderProjection::Position(position - self.offset),
            None => SliderProjection::Wait,
        })
    }
}

fn project_position(
    world: &World,
    entity: Entity,
    point: Vec2,
    state: &WidgetrySliderState,
    orientation: WidgetrySliderOrientation,
    parts: &WidgetrySliderLinear,
) -> Option<(f64, Vec2, f32, f32)> {
    let node = world.get::<ComputedNode>(parts.track)?;
    let transform = world.get::<UiGlobalTransform>(parts.track)?;
    let target = world.get::<ComputedUiRenderTargetInfo>(entity)?;
    let ui_scale = world.get_resource::<UiScale>().map_or(1.0, |scale| scale.0);
    let camera = world
        .get::<bevy::ui::ComputedUiTargetCamera>(entity)?
        .get()?;
    let viewport_origin = world
        .get::<Camera>(camera)?
        .physical_viewport_rect()?
        .min
        .as_vec2();
    let local = transform
        .try_inverse()?
        .transform_point2(point * target.scale_factor() / ui_scale - viewport_origin);
    let thumb = parts.thumb;
    let thumb = world.get::<ComputedNode>(thumb)?;
    let vertical = orientation == WidgetrySliderOrientation::Vertical;
    let size = if vertical {
        thumb.size().y
    } else {
        thumb.size().x
    };
    let axis = if vertical { -local.y } else { local.x };
    let length = if vertical {
        node.size().y
    } else {
        node.size().x
    };
    let travel = length;
    if !point.is_finite() || !local.is_finite() || !travel.is_finite() || travel <= 0.0 {
        return None;
    }
    let normalized = f64::from(axis / travel + 0.5);
    let value_position = if normalized == 0.0 {
        f64::from(state.min)
    } else if normalized == 1.0 {
        f64::from(state.max)
    } else {
        f64::from(state.min) + normalized * (f64::from(state.max) - f64::from(state.min))
    };
    Some((value_position, local, size, travel))
}
