use crate::{CanvasDocument, CanvasFill, CanvasGeometry, CanvasPaint, CanvasPathCommand};
use bevy::math::Affine3A;
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
    MissingGroupProxy(Entity),
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
            let region = snapshot.region;
            let [left, top] = region.min.as_dvec2().to_array();
            let [width, height] = region.size().as_dvec2().to_array();
            let mut svg = format!(
                "<svg xmlns=\"http://www.w3.org/2000/svg\" color-interpolation=\"linearRGB\" width=\"{width}\" height=\"{height}\" viewBox=\"{left} {top} {width} {height}\"><defs><clipPath id=\"canvas-region\"><rect x=\"{left}\" y=\"{top}\" width=\"{width}\" height=\"{height}\"/></clipPath></defs><g clip-path=\"url(#canvas-region)\">",
            );
            if snapshot.shapes.iter().any(|shape| match &shape.artwork {
                Artwork::Text(_) => true,
                Artwork::Connector(connector) => connector.label().is_some(),
                Artwork::Geometry(..) => false,
            }) {
                use base64::{Engine, engine::general_purpose::STANDARD};
                svg.push_str(
                    "<style>@font-face{font-family:CanvasDefault;src:url(data:font/ttf;base64,",
                );
                STANDARD.encode_string(bevy_widgetry_asset::BuiltinFont::Default.bytes(), &mut svg);
                svg.push_str(") format('truetype')}</style>");
            }
            for (index, shape) in snapshot.shapes.iter().enumerate() {
                let matrix = svg_matrix(shape.projection);
                let validate_color = |c: Color| {
                    let value = c.to_srgba();
                    if [value.red, value.green, value.blue, value.alpha]
                        .into_iter()
                        .all(|c| (0.0..=1.0).contains(&c))
                    {
                        Ok(())
                    } else {
                        Err(CanvasExportError::UnsupportedColor {
                            shape: shape.entity,
                            color: c,
                        })
                    }
                };
                match &shape.artwork {
                    Artwork::Geometry(geometry, paint) => {
                        write_svg_geometry(
                            &mut svg,
                            geometry,
                            *paint,
                            &matrix,
                            index,
                            validate_color,
                        )?;
                    }
                    Artwork::Text(text) => {
                        validate_color(text.style().color)?;
                        svg.push_str(&text.svg(&matrix));
                    }
                    Artwork::Connector(connector) => {
                        let (route, arrows) = connector.route().paths(connector.style());
                        let paint = connector.style().paint();
                        if !arrows.commands.is_empty() {
                            write_svg_geometry(
                                &mut svg,
                                &CanvasGeometry::Path(arrows),
                                CanvasPaint {
                                    stroke: None,
                                    ..paint
                                },
                                &matrix,
                                index,
                                validate_color,
                            )?;
                        }
                        write_svg_geometry(
                            &mut svg,
                            &CanvasGeometry::Path(route),
                            CanvasPaint {
                                fill: None,
                                ..paint
                            },
                            &matrix,
                            index,
                            validate_color,
                        )?;
                        if let Some((label, at)) = connector.label_at() {
                            validate_color(label.style().color)?;
                            let label_projection =
                                crate::CanvasConnector::label_projection(shape.projection, at);
                            svg.push_str(&label.svg(&svg_matrix(label_projection)));
                        }
                    }
                }
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

fn svg_matrix(projection: Affine3A) -> String {
    let [x, y] = crate::groups::planar_axes(&projection);
    let center = projection.translation.truncate().as_dvec2();
    format!(
        "matrix({} {} {} {} {} {})",
        x.x, x.y, y.x, y.y, center.x, center.y
    )
}

fn write_svg_geometry(
    svg: &mut String,
    geometry: &CanvasGeometry,
    paint: CanvasPaint,
    matrix: &str,
    index: usize,
    validate_color: impl Fn(Color) -> std::result::Result<(), CanvasExportError>,
) -> std::result::Result<(), CanvasExportError> {
    let geometry = match geometry {
        CanvasGeometry::Rectangle(rectangle) => {
            let size = rectangle.size.as_dvec2();
            let radius = f64::from(rectangle.corner_radius);
            format!(
                "rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" rx=\"{radius}\" ry=\"{radius}\"",
                -size.x * 0.5,
                -size.y * 0.5,
                size.x,
                size.y
            )
        }
        CanvasGeometry::Ellipse(ellipse) => {
            let [rx, ry] = ellipse.radii.as_dvec2().to_array();
            format!("ellipse cx=\"0\" cy=\"0\" rx=\"{rx}\" ry=\"{ry}\"")
        }
        CanvasGeometry::Path(path) => format!("path d=\"{}\"", path_data(path)),
    };
    if let Some(fill) = paint.fill {
        let fill = svg_fill(svg, fill, index, &validate_color)?;
        let rule = match paint.fill_rule {
            crate::CanvasFillRule::NonZero => "nonzero",
            crate::CanvasFillRule::EvenOdd => "evenodd",
        };
        svg.push_str(&format!(
            "<{geometry} transform=\"{matrix}\" {fill} fill-rule=\"{rule}\" stroke=\"none\" />"
        ));
    }
    if let Some(stroke) = paint.stroke {
        validate_color(stroke.color)?;
        let cap = match stroke.cap {
            crate::CanvasLineCap::Butt => "butt",
            crate::CanvasLineCap::Round => "round",
            crate::CanvasLineCap::Square => "square",
        };
        let join = match stroke.join {
            crate::CanvasLineJoin::Miter => "miter",
            crate::CanvasLineJoin::Round => "round",
            crate::CanvasLineJoin::Bevel => "bevel",
        };
        svg.push_str(&format!("<{geometry} transform=\"{matrix}\" fill=\"none\" {} stroke-width=\"{}\" stroke-linecap=\"{cap}\" stroke-linejoin=\"{join}\" stroke-miterlimit=\"4\" />",color("stroke",stroke.color),f64::from(stroke.width)));
    }
    Ok(())
}

fn svg_fill(
    svg: &mut String,
    fill: CanvasFill,
    index: usize,
    validate_color: impl Fn(Color) -> std::result::Result<(), CanvasExportError>,
) -> std::result::Result<String, CanvasExportError> {
    let (element, coordinates, stops) = match fill {
        CanvasFill::Solid(solid) => {
            validate_color(solid)?;
            return Ok(color("fill", solid));
        }
        CanvasFill::LinearGradient { start, end, stops } => {
            let [x1, y1] = start.to_array();
            let [x2, y2] = end.to_array();
            (
                "linearGradient",
                format!("x1=\"{x1}\" y1=\"{y1}\" x2=\"{x2}\" y2=\"{y2}\""),
                stops,
            )
        }
        CanvasFill::RadialGradient {
            center,
            radius,
            stops,
        } => {
            let [cx, cy] = center.to_array();
            (
                "radialGradient",
                format!("cx=\"{cx}\" cy=\"{cy}\" r=\"{radius}\""),
                stops,
            )
        }
    };
    let id = format!("canvas-gradient-{index}");
    svg.push_str(&format!(
        "<defs><{element} id=\"{id}\" gradientUnits=\"userSpaceOnUse\" {coordinates}>"
    ));
    for stop in stops.as_slice() {
        validate_color(stop.color)?;
        let c = stop.color.to_srgba();
        svg.push_str(&format!(
            "<stop offset=\"{}\" stop-color=\"rgb({}%,{}%,{}%)\" stop-opacity=\"{}\" />",
            stop.offset,
            c.red * 100.0,
            c.green * 100.0,
            c.blue * 100.0,
            c.alpha
        ));
    }
    svg.push_str(&format!("</{element}></defs>"));
    Ok(format!("fill=\"url(#{id})\""))
}

fn path_data(path: &crate::CanvasPath) -> String {
    path.commands
        .iter()
        .map(|command| match *command {
            CanvasPathCommand::MoveTo(p) => {
                let [x, y] = p.as_dvec2().to_array();
                format!("M {x} {y} ")
            }
            CanvasPathCommand::LineTo(p) => {
                let [x, y] = p.as_dvec2().to_array();
                format!("L {x} {y} ")
            }
            CanvasPathCommand::QuadTo { control, to } => {
                let [cx, cy] = control.as_dvec2().to_array();
                let [x, y] = to.as_dvec2().to_array();
                format!("Q {cx} {cy} {x} {y} ")
            }
            CanvasPathCommand::CubicTo {
                control1,
                control2,
                to,
            } => {
                let [cx1, cy1] = control1.as_dvec2().to_array();
                let [cx2, cy2] = control2.as_dvec2().to_array();
                let [x, y] = to.as_dvec2().to_array();
                format!("C {cx1} {cy1} {cx2} {cy2} {x} {y} ")
            }
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
                let [rx, ry] = radii.as_dvec2().to_array();
                let [x, y] = to.as_dvec2().to_array();
                format!(
                    "A {rx} {ry} {degrees} {} {} {x} {y} ",
                    u8::from(large_arc),
                    u8::from(sweep),
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

enum Artwork {
    Geometry(CanvasGeometry, CanvasPaint),
    Text(crate::CanvasText),
    Connector(crate::CanvasConnector),
}

impl crate::CanvasText {
    fn svg(&self, matrix: &str) -> String {
        let size = self.size();
        let style = self.style();
        let (x, anchor) = match style.align {
            crate::CanvasTextAlign::Start => (-size.x * 0.5, "start"),
            crate::CanvasTextAlign::Center => (0.0, "middle"),
            crate::CanvasTextAlign::End => (size.x * 0.5, "end"),
        };
        let mut svg = format!(
            "<text transform=\"{matrix}\" xml:space=\"preserve\" font-family=\"CanvasDefault, Smiley Sans Oblique, sans-serif\" font-size=\"{}\" text-anchor=\"{anchor}\" {}>",
            style.font_size,
            color("fill", style.color)
        );
        for (line, baseline) in self.lines() {
            let line = line
                .trim_end_matches(['\n', '\r', '\u{85}', '\u{2028}', '\u{2029}'])
                .replace('&', "&amp;")
                .replace('<', "&lt;")
                .replace('>', "&gt;");
            let y = baseline - size.y * 0.5;
            svg.push_str(&format!("<tspan x=\"{x}\" y=\"{y}\">{line}</tspan>"));
        }
        svg.push_str("</text>");
        svg
    }
}

struct Shape {
    entity: Entity,
    artwork: Artwork,
    projection: Affine3A,
    #[cfg(feature = "png_export")]
    container: crate::CanvasContainer,
    #[cfg(feature = "png_export")]
    local_transform: Transform,
}

#[cfg(feature = "png_export")]
struct Group {
    entity: Entity,
    container: crate::CanvasContainer,
    local_transform: Transform,
}

struct Snapshot {
    shapes: Vec<Shape>,
    region: Rect,
    #[cfg(feature = "png_export")]
    groups: Vec<Group>,
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
    #[cfg(feature = "png_export")]
    let mut groups = Vec::new();
    let mut bounds: Option<Rect> = None;
    let order = world
        .get::<crate::CanvasLayerStack>(document)
        .ok_or(CanvasExportError::InvalidDocument)?;
    let visible_layers = order.layers().iter().filter(|layer| layer.visible());
    crate::order::visit_draw_order(world, visible_layers, |entity, container, entry| {
        let group = entry.is_some_and(crate::groups::has_group_components);
        if !entry.is_some_and(|entry| {
            crate::order::is_item_child(entry, container.parent(document))
                && (group
                    || crate::model::has_shape_components(entry)
                    || entry.contains::<crate::CanvasText>()
                    || entry.contains::<crate::CanvasConnector>())
        }) {
            return Err(CanvasExportError::Model(
                crate::model::invalid_ownership(world, entity)
                    .at("document", document)
                    .at("shape", entity)
                    .into_failure(bevy_widgetry_diagnostics::ErrorFrame::new(
                        "canvas.export.model",
                    )),
            ));
        }
        let local_transform = *world.get::<Transform>(entity).ok_or_else(|| {
            CanvasExportError::Model(
                crate::error::missing(crate::CanvasPart::Transform)
                    .at("document", document)
                    .at("item", entity)
                    .into_failure(ErrorFrame::new("canvas.export.snapshot")),
            )
        })?;
        if group {
            crate::groups::validate_group_transform(local_transform).map_err(|error| {
                CanvasExportError::Model(
                    error
                        .at("document", document)
                        .at("group", entity)
                        .into_failure(ErrorFrame::new("canvas.export.snapshot")),
                )
            })?;
            #[cfg(feature = "png_export")]
            groups.push(Group {
                entity,
                container,
                local_transform,
            });
            return Ok(true);
        }
        let projection = crate::groups::document_transform(world, entity).map_err(|error| {
            CanvasExportError::Model(
                error
                    .at("document", document)
                    .at("shape", entity)
                    .into_failure(ErrorFrame::new("canvas.export.snapshot")),
            )
        })?;
        let (artwork, b) = if let Some(text) = world.get::<crate::CanvasText>(entity) {
            let b = text.bounds(&local_transform, &projection).map_err(|e| {
                CanvasExportError::Model(
                    e.at("text", entity)
                        .into_failure(ErrorFrame::new("canvas.export.bounds")),
                )
            })?;
            (Artwork::Text(text.clone()), b)
        } else if let Some(connector) = world.get::<crate::CanvasConnector>(entity) {
            if !connector.drawn() {
                return Ok(false);
            }
            let bounds = connector
                .bounds(&local_transform, &projection)
                .map_err(|error| {
                    CanvasExportError::Model(
                        error
                            .at("connector", entity)
                            .into_failure(ErrorFrame::new("canvas.export.bounds")),
                    )
                })?;
            (Artwork::Connector(connector.clone()), bounds)
        } else {
            let geometry = world.get::<CanvasGeometry>(entity);
            let paint = world.get::<CanvasPaint>(entity);
            let (Some(geometry), Some(paint)) = (geometry, paint) else {
                return Err(CanvasExportError::Model(
                    crate::error::missing(crate::CanvasPart::Geometry)
                        .at("document", document)
                        .at("shape", entity)
                        .into_failure(ErrorFrame::new("canvas.export.snapshot")),
                ));
            };
            let b = geometry
                .bounds_in_projection(&local_transform, &projection, paint)
                .map_err(|error| {
                    CanvasExportError::Model(
                        error
                            .at("document", document)
                            .at("shape", entity)
                            .into_failure(bevy_widgetry_diagnostics::ErrorFrame::new(
                                "canvas.export.bounds",
                            )),
                    )
                })?;
            (Artwork::Geometry(geometry.clone(), *paint), b)
        };
        bounds = Some(bounds.map_or(b, |old| Rect {
            min: old.min.min(b.min),
            max: old.max.max(b.max),
        }));
        shapes.push(Shape {
            entity,
            artwork,
            projection,
            #[cfg(feature = "png_export")]
            container,
            #[cfg(feature = "png_export")]
            local_transform,
        });
        Ok(true)
    })?;
    let bounds = bounds.ok_or(CanvasExportError::EmptyDocument)?;
    let region = region.unwrap_or(bounds);
    if !region.min.is_finite()
        || !region.max.is_finite()
        || !region.size().is_finite()
        || region.size().min_element() <= 0.0
    {
        return Err(CanvasExportError::InvalidRegion { region });
    }
    Ok(Snapshot {
        shapes,
        region,
        #[cfg(feature = "png_export")]
        groups,
    })
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
