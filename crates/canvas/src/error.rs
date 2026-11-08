use crate::{CanvasEllipse, CanvasGeometry, CanvasPaint, CanvasPathCommand, CanvasRectangle};
use bevy::prelude::*;
use bevy_widgetry_diagnostics::{ErrorFrame, WidgetryFailure, bevy::FailureState};
use bevy_widgetry_job::JobFailure;
use std::{error::Error, fmt};

pub type CanvasFailure = WidgetryFailure<CanvasError>;
pub(crate) type PublicResult<T = ()> = std::result::Result<T, JobFailure>;
pub(crate) type CanvasResult<T = ()> = std::result::Result<T, CanvasFault>;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CanvasPart {
    BatchGeometry,
    BatchPaint,
    Camera,
    CameraTransform,
    Document,
    DocumentOrder,
    Geometry,
    HitFillMesh,
    HitLayout,
    HitMeshIndices,
    HitMeshPositions,
    HitStrokeMesh,
    HitTargetScale,
    HitUICamera,
    HitUITransform,
    Layout,
    Material,
    MeshVisibility,
    NodeOverlay,
    NodeOverlayMesh,
    NodePaint,
    NodePath,
    NodePoint,
    NodeTransform,
    OutlineAssets,
    OutlineIndices,
    OutlineMesh,
    OutlinePositions,
    Paint,
    ParameterGeometry,
    ParameterPaint,
    ParameterTransform,
    PenPreview,
    PenPreviewMaterial,
    PenPreviewMesh,
    PenPreviewParts,
    PenState,
    PreviewAssets,
    PreviewFill,
    PreviewMaterial,
    PreviewParts,
    PreviewStroke,
    PreviewTransform,
    Projection,
    ProxyAssets,
    ProxyParts,
    ProxyTransform,
    Renderer,
    RotationTransform,
    ScalingTransform,
    SceneParts,
    SegmentEndpoint,
    SelectionGeometry,
    SelectionPaint,
    SelectionTransform,
    ShapePaint,
    ShapeTransform,
    SharedFillMesh,
    SharedMeshPositions,
    SharedStrokeMesh,
    StrokeTransform,
    SubpathMoveTo,
    TopologyPaint,
    TopologyTransform,
    Transform,
    View,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CanvasIssue {
    AnchorInsertion,
    AppendEllipse,
    AppendKind,
    AppendRectangle,
    ArcOverflow,
    ArcParameters,
    CenterTarget,
    CloseAnchor,
    CommandIndex,
    ContourPoints,
    CornerRadius,
    DocumentTarget,
    DuplicateShape,
    EllipseBounds,
    EllipseEndpoints,
    EllipseLocalBounds,
    EllipseMeshOverflow,
    EllipseRadii,
    EmptyPath,
    FlattenedBounds,
    GeometryKind,
    HitIndex,
    HitTriangle,
    MeshBounds,
    MeshIndexOverflow,
    OrderMembership,
    Ownership,
    Paint,
    PathBounds,
    PathClose,
    PathDerivedGeometry,
    PathMeshOverflow,
    PathPoint,
    PathStart,
    PenContext,
    PreviewBounds,
    RectangleBounds,
    RectangleSize,
    SegmentKind,
    SelectionTarget,
    TopologyKind,
    Transform,
    ZoomTarget,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CanvasErrorKind {
    Structure(CanvasPart),
    Invalid(CanvasIssue),
    Tessellation,
    IndexConversion,
}

#[derive(Clone, Debug)]
pub enum CanvasInput {
    Entity {
        field: &'static str,
        value: Entity,
    },
    Entities {
        field: &'static str,
        values: Vec<Entity>,
    },
    Float {
        field: &'static str,
        value: f32,
    },
    Index {
        field: &'static str,
        value: usize,
    },
    Flag {
        field: &'static str,
        value: bool,
    },
    Point {
        field: &'static str,
        value: Vec2,
    },
    Points {
        field: &'static str,
        values: Vec<Vec2>,
    },
    Rectangle(CanvasRectangle),
    Ellipse(CanvasEllipse),
    Geometry(CanvasGeometry),
    Command {
        index: usize,
        value: CanvasPathCommand,
    },
    Transform(Transform),
    Paint(CanvasPaint),
}

#[derive(Debug)]
pub enum CanvasErrorSource {
    Tessellation(lyon::tessellation::TessellationError),
    IndexConversion(std::num::TryFromIntError),
}

#[derive(Debug)]
pub struct CanvasError {
    pub kind: CanvasErrorKind,
    pub inputs: Vec<CanvasInput>,
    pub source: Option<CanvasErrorSource>,
}
impl fmt::Display for CanvasError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "canvas operation rejected: {:?} {:?}",
            self.kind, self.inputs
        )
    }
}
impl Error for CanvasError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match &self.source {
            Some(CanvasErrorSource::Tessellation(error)) => Some(error),
            Some(CanvasErrorSource::IndexConversion(error)) => Some(error),
            None => None,
        }
    }
}

#[derive(Debug)]
pub(crate) enum CanvasFault {
    Domain(CanvasError, Vec<ErrorFrame>),
    Managed(JobFailure),
    Boundary(BevyError),
}

impl CanvasFault {
    pub fn input(self, input: CanvasInput) -> Self {
        match self {
            Self::Domain(mut error, frames) => {
                error.inputs.push(input);
                Self::Domain(error, frames)
            }
            other => other
                .context(ErrorFrame::new("canvas.input").field("input", format_args!("{input:?}"))),
        }
    }
    pub fn at(self, field: &'static str, value: Entity) -> Self {
        self.input(CanvasInput::Entity { field, value })
    }
    pub fn context(self, frame: ErrorFrame) -> Self {
        match self {
            Self::Domain(error, mut frames) => {
                frames.push(frame);
                Self::Domain(error, frames)
            }
            Self::Managed(error) => Self::Managed(error.with_context(frame)),
            Self::Boundary(error) => Self::Boundary(
                bevy_widgetry_diagnostics::bevy::BoundaryFailure::Native(error)
                    .with_context(frame)
                    .into_bevy_error(),
            ),
        }
    }
    pub fn into_failure(self, frame: ErrorFrame) -> JobFailure {
        match self {
            Self::Domain(error, frames) => {
                let mut failure = CanvasFailure::new(error, frame);
                for frame in frames {
                    failure = failure.with_context(frame);
                }
                failure.into()
            }
            Self::Managed(error) => error.with_context(frame),
            Self::Boundary(error) => JobFailure::from(error).with_context(frame),
        }
    }
    pub fn into_bevy_error(self, frame: ErrorFrame) -> BevyError {
        match self {
            Self::Boundary(error) => {
                bevy_widgetry_diagnostics::bevy::BoundaryFailure::Native(error)
                    .with_context(frame)
                    .into_bevy_error()
            }
            other => other.into_failure(frame).into_bevy_error(),
        }
    }
    fn key(&self) -> Option<FaultKey> {
        if let Self::Domain(error, _) = self {
            let mut values = Vec::new();
            for input in &error.inputs {
                input.key(&mut values);
            }
            Some(FaultKey {
                kind: error.kind,
                values,
                fields: error
                    .inputs
                    .iter()
                    .filter_map(|input| match input {
                        CanvasInput::Entity { field, .. }
                        | CanvasInput::Entities { field, .. }
                        | CanvasInput::Float { field, .. }
                        | CanvasInput::Index { field, .. }
                        | CanvasInput::Flag { field, .. }
                        | CanvasInput::Point { field, .. }
                        | CanvasInput::Points { field, .. } => Some(*field),
                        _ => None,
                    })
                    .collect(),
            })
        } else {
            None
        }
    }
}

impl From<BevyError> for CanvasFault {
    fn from(error: BevyError) -> Self {
        if error.severity() == bevy::ecs::error::Severity::Error
            && error
                .downcast_ref::<bevy_widgetry_diagnostics::bevy::WidgetryEcsError>()
                .is_some()
        {
            Self::Managed(error.into())
        } else {
            Self::Boundary(error)
        }
    }
}
impl From<JobFailure> for CanvasFault {
    fn from(error: JobFailure) -> Self {
        Self::Managed(error)
    }
}
impl<E: Error + Send + Sync + 'static> From<WidgetryFailure<E>> for CanvasFault {
    fn from(error: WidgetryFailure<E>) -> Self {
        Self::Managed(error.into())
    }
}

pub(crate) fn missing(part: CanvasPart) -> CanvasFault {
    fault(CanvasErrorKind::Structure(part), None)
}
pub(crate) fn invalid(issue: CanvasIssue) -> CanvasFault {
    fault(CanvasErrorKind::Invalid(issue), None)
}
fn fault(kind: CanvasErrorKind, source: Option<CanvasErrorSource>) -> CanvasFault {
    CanvasFault::Domain(
        CanvasError {
            kind,
            inputs: Vec::new(),
            source,
        },
        Vec::new(),
    )
}
pub(crate) fn tessellation(source: lyon::tessellation::TessellationError) -> CanvasFault {
    fault(
        CanvasErrorKind::Tessellation,
        Some(CanvasErrorSource::Tessellation(source)),
    )
}
pub(crate) fn index_conversion(source: std::num::TryFromIntError) -> CanvasFault {
    fault(
        CanvasErrorKind::IndexConversion,
        Some(CanvasErrorSource::IndexConversion(source)),
    )
}

#[derive(PartialEq, Eq)]
struct FaultKey {
    fields: Vec<&'static str>,
    kind: CanvasErrorKind,
    values: Vec<u64>,
}
#[derive(Default)]
pub(crate) struct CanvasFailureState(FailureState<FaultKey>);
impl CanvasFailureState {
    pub fn observe<T>(
        &mut self,
        result: CanvasResult<T>,
        frame: ErrorFrame,
    ) -> std::result::Result<T, BevyError> {
        match result {
            Ok(value) => {
                self.0.recover(|| frame);
                Ok(value)
            }
            Err(error) => match error.key() {
                Some(key) => Err(self
                    .0
                    .failure_batch(key, || error.into_failure(frame).into_report())
                    .into_bevy_error()),
                None => Err(error.into_bevy_error(frame)),
            },
        }
    }
}

impl CanvasInput {
    fn key(&self, values: &mut Vec<u64>) {
        const INPUT_ENTITY: u64 = 0;
        const INPUT_ENTITIES: u64 = 1;
        const INPUT_FLOAT: u64 = 2;
        const INPUT_INDEX: u64 = 3;
        const INPUT_FLAG: u64 = 4;
        const INPUT_POINT: u64 = 5;
        const INPUT_POINTS: u64 = 6;
        const INPUT_RECTANGLE: u64 = 7;
        const INPUT_ELLIPSE: u64 = 8;
        const INPUT_GEOMETRY: u64 = 9;
        const INPUT_PATH: u64 = 10;
        const INPUT_COMMAND: u64 = 11;
        const INPUT_TRANSFORM: u64 = 12;
        const INPUT_PAINT: u64 = 13;

        fn floats(values: &mut Vec<u64>, floats: impl IntoIterator<Item = f32>) {
            values.extend(floats.into_iter().map(|v| u64::from(v.to_bits())));
        }
        fn command(values: &mut Vec<u64>, c: CanvasPathCommand) {
            use CanvasPathCommand::*;
            const COMMAND_MOVE_TO: u64 = 0;
            const COMMAND_LINE_TO: u64 = 1;
            const COMMAND_QUAD_TO: u64 = 2;
            const COMMAND_CUBIC_TO: u64 = 3;
            const COMMAND_ARC_TO: u64 = 4;
            const COMMAND_CLOSE: u64 = 5;
            match c {
                MoveTo(p) => {
                    values.push(COMMAND_MOVE_TO);
                    floats(values, p.to_array());
                }
                LineTo(p) => {
                    values.push(COMMAND_LINE_TO);
                    floats(values, p.to_array());
                }
                QuadTo { control, to } => {
                    values.push(COMMAND_QUAD_TO);
                    floats(values, control.to_array().into_iter().chain(to.to_array()));
                }
                CubicTo {
                    control1,
                    control2,
                    to,
                } => {
                    values.push(COMMAND_CUBIC_TO);
                    floats(
                        values,
                        control1
                            .to_array()
                            .into_iter()
                            .chain(control2.to_array())
                            .chain(to.to_array()),
                    );
                }
                ArcTo {
                    radii,
                    rotation,
                    large_arc,
                    sweep,
                    to,
                } => {
                    values.extend([COMMAND_ARC_TO, u64::from(large_arc), u64::from(sweep)]);
                    floats(
                        values,
                        radii
                            .to_array()
                            .into_iter()
                            .chain([rotation])
                            .chain(to.to_array()),
                    );
                }
                Close => values.push(COMMAND_CLOSE),
            }
        }
        match self {
            Self::Entity { value, .. } => values.extend([INPUT_ENTITY, value.to_bits()]),
            Self::Entities {
                values: entities, ..
            } => {
                values.extend([INPUT_ENTITIES, entities.len() as u64]);
                values.extend(entities.iter().map(|e| e.to_bits()));
            }
            Self::Float { value, .. } => values.extend([INPUT_FLOAT, u64::from(value.to_bits())]),
            Self::Index { value, .. } => values.extend([INPUT_INDEX, *value as u64]),
            Self::Flag { value, .. } => values.extend([INPUT_FLAG, u64::from(*value)]),
            Self::Point { value, .. } => {
                values.push(INPUT_POINT);
                floats(values, value.to_array());
            }
            Self::Points { values: points, .. } => {
                values.extend([INPUT_POINTS, points.len() as u64]);
                floats(values, points.iter().flat_map(|p| p.to_array()));
            }
            Self::Rectangle(r) => {
                values.push(INPUT_RECTANGLE);
                floats(
                    values,
                    r.size.to_array().into_iter().chain([r.corner_radius]),
                );
            }
            Self::Ellipse(e) => {
                values.push(INPUT_ELLIPSE);
                floats(values, e.radii.to_array());
            }
            Self::Geometry(g) => {
                values.push(INPUT_GEOMETRY);
                match g {
                    CanvasGeometry::Rectangle(r) => Self::Rectangle(*r).key(values),
                    CanvasGeometry::Ellipse(e) => Self::Ellipse(*e).key(values),
                    CanvasGeometry::Path(p) => {
                        values.extend([INPUT_PATH, p.commands.len() as u64]);
                        for c in &p.commands {
                            command(values, *c);
                        }
                    }
                }
            }
            Self::Command { index, value } => {
                values.extend([INPUT_COMMAND, *index as u64]);
                command(values, *value);
            }
            Self::Transform(t) => {
                values.push(INPUT_TRANSFORM);
                floats(
                    values,
                    t.translation
                        .to_array()
                        .into_iter()
                        .chain(t.rotation.to_array())
                        .chain(t.scale.to_array()),
                );
            }
            Self::Paint(p) => {
                values.extend([INPUT_PAINT, p.fill_rule as u64, u64::from(p.fill.is_some())]);
                if let Some(fill) = p.fill {
                    floats(values, fill.to_linear().to_f32_array());
                }
                values.push(u64::from(p.stroke.is_some()));
                if let Some(s) = p.stroke {
                    values.extend([s.cap as u64, s.join as u64]);
                    floats(
                        values,
                        [s.width]
                            .into_iter()
                            .chain(s.color.to_linear().to_f32_array()),
                    );
                }
            }
        }
    }
}

impl From<lyon::tessellation::TessellationError> for CanvasFault {
    fn from(error: lyon::tessellation::TessellationError) -> Self {
        tessellation(error)
    }
}
impl From<std::num::TryFromIntError> for CanvasFault {
    fn from(error: std::num::TryFromIntError) -> Self {
        index_conversion(error)
    }
}

#[cfg(test)]
#[allow(clippy::disallowed_macros, clippy::unwrap_used)]
mod tests {
    use super::*;
    use bevy::ecs::error::Severity;
    use bevy_widgetry_test_utils::{DiagnosticPhase, LogCapture};

    #[test]
    fn native_boundary_strategy_and_managed_batch_origins_survive_canvas_context() {
        let native = BevyError::error("foreign failure").with_severity(Severity::Panic);
        let native = CanvasFault::from(native)
            .at("view", Entity::PLACEHOLDER)
            .into_bevy_error(ErrorFrame::new("canvas.test"));
        assert_eq!(native.severity(), Severity::Panic);
        let first = missing(CanvasPart::View).into_failure(ErrorFrame::new("first"));
        let second = missing(CanvasPart::Paint).into_failure(ErrorFrame::new("second"));
        let original = first.combine(second);
        let ids: Vec<_> = original.diagnostics().map(|d| d.origin().id()).collect();
        let result = CanvasFault::from(original.clone().into_bevy_error())
            .into_bevy_error(ErrorFrame::new("canvas.propagate"));
        let managed = result
            .downcast_ref::<bevy_widgetry_diagnostics::bevy::WidgetryEcsError>()
            .unwrap();
        assert_eq!(result.severity(), Severity::Error);
        assert_eq!(
            managed
                .diagnostics()
                .map(|d| d.origin().id())
                .collect::<Vec<_>>(),
            ids
        );
        for (before, after) in original.diagnostics().zip(managed.diagnostics()) {
            assert!(std::sync::Arc::ptr_eq(before.origin(), after.origin()));
            assert!(
                after
                    .contexts()
                    .iter()
                    .any(|frame| frame.operation() == "canvas.propagate")
            );
        }
    }

    #[test]
    fn geometry_state_uses_exact_values_and_preserves_the_lyon_source() {
        use lyon::tessellation::{TessellationError, UnsupportedParamater};
        let logs = LogCapture::default();
        logs.run(|| {
            let mut state = CanvasFailureState::default();
            let fault = |value| {
                tessellation(TessellationError::UnsupportedParamater(
                    UnsupportedParamater::PositionIsNaN,
                ))
                .input(CanvasInput::Float {
                    field: "value",
                    value,
                })
            };
            let frame = || ErrorFrame::new("canvas.tessellate");
            let first = state
                .observe::<()>(Err(fault(f32::NAN)), frame())
                .unwrap_err();
            let second = state
                .observe::<()>(Err(fault(f32::NAN)), frame())
                .unwrap_err();
            let ids = |error: &BevyError| {
                error
                    .downcast_ref::<bevy_widgetry_diagnostics::bevy::WidgetryEcsError>()
                    .unwrap()
                    .diagnostics()
                    .next()
                    .unwrap()
                    .origin()
                    .id()
                    .clone()
            };
            assert_eq!(ids(&first), ids(&second));
            assert_eq!(
                logs.diagnostic_records(&ids(&first), DiagnosticPhase::Source)
                    .len(),
                1
            );
            let managed = first
                .downcast_ref::<bevy_widgetry_diagnostics::bevy::WidgetryEcsError>()
                .unwrap();
            let domain = managed
                .diagnostics()
                .next()
                .unwrap()
                .source()
                .unwrap()
                .downcast_ref::<CanvasError>()
                .unwrap();
            assert!(matches!(
                domain.source().unwrap().downcast_ref::<TessellationError>(),
                Some(TessellationError::UnsupportedParamater(
                    UnsupportedParamater::PositionIsNaN
                ))
            ));
            let changed = state
                .observe::<()>(Err(fault(f32::from_bits(f32::NAN.to_bits() + 1))), frame())
                .unwrap_err();
            assert_ne!(ids(&first), ids(&changed));
            state.observe(Ok(()), frame()).unwrap();
            let repeated = state
                .observe::<()>(Err(fault(f32::NAN)), frame())
                .unwrap_err();
            assert_ne!(ids(&first), ids(&repeated));
        });
    }
}
