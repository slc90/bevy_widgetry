macro_rules! asset_data {
    ($path:literal) => {
        ($path, include_bytes!($path))
    };
}

pub(super) const EMBEDDED_SOURCE: &str = "embedded";
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
