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
pub(super) const BASIC_REPLAY_WAVEFORM: (&str, &[u8]) =
    asset_data!("assets/waveform/basic_replay.wfrm");
pub(super) const WINDOW_BACKGROUND_1_IMAGE: (&str, &[u8]) =
    asset_data!("assets/pictures/background1.png");
pub(super) const WINDOW_BACKGROUND_2_IMAGE: (&str, &[u8]) =
    asset_data!("assets/pictures/background2.jpg");
pub(super) const SPLASH_BODY_OPEN: (&str, &[u8]) =
    asset_data!("assets/splash/kamuri/body_open.png");
pub(super) const SPLASH_BODY_CLOSED: (&str, &[u8]) =
    asset_data!("assets/splash/kamuri/body_closed.png");
pub(super) const SPLASH_ARM: (&str, &[u8]) = asset_data!("assets/splash/kamuri/arm.png");
pub(super) const SPLASH_RIBBON_UPPER: (&str, &[u8]) =
    asset_data!("assets/splash/kamuri/ribbon_upper.png");
pub(super) const SPLASH_RIBBON_LOWER: (&str, &[u8]) =
    asset_data!("assets/splash/kamuri/ribbon_lower.png");
pub(super) const SPLASH_LAYOUT: (&str, &[u8]) = asset_data!("assets/splash/kamuri/layer_map.json");
pub(super) const SPLASH_SHADER: (&str, &[u8]) = asset_data!("assets/splash/reveal.wesl");
