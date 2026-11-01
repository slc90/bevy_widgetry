use bevy::{ecs::system::SystemParamValidationError, prelude::Entity};
use bevy_widgetry_core::diagnostics::{CoreError, CoreFailure};
use bevy_widgetry_diagnostics::{ErrorFrame, WidgetryFailure};
use std::{error::Error, fmt};

pub type LinkButtonFailure = WidgetryFailure<LinkButtonError>;

#[derive(Debug)]
pub enum LinkUrlRejection {
    UnsafeCharacters,
    Incomplete,
    UnsupportedScheme,
    MissingHost,
    Parse(url::ParseError),
    Syntax(url::SyntaxViolation),
}

impl fmt::Display for LinkUrlRejection {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnsafeCharacters => f.write_str("URL 不能包含空白、控制字符或反斜杠"),
            Self::Incomplete => f.write_str("必须提供完整 HTTP / HTTPS URL"),
            Self::UnsupportedScheme => f.write_str("仅支持 HTTP / HTTPS URL"),
            Self::MissingHost => f.write_str("URL 缺少主机"),
            Self::Parse(_) => f.write_str("URL 解析失败"),
            Self::Syntax(violation) => f.write_str(violation.description()),
        }
    }
}

impl Error for LinkUrlRejection {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Parse(source) => Some(source),
            _ => None,
        }
    }
}

#[derive(Debug)]
pub enum LinkButtonError {
    InvalidTarget {
        root: Entity,
    },
    MissingPart {
        root: Entity,
        part: &'static str,
    },
    InvalidUrl {
        root: Entity,
        url: String,
        reason: LinkUrlRejection,
    },
    OpenBrowser {
        root: Entity,
        url: String,
        source: winsafe::co::ERROR,
    },
    ThemeQuery {
        source: SystemParamValidationError,
    },
    Core {
        source: CoreError,
    },
}

impl fmt::Display for LinkButtonError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidTarget { .. } => f.write_str("LinkButton 目标不存在或类型不匹配"),
            Self::MissingPart { part, .. } => write!(f, "LinkButton 缺失内部部件 {part}"),
            Self::InvalidUrl { .. } => f.write_str("LinkButton URL 无效"),
            Self::OpenBrowser { .. } => f.write_str("请求系统浏览器打开链接失败"),
            Self::ThemeQuery { .. } => f.write_str("LinkButton Theme 颜色查询失败"),
            Self::Core { .. } => f.write_str("LinkButton 共享状态操作失败"),
        }
    }
}

impl Error for LinkButtonError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::InvalidUrl { reason, .. } => Some(reason),
            Self::OpenBrowser { source, .. } => Some(source),
            Self::ThemeQuery { source } => Some(source),
            Self::Core { source } => Some(source),
            _ => None,
        }
    }
}

#[track_caller]
pub(crate) fn failure(kind: LinkButtonError, operation: &'static str) -> LinkButtonFailure {
    let mut context = ErrorFrame::new(operation);
    match &kind {
        LinkButtonError::InvalidTarget { root } => context = context.field("root", root),
        LinkButtonError::MissingPart { root, part } => {
            context = context.field("root", root).field("part", part)
        }
        LinkButtonError::InvalidUrl { root, url, .. }
        | LinkButtonError::OpenBrowser { root, url, .. } => {
            context = context.field("root", root).field("url", url)
        }
        _ => {}
    }
    LinkButtonFailure::new(kind, context)
}

pub(crate) fn core(error: CoreFailure, root: Entity, field: &'static str) -> LinkButtonFailure {
    error
        .map_kind(|source| LinkButtonError::Core { source })
        .with_context(
            ErrorFrame::new("link_button.validate_colors")
                .field("root", root)
                .field("field", field),
        )
}
