use crate::error::{CanvasResult, PublicResult, invalid};
use crate::{CanvasDocument, CanvasEditTarget, CanvasIssue, CanvasPaint};
use bevy::prelude::*;
use bevy_widgetry_diagnostics::ErrorFrame;

/// A color at a normalized position within a gradient.
#[derive(Clone, Copy, Debug, PartialEq, Reflect)]
pub struct CanvasColorStop {
    pub offset: f32,
    pub color: Color,
}

/// Two to eight color stops in offset order, with ties in input order.
#[derive(Clone, Copy, Debug, PartialEq, Reflect)]
pub struct CanvasGradientStops {
    count: usize,
    stops: [CanvasColorStop; Self::MAX],
}

impl CanvasGradientStops {
    pub const MIN: usize = 2;
    pub const MAX: usize = 8;

    pub fn new(stops: &[CanvasColorStop]) -> PublicResult<Self> {
        let empty = CanvasColorStop {
            offset: 0.0,
            color: Color::NONE,
        };
        let mut result = Self {
            count: stops.len(),
            stops: [empty; Self::MAX],
        };
        let accepted = result
            .stops
            .get_mut(..stops.len())
            .filter(|_| Self::valid_entries(stops));
        let Some(accepted) = accepted else {
            return Err(invalid(CanvasIssue::GradientStops)
                .into_failure(ErrorFrame::new("canvas.CanvasGradientStops.new")));
        };
        accepted.copy_from_slice(stops);
        accepted.sort_by(|left, right| {
            left.offset
                .partial_cmp(&right.offset)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        Ok(result)
    }

    pub fn as_slice(&self) -> &[CanvasColorStop] {
        self.stops.get(..self.count).unwrap_or_default()
    }

    fn valid_entries(stops: &[CanvasColorStop]) -> bool {
        (Self::MIN..=Self::MAX).contains(&stops.len())
            && stops.iter().all(|stop| {
                (0.0..=1.0).contains(&stop.offset) && crate::model::finite_color(stop.color)
            })
    }

    fn validate(self) -> CanvasResult {
        let stops = self.as_slice();
        if !Self::valid_entries(stops)
            || stops.windows(2).any(|pair| pair[0].offset > pair[1].offset)
        {
            return Err(invalid(CanvasIssue::GradientStops));
        }
        Ok(())
    }
}

/// The fill of a Canvas shape. Gradient coordinates are local to its geometry.
#[derive(Clone, Copy, Debug, PartialEq, Reflect)]
pub enum CanvasFill {
    Solid(Color),
    LinearGradient {
        start: Vec2,
        end: Vec2,
        stops: CanvasGradientStops,
    },
    RadialGradient {
        center: Vec2,
        radius: f32,
        stops: CanvasGradientStops,
    },
}

impl From<Color> for CanvasFill {
    fn from(color: Color) -> Self {
        Self::Solid(color)
    }
}

impl CanvasFill {
    pub(crate) fn validate(self) -> CanvasResult {
        let validate_length = |value: f64| {
            if (f64::from(f32::MIN_POSITIVE)..=f64::from(f32::MAX)).contains(&value) {
                Ok(())
            } else {
                Err(invalid(CanvasIssue::GradientGeometry))
            }
        };
        match self {
            Self::Solid(color) if !crate::model::finite_color(color) => {
                Err(invalid(CanvasIssue::Paint))
            }
            Self::Solid(_) => Ok(()),
            Self::LinearGradient { start, end, stops } => {
                validate_length((end.as_dvec2() - start.as_dvec2()).length())?;
                stops.validate()
            }
            Self::RadialGradient {
                center,
                radius,
                stops,
            } => {
                if !center.is_finite() || !(center + Vec2::X * radius).is_finite() {
                    return Err(invalid(CanvasIssue::GradientGeometry));
                }
                validate_length(f64::from(radius))?;
                stops.validate()
            }
        }
    }

    pub(crate) fn first_color(self) -> Color {
        match self {
            Self::Solid(color) => color,
            Self::LinearGradient { stops, .. } | Self::RadialGradient { stops, .. } => stops
                .as_slice()
                .first()
                .map_or(Color::NONE, |stop| stop.color),
        }
    }

    pub(crate) fn visible(self) -> bool {
        match self {
            Self::Solid(color) => color.alpha() > 0.0,
            Self::LinearGradient { stops, .. } | Self::RadialGradient { stops, .. } => {
                stops.as_slice().iter().any(|stop| stop.color.alpha() > 0.0)
            }
        }
    }
}

impl CanvasDocument {
    /// Replaces a shape's paint and records the change in the current document edit.
    pub fn update_paint(
        world: &mut World,
        edit: impl Into<CanvasEditTarget>,
        shape: Entity,
        paint: CanvasPaint,
    ) -> PublicResult {
        let edit = edit.into();
        crate::history::apply(world, edit, |world, document| {
            Self::_update_paint(world, document, shape, paint)
        })
        .map_err(|error| {
            error
                .at("document", edit.document())
                .at("shape", shape)
                .into_failure(ErrorFrame::new("canvas.CanvasDocument.update_paint"))
        })
    }

    pub(crate) fn _update_paint(
        world: &mut World,
        document: Entity,
        shape: Entity,
        paint: CanvasPaint,
    ) -> CanvasResult {
        crate::model::validate_shape(world, document, shape)?;
        let geometry = world
            .get::<crate::CanvasGeometry>(shape)
            .ok_or_else(|| crate::error::missing(crate::CanvasPart::Geometry))?;
        let transform = crate::groups::local_transform(world, shape)?;
        crate::groups::validate_shape_projection(world, shape, geometry, transform, &paint)?;
        crate::history::set_paint(world, document, shape, paint)
    }
}
