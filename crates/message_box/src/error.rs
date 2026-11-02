use crate::WidgetryMessageBoxResult;
use bevy::prelude::*;
use bevy_widgetry_diagnostics::{ErrorFrame, WidgetryFailure};
use std::{error::Error, fmt};

pub type MessageBoxFailure = WidgetryFailure<MessageBoxError>;
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum MessageBoxFault {
    MissingPart {
        entity: Entity,
        component: &'static str,
    },
    ManagedColors {
        entity: Entity,
        owner: Entity,
    },
}
#[derive(Debug)]
pub enum MessageBoxError {
    Resolve {
        root: Entity,
        result: WidgetryMessageBoxResult,
        missing: &'static str,
    },
    Structure {
        fault: MessageBoxFault,
    },
    Core {
        source: Box<bevy_widgetry_core::diagnostics::CoreError>,
    },
    Button {
        source: Box<bevy_widgetry_button::ButtonError>,
    },
}
impl fmt::Display for MessageBoxError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Resolve { .. } => "MessageBox 决议目标或状态失效",
            Self::Structure { .. } => "MessageBox 受管结构失效",
            Self::Core { .. } => "MessageBox 配色配置无效",
            Self::Button { .. } => "MessageBox 结果按钮配色失败",
        })
    }
}
impl Error for MessageBoxError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Core { source } => Some(source.as_ref()),
            Self::Button { source } => Some(source.as_ref()),
            _ => None,
        }
    }
}
pub(crate) fn frame(
    world: &World,
    root: Entity,
    entity: Entity,
    operation: &'static str,
) -> ErrorFrame {
    ErrorFrame::new(operation)
        .field("message_box", root)
        .field("entity", entity)
        .field(
            "native",
            format_args!(
                "{:?}",
                bevy_widgetry_window::widgetry_window_target(world, root)
            ),
        )
        .field(
            "camera",
            format_args!(
                "{:?}",
                world.get::<UiTargetCamera>(root).map(|camera| camera.0)
            ),
        )
}
