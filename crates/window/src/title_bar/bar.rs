use crate::{
    scene::WidgetryWindowControlsConfig,
    title_bar::{close::CloseButton, maximize::MaximizeButton, minimize::MinimizeButton},
};
use bevy::prelude::*;
use bevy_widgetry_asset::BuiltinIcon;
use bevy_widgetry_core::icon::WidgetryIcon;
use bevy_widgetry_theme::WidgetryThemeMode;

#[derive(Component)]
#[require(
    Node = title_bar_node(),
    BorderColor,
)]
pub(crate) struct TitleBar;

#[derive(Component)]
#[require(Node = title_bar_drag_area_node())]
pub(super) struct TitleBarDragArea;

#[derive(Component)]
#[require(
    Node = title_bar_content_node(),
    Pickable = Pickable::IGNORE,
)]
pub(super) struct TitleBarContent;

#[derive(Component)]
#[require(
    Node = window_controls_node(),
    Pickable = Pickable::IGNORE,
)]
pub(crate) struct WindowControls;

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

fn title_bar_content_node() -> Node {
    Node {
        flex_grow: 1.0,
        height: percent(100),
        align_items: AlignItems::Center,
        ..default()
    }
}

fn window_controls_node() -> Node {
    Node {
        height: percent(100),
        flex_direction: FlexDirection::Row,
        align_items: AlignItems::Stretch,
        ..default()
    }
}

pub(crate) fn title_bar(
    controls: WidgetryWindowControlsConfig,
    content: impl SceneList,
) -> impl Scene {
    bsn! {
        template(|_| Ok(TitleBar))
        template(|context| Ok(BorderColor::all(context.resource::<WidgetryThemeMode>().colors().window.title_bar.normal.border)))
        Children [
            template(|_| Ok(TitleBarDragArea))--
            template(|_| Ok(TitleBarContent)) Children [{content}]--

                template(|_| Ok(WindowControls))
                Children [
                    {controls.minimize_visible.then(|| bsn! {
                        template(|_| Ok(MinimizeButton))
                        Children [@system_icon(BuiltinIcon::WindowMinimize)]
                    })}--
                    {controls.maximize_visible.then(|| bsn! {
                        template(|_| Ok(MaximizeButton))
                        Children [@system_icon(BuiltinIcon::WindowMaximize)]
                    })}--
                    {controls.close_visible.then(|| bsn! {
                        template(|_| Ok(CloseButton))
                        Children [@system_icon(BuiltinIcon::WindowClose)]
                    })}
                ]

        ]
    }
}

fn system_icon(icon: BuiltinIcon) -> impl Scene {
    bsn! {
        @WidgetryIcon {
            @path: {icon.path()},
            @max_size: { Some(UVec2::new(16, 16)) },
        }
        template(|_| Ok(Pickable::IGNORE))
    }
}
