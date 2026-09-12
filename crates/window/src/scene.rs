use crate::{
    title_bar::{bar::title_bar, resize::window_resize_area},
    window_root::{WindowContent, WindowRoot},
};
use bevy::{prelude::*, window::CompositeAlphaMode};
use bevy_widgetry_core::ThemeMode;

/// 仅在场景展开时决定系统按钮显隐；可交互性始终读取原生 Window.enabled_buttons。
#[derive(Clone, Copy, Debug)]
pub struct WindowControlsConfig {
    /// 是否在标题栏生成最小化按钮。
    pub minimize_visible: bool,
    /// 是否在标题栏生成最大化按钮；关闭按钮始终存在。
    pub maximize_visible: bool,
}

/// 在创建原生窗口前准备透明、无系统装饰、预乘 alpha 合成的 Widgetry 窗口，其他配置保留调用方的值。
/// 必须在生成原生窗口之前调用；透明属性不能依赖运行期修改。
/// Windows DX12 应用须使用 DxgiFromVisual（DirectComposition）交换链；默认 HWND 交换链不支持此合成模式。
/// 调用方可通过 RenderCreation::Manual 在渲染初始化中配置；使用 Bevy 自动初始化时，
/// 则须在进程启动前设置 WGPU_DX12_PRESENTATION_SYSTEM=DxgiFromVisual。
pub fn widgetry_window(mut window: Window) -> Window {
    window.transparent = true;
    window.decorations = false;
    window.composite_alpha_mode = CompositeAlphaMode::PreMultiplied;
    window
}

/// 构造绑定原生窗口与显式 UI 相机的完整窗口场景，需先注册 WindowPlugin 和 Bevy 资产、场景插件。
///
/// 原生窗口须在创建前通过 widgetry_window 准备，保持 transparent 为 true、decorations 为 false。
/// composite_alpha_mode 必须为 PreMultiplied；Windows DX12 还须满足 widgetry_window 文档中的渲染初始化要求。
/// 每个原生 Window 只能绑定一个根；无效窗口、创建期属性、相机或重复绑定会清理新树。
/// 场景展开后的首个 PostUpdate 校验绑定；关闭窗口后清理 UI，但不拥有或销毁调用方相机。
/// 标题栏内容允许空 bsn_list，主体内容必须显式提供；普通标题内容应忽略拾取以允许拖动。
/// target_camera 必须是此窗口专用的全窗口 UI 相机，由调用方创建和销毁。
/// 绑定成功后系统将其 RenderTarget 指向 target_window，清除 viewport，并以 Custom(Color::NONE) 透明清屏。
/// 系统保留相机的 order、is_active 及其他业务可见性配置；其他相机的合成关系由调用方负责。
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
