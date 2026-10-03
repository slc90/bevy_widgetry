//! Coverage Map：本文件负责 headless state、增量 ListModel 与 lazy/external hierarchy mutation。
//! view.rs 负责真实 BSN 输入、focus、共享 source、virtualization 与 renderer/theme。
//! contract.rs 负责配置错误。
//! rendering.rs 负责真实 UI pipeline 的生成当帧消费。
//! facade 的 public_api.rs 负责消费者入口。
//! State：expanded/selected 与 Unknown/Loading/Loaded。
//! stimuli：公开 API、外部 hierarchy mutation。
//! Guard：失效 node、普通 leaf、重复操作。
//! invariant：Entity selection、未受影响 entry id/revision 保持。
//! Coupling：collapse 隐藏 selection，删除 node 清除 selection，lazy 请求不因 re-expand 重复。
//! 跨域 invariant：Entity 为唯一 UI authority，physical row 销毁不删除业务 node，选择先提交再通知，修复不发 Selected。

// 测试断言需要在 contract 不满足时立即失败。
// 生产代码的 panic lint 会拒绝这些表达式，因此仅在本测试 scope 允许所列 lint。
#![allow(clippy::disallowed_macros, clippy::expect_used, clippy::unwrap_used)]

use bevy::prelude::*;
use bevy_widgetry_list_view::WidgetryListModel;
use bevy_widgetry_test_utils::scene_app;
use bevy_widgetry_tree::{
    WidgetryTreeChildrenState, WidgetryTreeEvent, WidgetryTreeEventKind, WidgetryTreeModel,
    WidgetryTreeNode, WidgetryTreePlugin, WidgetryTreeVisibleItem,
};

#[derive(Resource, Default)]
struct Events(Vec<WidgetryTreeEventKind>);

fn fixture() -> (App, Entity, Entity, Entity, Entity) {
    let mut app = scene_app();
    app.add_plugins(WidgetryTreePlugin)
        .init_resource::<Events>();
    app.add_observer(|event: On<WidgetryTreeEvent>, mut events: ResMut<Events>| {
        events.0.push(event.kind)
    });
    let root = app.world_mut().spawn_empty().id();
    let a = app
        .world_mut()
        .spawn((WidgetryTreeNode, ChildOf(root)))
        .id();
    let b = app
        .world_mut()
        .spawn((WidgetryTreeNode, ChildOf(root)))
        .id();
    let c = app.world_mut().spawn((WidgetryTreeNode, ChildOf(a))).id();
    let source = app.world_mut().spawn(WidgetryTreeModel::new(root)).id();
    app.update();
    (app, source, a, b, c)
}

#[test]
fn selection_notifications_observe_committed_authority_and_clear() {
    let (mut app, source, a, _, c) = fixture();
    app.add_observer(
        |event: On<WidgetryTreeEvent>, trees: Query<&WidgetryTreeModel>| {
            if let WidgetryTreeEventKind::Selected(selected) = event.kind {
                assert_eq!(
                    trees.get(event.entity).unwrap().state().selected(),
                    selected
                );
            }
        },
    );
    assert!(WidgetryTreeModel::select(app.world_mut(), source, Some(c)).unwrap());
    assert_eq!(
        app.world()
            .get::<WidgetryTreeModel>(source)
            .unwrap()
            .visible_index(c),
        None
    );
    assert!(!WidgetryTreeModel::select(app.world_mut(), source, Some(c)).unwrap());
    assert!(WidgetryTreeModel::select(app.world_mut(), source, None).unwrap());
    assert!(!WidgetryTreeModel::select(app.world_mut(), source, None).unwrap());
    assert_eq!(
        app.world().resource::<Events>().0,
        vec![
            WidgetryTreeEventKind::Selected(Some(c)),
            WidgetryTreeEventKind::Selected(None)
        ]
    );
    assert!(
        !app.world()
            .get::<WidgetryTreeModel>(source)
            .unwrap()
            .state()
            .is_expanded(a)
    );
}

#[test]
fn invalid_requests_log_errors_without_state_changes() {
    let (mut app, source, a, b, _) = fixture();
    WidgetryTreeModel::select(app.world_mut(), source, Some(b)).unwrap();
    app.world_mut().resource_mut::<Events>().0.clear();
    let logs = bevy_widgetry_test_utils::LogCapture::default();
    logs.run(|| {
        for target in [source, Entity::PLACEHOLDER] {
            for error in [
                WidgetryTreeModel::select(app.world_mut(), target, Some(Entity::PLACEHOLDER))
                    .unwrap_err(),
                WidgetryTreeModel::expand(app.world_mut(), target, Entity::PLACEHOLDER)
                    .unwrap_err(),
                WidgetryTreeModel::collapse(app.world_mut(), target, Entity::PLACEHOLDER)
                    .unwrap_err(),
                WidgetryTreeModel::toggle_expand(app.world_mut(), target, Entity::PLACEHOLDER)
                    .unwrap_err(),
            ] {
                assert_eq!(error.severity(), bevy::ecs::error::Severity::Error);
                assert!(error.to_string().contains("Tree"));
            }
        }
    });
    assert_eq!(
        app.world()
            .get::<WidgetryTreeModel>(source)
            .unwrap()
            .state()
            .selected(),
        Some(b)
    );
    assert!(
        !app.world()
            .get::<WidgetryTreeModel>(source)
            .unwrap()
            .state()
            .is_expanded(a)
    );
    assert!(app.world().resource::<Events>().0.is_empty());
    assert_eq!(
        logs.records()
            .iter()
            .filter(|record| record.level == bevy::log::Level::ERROR)
            .count(),
        8
    );
}

#[test]
fn lazy_notifications_and_loader_api_preserve_committed_state() {
    let (mut app, source, _, node, _) = fixture();
    app.world_mut()
        .entity_mut(node)
        .insert(WidgetryTreeChildrenState::Unknown);
    app.add_observer(
        |event: On<WidgetryTreeEvent>,
         trees: Query<&WidgetryTreeModel>,
         lazy: Query<&WidgetryTreeChildrenState>| {
            if let WidgetryTreeEventKind::Expanded(node)
            | WidgetryTreeEventKind::ChildrenRequested(node) = event.kind
            {
                assert!(trees.get(event.entity).unwrap().state().is_expanded(node));
                assert_eq!(lazy.get(node).unwrap(), &WidgetryTreeChildrenState::Loading);
            }
        },
    );
    let logs = bevy_widgetry_test_utils::LogCapture::default();
    let error = logs
        .run(|| WidgetryTreeChildrenState::set_loaded(app.world_mut(), node))
        .unwrap_err();
    assert_eq!(error.severity(), bevy::ecs::error::Severity::Error);
    assert!(
        logs.records()
            .iter()
            .any(|record| record.level == bevy::log::Level::ERROR)
    );
    assert_eq!(
        app.world().get::<WidgetryTreeChildrenState>(node),
        Some(&WidgetryTreeChildrenState::Unknown)
    );
    assert!(WidgetryTreeModel::expand(app.world_mut(), source, node).unwrap());
    assert!(WidgetryTreeModel::collapse(app.world_mut(), source, node).unwrap());
    let child = app
        .world_mut()
        .spawn((WidgetryTreeNode, ChildOf(node)))
        .id();
    assert!(WidgetryTreeChildrenState::set_loaded(app.world_mut(), node).unwrap());
    assert!(!WidgetryTreeChildrenState::set_loaded(app.world_mut(), node).unwrap());
    app.update();
    assert_eq!(
        app.world()
            .get::<WidgetryTreeModel>(source)
            .unwrap()
            .visible_index(child),
        None
    );
    assert_eq!(
        app.world().resource::<Events>().0,
        vec![
            WidgetryTreeEventKind::Expanded(node),
            WidgetryTreeEventKind::ChildrenRequested(node),
            WidgetryTreeEventKind::Collapsed(node)
        ]
    );
    app.world_mut().despawn(node);
    assert!(WidgetryTreeChildrenState::set_loaded(app.world_mut(), node).is_err());
}

#[test]
fn expand_collapse_preserve_unaffected_entries_and_hidden_selection() {
    let (mut app, source, a, b, c) = fixture();
    let list = app
        .world()
        .get::<WidgetryListModel<WidgetryTreeVisibleItem>>(source)
        .unwrap();
    let b_id = list.id(1).unwrap();
    assert!(WidgetryTreeModel::expand(app.world_mut(), source, a).unwrap());
    assert!(!WidgetryTreeModel::expand(app.world_mut(), source, a).unwrap());
    assert!(!WidgetryTreeModel::expand(app.world_mut(), source, b).unwrap());
    assert!(WidgetryTreeModel::select(app.world_mut(), source, Some(c)).unwrap());
    assert!(!WidgetryTreeModel::select(app.world_mut(), source, Some(c)).unwrap());
    assert!(WidgetryTreeModel::collapse(app.world_mut(), source, a).unwrap());
    assert!(!WidgetryTreeModel::collapse(app.world_mut(), source, a).unwrap());
    let tree = app.world().get::<WidgetryTreeModel>(source).unwrap();
    assert_eq!(tree.state().selected(), Some(c));
    assert_eq!(tree.visible_index(c), None);
    let list = app
        .world()
        .get::<WidgetryListModel<WidgetryTreeVisibleItem>>(source)
        .unwrap();
    assert_eq!(list.len(), 2);
    assert_eq!(list.id(1), Some(b_id));
    assert_eq!(list.revision(1), Some(0));
    assert_eq!(
        app.world().resource::<Events>().0,
        vec![
            WidgetryTreeEventKind::Expanded(a),
            WidgetryTreeEventKind::Selected(Some(c)),
            WidgetryTreeEventKind::Collapsed(a)
        ]
    );
    assert!(WidgetryTreeModel::toggle_expand(app.world_mut(), source, a).unwrap());
    assert_eq!(
        app.world()
            .get::<WidgetryTreeModel>(source)
            .unwrap()
            .visible_index(c),
        Some(1)
    );
}

#[test]
fn lazy_children_request_is_once_and_external_completion_is_projected() {
    let (mut app, source, _, b, _) = fixture();
    app.world_mut()
        .entity_mut(b)
        .insert(WidgetryTreeChildrenState::Unknown);
    app.update();
    assert!(WidgetryTreeModel::expand(app.world_mut(), source, b).unwrap());
    assert_eq!(
        app.world().get::<WidgetryTreeChildrenState>(b),
        Some(&WidgetryTreeChildrenState::Loading)
    );
    assert!(WidgetryTreeModel::collapse(app.world_mut(), source, b).unwrap());
    assert!(WidgetryTreeModel::expand(app.world_mut(), source, b).unwrap());
    assert_eq!(
        app.world()
            .resource::<Events>()
            .0
            .iter()
            .filter(|event| **event == WidgetryTreeEventKind::ChildrenRequested(b))
            .count(),
        1
    );
    let child = app.world_mut().spawn((WidgetryTreeNode, ChildOf(b))).id();
    assert!(WidgetryTreeChildrenState::set_loaded(app.world_mut(), b).unwrap());
    app.update();
    assert_eq!(
        app.world()
            .get::<WidgetryTreeModel>(source)
            .unwrap()
            .visible_index(child),
        Some(2)
    );
}

#[test]
fn failed_lazy_expand_can_be_retried_after_source_repair() {
    let (mut app, source, _, node, _) = fixture();
    app.world_mut()
        .entity_mut(node)
        .insert(WidgetryTreeChildrenState::Unknown);
    app.world_mut()
        .entity_mut(source)
        .remove::<WidgetryListModel<WidgetryTreeVisibleItem>>();
    let error = WidgetryTreeModel::expand(app.world_mut(), source, node).unwrap_err();
    assert_eq!(error.severity(), bevy::ecs::error::Severity::Error);
    assert!(
        !app.world()
            .get::<WidgetryTreeModel>(source)
            .unwrap()
            .state()
            .is_expanded(node)
    );
    assert_eq!(
        app.world().get::<WidgetryTreeChildrenState>(node),
        Some(&WidgetryTreeChildrenState::Unknown)
    );
    assert!(app.world().resource::<Events>().0.is_empty());
    app.world_mut()
        .entity_mut(source)
        .insert(WidgetryListModel::<WidgetryTreeVisibleItem>::default());
    assert!(WidgetryTreeModel::expand(app.world_mut(), source, node).unwrap());
    assert_eq!(
        app.world().resource::<Events>().0,
        vec![
            WidgetryTreeEventKind::Expanded(node),
            WidgetryTreeEventKind::ChildrenRequested(node)
        ]
    );
}

#[test]
fn external_hierarchy_changes_repair_state_and_keep_identity() {
    let (mut app, source, a, b, c) = fixture();
    assert!(WidgetryTreeModel::expand(app.world_mut(), source, a).unwrap());
    assert!(WidgetryTreeModel::select(app.world_mut(), source, Some(c)).unwrap());
    let root = app.world().get::<WidgetryTreeModel>(source).unwrap().root();
    let old_id = app
        .world()
        .get::<WidgetryListModel<WidgetryTreeVisibleItem>>(source)
        .unwrap()
        .id(1);
    app.world_mut().entity_mut(c).insert(ChildOf(root));
    app.update();
    let list = app
        .world()
        .get::<WidgetryListModel<WidgetryTreeVisibleItem>>(source)
        .unwrap();
    assert_eq!(list.id(2), old_id);
    assert_eq!(list.get(2).unwrap().depth, 0);
    assert_eq!(
        app.world()
            .get::<WidgetryTreeModel>(source)
            .unwrap()
            .state()
            .selected(),
        Some(c)
    );
    app.world_mut().entity_mut(c).despawn();
    app.update();
    assert_eq!(
        app.world()
            .get::<WidgetryTreeModel>(source)
            .unwrap()
            .state()
            .selected(),
        None
    );
    assert!(WidgetryTreeModel::select(app.world_mut(), source, Some(c)).is_err());
    assert!(WidgetryTreeModel::expand(app.world_mut(), Entity::PLACEHOLDER, b).is_err());
    assert!(WidgetryTreeModel::select(app.world_mut(), source, Some(b)).unwrap());
    assert!(WidgetryTreeModel::select(app.world_mut(), source, None).unwrap());
}

#[test]
fn invalid_targets_are_rejected_using_live_hierarchy() {
    let (mut app, source, a, b, c) = fixture();
    let root = app.world().get::<WidgetryTreeModel>(source).unwrap().root();
    let other_root = app.world_mut().spawn_empty().id();
    let foreign = app
        .world_mut()
        .spawn((WidgetryTreeNode, ChildOf(other_root)))
        .id();
    app.world_mut().entity_mut(a).remove::<WidgetryTreeNode>();
    assert!(WidgetryTreeModel::select(app.world_mut(), source, Some(b)).unwrap());
    for invalid in [root, a, c, foreign, Entity::PLACEHOLDER] {
        assert!(WidgetryTreeModel::select(app.world_mut(), source, Some(invalid)).is_err());
        assert!(WidgetryTreeModel::expand(app.world_mut(), source, invalid).is_err());
        assert!(WidgetryTreeModel::collapse(app.world_mut(), source, invalid).is_err());
        assert!(WidgetryTreeModel::toggle_expand(app.world_mut(), source, invalid).is_err());
        assert_eq!(
            app.world()
                .get::<WidgetryTreeModel>(source)
                .unwrap()
                .state()
                .selected(),
            Some(b)
        );
    }
    app.world_mut().entity_mut(b).insert(ChildOf(other_root));
    assert!(WidgetryTreeModel::select(app.world_mut(), source, Some(b)).is_err());
    app.update();
    let tree = app.world().get::<WidgetryTreeModel>(source).unwrap();
    assert_eq!(tree.state().selected(), None);
    assert!(tree.visible_items().is_empty());
    assert_eq!(
        app.world().resource::<Events>().0,
        vec![WidgetryTreeEventKind::Selected(Some(b))]
    );
    app.world_mut().despawn(source);
    assert!(WidgetryTreeModel::select(app.world_mut(), source, None).is_err());
    assert!(WidgetryTreeModel::toggle_expand(app.world_mut(), source, foreign).is_err());
}

#[test]
fn hidden_descendant_intent_survives_collapse_and_root_destruction_repairs_state() {
    let (mut app, source, a, _, c) = fixture();
    let d = app.world_mut().spawn((WidgetryTreeNode, ChildOf(c))).id();
    assert!(WidgetryTreeModel::expand(app.world_mut(), source, c).unwrap());
    assert!(WidgetryTreeModel::select(app.world_mut(), source, Some(d)).unwrap());
    assert_eq!(
        app.world()
            .get::<WidgetryTreeModel>(source)
            .unwrap()
            .visible_index(d),
        None
    );
    assert!(WidgetryTreeModel::expand(app.world_mut(), source, a).unwrap());
    assert_eq!(
        app.world()
            .get::<WidgetryTreeModel>(source)
            .unwrap()
            .visible_index(d),
        Some(2)
    );
    assert!(WidgetryTreeModel::collapse(app.world_mut(), source, a).unwrap());
    assert!(
        app.world()
            .get::<WidgetryTreeModel>(source)
            .unwrap()
            .state()
            .is_expanded(c)
    );
    assert!(WidgetryTreeModel::expand(app.world_mut(), source, a).unwrap());
    assert_eq!(
        app.world()
            .get::<WidgetryTreeModel>(source)
            .unwrap()
            .visible_index(d),
        Some(2)
    );
    let root = app.world().get::<WidgetryTreeModel>(source).unwrap().root();
    app.world_mut().resource_mut::<Events>().0.clear();
    app.world_mut().despawn(root);
    app.update();
    let tree = app.world().get::<WidgetryTreeModel>(source).unwrap();
    assert!(tree.visible_items().is_empty());
    assert_eq!(tree.state().selected(), None);
    for node in [a, c, d] {
        assert!(!tree.state().is_expanded(node));
        assert_eq!(tree.visible_index(node), None);
    }
    assert!(
        app.world()
            .get::<WidgetryListModel<WidgetryTreeVisibleItem>>(source)
            .unwrap()
            .is_empty()
    );
    assert!(app.world().resource::<Events>().0.is_empty());
}

#[test]
fn lazy_completion_while_collapsed_and_empty_completion_preserve_request_contract() {
    for empty in [false, true] {
        let (mut app, source, _, b, _) = fixture();
        app.world_mut()
            .entity_mut(b)
            .insert(WidgetryTreeChildrenState::Unknown);
        assert!(WidgetryTreeModel::expand(app.world_mut(), source, b).unwrap());
        assert!(WidgetryTreeModel::collapse(app.world_mut(), source, b).unwrap());
        let child = (!empty).then(|| app.world_mut().spawn((WidgetryTreeNode, ChildOf(b))).id());
        assert!(WidgetryTreeChildrenState::set_loaded(app.world_mut(), b).unwrap());
        app.update();
        let tree = app.world().get::<WidgetryTreeModel>(source).unwrap();
        assert_eq!(tree.visible_items()[1].has_children, !empty);
        assert!(!tree.state().is_expanded(b));
        if let Some(child) = child {
            assert_eq!(tree.visible_index(child), None);
        }
        assert_eq!(
            WidgetryTreeModel::expand(app.world_mut(), source, b).unwrap(),
            !empty
        );
        if let Some(child) = child {
            assert_eq!(
                app.world()
                    .get::<WidgetryTreeModel>(source)
                    .unwrap()
                    .visible_index(child),
                Some(2)
            );
        }
        let mut expected = vec![
            WidgetryTreeEventKind::Expanded(b),
            WidgetryTreeEventKind::ChildrenRequested(b),
            WidgetryTreeEventKind::Collapsed(b),
        ];
        if !empty {
            expected.push(WidgetryTreeEventKind::Expanded(b));
        }
        assert_eq!(app.world().resource::<Events>().0, expected);
    }
}

#[test]
fn sibling_reorder_and_noop_updates_preserve_incremental_projection() {
    let (mut app, source, a, b, _) = fixture();
    let before = app
        .world()
        .get::<WidgetryListModel<WidgetryTreeVisibleItem>>(source)
        .unwrap();
    let a_id = before.id(0).unwrap();
    let b_id = before.id(1).unwrap();
    let tick = app
        .world()
        .entity(source)
        .get_ref::<WidgetryListModel<WidgetryTreeVisibleItem>>()
        .unwrap()
        .last_changed();
    app.update();
    assert_eq!(
        app.world()
            .entity(source)
            .get_ref::<WidgetryListModel<WidgetryTreeVisibleItem>>()
            .unwrap()
            .last_changed(),
        tick
    );
    let root = app.world().get::<WidgetryTreeModel>(source).unwrap().root();
    app.world_mut().entity_mut(root).insert_children(0, &[b]);
    app.update();
    let list = app
        .world()
        .get::<WidgetryListModel<WidgetryTreeVisibleItem>>(source)
        .unwrap();
    assert_eq!(list.id(0), Some(b_id));
    assert_eq!(list.id(1), Some(a_id));
    assert_eq!(list.revision(0), Some(0));
    assert_eq!(list.revision(1), Some(0));
    assert!(WidgetryTreeModel::expand(app.world_mut(), source, a).unwrap());
    let list = app
        .world()
        .get::<WidgetryListModel<WidgetryTreeVisibleItem>>(source)
        .unwrap();
    assert_eq!(list.id(0), Some(b_id));
    assert_eq!(list.id(1), Some(a_id));
    assert_eq!(list.revision(0), Some(0));
    assert_eq!(list.revision(1), Some(1));
    assert!(
        app.world()
            .resource::<Events>()
            .0
            .iter()
            .all(|event| !matches!(event, WidgetryTreeEventKind::Selected(_)))
    );
}
