macro_rules! asset_data {
    ($path:literal) => {
        // Paths are used by src/assets.rs, but include_bytes! resolves them relative to src/assets/constants.rs.
        // Return to src before using the same path so embedded locations and load paths agree.
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
pub(super) const SPLASH_FRAMES: [(&str, &[u8]); 40] = [
    asset_data!("assets/splash/frames/frame_000.png"),
    asset_data!("assets/splash/frames/frame_001.png"),
    asset_data!("assets/splash/frames/frame_002.png"),
    asset_data!("assets/splash/frames/frame_003.png"),
    asset_data!("assets/splash/frames/frame_004.png"),
    asset_data!("assets/splash/frames/frame_005.png"),
    asset_data!("assets/splash/frames/frame_006.png"),
    asset_data!("assets/splash/frames/frame_007.png"),
    asset_data!("assets/splash/frames/frame_008.png"),
    asset_data!("assets/splash/frames/frame_009.png"),
    asset_data!("assets/splash/frames/frame_010.png"),
    asset_data!("assets/splash/frames/frame_011.png"),
    asset_data!("assets/splash/frames/frame_012.png"),
    asset_data!("assets/splash/frames/frame_013.png"),
    asset_data!("assets/splash/frames/frame_014.png"),
    asset_data!("assets/splash/frames/frame_015.png"),
    asset_data!("assets/splash/frames/frame_016.png"),
    asset_data!("assets/splash/frames/frame_017.png"),
    asset_data!("assets/splash/frames/frame_018.png"),
    asset_data!("assets/splash/frames/frame_019.png"),
    asset_data!("assets/splash/frames/frame_020.png"),
    asset_data!("assets/splash/frames/frame_021.png"),
    asset_data!("assets/splash/frames/frame_022.png"),
    asset_data!("assets/splash/frames/frame_023.png"),
    asset_data!("assets/splash/frames/frame_024.png"),
    asset_data!("assets/splash/frames/frame_025.png"),
    asset_data!("assets/splash/frames/frame_026.png"),
    asset_data!("assets/splash/frames/frame_027.png"),
    asset_data!("assets/splash/frames/frame_028.png"),
    asset_data!("assets/splash/frames/frame_029.png"),
    asset_data!("assets/splash/frames/frame_030.png"),
    asset_data!("assets/splash/frames/frame_031.png"),
    asset_data!("assets/splash/frames/frame_032.png"),
    asset_data!("assets/splash/frames/frame_033.png"),
    asset_data!("assets/splash/frames/frame_034.png"),
    asset_data!("assets/splash/frames/frame_035.png"),
    asset_data!("assets/splash/frames/frame_036.png"),
    asset_data!("assets/splash/frames/frame_037.png"),
    asset_data!("assets/splash/frames/frame_038.png"),
    asset_data!("assets/splash/frames/frame_039.png"),
];
