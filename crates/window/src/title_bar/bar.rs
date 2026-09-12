use crate::{
    scene::WindowControlsConfig,
    title_bar::{close::CloseButton, maximize::MaximizeButton, minimize::MinimizeButton},
};
use bevy::prelude::*;
use bevy_widgetry_asset::BuiltinIcon;
use bevy_widgetry_core::{ThemeMode, icon::Icon};

/// 自定义窗口顶部容器，组合拖动区域、应用内容和系统控制按钮。
#[derive(Component)]
#[require(
    Node = title_bar_node(),
    BorderColor,
)]
pub(crate) struct TitleBar;

/// 铺满标题栏底层以承接空白区域的窗口拖动。
#[derive(Component)]
#[require(Node = title_bar_drag_area_node())]
pub(super) struct TitleBarDragArea;

/// 承载调用方提供的标题栏内容，自身不拦截指针拾取。
#[derive(Component)]
#[require(
    Node = title_bar_content_node(),
    Pickable = Pickable::IGNORE,
)]
struct TitleBarContent;

/// 按顺序排列最小化、最大化和关闭按钮。
#[derive(Component)]
#[require(
    Node = window_controls_node(),
    Pickable = Pickable::IGNORE,
)]
struct WindowControls;

/// 固定标题栏高度，横向排列内容与系统按钮并用底边框分隔内容。
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

/// 覆盖标题栏全部区域，使空白处也能响应窗口拖动。
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

/// 横向排列系统按钮并填满标题栏高度。
fn window_controls_node() -> Node {
    Node {
        height: percent(100),
        flex_direction: FlexDirection::Row,
        align_items: AlignItems::Stretch,
        ..default()
    }
}

/// 保留底层拖动区，插槽容器不参与拾取，系统按钮始终位于上层。
pub(crate) fn title_bar(controls: WindowControlsConfig, content: impl SceneList) -> impl Scene {
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
                    (template(|_| Ok(CloseButton))
                        Children [system_icon(BuiltinIcon::WindowClose)]),
                ]
            ),
        ]
    }
}

/// 从 BSN 资源上下文加载内嵌图标，固定前景色以隔离窗口主题变化。
fn system_icon(icon: BuiltinIcon) -> impl Scene {
    bsn! {
        template(move |context| {
            Ok(
                Icon::new(context.resource::<AssetServer>(), icon.path())
                    .with_size(16, 16)
                    .with_color(Color::WHITE)
            )
        })
        template(|_| Ok(Pickable::IGNORE))
    }
}
