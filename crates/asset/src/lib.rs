#[cfg(not(all(target_os = "windows", target_pointer_width = "64")))]
compile_error!("bevy_widgetry 仅支持 Windows 64 位 target");

mod constants;

use bevy::asset::io::embedded::{EmbeddedAssetRegistry, watched_path};
use bevy::asset::{AssetPath, embedded_path};
use bevy::prelude::*;
use bevy_widgetry_internal_log::widgetry_info;
use constants::{
    CHECKBOX_CHECK_ICON, CHECKBOX_INDETERMINATE_ICON, CHEVRON_DOWN_ICON, CHEVRON_UP_ICON,
    DEFAULT_FONT, EMBEDDED_SOURCE, FILE_DIALOG_BACK_ICON, FILE_DIALOG_FILE_ICON,
    FILE_DIALOG_FOLDER_ICON, FILE_DIALOG_FORWARD_ICON, FILE_DIALOG_REFRESH_ICON,
    FILE_DIALOG_UP_ICON, TREE_COLLAPSE_ICON, TREE_EXPAND_ICON, WINDOW_CLOSE_ICON,
    WINDOW_MAXIMIZE_ICON, WINDOW_MINIMIZE_ICON, WINDOW_RESTORE_ICON,
};
use constants::{KAMURI_POINTER_ATLAS, KAMURI_STATIC_ATLAS, KAMURI_WAIT_ATLAS};

pub struct WidgetryAssetPlugin;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum BuiltinShader {
    HoldButton,
    CanvasStraightAlpha,
}

impl BuiltinShader {
    pub fn path(self) -> AssetPath<'static> {
        let path = match self {
            Self::HoldButton => embedded_path!(constants::HOLD_BUTTON_SHADER.0),
            Self::CanvasStraightAlpha => embedded_path!(constants::CANVAS_STRAIGHT_ALPHA_SHADER.0),
        };
        AssetPath::from_path_buf(path).with_source(EMBEDDED_SOURCE)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum BuiltinIcon {
    PlaybackPlay,
    PlaybackPause,
    TableSort,
    TableFilter,
    TableFilterActive,
    FileDialogBack,
    FileDialogForward,
    FileDialogUp,
    FileDialogRefresh,
    FileDialogFolder,
    FileDialogFile,
    CheckboxCheck,
    CheckboxIndeterminate,
    ChevronDown,
    ChevronUp,
    TreeExpand,
    TreeCollapse,
    WindowClose,
    WindowMaximize,
    WindowMinimize,
    WindowRestore,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum BuiltinFont {
    Default,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum BuiltinCursorImage {
    KamuriPointerAtlas,
    KamuriWaitAtlas,
    KamuriStaticAtlas,
}

impl BuiltinCursorImage {
    pub fn atlas_layout(self) -> TextureAtlasLayout {
        match self {
            Self::KamuriPointerAtlas => {
                TextureAtlasLayout::from_grid(UVec2::splat(64), 4, 3, None, None)
            }
            Self::KamuriWaitAtlas => {
                TextureAtlasLayout::from_grid(UVec2::splat(64), 6, 4, None, None)
            }
            Self::KamuriStaticAtlas => {
                let mut layout = TextureAtlasLayout::from_grid(UVec2::splat(96), 8, 6, None, None);
                layout.textures.truncate(45);
                for (index, rect) in layout.textures.iter_mut().enumerate() {
                    rect.max =
                        rect.min + UVec2::splat(if matches!(index, 1 | 32) { 64 } else { 48 });
                }
                layout
            }
        }
    }

    pub fn path(self) -> AssetPath<'static> {
        let path = match self {
            Self::KamuriPointerAtlas => embedded_path!(KAMURI_POINTER_ATLAS.0),
            Self::KamuriWaitAtlas => embedded_path!(KAMURI_WAIT_ATLAS.0),
            Self::KamuriStaticAtlas => embedded_path!(KAMURI_STATIC_ATLAS.0),
        };
        AssetPath::from_path_buf(path).with_source(EMBEDDED_SOURCE)
    }
}

impl BuiltinFont {
    pub fn path(self) -> AssetPath<'static> {
        let path = match self {
            Self::Default => embedded_path!(DEFAULT_FONT.0),
        };
        AssetPath::from_path_buf(path).with_source(EMBEDDED_SOURCE)
    }
}

impl BuiltinIcon {
    pub fn path(self) -> AssetPath<'static> {
        let path = match self {
            Self::PlaybackPlay => embedded_path!(constants::PLAYBACK_PLAY_ICON.0),
            Self::PlaybackPause => embedded_path!(constants::PLAYBACK_PAUSE_ICON.0),
            Self::TableSort => embedded_path!(constants::TABLE_SORT_ICON.0),
            Self::TableFilter => embedded_path!(constants::TABLE_FILTER_ICON.0),
            Self::TableFilterActive => embedded_path!(constants::TABLE_FILTER_ACTIVE_ICON.0),
            Self::FileDialogBack => embedded_path!(FILE_DIALOG_BACK_ICON.0),
            Self::FileDialogForward => embedded_path!(FILE_DIALOG_FORWARD_ICON.0),
            Self::FileDialogUp => embedded_path!(FILE_DIALOG_UP_ICON.0),
            Self::FileDialogRefresh => embedded_path!(FILE_DIALOG_REFRESH_ICON.0),
            Self::FileDialogFolder => embedded_path!(FILE_DIALOG_FOLDER_ICON.0),
            Self::FileDialogFile => embedded_path!(FILE_DIALOG_FILE_ICON.0),
            Self::CheckboxCheck => embedded_path!(CHECKBOX_CHECK_ICON.0),
            Self::CheckboxIndeterminate => embedded_path!(CHECKBOX_INDETERMINATE_ICON.0),
            Self::ChevronDown => embedded_path!(CHEVRON_DOWN_ICON.0),
            Self::ChevronUp => embedded_path!(CHEVRON_UP_ICON.0),
            Self::TreeExpand => embedded_path!(TREE_EXPAND_ICON.0),
            Self::TreeCollapse => embedded_path!(TREE_COLLAPSE_ICON.0),
            Self::WindowClose => embedded_path!(WINDOW_CLOSE_ICON.0),
            Self::WindowMaximize => embedded_path!(WINDOW_MAXIMIZE_ICON.0),
            Self::WindowMinimize => embedded_path!(WINDOW_MINIMIZE_ICON.0),
            Self::WindowRestore => embedded_path!(WINDOW_RESTORE_ICON.0),
        };
        AssetPath::from_path_buf(path).with_source(EMBEDDED_SOURCE)
    }
}

impl Plugin for WidgetryAssetPlugin {
    fn build(&self, app: &mut App) {
        let embedded = app.world_mut().resource_mut::<EmbeddedAssetRegistry>();
        for (path, bytes) in [
            constants::PLAYBACK_PLAY_ICON,
            constants::PLAYBACK_PAUSE_ICON,
            constants::TABLE_SORT_ICON,
            constants::TABLE_FILTER_ICON,
            constants::TABLE_FILTER_ACTIVE_ICON,
            constants::HOLD_BUTTON_SHADER,
            constants::CANVAS_STRAIGHT_ALPHA_SHADER,
            FILE_DIALOG_BACK_ICON,
            FILE_DIALOG_FORWARD_ICON,
            FILE_DIALOG_UP_ICON,
            FILE_DIALOG_REFRESH_ICON,
            FILE_DIALOG_FOLDER_ICON,
            FILE_DIALOG_FILE_ICON,
            CHECKBOX_CHECK_ICON,
            CHECKBOX_INDETERMINATE_ICON,
            DEFAULT_FONT,
            WINDOW_CLOSE_ICON,
            WINDOW_MAXIMIZE_ICON,
            WINDOW_MINIMIZE_ICON,
            WINDOW_RESTORE_ICON,
            CHEVRON_DOWN_ICON,
            CHEVRON_UP_ICON,
            TREE_EXPAND_ICON,
            TREE_COLLAPSE_ICON,
            KAMURI_POINTER_ATLAS,
            KAMURI_WAIT_ATLAS,
            KAMURI_STATIC_ATLAS,
        ] {
            embedded.insert_asset(watched_path(file!(), path), &embedded_path!(path), bytes);
        }
        widgetry_info!("WidgetryAssetPlugin 注册完成");
    }
}
