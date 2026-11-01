use bevy::{asset::UntypedAssetId, ecs::query::QueryEntityError, prelude::Entity};
use bevy_widgetry_core::diagnostics::{CoreError, CoreFailure};
use bevy_widgetry_diagnostics::{ErrorFrame, WidgetryFailure};
use std::{error::Error, fmt};

pub type HoldButtonFailure = WidgetryFailure<HoldButtonError>;

#[derive(Debug)]
pub enum HoldButtonError {
    InvalidTarget {
        root: Entity,
    },
    InvalidDiameter {
        root: Entity,
        diameter: f32,
    },
    MissingPart {
        root: Entity,
        part: &'static str,
    },
    MissingStructure {
        root: Entity,
        missing: Vec<&'static str>,
        source: QueryEntityError,
    },
    MissingMaterial {
        root: Entity,
        material: UntypedAssetId,
    },
    Core {
        source: CoreError,
    },
}

impl fmt::Display for HoldButtonError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidTarget { .. } => f.write_str("HoldButton 目标不存在或类型不匹配"),
            Self::InvalidDiameter { .. } => f.write_str("HoldButton 直径必须为有限正数"),
            Self::MissingPart { part, .. } => write!(f, "HoldButton 缺失内部部件 {part}"),
            Self::MissingStructure { missing, .. } => {
                write!(f, "HoldButton 缺失渲染结构 {missing:?}")
            }
            Self::MissingMaterial { .. } => f.write_str("HoldButton 已拥有的材质缺失"),
            Self::Core { .. } => f.write_str("HoldButton 共享颜色操作失败"),
        }
    }
}

impl Error for HoldButtonError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Core { source } => Some(source),
            Self::MissingStructure { source, .. } => Some(source),
            _ => None,
        }
    }
}

#[track_caller]
pub(crate) fn failure(kind: HoldButtonError, operation: &'static str) -> HoldButtonFailure {
    let mut context = ErrorFrame::new(operation);
    match &kind {
        HoldButtonError::InvalidTarget { root } => context = context.field("root", root),
        HoldButtonError::InvalidDiameter { root, diameter } => {
            context = context.field("root", root).field("diameter", diameter)
        }
        HoldButtonError::MissingPart { root, part } => {
            context = context.field("root", root).field("part", part)
        }
        HoldButtonError::MissingStructure { root, missing, .. } => {
            context = context
                .field("root", root)
                .field("missing", format_args!("{missing:?}"))
        }
        HoldButtonError::MissingMaterial { root, material } => {
            context = context
                .field("root", root)
                .field("material", format_args!("{material:?}"))
        }
        _ => {}
    }
    HoldButtonFailure::new(kind, context)
}

pub(crate) fn core(
    error: CoreFailure,
    root: Entity,
    state: &'static str,
    field: &'static str,
) -> HoldButtonFailure {
    error
        .map_kind(|source| HoldButtonError::Core { source })
        .with_context(
            ErrorFrame::new("hold_button.validate_colors")
                .field("root", root)
                .field("state", state)
                .field("field", field),
        )
}
