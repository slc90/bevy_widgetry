use crate::window_root::{WindowRoot, find_window_root};
use bevy::ecs::system::Commands;
use bevy::prelude::{Children, Scene, bsn, template};
use bevy::window::SystemCursorIcon;
use bevy::{
    ecs::entity::ContainsEntity,
    ecs::{component::Component, entity::Entity, hierarchy::ChildOf, observer::On, system::Query},
    math::CompassOctant,
    picking::{Pickable, events::PointerPress, pointer::PointerButton},
    ui::{Node, PositionType, percent, px},
    utils::default,
    window::Window,
};
use bevy_widgetry_cursor::CursorLease;

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
pub(crate) struct WindowResizeArea;

#[derive(Component)]
pub(super) struct WindowResizeHandle {
    direction: CompassOctant,
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
    event: On<PointerPress>,
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

    let Some(lease) =
        CursorLease::native(event.pointer.id, &event.pointer.location(), event.button)
    else {
        return;
    };
    if !matches!(event.pointer.target, bevy::camera::NormalizedRenderTarget::Window(target) if target.entity() == root.target_window)
    {
        return;
    }
    commands.entity(event.entity).insert(lease);
    window.start_drag_resize(handle.direction);
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

pub(crate) fn window_resize_area() -> impl Scene {
    bsn! {
        template(|_| Ok(WindowResizeArea))
        Children [{RESIZE_DIRECTIONS.into_iter().map(resize_handle).collect::<Vec<_>>()}]
    }
}

fn resize_handle(direction: CompassOctant) -> impl Scene {
    bsn! {
        template(move |_| Ok(WindowResizeHandle { direction }))
        template(move |_| Ok(bevy::picking::cursor::EntityCursor::System(resize_cursor(direction))))
        template(move |_| Ok(resize_handle_node(direction)))
    }
}

pub(super) fn sync_resize_handles(
    handles: Query<(Entity, Option<&Pickable>), bevy::prelude::With<WindowResizeHandle>>,
    parents: Query<&ChildOf>,
    roots: Query<&WindowRoot>,
    windows: Query<&Window>,
    mut commands: Commands,
) {
    for (entity, pickable) in &handles {
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
        }
        if !enabled {
            commands.entity(entity).remove::<CursorLease>();
        }
    }
}

// 测试断言需要在 contract 不满足时立即失败。
// 生产代码的 panic lint 会拒绝这些表达式，因此仅在本测试 scope 允许所列 lint。
#[cfg(test)]
#[allow(clippy::disallowed_macros)]
mod tests {
    use super::*;
    use crate::{
        WidgetryWindowBackground, WidgetryWindowControlsConfig, WidgetryWindowPlugin,
        owned_widgetry_window,
    };
    use bevy::picking::events::Pointer;
    use bevy::prelude::*;
    use bevy_widgetry_test_utils::{primary_press, scene_app};

    #[test]
    fn direction_geometry_and_cursor_match_all_eight_octants() {
        let auto = Val::Auto;
        for (direction, geometry, cursor) in [
            (
                CompassOctant::North,
                [px(6), px(6), px(0), auto, auto, px(6)],
                SystemCursorIcon::NResize,
            ),
            (
                CompassOctant::NorthEast,
                [auto, px(0), px(0), auto, px(6), px(6)],
                SystemCursorIcon::NeResize,
            ),
            (
                CompassOctant::East,
                [auto, px(0), px(6), px(6), px(6), auto],
                SystemCursorIcon::EResize,
            ),
            (
                CompassOctant::SouthEast,
                [auto, px(0), auto, px(0), px(6), px(6)],
                SystemCursorIcon::SeResize,
            ),
            (
                CompassOctant::South,
                [px(6), px(6), auto, px(0), auto, px(6)],
                SystemCursorIcon::SResize,
            ),
            (
                CompassOctant::SouthWest,
                [px(0), auto, auto, px(0), px(6), px(6)],
                SystemCursorIcon::SwResize,
            ),
            (
                CompassOctant::West,
                [px(0), auto, px(6), px(6), px(6), auto],
                SystemCursorIcon::WResize,
            ),
            (
                CompassOctant::NorthWest,
                [px(0), auto, px(0), auto, px(6), px(6)],
                SystemCursorIcon::NwResize,
            ),
        ] {
            let node = resize_handle_node(direction);
            assert_eq!(node.position_type, PositionType::Absolute);
            assert_eq!(
                [
                    node.left,
                    node.right,
                    node.top,
                    node.bottom,
                    node.width,
                    node.height
                ],
                geometry,
                "{direction:?}"
            );
            assert_eq!(resize_cursor(direction), cursor);
            assert_eq!(node.margin, UiRect::ZERO);
        }
        let area = window_resize_area_node();
        assert_eq!(area.position_type, PositionType::Absolute);
        assert_eq!((area.width, area.height), (percent(100), percent(100)));
    }

    fn fixture() -> (App, [(Entity, Entity, Entity); 2]) {
        let mut app = scene_app();
        app.add_plugins(WidgetryWindowPlugin);
        let roots: Vec<_> = (0..2).map(|_| app.world_mut().commands().spawn_scene(bsn! {
            @owned_widgetry_window(Window::default(), WidgetryWindowControlsConfig::default(), WidgetryWindowBackground::Theme, Default::default(),  bsn_list!{}, bsn_list!{})
        }).id()).collect();
        app.update();
        let mut bindings = Vec::new();
        for root in roots {
            let window = app.world().get::<WindowRoot>(root).unwrap().target_window;
            let handle = app
                .world_mut()
                .query::<(Entity, &WindowResizeHandle)>()
                .iter(app.world())
                .find(|(entity, handle)| {
                    if handle.direction != CompassOctant::NorthEast {
                        return false;
                    }
                    let mut current = *entity;
                    while let Some(parent) = app.world().get::<ChildOf>(current) {
                        current = parent.parent();
                    }
                    current == root
                })
                .unwrap()
                .0;
            bindings.push((root, window, handle));
        }
        (app, [bindings[0], bindings[1]])
    }

    #[test]
    fn resize_press_respects_native_resizable_and_window_binding() {
        let (mut app, [(_, first, handle), (_, second, _)]) = fixture();
        let mut secondary = primary_press(handle);
        secondary.button = PointerButton::Secondary;
        app.world_mut().trigger(secondary);
        app.world_mut().flush();
        assert_eq!(
            app.world_mut()
                .get_mut::<Window>(first)
                .unwrap()
                .internal
                .take_resize_request(),
            None
        );
        assert!(app.world().get::<CursorLease>(handle).is_none());
        app.world_mut().get_mut::<Window>(first).unwrap().resizable = false;
        let mut event = primary_press(handle);
        event.pointer = Pointer::new(
            bevy::picking::pointer::PointerId::Mouse,
            bevy::picking::pointer::Location {
                target: bevy::camera::RenderTarget::Window(bevy::window::WindowRef::Entity(first))
                    .normalize(None)
                    .unwrap(),
                position: Vec2::ONE,
            },
        );
        app.world_mut().trigger(event);
        app.world_mut().flush();
        assert_eq!(
            app.world_mut()
                .get_mut::<Window>(first)
                .unwrap()
                .internal
                .take_resize_request(),
            None
        );
        assert!(app.world().get::<CursorLease>(handle).is_none());
        app.world_mut().get_mut::<Window>(first).unwrap().resizable = true;
        let mut event = primary_press(handle);
        event.pointer = Pointer::new(
            bevy::picking::pointer::PointerId::Mouse,
            bevy::picking::pointer::Location {
                target: bevy::camera::RenderTarget::Window(bevy::window::WindowRef::Entity(first))
                    .normalize(None)
                    .unwrap(),
                position: Vec2::ONE,
            },
        );
        app.world_mut().trigger(event);
        app.world_mut().flush();
        assert_eq!(
            app.world_mut()
                .get_mut::<Window>(first)
                .unwrap()
                .internal
                .take_resize_request(),
            Some(CompassOctant::NorthEast)
        );
        assert!(app.world().get::<CursorLease>(handle).is_some());
        assert_eq!(
            app.world_mut()
                .get_mut::<Window>(second)
                .unwrap()
                .internal
                .take_resize_request(),
            None
        );
    }

    #[test]
    fn native_resize_ignores_non_mouse_and_invalid_target() {
        let (mut app, [(_, window, handle), (_, other, _)]) = fixture();
        for (id, target) in [
            (bevy_widgetry_test_utils::pointer_ids()[1], window),
            (bevy::picking::pointer::PointerId::Mouse, other),
        ] {
            let mut event = primary_press(handle);
            event.pointer = Pointer::new(
                id,
                bevy::picking::pointer::Location {
                    target: bevy::camera::RenderTarget::Window(bevy::window::WindowRef::Entity(
                        target,
                    ))
                    .normalize(None)
                    .unwrap(),
                    position: Vec2::ONE,
                },
            );
            app.world_mut().trigger(event);
            app.world_mut().flush();
            assert_eq!(
                app.world_mut()
                    .get_mut::<Window>(window)
                    .unwrap()
                    .internal
                    .take_resize_request(),
                None
            );
            assert!(
                app.world()
                    .get::<bevy_widgetry_cursor::CursorLease>(handle)
                    .is_none()
            );
        }
    }
}
