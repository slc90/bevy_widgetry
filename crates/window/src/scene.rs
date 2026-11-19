use crate::{
    background::{WidgetryWindowBackground, window_background},
    title_bar::{
        bar::{custom_controls, default_controls, title_bar},
        resize::window_resize_area,
    },
    window_root::{OwnedWindow, WindowContent, WindowRoot},
};
use bevy::{prelude::*, scene::SceneScope, window::CompositeAlphaMode};
use bevy_widgetry_theme::WidgetryTheme;

#[derive(Clone, Copy, Debug)]
pub struct WidgetryWindowControlsConfig {
    pub minimize_visible: bool,
    pub maximize_visible: bool,
    pub close_visible: bool,
    pub resizable: bool,
    pub double_click_maximize: bool,
}

#[derive(Clone, Copy, Debug)]
pub struct WidgetryWindowBehavior {
    pub resizable: bool,
    pub double_click_maximize: bool,
}

impl Default for WidgetryWindowBehavior {
    fn default() -> Self {
        Self {
            resizable: true,
            double_click_maximize: true,
        }
    }
}

// The default Windows DX12 HWND swap chain does not support transparent compositing.
// Set transparency before creating the native window and select DxgiFromVisual during host rendering initialization.
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
    background: WidgetryWindowBackground,
    colors: crate::WidgetryWindowColorOverrides,
    title_bar_content: impl SceneList,
    content: impl SceneList,
) -> impl Scene {
    let title_bar = title_bar(
        controls.double_click_maximize,
        title_bar_content,
        default_controls(controls),
    );
    borrowed_window(
        target_window,
        target_camera,
        controls.resizable,
        background,
        colors,
        title_bar,
        content,
    )
}

pub fn widgetry_window_with_controls(
    target_window: Entity,
    target_camera: Entity,
    behavior: WidgetryWindowBehavior,
    background: WidgetryWindowBackground,
    colors: crate::WidgetryWindowColorOverrides,
    title_bar_content: impl SceneList,
    controls: impl SceneList,
    content: impl SceneList,
) -> impl Scene {
    let title_bar = title_bar(
        behavior.double_click_maximize,
        title_bar_content,
        custom_controls(controls),
    );
    borrowed_window(
        target_window,
        target_camera,
        behavior.resizable,
        background,
        colors,
        title_bar,
        content,
    )
}

fn borrowed_window(
    target_window: Entity,
    target_camera: Entity,
    resizable: bool,
    background: WidgetryWindowBackground,
    colors: crate::WidgetryWindowColorOverrides,
    title_bar: impl Scene,
    content: impl SceneList,
) -> impl Scene {
    bsn! {
        template(move |_| Ok(WindowRoot { target_window }))
        template(move |_| Ok(UiTargetCamera(target_camera)))
        @window_shell(resizable, background, colors, title_bar, content)
    }
}

pub fn owned_widgetry_window(
    native_window: Window,
    controls: WidgetryWindowControlsConfig,
    background: WidgetryWindowBackground,
    colors: crate::WidgetryWindowColorOverrides,
    title_bar_content: impl SceneList,
    content: impl SceneList,
) -> impl Scene {
    let title_bar = title_bar(
        controls.double_click_maximize,
        title_bar_content,
        default_controls(controls),
    );
    owned_window(
        native_window,
        controls.resizable,
        background,
        colors,
        title_bar,
        content,
    )
}

pub fn owned_widgetry_window_with_controls(
    native_window: Window,
    behavior: WidgetryWindowBehavior,
    background: WidgetryWindowBackground,
    colors: crate::WidgetryWindowColorOverrides,
    title_bar_content: impl SceneList,
    controls: impl SceneList,
    content: impl SceneList,
) -> impl Scene {
    let title_bar = title_bar(
        behavior.double_click_maximize,
        title_bar_content,
        custom_controls(controls),
    );
    owned_window(
        native_window,
        behavior.resizable,
        background,
        colors,
        title_bar,
        content,
    )
}

fn owned_window(
    native_window: Window,
    resizable: bool,
    background: WidgetryWindowBackground,
    colors: crate::WidgetryWindowColorOverrides,
    title_bar: impl Scene,
    content: impl SceneList,
) -> impl Scene {
    let allocation_colors = colors.clone();
    bsn! {
        template(move |context| {
            let root = context.entity.id();
            allocation_colors.validate().map_err(|error| error.with_context(bevy_widgetry_diagnostics::ErrorFrame::new("window.allocate").field("root", root)).into_bevy_error())?;
            let (target_window, camera) = context.entity.world_scope(|world| {
                let target = world.spawn(prepare_native_window(native_window.clone())).id();
                let camera = world.spawn(Camera2d).id();
                (target, camera)
            });
            context.entity.insert((UiTargetCamera(camera), OwnedWindow { window: target_window, camera }));
            Ok(WindowRoot { target_window })
        })
        @window_shell(resizable, background, colors, title_bar, content)
    }
}

fn window_shell(
    resizable: bool,
    background: WidgetryWindowBackground,
    colors: crate::WidgetryWindowColorOverrides,
    title_bar: impl Scene,
    content: impl SceneList,
) -> impl Scene {
    bsn! {
        template(move |context| colors.clone().initial().map_err(|error| {
            let root = context.entity.id();
            let frame = context.entity.world_scope(|world| crate::runtime_error::frame(world, root, "window.colors.initialize"));
            error.with_context(frame).into_bevy_error()
        }))
        @window_background(background)
        template(|context| Ok(BorderColor::all(context.resource::<WidgetryTheme>().window.frame.normal.border)))
        Children [
            {SceneScope(title_bar)}--
            template(|_| Ok(WindowContent)) Children [{content}]--
            {resizable.then(|| bsn! { @window_resize_area() })}
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
            double_click_maximize: true,
        }
    }
}
