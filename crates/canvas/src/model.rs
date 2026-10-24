use bevy::prelude::*;
use bevy_widgetry_internal_log::widgetry_error;

#[derive(Component, Default, Reflect)]
#[reflect(Component)]
pub struct CanvasDocument;

#[derive(Component, Clone, Copy, Debug, PartialEq, Reflect)]
#[reflect(Component)]
pub struct CanvasRectangle {
    pub size: Vec2,
}

#[derive(Clone, Copy, Debug, PartialEq, Reflect)]
pub struct CanvasStroke {
    pub color: Color,
    pub width: f32,
}

#[derive(Component, Clone, Copy, Debug, PartialEq, Reflect)]
#[reflect(Component)]
pub struct CanvasPaint {
    pub fill: Option<Color>,
    pub stroke: Option<CanvasStroke>,
}

impl Default for CanvasPaint {
    fn default() -> Self {
        Self {
            fill: Some(Color::WHITE),
            stroke: None,
        }
    }
}

impl CanvasDocument {
    pub fn create_rectangle(
        world: &mut World,
        document: Entity,
        rectangle: CanvasRectangle,
        transform: Transform,
        paint: CanvasPaint,
    ) -> Result<Entity> {
        let result = (|| {
            validate_document(world, document)?;
            rectangle.validate(&transform, &paint)
        })();
        result.inspect_err(|error| widgetry_error!(?document, %error, "Canvas 矩形创建被拒绝"))?;
        Ok(world
            .spawn((rectangle, transform, paint, ChildOf(document)))
            .id())
    }

    pub fn update_rectangle(
        world: &mut World,
        document: Entity,
        shape: Entity,
        rectangle: CanvasRectangle,
        transform: Transform,
        paint: CanvasPaint,
    ) -> Result {
        let result = (|| {
            validate_shape(world, document, shape)?;
            rectangle.validate(&transform, &paint)
        })();
        result.inspect_err(
            |error| widgetry_error!(?document, ?shape, %error, "Canvas 矩形更新被拒绝"),
        )?;
        world
            .entity_mut(shape)
            .insert((rectangle, transform, paint));
        Ok(())
    }

    pub fn remove_rectangle(world: &mut World, document: Entity, shape: Entity) -> Result {
        validate_shape(world, document, shape).inspect_err(
            |error| widgetry_error!(?document, ?shape, %error, "Canvas 矩形删除被拒绝"),
        )?;
        world.despawn(shape);
        Ok(())
    }
}

fn validate_document(world: &World, document: Entity) -> Result {
    if world.get::<CanvasDocument>(document).is_none() {
        return Err(BevyError::error("CanvasDocument missing"));
    }
    Ok(())
}

fn validate_shape(world: &World, document: Entity, shape: Entity) -> Result {
    validate_document(world, document)?;
    if world.get::<CanvasRectangle>(shape).is_none()
        || world.get::<Transform>(shape).is_none()
        || world.get::<CanvasPaint>(shape).is_none()
        || world
            .get::<ChildOf>(shape)
            .is_none_or(|parent| parent.parent() != document)
    {
        return Err(BevyError::error(
            "Canvas rectangle does not belong to this document",
        ));
    }
    Ok(())
}

impl CanvasRectangle {
    pub fn validate(&self, transform: &Transform, paint: &CanvasPaint) -> Result {
        if !self.size.is_finite() || self.size.min_element() < 1.0 {
            return Err(BevyError::error(
                "Canvas rectangle size must be finite and at least one unit per axis",
            ));
        }
        if !transform.translation.is_finite()
            || !transform.scale.is_finite()
            || transform.scale.x == 0.0
            || transform.scale.y == 0.0
            || !transform.rotation.is_finite()
            || !transform.rotation.is_normalized()
            || transform.rotation.x != 0.0
            || transform.rotation.y != 0.0
        {
            return Err(BevyError::error(
                "Canvas transform must be finite, invertible in XY and rotate about Z",
            ));
        }
        if paint.fill.is_some_and(|color| !finite_color(color))
            || paint.stroke.is_some_and(|stroke| {
                !finite_color(stroke.color) || !stroke.width.is_finite() || stroke.width <= 0.0
            })
        {
            return Err(BevyError::error(
                "Canvas paint requires finite colors and a positive finite stroke width",
            ));
        }
        let stroke = paint.stroke.map_or(0.0, |stroke| stroke.width);
        let outer = CanvasRectangle {
            size: self.size + Vec2::splat(stroke),
        };
        let corners = outer.corners(transform);
        let min = corners
            .into_iter()
            .fold(Vec2::splat(f32::INFINITY), Vec2::min);
        let max = corners
            .into_iter()
            .fold(Vec2::splat(f32::NEG_INFINITY), Vec2::max);
        if !outer.size.is_finite()
            || corners.iter().any(|point| !point.is_finite())
            || !(max - min).is_finite()
        {
            return Err(BevyError::error(
                "Canvas transformed rectangle bounds overflow",
            ));
        }
        Ok(())
    }

    pub fn corners(&self, transform: &Transform) -> [Vec2; 4] {
        let half = self.size * 0.5;
        [
            Vec2::new(-half.x, -half.y),
            Vec2::new(half.x, -half.y),
            half,
            Vec2::new(-half.x, half.y),
        ]
        .map(|point| transform.transform_point(point.extend(0.0)).truncate())
    }

    pub fn bounds(&self, transform: &Transform, paint: &CanvasPaint) -> Result<Rect> {
        self.validate(transform, paint)?;
        let stroke = paint.stroke.map_or(0.0, |stroke| stroke.width);
        let outer = CanvasRectangle {
            size: self.size + Vec2::splat(stroke),
        };
        let corners = outer.corners(transform);
        let min = corners
            .into_iter()
            .fold(Vec2::splat(f32::INFINITY), Vec2::min);
        let max = corners
            .into_iter()
            .fold(Vec2::splat(f32::NEG_INFINITY), Vec2::max);
        Ok(Rect::from_corners(min, max))
    }
}

fn finite_color(color: Color) -> bool {
    color
        .to_linear()
        .to_f32_array()
        .into_iter()
        .all(f32::is_finite)
}
