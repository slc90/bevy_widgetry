use crate::{WidgetryTreeModel, WidgetryTreeVisibleItem, behavior::contains_node};
use bevy::{ecs::change_detection::Tick, prelude::*, ui::InteractionDisabled};
use bevy_widgetry_list_view::{WidgetryListData, WidgetryListDataSource, WidgetryListValue};

pub(crate) struct TreeSource;

impl WidgetryListDataSource<WidgetryTreeVisibleItem, Entity> for TreeSource {
    fn changed_at(world: &World, source: Entity) -> Option<Tick> {
        Some(
            world
                .get_entity(source)
                .ok()?
                .get_ref::<WidgetryTreeModel>()?
                .last_changed(),
        )
    }

    fn ids(world: &World, source: Entity) -> Option<Vec<Entity>> {
        Some(
            world
                .get::<WidgetryTreeModel>(source)?
                .visible_items()
                .iter()
                .map(|item| item.entity)
                .collect(),
        )
    }

    fn contains(world: &World, source: Entity, id: Entity) -> bool {
        contains_node(world, source, id)
    }

    fn row(
        world: &World,
        source: Entity,
        id: Entity,
    ) -> Option<WidgetryListData<'_, WidgetryTreeVisibleItem>> {
        if !contains_node(world, source, id) {
            return None;
        }
        let tree = world.get::<WidgetryTreeModel>(source)?;
        let item = *tree.visible_items().get(tree.visible_index(id)?)?;
        let revision = u64::from(item.depth) << 2
            | u64::from(item.has_children) << 1
            | u64::from(item.expanded);
        Some(WidgetryListData {
            value: WidgetryListValue::Owned(item),
            disabled: world.get::<InteractionDisabled>(id).is_some(),
            revision: Some(revision),
        })
    }
}

#[cfg(test)]
#[allow(clippy::disallowed_macros)]
mod tests {
    use super::*;
    use crate::{WidgetryTreeChildrenState, WidgetryTreeNode};

    #[test]
    fn hidden_nodes_remain_members_but_detached_or_unmarked_nodes_do_not() {
        let mut world = World::new();
        let root = world.spawn_empty().id();
        let branch = world.spawn((WidgetryTreeNode, ChildOf(root))).id();
        let leaf = world.spawn((WidgetryTreeNode, ChildOf(branch))).id();
        let mut tree = WidgetryTreeModel::new(root);
        tree.refresh(&world).unwrap();
        let source = world.spawn(tree).id();
        assert_eq!(TreeSource::ids(&world, source), Some(vec![branch]));
        assert!(TreeSource::contains(&world, source, leaf));
        assert!(TreeSource::row(&world, source, leaf).is_none());
        WidgetryTreeModel::expand(&mut world, source, branch).unwrap();
        assert_eq!(TreeSource::ids(&world, source), Some(vec![branch, leaf]));
        let row = TreeSource::row(&world, source, leaf).unwrap();
        assert_eq!(row.value.as_ref().entity, leaf);
        assert_eq!(row.value.as_ref().depth, 1);
        world.entity_mut(branch).remove::<WidgetryTreeNode>();
        assert!(!TreeSource::contains(&world, source, leaf));
        assert!(TreeSource::row(&world, source, leaf).is_none());
        world.entity_mut(branch).insert(WidgetryTreeNode);
        world.entity_mut(leaf).remove::<ChildOf>();
        assert!(!TreeSource::contains(&world, source, leaf));
        assert!(TreeSource::row(&world, source, leaf).is_none());
    }

    #[test]
    fn row_metadata_tracks_disabled_and_lazy_children_without_an_intermediate_model() {
        let mut world = World::new();
        let root = world.spawn_empty().id();
        let node = world
            .spawn((
                WidgetryTreeNode,
                ChildOf(root),
                WidgetryTreeChildrenState::Unknown,
            ))
            .id();
        let mut tree = WidgetryTreeModel::new(root);
        tree.refresh(&world).unwrap();
        let source = world.spawn(tree).id();
        let before = TreeSource::row(&world, source, node).unwrap();
        assert!(before.value.as_ref().has_children);
        assert!(!before.disabled);
        let revision = before.revision;
        WidgetryTreeModel::expand(&mut world, source, node).unwrap();
        world.entity_mut(node).insert(InteractionDisabled);
        let after = TreeSource::row(&world, source, node).unwrap();
        assert!(after.disabled);
        assert!(after.value.as_ref().expanded);
        assert_ne!(after.revision, revision);
        assert!(
            world
                .get::<bevy_widgetry_list_view::WidgetryListModel<WidgetryTreeVisibleItem>>(source)
                .is_none()
        );
    }
}
