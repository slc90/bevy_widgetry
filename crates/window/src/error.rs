use bevy::{ecs::query::QuerySingleError, prelude::*};
use bevy_widgetry_diagnostics::WidgetryFailure;
use std::{error::Error, fmt};

pub type WindowFailure = WidgetryFailure<WindowError>;

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum WindowFault {
    MissingPart {
        entity: Entity,
        component: &'static str,
    },
    NativeProperties {
        native: Entity,
    },
    DuplicateBinding {
        entity: Entity,
        other_root: Entity,
        component: &'static str,
    },
    CameraTarget {
        camera: Entity,
        native: Entity,
    },
    TitleOutsideBar {
        title: Entity,
    },
    AmbiguousTitle {
        titles: Vec<Entity>,
    },
    ManagedColors {
        entity: Entity,
        owner: Entity,
    },
}

#[derive(Debug)]
pub enum WindowError {
    Structure {
        fault: WindowFault,
    },
    Core {
        source: Box<bevy_widgetry_core::diagnostics::CoreError>,
    },
    KeyboardSchedule {
        source: Box<bevy::ecs::schedule::ScheduleError>,
    },
    Adapter {
        backend: wgpu::Backend,
        source: wgpu::RequestAdapterError,
    },
    Device {
        adapter: Box<wgpu::AdapterInfo>,
        source: wgpu::RequestDeviceError,
    },
    IconDecode {
        path: String,
        format: image::ImageFormat,
        source: Box<image::ImageError>,
    },
    IconCreate {
        path: String,
        width: u32,
        height: u32,
        bytes: usize,
        source: winit::window::BadIcon,
    },
    PrimaryWindow {
        entities: Vec<Entity>,
        source: QuerySingleError,
    },
}

impl fmt::Display for WindowError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Structure { .. } => "Window 结构或绑定失效",
            Self::Core { .. } => "Window 底层配置无效",
            Self::KeyboardSchedule { .. } => "窗口键盘分发替换失败",
            Self::Adapter { .. } => "透明 renderer 的 adapter 创建失败",
            Self::Device { .. } => "透明 renderer 的 device 创建失败",
            Self::IconDecode { .. } => "native window PNG 图标解码失败",
            Self::IconCreate { .. } => "native window 图标创建失败",
            Self::PrimaryWindow { .. } => "设置 native window 图标时无法确定主窗口",
        })
    }
}

impl Error for WindowError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        Some(match self {
            Self::Structure { .. } => return None,
            Self::Core { source } => source.as_ref(),
            Self::KeyboardSchedule { source } => source.as_ref(),
            Self::Adapter { source, .. } => source,
            Self::Device { source, .. } => source,
            Self::IconDecode { source, .. } => source.as_ref(),
            Self::IconCreate { source, .. } => source,
            Self::PrimaryWindow { source, .. } => source,
        })
    }
}
