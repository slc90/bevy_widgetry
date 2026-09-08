use crate::{WindowRoot, title_bar::TitleBarContent, window_root::find_window_root};
use bevy::{
    ecs::{hierarchy::ChildOf, observer::On, query::With, system::Query},
    picking::{
        events::{Pointer, Press},
        pointer::PointerButton,
    },
    window::Window,
};

pub(super) fn on_title_bar_press(
    event: On<Pointer<Press>>,
    contents: Query<(), With<TitleBarContent>>,
    parents: Query<&ChildOf>,
    roots: Query<&WindowRoot>,
    mut windows: Query<&mut Window>,
) {
    if event.button != PointerButton::Primary {
        return;
    }

    if contents.get(event.entity).is_err() {
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
