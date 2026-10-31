use crate::diagnostics::{CoreError, CoreFailure, failure};
use bevy::prelude::*;
use bevy::ui::Checked;

pub enum InitialSelection {
    First,
    Checked,
}

#[derive(Component, Default)]
pub struct SelectionHistory {
    order: Vec<Entity>,
    removed_selection: Option<Entity>,
}

impl SelectionHistory {
    pub fn remember_removed(&mut self, entity: Entity) {
        if self.order.contains(&entity) {
            self.removed_selection = Some(entity);
        }
    }

    pub fn reset(world: &mut World, root: Entity) -> Result<(), CoreFailure> {
        world
            .get_entity_mut(root)
            .map_err(|source| {
                failure(
                    CoreError::EntityAccess {
                        entity: root,
                        source,
                    },
                    "update selection history",
                )
            })?
            .insert(Self::default());
        Ok(())
    }

    pub fn repair(
        world: &mut World,
        root: Entity,
        children: &[Entity],
        initial: InitialSelection,
    ) -> Result<(), CoreFailure> {
        world.get_entity_mut(root).map_err(|source| {
            failure(
                CoreError::EntityAccess {
                    entity: root,
                    source,
                },
                "repair selection history",
            )
        })?;
        let first = children.first().copied().ok_or_else(|| {
            failure(
                CoreError::EmptySelection { root },
                "repair exclusive selection",
            )
        })?;
        let target = match world.get::<Self>(root) {
            None => match initial {
                InitialSelection::First => first,
                InitialSelection::Checked => {
                    let mut checked = children
                        .iter()
                        .copied()
                        .filter(|child| world.get::<Checked>(*child).is_some());
                    let target = checked.next().unwrap_or(first);
                    if checked.next().is_some() {
                        return Err(failure(
                            CoreError::MultipleChecked {
                                root,
                                options: children
                                    .iter()
                                    .copied()
                                    .filter(|child| world.get::<Checked>(*child).is_some())
                                    .collect(),
                            },
                            "initialize exclusive selection",
                        ));
                    }
                    target
                }
            },
            Some(history) => history
                .order
                .iter()
                .copied()
                .find(|child| children.contains(child) && world.get::<Checked>(*child).is_some())
                .or_else(|| {
                    let index = history
                        .order
                        .iter()
                        .position(|child| Some(*child) == history.removed_selection)?;
                    history.order[index + 1..]
                        .iter()
                        .chain(history.order[..index].iter().rev())
                        .copied()
                        .find(|child| children.contains(child))
                })
                .unwrap_or(first),
        };
        Self::select(world, children, target)?;
        world
            .get_entity_mut(root)
            .map_err(|source| {
                failure(
                    CoreError::EntityAccess {
                        entity: root,
                        source,
                    },
                    "update selection history",
                )
            })?
            .insert(Self {
                order: children.to_vec(),
                removed_selection: None,
            });
        Ok(())
    }

    pub fn select(
        world: &mut World,
        children: &[Entity],
        target: Entity,
    ) -> Result<(), CoreFailure> {
        if !children.contains(&target) {
            return Err(failure(
                CoreError::InvalidSelection { target },
                "select exclusive option",
            ));
        }
        for &child in children {
            world.get_entity_mut(child).map_err(|source| {
                failure(
                    CoreError::EntityAccess {
                        entity: child,
                        source,
                    },
                    "validate exclusive option",
                )
            })?;
        }
        for &child in children {
            let checked = world.get::<Checked>(child).is_some();
            if child == target && !checked {
                world
                    .get_entity_mut(child)
                    .map_err(|source| {
                        failure(
                            CoreError::EntityAccess {
                                entity: child,
                                source,
                            },
                            "select exclusive option",
                        )
                    })?
                    .insert(Checked);
            } else if child != target && checked {
                world
                    .get_entity_mut(child)
                    .map_err(|source| {
                        failure(
                            CoreError::EntityAccess {
                                entity: child,
                                source,
                            },
                            "select exclusive option",
                        )
                    })?
                    .remove::<Checked>();
            }
        }
        Ok(())
    }
}
