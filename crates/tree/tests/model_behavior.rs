//! Coverage Map：本文件负责 headless state、增量 ListModel 与 lazy/external hierarchy mutation。
//! State：expanded/selected 与 Unknown/Loading/Loaded；stimuli：公开 API、外部 hierarchy mutation。
//! Guard：失效 node、普通 leaf、重复操作；invariant：Entity selection、未受影响 entry id/revision 保持。
//! Coupling：collapse 隐藏 selection，删除 node 清除 selection，lazy 请求不因 re-expand 重复。

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
    assert!(WidgetryTreeModel::expand(app.world_mut(), source, a));
    assert!(!WidgetryTreeModel::expand(app.world_mut(), source, a));
    assert!(!WidgetryTreeModel::expand(app.world_mut(), source, b));
    assert!(WidgetryTreeModel::select(app.world_mut(), source, Some(c)));
    assert!(!WidgetryTreeModel::select(app.world_mut(), source, Some(c)));
    assert!(WidgetryTreeModel::collapse(app.world_mut(), source, a));
    assert!(!WidgetryTreeModel::collapse(app.world_mut(), source, a));
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
    assert!(WidgetryTreeModel::toggle_expand(app.world_mut(), source, a));
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
    assert!(WidgetryTreeModel::expand(app.world_mut(), source, b));
    assert_eq!(
        app.world().get::<WidgetryTreeChildrenState>(b),
        Some(&WidgetryTreeChildrenState::Loading)
    );
    assert!(WidgetryTreeModel::collapse(app.world_mut(), source, b));
    assert!(WidgetryTreeModel::expand(app.world_mut(), source, b));
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

/// selection 在 reparent 后仍指向同一 Entity，删除后清空；invalid source/node 无副作用。
#[test]
fn external_hierarchy_changes_repair_state_and_keep_identity() {
    let (mut app, source, a, b, c) = fixture();
    assert!(WidgetryTreeModel::expand(app.world_mut(), source, a));
    assert!(WidgetryTreeModel::select(app.world_mut(), source, Some(c)));
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
    assert!(!WidgetryTreeModel::select(app.world_mut(), source, Some(c)));
    assert!(!WidgetryTreeModel::expand(
        app.world_mut(),
        Entity::PLACEHOLDER,
        b
    ));
    assert!(WidgetryTreeModel::select(app.world_mut(), source, Some(b)));
    assert!(WidgetryTreeModel::select(app.world_mut(), source, None));
}
