const TITLE_ICON_SIZE: UVec2 = UVec2::splat(16);

const TITLE_BAR_HEIGHT: f32 = 36.0;

use crate::{
    scene::WidgetryWindowControlsConfig,
    title_bar::{close::CloseButton, maximize::MaximizeButton, minimize::MinimizeButton},
};
use bevy::prelude::*;
use bevy_widgetry_asset::BuiltinIcon;
use bevy_widgetry_core::icon::WidgetryIcon;
use bevy_widgetry_theme::WidgetryTheme;

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
pub(crate) struct TitleBarContent;

#[derive(Component)]
#[require(
    Node = window_controls_node(),
    Pickable = Pickable::IGNORE,
)]
pub(crate) struct WindowControls;

fn title_bar_node() -> Node {
    Node {
        width: percent(100),
        min_width: px(0),
        height: px(TITLE_BAR_HEIGHT),
        flex_shrink: 0.0,
        border_radius: BorderRadius::top(px(crate::window_root::WINDOW_CORNER_RADIUS)),
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
        flex_basis: px(0),
        min_width: px(0),
        overflow: Overflow {
            x: OverflowAxis::Clip,
            y: OverflowAxis::Visible,
        },
        height: percent(100),
        align_items: AlignItems::Center,
        ..default()
    }
}

fn window_controls_node() -> Node {
    Node {
        flex_shrink: 0.0,
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
        template(|context| Ok(BorderColor::all(context.resource::<WidgetryTheme>().window.title_bar.normal.border)))
        Children [
            template(|_| Ok(TitleBarDragArea)) template(|_| Ok(bevy::picking::cursor::EntityCursor::default()))--
            template(|_| Ok(TitleBarContent)) Children [{content}]--

                template(|_| Ok(WindowControls))
                Children [
                    {controls.minimize_visible.then(|| bsn! {
                        template(|_| Ok(MinimizeButton))
                        template(|_| Ok(bevy::picking::cursor::EntityCursor::System(bevy::window::SystemCursorIcon::Pointer)))
                        Children [@system_icon(BuiltinIcon::WindowMinimize)]
                    })}--
                    {controls.maximize_visible.then(|| bsn! {
                        template(|_| Ok(MaximizeButton))
                        template(|_| Ok(bevy::picking::cursor::EntityCursor::System(bevy::window::SystemCursorIcon::Pointer)))
                        Children [@system_icon(BuiltinIcon::WindowMaximize)]
                    })}--
                    {controls.close_visible.then(|| bsn! {
                        template(|_| Ok(CloseButton))
                        template(|_| Ok(bevy::picking::cursor::EntityCursor::System(bevy::window::SystemCursorIcon::Pointer)))
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
            @max_size: { Some(TITLE_ICON_SIZE) },
        }
        template(|_| Ok(Pickable::IGNORE))
    }
}
