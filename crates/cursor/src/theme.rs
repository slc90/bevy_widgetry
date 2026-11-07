use bevy::{prelude::*, window::SystemCursorIcon};
use std::collections::HashMap;

#[derive(Resource, Default, Clone, Debug, PartialEq, Eq, Reflect)]
#[reflect(Resource, Default)]
pub struct WidgetryCursorTheme {
    styles: HashMap<CursorSemantic, CursorStyle>,
    #[cfg(feature = "kamuri_cursor")]
    kamuri: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Reflect)]
pub enum CursorSemantic {
    System(SystemCursorIcon),
    Hint(WidgetryCursorHint),
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, Reflect)]
pub enum CursorImageSource {
    Path(String),
    Handle(Handle<Image>),
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, Reflect)]
pub struct CursorImage {
    pub source: CursorImageSource,
    pub rect: Option<URect>,
    pub hotspot: (u16, u16),
}

#[derive(Clone, Debug, PartialEq, Eq, Reflect)]
pub enum CursorStyle {
    System(SystemCursorIcon),
    Image(CursorImage),
}

#[derive(Debug)]
pub enum CursorThemeError {
    EmptyImagePath,
    InvalidImagePath {
        path: String,
        source: bevy::asset::ParseAssetPathError,
    },
    InvalidRegion {
        rect: URect,
    },
    UnavailableRegion {
        rect: URect,
        size: UVec2,
    },
    InvalidHotspot {
        hotspot: (u16, u16),
        size: UVec2,
    },
}
impl std::fmt::Display for CursorThemeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::EmptyImagePath => f.write_str("Cursor 图像路径不能为空"),
            Self::InvalidImagePath { path, source } => {
                write!(f, "Cursor 图像路径无效: {path}: {source}")
            }
            Self::InvalidRegion { .. } => f.write_str("Cursor 图像区域必须为正数尺寸"),
            Self::UnavailableRegion { .. } => f.write_str("Cursor 图像区域超出实际图像范围"),
            Self::InvalidHotspot { .. } => f.write_str("Cursor 热点必须位于图像区域内"),
        }
    }
}
impl std::error::Error for CursorThemeError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::InvalidImagePath { source, .. } => Some(source),
            _ => None,
        }
    }
}
pub type CursorThemeFailure = bevy_widgetry_diagnostics::WidgetryFailure<CursorThemeError>;

impl WidgetryCursorTheme {
    pub fn system() -> Self {
        Self::default()
    }
    #[cfg(feature = "kamuri_cursor")]
    pub fn kamuri() -> Self {
        Self {
            kamuri: true,
            ..default()
        }
    }
    pub fn new(
        styles: impl IntoIterator<Item = (CursorSemantic, CursorStyle)>,
    ) -> Result<Self, CursorThemeFailure> {
        let styles: HashMap<_, _> = styles.into_iter().collect();
        for (semantic, style) in &styles {
            if let CursorStyle::Image(image) = style {
                if let CursorImageSource::Path(path) = &image.source
                    && let Err(source) = bevy::asset::AssetPath::try_parse(path)
                {
                    return Err(CursorThemeFailure::new(
                        CursorThemeError::InvalidImagePath {
                            path: path.clone(),
                            source,
                        },
                        bevy_widgetry_diagnostics::ErrorFrame::new("cursor.theme")
                            .field("semantic", format_args!("{semantic:?}")),
                    ));
                }
                let invalid = match &image.source {
                    CursorImageSource::Path(path) if path.trim().is_empty() => {
                        Some(CursorThemeError::EmptyImagePath)
                    }
                    _ => match image.rect {
                        Some(rect) if !rect.max.cmpgt(rect.min).all() => {
                            Some(CursorThemeError::InvalidRegion { rect })
                        }
                        Some(rect)
                            if u32::from(image.hotspot.0) >= rect.width()
                                || u32::from(image.hotspot.1) >= rect.height() =>
                        {
                            Some(CursorThemeError::InvalidHotspot {
                                hotspot: image.hotspot,
                                size: rect.size(),
                            })
                        }
                        _ => None,
                    },
                };
                if let Some(error) = invalid {
                    return Err(CursorThemeFailure::new(
                        error,
                        bevy_widgetry_diagnostics::ErrorFrame::new("cursor.theme")
                            .field("semantic", format_args!("{semantic:?}"))
                            .field("image", format_args!("{image:?}")),
                    ));
                }
            }
        }
        Ok(Self {
            styles,
            ..default()
        })
    }
    pub fn styles(&self) -> &HashMap<CursorSemantic, CursorStyle> {
        &self.styles
    }
    pub fn is_system(&self) -> bool {
        self == &Self::system()
    }
    pub fn set_in_world(world: &mut World, theme: Self) -> bool {
        if world.get_resource::<Self>() == Some(&theme) {
            return false;
        }
        world.insert_resource(theme);
        true
    }
    pub fn set(commands: &mut Commands, theme: Self) {
        commands.queue(move |world: &mut World| {
            Self::set_in_world(world, theme);
        });
    }
}

#[derive(Component, Default, Clone, Copy, Debug, Reflect)]
#[reflect(Component, Default)]
pub struct WidgetryCursorLoading;

#[derive(Component, Clone, Copy, Debug, PartialEq, Eq, Hash, Reflect)]
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

    #[cfg(feature = "kamuri_cursor")]
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
