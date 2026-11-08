use bevy::prelude::*;
use bevy_widgetry_diagnostics::{ErrorFrame, WidgetryFailure};
use std::{error::Error, fmt};

pub type SplashFailure = WidgetryFailure<SplashError>;

#[derive(Debug)]
pub enum SplashError {
    UnknownSplash {
        splash: Entity,
    },
    WindowMissing {
        window: Entity,
    },
    NativeHandleMissing {
        window: Entity,
    },
    NativeWindowQuery {
        window: Entity,
        source: winsafe::co::ERROR,
    },
    WindowTransition {
        window: Entity,
        disabled: bool,
        source: winsafe::co::HRESULT,
    },
    Presentation {
        window: Entity,
        source: winsafe::co::HRESULT,
    },
    PluginMissing,
    MainWindowMissing {
        main: Entity,
    },
    InvalidMainWindow {
        main: Entity,
    },
    InvalidSize {
        size: UVec2,
    },
    TooLarge {
        main: Entity,
        size: UVec2,
        monitor: IRect,
    },
    NativePosition {
        main: Entity,
        source: winit::error::NotSupportedError,
    },
}

impl fmt::Display for SplashError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnknownSplash { splash } => write!(f, "Splash 会话 {splash:?} 不存在"),
            Self::WindowMissing { window } => write!(f, "Splash 呈现窗口 {window:?} 已不存在"),
            Self::NativeHandleMissing { window } => {
                write!(f, "Splash 交接窗口 {window:?} 缺少有效原生句柄")
            }
            Self::NativeWindowQuery { window, source } => {
                write!(f, "查找 Splash 交接窗口 {window:?} 失败：{source}")
            }
            Self::WindowTransition {
                window,
                disabled,
                source,
            } => write!(
                f,
                "设置 Splash 交接窗口 {window:?} 的显示动画 disabled={disabled} 失败：{source}"
            ),
            Self::Presentation { window, source } => {
                write!(f, "等待窗口 {window:?} 的桌面呈现失败：{source}")
            }
            Self::PluginMissing => write!(f, "启动 Splash 前需要安装 SplashPlugin"),
            Self::MainWindowMissing { main } => write!(f, "Splash 关联的主窗口 {main:?} 不存在"),
            Self::InvalidMainWindow { main } => {
                write!(f, "Splash 不能将另一个 Splash 窗口 {main:?} 用作主窗口")
            }
            Self::InvalidSize { size } => write!(f, "Splash 物理尺寸必须非零：{size:?}"),
            Self::TooLarge {
                main,
                size,
                monitor,
            } => write!(
                f,
                "主窗口 {main:?} 所在显示器 {monitor:?} 无法容纳 Splash 物理尺寸 {size:?}"
            ),
            Self::NativePosition { main, source } => {
                write!(f, "读取 Splash 主窗口 {main:?} 的原生位置失败：{source}")
            }
        }
    }
}

impl Error for SplashError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Presentation { source, .. } => Some(source),
            Self::WindowTransition { source, .. } => Some(source),
            Self::NativeWindowQuery { source, .. } => Some(source),
            Self::NativePosition { source, .. } => Some(source),
            _ => None,
        }
    }
}

#[track_caller]
pub(crate) fn failure(kind: SplashError, operation: &'static str) -> SplashFailure {
    SplashFailure::new(kind, ErrorFrame::new(operation))
}
