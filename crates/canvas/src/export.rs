use crate::{CanvasDocument, CanvasGeometry, CanvasPaint, CanvasPathCommand};
use bevy::prelude::*;
use bevy_widgetry_diagnostics::{ErrorFrame, WidgetryFailure};
use bevy_widgetry_job::JobFailure;
use std::{fmt, path::Path};
#[cfg(feature = "png_export")]
mod png;
#[cfg(feature = "png_export")]
pub use png::{CanvasPngExport, CanvasPngExportFinished, CanvasPngOptions, CanvasPngStop};
#[cfg(feature = "png_export")]
pub(crate) fn install(app: &mut App) {
    png::install(app);
}

#[cfg(feature = "png_export")]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CanvasRenderIssue {
    MissingTileResources,
    MissingSceneParts,
    MissingProbe,
    MissingOperation,
    MissingResources,
    MissingExportIdentity,
    Timeout,
    UnexpectedCompletion,
    ReceiverClosed,
    MissingCompletion,
    HandoffPoisoned,
    ReadbackSize { expected: UVec2, actual: UVec2 },
}

#[derive(Debug)]
pub enum CanvasExportError {
    EmptyDocument,
    InvalidDocument,
    Model(bevy_widgetry_job::JobFailure),
    InvalidRegion {
        region: Rect,
    },
    UnsupportedColor {
        shape: Entity,
        color: Color,
    },
    #[cfg(feature = "png_export")]
    InvalidScale,
    #[cfg(feature = "png_export")]
    InvalidResolution,
    #[cfg(feature = "png_export")]
    RendererUnavailable,
    #[cfg(feature = "png_export")]
    Render(CanvasRenderIssue),
    #[cfg(feature = "png_export")]
    Offscreen(bevy_widgetry_core::offscreen::OffscreenFailures),
    #[cfg(feature = "png_export")]
    Cleanup {
        primary: Box<CanvasExportError>,
        cleanup: bevy_widgetry_core::offscreen::OffscreenFailures,
    },
    #[cfg(feature = "png_export")]
    Scene(bevy_widgetry_core::scene::SceneFailure),
    #[cfg(feature = "png_export")]
    Encoding(image::ImageError),
    Io(std::io::Error),
    #[cfg(feature = "png_export")]
    Job(bevy_widgetry_job::SubmitFailure),
    #[cfg(feature = "png_export")]
    Background(bevy_widgetry_job::JobFailure),
}

impl fmt::Display for CanvasExportError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyDocument => f.write_str("Canvas document has no committed shapes"),
            Self::InvalidDocument => f.write_str("CanvasDocument missing"),
            Self::InvalidRegion { region } => {
                write!(
                    f,
                    "Canvas export region must be finite with positive dimensions: {region:?}"
                )
            }
            Self::Io(error) => write!(f, "Canvas export I/O failed: {error}"),
            #[cfg(feature = "png_export")]
            Self::Job(error) => write!(f, "Canvas PNG Job submission failed: {error}"),
            Self::Model(error) => fmt::Display::fmt(error, f),
            #[cfg(feature = "png_export")]
            Self::Background(error) => fmt::Display::fmt(error, f),
            #[cfg(feature = "png_export")]
            Self::InvalidScale => f.write_str("PNG scale must be finite and positive"),
            #[cfg(feature = "png_export")]
            Self::InvalidResolution => {
                f.write_str("PNG resolution exceeds texture, buffer or memory limits")
            }
            #[cfg(feature = "png_export")]
            Self::RendererUnavailable => f.write_str("Canvas PNG renderer unavailable"),
            #[cfg(feature = "png_export")]
            Self::Offscreen(error) => write!(f, "Canvas PNG offscreen failed: {error}"),
            #[cfg(feature = "png_export")]
            Self::Cleanup { primary, cleanup } => write!(f, "{primary}; cleanup: {cleanup}"),
            #[cfg(feature = "png_export")]
            Self::Scene(error) => write!(f, "Canvas PNG scene failed: {error}"),
            #[cfg(feature = "png_export")]
            Self::Render(error) => write!(f, "Canvas PNG rendering failed: {error:?}"),
            #[cfg(feature = "png_export")]
            Self::Encoding(error) => write!(f, "Canvas PNG encoding failed: {error}"),
            Self::UnsupportedColor { shape, color } => {
                write!(
                    f,
                    "SVG requires sRGB colors and alpha within [0, 1]: {shape:?}, {color:?}"
                )
            }
        }
    }
}

impl std::error::Error for CanvasExportError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            #[cfg(feature = "png_export")]
            Self::Offscreen(error) => Some(error),
            #[cfg(feature = "png_export")]
            Self::Cleanup { primary, .. } => Some(primary.as_ref()),
            #[cfg(feature = "png_export")]
            Self::Scene(error) => Some(error.diagnostics()),
            Self::Io(error) => Some(error),
            #[cfg(feature = "png_export")]
            Self::Encoding(error) => Some(error),
            #[cfg(feature = "png_export")]
            Self::Job(error) => Some(error),
            Self::Model(error) => Some(error),
            #[cfg(feature = "png_export")]
            Self::Background(error) => Some(error),
            _ => None,
        }
    }
}

impl CanvasDocument {
    pub fn export_svg(
        world: &mut World,
        document: Entity,
        path: impl AsRef<Path>,
        region: Option<Rect>,
    ) -> std::result::Result<(), JobFailure> {
        let path = path.as_ref();
        let mut actual_region = region;
        let result = (|| {
            let snapshot = snapshot(world, document, region)?;
            actual_region = Some(snapshot.region);
            for shape in &snapshot.shapes {
                for color in shape
                    .paint
                    .fill
                    .into_iter()
                    .chain(shape.paint.stroke.map(|stroke| stroke.color))
                {
                    let c = color.to_srgba();
                    if [c.red, c.green, c.blue, c.alpha]
                        .into_iter()
                        .any(|channel| !(0.0..=1.0).contains(&channel))
                    {
                        return Err(CanvasExportError::UnsupportedColor {
                            shape: shape.entity,
                            color,
                        });
                    }
                }
            }
            let region = snapshot.region;
            let mut svg = format!(
                "<svg xmlns=\"http://www.w3.org/2000/svg\" color-interpolation=\"linearRGB\" width=\"{}\" height=\"{}\" viewBox=\"{} {} {} {}\"><defs><clipPath id=\"canvas-region\"><rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\"/></clipPath></defs><g clip-path=\"url(#canvas-region)\">",
                region.width(),
                region.height(),
                region.min.x,
                region.min.y,
                region.width(),
                region.height(),
                region.min.x,
                region.min.y,
                region.width(),
                region.height()
            );
            let mut paints = Vec::new();
            for shape in &snapshot.shapes {
                if let Some(fill) = shape.paint.fill {
                    let rule = match shape.paint.fill_rule {
                        crate::CanvasFillRule::NonZero => "nonzero",
                        crate::CanvasFillRule::EvenOdd => "evenodd",
                    };
                    paints.push((
                        shape,
                        format!(
                            "{} fill-rule=\"{rule}\" stroke=\"none\"",
                            color("fill", fill)
                        ),
                    ));
                }
                if let Some(stroke) = shape.paint.stroke {
                    paints.push((
                        shape,
                        format!(
                            "fill=\"none\" {} stroke-width=\"{}\" stroke-linecap=\"{}\" stroke-linejoin=\"{}\" stroke-miterlimit=\"4\"",
                            color("stroke", stroke.color),
                            stroke.width,
                            match stroke.cap {
                                crate::CanvasLineCap::Butt => "butt",
                                crate::CanvasLineCap::Round => "round",
                                crate::CanvasLineCap::Square => "square",
                            },
                            match stroke.join {
                                crate::CanvasLineJoin::Miter => "miter",
                                crate::CanvasLineJoin::Round => "round",
                                crate::CanvasLineJoin::Bevel => "bevel",
                            }
                        ),
                    ));
                }
            }
            for (shape, style) in paints {
                let transform = &shape.transform;
                let x = (transform.rotation * Vec3::X * transform.scale.x).truncate();
                let y = (transform.rotation * Vec3::Y * transform.scale.y).truncate();
                let geometry = match &shape.geometry {
                    CanvasGeometry::Rectangle(rectangle) => format!(
                        "rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" rx=\"{}\" ry=\"{}\"",
                        -rectangle.size.x * 0.5,
                        -rectangle.size.y * 0.5,
                        rectangle.size.x,
                        rectangle.size.y,
                        rectangle.corner_radius,
                        rectangle.corner_radius
                    ),
                    CanvasGeometry::Ellipse(ellipse) => format!(
                        "ellipse cx=\"0\" cy=\"0\" rx=\"{}\" ry=\"{}\"",
                        ellipse.radii.x, ellipse.radii.y
                    ),
                    CanvasGeometry::Path(path) => format!("path d=\"{}\"", path_data(path)),
                };
                svg.push_str(&format!(
                    "<{geometry} transform=\"matrix({} {} {} {} {} {})\" {style} />",
                    x.x, x.y, y.x, y.y, transform.translation.x, transform.translation.y
                ));
            }
            svg.push_str("</g></svg>\n");
            std::fs::write(path, svg).map_err(CanvasExportError::Io)
        })();
        result.map_err(|error| {
            error.into_failure(
                ErrorFrame::new("canvas.export.svg")
                    .field("document", document)
                    .field("path", path.display())
                    .field("region", format_args!("{actual_region:?}")),
            )
        })
    }
}

fn path_data(path: &crate::CanvasPath) -> String {
    path.commands
        .iter()
        .map(|command| match *command {
            CanvasPathCommand::MoveTo(p) => format!("M {} {} ", p.x, p.y),
            CanvasPathCommand::LineTo(p) => format!("L {} {} ", p.x, p.y),
            CanvasPathCommand::QuadTo { control, to } => {
                format!("Q {} {} {} {} ", control.x, control.y, to.x, to.y)
            }
            CanvasPathCommand::CubicTo {
                control1,
                control2,
                to,
            } => format!(
                "C {} {} {} {} {} {} ",
                control1.x, control1.y, control2.x, control2.y, to.x, to.y
            ),
            CanvasPathCommand::ArcTo {
                radii,
                rotation,
                large_arc,
                sweep,
                to,
            } => {
                // Normalize through the same trig reduction as the View before changing units.
                // Large unreduced angles lose their axis direction in SVG consumers.
                let (sin, cos) = f64::from(rotation).sin_cos();
                let degrees = sin.atan2(cos).to_degrees();
                format!(
                    "A {} {} {} {} {} {} {} ",
                    radii.x,
                    radii.y,
                    degrees,
                    u8::from(large_arc),
                    u8::from(sweep),
                    to.x,
                    to.y
                )
            }
            CanvasPathCommand::Close => "Z ".to_owned(),
        })
        .collect()
}

fn color(attribute: &str, value: Color) -> String {
    let c = value.to_srgba();
    format!(
        "{attribute}=\"rgb({}%,{}%,{}%)\" {attribute}-opacity=\"{}\"",
        c.red * 100.0,
        c.green * 100.0,
        c.blue * 100.0,
        c.alpha
    )
}

struct Shape {
    entity: Entity,
    geometry: CanvasGeometry,
    transform: Transform,
    paint: CanvasPaint,
}

struct Snapshot {
    shapes: Vec<Shape>,
    region: Rect,
}

fn snapshot(
    world: &mut World,
    document: Entity,
    region: Option<Rect>,
) -> std::result::Result<Snapshot, CanvasExportError> {
    if world.get::<CanvasDocument>(document).is_none() {
        return Err(CanvasExportError::InvalidDocument);
    }
    let mut shapes = Vec::new();
    let mut bounds: Option<Rect> = None;
    let order = world
        .get::<crate::order::ShapeOrder>(document)
        .ok_or(CanvasExportError::InvalidDocument)?;
    for &entity in &order.0 {
        crate::model::validate_shape(world, document, entity).map_err(|error| {
            CanvasExportError::Model(
                error
                    .at("document", document)
                    .at("shape", entity)
                    .into_failure(bevy_widgetry_diagnostics::ErrorFrame::new(
                        "canvas.export.model",
                    )),
            )
        })?;
        let geometry = world.get::<CanvasGeometry>(entity);
        let transform = world.get::<Transform>(entity);
        let paint = world.get::<CanvasPaint>(entity);
        let (Some(geometry), Some(transform), Some(paint)) = (geometry, transform, paint) else {
            return Err(CanvasExportError::Model(
                crate::error::missing(crate::CanvasPart::Geometry)
                    .at("document", document)
                    .at("shape", entity)
                    .into_failure(ErrorFrame::new("canvas.export.snapshot")),
            ));
        };
        let b = geometry._bounds(transform, paint).map_err(|error| {
            CanvasExportError::Model(
                error
                    .at("document", document)
                    .at("shape", entity)
                    .into_failure(bevy_widgetry_diagnostics::ErrorFrame::new(
                        "canvas.export.bounds",
                    )),
            )
        })?;
        bounds = Some(bounds.map_or(b, |old| Rect {
            min: old.min.min(b.min),
            max: old.max.max(b.max),
        }));
        shapes.push(Shape {
            entity,
            geometry: geometry.clone(),
            transform: *transform,
            paint: *paint,
        });
    }
    let bounds = bounds.ok_or(CanvasExportError::EmptyDocument)?;
    let region = region.unwrap_or(bounds);
    if !region.min.is_finite()
        || !region.max.is_finite()
        || !region.size().is_finite()
        || region.size().min_element() <= 0.0
    {
        return Err(CanvasExportError::InvalidRegion { region });
    }
    Ok(Snapshot { shapes, region })
}

impl CanvasExportError {
    #[track_caller]
    fn into_failure(self, frame: ErrorFrame) -> JobFailure {
        match self {
            Self::Model(error) => error.with_context(frame),
            #[cfg(feature = "png_export")]
            Self::Background(error) => error.with_context(frame),
            #[cfg(feature = "png_export")]
            Self::Job(error) => JobFailure::from(error).with_context(frame),
            #[cfg(feature = "png_export")]
            Self::Offscreen(error) => JobFailure::from(error.into_bevy_error()).with_context(frame),
            #[cfg(feature = "png_export")]
            Self::Scene(error) => JobFailure::from(error.into_bevy_error()).with_context(frame),
            #[cfg(feature = "png_export")]
            Self::Cleanup { primary, cleanup } => primary
                .into_failure(frame.clone())
                .combine(JobFailure::from(cleanup.into_bevy_error()).with_context(frame)),
            error => WidgetryFailure::new(error, frame).into(),
        }
    }
    #[cfg(feature = "png_export")]
    fn with_cleanup(
        self,
        result: Result<(), bevy_widgetry_core::offscreen::OffscreenFailures>,
    ) -> Self {
        match result {
            Ok(()) => self,
            Err(cleanup) => Self::Cleanup {
                primary: Box::new(self),
                cleanup,
            },
        }
    }
}
