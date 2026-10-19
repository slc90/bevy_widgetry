use crate::{
    WidgetryTreeModel, WidgetryTreeState, WidgetryTreeView, WidgetryTreeVisibleItem,
    behavior::contains_node,
};
use bevy::{ecs::change_detection::Tick, prelude::*, ui::InteractionDisabled};
use bevy_widgetry_list_view::{WidgetryListData, WidgetryListDataSource, WidgetryListValue};

pub(crate) struct TreeSource;

impl WidgetryListDataSource<WidgetryTreeVisibleItem, Entity> for TreeSource {
    fn changed_at(world: &World, source: Entity) -> Option<Tick> {
        let view = world.get::<WidgetryTreeView>(source)?;
        world.get::<WidgetryTreeModel>(view.source())?;
        Some(
            world
                .get_entity(source)
                .ok()?
                .get_ref::<WidgetryTreeState>()?
                .last_changed(),
        )
    }

    fn ids(world: &World, source: Entity) -> Option<Vec<Entity>> {
        Some(
            world
                .get::<WidgetryTreeState>(source)?
                .visible_items()
                .iter()
                .map(|item| item.entity)
                .collect(),
        )
    }

    fn contains(world: &World, source: Entity, id: Entity) -> bool {
        world
            .get::<WidgetryTreeView>(source)
            .is_some_and(|view| contains_node(world, view.source(), id))
    }

    fn row(
        world: &World,
        source: Entity,
        id: Entity,
    ) -> Option<WidgetryListData<'_, WidgetryTreeVisibleItem>> {
        if !world
            .get::<WidgetryTreeView>(source)
            .is_some_and(|view| contains_node(world, view.source(), id))
        {
            return None;
        }
        let tree = world.get::<WidgetryTreeState>(source)?;
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
    use crate::{WidgetryTreeChildrenState, WidgetryTreeNode, WidgetryTreePlugin};

    #[test]
    fn adapter_reads_view_metadata_and_rejects_live_invalid_membership() {
        let mut app = bevy_widgetry_test_utils::scene_app();
        app.add_plugins(WidgetryTreePlugin);
        let root = app.world_mut().spawn_empty().id();
        let node = app
            .world_mut()
            .spawn((
                WidgetryTreeNode,
                ChildOf(root),
                WidgetryTreeChildrenState::Unknown,
            ))
            .id();
        let leaf = app
            .world_mut()
            .spawn((WidgetryTreeNode, ChildOf(node)))
            .id();
        let model = app.world_mut().spawn(WidgetryTreeModel::new(root)).id();
        let view = app
            .world_mut()
            .spawn_scene(bsn! { @WidgetryTreeView { @source: model } })
            .unwrap()
            .id();
        app.update();
        assert_eq!(TreeSource::ids(app.world(), view), Some(vec![node]));
        assert!(TreeSource::contains(app.world(), view, leaf));
        assert!(TreeSource::row(app.world(), view, leaf).is_none());
        let revision = TreeSource::row(app.world(), view, node).unwrap().revision;
        WidgetryTreeView::expand(app.world_mut(), view, node).unwrap();
        app.world_mut().entity_mut(node).insert(InteractionDisabled);
        let row = TreeSource::row(app.world(), view, node).unwrap();
        assert!(row.disabled && row.value.as_ref().expanded && row.value.as_ref().has_children);
        assert_ne!(row.revision, revision);
        assert_eq!(
            TreeSource::row(app.world(), view, leaf)
                .unwrap()
                .value
                .as_ref()
                .depth,
            1
        );
        app.world_mut()
            .entity_mut(node)
            .remove::<WidgetryTreeNode>();
        assert!(!TreeSource::contains(app.world(), view, leaf));
        assert!(TreeSource::row(app.world(), view, leaf).is_none());
        app.world_mut().entity_mut(node).insert(WidgetryTreeNode);
        app.world_mut().entity_mut(leaf).remove::<ChildOf>();
        assert!(!TreeSource::contains(app.world(), view, leaf));
        assert!(TreeSource::row(app.world(), view, leaf).is_none());
        let saved = app
            .world_mut()
            .entity_mut(model)
            .take::<WidgetryTreeModel>()
            .unwrap();
        assert!(TreeSource::changed_at(app.world(), view).is_none());
        app.world_mut().entity_mut(model).insert(saved);
        app.update();
        assert!(TreeSource::changed_at(app.world(), view).is_some());
        assert_eq!(TreeSource::ids(app.world(), view), Some(vec![node]));
    }
}
