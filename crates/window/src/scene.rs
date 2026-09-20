use crate::{
    title_bar::{bar::title_bar, resize::window_resize_area},
    window_root::{OwnedWindow, WindowContent, WindowRoot},
};
use bevy::{prelude::*, window::CompositeAlphaMode};
use bevy_widgetry_core::ThemeMode;

/// 仅在 Scene 展开时决定 button 显隐和 resize 区域；native 配置仍决定最终可交互性。
#[derive(Clone, Copy, Debug)]
pub struct WidgetryWindowControlsConfig {
    /// 是否在 title bar 生成 minimize button。
    pub minimize_visible: bool,
    /// 是否在 title bar 生成 maximize button。
    pub maximize_visible: bool,
    /// 是否在 title bar 生成 close button，不影响操作系统级关闭。
    pub close_visible: bool,
    /// 是否生成 Widgetry resize hit area，native Window.resizable 仍决定最终约束。
    pub resizable: bool,
}

/// 在创建 native window 前准备透明、无系统 decoration、采用 premultiplied alpha compositing 的 Widgetry window，其他配置保留调用方的值。
/// 必须在生成 native window 之前调用；transparent 属性不能依赖运行期修改。
/// Windows DX12 应用须使用 DxgiFromVisual（DirectComposition）swap chain；默认 HWND swap chain 不支持此 compositing 模式。
/// 调用方可通过 RenderCreation::Manual 在 rendering 初始化中配置；使用 Bevy 自动初始化时，
/// 则须在进程启动前设置 WGPU_DX12_PRESENTATION_SYSTEM=DxgiFromVisual。
pub fn prepare_native_window(mut window: Window) -> Window {
    window.transparent = true;
    window.decorations = false;
    window.composite_alpha_mode = CompositeAlphaMode::PreMultiplied;
    window
}

/// 构造绑定 native window 与显式 UI camera 的完整 window Scene，需先注册 WidgetryWindowPlugin 和 Bevy asset、Scene plugin。
///
/// native window 须在创建前通过 prepare_native_window 准备，保持 transparent 为 true、decorations 为 false。
/// composite_alpha_mode 必须为 PreMultiplied；Windows DX12 还须满足 prepare_native_window 文档中的 rendering 初始化要求。
/// 每个 native Window 只能绑定一个 root；无效 window、创建期属性、camera 或重复绑定会清理新 tree。
/// Scene 展开后的首个 PostUpdate 校验绑定；关闭 window 后清理 UI，但不拥有或销毁调用方 camera。
/// title bar 内容允许空 bsn_list，主体内容必须显式提供；普通标题内容应忽略 picking 以允许 drag。
/// target_camera 必须是此 window 专用且覆盖整个 window 的 UI camera，由调用方创建和销毁。
/// 绑定成功后 system 将其 RenderTarget 指向 target_window，清除 viewport，并以 Custom(Color::NONE) 透明清屏。
/// system 保留 camera 的 order、is_active 及其他业务 visibility 配置；其他 camera 的 compositing 关系由调用方负责。
pub fn widgetry_window(
    target_window: Entity,
    target_camera: Entity,
    controls: WidgetryWindowControlsConfig,
    title_bar_content: impl SceneList,
    content: impl SceneList,
) -> impl Scene {
    bsn! {
        template(move |_| Ok(WindowRoot { target_window, maximized: false }))
        template(move |_| Ok(UiTargetCamera(target_camera)))
        window_shell(controls, title_bar_content, content)
    }
}

/// 创建并拥有 native window 与专用 Camera2d；root 销毁时自动释放两者。
/// 需先注册 WidgetryWindowPlugin 和 Bevy asset、Scene plugin。native 属性自动经 prepare_native_window 准备。
/// native window 被操作系统关闭时同样回收整棵 UI tree 与 camera；仅移除内部 ownership marker 不会销毁资源。
pub fn owned_widgetry_window(
    native_window: Window,
    controls: WidgetryWindowControlsConfig,
    title_bar_content: impl SceneList,
    content: impl SceneList,
) -> impl Scene {
    bsn! {
        template(move |context| {
            let (target_window, camera) = context.entity.world_scope(|world| {
                let target = world.spawn(prepare_native_window(native_window.clone())).id();
                let camera = world.spawn(Camera2d).id();
                (target, camera)
            });
            context.entity.insert(UiTargetCamera(camera));
            Ok(WindowRoot { target_window, maximized: false })
        })
        template(|_| Ok(OwnedWindow))
        window_shell(controls, title_bar_content, content)
    }
}

/// 两种 ownership 入口共用同一 window 外壳，不增加额外 root node。
fn window_shell(
    controls: WidgetryWindowControlsConfig,
    title_bar_content: impl SceneList,
    content: impl SceneList,
) -> impl Scene {
    bsn! {
        template(|context| Ok(BackgroundColor(context.resource::<ThemeMode>().colors().window_background)))
        template(|context| Ok(BorderColor::all(context.resource::<ThemeMode>().colors().window_border)))
        Children [
            title_bar(controls, title_bar_content),
            (template(|_| Ok(WindowContent)) Children [{content}]),
            {controls.resizable.then(|| bsn! { window_resize_area() })},
        ]
    }
}

impl Default for WidgetryWindowControlsConfig {
    fn default() -> Self {
        Self {
            minimize_visible: true,
            maximize_visible: true,
            close_visible: true,
            resizable: true,
        }
    }
}
