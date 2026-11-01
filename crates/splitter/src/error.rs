use crate::WidgetrySplitterOrientation;
use bevy::{ecs::query::QueryEntityError, prelude::*, ui::ValArithmeticError};
use bevy_widgetry_core::diagnostics::{CoreError, CoreFailure};
use bevy_widgetry_diagnostics::{ErrorFrame, WidgetryFailure};
use std::{error::Error, fmt};

pub type SplitterFailure = WidgetryFailure<SplitterError>;

#[derive(Debug)]
pub enum SplitterError {
    InvalidTarget {
        root: Entity,
    },
    InvalidRatio {
        root: Entity,
        ratio: f32,
    },
    InvalidDimension {
        root: Entity,
        field: &'static str,
        value: f32,
        handle_thickness: f32,
    },
    MissingPart {
        root: Option<Entity>,
        entity: Entity,
        part: &'static str,
    },
    InvalidOwner {
        root: Option<Entity>,
        entity: Entity,
        parent: Option<Entity>,
        part: &'static str,
    },
    Layout {
        root: Entity,
        orientation: Option<WidgetrySplitterOrientation>,
        parts: Option<[Entity; 3]>,
        source: Box<SplitterLayoutError>,
    },
    Core {
        source: CoreError,
    },
}

#[derive(Debug)]
pub enum SplitterLayoutError {
    AtPart {
        entity: Entity,
        part: &'static str,
        source: Box<Self>,
    },
    MissingPart {
        entity: Entity,
        part: &'static str,
    },
    Query {
        source: QueryEntityError,
        part: &'static str,
    },
    InvalidParent {
        entity: Entity,
        expected: Entity,
        actual: Option<Entity>,
    },
    ConstraintResolution {
        value: Val,
        source: ValArithmeticError,
    },
    InvalidConstraint {
        value: Val,
    },
    NoHandleSpace {
        a: [f32; 2],
        b: [f32; 2],
        thickness: f32,
    },
    ConflictingBounds {
        min: f32,
        max: f32,
    },
    ConflictingPanes {
        a: [Val; 2],
        b: [Val; 2],
        base: f32,
    },
    LayoutUnavailable,
}
impl fmt::Display for SplitterError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidTarget { .. } => f.write_str("Splitter 目标不存在或类型不匹配"),
            Self::InvalidRatio { .. } => f.write_str("Splitter 比例必须有限且位于 [0,1]"),
            Self::InvalidDimension { field, .. } => write!(f, "Splitter 尺寸配置无效: {field}"),
            Self::MissingPart { part, .. } => write!(f, "Splitter 缺失必需部件 {part}"),
            Self::InvalidOwner { part, .. } => write!(f, "Splitter 部件归属无效: {part}"),
            Self::Layout { .. } => f.write_str("Splitter 布局失败"),
            Self::Core { .. } => f.write_str("Splitter 共享状态操作失败"),
        }
    }
}
impl Error for SplitterError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Layout { source, .. } => Some(source.as_ref()),
            Self::Core { source } => Some(source),
            _ => None,
        }
    }
}
impl fmt::Display for SplitterLayoutError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::AtPart { part, .. } => write!(f, "布局部件 {part} 失败"),
            Self::MissingPart { part, .. } => write!(f, "布局缺失必需部件 {part}"),
            Self::Query { part, .. } => write!(f, "布局部件查询失败: {part}"),
            Self::InvalidParent { .. } => f.write_str("Pane 或 Handle 父级无效"),
            Self::ConstraintResolution { .. } => f.write_str("无法解析 Pane 约束"),
            Self::InvalidConstraint { .. } => f.write_str("Pane 约束必须有限且非负"),
            Self::NoHandleSpace { .. } => f.write_str("Pane 约束没有为 Handle 留出空间"),
            Self::ConflictingBounds { .. } | Self::ConflictingPanes { .. } => {
                f.write_str("Pane 最小约束超过最大约束")
            }
            Self::LayoutUnavailable => f.write_str("需要已经初始化的有效主轴布局"),
        }
    }
}
impl Error for SplitterLayoutError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::AtPart { source, .. } => Some(source.as_ref()),
            Self::Query { source, .. } => Some(source),
            Self::ConstraintResolution { source, .. } => Some(source),
            _ => None,
        }
    }
}

#[track_caller]
pub(crate) fn failure(kind: SplitterError, frame: ErrorFrame) -> SplitterFailure {
    let frame = frame.field("failure", format_args!("{kind:?}"));
    SplitterFailure::new(kind, frame)
}
pub(crate) fn core(error: CoreFailure, frame: ErrorFrame) -> SplitterFailure {
    error
        .map_kind(|source| SplitterError::Core { source })
        .with_context(frame)
}
#[track_caller]
pub(crate) fn missing(root: Option<Entity>, entity: Entity, part: &'static str) -> SplitterFailure {
    failure(
        SplitterError::MissingPart { root, entity, part },
        ErrorFrame::new("splitter.access_part"),
    )
}
pub(crate) fn layout_kind(
    root: Entity,
    state: Option<&crate::splitter::State>,
    parts: Option<&crate::splitter::Parts>,
    source: SplitterLayoutError,
) -> SplitterError {
    SplitterError::Layout {
        root,
        orientation: state.map(|state| state.orientation),
        parts: parts.map(|parts| [parts.a, parts.handle, parts.b]),
        source: Box::new(source),
    }
}
pub(crate) fn layout(world: &World, root: Entity, source: SplitterLayoutError) -> SplitterFailure {
    failure(
        layout_kind(
            root,
            world.get::<crate::splitter::State>(root),
            world.get::<crate::splitter::Parts>(root),
            source,
        ),
        ErrorFrame::new("splitter.layout").field("root", root),
    )
}

#[derive(PartialEq, Eq)]
pub(crate) enum LayoutKey {
    AtPart(Entity, &'static str, Box<Self>),
    Missing(Entity, &'static str),
    Query(Entity, QueryFailure, &'static str),
    Parent(Entity, Entity, Option<Entity>),
    Resolution(ValKey, ValArithmeticError),
    Constraint(ValKey),
    HandleSpace([u32; 5]),
    Bounds(u32, u32),
    Panes([ValKey; 4], u32),
    Unavailable,
}
#[derive(PartialEq, Eq)]
pub(crate) enum QueryFailure {
    Missing,
    NotSpawned,
    Aliased,
}
#[derive(PartialEq, Eq)]
pub(crate) enum ValKey {
    Auto,
    Px(u32),
    Percent(u32),
    Vw(u32),
    Vh(u32),
    VMin(u32),
    VMax(u32),
    Em(u32),
    Rem(u32),
}
impl From<Val> for ValKey {
    fn from(value: Val) -> Self {
        match value {
            Val::Auto => Self::Auto,
            Val::Px(value) => Self::Px(value.to_bits()),
            Val::Percent(value) => Self::Percent(value.to_bits()),
            Val::Vw(value) => Self::Vw(value.to_bits()),
            Val::Vh(value) => Self::Vh(value.to_bits()),
            Val::VMin(value) => Self::VMin(value.to_bits()),
            Val::VMax(value) => Self::VMax(value.to_bits()),
            Val::Em(value) => Self::Em(value.to_bits()),
            Val::Rem(value) => Self::Rem(value.to_bits()),
        }
    }
}
impl SplitterLayoutError {
    pub(crate) fn at(self, entity: Entity, part: &'static str) -> Self {
        Self::AtPart {
            entity,
            part,
            source: Box::new(self),
        }
    }
    pub(crate) fn key(&self) -> LayoutKey {
        match self {
            Self::AtPart {
                entity,
                part,
                source,
            } => LayoutKey::AtPart(*entity, part, Box::new(source.key())),
            Self::MissingPart { entity, part } => LayoutKey::Missing(*entity, part),
            Self::Query { source, part } => {
                let (entity, category) = match source {
                    QueryEntityError::QueryDoesNotMatch(entity, _) => {
                        (*entity, QueryFailure::Missing)
                    }
                    QueryEntityError::NotSpawned(source) => {
                        (source.entity(), QueryFailure::NotSpawned)
                    }
                    QueryEntityError::AliasedMutability(entity) => (*entity, QueryFailure::Aliased),
                };
                LayoutKey::Query(entity, category, part)
            }
            Self::InvalidParent {
                entity,
                expected,
                actual,
            } => LayoutKey::Parent(*entity, *expected, *actual),
            Self::ConstraintResolution { value, source } => {
                LayoutKey::Resolution((*value).into(), *source)
            }
            Self::InvalidConstraint { value } => LayoutKey::Constraint((*value).into()),
            Self::NoHandleSpace { a, b, thickness } => LayoutKey::HandleSpace([
                a[0].to_bits(),
                a[1].to_bits(),
                b[0].to_bits(),
                b[1].to_bits(),
                thickness.to_bits(),
            ]),
            Self::ConflictingBounds { min, max } => LayoutKey::Bounds(min.to_bits(), max.to_bits()),
            Self::ConflictingPanes { a, b, base } => LayoutKey::Panes(
                [a[0].into(), a[1].into(), b[0].into(), b[1].into()],
                base.to_bits(),
            ),
            Self::LayoutUnavailable => LayoutKey::Unavailable,
        }
    }
}
