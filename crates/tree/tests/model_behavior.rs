// 测试及其 helper 使用断言和 expect 验证 contract；生产代码仍禁止主动 panic。
#![allow(clippy::disallowed_macros, clippy::expect_used, clippy::unwrap_used)]

//! Coverage Map：本文件负责 headless state、增量 ListModel 与 lazy/external hierarchy mutation。
//! view.rs 负责真实 BSN 输入、focus、共享 source、virtualization 与 renderer/theme；contract.rs 负责配置错误。
//! rendering.rs 负责真实 UI pipeline 的生成当帧消费；facade 的 public_api.rs 负责消费者入口。
//! State：expanded/selected 与 Unknown/Loading/Loaded；stimuli：公开 API、外部 hierarchy mutation。
//! Guard：失效 node、普通 leaf、重复操作；invariant：Entity selection、未受影响 entry id/revision 保持。
//! Coupling：collapse 隐藏 selection，删除 node 清除 selection，lazy 请求不因 re-expand 重复。
//! 跨域 invariant：Entity 为唯一 UI authority，physical row 销毁不删除业务 node，程序选择/修复不发 Selected。

use bevy::prelude::*;
use bevy_widgetry_list_view::WidgetryListModel;
use bevy_widgetry_test_utils::scene_app;
use bevy_widgetry_tree::{
    WidgetryTreeChildrenState, WidgetryTreeEvent, WidgetryTreeEventKind, WidgetryTreeModel,
    WidgetryTreeNode, WidgetryTreePlugin, WidgetryTreeVisibleItem,
};

/// 捕获 Tree 语义；programmatic selection 不追加用户通知。
#[derive(Resource, Default)]
struct Events(Vec<WidgetryTreeEventKind>);

/// 两个顶层 node 与一个 descendant，独立 model source。
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

/// 展开与收起仅变更 descendant 区间，剩余 entries 的 stable id 和 revision 保持。
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

/// Unknown 只请求一次；collapse/loading/re-expand 保持 request，Loaded children 自动参与 projection。
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
    app.world_mut()
        .entity_mut(b)
        .insert(WidgetryTreeChildrenState::Loaded);
    app.update();
    assert_eq!(
        app.world()
            .get::<WidgetryTreeModel>(source)
            .unwrap()
            .visible_index(child),
        Some(2)
    );
}

/// 展开失败不能留下未发出请求的 Loading；恢复依赖后能通过同一公开入口重试。
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

/// selection 在 reparent 后仍指向同一 Entity，删除后清空；invalid source/node 无副作用。
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
    assert!(!WidgetryTreeModel::select(app.world_mut(), source, Some(c)).unwrap());
    assert!(!WidgetryTreeModel::expand(app.world_mut(), Entity::PLACEHOLDER, b).unwrap());
    assert!(WidgetryTreeModel::select(app.world_mut(), source, Some(b)).unwrap());
    assert!(WidgetryTreeModel::select(app.world_mut(), source, None).unwrap());
}

/// API 必须按当前 hierarchy 拒绝 root、跨 source、unmarked ancestor 与 stale identity，拒绝不改变 state/event。
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
        assert!(!WidgetryTreeModel::select(app.world_mut(), source, Some(invalid)).unwrap());
        assert!(!WidgetryTreeModel::expand(app.world_mut(), source, invalid).unwrap());
        assert!(!WidgetryTreeModel::collapse(app.world_mut(), source, invalid).unwrap());
        assert!(!WidgetryTreeModel::toggle_expand(app.world_mut(), source, invalid).unwrap());
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
    assert!(!WidgetryTreeModel::select(app.world_mut(), source, Some(b)).unwrap());
    app.update();
    let tree = app.world().get::<WidgetryTreeModel>(source).unwrap();
    assert_eq!(tree.state().selected(), None);
    assert!(tree.visible_items().is_empty());
    assert!(app.world().resource::<Events>().0.is_empty());
    app.world_mut().despawn(source);
    assert!(!WidgetryTreeModel::select(app.world_mut(), source, None).unwrap());
    assert!(!WidgetryTreeModel::toggle_expand(app.world_mut(), source, foreign).unwrap());
}

/// 隐藏 descendant 可程序展开/选择；祖先 re-expand 恢复展开意图，随后 root 删除静默清空全部 projection。
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

/// lazy 完成时已 collapse：新 child 保持隐藏，下一次 expand 不再请求；空 Loaded node 移除 expander 且 expand 为 no-op。
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
        app.world_mut()
            .entity_mut(b)
            .insert(WidgetryTreeChildrenState::Loaded);
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

/// 外部 reorder/insert/update 保留仍可见 node 的 id；无变更 update 不推进 ListModel change tick 或 entry revision。
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
