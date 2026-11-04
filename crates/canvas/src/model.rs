use crate::error::CanvasResult as Result;
use bevy::prelude::*;

#[derive(Component, Default, Reflect)]
#[reflect(Component)]
#[require(Transform, crate::order::ShapeOrder)]
pub struct CanvasDocument;

#[derive(Component, Clone, Copy, Debug, Default, Reflect)]
#[reflect(Component)]
pub struct CanvasShape;

#[derive(Component, Clone, Debug, PartialEq, Reflect)]
#[reflect(Component)]
#[require(CanvasShape)]
pub enum CanvasGeometry {
    Rectangle(CanvasRectangle),
    Ellipse(CanvasEllipse),
    Path(crate::CanvasPath),
}

impl CanvasGeometry {
    pub fn rectangle(&self) -> Option<&CanvasRectangle> {
        match self {
            Self::Rectangle(rectangle) => Some(rectangle),
            Self::Ellipse(_) | Self::Path(_) => None,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Reflect)]
pub struct CanvasEllipse {
    pub radii: Vec2,
}

impl Default for CanvasEllipse {
    fn default() -> Self {
        Self {
            radii: Vec2::splat(1.0),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Reflect)]
pub struct CanvasRectangle {
    pub size: Vec2,
    pub corner_radius: f32,
}

#[derive(Clone, Copy, Debug, PartialEq, Reflect)]
pub struct CanvasStroke {
    pub color: Color,
    pub width: f32,
    pub cap: CanvasLineCap,
    pub join: CanvasLineJoin,
}

impl Default for CanvasStroke {
    fn default() -> Self {
        Self {
            color: Color::BLACK,
            width: 1.0,
            cap: CanvasLineCap::Butt,
            join: CanvasLineJoin::Miter,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Reflect)]
pub enum CanvasFillRule {
    #[default]
    NonZero,
    EvenOdd,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Reflect)]
pub enum CanvasLineCap {
    #[default]
    Butt,
    Round,
    Square,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Reflect)]
pub enum CanvasLineJoin {
    #[default]
    Miter,
    Round,
    Bevel,
}

#[derive(Component, Clone, Copy, Debug, PartialEq, Reflect)]
#[reflect(Component)]
pub struct CanvasPaint {
    pub fill: Option<Color>,
    pub fill_rule: CanvasFillRule,
    pub stroke: Option<CanvasStroke>,
}

impl Default for CanvasPaint {
    fn default() -> Self {
        Self {
            fill: Some(Color::WHITE),
            fill_rule: CanvasFillRule::NonZero,
            stroke: None,
        }
    }
}

impl CanvasDocument {
    pub fn update_transform(
        world: &mut World,
        document: Entity,
        shape: Entity,
        transform: Transform,
    ) -> crate::error::PublicResult {
        Self::_update_transform(world, document, shape, transform).map_err(|error| {
            error.into_failure(bevy_widgetry_diagnostics::ErrorFrame::new(
                "canvas.CanvasDocument.update_transform",
            ))
        })
    }

    pub(crate) fn _update_transform(
        world: &mut World,
        document: Entity,
        shape: Entity,
        transform: Transform,
    ) -> Result {
        (|| -> Result {
            let result = (|| {
                validate_shape(world, document, shape)?;
                let geometry = world
                    .get::<CanvasGeometry>(shape)
                    .ok_or_else(|| crate::error::missing(crate::CanvasPart::Geometry))?;
                let paint = world
                    .get::<CanvasPaint>(shape)
                    .ok_or_else(|| crate::error::missing(crate::CanvasPart::Paint))?;
                geometry._validate(&transform, paint)
            })();
            result?;
            world.entity_mut(shape).insert(transform);
            Ok(())
        })()
        .map_err(|error| {
            error
                .input(crate::CanvasInput::Entity {
                    field: "document",
                    value: document,
                })
                .input(crate::CanvasInput::Entity {
                    field: "shape",
                    value: shape,
                })
                .input(crate::CanvasInput::Transform(transform))
        })
    }

    pub fn create_rectangle(
        world: &mut World,
        document: Entity,
        rectangle: CanvasRectangle,
        transform: Transform,
        paint: CanvasPaint,
    ) -> crate::error::PublicResult<Entity> {
        Self::_create_rectangle(world, document, rectangle, transform, paint).map_err(|error| {
            error.into_failure(bevy_widgetry_diagnostics::ErrorFrame::new(
                "canvas.CanvasDocument.create_rectangle",
            ))
        })
    }

    pub(crate) fn _create_rectangle(
        world: &mut World,
        document: Entity,
        rectangle: CanvasRectangle,
        transform: Transform,
        paint: CanvasPaint,
    ) -> Result<Entity> {
        Self::create_shape(
            world,
            document,
            CanvasGeometry::Rectangle(rectangle),
            transform,
            paint,
        )
        .map_err(|error| {
            error
                .input(crate::CanvasInput::Entity {
                    field: "document",
                    value: document,
                })
                .input(crate::CanvasInput::Rectangle(rectangle))
                .input(crate::CanvasInput::Transform(transform))
                .input(crate::CanvasInput::Paint(paint))
        })
    }

    pub fn create_ellipse(
        world: &mut World,
        document: Entity,
        ellipse: CanvasEllipse,
        transform: Transform,
        paint: CanvasPaint,
    ) -> crate::error::PublicResult<Entity> {
        Self::_create_ellipse(world, document, ellipse, transform, paint).map_err(|error| {
            error.into_failure(bevy_widgetry_diagnostics::ErrorFrame::new(
                "canvas.CanvasDocument.create_ellipse",
            ))
        })
    }

    pub(crate) fn _create_ellipse(
        world: &mut World,
        document: Entity,
        ellipse: CanvasEllipse,
        transform: Transform,
        paint: CanvasPaint,
    ) -> Result<Entity> {
        Self::create_shape(
            world,
            document,
            CanvasGeometry::Ellipse(ellipse),
            transform,
            paint,
        )
        .map_err(|error| {
            error
                .input(crate::CanvasInput::Entity {
                    field: "document",
                    value: document,
                })
                .input(crate::CanvasInput::Ellipse(ellipse))
                .input(crate::CanvasInput::Transform(transform))
                .input(crate::CanvasInput::Paint(paint))
        })
    }

    pub fn create_path(
        world: &mut World,
        document: Entity,
        path: crate::CanvasPath,
        transform: Transform,
        paint: CanvasPaint,
    ) -> crate::error::PublicResult<Entity> {
        Self::_create_path(world, document, path, transform, paint).map_err(|error| {
            error.into_failure(bevy_widgetry_diagnostics::ErrorFrame::new(
                "canvas.CanvasDocument.create_path",
            ))
        })
    }

    pub(crate) fn _create_path(
        world: &mut World,
        document: Entity,
        path: crate::CanvasPath,
        transform: Transform,
        paint: CanvasPaint,
    ) -> Result<Entity> {
        Self::create_shape(
            world,
            document,
            CanvasGeometry::Path(path),
            transform,
            paint,
        )
        .map_err(|error| {
            error
                .input(crate::CanvasInput::Entity {
                    field: "document",
                    value: document,
                })
                .input(crate::CanvasInput::Transform(transform))
                .input(crate::CanvasInput::Paint(paint))
        })
    }

    pub fn update_path(
        world: &mut World,
        document: Entity,
        shape: Entity,
        path: crate::CanvasPath,
        transform: Transform,
        paint: CanvasPaint,
    ) -> crate::error::PublicResult {
        Self::_update_path(world, document, shape, path, transform, paint).map_err(|error| {
            error.into_failure(bevy_widgetry_diagnostics::ErrorFrame::new(
                "canvas.CanvasDocument.update_path",
            ))
        })
    }

    pub(crate) fn _update_path(
        world: &mut World,
        document: Entity,
        shape: Entity,
        path: crate::CanvasPath,
        transform: Transform,
        paint: CanvasPaint,
    ) -> Result {
        Self::update_shape(
            world,
            document,
            shape,
            CanvasGeometry::Path(path),
            transform,
            paint,
        )
        .map_err(|error| {
            error
                .input(crate::CanvasInput::Entity {
                    field: "document",
                    value: document,
                })
                .input(crate::CanvasInput::Entity {
                    field: "shape",
                    value: shape,
                })
                .input(crate::CanvasInput::Transform(transform))
                .input(crate::CanvasInput::Paint(paint))
        })
    }

    fn create_shape(
        world: &mut World,
        document: Entity,
        geometry: CanvasGeometry,
        transform: Transform,
        paint: CanvasPaint,
    ) -> Result<Entity> {
        (|| -> Result<Entity> {
            let result = (|| {
                validate_document(world, document)?;
                geometry._validate(&transform, &paint)
            })();
            result?;
            let shape = world
                .spawn((geometry, transform, paint, ChildOf(document)))
                .id();
            world
                .get_mut::<crate::order::ShapeOrder>(document)
                .ok_or_else(|| crate::error::missing(crate::CanvasPart::DocumentOrder))?
                .0
                .push(shape);
            Ok(shape)
        })()
        .map_err(|error| error.at("document", document))
    }

    pub fn update_rectangle(
        world: &mut World,
        document: Entity,
        shape: Entity,
        rectangle: CanvasRectangle,
        transform: Transform,
        paint: CanvasPaint,
    ) -> crate::error::PublicResult {
        Self::_update_rectangle(world, document, shape, rectangle, transform, paint).map_err(
            |error| {
                error.into_failure(bevy_widgetry_diagnostics::ErrorFrame::new(
                    "canvas.CanvasDocument.update_rectangle",
                ))
            },
        )
    }

    pub(crate) fn _update_rectangle(
        world: &mut World,
        document: Entity,
        shape: Entity,
        rectangle: CanvasRectangle,
        transform: Transform,
        paint: CanvasPaint,
    ) -> Result {
        Self::update_shape(
            world,
            document,
            shape,
            CanvasGeometry::Rectangle(rectangle),
            transform,
            paint,
        )
        .map_err(|error| {
            error
                .input(crate::CanvasInput::Entity {
                    field: "document",
                    value: document,
                })
                .input(crate::CanvasInput::Entity {
                    field: "shape",
                    value: shape,
                })
                .input(crate::CanvasInput::Rectangle(rectangle))
                .input(crate::CanvasInput::Transform(transform))
                .input(crate::CanvasInput::Paint(paint))
        })
    }

    pub fn update_ellipse(
        world: &mut World,
        document: Entity,
        shape: Entity,
        ellipse: CanvasEllipse,
        transform: Transform,
        paint: CanvasPaint,
    ) -> crate::error::PublicResult {
        Self::_update_ellipse(world, document, shape, ellipse, transform, paint).map_err(|error| {
            error.into_failure(bevy_widgetry_diagnostics::ErrorFrame::new(
                "canvas.CanvasDocument.update_ellipse",
            ))
        })
    }

    pub(crate) fn _update_ellipse(
        world: &mut World,
        document: Entity,
        shape: Entity,
        ellipse: CanvasEllipse,
        transform: Transform,
        paint: CanvasPaint,
    ) -> Result {
        Self::update_shape(
            world,
            document,
            shape,
            CanvasGeometry::Ellipse(ellipse),
            transform,
            paint,
        )
        .map_err(|error| {
            error
                .input(crate::CanvasInput::Entity {
                    field: "document",
                    value: document,
                })
                .input(crate::CanvasInput::Entity {
                    field: "shape",
                    value: shape,
                })
                .input(crate::CanvasInput::Ellipse(ellipse))
                .input(crate::CanvasInput::Transform(transform))
                .input(crate::CanvasInput::Paint(paint))
        })
    }

    fn update_shape(
        world: &mut World,
        document: Entity,
        shape: Entity,
        geometry: CanvasGeometry,
        transform: Transform,
        paint: CanvasPaint,
    ) -> Result {
        (|| -> Result {
            let result = (|| {
                validate_shape(world, document, shape)?;
                let old = world
                    .get::<CanvasGeometry>(shape)
                    .ok_or_else(|| crate::error::missing(crate::CanvasPart::Geometry))?;
                if std::mem::discriminant(old) != std::mem::discriminant(&geometry) {
                    return Err(crate::error::invalid(crate::CanvasIssue::GeometryKind));
                }
                geometry._validate(&transform, &paint)
            })();
            result?;
            world.entity_mut(shape).insert((geometry, transform, paint));
            Ok(())
        })()
        .map_err(|error| error.at("document", document).at("shape", shape))
    }

    pub fn remove_shape(
        world: &mut World,
        document: Entity,
        shape: Entity,
    ) -> crate::error::PublicResult {
        Self::_remove_shape(world, document, shape).map_err(|error| {
            error.into_failure(bevy_widgetry_diagnostics::ErrorFrame::new(
                "canvas.CanvasDocument.remove_shape",
            ))
        })
    }

    pub(crate) fn _remove_shape(world: &mut World, document: Entity, shape: Entity) -> Result {
        (|| -> Result {
            validate_shape(world, document, shape)?;
            world
                .get_mut::<crate::order::ShapeOrder>(document)
                .ok_or_else(|| crate::error::missing(crate::CanvasPart::DocumentOrder))?
                .0
                .retain(|entity| *entity != shape);
            world.despawn(shape);
            Ok(())
        })()
        .map_err(|error| {
            error
                .input(crate::CanvasInput::Entity {
                    field: "document",
                    value: document,
                })
                .input(crate::CanvasInput::Entity {
                    field: "shape",
                    value: shape,
                })
        })
    }
}

pub(crate) fn validate_document(world: &World, document: Entity) -> Result {
    (|| -> Result {
        if world.get::<CanvasDocument>(document).is_none()
            || world.get::<crate::order::ShapeOrder>(document).is_none()
        {
            return Err(crate::error::missing(crate::CanvasPart::Document));
        }
        Ok(())
    })()
    .map_err(|error| error.at("document", document))
}

pub(crate) fn validate_shape(world: &World, document: Entity, shape: Entity) -> Result {
    (|| -> Result {
        validate_document(world, document)?;
        if world.get::<CanvasShape>(shape).is_none()
            || world.get::<CanvasGeometry>(shape).is_none()
            || world.get::<Transform>(shape).is_none()
            || world.get::<CanvasPaint>(shape).is_none()
            || world
                .get::<ChildOf>(shape)
                .is_none_or(|parent| parent.parent() != document)
        {
            let mut error = crate::error::invalid(crate::CanvasIssue::Ownership);
            if let Some(parent) = world.get::<ChildOf>(shape) {
                error = error.at("actual_document", parent.parent());
            }
            return Err(error);
        }
        Ok(())
    })()
    .map_err(|error| error.at("document", document).at("shape", shape))
}

impl CanvasRectangle {
    pub fn validate(
        &self,
        transform: &Transform,
        paint: &CanvasPaint,
    ) -> crate::error::PublicResult {
        self._validate(transform, paint).map_err(|error| {
            error.into_failure(bevy_widgetry_diagnostics::ErrorFrame::new(
                "canvas.CanvasRectangle.validate",
            ))
        })
    }

    pub(crate) fn _validate(&self, transform: &Transform, paint: &CanvasPaint) -> Result {
        (|| -> Result {
            if !self.size.is_finite() || self.size.min_element() < 1.0 {
                return Err(crate::error::invalid(crate::CanvasIssue::RectangleSize));
            }
            if !self.corner_radius.is_finite()
                || self.corner_radius < 0.0
                || self.corner_radius > self.size.min_element() * 0.5
            {
                return Err(crate::error::invalid(crate::CanvasIssue::CornerRadius));
            }
            validate_transform_and_paint(transform, paint)?;
            let stroke = paint.stroke.map_or(0.0, |stroke| stroke.width);
            let outer = CanvasRectangle {
                corner_radius: 0.0,
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
                return Err(crate::error::invalid(crate::CanvasIssue::RectangleBounds));
            }
            Ok(())
        })()
        .map_err(|error| {
            error
                .input(crate::CanvasInput::Rectangle(*self))
                .input(crate::CanvasInput::Transform(*transform))
                .input(crate::CanvasInput::Paint(*paint))
        })
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

    pub fn bounds(
        &self,
        transform: &Transform,
        paint: &CanvasPaint,
    ) -> crate::error::PublicResult<Rect> {
        self._bounds(transform, paint).map_err(|error| {
            error.into_failure(bevy_widgetry_diagnostics::ErrorFrame::new(
                "canvas.CanvasRectangle.bounds",
            ))
        })
    }

    pub(crate) fn _bounds(&self, transform: &Transform, paint: &CanvasPaint) -> Result<Rect> {
        (|| -> Result<Rect> {
            self._validate(transform, paint)?;
            if self.corner_radius > 0.0 {
                let x = (transform.rotation * Vec3::X * transform.scale.x)
                    .truncate()
                    .as_dvec2();
                let y = (transform.rotation * Vec3::Y * transform.scale.y)
                    .truncate()
                    .as_dvec2();
                let radius = f64::from(self.corner_radius);
                let straight = self.size.as_dvec2() * 0.5 - bevy::math::DVec2::splat(radius);
                let radius = radius + f64::from(paint.stroke.map_or(0.0, |s| s.width)) * 0.5;
                let extent = bevy::math::DVec2::new(
                    x.x.abs() * straight.x + y.x.abs() * straight.y + radius * x.x.hypot(y.x),
                    x.y.abs() * straight.x + y.y.abs() * straight.y + radius * x.y.hypot(y.y),
                );
                let center = transform.translation.truncate().as_dvec2();
                return Ok(Rect {
                    min: (center - extent).as_vec2(),
                    max: (center + extent).as_vec2(),
                });
            }
            let stroke = paint.stroke.map_or(0.0, |stroke| stroke.width);
            let outer = CanvasRectangle {
                corner_radius: 0.0,
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
        })()
        .map_err(|error| {
            error
                .input(crate::CanvasInput::Rectangle(*self))
                .input(crate::CanvasInput::Transform(*transform))
                .input(crate::CanvasInput::Paint(*paint))
        })
    }
}

fn finite_color(color: Color) -> bool {
    color
        .to_linear()
        .to_f32_array()
        .into_iter()
        .all(f32::is_finite)
}

pub(crate) fn validate_transform_and_paint(transform: &Transform, paint: &CanvasPaint) -> Result {
    if !transform.translation.is_finite()
        || !transform.scale.is_finite()
        || transform.scale.x == 0.0
        || transform.scale.y == 0.0
        || !transform.rotation.is_finite()
        || !transform.rotation.is_normalized()
        || transform.rotation.x != 0.0
        || transform.rotation.y != 0.0
    {
        return Err(crate::error::invalid(crate::CanvasIssue::Transform));
    }
    if paint.fill.is_some_and(|color| !finite_color(color))
        || paint.stroke.is_some_and(|stroke| {
            !finite_color(stroke.color) || !stroke.width.is_finite() || stroke.width <= 0.0
        })
    {
        return Err(crate::error::invalid(crate::CanvasIssue::Paint));
    }
    Ok(())
}

impl CanvasGeometry {
    pub fn validate(
        &self,
        transform: &Transform,
        paint: &CanvasPaint,
    ) -> crate::error::PublicResult {
        self._validate(transform, paint).map_err(|error| {
            error.into_failure(bevy_widgetry_diagnostics::ErrorFrame::new(
                "canvas.CanvasGeometry.validate",
            ))
        })
    }

    pub(crate) fn _validate(&self, transform: &Transform, paint: &CanvasPaint) -> Result {
        (match self {
            Self::Rectangle(rectangle) => rectangle._validate(transform, paint),
            Self::Ellipse(ellipse) => ellipse._validate(transform, paint),
            Self::Path(path) => path._validate(transform, paint),
        })
        .map_err(|error| {
            error
                .input(crate::CanvasInput::Geometry(self.clone()))
                .input(crate::CanvasInput::Transform(*transform))
                .input(crate::CanvasInput::Paint(*paint))
        })
    }

    pub fn bounds(
        &self,
        transform: &Transform,
        paint: &CanvasPaint,
    ) -> crate::error::PublicResult<Rect> {
        self._bounds(transform, paint).map_err(|error| {
            error.into_failure(bevy_widgetry_diagnostics::ErrorFrame::new(
                "canvas.CanvasGeometry.bounds",
            ))
        })
    }

    pub(crate) fn _bounds(&self, transform: &Transform, paint: &CanvasPaint) -> Result<Rect> {
        (match self {
            Self::Rectangle(rectangle) => rectangle._bounds(transform, paint),
            Self::Ellipse(ellipse) => ellipse._bounds(transform, paint),
            Self::Path(path) => path._bounds(transform, paint),
        })
        .map_err(|error| {
            error
                .input(crate::CanvasInput::Geometry(self.clone()))
                .input(crate::CanvasInput::Transform(*transform))
                .input(crate::CanvasInput::Paint(*paint))
        })
    }
}

impl CanvasEllipse {
    pub fn validate(
        &self,
        transform: &Transform,
        paint: &CanvasPaint,
    ) -> crate::error::PublicResult {
        self._validate(transform, paint).map_err(|error| {
            error.into_failure(bevy_widgetry_diagnostics::ErrorFrame::new(
                "canvas.CanvasEllipse.validate",
            ))
        })
    }

    pub(crate) fn _validate(&self, transform: &Transform, paint: &CanvasPaint) -> Result {
        (|| -> Result {
            if !self.radii.is_finite()
                || self.radii.min_element() <= 0.0
                || !(self.radii * 2.0).is_finite()
            {
                return Err(crate::error::invalid(crate::CanvasIssue::EllipseRadii));
            }
            validate_transform_and_paint(transform, paint)?;
            let outer =
                self.radii + Vec2::splat(paint.stroke.map_or(0.0, |stroke| stroke.width * 0.5));
            if !outer.is_finite() || !(outer * 2.0).is_finite() {
                return Err(crate::error::invalid(
                    crate::CanvasIssue::EllipseLocalBounds,
                ));
            }
            let bounds = self.transformed_bounds(transform, paint);
            if !bounds.min.is_finite() || !bounds.max.is_finite() || !bounds.size().is_finite() {
                return Err(crate::error::invalid(crate::CanvasIssue::EllipseBounds));
            }
            Ok(())
        })()
        .map_err(|error| {
            error
                .input(crate::CanvasInput::Ellipse(*self))
                .input(crate::CanvasInput::Transform(*transform))
                .input(crate::CanvasInput::Paint(*paint))
        })
    }

    pub fn bounds(
        &self,
        transform: &Transform,
        paint: &CanvasPaint,
    ) -> crate::error::PublicResult<Rect> {
        self._bounds(transform, paint).map_err(|error| {
            error.into_failure(bevy_widgetry_diagnostics::ErrorFrame::new(
                "canvas.CanvasEllipse.bounds",
            ))
        })
    }

    pub(crate) fn _bounds(&self, transform: &Transform, paint: &CanvasPaint) -> Result<Rect> {
        (|| -> Result<Rect> {
            self._validate(transform, paint)?;
            Ok(self.transformed_bounds(transform, paint))
        })()
        .map_err(|error| {
            error
                .input(crate::CanvasInput::Ellipse(*self))
                .input(crate::CanvasInput::Transform(*transform))
                .input(crate::CanvasInput::Paint(*paint))
        })
    }

    fn transformed_bounds(&self, transform: &Transform, paint: &CanvasPaint) -> Rect {
        // f64 avoids overflow in squared support lengths for otherwise representable f32 bounds.
        let x = (transform.rotation * Vec3::X * transform.scale.x)
            .truncate()
            .as_dvec2();
        let y = (transform.rotation * Vec3::Y * transform.scale.y)
            .truncate()
            .as_dvec2();
        let radii = self.radii.as_dvec2();
        let half_stroke = f64::from(paint.stroke.map_or(0.0, |stroke| stroke.width)) * 0.5;
        let extent = bevy::math::DVec2::new(
            (x.x * radii.x).hypot(y.x * radii.y) + half_stroke * x.x.hypot(y.x),
            (x.y * radii.x).hypot(y.y * radii.y) + half_stroke * x.y.hypot(y.y),
        );
        let center = transform.translation.truncate().as_dvec2();
        Rect {
            min: (center - extent).as_vec2(),
            max: (center + extent).as_vec2(),
        }
    }
}
