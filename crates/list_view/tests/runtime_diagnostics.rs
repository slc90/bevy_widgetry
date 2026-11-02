#![allow(clippy::disallowed_macros, clippy::unwrap_used)]

use bevy::prelude::*;
use bevy_widgetry_diagnostics::bevy::{Diagnostic, WidgetryEcsError};
use bevy_widgetry_list_view::*;
use bevy_widgetry_scroll_area::WidgetryScrollAreaViewport;
use bevy_widgetry_test_utils::{DiagnosticPhase, ErrorCapture, LogCapture, scene_app};

static TEST_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());
fn list_kind(diagnostic: &dyn Diagnostic) -> Option<&ListError> {
    let mut source = diagnostic.source();
    while let Some(error) = source {
        if let Some(kind) = error.downcast_ref::<ListError>() {
            return Some(kind);
        }
        source = error.source();
    }
    None
}
fn is_list(error: &BevyError) -> bool {
    error
        .downcast_ref::<WidgetryEcsError>()
        .is_some_and(|error| {
            error
                .diagnostics()
                .any(|diagnostic| list_kind(diagnostic).is_some())
        })
}
fn viewport(app: &App, root: Entity) -> Entity {
    app.world()
        .get::<Children>(root)
        .unwrap()
        .iter()
        .find(|child| {
            app.world()
                .get::<WidgetryScrollAreaViewport>(*child)
                .is_some()
        })
        .unwrap()
}
fn size(app: &mut App, viewport: Entity, height: f32) {
    app.world_mut().entity_mut(viewport).insert(ComputedNode {
        size: Vec2::new(100., height),
        inverse_scale_factor: 1.,
        ..default()
    });
}
fn app() -> App {
    let mut app = scene_app();
    app.add_plugins(WidgetryListViewPlugin)
        .set_error_handler(ErrorCapture::handler());
    app.register_widgetry_list_view::<u32>().unwrap();
    app
}

fn view(app: &mut App, source: Entity) -> Entity {
    bevy_widgetry_core::scene::spawn_scene(app.world_mut(), bsn! {
        @WidgetryListView::<u32> {@source:source,@item_height:20.,@renderer:{WidgetryListViewRenderer::new(|_:&u32| bsn_list!{Node})}}
    }).unwrap()
}

fn model(app: &mut App) -> Entity {
    let mut model = WidgetryListModel::default();
    model.push(7u32).unwrap();
    app.world_mut().spawn(model).id()
}

#[test]
fn all_source_failures_reach_the_host_with_stable_origins_and_no_false_handling() {
    let _guard = TEST_LOCK.lock().unwrap();
    let mut app = app();
    let invalid = app.world_mut().spawn_empty().id();
    let a = view(&mut app, invalid);
    let b = view(&mut app, invalid);
    let source = model(&mut app);
    let _healthy = view(&mut app, source);
    let errors = ErrorCapture::default();
    let logs = LogCapture::default();
    logs.run(|| {
        errors.run(|| {
            app.world_mut().run_schedule(PreUpdate);
            app.world_mut().flush();
        });
        let failures = errors
            .take()
            .into_iter()
            .filter(is_list)
            .collect::<Vec<_>>();
        assert_eq!(failures.len(), 2);
        let diagnostics = failures
            .iter()
            .flat_map(|error| {
                error
                    .downcast_ref::<WidgetryEcsError>()
                    .unwrap()
                    .diagnostics()
            })
            .collect::<Vec<_>>();
        assert_eq!(diagnostics.len(), 2);
        let origins = diagnostics
            .iter()
            .map(|d| d.origin().clone())
            .collect::<Vec<_>>();
        assert_ne!(origins[0].id(), origins[1].id());
        for root in [a, b] {
            assert!(diagnostics.iter().any(|d| {
                d.contexts()
                    .iter()
                    .any(|f| f.fields().contains(&("view".into(), root.to_string())))
            }));
        }
        errors.run(|| {
            app.world_mut().run_schedule(PreUpdate);
            app.world_mut().flush();
        });
        for error in errors.take() {
            assert_eq!(error.severity(), bevy::ecs::error::Severity::Error);
            for diagnostic in error
                .downcast_ref::<WidgetryEcsError>()
                .unwrap()
                .diagnostics()
            {
                assert!(
                    origins
                        .iter()
                        .any(|origin| std::sync::Arc::ptr_eq(origin, diagnostic.origin()))
                );
            }
        }
        for origin in origins {
            assert_eq!(
                logs.diagnostic_records(origin.id(), DiagnosticPhase::Source)
                    .len(),
                1
            );
            assert!(
                logs.diagnostic_records(origin.id(), DiagnosticPhase::Handled)
                    .is_empty()
            );
        }
    });
}

#[test]
fn missing_viewports_on_two_roots_are_both_reported_and_the_healthy_root_still_renders() {
    let _guard = TEST_LOCK.lock().unwrap();
    let mut app = app();
    let source = model(&mut app);
    let roots = [
        view(&mut app, source),
        view(&mut app, source),
        view(&mut app, source),
    ];
    for root in &roots[..2] {
        let viewport = app
            .world()
            .get::<Children>(*root)
            .unwrap()
            .iter()
            .find(|child| {
                app.world()
                    .get::<WidgetryScrollAreaViewport>(*child)
                    .is_some()
            })
            .unwrap();
        app.world_mut().despawn(viewport);
    }
    let viewport = app
        .world()
        .get::<Children>(roots[2])
        .unwrap()
        .iter()
        .find(|child| {
            app.world()
                .get::<WidgetryScrollAreaViewport>(*child)
                .is_some()
        })
        .unwrap();
    app.world_mut().entity_mut(viewport).insert(ComputedNode {
        size: Vec2::new(100., 40.),
        inverse_scale_factor: 1.,
        ..default()
    });
    let errors = ErrorCapture::default();
    errors.run(|| app.update());
    let failures = errors
        .take()
        .into_iter()
        .filter(is_list)
        .collect::<Vec<_>>();
    assert_eq!(failures.len(), 2);
    let diagnostics = failures
        .iter()
        .flat_map(|error| {
            error
                .downcast_ref::<WidgetryEcsError>()
                .unwrap()
                .diagnostics()
        })
        .collect::<Vec<_>>();
    assert_eq!(diagnostics.len(), 2);
    for root in &roots[..2] {
        assert!(
            diagnostics
                .iter()
                .any(|d| matches!(list_kind(*d),Some(ListError::MissingRuntimePart {view,..}) if view == root))
        );
    }
    assert_eq!(
        app.world_mut()
            .query::<&WidgetryListViewItem>()
            .iter(app.world())
            .count(),
        1
    );
}

#[test]
fn a_repaired_spacer_waits_for_real_layout_before_reporting_recovery() {
    let _guard = TEST_LOCK.lock().unwrap();
    let mut app = app();
    let source = model(&mut app);
    let root = view(&mut app, source);
    let viewport = viewport(&app, root);
    size(&mut app, viewport, 40.);
    app.update();
    let content = app.world().get::<Children>(viewport).unwrap()[0];
    let top = app.world().get::<Children>(content).unwrap()[0];
    app.world_mut().entity_mut(top).remove::<Node>();
    let errors = ErrorCapture::default();
    let logs = LogCapture::default();
    logs.run(|| {
        errors.run(|| app.update());
        let failures = errors.take().into_iter().filter(is_list).collect::<Vec<_>>();
        assert_eq!(failures.len(),1);
        let diagnostic = failures[0].downcast_ref::<WidgetryEcsError>().unwrap().diagnostics().next().unwrap();
        assert!(matches!(list_kind(diagnostic),Some(ListError::MissingRuntimePart {view,entity,component}) if *view == root && *entity == top && *component == "TopSpacer Node"));
        let origin = diagnostic.origin().clone();
        errors.run(|| app.update());
        let repeated = errors.take().into_iter().filter(is_list).collect::<Vec<_>>();
        let diagnostic = repeated[0].downcast_ref::<WidgetryEcsError>().unwrap().diagnostics().next().unwrap();
        assert!(std::sync::Arc::ptr_eq(&origin,diagnostic.origin()));
        assert_eq!(logs.diagnostic_records(origin.id(),DiagnosticPhase::Source).len(),1);
        assert!(logs.diagnostic_records(origin.id(),DiagnosticPhase::Handled).is_empty());
        app.world_mut().entity_mut(top).insert(Node::default());
        size(&mut app,viewport,0.);
        errors.run(|| app.update());
        assert!(errors.take().into_iter().all(|error| !is_list(&error)));
        assert_eq!(logs.records_for_error(origin.id()).iter().filter(|r| r.level == bevy::log::Level::INFO).count(),0);
        size(&mut app,viewport,40.);
        errors.run(|| app.update());
        assert!(errors.take().is_empty());
        assert_eq!(logs.records_for_error(origin.id()).iter().filter(|r| r.level == bevy::log::Level::INFO).count(),1);
        assert_eq!(app.world_mut().query::<&WidgetryListViewItem>().iter(app.world()).count(),1);
        app.world_mut().entity_mut(top).remove::<Node>();
        errors.run(|| app.update());
        let next = errors.take().into_iter().filter(is_list).collect::<Vec<_>>();
        let diagnostic = next[0].downcast_ref::<WidgetryEcsError>().unwrap().diagnostics().next().unwrap();
        assert_ne!(origin.id(),diagnostic.origin().id());
    });
}

#[test]
fn cached_locator_failure_keeps_its_origin_after_runtime_cleanup() {
    let _guard = TEST_LOCK.lock().unwrap();
    for missing_part in 0..7 {
        let mut app = app();
        let source = model(&mut app);
        let root = view(&mut app, source);
        let viewport = viewport(&app, root);
        size(&mut app, viewport, 40.);
        app.update();
        let content = app.world().get::<Children>(viewport).unwrap()[0];
        let missing = if missing_part == 1 {
            app.world_mut()
                .entity_mut(content)
                .remove::<bevy_widgetry_scroll_area::WidgetryScrollAreaContent>();
            content
        } else if missing_part == 0 {
            app.world_mut()
                .entity_mut(viewport)
                .remove::<WidgetryScrollAreaViewport>();
            viewport
        } else {
            let missing = match missing_part {
                2 => app.world().get::<Children>(content).unwrap()[0],
                3 => *app
                    .world()
                    .get::<Children>(content)
                    .unwrap()
                    .last()
                    .unwrap(),
                4 => viewport,
                5 => content,
                _ => {
                    let spacers = app.world().get::<Children>(content).unwrap();
                    let top = spacers[0];
                    let bottom = *spacers.last().unwrap();
                    app.world_mut().despawn(bottom);
                    top
                }
            };
            app.world_mut().despawn(missing);
            missing
        };
        let errors = ErrorCapture::default();
        let logs = LogCapture::default();
        logs.run(|| {
            let mut origin = None;
            for _ in 0..3 {
                errors.run(||app.update());
                let failures = errors.take().into_iter().filter(is_list).collect::<Vec<_>>();
                assert_eq!(failures.len(),1);
                let diagnostic = failures[0].downcast_ref::<WidgetryEcsError>().unwrap().diagnostics().next().unwrap();
                assert!(matches!(list_kind(diagnostic),Some(ListError::MissingRuntimePart {view,entity,..}) if *view == root && *entity == missing));
                if let Some(origin) = &origin {assert!(std::sync::Arc::ptr_eq(origin,diagnostic.origin()));}
                else {origin=Some(diagnostic.origin().clone());}
            }
            let origin = origin.unwrap();
            assert_eq!(logs.diagnostic_records(origin.id(),DiagnosticPhase::Source).len(),1);
        });
    }
}

#[test]
fn row_style_failure_does_not_recover_when_zero_layout_destroys_the_row() {
    let _guard = TEST_LOCK.lock().unwrap();
    let mut app = app();
    let source = model(&mut app);
    let root = view(&mut app, source);
    let viewport = viewport(&app, root);
    size(&mut app, viewport, 40.);
    app.update();
    let row = app
        .world_mut()
        .query_filtered::<Entity, With<WidgetryListViewItem>>()
        .single(app.world())
        .unwrap();
    app.world_mut().entity_mut(row).remove::<BackgroundColor>();
    let errors = ErrorCapture::default();
    let logs = LogCapture::default();
    logs.run(|| {
        errors.run(|| app.update());
        let failures = errors.take();
        assert_eq!(failures.len(), 1);
        let origin = failures[0]
            .downcast_ref::<WidgetryEcsError>()
            .unwrap()
            .diagnostics()
            .next()
            .unwrap()
            .origin()
            .clone();
        size(&mut app, viewport, 0.);
        errors.run(|| app.update());
        assert!(errors.take().is_empty());
        assert!(app.world().get_entity(row).is_err());
        let recovered = || {
            logs.records_for_error(origin.id())
                .iter()
                .filter(|record| record.level == bevy::log::Level::INFO)
                .count()
        };
        assert_eq!(recovered(), 0);
        size(&mut app, viewport, 40.);
        errors.run(|| app.update());
        assert!(errors.take().is_empty());
        assert_eq!(recovered(), 1);
        let new_row = app
            .world_mut()
            .query_filtered::<Entity, With<WidgetryListViewItem>>()
            .single(app.world())
            .unwrap();
        assert_ne!(new_row, row);
        assert!(app.world().get::<BackgroundColor>(new_row).is_some());
    });
}

#[derive(Component)]
struct BusinessRecord;

#[test]
fn independent_native_renderer_attempts_keep_their_actual_cause_and_payload() {
    let _guard = TEST_LOCK.lock().unwrap();
    let mut app = app();
    let source = model(&mut app);
    let attempts = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let renderer = WidgetryListViewRenderer::new(move |_: &u32| {
        let attempts = attempts.clone();
        bsn_list! {template(move |_| {
            let attempt = attempts.fetch_add(1,std::sync::atomic::Ordering::Relaxed);
            Err::<Node,_>(BevyError::error(std::io::Error::other(format!("attempt {attempt}"))).with_payload(Box::new(attempt)))
        })}
    });
    let root = bevy_widgetry_core::scene::spawn_scene(
        app.world_mut(),
        bsn! {
            @WidgetryListView::<u32> {@source:source,@renderer:renderer}
        },
    )
    .unwrap();
    let viewport = viewport(&app, root);
    size(&mut app, viewport, 40.);
    let errors = ErrorCapture::default();
    let mut previous_origin = None;
    for attempt in 0..2usize {
        errors.run(|| app.update());
        let mut failures = errors.take();
        assert_eq!(failures.len(), 1);
        let error = &mut failures[0];
        assert_eq!(
            *error.take_payload().unwrap().downcast::<usize>().unwrap(),
            attempt
        );
        let diagnostic = error
            .downcast_ref::<WidgetryEcsError>()
            .unwrap()
            .diagnostics()
            .next()
            .unwrap();
        assert_ne!(previous_origin.as_ref(), Some(diagnostic.origin().id()));
        previous_origin = Some(diagnostic.origin().id().clone());
        let mut cause = diagnostic.source();
        let mut found = false;
        while let Some(source) = cause {
            if let Some(scene) = source.downcast_ref::<bevy_widgetry_core::scene::SceneError>() {
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
            cause = source.source();
        }
        assert!(found);
    }
}

#[test]
fn two_renderer_origins_and_a_native_panic_policy_survive_the_same_update() {
    let _guard = TEST_LOCK.lock().unwrap();
    let mut app = app();
    let source = model(&mut app);
    let originals = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
    let mut roots = Vec::new();
    for _ in 0..2 {
        let sink = originals.clone();
        let renderer = WidgetryListViewRenderer::new(move |_: &u32| {
            let sink = sink.clone();
            bsn_list! {template(move |context| {
                context.entity.world_scope(|world| {world.spawn(BusinessRecord);});
                let error = bevy_widgetry_core::color::validate_color(Color::linear_rgba(f32::NAN,0.,0.,1.)).unwrap_err();
                sink.lock().unwrap().push(error.origin().clone());
                Err::<Node,_>(error.into_bevy_error())
            })}
        });
        roots.push(
            bevy_widgetry_core::scene::spawn_scene(
                app.world_mut(),
                bsn! {
                    @WidgetryListView::<u32> {@source:source,@renderer:renderer}
                },
            )
            .unwrap(),
        );
    }
    roots.push(bevy_widgetry_core::scene::spawn_scene(app.world_mut(), bsn! {
        @WidgetryListView::<u32> {@source:source,@renderer:{WidgetryListViewRenderer::new(|_:&u32| bsn_list! {
            template(|_| Err::<Node,_>(BevyError::panic(std::io::Error::other("native renderer"),Box::new("panic payload".to_owned()))))
        })}}
    }).unwrap());
    for root in &roots {
        let viewport = viewport(&app, *root);
        size(&mut app, viewport, 40.);
    }
    let errors = ErrorCapture::default();
    let logs = LogCapture::default();
    logs.run(|| errors.run(|| app.update()));
    let mut failures = errors.take();
    assert_eq!(failures.len(), 3);
    let mut managed_count = 0;
    let mut panic_count = 0;
    for error in &mut failures {
        if error.severity() == bevy::ecs::error::Severity::Panic {
            panic_count += 1;
            assert_eq!(
                *error.take_payload().unwrap().downcast::<String>().unwrap(),
                "panic payload"
            );
        } else {
            assert_eq!(error.severity(), bevy::ecs::error::Severity::Error);
            let diagnostic = error
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
            let frame = diagnostic
                .contexts()
                .iter()
                .find(|f| f.operation() == "list_view.row_spawn")
                .unwrap();
            for field in [
                "view", "source", "key", "key_type", "index", "row", "range", "revision",
            ] {
                assert!(frame.fields().iter().any(|(name, _)| name == field));
            }
            assert!(
                frame
                    .fields()
                    .iter()
                    .any(|(name, value)| name == "row" && value.starts_with("Some("))
            );
            managed_count += 1;
        }
    }
    assert_eq!((managed_count, panic_count), (2, 1));
    assert_eq!(
        app.world_mut()
            .query::<&WidgetryListViewItem>()
            .iter(app.world())
            .count(),
        0
    );
    assert_eq!(
        app.world_mut()
            .query::<&BusinessRecord>()
            .iter(app.world())
            .count(),
        2
    );
}

#[derive(Component)]
struct ExternalRows {
    ids: Vec<u128>,
    unavailable: bool,
    unenumerable: bool,
}
struct ExternalSource;
impl WidgetryListDataSource<u32, u128> for ExternalSource {
    fn changed_at(world: &World, source: Entity) -> Option<bevy::ecs::change_detection::Tick> {
        Some(
            world
                .get_entity(source)
                .ok()?
                .get_ref::<ExternalRows>()?
                .last_changed(),
        )
    }
    fn ids(world: &World, source: Entity) -> Option<Vec<u128>> {
        let rows = world.get::<ExternalRows>(source)?;
        (!rows.unenumerable).then(|| rows.ids.clone())
    }
    fn row(world: &World, source: Entity, id: u128) -> Option<WidgetryListData<'_, u32>> {
        let rows = world.get::<ExternalRows>(source)?;
        (!rows.unavailable && rows.ids.contains(&id)).then_some(WidgetryListData {
            value: WidgetryListValue::Owned(7),
            disabled: false,
            revision: Some(1),
        })
    }
}

#[test]
fn duplicate_identity_unavailable_row_and_total_height_keep_exact_source_data() {
    let _guard = TEST_LOCK.lock().unwrap();
    let key = u128::MAX - 3;
    for scenario in 0..4 {
        let mut app = scene_app();
        app.add_plugins(WidgetryListViewPlugin)
            .set_error_handler(ErrorCapture::handler());
        app.register_widgetry_list_source::<u32, u128, ExternalSource>()
            .unwrap();
        let ids = if scenario == 0 {
            vec![key, key]
        } else {
            vec![key, key - 1]
        };
        let source = app
            .world_mut()
            .spawn(ExternalRows {
                ids,
                unavailable: scenario == 1,
                unenumerable: scenario == 3,
            })
            .id();
        let height = if scenario == 2 { f32::MAX } else { 20. };
        let root = bevy_widgetry_core::scene::spawn_scene(app.world_mut(),bsn!{
            @WidgetryListView::<u32,u128> {@source:source,@item_height:height,@renderer:{WidgetryListViewRenderer::new(|_:&u32| bsn_list!{Node})}}
        }).unwrap();
        let viewport = viewport(&app, root);
        size(&mut app, viewport, 40.);
        let errors = ErrorCapture::default();
        errors.run(|| app.update());
        let failures = errors
            .take()
            .into_iter()
            .filter(is_list)
            .collect::<Vec<_>>();
        assert_eq!(failures.len(), 1);
        assert_eq!(failures[0].severity(), bevy::ecs::error::Severity::Error);
        let diagnostic = failures[0]
            .downcast_ref::<WidgetryEcsError>()
            .unwrap()
            .diagnostics()
            .next()
            .unwrap();
        match list_kind(diagnostic).unwrap() {
            ListError::InvalidSource {
                view,
                source: actual,
                reason: ListSourceRejection::CannotEnumerate,
                item_type,
                key_type,
            } => {
                assert_eq!(scenario, 3);
                assert_eq!((*view, *actual), (root, source));
                assert_eq!(*item_type, std::any::type_name::<u32>());
                assert_eq!(*key_type, std::any::type_name::<u128>());
            }
            ListError::DuplicateKey {
                view,
                source: actual,
                key: actual_key,
                index,
                previous_index,
            } => {
                assert_eq!(scenario, 0);
                assert_eq!(
                    (*view, *actual, *index, *previous_index),
                    (root, source, 1, 0)
                );
                assert_eq!(actual_key.downcast_ref::<u128>(), Some(&key));
            }
            ListError::UnavailableRow {
                view,
                source: actual,
                key: actual_key,
                index,
            } => {
                assert_eq!(scenario, 1);
                assert_eq!((*view, *actual, *index), (root, source, 0));
                assert_eq!(actual_key.downcast_ref::<u128>(), Some(&key));
            }
            ListError::TotalHeight {
                view,
                source: actual,
                len,
                item_height,
            } => {
                assert_eq!(scenario, 2);
                assert_eq!((*view, *actual, *len), (root, source, 2));
                assert_eq!(item_height.to_bits(), height.to_bits());
            }
            kind => panic!("unexpected diagnostic: {kind:?}"),
        }
        assert_eq!(
            app.world_mut()
                .query::<&WidgetryListViewItem<u128>>()
                .iter(app.world())
                .count(),
            0
        );
    }
}

#[test]
fn enumeration_failure_clears_stale_rows_and_repair_rebuilds_them() {
    let _guard = TEST_LOCK.lock().unwrap();
    let mut app = scene_app();
    app.add_plugins(WidgetryListViewPlugin)
        .set_error_handler(ErrorCapture::handler());
    app.register_widgetry_list_source::<u32, u128, ExternalSource>()
        .unwrap();
    let source = app
        .world_mut()
        .spawn(ExternalRows {
            ids: vec![17],
            unavailable: false,
            unenumerable: false,
        })
        .id();
    let root = bevy_widgetry_core::scene::spawn_scene(app.world_mut(),bsn!{
        @WidgetryListView::<u32,u128> {@source:source,@renderer:{WidgetryListViewRenderer::new(|_:&u32| bsn_list!{Node})}}
    }).unwrap();
    let viewport = viewport(&app, root);
    size(&mut app, viewport, 40.);
    app.update();
    let old = app
        .world_mut()
        .query_filtered::<Entity, With<WidgetryListViewItem<u128>>>()
        .single(app.world())
        .unwrap();
    app.world_mut()
        .get_mut::<ExternalRows>(source)
        .unwrap()
        .unenumerable = true;
    let errors = ErrorCapture::default();
    errors.run(|| app.update());
    let failures = errors.take();
    assert_eq!(failures.len(), 1);
    let diagnostic = failures[0]
        .downcast_ref::<WidgetryEcsError>()
        .unwrap()
        .diagnostics()
        .next()
        .unwrap();
    assert!(
        matches!(list_kind(diagnostic),Some(ListError::InvalidSource {view,source:actual,reason:ListSourceRejection::CannotEnumerate,..}) if *view == root && *actual == source)
    );
    assert!(
        app.world()
            .get::<WidgetryListProjection<u128>>(root)
            .is_none()
    );
    assert!(app.world().get_entity(old).is_err());
    app.world_mut()
        .get_mut::<ExternalRows>(source)
        .unwrap()
        .unenumerable = false;
    errors.run(|| app.update());
    assert!(errors.take().is_empty());
    let item = app
        .world_mut()
        .query::<&WidgetryListViewItem<u128>>()
        .single(app.world())
        .unwrap();
    assert_eq!((item.id, item.index), (17, 0));
}

#[test]
fn every_invalid_style_root_reaches_the_host_and_recovers_with_the_same_origin() {
    let _guard = TEST_LOCK.lock().unwrap();
    let mut app = app();
    let source = model(&mut app);
    let roots = [view(&mut app, source), view(&mut app, source)];
    app.update();
    for root in roots {
        app.world_mut().entity_mut(root).remove::<BackgroundColor>();
    }
    let errors = ErrorCapture::default();
    let logs = LogCapture::default();
    logs.run(|| {
        errors.run(||app.update());
        let failures = errors.take();
        assert_eq!(failures.len(),1);
        let diagnostics = failures[0].downcast_ref::<WidgetryEcsError>().unwrap().diagnostics().collect::<Vec<_>>();
        assert_eq!(diagnostics.len(),2);
        let origins = diagnostics.iter().map(|d|d.origin().clone()).collect::<Vec<_>>();
        for root in roots {
            assert!(diagnostics.iter().any(|d|matches!(list_kind(*d),Some(ListError::MissingRuntimePart {view,entity,component}) if *view == root && *entity == root && *component == std::any::type_name::<BackgroundColor>())));
        }
        errors.run(||app.update());
        for error in errors.take() {
            assert_eq!(error.severity(),bevy::ecs::error::Severity::Error);
            for diagnostic in error.downcast_ref::<WidgetryEcsError>().unwrap().diagnostics() {
                assert!(origins.iter().any(|origin|std::sync::Arc::ptr_eq(origin,diagnostic.origin())));
            }
        }
        for root in roots {app.world_mut().entity_mut(root).insert(BackgroundColor::default());}
        errors.run(||app.update());
        assert!(errors.take().is_empty());
        for origin in origins {
            assert_eq!(logs.diagnostic_records(origin.id(),DiagnosticPhase::Source).len(),1);
            assert!(logs.diagnostic_records(origin.id(),DiagnosticPhase::Handled).is_empty());
            assert_eq!(logs.records_for_error(origin.id()).iter().filter(|r|r.level == bevy::log::Level::INFO).count(),1);
        }
    });
}

#[test]
fn color_requests_retain_core_sources_and_reject_managed_targets_without_mutation() {
    let _guard = TEST_LOCK.lock().unwrap();
    let mut app = app();
    let source = model(&mut app);
    let root = view(&mut app, source);
    let original = WidgetryListViewColorOverrides::get(app.world(), root)
        .unwrap()
        .clone();
    let mut invalid = original.clone();
    invalid.item.selected.background = Some(Color::linear_rgba(f32::NAN, 0., 0., 1.));
    let error =
        WidgetryListViewColorOverrides::set_in_world(app.world_mut(), root, invalid).unwrap_err();
    assert!(matches!(error.kind(), ListError::Core { .. }));
    assert!(std::error::Error::source(error.kind()).is_some());
    assert_eq!(
        WidgetryListViewColorOverrides::get(app.world(), root).unwrap(),
        &original
    );
    let owner = app.world_mut().spawn_empty().id();
    app.world_mut()
        .entity_mut(root)
        .insert(bevy_widgetry_core::color::WidgetryStyleOwner::<
            WidgetryListView<u32>,
        >::new(owner));
    let error = WidgetryListViewColorOverrides::clear_in_world(app.world_mut(), root).unwrap_err();
    assert!(
        matches!(error.kind(),ListError::ManagedColor {entity,owner:actual} if *entity == root && *actual == owner)
    );
    assert_eq!(
        WidgetryListViewColorOverrides::get(app.world(), root).unwrap(),
        &original
    );
    app.world_mut().despawn(root);
    let error = WidgetryListViewColorOverrides::get(app.world(), root).unwrap_err();
    assert!(matches!(error.kind(),ListError::InvalidColorTarget {entity} if *entity == root));
}
