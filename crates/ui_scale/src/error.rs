use bevy_widgetry_diagnostics::WidgetryFailure;
use std::{error::Error, fmt};

pub type UiScaleFailure = WidgetryFailure<UiScaleError>;

#[derive(Debug)]
pub enum UiScaleError {
    InvalidScale { requested: f32 },
}

impl fmt::Display for UiScaleError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidScale { requested } => {
                write!(f, "应用缩放必须是有限正数：{requested}")
            }
        }
    }
}

impl Error for UiScaleError {}
