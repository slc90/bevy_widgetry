use crate::{
    title_bar::{bar::title_bar, resize::window_resize_area},
    window_root::{WindowContent, WindowRoot},
};
use bevy::prelude::*;
use bevy_widgetry_core::ThemeMode;

/// 仅在场景展开时决定系统按钮显隐；可交互性始终读取原生 Window.enabled_buttons。
#[derive(Clone, Copy, Debug)]
pub struct WindowControlsConfig {
    /// 是否在标题栏生成最小化按钮。
    pub minimize_visible: bool,
    /// 是否在标题栏生成最大化按钮；关闭按钮始终存在。
    pub maximize_visible: bool,
}

/// 构造绑定原生窗口与显式 UI 相机的完整窗口场景，需先注册 WindowPlugin 和 Bevy 资产、场景插件。
///
/// 每个原生 Window 只能绑定一个根；无效窗口、无效相机或重复绑定会记录错误并清理新树。
/// 场景展开后的首个 PostUpdate 校验绑定并关闭原生 decorations；关闭窗口后清理 UI，但不拥有或销毁调用方相机。
/// 标题栏内容允许空 bsn_list，主体内容必须显式提供；普通标题内容应忽略拾取以允许拖动。
/// 相机必须由调用方配置为渲染 target_window，系统不会修改其渲染目标。
pub fn window(
    target_window: Entity,
    target_camera: Entity,
    controls: WindowControlsConfig,
    title_bar_content: impl SceneList,
    content: impl SceneList,
) -> impl Scene {
    bsn! {
        template(move |_| Ok(WindowRoot { target_window, maximized: false }))
        template(move |_| Ok(UiTargetCamera(target_camera)))
        template(|context| Ok(BackgroundColor(context.resource::<ThemeMode>().colors().window_background)))
        template(|context| Ok(BorderColor::all(context.resource::<ThemeMode>().colors().window_border)))
        Children [
            title_bar(controls, title_bar_content),
            (template(|_| Ok(WindowContent)) Children [{content}]),
            window_resize_area(),
        ]
    }
}

impl Default for WindowControlsConfig {
    fn default() -> Self {
        Self {
            minimize_visible: true,
            maximize_visible: true,
        }
    }
}
