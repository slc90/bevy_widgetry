use crate::window_root::{WindowRoot, find_window_root};
use bevy::ecs::query::With;
use bevy::ecs::system::{Commands, Res};
use bevy::input::ButtonInput;
use bevy::input::mouse::MouseButton;
use bevy::picking::events::{Out, Over};
use bevy::window::{CursorIcon, SystemCursorIcon};
use bevy::{
    ecs::{
        component::Component,
        entity::Entity,
        hierarchy::{ChildOf, ChildSpawnerCommands},
        observer::On,
        system::Query,
    },
    math::CompassOctant,
    picking::{
        Pickable,
        events::{Pointer, Press},
        pointer::PointerButton,
    },
    ui::{Node, PositionType, percent, px},
    utils::default,
    window::Window,
};

const RESIZE_HANDLE_SIZE: f32 = 6.0;
const RESIZE_DIRECTIONS: [CompassOctant; 8] = [
    CompassOctant::North,
    CompassOctant::NorthEast,
    CompassOctant::East,
    CompassOctant::SouthEast,
    CompassOctant::South,
    CompassOctant::SouthWest,
    CompassOctant::West,
    CompassOctant::NorthWest,
];

#[derive(Component)]
#[require(
    Node = window_resize_area_node(),
    Pickable = Pickable::IGNORE,
)]
pub struct WindowResizeArea;

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

#[derive(Component)]
pub(super) struct Resizing;

impl WindowResizeArea {
    pub fn spawn(parent: &mut ChildSpawnerCommands<'_>) -> Entity {
        let mut area = parent.spawn(WindowResizeArea);
        let entity = area.id();

        area.with_children(|area| {
            for direction in RESIZE_DIRECTIONS {
                area.spawn((
                    WindowResizeHandle { direction },
                    resize_handle_node(direction),
                ));
            }
        });

        entity
    }
}

fn resize_handle_node(direction: CompassOctant) -> Node {
    let size = px(RESIZE_HANDLE_SIZE);

    match direction {
        CompassOctant::North => Node {
            position_type: PositionType::Absolute,
            left: size,
            right: size,
            top: px(0),
            height: size,
            ..default()
        },

        CompassOctant::NorthEast => Node {
            position_type: PositionType::Absolute,
            right: px(0),
            top: px(0),
            width: size,
            height: size,
            ..default()
        },

        CompassOctant::East => Node {
            position_type: PositionType::Absolute,
            right: px(0),
            top: size,
            bottom: size,
            width: size,
            ..default()
        },

        CompassOctant::SouthEast => Node {
            position_type: PositionType::Absolute,
            right: px(0),
            bottom: px(0),
            width: size,
            height: size,
            ..default()
        },

        CompassOctant::South => Node {
            position_type: PositionType::Absolute,
            left: size,
            right: size,
            bottom: px(0),
            height: size,
            ..default()
        },

        CompassOctant::SouthWest => Node {
            position_type: PositionType::Absolute,
            left: px(0),
            bottom: px(0),
            width: size,
            height: size,
            ..default()
        },

        CompassOctant::West => Node {
            position_type: PositionType::Absolute,
            left: px(0),
            top: size,
            bottom: size,
            width: size,
            ..default()
        },

        CompassOctant::NorthWest => Node {
            position_type: PositionType::Absolute,
            left: px(0),
            top: px(0),
            width: size,
            height: size,
            ..default()
        },
    }
}

pub(super) fn on_window_resize_press(
    event: On<Pointer<Press>>,
    handles: Query<&WindowResizeHandle>,
    parents: Query<&ChildOf>,
    roots: Query<&WindowRoot>,
    mut windows: Query<&mut Window>,
    mut commands: Commands,
) {
    if event.button != PointerButton::Primary {
        return;
    }

    let Ok(handle) = handles.get(event.entity) else {
        return;
    };

    let Some(root) = find_window_root(event.entity, &parents, &roots) else {
        return;
    };

    commands.entity(event.entity).insert(Resizing);

    let Ok(mut window) = windows.get_mut(root.target_window) else {
        return;
    };

    window.start_drag_resize(handle.direction);
}

pub(super) fn on_window_resize_over(
    event: On<Pointer<Over>>,
    handles: Query<&WindowResizeHandle>,
    parents: Query<&ChildOf>,
    roots: Query<&WindowRoot>,
    mut commands: Commands,
) {
    let Ok(handle) = handles.get(event.entity) else {
        return;
    };

    let Some(root) = find_window_root(event.entity, &parents, &roots) else {
        return;
    };

    commands
        .entity(root.target_window)
        .insert(CursorIcon::System(resize_cursor(handle.direction)));
}

pub(super) fn on_window_resize_out(
    event: On<Pointer<Out>>,
    handles: Query<(), With<WindowResizeHandle>>,
    resizing: Query<(), With<Resizing>>,
    parents: Query<&ChildOf>,
    roots: Query<&WindowRoot>,
    mut commands: Commands,
) {
    let Ok(()) = handles.get(event.entity) else {
        return;
    };

    // 原生 resize 刚开始导致的 Out，忽略
    if resizing.contains(event.entity) {
        return;
    }

    let Some(root) = find_window_root(event.entity, &parents, &roots) else {
        return;
    };

    commands
        .entity(root.target_window)
        .insert(CursorIcon::System(SystemCursorIcon::Default));
}

fn resize_cursor(direction: CompassOctant) -> SystemCursorIcon {
    match direction {
        CompassOctant::North => SystemCursorIcon::NResize,
        CompassOctant::NorthEast => SystemCursorIcon::NeResize,
        CompassOctant::East => SystemCursorIcon::EResize,
        CompassOctant::SouthEast => SystemCursorIcon::SeResize,
        CompassOctant::South => SystemCursorIcon::SResize,
        CompassOctant::SouthWest => SystemCursorIcon::SwResize,
        CompassOctant::West => SystemCursorIcon::WResize,
        CompassOctant::NorthWest => SystemCursorIcon::NwResize,
    }
}

pub(super) fn finish_window_resize(
    mouse: Res<ButtonInput<MouseButton>>,
    mut commands: Commands,
    resizing: Query<Entity, With<Resizing>>,
) {
    if !mouse.just_released(MouseButton::Left) {
        return;
    }

    for entity in &resizing {
        commands.entity(entity).remove::<Resizing>();
    }
}
