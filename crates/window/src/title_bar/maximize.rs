use crate::{title_bar::bar::TitleBar, window_root::WindowContent};
use crate::{
    title_bar::controls::window_control_button_node,
    window_root::{WindowRoot, find_window_root},
};
use bevy::prelude::{BorderRadius, px};
use bevy::{
    asset::AssetServer,
    ecs::{
        component::Component,
        entity::Entity,
        hierarchy::{ChildOf, Children},
        observer::On,
        query::With,
        system::{NonSendMarker, Query, Res},
    },
    picking::hover::Hovered,
    ui::{BackgroundColor, Node},
    ui_widgets::{Activate, Button},
    window::Window,
    winit::WINIT_WINDOWS,
};
use bevy_widgetry_asset::BuiltinIcon;
use bevy_widgetry_core::icon::Icon;

/// 在最大化与还原之间切换，并保持图标与真实窗口状态同步。
#[derive(Component)]
#[require(
    Button,
    Hovered,
    Node = window_control_button_node(),
    BackgroundColor,
)]
pub(super) struct MaximizeButton;

/// 根据真实窗口当前状态切换最大化与还原，并更新按钮图标。
pub(super) fn on_maximize_restore(
    event: On<Activate>,
    _non_send_marker: NonSendMarker,
    buttons: Query<(), With<MaximizeButton>>,
    parents: Query<&ChildOf>,
    roots: Query<&WindowRoot>,
    mut windows: Query<&mut Window>,
) {
    let Ok(()) = buttons.get(event.entity) else {
        return;
    };

    let Some(root) = find_window_root(event.entity, &parents, &roots) else {
        return;
    };

    let actual_maximized = WINIT_WINDOWS.with_borrow(|winit_windows| {
        winit_windows
            .get_window(root.target_window)
            .map(|window| window.is_maximized())
    });

    let Some(actual_maximized) = actual_maximized else {
        return;
    };

    let Ok(mut window) = windows.get_mut(root.target_window) else {
        return;
    };

    if !window.enabled_buttons.maximize {
        return;
    }

    window.set_maximized(!actual_maximized);
}

/// 只读取 winit 实际状态，统一同步图标、三个容器圆角和缩放依据。
pub(super) fn sync_maximize_state(
    _non_send_marker: NonSendMarker,
    mut roots: Query<(Entity, &mut WindowRoot)>,
    children: Query<&Children>,
    buttons: Query<(), With<MaximizeButton>>,
    bars: Query<(), With<TitleBar>>,
    contents: Query<(), With<WindowContent>>,
    mut nodes: Query<&mut Node>,
    mut icons: Query<&mut Icon>,
    asset_server: Res<AssetServer>,
) {
    WINIT_WINDOWS.with_borrow(|winit_windows| {
        for (entity, mut root) in &mut roots {
            let Some(window) = winit_windows.get_window(root.target_window) else {
                continue;
            };
            let maximized = window.is_maximized();
            if root.maximized == maximized {
                continue;
            }
            root.maximized = maximized;
            let radius = px(if maximized { 0.0 } else { 8.0 });
            if let Ok(mut node) = nodes.get_mut(entity) {
                node.border_radius = BorderRadius::all(radius);
            }
            for descendant in children.iter_descendants(entity) {
                if bars.contains(descendant) {
                    if let Ok(mut node) = nodes.get_mut(descendant) {
                        node.border_radius = BorderRadius::top(radius);
                    }
                } else if contents.contains(descendant) {
                    if let Ok(mut node) = nodes.get_mut(descendant) {
                        node.border_radius = BorderRadius::bottom(radius);
                    }
                } else if buttons.contains(descendant) {
                    let Ok(button_children) = children.get(descendant) else {
                        continue;
                    };
                    for &child in button_children.iter() {
                        if let Ok(mut icon) = icons.get_mut(child) {
                            icon.set_svg(
                                &asset_server,
                                if maximized {
                                    BuiltinIcon::WindowRestore
                                } else {
                                    BuiltinIcon::WindowMaximize
                                }
                                .path(),
                            );
                        }
                    }
                }
            }
        }
    });
}
