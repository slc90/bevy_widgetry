macro_rules! asset_data {
    ($path:literal) => {
        ($path, include_bytes!($path))
    };
}

pub(super) const EMBEDDED_SOURCE: &str = "embedded";
pub(super) const DATE_PICKER_TODAY_ICON: (&str, &[u8]) =
    asset_data!("assets/icons/date_picker_today_brush.svg");
pub(super) const PLAYBACK_PLAY_ICON: (&str, &[u8]) = asset_data!("assets/icons/playback_play.svg");
pub(super) const PLAYBACK_PAUSE_ICON: (&str, &[u8]) =
    asset_data!("assets/icons/playback_pause.svg");
pub(super) const TABLE_SORT_ICON: (&str, &[u8]) = asset_data!("assets/icons/table_sort.svg");
pub(super) const TABLE_FILTER_ICON: (&str, &[u8]) = asset_data!("assets/icons/table_filter.svg");
pub(super) const TABLE_FILTER_ACTIVE_ICON: (&str, &[u8]) =
    asset_data!("assets/icons/table_filter_active.svg");
pub(super) const KAMURI_POINTER_ATLAS: (&str, &[u8]) =
    asset_data!("assets/cursors/kamuri/pointer_atlas_64.png");
pub(super) const KAMURI_WAIT_ATLAS: (&str, &[u8]) =
    asset_data!("assets/cursors/kamuri/wait_atlas_64.png");
pub(super) const KAMURI_STATIC_ATLAS: (&str, &[u8]) =
    asset_data!("assets/cursors/kamuri/static_atlas_96.png");
pub(super) const FILE_DIALOG_FOLDER_ICON: (&str, &[u8]) =
    asset_data!("assets/icons/file_dialog_folder.svg");
pub(super) const FILE_DIALOG_FILE_ICON: (&str, &[u8]) =
    asset_data!("assets/icons/file_dialog_file.svg");
pub(super) const DEFAULT_FONT: (&str, &[u8]) = asset_data!("assets/fonts/SmileySans-Oblique.ttf");
pub(super) const CHECKBOX_CHECK_ICON: (&str, &[u8]) =
    asset_data!("assets/icons/checkbox_check.svg");
pub(super) const CHECKBOX_INDETERMINATE_ICON: (&str, &[u8]) =
    asset_data!("assets/icons/checkbox_indeterminate.svg");
pub(super) const CHEVRON_DOWN_ICON: (&str, &[u8]) = asset_data!("assets/icons/chevron_down.svg");
pub(super) const CHEVRON_UP_ICON: (&str, &[u8]) = asset_data!("assets/icons/chevron_up.svg");
pub(super) const TREE_EXPAND_ICON: (&str, &[u8]) = asset_data!("assets/icons/tree_expand.svg");
pub(super) const TREE_COLLAPSE_ICON: (&str, &[u8]) = asset_data!("assets/icons/tree_collapse.svg");
pub(super) const WINDOW_CLOSE_ICON: (&str, &[u8]) = asset_data!("assets/icons/window_close.svg");
pub(super) const WINDOW_MAXIMIZE_ICON: (&str, &[u8]) =
    asset_data!("assets/icons/window_maximize.svg");
pub(super) const WINDOW_MINIMIZE_ICON: (&str, &[u8]) =
    asset_data!("assets/icons/window_minimize.svg");
pub(super) const WINDOW_RESTORE_ICON: (&str, &[u8]) =
    asset_data!("assets/icons/window_restore.svg");

pub(super) const FILE_DIALOG_BACK_ICON: (&str, &[u8]) =
    asset_data!("assets/icons/file_dialog_back.svg");

pub(super) const FILE_DIALOG_FORWARD_ICON: (&str, &[u8]) =
    asset_data!("assets/icons/file_dialog_forward.svg");

pub(super) const FILE_DIALOG_UP_ICON: (&str, &[u8]) =
    asset_data!("assets/icons/file_dialog_up.svg");

pub(super) const FILE_DIALOG_REFRESH_ICON: (&str, &[u8]) =
    asset_data!("assets/icons/file_dialog_refresh.svg");

pub(super) const HOLD_BUTTON_SHADER: (&str, &[u8]) = asset_data!("assets/shaders/hold_button.wesl");
pub(super) const CANVAS_STRAIGHT_ALPHA_SHADER: (&str, &[u8]) =
    asset_data!("assets/shaders/canvas_straight_alpha.wesl");
