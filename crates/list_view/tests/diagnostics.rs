#![allow(clippy::disallowed_macros, clippy::unwrap_used)]

use bevy::prelude::*;
use bevy_widgetry_list_view::*;
use bevy_widgetry_test_utils::{DiagnosticPhase, ErrorCapture, LogCapture, scene_app};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
struct WideKey(u128);
#[derive(Component)]
struct CustomSource(Vec<(WideKey, u32)>);
struct AlternativeSource;
impl WidgetryListDataSource<u32, WideKey> for AlternativeSource {
    fn changed_at(world: &World, source: Entity) -> Option<bevy::ecs::change_detection::Tick> {
        CustomSource::changed_at(world, source)
    }
    fn ids(world: &World, source: Entity) -> Option<Vec<WideKey>> {
        CustomSource::ids(world, source)
    }
    fn row(world: &World, source: Entity, id: WideKey) -> Option<WidgetryListData<'_, u32>> {
        CustomSource::row(world, source, id)
    }
}
impl WidgetryListDataSource<u32, WideKey> for CustomSource {
    fn changed_at(world: &World, source: Entity) -> Option<bevy::ecs::change_detection::Tick> {
        Some(
            world
                .get_entity(source)
                .ok()?
                .get_ref::<Self>()?
                .last_changed(),
        )
    }
    fn ids(world: &World, source: Entity) -> Option<Vec<WideKey>> {
        Some(
            world
                .get::<Self>(source)?
                .0
                .iter()
                .map(|(id, _)| *id)
                .collect(),
        )
    }
    fn row(world: &World, source: Entity, id: WideKey) -> Option<WidgetryListData<'_, u32>> {
        let (_, value) = world
            .get::<Self>(source)?
            .0
            .iter()
            .find(|(key, _)| *key == id)?;
        Some(WidgetryListData {
            value: WidgetryListValue::Borrowed(value),
            disabled: false,
            revision: Some(0),
        })
    }
}

#[test]
fn custom_key_snapshot_retains_the_original_type_and_full_value() {
    let mut app = scene_app();
    app.add_plugins(WidgetryListViewPlugin);
    app.register_widgetry_list_source::<u32, WideKey, CustomSource>()
        .unwrap();
    let valid = WideKey(u128::MAX - 1);
    let absent = WideKey(u128::MAX);
    let source = app.world_mut().spawn(CustomSource(vec![(valid, 7)])).id();
    let root = bevy_widgetry_core::scene::spawn_scene(app.world_mut(), bsn! {
        @WidgetryListView::<u32,WideKey> {@source:source,@renderer:{WidgetryListViewRenderer::new(|_:&u32| bsn_list!{Node})}}
    }).unwrap();
    let logs = LogCapture::default();
    logs.run(|| {
        assert!(CustomSource::row(app.world(), source, absent).is_none());
        assert!(CustomSource::ids(app.world(), Entity::PLACEHOLDER).is_none());
        assert!(logs.records().is_empty());
        let error = WidgetryListView::<u32, WideKey>::replace_selection_in_world(
            app.world_mut(),
            root,
            [valid, absent],
        )
        .unwrap_err();
        let ListError::InvalidKey { key, .. } = error.kind() else {
            panic!("{error}");
        };
        assert_eq!(key.downcast_ref::<WideKey>(), Some(&absent));
        assert!(key.downcast_ref::<u128>().is_none());
        assert_eq!(key.type_name, std::any::type_name::<WideKey>());
        assert!(
            WidgetryListView::<u32, WideKey>::selection(app.world(), root)
                .unwrap()
                .selected_ids()
                .is_empty()
        );
        let diagnostic_count = logs.records().len();
        WidgetryListView::<u32, WideKey>::replace_selection_in_world(
            app.world_mut(),
            root,
            [valid, valid],
        )
        .unwrap();
        assert_eq!(
            WidgetryListView::<u32, WideKey>::selection(app.world(), root)
                .unwrap()
                .selected_id(),
            Some(valid)
        );
        assert!(
            !WidgetryListView::<u32, WideKey>::replace_selection_in_world(
                app.world_mut(),
                root,
                [valid, valid]
            )
            .unwrap()
        );
        assert!(
            WidgetryListView::<u32, WideKey>::clear_selection_in_world(app.world_mut(), root)
                .unwrap()
        );
        assert!(
            !WidgetryListView::<u32, WideKey>::clear_selection_in_world(app.world_mut(), root)
                .unwrap()
        );
        assert_eq!(logs.records().len(), diagnostic_count);
    });
}

#[test]
fn registration_and_source_failures_keep_concrete_types_and_targets() {
    let mut app = scene_app();
    let missing = app
        .register_widgetry_list_source::<u32, WideKey, CustomSource>()
        .err()
        .unwrap();
    assert!(
        matches!(missing.kind(), ListError::MissingPlugin {item_type,key_type,adapter_type} if *item_type == "u32" && *key_type == std::any::type_name::<WideKey>() && *adapter_type == std::any::type_name::<CustomSource>())
    );
    app.add_plugins(WidgetryListViewPlugin);
    app.register_widgetry_list_source::<u32, WideKey, CustomSource>()
        .unwrap();
    let conflict = app
        .register_widgetry_list_source::<u32, WideKey, AlternativeSource>()
        .err()
        .unwrap();
    assert!(
        matches!(conflict.kind(), ListError::AdapterConflict {requested,existing,..} if *requested == std::any::TypeId::of::<AlternativeSource>() && *existing == std::any::TypeId::of::<CustomSource>())
    );
    let source = app.world_mut().spawn(CustomSource(vec![])).id();
    let root = bevy_widgetry_core::scene::spawn_scene(app.world_mut(), bsn! {
        @WidgetryListView::<u32,WideKey> {@source:source,@renderer:{WidgetryListViewRenderer::new(|_:&u32| bsn_list!{Node})}}
    }).unwrap();
    let invalid = WidgetryListView::<u32, WideKey>::set_source_in_world(
        app.world_mut(),
        root,
        Entity::PLACEHOLDER,
    )
    .unwrap_err();
    assert!(
        matches!(invalid.kind(), ListError::InvalidSource {view,source:actual,item_type,key_type,reason:ListSourceRejection::Unavailable} if *view == root && *actual == Entity::PLACEHOLDER && *item_type == "u32" && *key_type == std::any::type_name::<WideKey>())
    );
    assert_eq!(
        app.world()
            .get::<WidgetryListView<u32, WideKey>>(root)
            .unwrap()
            .source(),
        source
    );
}

#[test]
fn insertion_error_retains_index_length_and_identity_counter_without_mutation() {
    let mut model = WidgetryListModel::default();
    let id = model.push(7u32).unwrap();
    let logs = LogCapture::default();
    let error = logs.run(|| model.insert(3, 8)).unwrap_err();
    assert!(matches!(
        error.kind(),
        ListError::InsertionIndex {
            index: 3,
            len: 1,
            next_id: 1
        }
    ));
    assert_eq!(model.get(0), Some(&7));
    assert_eq!(model.id(0), Some(id));
    assert_eq!(
        logs.diagnostic_records(error.origin().id(), DiagnosticPhase::Source)
            .len(),
        1
    );
    assert_eq!(model.get_mut(9).unwrap(), None);
    assert_eq!(model.remove(9), None);
    assert!(!model.move_item(0, 9));
}

#[test]
fn selection_requests_keep_exact_id_and_queued_errors_reach_the_host() {
    let mut app = scene_app();
    app.add_plugins(WidgetryListViewPlugin)
        .set_error_handler(ErrorCapture::handler());
    app.register_widgetry_list_view::<u32>().unwrap();
    let mut model = WidgetryListModel::default();
    let valid = model.push(1u32).unwrap();
    let removed = model.push(2).unwrap();
    model.remove(1);
    let source = app.world_mut().spawn(model).id();
    let root = bevy_widgetry_core::scene::spawn_scene(app.world_mut(), bsn! {
        @WidgetryListView::<u32> { @source:source, @renderer: {WidgetryListViewRenderer::new(|_:&u32| bsn_list!{Node})} }
    }).unwrap();
    let logs = LogCapture::default();
    logs.run(|| {
        let error = WidgetryListView::<u32>::select_id_in_world(app.world_mut(), root, removed)
            .unwrap_err();
        let ListError::InvalidKey {
            view,
            source: actual,
            key,
            ..
        } = error.kind()
        else {
            panic!("{error}");
        };
        assert_eq!((*view, *actual), (root, source));
        assert_eq!(key.downcast_ref::<WidgetryListItemId>(), Some(&removed));
        assert_eq!(key.type_name, std::any::type_name::<WidgetryListItemId>());
        assert!(
            WidgetryListView::<u32>::selection(app.world(), root)
                .unwrap()
                .selected_ids()
                .is_empty()
        );
        WidgetryListView::<u32>::select_id_in_world(app.world_mut(), root, valid).unwrap();
        let errors = ErrorCapture::default();
        WidgetryListView::<u32>::select_index(&mut app.world_mut().commands(), root, 8);
        errors.run(|| app.world_mut().flush());
        let errors = errors.take();
        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0].severity(), bevy::ecs::error::Severity::Error);
        let diagnostic = errors[0]
            .downcast_ref::<bevy_widgetry_diagnostics::bevy::WidgetryEcsError>()
            .unwrap()
            .diagnostics()
            .next()
            .unwrap();
        assert_eq!(
            logs.diagnostic_records(diagnostic.origin().id(), DiagnosticPhase::Source)
                .len(),
            1
        );
        assert!(
            logs.diagnostic_records(diagnostic.origin().id(), DiagnosticPhase::Handled)
                .is_empty()
        );
        assert_eq!(
            WidgetryListView::<u32>::selection(app.world(), root)
                .unwrap()
                .selected_id(),
            Some(valid)
        );
    });
}
