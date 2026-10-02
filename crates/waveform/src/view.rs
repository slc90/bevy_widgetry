use crate::{
    WaveformConfig, WaveformCursor, WaveformOutputLength, WaveformRuntime, WaveformSource,
    WaveformStyle,
};
use bevy::prelude::*;
use bevy_widgetry_core::scene::logged_error;
use bevy_widgetry_log::widgetry_error;
use std::sync::Arc;

/// BSN Waveform root；安装 WaveformRenderPlugin，宿主负责正常 Bevy UI/2D render plugins。
/// renderer 独占 1024 及以上的 RenderLayers 区间，宿主不要将其他 mesh/camera 放入该区间。
/// source 必须通过 props 提供；运行时 config/source/reducer 由唯一 WaveformRuntime 持有。
#[derive(SceneComponent, FromTemplate)]
#[scene(WaveformProps)]
#[require(crate::renderer::InitializationFailure)]
pub struct Waveform;

/// 一次性构造输入，Scene 展开后只保留 persistent runtime/style/cursor。
pub struct WaveformProps {
    /// 固定规格；clone 仅用于可重复展开的 Scene template，不是运行期第二份配置。
    pub config: WaveformConfig,
    /// 只读 adapter，必须提供；source 只消费 config 定义的规格。
    pub source: Option<Arc<dyn WaveformSource>>,
    /// 初始化线宽、颜色与背景，后续修改 persistent WaveformStyle。
    pub style: WaveformStyle,
}

/// 内部唯一 viewport；不在每 channel 创建 UI entity。
#[derive(Component)]
pub(crate) struct WaveformViewport;

impl Default for WaveformProps {
    fn default() -> Self {
        Self {
            config: WaveformConfig {
                sample_rate: 1000,
                visible_duration_ms: 1000,
                channel_ranges: vec![-1.0..=1.0],
            },
            source: None,
            style: WaveformStyle::default(),
        }
    }
}

impl Waveform {
    /// 先校验再建立 shell；Image/Mesh 与 camera 由 Build 阶段初始化。
    fn scene(props: WaveformProps) -> impl Scene {
        let validation_style = props.style.clone();
        bsn! {
            Waveform
            template(move |_| {
                let source = props.source.clone().ok_or_else(|| {
                    widgetry_error!("Waveform Scene 必须提供 source");
                    logged_error("Waveform requires a source")
                })?;
                validation_style.validate().map_err(|error| {
                    widgetry_error!(%error, "Waveform Scene style 无效");
                    logged_error("Invalid WaveformStyle")
                })?;
                WaveformRuntime::new(props.config.clone(), source)
                    .map_err(|error| logged_error(error.to_string()))
            })
            template(move |_| Ok(props.style.clone()))
            WaveformCursor::default()
            template(|_| Ok(WaveformOutputLength::default()))
            Node { min_width: px(0), min_height: px(0) }
            Visibility::default()
            Children [(
                template(|_| Ok(WaveformViewport))
                template(|_| Ok(ViewportNode { camera: None }))
                Pickable::IGNORE
                Node { width: percent(100), height: percent(100), min_width: px(0), min_height: px(0) }
            )]
        }
    }
}
