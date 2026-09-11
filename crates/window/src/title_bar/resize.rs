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

/// 缩放命中区域的逻辑像素宽度，边与角共用。
const RESIZE_HANDLE_SIZE: f32 = 6.0;

/// 覆盖四边与四角，构造时每个方向只生成一个命中区域。
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

/// 覆盖窗口边缘的缩放容器，自身不拦截指针拾取。
#[derive(Component)]
#[require(
    Node = window_resize_area_node(),
    Pickable = Pickable::IGNORE,
)]
pub(crate) struct WindowResizeArea;

/// 用方向区分八个缩放命中区域，以调用原生窗口缩放。
#[derive(Component)]
pub(super) struct WindowResizeHandle {
    /// 命中区域对应的原生缩放方向。
    direction: CompassOctant,
}

/// 原生缩放开始后保留光标，直到鼠标释放再解除该状态。
#[derive(Component)]
pub(super) struct Resizing;

/// 将缩放容器绝对定位到整个窗口 UI，避免占用内容布局空间。
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

/// 按边或角设置命中区域，边区域避开角区域以明确缩放方向。
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

/// 只接受主键按压，将命中方向传给关联窗口的原生缩放。
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

/// 指针进入缩放命中区时更新真实窗口光标以指示方向。
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

/// 离开命中区时恢复默认光标，原生缩放过程中保留方向提示。
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

/// 将八个几何方向映射为对应的系统缩放光标。
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

/// 主键释放后清除缩放标记，使后续离开事件能够恢复光标。
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

impl WindowResizeArea {
    pub(crate) fn spawn(parent: &mut ChildSpawnerCommands<'_>) -> Entity {
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
