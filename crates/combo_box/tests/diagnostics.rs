#![allow(clippy::disallowed_macros, clippy::unwrap_used)]

use bevy::prelude::*;
use bevy_widgetry_combo_box::*;
use bevy_widgetry_diagnostics::bevy::{Diagnostic, WidgetryEcsError};
use bevy_widgetry_list_view::*;
use bevy_widgetry_test_utils::{DiagnosticPhase, ErrorCapture, LogCapture, scene_app};

static TEST_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());
fn kind(diagnostic: &dyn Diagnostic) -> Option<&ComboError> {
    let mut cause = diagnostic.source();
    while let Some(error) = cause {
        if let Some(kind) = error.downcast_ref::<ComboError>() {
            return Some(kind);
        }
        cause = error.source();
    }
    None
}
fn app() -> App {
    let mut app = scene_app();
    app.register_widgetry_combo_box::<u32>().unwrap();
    app.set_error_handler(ErrorCapture::handler());
    app.edit_schedule(PostUpdate, |schedule| {
        schedule.set_executor(bevy::ecs::schedule::SingleThreadedExecutor::new());
    });
    app
}
fn combo(app: &mut App, source: Entity) -> Entity {
    bevy_widgetry_core::scene::spawn_scene(app.world_mut(),bsn!{
        @WidgetryComboBox::<u32> {@source:source,@renderer:{WidgetryListViewRenderer::new(|_:&u32|bsn_list!{Node})}}
    }).unwrap()
}

#[test]
fn independent_invalid_root_requests_keep_domain_targets_and_distinct_origins() {
    let _guard = TEST_LOCK.lock().unwrap_or_else(|error| error.into_inner());
    let mut app = app();
    let root = app.world_mut().spawn_empty().id();
    let errors = ErrorCapture::default();
    let logs = LogCapture::default();
    logs.run(|| {
        errors.run(|| {
            WidgetryComboBox::<u32>::select_index(&mut app.world_mut().commands(), root, 7);
            WidgetryComboBox::<u32>::clear_selection(&mut app.world_mut().commands(), root);
            app.world_mut().flush();
        })
    });
    let failures = errors.take();
    assert_eq!(failures.len(), 2);
    let mut origins = Vec::new();
    for error in &failures {
        assert_eq!(error.severity(), bevy::ecs::error::Severity::Error);
        let diagnostic = error
            .downcast_ref::<WidgetryEcsError>()
            .unwrap()
            .diagnostics()
            .next()
            .unwrap();
        assert!(
            matches!(kind(diagnostic),Some(ComboError::InvalidRoot{root:actual,item_type}) if *actual==root && *item_type==std::any::type_name::<u32>())
        );
        assert_eq!(
            logs.diagnostic_records(diagnostic.origin().id(), DiagnosticPhase::Source)
                .len(),
            1
        );
        assert!(
            logs.diagnostic_records(diagnostic.origin().id(), DiagnosticPhase::Handled)
                .is_empty()
        );
        origins.push(diagnostic.origin().id().clone());
    }
    assert_ne!(origins[0], origins[1]);
}

#[test]
fn all_field_structure_failures_reach_the_host_and_repeated_observations_share_origins() {
    let _guard = TEST_LOCK.lock().unwrap_or_else(|error| error.into_inner());
    let mut app = app();
    let source = app
        .world_mut()
        .spawn(WidgetryListModel::<u32>::default())
        .id();
    let roots = [combo(&mut app, source), combo(&mut app, source)];
    for root in roots {
        let field = app.world().get::<Children>(root).unwrap()[0];
        app.world_mut().despawn(field);
    }
    let errors = ErrorCapture::default();
    let logs = LogCapture::default();
    logs.run(||{
        let mut originals=Vec::new();
        for pass in 0..2 {
            errors.run(||app.update());
            let failures=errors.take();
            let diagnostics=failures.iter().filter_map(|e|e.downcast_ref::<WidgetryEcsError>()).flat_map(|e|e.diagnostics())
                .filter(|d|d.contexts().iter().any(|frame|frame.operation()=="combo_box.field_project")).collect::<Vec<_>>();
            assert_eq!(diagnostics.len(),2);
            for root in roots {
                let diagnostic=diagnostics.iter().find(|d|matches!(kind(**d),Some(ComboError::MissingPart{root:actual,..}) if *actual==root)).unwrap();
                if pass==0 {originals.push(diagnostic.origin().clone());}
                else {assert!(originals.iter().any(|origin|std::sync::Arc::ptr_eq(origin,diagnostic.origin())));}
            }
        }
        for origin in originals {
            assert_eq!(logs.diagnostic_records(origin.id(),DiagnosticPhase::Source).len(),1);
            assert!(logs.diagnostic_records(origin.id(),DiagnosticPhase::Handled).is_empty());
        }
    });
}

fn internal_list(world: &World, root: Entity) -> Entity {
    let popup = world.get::<Children>(root).unwrap()[1];
    world
        .get::<Children>(popup)
        .unwrap()
        .iter()
        .find(|child| world.get::<WidgetryListView<u32>>(*child).is_some())
        .unwrap()
}

#[test]
fn source_mapping_item_and_index_rejections_keep_their_exact_targets() {
    let _guard = TEST_LOCK.lock().unwrap_or_else(|error| error.into_inner());
    let mut app = app();
    let mut model = WidgetryListModel::default();
    let deleted = model.push(7u32).unwrap();
    model.remove(0);
    let source = app.world_mut().spawn(model).id();
    let root = combo(&mut app, source);
    let list = internal_list(app.world(), root);
    let errors = ErrorCapture::default();
    errors.run(|| {
        WidgetryComboBox::<u32>::select_id(&mut app.world_mut().commands(), root, deleted);
        WidgetryComboBox::<u32>::select_index(&mut app.world_mut().commands(), root, 17);
        app.world_mut().flush();
    });
    let failures = errors.take();
    assert_eq!(failures.len(), 2);
    assert!(failures.iter().any(|error|error.downcast_ref::<WidgetryEcsError>().unwrap().diagnostics().any(|d|matches!(kind(d),Some(ComboError::MissingItem{root:actual,source:actual_source,list:actual_list,item_id,..}) if (*actual,*actual_source,*actual_list,*item_id)==(root,source,list,deleted)))));
    let index=failures.iter().flat_map(|error|error.downcast_ref::<WidgetryEcsError>().unwrap().diagnostics()).find(|d|matches!(kind(*d),Some(ComboError::List{source}) if matches!(source.as_ref(),ListError::InvalidIndex{view,index:17,len:0,..} if *view==list))).unwrap();
    assert_eq!(
        failures
            .iter()
            .filter(|error| error.severity() == bevy::ecs::error::Severity::Error)
            .count(),
        2
    );
    assert!(
        index
            .contexts()
            .iter()
            .any(|frame| frame.fields().contains(&("combo".into(), root.to_string())))
    );
    assert_eq!(
        WidgetryComboBox::<u32>::selected_id(app.world(), root).unwrap(),
        None
    );
    let other = app
        .world_mut()
        .spawn(WidgetryListModel::<u32>::default())
        .id();
    WidgetryListView::<u32>::set_source_in_world(app.world_mut(), list, other).unwrap();
    let failure = WidgetryComboBox::<u32>::selected_id(app.world(), root).unwrap_err();
    assert!(
        matches!(failure.kind(),ComboError::ListMapping{root:actual,source:actual_source,list:actual_list,actual_source:Some(actual_other)} if (*actual,*actual_source,*actual_list,*actual_other)==(root,source,list,other))
    );
    let popup = app.world().get::<Children>(root).unwrap()[1];
    app.world_mut().entity_mut(popup).remove::<Visibility>();
    let failure = WidgetryComboBox::<u32>::selected_id(app.world(), root).unwrap_err();
    assert!(
        matches!(failure.kind(),ComboError::MissingList{root:actual,source:actual_source,popup:Some(actual_popup)} if (*actual,*actual_source,*actual_popup)==(root,source,popup))
    );
    app.world_mut().entity_mut(popup).insert(Visibility::Hidden);
    app.world_mut().despawn(source);
    let failure = WidgetryComboBox::<u32>::selected_id(app.world(), root).unwrap_err();
    assert!(
        matches!(failure.kind(),ComboError::InvalidSource{root:actual,source:actual_source,item_type} if *actual==root && *actual_source==source && *item_type==std::any::type_name::<u32>())
    );
}

#[derive(Component)]
struct BusinessRecord;

#[test]
fn field_scene_failures_keep_all_origins_native_payloads_and_business_ownership() {
    let _guard = TEST_LOCK.lock().unwrap_or_else(|error| error.into_inner());
    let mut app = app();
    let source = app
        .world_mut()
        .spawn(WidgetryListModel::<u32>::default())
        .id();
    let originals = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
    let mut roots = Vec::new();
    for _ in 0..2 {
        let sink = originals.clone();
        let placeholder = WidgetryComboBoxContent::new(move || {
            let sink = sink.clone();
            bsn_list! {Node--template(move|context|{
                context.entity.world_scope(|world|{world.spawn(BusinessRecord);});
                let failure=bevy_widgetry_core::color::validate_color(Color::linear_rgba(f32::NAN,0.,0.,1.)).unwrap_err();
                sink.lock().unwrap().push(failure.origin().clone());
                Err::<Node,_>(failure.into_bevy_error())
            })}
        });
        roots.push(bevy_widgetry_core::scene::spawn_scene(app.world_mut(),bsn!{
            @WidgetryComboBox::<u32> {@source:source,@placeholder:placeholder,@renderer:{WidgetryListViewRenderer::new(|_:&u32|bsn_list!{Node})}}
        }).unwrap());
    }
    let attempts = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
    for panic_policy in [false, true] {
        let attempts = attempts.clone();
        let placeholder = WidgetryComboBoxContent::new(move || {
            let attempts = attempts.clone();
            bsn_list! {template(move|_|{
                if panic_policy {Err::<Node,_>(BevyError::panic(std::io::Error::other("native panic"),Box::new("payload".to_owned())))}
                else {
                    let attempt=attempts.fetch_add(1,std::sync::atomic::Ordering::Relaxed);
                    Err::<Node,_>(BevyError::error(std::io::Error::other(format!("attempt {attempt}"))).with_payload(Box::new(attempt)))
                }
            })}
        });
        roots.push(bevy_widgetry_core::scene::spawn_scene(app.world_mut(),bsn!{
            @WidgetryComboBox::<u32> {@source:source,@placeholder:placeholder,@renderer:{WidgetryListViewRenderer::new(|_:&u32|bsn_list!{Node})}}
        }).unwrap());
    }
    let healthy = combo(&mut app, source);
    let errors = ErrorCapture::default();
    let logs = LogCapture::default();
    let mut previous_native = None;
    logs.run(|| {
        for attempt in 0..2usize {
            errors.run(|| app.update());
            let mut failures = errors.take();
            assert_eq!(failures.len(), 4);
            let (mut managed, mut native, mut panics) = (0, 0, 0);
            for failure in &mut failures {
                if failure.severity() == bevy::ecs::error::Severity::Panic {
                    assert_eq!(
                        *failure
                            .take_payload()
                            .unwrap()
                            .downcast::<String>()
                            .unwrap(),
                        "payload"
                    );
                    panics += 1;
                } else if let Some(payload) = failure.take_payload() {
                    assert_eq!(*payload.downcast::<usize>().unwrap(), attempt);
                    let diagnostic = failure
                        .downcast_ref::<WidgetryEcsError>()
                        .unwrap()
                        .diagnostics()
                        .next()
                        .unwrap();
                    assert_ne!(previous_native.as_ref(), Some(diagnostic.origin().id()));
                    previous_native = Some(diagnostic.origin().id().clone());
                    let mut source = diagnostic.source();
                    let mut found = false;
                    while let Some(cause) = source {
                        if let Some(scene) =
                            cause.downcast_ref::<bevy_widgetry_core::scene::SceneError>()
                        {
                            assert_eq!(
                                scene
                                    .native_template_error()
                                    .unwrap()
                                    .downcast_ref::<std::io::Error>()
                                    .unwrap()
                                    .to_string(),
                                format!("attempt {attempt}")
                            );
                            found = true;
                        }
                        source = cause.source();
                    }
                    assert!(found);
                    native += 1;
                } else {
                    assert_eq!(failure.severity(), bevy::ecs::error::Severity::Error);
                    let diagnostic = failure
                        .downcast_ref::<WidgetryEcsError>()
                        .unwrap()
                        .diagnostics()
                        .next()
                        .unwrap();
                    assert!(
                        originals
                            .lock()
                            .unwrap()
                            .iter()
                            .any(|origin| std::sync::Arc::ptr_eq(origin, diagnostic.origin()))
                    );
                    assert_eq!(
                        logs.diagnostic_records(diagnostic.origin().id(), DiagnosticPhase::Source)
                            .len(),
                        1
                    );
                    assert!(
                        logs.diagnostic_records(diagnostic.origin().id(), DiagnosticPhase::Handled)
                            .is_empty()
                    );
                    let context = diagnostic
                        .contexts()
                        .iter()
                        .find(|frame| {
                            frame.operation() == "combo_box.field_renderer"
                                && frame.fields().iter().any(|(key, _)| key == "combo")
                        })
                        .unwrap();
                    for key in [
                        "combo", "source", "list", "content", "item_id", "index", "revision",
                    ] {
                        assert!(context.fields().iter().any(|(name, _)| name == key));
                    }
                    managed += 1;
                }
            }
            assert_eq!((managed, native, panics), (2, 1, 1));
        }
    });
    assert_eq!(
        app.world_mut()
            .query::<&BusinessRecord>()
            .iter(app.world())
            .count(),
        4
    );
    for root in roots {
        let field = app.world().get::<Children>(root).unwrap()[0];
        let button = app.world().get::<Children>(field).unwrap()[0];
        let content = app.world().get::<Children>(button).unwrap()[0];
        assert!(
            app.world()
                .get::<Children>(content)
                .is_none_or(|children| children.is_empty())
        );
    }
    assert!(
        WidgetryComboBox::<u32>::selected_id(app.world(), healthy)
            .unwrap()
            .is_none()
    );
}

#[test]
fn color_rejections_preserve_original_values_and_source_types() {
    let _guard = TEST_LOCK.lock().unwrap_or_else(|error| error.into_inner());
    let mut app = app();
    let source = app
        .world_mut()
        .spawn(WidgetryListModel::<u32>::default())
        .id();
    let root = combo(&mut app, source);
    let original = WidgetryComboBoxColorOverrides::get(app.world(), root)
        .unwrap()
        .clone();
    let mut invalid = original.clone();
    invalid.field.pressed.background = Some(Color::linear_rgba(f32::NAN, 0., 0., 1.));
    let failure =
        WidgetryComboBoxColorOverrides::set_in_world(app.world_mut(), root, invalid).unwrap_err();
    assert!(matches!(failure.kind(), ComboError::Core { .. }));
    assert!(std::error::Error::source(failure.kind()).is_some());
    assert_eq!(
        WidgetryComboBoxColorOverrides::get(app.world(), root).unwrap(),
        &original
    );
    let owner = app.world_mut().spawn_empty().id();
    app.world_mut()
        .entity_mut(root)
        .insert(bevy_widgetry_core::color::WidgetryStyleOwner::<
            WidgetryComboBox<u32>,
        >::new(owner));
    let failure =
        WidgetryComboBoxColorOverrides::clear_in_world(app.world_mut(), root).unwrap_err();
    assert!(
        matches!(failure.kind(),ComboError::ManagedColors{root:actual,owner:actual_owner} if (*actual,*actual_owner)==(root,owner))
    );
    app.world_mut().despawn(root);
    assert!(
        matches!(WidgetryComboBoxColorOverrides::get(app.world(),root).unwrap_err().kind(),ComboError::InvalidColorTarget{root:actual} if *actual==root)
    );
}

struct AlternativeSource;
impl WidgetryListDataSource<u32, WidgetryListItemId> for AlternativeSource {
    fn changed_at(world: &World, source: Entity) -> Option<bevy::ecs::change_detection::Tick> {
        WidgetryListModelSource::<u32>::changed_at(world, source)
    }
    fn ids(world: &World, source: Entity) -> Option<Vec<WidgetryListItemId>> {
        WidgetryListModelSource::<u32>::ids(world, source)
    }
    fn row(
        world: &World,
        source: Entity,
        id: WidgetryListItemId,
    ) -> Option<WidgetryListData<'_, u32>> {
        WidgetryListModelSource::<u32>::row(world, source, id)
    }
}

#[test]
fn registration_conflict_inherits_list_source_and_logs_only_that_origin() {
    let _guard = TEST_LOCK.lock().unwrap_or_else(|error| error.into_inner());
    let mut app = scene_app();
    app.add_plugins(WidgetryListViewPlugin)
        .register_widgetry_list_source::<u32, WidgetryListItemId, AlternativeSource>()
        .unwrap();
    let logs = LogCapture::default();
    let failure = logs.run(|| app.register_widgetry_combo_box::<u32>().err().unwrap());
    assert!(
        matches!(failure.kind(),ComboError::List{source} if matches!(source.as_ref(),ListError::AdapterConflict{requested,existing,..} if *requested==std::any::TypeId::of::<WidgetryListModelSource<u32>>() && *existing==std::any::TypeId::of::<AlternativeSource>()))
    );
    assert!(
        std::error::Error::source(failure.kind())
            .unwrap()
            .is::<ListError>()
    );
    assert_eq!(
        logs.diagnostic_records(failure.origin().id(), DiagnosticPhase::Source)
            .len(),
        1
    );
    assert_eq!(
        logs.records()
            .iter()
            .filter(|record| record.level == bevy::log::Level::ERROR)
            .count(),
        1
    );
}
