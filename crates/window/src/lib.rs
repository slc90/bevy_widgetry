//! 提供自定义窗口界面的 Scene 构造与 native window 操作能力。
//! 适用于需要自定义 title bar、正文内容、多窗口和 modal 交互的桌面应用。
//!
//! widgetry_window 将调用方已有的 native window 与 camera 绑定到 Widgetry 窗口界面。
//! owned_widgetry_window 创建窗口与 camera，并由窗口界面的 lifecycle 管理这些资源。
//! title bar 与正文均接收 SceneList，可组合标签、工具按钮和其他 Widget 内容。
//! 支持拖动 title bar 移动窗口，通过边缘 resize 区域调整尺寸。
//! 提供最小化、最大化或恢复、关闭 controls，可分别配置是否显示，并配置窗口是否 resizable。
//! prepare_native_window 配置透明、无 native decorations 的窗口属性，用于承载自定义窗口外观。
//! taskbar_icon! 使用应用 package 相对 PNG 路径编译期嵌入图标，返回 Result<impl Plugin>。
//! 应用提供素材，初始化失败直接返回错误，成功安装后仅设置主窗口与任务栏图标一次。
//! 无需运行时读取 PNG，EXE ICO 则由应用 build script 使用独立 app_icon_build 能力嵌入。
//! WidgetryModalWindow 将 child 关联到 parent native window，并在 parent 界面提供 pointer blocker。
//! 构造时显式选择 Theme 背景或带 opacity 的 Image 背景。
//! Stretch 将完整图片铺满窗口，允许随窗口比例变化而变形。
//! Cover 保持原图比例，从中心裁掉超出部分，并随本帧 layout 尺寸更新采样区域。
//! Theme 背景、窗口 border 与 title bar border 随 theme 更新，Image 背景保持构造配置。
//!
//! 调用方提供的 native window 和 camera 由调用方持有，窗口界面回收时保留这些资源。
//! owned 窗口在界面 root 回收时清理自己创建的 native window 与 camera。
//! 一个 native window 与 camera 分别绑定到一个窗口界面，构造时要求目标有效且窗口属性已准备。
//! WidgetryWindowPlugin 为桌面 App 安装运行期多窗口 rendering 支持，没有 RenderApp 时跳过。
//! 桌面 App 可将 transparent_render_creation() 提供给 Bevy RenderPlugin，创建支持透明 compositing 的 DX12 renderer。
//! Image 未加载时背景透明，不使用 Theme 色作为 fallback。
//! 图片直接覆盖整个窗口，包括 title bar 与正文，并跟随窗口圆角及最大化状态。
//! opacity 使用 0.0..=1.0，只改变图片整体 alpha，透明区域显示 native window 后方内容。
//! 图片 handle、mode 与 opacity 均为构造期配置，不提供运行时切换或更新 API。
//! 调用方负责图片加载、格式支持与纹理尺寸策略，Window 不限制分辨率或自动 downscale。
//! title bar 与正文的自定义内容由调用方或所组合的 Widget 管理配色。
//! 内建 controls 使用固定配色，并按 hover 和 pressed 更新外观。
//! modal 的 parent 参数使用 native window Entity，parent 界面结束时清理对应 modal child。
//! 同一 parent 的多个 modal child 共享 blocker，最后一个有效 child 结束后释放阻挡。

//! 最近打开的 sibling 与最内层 modal 接受 pointer/keyboard 输入，Tab 在该窗口有效控件中循环。
//! Modal 捕获并校验之前的 focus，关闭后恢复有效目标，保留调用方维护的 InteractionDisabled。
//! keyboard/IME 按实际 native Window 路由，继续复用官方 EditableText 与 Widget observers。
//! 原始输入 message 保留供宿主读取，宿主全局快捷键须自行尊重 modal 状态。

#[cfg(not(all(target_os = "windows", target_pointer_width = "64")))]
compile_error!("bevy_widgetry 仅支持 Windows 64 位 target");

mod background;
mod input;
mod modal;
mod native_icon;
mod render;
mod scene;
mod title_bar;
mod window_root;

pub use background::{
    WidgetryWindowBackground, WidgetryWindowImageBackground, WidgetryWindowImageMode,
};
pub use modal::WidgetryModalWindow;
#[doc(hidden)]
pub use native_icon::native_icon_plugin;
pub use render::transparent_render_creation;
pub use scene::{
    WidgetryWindowControlsConfig, owned_widgetry_window, prepare_native_window, widgetry_window,
};
pub use title_bar::WidgetryWindowPlugin;

/// 窗口或 modal 首次取得 focus 时的候选控件，较小 priority 优先。
#[derive(bevy::prelude::Component, bevy::prelude::FromTemplate, Default, Clone, Copy, Debug)]
pub struct WidgetryWindowInitialFocus(pub i32);

/// 返回窗口界面所绑定的 native Window。构造期间也可查询，不公开内部 root component。
pub fn widgetry_window_target(world: &World, root: Entity) -> Option<Entity> {
    world
        .get::<window_root::WindowRoot>(root)
        .map(|root| root.target_window)
}

/// 检查 native Window 是否已绑定一个有效的 Widgetry 窗口界面。
pub fn is_widgetry_window(world: &mut World, native: Entity) -> bool {
    if world.get::<Window>(native).is_none_or(|window| {
        !window.transparent
            || window.decorations
            || window.composite_alpha_mode != bevy::window::CompositeAlphaMode::PreMultiplied
    }) {
        return false;
    }
    let roots: Vec<_> = world
        .query::<(
            Entity,
            &window_root::WindowRoot,
            Option<&bevy::prelude::UiTargetCamera>,
        )>()
        .iter(world)
        .map(|(entity, root, camera)| (entity, root.target_window, camera.map(|camera| camera.0)))
        .collect();
    let matches: Vec<_> = roots
        .iter()
        .filter(|(_, target, _)| *target == native)
        .collect();
    let [bound] = matches.as_slice() else {
        return false;
    };
    let Some(camera) = bound.2 else {
        return false;
    };
    if world.get::<bevy::prelude::Camera>(camera).is_none()
        || roots
            .iter()
            .any(|root| root.0 != bound.0 && root.2 == Some(camera))
    {
        return false;
    }
    world
        .get::<window_root::WindowInitialized>(bound.0)
        .is_none()
        || matches!(world.get::<bevy::camera::RenderTarget>(camera),Some(bevy::camera::RenderTarget::Window(bevy::window::WindowRef::Entity(target))) if *target==native)
}

use bevy::prelude::{Entity, Window, World};

mod colors;
mod style;
pub use colors::*;
pub mod internal {
    pub use crate::style::apply_owned_window_colors;
    pub use crate::window_root::WindowRoot;
}
