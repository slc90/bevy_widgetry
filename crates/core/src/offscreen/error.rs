use bevy::{ecs::world::error::EntityMutableFetchError, prelude::*};
use bevy_widgetry_diagnostics::{
    ErrorFrame, WidgetryFailure,
    bevy::{Diagnostic, WidgetryEcsError},
};
use std::{error::Error, fmt, sync::Arc};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct OffscreenTargetIdentity {
    pub image: AssetId<Image>,
    pub layer: usize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct OffscreenBinding {
    pub owner: Entity,
    pub viewport: Entity,
    pub camera: Entity,
    pub scene: Entity,
    pub target: OffscreenTargetIdentity,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OffscreenResource {
    Layers,
    Images,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OffscreenBindingRejection {
    MissingOwner,
    MissingViewportNode,
    MissingCamera,
    MissingScene,
    CameraOutsideScene,
    SceneInUse,
    CameraInUse,
    ViewportInUse,
    SceneIsOwner,
}

impl fmt::Display for OffscreenBindingRejection {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{self:?}")
    }
}
impl Error for OffscreenBindingRejection {}

#[derive(Debug)]
pub enum OffscreenError {
    EntityAccess {
        owner: Entity,
        entity: Entity,
        source: EntityMutableFetchError,
    },
    MissingResource {
        owner: Entity,
        target: Option<OffscreenTargetIdentity>,
        resource: OffscreenResource,
    },
    LayerExhausted {
        owner: Entity,
        next: usize,
    },
    InvalidBinding {
        binding: OffscreenBinding,
        reason: OffscreenBindingRejection,
    },
    MissingViewport {
        owner: Entity,
    },
    OwnerMismatch {
        binding: OffscreenBinding,
        actual_owner: Entity,
    },
    AssociationBroken {
        binding: OffscreenBinding,
        actual_camera: Option<Entity>,
    },
    MissingComputedNode {
        binding: OffscreenBinding,
    },
    MissingCamera {
        binding: OffscreenBinding,
    },
    MissingScene {
        binding: OffscreenBinding,
    },
    MissingImage {
        binding: OffscreenBinding,
    },
}

impl fmt::Display for OffscreenError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EntityAccess { .. } => f.write_str("Offscreen entity is unavailable"),
            Self::MissingResource { resource, .. } => {
                write!(f, "Offscreen resource missing: {resource:?}")
            }
            Self::LayerExhausted { .. } => f.write_str("Offscreen layer overflow"),
            Self::InvalidBinding { .. } => f.write_str("Offscreen binding rejected"),
            Self::MissingViewport { .. } => f.write_str("Offscreen viewport missing"),
            Self::OwnerMismatch { .. } => f.write_str("Offscreen owner mismatch"),
            Self::AssociationBroken { .. } => f.write_str("Offscreen viewport association broken"),
            Self::MissingComputedNode { .. } => f.write_str("Offscreen ComputedNode missing"),
            Self::MissingCamera { .. } => f.write_str("Offscreen camera missing"),
            Self::MissingScene { .. } => f.write_str("Offscreen scene missing"),
            Self::MissingImage { .. } => f.write_str("Offscreen image missing"),
        }
    }
}

impl Error for OffscreenError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::EntityAccess { source, .. } => Some(source),
            Self::InvalidBinding { reason, .. } => Some(reason),
            _ => None,
        }
    }
}

pub type OffscreenFailure = WidgetryFailure<OffscreenError>;

pub struct OffscreenFailures(pub(super) WidgetryEcsError);

impl OffscreenFailures {
    pub fn diagnostics(&self) -> impl Iterator<Item = &dyn Diagnostic> {
        self.0.diagnostics()
    }
    pub fn with_context(self, context: ErrorFrame) -> Self {
        Self(self.0.with_context(context))
    }
    pub fn append(mut self, other: Self) -> Self {
        for diagnostic in other.0.shared_diagnostics() {
            self.0.push_shared(diagnostic);
        }
        self
    }
    pub fn into_bevy_error(self) -> BevyError {
        self.0.into_bevy_error()
    }
}

impl fmt::Display for OffscreenFailures {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.0, f)
    }
}
impl fmt::Debug for OffscreenFailures {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, f)
    }
}
impl Error for OffscreenFailures {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        self.0.source()
    }
}

pub(super) fn binding_frame(binding: &OffscreenBinding, operation: &'static str) -> ErrorFrame {
    ErrorFrame::new(operation)
        .field("owner", binding.owner)
        .field("viewport", binding.viewport)
        .field("camera", binding.camera)
        .field("scene", binding.scene)
        .field("layer", binding.target.layer)
        .field("image", format_args!("{:?}", binding.target.image))
}

#[track_caller]
pub(super) fn failure(kind: OffscreenError, operation: &'static str) -> Arc<dyn Diagnostic> {
    let mut frame = ErrorFrame::new(operation);
    match &kind {
        OffscreenError::EntityAccess { owner, entity, .. } => {
            frame = frame.field("owner", owner).field("entity", entity)
        }
        OffscreenError::MissingResource {
            owner,
            target,
            resource,
        } => {
            frame = frame
                .field("owner", owner)
                .field("resource", format_args!("{resource:?}"));
            if let Some(target) = target {
                frame = frame
                    .field("layer", target.layer)
                    .field("image", format_args!("{:?}", target.image));
            }
        }
        OffscreenError::LayerExhausted { owner, next } => {
            frame = frame.field("owner", owner).field("next", next)
        }
        OffscreenError::MissingViewport { owner } => frame = frame.field("owner", owner),
        OffscreenError::InvalidBinding { binding, reason } => {
            frame = binding_frame(binding, operation).field("reason", reason)
        }
        OffscreenError::OwnerMismatch {
            binding,
            actual_owner,
        } => frame = binding_frame(binding, operation).field("actual_owner", actual_owner),
        OffscreenError::AssociationBroken {
            binding,
            actual_camera,
        } => {
            frame = binding_frame(binding, operation)
                .field("actual_camera", format_args!("{actual_camera:?}"))
        }
        OffscreenError::MissingComputedNode { binding }
        | OffscreenError::MissingCamera { binding }
        | OffscreenError::MissingScene { binding }
        | OffscreenError::MissingImage { binding } => frame = binding_frame(binding, operation),
    }
    Arc::new(OffscreenFailure::new(kind, frame))
}

#[track_caller]
pub(super) fn single(kind: OffscreenError, operation: &'static str) -> OffscreenFailures {
    OffscreenFailures(WidgetryEcsError::from_shared(failure(kind, operation)))
}

pub(super) fn push(failures: &mut Option<OffscreenFailures>, next: OffscreenFailures) {
    *failures = Some(match failures.take() {
        Some(current) => current.append(next),
        None => next,
    });
}
