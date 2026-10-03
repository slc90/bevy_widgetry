use crate::{
    title_bar::{bar::title_bar, resize::window_resize_area},
    window_root::{OwnedWindow, WindowContent, WindowRoot},
};
use bevy::{prelude::*, window::CompositeAlphaMode};
use bevy_widgetry_core::ThemeMode;

#[derive(Clone, Copy, Debug)]
pub struct WidgetryWindowControlsConfig {
    pub minimize_visible: bool,
    pub maximize_visible: bool,
    pub close_visible: bool,
    pub resizable: bool,
}

// Windows DX12 默认 HWND swap chain 不支持透明 compositing；native window 须在创建前设置透明属性，并由宿主 rendering 初始化选用 DxgiFromVisual。
pub fn prepare_native_window(mut window: Window) -> Window {
    window.transparent = true;
    window.decorations = false;
    window.composite_alpha_mode = CompositeAlphaMode::PreMultiplied;
    window
}

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
