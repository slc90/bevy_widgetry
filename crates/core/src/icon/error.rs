use bevy::{
    asset::{AssetLoadError, AssetPath},
    prelude::*,
};
use bevy_widgetry_diagnostics::{ErrorFrame, WidgetryFailure};
use std::{error::Error, fmt, io, sync::Arc};

pub type IconFailure = WidgetryFailure<IconError>;

#[derive(Debug)]
pub enum IconError {
    EmptyPath {
        entity: Option<Entity>,
        path: Box<AssetPath<'static>>,
    },
    InvalidTarget {
        entity: Entity,
        missing: &'static str,
        path: Option<Box<AssetPath<'static>>>,
    },
    ManagedColor {
        entity: Entity,
        owner: Entity,
    },
    InvalidColor {
        entity: Option<Entity>,
        state: &'static str,
        color: Color,
    },
    MissingAssetServer {
        entity: Entity,
        path: Box<AssetPath<'static>>,
    },
    MissingImageNode {
        entity: Entity,
        image_entity: Entity,
    },
    Read {
        path: Box<AssetPath<'static>>,
        source: io::Error,
    },
    Parse {
        path: Box<AssetPath<'static>>,
        source: resvg::usvg::Error,
    },
    AssetLoad {
        entity: Entity,
        path: Option<Box<AssetPath<'static>>>,
        source: Arc<AssetLoadError>,
    },
    Rasterize {
        entity: Entity,
        path: Option<Box<AssetPath<'static>>>,
        requested_size: Option<UVec2>,
        width: u32,
        height: u32,
        scale: f32,
    },
}

impl fmt::Display for IconError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyPath { .. } => f.write_str("Icon 初始化路径不能为空"),
            Self::InvalidTarget { missing, .. } => write!(f, "Icon 更新目标不存在或缺失 {missing}"),
            Self::ManagedColor { .. } => f.write_str("Icon 颜色属于托管部件"),
            Self::InvalidColor { .. } => f.write_str("Icon 颜色分量必须有限且 alpha 位于 [0,1]"),
            Self::MissingAssetServer { .. } => f.write_str("Icon SVG 更新缺失 AssetServer"),
            Self::MissingImageNode { .. } => f.write_str("Icon 缺失内部 ImageNode"),
            Self::Read { .. } => f.write_str("SVG asset 读取失败"),
            Self::Parse { .. } => f.write_str("SVG asset 解析失败"),
            Self::AssetLoad { .. } => f.write_str("Icon SVG 加载失败"),
            Self::Rasterize { width, height, .. } => {
                write!(f, "图标像素缓冲区创建失败: {width}x{height}")
            }
        }
    }
}

impl Error for IconError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Read { source, .. } => Some(source),
            Self::Parse { source, .. } => Some(source),
            Self::AssetLoad { source, .. } => Some(source.as_ref()),
            _ => None,
        }
    }
}

#[track_caller]
pub(super) fn failure(kind: IconError, operation: &'static str) -> IconFailure {
    let mut context = ErrorFrame::new(operation);
    match &kind {
        IconError::EmptyPath { entity, path } => {
            context = context.field("path", path);
            if let Some(entity) = entity {
                context = context.field("entity", entity);
            }
        }
        IconError::Read { path, .. } | IconError::Parse { path, .. } => {
            context = context.field("path", path)
        }
        IconError::MissingAssetServer { entity, path } => {
            context = context.field("entity", entity).field("path", path)
        }
        IconError::AssetLoad { entity, path, .. } | IconError::Rasterize { entity, path, .. } => {
            context = context.field("entity", entity);
            if let Some(path) = path {
                context = context.field("path", path);
            }
        }
        IconError::InvalidTarget {
            entity,
            missing,
            path,
        } => {
            context = context.field("entity", entity).field("missing", missing);
            if let Some(path) = path {
                context = context.field("path", path);
            }
        }
        IconError::ManagedColor { entity, owner } => {
            context = context.field("entity", entity).field("owner", owner)
        }
        IconError::MissingImageNode {
            entity,
            image_entity,
        } => {
            context = context
                .field("entity", entity)
                .field("image_entity", image_entity)
        }
        IconError::InvalidColor {
            entity,
            state,
            color,
        } => {
            context = context
                .field("state", state)
                .field("color", format_args!("{color:?}"));
            if let Some(entity) = entity {
                context = context.field("entity", entity);
            }
        }
    }
    if let IconError::Rasterize {
        width,
        height,
        scale,
        requested_size,
        ..
    } = &kind
    {
        context = context
            .field("width", width)
            .field("height", height)
            .field("scale", scale)
            .field("requested_size", format_args!("{requested_size:?}"));
    }
    IconFailure::new(kind, context)
}
