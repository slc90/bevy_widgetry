mod http;
mod progress;

use bevy::prelude::*;
use bevy_brp_extras::BrpExtrasPlugin;
use std::time::Duration;

use self::http::GalleryRemoteHttpPlugin;

/// 为 Gallery 装配 Extras methods 与请求入队后主动 wake 的 HTTP transport。
pub(crate) struct GalleryBrpPlugin {
    /// 普通 HTTP 请求等待 BRP 最终结果的最长时间。
    pub(crate) request_deadline: Duration,
}

impl Default for GalleryBrpPlugin {
    fn default() -> Self {
        Self {
            request_deadline: Duration::from_secs(30),
        }
    }
}

impl Plugin for GalleryBrpPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((
            BrpExtrasPlugin::without_http_transport(),
            GalleryRemoteHttpPlugin::new(self.request_deadline),
        ));
    }
}
