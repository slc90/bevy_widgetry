use crate::{CanvasDocument, CanvasGeometry, CanvasPaint, CanvasRectangle, CanvasShape};
use bevy::prelude::*;
use bevy_widgetry_internal_log::widgetry_error;
use std::{fmt, path::Path};
mod png;
pub use png::{CanvasPngExport, CanvasPngExportFinished, CanvasPngOptions};
pub(crate) fn install(app: &mut App) {
    png::install(app);
}

#[derive(Debug)]
pub enum CanvasExportError {
    EmptyDocument,
    InvalidDocument,
    InvalidGeometry(String),
    InvalidRegion,
    UnsupportedColor,
    InvalidScale,
    InvalidResolution,
    RendererUnavailable,
    Render(String),
    Encoding(image::ImageError),
    Io(std::io::Error),
    Job(bevy_widgetry_job::SubmitError),
    Canceled,
}

impl fmt::Display for CanvasExportError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyDocument => f.write_str("Canvas document has no committed shapes"),
            Self::InvalidDocument => f.write_str("CanvasDocument missing"),
            Self::InvalidGeometry(error) => write!(f, "Invalid Canvas geometry: {error}"),
            Self::InvalidRegion => {
                f.write_str("Canvas export region must be finite with positive dimensions")
            }
            Self::Io(error) => write!(f, "Canvas export I/O failed: {error}"),
            Self::Job(error) => write!(f, "Canvas PNG Job submission failed: {error}"),
            Self::Canceled => f.write_str("Canvas PNG export canceled"),
            Self::InvalidScale => f.write_str("PNG scale must be finite and positive"),
            Self::InvalidResolution => {
                f.write_str("PNG resolution exceeds texture, buffer or memory limits")
            }
            Self::RendererUnavailable => f.write_str("Canvas PNG renderer unavailable"),
            Self::Render(error) => write!(f, "Canvas PNG rendering failed: {error}"),
            Self::Encoding(error) => write!(f, "Canvas PNG encoding failed: {error}"),
            Self::UnsupportedColor => {
                f.write_str("SVG requires sRGB colors and alpha within [0, 1]")
            }
        }
    }
}

impl std::error::Error for CanvasExportError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io(error) => Some(error),
            Self::Encoding(error) => Some(error),
            Self::Job(error) => Some(error),
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
    ) -> std::result::Result<(), CanvasExportError> {
        let path = path.as_ref();
        let result = (|| {
            let snapshot = snapshot(world, document, region)?;
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
                        return Err(CanvasExportError::UnsupportedColor);
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
                    paints.push((
                        shape.transform.translation.z,
                        shape,
                        format!("{} stroke=\"none\"", color("fill", fill)),
                    ));
                }
                if let Some(stroke) = shape.paint.stroke {
                    paints.push((
                        shape.transform.translation.z + 0.01,
                        shape,
                        format!(
                            "fill=\"none\" {} stroke-width=\"{}\" stroke-linejoin=\"miter\"",
                            color("stroke", stroke.color),
                            stroke.width
                        ),
                    ));
                }
            }
            paints.sort_by(|a, b| a.0.total_cmp(&b.0));
            for (_, shape, style) in paints {
                let Shape {
                    rectangle,
                    transform,
                    ..
                } = shape;
                let x = (transform.rotation * Vec3::X * transform.scale.x).truncate();
                let y = (transform.rotation * Vec3::Y * transform.scale.y).truncate();
                svg.push_str(&format!("<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" transform=\"matrix({} {} {} {} {} {})\" {} />",
                    -rectangle.size.x*0.5,-rectangle.size.y*0.5,rectangle.size.x,rectangle.size.y,
                    x.x,x.y,y.x,y.y,transform.translation.x,transform.translation.y,
                    style));
            }
            svg.push_str("</g></svg>\n");
            std::fs::write(path, svg).map_err(CanvasExportError::Io)
        })();
        result.inspect_err(|error| widgetry_error!(?document,?path,%error,"Canvas SVG 导出失败"))
    }
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
    rectangle: CanvasRectangle,
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
    for (parent, geometry, transform, paint) in world
        .query_filtered::<(
            &ChildOf,
            Option<&CanvasGeometry>,
            Option<&Transform>,
            Option<&CanvasPaint>,
        ), With<CanvasShape>>()
        .iter(world)
    {
        if parent.parent() != document {
            continue;
        }
        let (Some(geometry), Some(transform), Some(paint)) = (geometry, transform, paint) else {
            return Err(CanvasExportError::InvalidGeometry(
                "Shape requires CanvasGeometry, Transform and CanvasPaint".to_owned(),
            ));
        };
        let CanvasGeometry::Rectangle(rectangle) = geometry;
        let b = rectangle
            .bounds(transform, paint)
            .map_err(|error| CanvasExportError::InvalidGeometry(error.to_string()))?;
        bounds = Some(bounds.map_or(b, |old| Rect {
            min: old.min.min(b.min),
            max: old.max.max(b.max),
        }));
        shapes.push(Shape {
            rectangle: *rectangle,
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
        return Err(CanvasExportError::InvalidRegion);
    }
    Ok(Snapshot { shapes, region })
}
