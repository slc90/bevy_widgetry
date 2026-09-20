use crate::{
    scene::WidgetryWindowControlsConfig,
    title_bar::{close::CloseButton, maximize::MaximizeButton, minimize::MinimizeButton},
};
use bevy::prelude::*;
use bevy_widgetry_asset::BuiltinIcon;
use bevy_widgetry_core::{ThemeMode, icon::WidgetryIcon};

/// 自定义 window 顶部容器，组合 drag 区域、应用内容和系统控制按钮。
#[derive(Component)]
#[require(
    Node = title_bar_node(),
    BorderColor,
)]
pub(crate) struct TitleBar;

/// 铺满 title bar 底层以承接空白区域的 window drag。
#[derive(Component)]
#[require(Node = title_bar_drag_area_node())]
pub(super) struct TitleBarDragArea;

/// 承载调用方提供的 title bar 内容，自身不拦截 pointer picking。
#[derive(Component)]
#[require(
    Node = title_bar_content_node(),
    Pickable = Pickable::IGNORE,
)]
pub(super) struct TitleBarContent;

/// 按顺序排列 minimize、maximize 和 close button。
#[derive(Component)]
#[require(
    Node = window_controls_node(),
    Pickable = Pickable::IGNORE,
)]
pub(super) struct WindowControls;

/// 固定 title bar 高度，横向排列内容与系统按钮并用 bottom border 分隔内容。
fn title_bar_node() -> Node {
    Node {
        width: percent(100),
        height: px(36),
        flex_shrink: 0.0,
        border_radius: BorderRadius::top(px(8)),
        flex_direction: FlexDirection::Row,
        align_items: AlignItems::Stretch,
        border: UiRect::bottom(px(1)),
        ..default()
    }
}

/// 覆盖 title bar 全部区域，使空白处也能响应 window drag。
fn title_bar_drag_area_node() -> Node {
    Node {
        position_type: PositionType::Absolute,
        left: px(0),
        top: px(0),
        width: percent(100),
        height: percent(100),
        ..default()
    }
}

/// 让调用方内容填充系统按钮之外的空间并垂直居中。
fn title_bar_content_node() -> Node {
    Node {
        flex_grow: 1.0,
        height: percent(100),
        align_items: AlignItems::Center,
        ..default()
    }
}

/// 横向排列系统按钮并填满 title bar 高度。
fn window_controls_node() -> Node {
    Node {
        height: percent(100),
        flex_direction: FlexDirection::Row,
        align_items: AlignItems::Stretch,
        ..default()
    }
}

/// 保留底层 drag 区域，slot 容器不参与 picking，系统按钮始终位于上层。
pub(crate) fn title_bar(
    controls: WidgetryWindowControlsConfig,
    content: impl SceneList,
) -> impl Scene {
    bsn! {
        template(|_| Ok(TitleBar))
        template(|context| Ok(BorderColor::all(context.resource::<ThemeMode>().colors().title_bar_border)))
        Children [
            (template(|_| Ok(TitleBarDragArea))),
            (template(|_| Ok(TitleBarContent)) Children [{content}]),
            (
                template(|_| Ok(WindowControls))
                Children [
                    {controls.minimize_visible.then(|| bsn! {
                        template(|_| Ok(MinimizeButton))
                        Children [system_icon(BuiltinIcon::WindowMinimize)]
                    })},
                    {controls.maximize_visible.then(|| bsn! {
                        template(|_| Ok(MaximizeButton))
                        Children [system_icon(BuiltinIcon::WindowMaximize)]
                    })},
                    {controls.close_visible.then(|| bsn! {
                        template(|_| Ok(CloseButton))
                        Children [system_icon(BuiltinIcon::WindowClose)]
                    })},
                ]
            ),
        ]
    }
}

/// 组合固定尺寸的 embedded icon，固定 foreground color 并穿透 picking 以供 window button 使用。
fn system_icon(icon: BuiltinIcon) -> impl Scene {
    bsn! {
        @WidgetryIcon {
            @path: {icon.path()},
            @max_size: { Some(UVec2::new(16, 16)) },
            @color: { Some(Color::WHITE) },
        }
        template(|_| Ok(Pickable::IGNORE))
    }
}
