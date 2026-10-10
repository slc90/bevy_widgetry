macro_rules! asset_data {
    ($path:literal) => {
        // 路径供 src/assets.rs 使用，include_bytes! 则相对当前 src/assets/constants.rs 解析。
        // 回到 src 后使用相同路径，避免嵌入位置与加载路径不一致。
        ($path, include_bytes!(concat!("../", $path)))
    };
}

pub(super) const EMBEDDED_SOURCE: &str = "embedded";
pub(super) const LOGO_ICON: (&str, &[u8]) = asset_data!("assets/icons/logo.svg");
pub(super) const BUTTON_STAR_ICON: (&str, &[u8]) = asset_data!("assets/icons/button_star.svg");
pub(super) const NATIVE_WINDOW_ICON: (&str, &[u8]) =
    asset_data!("assets/icons/widget_gallery_taskbar.png");
pub(super) const BASIC_REPLAY_WAVEFORM: (&str, &[u8]) =
    asset_data!("assets/waveform/basic_replay.wfrm");
pub(super) const WINDOW_BACKGROUND_1_IMAGE: (&str, &[u8]) =
    asset_data!("assets/pictures/background1.png");
pub(super) const WINDOW_BACKGROUND_2_IMAGE: (&str, &[u8]) =
    asset_data!("assets/pictures/background2.jpg");
