use bevy::{
    asset::io::embedded::EmbeddedAssetRegistry,
    math::CompassOctant,
    picking::hover::Hovered,
    prelude::*,
    ui::Pressed,
    ui_widgets::{Activate, Button},
    window::{SystemCursorIcon, WindowCloseRequested},
};
use bevy::{ecs::system::NonSendMarker, winit::WINIT_WINDOWS};
use bevy_widgetry_core::icon::{Icon, IconPlugin};
use std::path::{Path, PathBuf};

#[derive(Component)]
#[require(
    Node = title_bar_node(),
    BackgroundColor = title_bar_background(),
    BorderColor = title_bar_border_color(),
)]
pub struct TitleBar {
    pub target_window: Entity,
}

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
#[require(
    Node = title_bar_content_node(),
)]
pub struct TitleBarContent;

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
)]
pub struct WindowControls;

fn window_controls_node() -> Node {
    Node {
        height: percent(100),
        flex_direction: FlexDirection::Row,
        align_items: AlignItems::Stretch,
        ..default()
    }
}

#[derive(Component)]
#[require(
    Button,
    Hovered,
    Node = window_control_button_node(),
    BackgroundColor,
)]
pub struct MinimizeButton;

#[derive(Component)]
#[require(
    Button,
    Hovered,
    Node = window_control_button_node(),
    BackgroundColor,
)]
pub struct MaximizeButton;

#[derive(Component)]
#[require(
    Button,
    Hovered,
    Node = window_control_button_node(),
    BackgroundColor,
)]
pub struct CloseButton;

fn window_control_button_node() -> Node {
    Node {
        width: px(46),
        height: percent(100),
        align_items: AlignItems::Center,
        justify_content: JustifyContent::Center,
        ..default()
    }
}

fn window_control_background(hovered: bool, pressed: bool) -> Color {
    if pressed {
        Color::srgba(1.0, 1.0, 1.0, 0.14)
    } else if hovered {
        Color::srgba(1.0, 1.0, 1.0, 0.08)
    } else {
        Color::srgba(0.0, 0.0, 0.0, 0.0)
    }
}

fn update_window_control_style_changed(
    mut query: Query<
        (&Hovered, Has<Pressed>, &mut BackgroundColor),
        (
            Or<(With<MinimizeButton>, With<MaximizeButton>)>,
            Or<(Changed<Hovered>, Added<Pressed>)>,
        ),
    >,
) {
    for (hovered, pressed, mut background) in &mut query {
        background.0 = window_control_background(hovered.0, pressed);
    }
}

fn update_window_control_style_released(
    mut removed_pressed: RemovedComponents<Pressed>,
    mut query: Query<
        (&Hovered, Has<Pressed>, &mut BackgroundColor),
        Or<(With<MinimizeButton>, With<MaximizeButton>)>,
    >,
) {
    for entity in removed_pressed.read() {
        let Ok((hovered, pressed, mut background)) = query.get_mut(entity) else {
            continue;
        };

        background.0 = window_control_background(hovered.0, pressed);
    }
}

fn close_button_background(hovered: bool, pressed: bool) -> Color {
    if pressed {
        Color::srgb_u8(180, 30, 30)
    } else if hovered {
        Color::srgb_u8(196, 43, 28)
    } else {
        Color::NONE
    }
}

fn update_close_button_style_changed(
    mut query: Query<
        (&Hovered, Has<Pressed>, &mut BackgroundColor),
        (With<CloseButton>, Or<(Changed<Hovered>, Added<Pressed>)>),
    >,
) {
    for (hovered, pressed, mut background) in &mut query {
        background.0 = close_button_background(hovered.0, pressed);
    }
}

fn update_close_button_style_released(
    mut removed_pressed: RemovedComponents<Pressed>,
    mut query: Query<(&Hovered, Has<Pressed>, &mut BackgroundColor), With<CloseButton>>,
) {
    for entity in removed_pressed.read() {
        let Ok((hovered, pressed, mut background)) = query.get_mut(entity) else {
            continue;
        };

        background.0 = close_button_background(hovered.0, pressed);
    }
}

pub struct TitleBarPlugin;

impl Plugin for TitleBarPlugin {
    fn build(&self, app: &mut App) {
        if !app.is_plugin_added::<IconPlugin>() {
            app.add_plugins(IconPlugin);
        }

        app.world_mut()
            .resource_mut::<EmbeddedAssetRegistry>()
            .insert_asset(
                PathBuf::from(concat!(
                    env!("CARGO_MANIFEST_DIR"),
                    "/assets/icons/minimize.svg"
                )),
                Path::new("bevy_widgetry_window/icons/minimize.svg"),
                include_bytes!("../assets/icons/minimize.svg"),
            );

        app.world_mut()
            .resource_mut::<EmbeddedAssetRegistry>()
            .insert_asset(
                PathBuf::from(concat!(
                    env!("CARGO_MANIFEST_DIR"),
                    "/assets/icons/maximize.svg"
                )),
                Path::new("bevy_widgetry_window/icons/maximize.svg"),
                include_bytes!("../assets/icons/maximize.svg"),
            );

        app.world_mut()
            .resource_mut::<EmbeddedAssetRegistry>()
            .insert_asset(
                PathBuf::from(concat!(
                    env!("CARGO_MANIFEST_DIR"),
                    "/assets/icons/restore.svg"
                )),
                Path::new("bevy_widgetry_window/icons/restore.svg"),
                include_bytes!("../assets/icons/restore.svg"),
            );

        app.world_mut()
            .resource_mut::<EmbeddedAssetRegistry>()
            .insert_asset(
                PathBuf::from(concat!(
                    env!("CARGO_MANIFEST_DIR"),
                    "/assets/icons/close.svg"
                )),
                Path::new("bevy_widgetry_window/icons/close.svg"),
                include_bytes!("../assets/icons/close.svg"),
            );

        app.add_observer(on_minimize)
            .add_observer(on_maximize_restore)
            .add_observer(on_close)
            .add_observer(on_title_bar_press)
            .add_observer(on_window_resize_press)
            .add_systems(
                Update,
                (
                    sync_maximize_state,
                    update_window_control_style_changed,
                    update_window_control_style_released,
                    update_close_button_style_changed,
                    update_close_button_style_released,
                ),
            );
    }
}

fn on_minimize(
    event: On<Activate>,
    buttons: Query<&ChildOf, With<MinimizeButton>>,
    parents: Query<&ChildOf>,
    title_bars: Query<&TitleBar>,
    mut windows: Query<&mut Window>,
) {
    let Ok(controls_parent) = buttons.get(event.entity) else {
        return;
    };

    let Ok(title_bar_parent) = parents.get(controls_parent.parent()) else {
        return;
    };

    let Ok(title_bar) = title_bars.get(title_bar_parent.parent()) else {
        return;
    };

    let Ok(mut window) = windows.get_mut(title_bar.target_window) else {
        return;
    };

    window.set_minimized(true);
}

fn on_maximize_restore(
    event: On<Activate>,
    _non_send_marker: NonSendMarker,
    buttons: Query<&ChildOf, With<MaximizeButton>>,
    parents: Query<&ChildOf>,
    title_bars: Query<&TitleBar>,
    mut windows: Query<&mut Window>,
) {
    let Ok(controls_parent) = buttons.get(event.entity) else {
        return;
    };

    let Ok(title_bar_parent) = parents.get(controls_parent.parent()) else {
        return;
    };

    let Ok(title_bar) = title_bars.get(title_bar_parent.parent()) else {
        return;
    };

    let actual_maximized = WINIT_WINDOWS.with_borrow(|winit_windows| {
        winit_windows
            .get_window(title_bar.target_window)
            .map(|window| window.is_maximized())
    });

    let Some(actual_maximized) = actual_maximized else {
        return;
    };

    let Ok(mut window) = windows.get_mut(title_bar.target_window) else {
        return;
    };

    window.set_maximized(!actual_maximized);
}

fn on_close(
    event: On<Activate>,
    buttons: Query<&ChildOf, With<CloseButton>>,
    parents: Query<&ChildOf>,
    title_bars: Query<&TitleBar>,
    mut close_requests: MessageWriter<WindowCloseRequested>,
) {
    let Ok(controls_parent) = buttons.get(event.entity) else {
        return;
    };

    let Ok(title_bar_parent) = parents.get(controls_parent.parent()) else {
        return;
    };

    let Ok(title_bar) = title_bars.get(title_bar_parent.parent()) else {
        return;
    };

    info!("on_close");

    close_requests.write(WindowCloseRequested {
        window: title_bar.target_window,
    });
}

fn sync_maximize_state(
    _non_send_marker: NonSendMarker,
    parents: Query<&ChildOf>,
    title_bars: Query<&TitleBar>,
    children: Query<&Children>,
    buttons: Query<Entity, With<MaximizeButton>>,
    mut icons: Query<&mut Icon>,
    asset_server: Res<AssetServer>,
) {
    WINIT_WINDOWS.with_borrow(|winit_windows| {
        for button_entity in &buttons {
            // MaximizeButton -> WindowControls
            let Ok(controls_parent) = parents.get(button_entity) else {
                continue;
            };

            // WindowControls -> TitleBar
            let Ok(title_bar_parent) = parents.get(controls_parent.parent()) else {
                continue;
            };

            let Ok(title_bar) = title_bars.get(title_bar_parent.parent()) else {
                continue;
            };

            // 找到这个 TitleBar 实际控制的原生窗口
            let Some(winit_window) = winit_windows.get_window(title_bar.target_window) else {
                continue;
            };

            let maximized = winit_window.is_maximized();

            // MaximizeButton -> Icon
            let Ok(button_children) = children.get(button_entity) else {
                continue;
            };

            for child in button_children.iter() {
                let Ok(mut icon) = icons.get_mut(child) else {
                    continue;
                };

                if maximized {
                    icon.set_svg(
                        &asset_server,
                        "embedded://bevy_widgetry_window/icons/restore.svg",
                    );
                } else {
                    icon.set_svg(
                        &asset_server,
                        "embedded://bevy_widgetry_window/icons/maximize.svg",
                    );
                }

                break;
            }
        }
    });
}

fn on_title_bar_press(
    event: On<Pointer<Press>>,
    contents: Query<&ChildOf, With<TitleBarContent>>,
    title_bars: Query<&TitleBar>,
    mut windows: Query<&mut Window>,
) {
    if event.button != PointerButton::Primary {
        return;
    }

    // event.entity 此时是 TitleBarContent。
    let Ok(title_bar_parent) = contents.get(event.entity) else {
        return;
    };

    let title_bar_entity = title_bar_parent.parent();

    let Ok(title_bar) = title_bars.get(title_bar_entity) else {
        return;
    };

    let Ok(mut window) = windows.get_mut(title_bar.target_window) else {
        return;
    };

    window.start_drag_move();
}

impl TitleBar {
    pub fn spawn(
        commands: &mut Commands,
        asset_server: &AssetServer,
        target_window: Entity,
        content: impl FnOnce(&mut ChildSpawnerCommands<'_>),
    ) -> Entity {
        let mut title_bar = commands.spawn(TitleBar { target_window });

        let entity = title_bar.id();

        title_bar.with_children(|title_bar| {
            title_bar.spawn(TitleBarContent).with_children(content);

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

        entity
    }
}

#[derive(Component)]
#[require(
    Node = window_resize_area_node(),
    Pickable = Pickable::IGNORE,
)]
pub struct WindowResizeArea {
    pub target_window: Entity,
}

fn window_resize_area_node() -> Node {
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
pub struct WindowResizeHandle {
    pub direction: CompassOctant,
}

fn on_window_resize_press(
    event: On<Pointer<Press>>,
    handles: Query<(&WindowResizeHandle, &ChildOf)>,
    resize_areas: Query<&WindowResizeArea>,
    mut windows: Query<&mut Window>,
) {
    if event.button != PointerButton::Primary {
        return;
    }

    let Ok((handle, parent)) = handles.get(event.entity) else {
        return;
    };

    let Ok(resize_area) = resize_areas.get(parent.parent()) else {
        return;
    };

    let Ok(mut window) = windows.get_mut(resize_area.target_window) else {
        return;
    };

    window.start_drag_resize(handle.direction);
}

fn east_resize_handle_node() -> Node {
    Node {
        position_type: PositionType::Absolute,
        right: px(0),
        top: px(0),
        width: px(6),
        height: percent(100),
        ..default()
    }
}

impl WindowResizeArea {
    pub fn spawn(commands: &mut Commands, target_window: Entity) -> Entity {
        commands
            .spawn(WindowResizeArea { target_window })
            .with_children(|area| {
                area.spawn((
                    WindowResizeHandle {
                        direction: CompassOctant::East,
                    },
                    east_resize_handle_node(),
                ));
            })
            .id()
    }
}
