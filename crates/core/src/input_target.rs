use bevy::prelude::*;

#[derive(Component, FromTemplate, Clone, Copy)]
pub struct WidgetryInputTarget {
    pub target: Entity,
}
