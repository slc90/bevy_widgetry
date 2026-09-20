use crate::window_root::{WindowRoot, find_window_root};
use bevy::ecs::query::With;
use bevy::ecs::system::{Commands, Res};
use bevy::input::ButtonInput;
use bevy::input::mouse::MouseButton;
use bevy::picking::events::{Out, Over};
use bevy::prelude::{Children, Has, Scene, bsn, template};
use bevy::window::{CursorIcon, SystemCursorIcon};
use bevy::{
    ecs::{component::Component, entity::Entity, hierarchy::ChildOf, observer::On, system::Query},
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

/// resize hit area 的逻辑像素宽度，边与角共用。
const RESIZE_HANDLE_SIZE: f32 = 6.0;

/// 覆盖四边与四角，构造时每个方向只生成一个 hit area。
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

/// 覆盖 window 边缘的 resize 容器，自身不拦截 pointer picking。
#[derive(Component)]
#[require(
    Node = window_resize_area_node(),
    Pickable = Pickable::IGNORE,
)]
pub(crate) struct WindowResizeArea;

/// 用方向区分八个 resize hit area，以调用 native window resize。
#[derive(Component)]
pub(super) struct WindowResizeHandle {
    /// hit area 对应的 native resize 方向。
    direction: CompassOctant,
}

/// native resize 开始后保留 cursor，直到鼠标 release 再解除该 state。
#[derive(Component)]
pub(super) struct Resizing;

/// 将 resize 容器以 absolute positioning 覆盖整个 window UI，避免占用内容 layout 空间。
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

/// 按边或角设置 hit area，边区域避开角区域以明确 resize 方向。
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

/// 只接受主键 press，将命中方向传给关联 window 的 native resize。
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

    let Ok(mut window) = windows.get_mut(root.target_window) else {
        return;
    };

    if !window.resizable || root.maximized {
        return;
    }

    commands.entity(event.entity).insert(Resizing);
    window.start_drag_resize(handle.direction);
}

/// pointer 进入 resize hit area 时更新真实 window cursor 以指示方向。
pub(super) fn on_window_resize_over(
    event: On<Pointer<Over>>,
    windows: Query<&Window>,
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

    if root.maximized
        || !windows
            .get(root.target_window)
            .is_ok_and(|window| window.resizable)
    {
        return;
    }

    commands
        .entity(root.target_window)
        .insert(CursorIcon::System(resize_cursor(handle.direction)));
}

/// 离开 hit area 时恢复默认 cursor，native resize 过程中保留方向提示。
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

    // 忽略 native resize 刚开始导致的 Out。
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

/// 将八个几何方向映射为对应的系统 resize cursor。
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

/// 主键 release 后清除 resize marker，使后续 Out event 能够恢复 cursor。
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

/// 用 SceneList 一次性声明八个边缘区域，不混用命令式 entity 构造。
pub(crate) fn window_resize_area() -> impl Scene {
    bsn! {
        template(|_| Ok(WindowResizeArea))
        Children [{RESIZE_DIRECTIONS.into_iter().map(resize_handle).collect::<Vec<_>>()}]
    }
}

/// 将方向和 hit area 几何绑定在同一 Scene 中。
fn resize_handle(direction: CompassOctant) -> impl Scene {
    bsn! {
        template(move |_| Ok(WindowResizeHandle { direction }))
        template(move |_| Ok(resize_handle_node(direction)))
    }
}

/// 不可 resize 或 maximized 时穿透边缘 picking，同时撤销已 hover 或 drag 的 resize cursor。
pub(super) fn sync_resize_handles(
    handles: Query<(Entity, Option<&Pickable>, Has<Resizing>), With<WindowResizeHandle>>,
    parents: Query<&ChildOf>,
    roots: Query<&WindowRoot>,
    windows: Query<&Window>,
    mut commands: Commands,
) {
    for (entity, pickable, resizing) in &handles {
        let Some(root) = find_window_root(entity, &parents, &roots) else {
            continue;
        };
        let Ok(window) = windows.get(root.target_window) else {
            continue;
        };
        let enabled = window.resizable && !root.maximized;
        if pickable.is_none_or(|pickable| pickable.is_hoverable != enabled) {
            commands.entity(entity).insert(if enabled {
                Pickable::default()
            } else {
                Pickable::IGNORE
            });
            if !enabled {
                commands
                    .entity(root.target_window)
                    .insert(CursorIcon::System(SystemCursorIcon::Default));
            }
        }
        if !enabled && resizing {
            commands.entity(entity).remove::<Resizing>();
        }
    }
}
