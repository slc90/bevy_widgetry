use bevy::{prelude::*, window::SystemCursorIcon};
use std::collections::HashMap;

#[derive(Resource, Default, Clone, Debug, PartialEq, Eq, Reflect)]
#[reflect(Resource, Default)]
pub struct WidgetryCursorTheme {
    styles: HashMap<CursorSemantic, CursorStyle>,
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
    Animation(crate::CursorAnimation),
}

#[derive(Debug)]
pub enum CursorThemeError {
    EmptyAnimation,
    InvalidFrameDuration {
        frame: usize,
    },
    AnimationDurationOverflow,
    InvalidAtlasSize {
        size: UVec2,
    },
    UnexpectedAtlasSize {
        expected: UVec2,
        actual: UVec2,
    },
    InconsistentFrameSize {
        frame: usize,
        expected: UVec2,
        actual: UVec2,
    },
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
            Self::EmptyAnimation => f.write_str("Cursor 动画帧不能为空"),
            Self::InvalidFrameDuration { .. } => f.write_str("Cursor 动画帧时长必须大于零"),
            Self::AnimationDurationOverflow => f.write_str("Cursor 动画总时长溢出"),
            Self::InvalidAtlasSize { .. } => f.write_str("Cursor 图集尺寸必须大于零"),
            Self::UnexpectedAtlasSize { .. } => f.write_str("Cursor 图集实际尺寸与定义不一致"),
            Self::InconsistentFrameSize { .. } => f.write_str("Cursor 动画帧画布尺寸必须一致"),
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
    pub fn new(
        styles: impl IntoIterator<Item = (CursorSemantic, CursorStyle)>,
    ) -> Result<Self, CursorThemeFailure> {
        let styles: HashMap<_, _> = styles.into_iter().collect();
        for (semantic, style) in &styles {
            let result = match style {
                CursorStyle::System(_) => Ok(()),
                CursorStyle::Image(image) => validate_image(image),
                CursorStyle::Animation(animation) => animation.validate(),
            };
            result.map_err(|error| {
                CursorThemeFailure::new(
                    error,
                    bevy_widgetry_diagnostics::ErrorFrame::new("cursor.theme")
                        .field("semantic", format_args!("{semantic:?}"))
                        .field("style", format_args!("{style:?}")),
                )
            })?;
        }
        Ok(Self { styles })
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

pub(crate) fn validate_source(source: &CursorImageSource) -> Result<(), CursorThemeError> {
    if let CursorImageSource::Path(path) = source {
        if path.trim().is_empty() {
            return Err(CursorThemeError::EmptyImagePath);
        }
        bevy::asset::AssetPath::try_parse(path).map_err(|source| {
            CursorThemeError::InvalidImagePath {
                path: path.clone(),
                source,
            }
        })?;
    }
    Ok(())
}

pub(crate) fn validate_image(image: &CursorImage) -> Result<(), CursorThemeError> {
    validate_source(&image.source)?;
    if let Some(rect) = image.rect {
        if !rect.max.cmpgt(rect.min).all() {
            return Err(CursorThemeError::InvalidRegion { rect });
        }
        validate_hotspot(image.hotspot, rect.size())?;
    }
    Ok(())
}

pub(crate) fn validate_hotspot(hotspot: (u16, u16), size: UVec2) -> Result<(), CursorThemeError> {
    if u32::from(hotspot.0) >= size.x || u32::from(hotspot.1) >= size.y {
        return Err(CursorThemeError::InvalidHotspot { hotspot, size });
    }
    Ok(())
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
}
