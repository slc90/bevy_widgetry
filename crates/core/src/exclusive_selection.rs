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

    pub fn reset(world: &mut World, root: Entity) -> Result<(), BevyError> {
        world
            .get_entity_mut(root)
            .map_err(BevyError::error)?
            .insert(Self::default());
        Ok(())
    }

    pub fn repair(
        world: &mut World,
        root: Entity,
        children: &[Entity],
        initial: InitialSelection,
    ) -> Result<(), BevyError> {
        let first = children
            .first()
            .copied()
            .ok_or_else(|| BevyError::error("Exclusive selection requires at least one option"))?;
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
                        return Err(BevyError::error(
                            "Exclusive selection has multiple initial Checked options",
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
            .map_err(BevyError::error)?
            .insert(Self {
                order: children.to_vec(),
                removed_selection: None,
            });
        Ok(())
    }

    pub fn select(world: &mut World, children: &[Entity], target: Entity) -> Result<(), BevyError> {
        for &child in children {
            let checked = world.get::<Checked>(child).is_some();
            if child == target && !checked {
                world
                    .get_entity_mut(child)
                    .map_err(BevyError::error)?
                    .insert(Checked);
            } else if child != target && checked {
                world
                    .get_entity_mut(child)
                    .map_err(BevyError::error)?
                    .remove::<Checked>();
            }
        }
        Ok(())
    }
}
