use crate::assets::GalleryCursorAnimation;
use bevy::{prelude::*, window::SystemCursorIcon};
use bevy_widgetry::cursor::{CursorSemantic, CursorStyle, WidgetryCursorHint, WidgetryCursorTheme};

pub(crate) fn kamuri() -> Result<WidgetryCursorTheme> {
    WidgetryCursorTheme::new(GalleryCursorAnimation::ALL.into_iter().map(|animation| {
        (
            animation.semantic(),
            CursorStyle::Animation(animation.definition()),
        )
    }))
    .map_err(|error| error.into_bevy_error())
}

#[cfg(test)]
mod tests;
impl GalleryCursorAnimation {
    fn semantic(self) -> CursorSemantic {
        match self {
            Self::Default => CursorSemantic::System(SystemCursorIcon::Default),
            Self::Pointer => CursorSemantic::System(SystemCursorIcon::Pointer),
            Self::ContextMenu => CursorSemantic::System(SystemCursorIcon::ContextMenu),
            Self::Help => CursorSemantic::System(SystemCursorIcon::Help),
            Self::Text => CursorSemantic::System(SystemCursorIcon::Text),
            Self::VerticalText => CursorSemantic::System(SystemCursorIcon::VerticalText),
            Self::Cell => CursorSemantic::System(SystemCursorIcon::Cell),
            Self::Crosshair => CursorSemantic::System(SystemCursorIcon::Crosshair),
            Self::Grab => CursorSemantic::System(SystemCursorIcon::Grab),
            Self::Grabbing => CursorSemantic::System(SystemCursorIcon::Grabbing),
            Self::Move => CursorSemantic::System(SystemCursorIcon::Move),
            Self::Copy => CursorSemantic::System(SystemCursorIcon::Copy),
            Self::Alias => CursorSemantic::System(SystemCursorIcon::Alias),
            Self::NoDrop => CursorSemantic::System(SystemCursorIcon::NoDrop),
            Self::EResize => CursorSemantic::System(SystemCursorIcon::EResize),
            Self::NResize => CursorSemantic::System(SystemCursorIcon::NResize),
            Self::NeResize => CursorSemantic::System(SystemCursorIcon::NeResize),
            Self::NwResize => CursorSemantic::System(SystemCursorIcon::NwResize),
            Self::SResize => CursorSemantic::System(SystemCursorIcon::SResize),
            Self::SeResize => CursorSemantic::System(SystemCursorIcon::SeResize),
            Self::SwResize => CursorSemantic::System(SystemCursorIcon::SwResize),
            Self::WResize => CursorSemantic::System(SystemCursorIcon::WResize),
            Self::EwResize => CursorSemantic::System(SystemCursorIcon::EwResize),
            Self::NsResize => CursorSemantic::System(SystemCursorIcon::NsResize),
            Self::NeswResize => CursorSemantic::System(SystemCursorIcon::NeswResize),
            Self::NwseResize => CursorSemantic::System(SystemCursorIcon::NwseResize),
            Self::ColResize => CursorSemantic::System(SystemCursorIcon::ColResize),
            Self::RowResize => CursorSemantic::System(SystemCursorIcon::RowResize),
            Self::AllScroll => CursorSemantic::System(SystemCursorIcon::AllScroll),
            Self::ZoomIn => CursorSemantic::System(SystemCursorIcon::ZoomIn),
            Self::ZoomOut => CursorSemantic::System(SystemCursorIcon::ZoomOut),
            Self::Progress => CursorSemantic::System(SystemCursorIcon::Progress),
            Self::Wait => CursorSemantic::System(SystemCursorIcon::Wait),
            Self::NotAllowed => CursorSemantic::System(SystemCursorIcon::NotAllowed),
            Self::AddSelection => CursorSemantic::Hint(WidgetryCursorHint::AddSelection),
            Self::RemoveSelection => CursorSemantic::Hint(WidgetryCursorHint::RemoveSelection),
            Self::RangeSelection => CursorSemantic::Hint(WidgetryCursorHint::RangeSelection),
            Self::DragSort => CursorSemantic::Hint(WidgetryCursorHint::DragSort),
            Self::InsertDrop => CursorSemantic::Hint(WidgetryCursorHint::InsertDrop),
            Self::ReplaceDrop => CursorSemantic::Hint(WidgetryCursorHint::ReplaceDrop),
            Self::HorizontalValueAdjust => {
                CursorSemantic::Hint(WidgetryCursorHint::HorizontalValueAdjust)
            }
            Self::VerticalValueAdjust => {
                CursorSemantic::Hint(WidgetryCursorHint::VerticalValueAdjust)
            }
            Self::PanReady => CursorSemantic::Hint(WidgetryCursorHint::PanReady),
            Self::Panning => CursorSemantic::Hint(WidgetryCursorHint::Panning),
            Self::ReadOnly => CursorSemantic::Hint(WidgetryCursorHint::ReadOnly),
        }
    }
}
