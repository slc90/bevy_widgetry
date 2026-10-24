use crate::WidgetryCollapsible;
use crate::collapsible::{Expanded, Parts};
use bevy::prelude::*;
use bevy_widgetry_internal_log::widgetry_error;

#[derive(EntityEvent, Debug, Clone, Copy)]
pub struct WidgetryCollapsibleChanged {
    pub entity: Entity,
    pub expanded: bool,
}

pub(crate) fn invalid(reason: &str) -> BevyError {
    widgetry_error!(reason, "Collapsible 请求或结构无效");
    bevy_widgetry_core::scene::logged_error(reason)
}

pub(crate) fn structure(world: &World, root: Entity) -> Result<Parts, BevyError> {
    if world.get::<WidgetryCollapsible>(root).is_none() || world.get::<Expanded>(root).is_none() {
        return Err(invalid("WidgetryCollapsible root or state is missing"));
    }
    let parts = *world
        .get::<Parts>(root)
        .ok_or_else(|| invalid("Collapsible parts are missing"))?;
    for (child, parent) in [
        (parts.header, root),
        (parts.content, root),
        (parts.header_content, parts.header),
        (parts.button, parts.header),
    ] {
        if world.get::<Node>(child).is_none()
            || world.get::<Visibility>(child).is_none()
            || world
                .get::<ChildOf>(child)
                .is_none_or(|p| p.parent() != parent)
        {
            return Err(invalid("Collapsible internal hierarchy is invalid"));
        }
    }
    if world
        .get::<bevy_widgetry_button::WidgetryButton>(parts.button)
        .is_none()
    {
        return Err(invalid("Collapsible toggle button is missing"));
    }
    Ok(parts)
}

pub(crate) fn install(app: &mut App) {
    app.init_resource::<Pending>()
        .add_observer(|event: On<Insert<Parts>>, mut pending: ResMut<Pending>| {
            pending.0.push(event.entity);
        })
        .add_systems(
            PreUpdate,
            initialize
                .before(bevy::input_focus::InputFocusSystems::Dispatch)
                .before(bevy::picking::PickingSystems::Hover),
        )
        .add_systems(
            PostUpdate,
            initialize.in_set(bevy_widgetry_core::ui::WidgetryUiSystems::Build),
        );
    let world = app.world_mut();
    let roots = world
        .query_filtered::<Entity, With<WidgetryCollapsible>>()
        .iter(world)
        .collect::<Vec<_>>();
    world.resource_mut::<Pending>().0.extend(roots);
}

#[derive(Resource, Default)]
struct Pending(Vec<Entity>);

fn initialize(world: &mut World) -> Result<(), BevyError> {
    let roots = std::mem::take(&mut world.resource_mut::<Pending>().0);
    let mut failure = None;
    for root in roots {
        if world.get::<WidgetryCollapsible>(root).is_none() {
            continue;
        }
        match structure(world, root) {
            Ok(parts) => {
                let expanded = world
                    .get::<Expanded>(root)
                    .ok_or_else(|| invalid("Collapsible state is missing"))?
                    .0;
                project(world, parts.content, expanded);
            }
            Err(error) => {
                if failure.is_none() {
                    failure = Some(error);
                }
            }
        }
    }
    match failure {
        Some(error) => Err(error),
        None => Ok(()),
    }
}

fn project(world: &mut World, content: Entity, expanded: bool) {
    world.entity_mut(content).insert(if expanded {
        Visibility::Inherited
    } else {
        Visibility::Hidden
    });
    if let Some(mut node) = world.get_mut::<Node>(content) {
        node.display = if expanded {
            Display::Flex
        } else {
            Display::None
        };
    }
    // Focus navigation can precede PostUpdate visibility propagation after a setter.
    let mut ancestor = world.get::<ChildOf>(content).map(ChildOf::parent);
    let mut parent_visible = true;
    while let Some(entity) = ancestor {
        match world.get::<Visibility>(entity) {
            Some(Visibility::Hidden) => {
                parent_visible = false;
                break;
            }
            Some(Visibility::Inherited) => {
                ancestor = world.get::<ChildOf>(entity).map(ChildOf::parent)
            }
            _ => break,
        }
    }
    let mut pending = vec![(content, parent_visible)];
    while let Some((entity, parent_visible)) = pending.pop() {
        let visible = match world.get::<Visibility>(entity) {
            Some(Visibility::Hidden) => false,
            Some(Visibility::Visible) => true,
            Some(Visibility::Inherited) => parent_visible,
            None => continue,
        };
        if let Some(mut inherited) = world.get_mut::<InheritedVisibility>(entity) {
            inherited.set_if_neq(if visible {
                InheritedVisibility::VISIBLE
            } else {
                InheritedVisibility::HIDDEN
            });
        }
        if let Some(children) = world.get::<Children>(entity) {
            pending.extend(children.iter().map(|child| (child, visible)));
        }
    }
}

impl WidgetryCollapsible {
    pub fn expanded(world: &World, root: Entity) -> Result<bool, BevyError> {
        structure(world, root)?;
        Ok(world
            .get::<Expanded>(root)
            .ok_or_else(|| invalid("Collapsible state is missing"))?
            .0)
    }

    pub fn set_expanded(commands: &mut Commands, root: Entity, expanded: bool) {
        commands.queue(move |world: &mut World| {
            Self::set_expanded_in_world(world, root, expanded).map(|_| ())
        });
    }

    pub fn set_expanded_in_world(
        world: &mut World,
        root: Entity,
        expanded: bool,
    ) -> Result<bool, BevyError> {
        let parts = structure(world, root)?;
        let mut state = world
            .get_mut::<Expanded>(root)
            .ok_or_else(|| invalid("Collapsible state is missing"))?;
        if state.0 == expanded {
            return Ok(false);
        }
        state.0 = expanded;
        project(world, parts.content, expanded);
        world.trigger(WidgetryCollapsibleChanged {
            entity: root,
            expanded,
        });
        Ok(true)
    }
}
