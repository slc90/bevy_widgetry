use crate::title_bar::{close::CloseButton, maximize::MaximizeButton, minimize::MinimizeButton};
use bevy::{
    asset::AssetServer,
    color::Color,
    ecs::{component::Component, entity::Entity, hierarchy::ChildSpawnerCommands},
    picking::Pickable,
    ui::{
        AlignItems, BackgroundColor, BorderColor, FlexDirection, Node, PositionType, UiRect,
        percent, px,
    },
    utils::default,
};
use bevy_widgetry_core::icon::Icon;

#[derive(Component)]
#[require(
    Node = title_bar_node(),
    BackgroundColor = title_bar_background(),
    BorderColor = title_bar_border_color(),
)]
pub(crate) struct TitleBar;

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

fn title_bar_background() -> BackgroundColor {
    BackgroundColor(Color::srgb(0.12, 0.12, 0.13))
}

fn title_bar_border_color() -> BorderColor {
    BorderColor::all(Color::srgb(0.22, 0.22, 0.24))
}

#[derive(Component)]
#[require(Node = title_bar_drag_area_node())]
pub(super) struct TitleBarDragArea;

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

#[derive(Component)]
#[require(
    Node = title_bar_content_node(),
    Pickable = Pickable::IGNORE,
)]
pub(super) struct TitleBarContent;

fn title_bar_content_node() -> Node {
    Node {
        flex_grow: 1.0,
        height: percent(100),
        align_items: AlignItems::Center,
        ..default()
    }
}

#[derive(Component)]
#[require(
    Node = window_controls_node(),
    Pickable = Pickable::IGNORE,
)]
struct WindowControls;

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
        parent: &mut ChildSpawnerCommands<'_>,
        asset_server: &AssetServer,
    ) -> Entity {
        let mut content_entity = None;

        parent.spawn(TitleBar).with_children(|title_bar| {
            // 最底层，铺满整个标题栏
            title_bar.spawn(TitleBarDragArea);

            content_entity = Some(title_bar.spawn(TitleBarContent).id());

            // 上层窗口按钮
            title_bar.spawn(WindowControls).with_children(|controls| {
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
        });

        content_entity.unwrap()
    }
}
