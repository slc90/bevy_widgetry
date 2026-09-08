use bevy::prelude::*;

#[derive(Component)]
#[require(Node = window_root_node())]
pub struct WindowRoot {
    pub target_window: Entity,
}

fn window_root_node() -> Node {
    Node {
        width: percent(100),
        height: percent(100),
        flex_direction: FlexDirection::Column,
        ..default()
    }
}

#[derive(Component)]
#[require(Node = window_content_node())]
pub struct WindowContent;

fn window_content_node() -> Node {
    Node {
        width: percent(100),
        flex_grow: 1.0,
        ..default()
    }
}

pub(super) fn find_window_root<'a>(
    entity: Entity,
    parents: &Query<&ChildOf>,
    roots: &'a Query<&WindowRoot>,
) -> Option<&'a WindowRoot> {
    parents
        .iter_ancestors(entity)
        .find_map(|ancestor| roots.get(ancestor).ok())
}
