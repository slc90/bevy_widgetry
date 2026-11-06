use bevy::prelude::Entity;
use bevy_widgetry_diagnostics::{ErrorFrame, WidgetryFailure};
use std::{error::Error, fmt};
pub type OpenUrlFailure = WidgetryFailure<OpenUrlError>;

#[derive(Debug)]
pub enum OpenUrlRejection {
    UnsafeCharacters,
    Incomplete,
    UnsupportedScheme,
    MissingHost,
    Parse(url::ParseError),
    Syntax(url::SyntaxViolation),
}

impl fmt::Display for OpenUrlRejection {
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

impl Error for OpenUrlRejection {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Parse(source) => Some(source),
            _ => None,
        }
    }
}

#[derive(Debug)]
pub enum OpenUrlError {
    InvalidTarget {
        root: Entity,
    },
    MissingPart {
        root: Entity,
        part: &'static str,
    },
    MissingResource {
        root: Entity,
        resource: &'static str,
    },
    InvalidUrl {
        root: Entity,
        url: String,
        reason: OpenUrlRejection,
    },
    OpenBrowser {
        root: Entity,
        url: String,
        source: winsafe::co::ERROR,
    },
}
impl fmt::Display for OpenUrlError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidTarget { .. } => f.write_str("OpenUrl 目标不存在或类型不匹配"),
            Self::MissingPart { part, .. } => write!(f, "OpenUrl 缺失必要能力 {part}"),
            Self::MissingResource { resource, .. } => write!(f, "OpenUrl 缺失运行资源 {resource}"),
            Self::InvalidUrl { .. } => f.write_str("OpenUrl URL 无效"),
            Self::OpenBrowser { .. } => f.write_str("请求系统浏览器打开链接失败"),
        }
    }
}
impl Error for OpenUrlError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::InvalidUrl { reason, .. } => Some(reason),
            Self::OpenBrowser { source, .. } => Some(source),
            _ => None,
        }
    }
}
#[track_caller]
pub(crate) fn failure(kind: OpenUrlError, operation: &'static str) -> OpenUrlFailure {
    let context = match &kind {
        OpenUrlError::InvalidTarget { root } => ErrorFrame::new(operation).field("root", root),
        OpenUrlError::MissingPart { root, part } => ErrorFrame::new(operation)
            .field("root", root)
            .field("part", part),
        OpenUrlError::MissingResource { root, resource } => ErrorFrame::new(operation)
            .field("root", root)
            .field("resource", resource),
        OpenUrlError::InvalidUrl { root, url, .. }
        | OpenUrlError::OpenBrowser { root, url, .. } => ErrorFrame::new(operation)
            .field("root", root)
            .field("url", url),
    };
    OpenUrlFailure::new(kind, context)
}
