use bevy::{prelude::*, window::SystemCursorIcon};

#[derive(Resource, Default, Clone, Copy, Debug, PartialEq, Eq, Reflect)]
#[reflect(Resource, Default)]
pub enum WidgetryCursorTheme {
    #[default]
    System,
    Kamuri,
}

#[derive(Component, Clone, Copy, Debug, PartialEq, Eq, Reflect)]
#[reflect(Component)]
pub enum WidgetryCursorHint {
    AddSelection,
    RemoveSelection,
    RangeSelection,
    DragSort,
    InsertDrop,
    ReplaceDrop,
    HorizontalValueAdjust,
    VerticalValueAdjust,
    PanReady,
    Panning,
    ReadOnly,
}

impl WidgetryCursorHint {
    pub fn system_fallback(self) -> SystemCursorIcon {
        match self {
            Self::AddSelection => SystemCursorIcon::Copy,
            Self::RemoveSelection => SystemCursorIcon::NotAllowed,
            Self::RangeSelection => SystemCursorIcon::Crosshair,
            Self::DragSort => SystemCursorIcon::NsResize,
            Self::InsertDrop => SystemCursorIcon::Copy,
            Self::ReplaceDrop => SystemCursorIcon::Move,
            Self::HorizontalValueAdjust => SystemCursorIcon::EwResize,
            Self::VerticalValueAdjust => SystemCursorIcon::NsResize,
            Self::PanReady => SystemCursorIcon::Grab,
            Self::Panning => SystemCursorIcon::Grabbing,
            Self::ReadOnly => SystemCursorIcon::Cell,
        }
    }
}

#[derive(Clone, PartialEq)]
pub(crate) enum CursorIntent {
    System(SystemCursorIcon),
    Hint(WidgetryCursorHint),
    Custom(bevy::window::CustomCursor),
}

impl Default for CursorIntent {
    fn default() -> Self {
        Self::System(SystemCursorIcon::Default)
    }
}

impl CursorIntent {
    pub(crate) fn from_entity(cursor: &bevy::picking::cursor::EntityCursor) -> Self {
        match cursor {
            bevy::picking::cursor::EntityCursor::System(icon) => Self::System(*icon),
            bevy::picking::cursor::EntityCursor::Custom(custom) => Self::Custom(custom.clone()),
        }
    }

    pub(crate) fn fallback(&self) -> bevy::window::CursorIcon {
        match self {
            Self::System(icon) => bevy::window::CursorIcon::System(*icon),
            Self::Hint(hint) => bevy::window::CursorIcon::System(hint.system_fallback()),
            Self::Custom(custom) => bevy::window::CursorIcon::Custom(custom.clone()),
        }
    }

    pub(crate) fn static_index(&self) -> Option<usize> {
        match self {
            Self::System(SystemCursorIcon::Default) => Some(0),
            Self::System(SystemCursorIcon::Pointer) => Some(1),
            Self::System(SystemCursorIcon::ContextMenu) => Some(2),
            Self::System(SystemCursorIcon::Help) => Some(3),
            Self::System(SystemCursorIcon::Text) => Some(4),
            Self::System(SystemCursorIcon::VerticalText) => Some(5),
            Self::System(SystemCursorIcon::Cell) => Some(6),
            Self::System(SystemCursorIcon::Crosshair) => Some(7),
            Self::System(SystemCursorIcon::Grab) => Some(8),
            Self::System(SystemCursorIcon::Grabbing) => Some(9),
            Self::System(SystemCursorIcon::Move) => Some(10),
            Self::System(SystemCursorIcon::Copy) => Some(11),
            Self::System(SystemCursorIcon::Alias) => Some(12),
            Self::System(SystemCursorIcon::NoDrop) => Some(13),
            Self::System(SystemCursorIcon::EResize) => Some(14),
            Self::System(SystemCursorIcon::NResize) => Some(15),
            Self::System(SystemCursorIcon::NeResize) => Some(16),
            Self::System(SystemCursorIcon::NwResize) => Some(17),
            Self::System(SystemCursorIcon::SResize) => Some(18),
            Self::System(SystemCursorIcon::SeResize) => Some(19),
            Self::System(SystemCursorIcon::SwResize) => Some(20),
            Self::System(SystemCursorIcon::WResize) => Some(21),
            Self::System(SystemCursorIcon::EwResize) => Some(22),
            Self::System(SystemCursorIcon::NsResize) => Some(23),
            Self::System(SystemCursorIcon::NeswResize) => Some(24),
            Self::System(SystemCursorIcon::NwseResize) => Some(25),
            Self::System(SystemCursorIcon::ColResize) => Some(26),
            Self::System(SystemCursorIcon::RowResize) => Some(27),
            Self::System(SystemCursorIcon::AllScroll) => Some(28),
            Self::System(SystemCursorIcon::ZoomIn) => Some(29),
            Self::System(SystemCursorIcon::ZoomOut) => Some(30),
            Self::System(SystemCursorIcon::Progress) => Some(31),
            Self::System(SystemCursorIcon::Wait) => Some(32),
            Self::System(SystemCursorIcon::NotAllowed) => Some(33),
            Self::Hint(WidgetryCursorHint::AddSelection) => Some(34),
            Self::Hint(WidgetryCursorHint::RemoveSelection) => Some(35),
            Self::Hint(WidgetryCursorHint::RangeSelection) => Some(36),
            Self::Hint(WidgetryCursorHint::DragSort) => Some(37),
            Self::Hint(WidgetryCursorHint::InsertDrop) => Some(38),
            Self::Hint(WidgetryCursorHint::ReplaceDrop) => Some(39),
            Self::Hint(WidgetryCursorHint::HorizontalValueAdjust) => Some(40),
            Self::Hint(WidgetryCursorHint::VerticalValueAdjust) => Some(41),
            Self::Hint(WidgetryCursorHint::PanReady) => Some(42),
            Self::Hint(WidgetryCursorHint::Panning) => Some(43),
            Self::Hint(WidgetryCursorHint::ReadOnly) => Some(44),
            Self::Custom(_) => None,
        }
    }
}
