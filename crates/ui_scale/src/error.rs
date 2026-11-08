use bevy_widgetry_diagnostics::WidgetryFailure;
use std::{error::Error, fmt};

pub type UiScaleFailure = WidgetryFailure<UiScaleError>;

#[derive(Debug)]
pub enum UiScaleError {
    InvalidScale {
        requested: f32,
    },
    WorkArea {
        window: bevy::prelude::Entity,
        point: bevy::prelude::IVec2,
        source: winsafe::co::ERROR,
    },
}

impl fmt::Display for UiScaleError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidScale { requested } => {
                write!(f, "应用缩放必须是有限正数：{requested}")
            }
            Self::WorkArea { window, point, .. } => {
                write!(f, "读取窗口 {window:?} 在 {point:?} 的工作区失败")
            }
        }
    }
}

impl Error for UiScaleError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::InvalidScale { .. } => None,
            Self::WorkArea { source, .. } => Some(source),
        }
    }
}
