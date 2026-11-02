use bevy_widgetry_diagnostics::WidgetryFailure;
use std::{error::Error, fmt};
use tray_icon::TrayIconId;

pub type TrayFailure = WidgetryFailure<TrayError>;

#[derive(Debug)]
pub enum TrayError {
    Decode {
        source: Box<image::ImageError>,
    },
    MissingEventLoop,
    Rgba {
        width: u32,
        height: u32,
        source: tray_icon::BadIcon,
    },
    Create {
        source: tray_icon::Error,
    },
    Register {
        id: TrayIconId,
        source: tray_icon::Error,
    },
    ClickOverflow {
        id: TrayIconId,
        capacity: usize,
    },
    Wake {
        id: TrayIconId,
        source: Box<dyn Error + Send + Sync>,
    },
    RouteLock,
}

impl fmt::Display for TrayError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Decode { .. } => "托盘 ICO 图标解码失败",
            Self::MissingEventLoop => "创建托盘需要 WinitPlugin",
            Self::Rgba { .. } => "托盘 RGBA 图标转换失败",
            Self::Create { .. } => "原生托盘图标创建失败",
            Self::Register { .. } => "原生托盘图标注册确认失败",
            Self::ClickOverflow { .. } => "托盘点击队列已满，拒绝本次点击",
            Self::Wake { .. } => "托盘点击唤醒失败",
            Self::RouteLock => "托盘事件路由锁中毒",
        })
    }
}

impl Error for TrayError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Decode { source } => Some(source.as_ref()),
            Self::Rgba { source, .. } => Some(source),
            Self::Create { source } | Self::Register { source, .. } => Some(source),
            Self::Wake { source, .. } => Some(source.as_ref()),
            _ => None,
        }
    }
}
