use crate::title_bar::{close::CloseButton, maximize::MaximizeButton, minimize::MinimizeButton};
use bevy::{
    asset::AssetServer,
    color::Color,
    ecs::{component::Component, entity::Entity, hierarchy::ChildOf, system::Commands},
    picking::Pickable,
    ui::{
        AlignItems, BackgroundColor, BorderColor, FlexDirection, Node, PositionType, UiRect,
        percent, px,
    },
    utils::default,
};
use bevy_widgetry_core::icon::Icon;

/// 自定义窗口顶部容器，组合拖动区域、应用内容和系统控制按钮。
#[derive(Component)]
#[require(
    Node = title_bar_node(),
    BackgroundColor = title_bar_background(),
    BorderColor = title_bar_border_color(),
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
        flex_direction: FlexDirection::Row,
        align_items: AlignItems::Stretch,
        border: UiRect::bottom(px(1)),
        ..default()
    }
}

/// 提供自定义标题栏的固定深色背景。
fn title_bar_background() -> BackgroundColor {
    BackgroundColor(Color::srgb(0.12, 0.12, 0.13))
}

/// 用浅于背景的底边框区分标题栏与窗口内容。
fn title_bar_border_color() -> BorderColor {
    BorderColor::all(Color::srgb(0.22, 0.22, 0.24))
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

impl TitleBar {
    pub(crate) fn spawn(
        commands: &mut Commands,
        parent: Entity,
        asset_server: &AssetServer,
    ) -> Entity {
        let bar = commands.spawn((TitleBar, ChildOf(parent))).id();
        // 最底层，铺满整个标题栏
        commands.spawn((TitleBarDragArea, ChildOf(bar)));

        let content_entity = commands.spawn((TitleBarContent, ChildOf(bar))).id();

        // 上层窗口按钮
        commands
            .spawn((WindowControls, ChildOf(bar)))
            .with_children(|controls| {
                controls.spawn(MinimizeButton).with_child(
                    Icon::new(
                        asset_server,
                        "embedded://bevy_widgetry_window/icons/minimize.svg",
                    )
                    .with_size(16, 16),
                );
                controls.spawn(MaximizeButton).with_child(
                    Icon::new(
                        asset_server,
                        "embedded://bevy_widgetry_window/icons/maximize.svg",
                    )
                    .with_size(16, 16),
                );

                controls.spawn(CloseButton).with_child(
                    Icon::new(
                        asset_server,
                        "embedded://bevy_widgetry_window/icons/close.svg",
                    )
                    .with_size(16, 16),
                );
            });

        content_entity
    }
}
