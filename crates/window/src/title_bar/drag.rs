use crate::{
    title_bar::bar::TitleBarDragArea,
    window_root::{WindowRoot, find_window_root},
};
use bevy::{
    ecs::{hierarchy::ChildOf, observer::On, query::With, system::Query},
    picking::{
        events::{Pointer, Press},
        pointer::PointerButton,
    },
    window::Window,
};

/// 将标题栏空白区域的主键按压交给原生窗口拖动。
pub(super) fn on_title_bar_press(
    event: On<Pointer<Press>>,
    drag_areas: Query<(), With<TitleBarDragArea>>,
    parents: Query<&ChildOf>,
    roots: Query<&WindowRoot>,
    mut windows: Query<&mut Window>,
) {
    if event.button != PointerButton::Primary {
        return;
    }

    if drag_areas.get(event.entity).is_err() {
        return;
    }

    let Some(root) = find_window_root(event.entity, &parents, &roots) else {
        return;
    };

    let Ok(mut window) = windows.get_mut(root.target_window) else {
        return;
    };

    window.start_drag_move();
}
