#![allow(clippy::disallowed_macros, clippy::unwrap_used)]
#[allow(dead_code)]
mod common;
use bevy::{
    ecs::{error::Severity, schedule::SingleThreadedExecutor},
    prelude::*,
};
use bevy_widgetry_diagnostics::bevy::Diagnostic;
use bevy_widgetry_table::*;
use bevy_widgetry_test_utils::{DiagnosticPhase, ErrorCapture, LogCapture};
use std::collections::HashSet;

static TEST_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

fn configure(app: &mut App) {
    app.set_error_handler(ErrorCapture::handler());
    app.edit_schedule(PostUpdate, |schedule| {
        schedule.set_executor(SingleThreadedExecutor::new());
    });
}
fn field(diagnostic: &dyn Diagnostic, key: &str) -> String {
    diagnostic
        .contexts()
        .iter()
        .rev()
        .flat_map(|frame| frame.fields())
        .find(|(name, _)| name == key)
        .unwrap()
        .1
        .clone()
}
fn fault(diagnostic: &dyn Diagnostic) -> Option<TableRuntimeFault> {
    let mut source = diagnostic.source();
    while let Some(error) = source {
        if let Some(TableError::Runtime { fault }) = error.downcast_ref::<TableError>() {
            return Some(*fault);
        }
        source = error.source();
    }
    None
}

#[test]
fn all_cell_and_header_scene_attempts_keep_native_causes_payloads_and_healthy_output() {
    let _guard = TEST_LOCK.lock().unwrap_or_else(|error| error.into_inner());
    let (mut app, source, view, _) = common::fixture(2, 2);
    let other = app
        .world_mut()
        .spawn_scene(
            bsn! { @WidgetryTable::<u32> { @source:source } Node {width:px(286),height:px(144)} },
        )
        .unwrap()
        .id();
    app.world_mut()
        .get_mut::<WidgetryTableLayout>(view)
        .unwrap()
        .frozen_columns = 1;
    app.update();
    app.update();
    configure(&mut app);
    app.register_table_cell_renderer(WidgetryTableCellRenderer::new(|value: &String| {
        let value=value.clone();
        bsn_list! { template(move |_| {
            if value.ends_with("/0") {
                let cause=std::io::Error::other(value.clone());
                Err(if value=="0/0" {BevyError::panic(cause,Box::new(value.clone()))}else{BevyError::error(cause).with_payload(Box::new(value.clone()))})
            } else {Ok(Text(format!("healthy {value}")))}
        }) }
    })).unwrap();
    app.register_table_header_renderer(WidgetryTableHeaderRenderer::new(|value: &String| {
        let value=value.clone();
        bsn_list! { template(move |_| {
            if value=="C1" {Err(BevyError::error(std::io::Error::other(value.clone())).with_payload(Box::new(value.clone())))} else {Ok(Text(value.clone()))}
        }) }
    })).unwrap();
    let errors = ErrorCapture::default();
    let logs = LogCapture::default();
    let mut ids = HashSet::new();
    for _ in 0..2 {
        logs.run(|| errors.run(|| app.update()));
        let records = errors.take_records();
        assert_eq!(records.len(), 6);
        let mut locations = HashSet::new();
        for mut record in records {
            let payload = *record
                .error
                .take_payload()
                .unwrap()
                .downcast::<String>()
                .unwrap();
            assert_eq!(
                record.error.severity(),
                if payload == "0/0" {
                    Severity::Panic
                } else {
                    Severity::Error
                }
            );
            let diagnostic = record.managed().unwrap().diagnostics().next().unwrap();
            assert!(ids.insert(diagnostic.origin().id().clone()));
            assert_eq!(
                logs.diagnostic_records(diagnostic.origin().id(), DiagnosticPhase::Source)
                    .len(),
                1
            );
            assert!(
                logs.diagnostic_records(diagnostic.origin().id(), DiagnosticPhase::Handled)
                    .is_empty()
            );
            let actual_view = field(diagnostic, "view");
            assert!([view.to_string(), other.to_string()].contains(&actual_view));
            assert!(field(diagnostic, "source").contains(&format!("{source:?}")));
            assert_ne!(field(diagnostic, "column"), "None");
            if payload != "C1" {
                assert_ne!(field(diagnostic, "row"), "None");
            }
            if actual_view == view.to_string() && payload != "C1" {
                assert_eq!(field(diagnostic, "region"), "FrozenCell");
            }
            let mut cause = diagnostic.source();
            let mut found = false;
            while let Some(error) = cause {
                if let Some(scene) = error.downcast_ref::<bevy_widgetry_core::scene::SceneError>() {
                    assert_eq!(
                        scene
                            .native_template_error()
                            .unwrap()
                            .downcast_ref::<std::io::Error>()
                            .unwrap()
                            .to_string(),
                        payload
                    );
                    found = true;
                    break;
                }
                cause = error.source();
            }
            assert!(found);
            locations.insert((actual_view, payload));
        }
        assert_eq!(locations.len(), 6);
        let healthy = app
            .world_mut()
            .query::<&Text>()
            .iter(app.world())
            .filter(|text| text.0.starts_with("healthy "))
            .count();
        assert_eq!(healthy, 4);
    }
    assert_eq!(ids.len(), 12);
}

#[test]
fn missing_cell_registry_is_persistent_per_cell_and_recovers_only_when_validated() {
    let _guard = TEST_LOCK.lock().unwrap_or_else(|error| error.into_inner());
    let (mut app, _, view, _) = common::fixture(2, 2);
    configure(&mut app);
    let registry = app
        .world_mut()
        .remove_resource::<WidgetryTableCellRendererRegistry>()
        .unwrap();
    let logs = LogCapture::default();
    let errors = ErrorCapture::default();
    let mut origins = std::collections::HashMap::new();
    for _ in 0..3 {
        logs.run(|| errors.run(|| app.update()));
        let records = errors.take_records();
        assert_eq!(records.len(), 4);
        for record in records {
            assert_eq!(record.error.severity(), Severity::Error);
            let diagnostic = record.managed().unwrap().diagnostics().next().unwrap();
            assert!(matches!(
                fault(diagnostic),
                Some(TableRuntimeFault::Missing {
                    component: "WidgetryTableCellRendererRegistry",
                    ..
                })
            ));
            let key = (field(diagnostic, "row"), field(diagnostic, "column"));
            assert_eq!(field(diagnostic, "view"), view.to_string());
            if let Some(old) = origins.insert(key, diagnostic.origin().clone()) {
                assert!(std::sync::Arc::ptr_eq(&old, diagnostic.origin()));
            }
            assert_eq!(
                logs.diagnostic_records(diagnostic.origin().id(), DiagnosticPhase::Source)
                    .len(),
                1
            );
        }
    }
    app.world_mut().insert_resource(registry);
    logs.run(|| errors.run(|| app.update()));
    assert!(errors.take().is_empty());
    for origin in origins.values() {
        assert_eq!(
            logs.records()
                .iter()
                .filter(
                    |record| record.matches_field("error_id", &origin.id().to_string())
                        && record.matches_field("phase", "recovered")
                )
                .count(),
            1
        );
    }
}

#[test]
fn owned_viewports_and_canvases_report_actual_slot_until_repaired() {
    let _guard = TEST_LOCK.lock().unwrap_or_else(|error| error.into_inner());
    for mode in [
        "body_marker",
        "body_node",
        "body_despawn",
        "canvas_node",
        "canvas_despawn",
        "frozen_marker",
        "header_marker",
    ] {
        let (mut app, _, view, body) = common::fixture(2, 2);
        configure(&mut app);
        let canvas = app.world().get::<Children>(body).unwrap()[0];
        let frozen = app
            .world()
            .get::<Children>(view)
            .unwrap()
            .iter()
            .find(|&entity| app.world().get::<WidgetryTableFrozenBody>(entity).is_some())
            .unwrap();
        let header = app
            .world()
            .get::<Children>(view)
            .unwrap()
            .iter()
            .find(|&entity| {
                app.world()
                    .get::<WidgetryTableColumnHeaders>(entity)
                    .is_some()
            })
            .unwrap();
        match mode {
            "body_marker" => {
                app.world_mut()
                    .entity_mut(body)
                    .remove::<WidgetryTableBody>();
            }
            "body_node" => {
                app.world_mut().entity_mut(body).remove::<Node>();
            }
            "body_despawn" => {
                app.world_mut().despawn(body);
            }
            "canvas_node" => {
                app.world_mut().entity_mut(canvas).remove::<Node>();
            }
            "canvas_despawn" => {
                app.world_mut().despawn(canvas);
            }
            "frozen_marker" => {
                app.world_mut()
                    .entity_mut(frozen)
                    .remove::<WidgetryTableFrozenBody>();
            }
            _ => {
                app.world_mut()
                    .entity_mut(header)
                    .remove::<WidgetryTableColumnHeaders>();
            }
        }
        let logs = LogCapture::default();
        let errors = ErrorCapture::default();
        let mut origin = None;
        for _ in 0..3 {
            logs.run(|| errors.run(|| app.update()));
            let records = errors.take_records();
            assert_eq!(records.len(), 1, "{mode}");
            let diagnostic = records[0].managed().unwrap().diagnostics().next().unwrap();
            assert_eq!(
                field(diagnostic, "region"),
                match mode {
                    "frozen_marker" => "FrozenBody",
                    "header_marker" => "ColumnHeaders",
                    _ => "Body",
                }
            );
            if let Some(old) = &origin {
                assert!(std::sync::Arc::ptr_eq(old, diagnostic.origin()), "{mode}");
            } else {
                origin = Some(diagnostic.origin().clone());
            }
            assert_eq!(
                logs.diagnostic_records(diagnostic.origin().id(), DiagnosticPhase::Source)
                    .len(),
                1
            );
        }
        if mode == "body_marker" {
            app.world_mut().entity_mut(body).insert(WidgetryTableBody);
            logs.run(|| errors.run(|| app.update()));
            assert!(errors.take().is_empty());
            assert_eq!(
                logs.records()
                    .iter()
                    .filter(|record| record.matches_field("phase", "recovered"))
                    .count(),
                1
            );
            app.world_mut()
                .entity_mut(body)
                .remove::<WidgetryTableBody>();
            logs.run(|| errors.run(|| app.update()));
            let records = errors.take_records();
            assert_eq!(records.len(), 1);
            assert_ne!(
                records[0]
                    .managed()
                    .unwrap()
                    .diagnostics()
                    .next()
                    .unwrap()
                    .origin()
                    .id(),
                origin.as_ref().unwrap().id()
            );
        }
        let recovered = logs
            .records()
            .iter()
            .filter(|record| record.matches_field("phase", "recovered"))
            .count();
        app.world_mut().despawn(view);
        logs.run(|| errors.run(|| app.update()));
        assert_eq!(
            logs.records()
                .iter()
                .filter(|record| record.matches_field("phase", "recovered"))
                .count(),
            recovered
        );
    }
}

#[test]
fn renderer_cleanup_detaches_existing_business_subtrees_and_preserves_committed_state() {
    let _guard = TEST_LOCK.lock().unwrap_or_else(|error| error.into_inner());
    let (mut app, _, _, _) = common::fixture(1, 1);
    configure(&mut app);
    let business = app.world_mut().spawn(Name::new("business")).id();
    app.register_table_cell_renderer(WidgetryTableCellRenderer::new(move |_: &String| bsn_list! {
        template(move |ctx| {
            let container=ctx.entity.id();
            ctx.entity.world_scope(|world|{world.entity_mut(business).insert(Name::new("committed")).insert(ChildOf(container));});
            Ok(Name::new("attempt"))
        })
        Children [template(|_| -> Result<Name,BevyError> {Err(BevyError::error("failed sibling"))})]
    })).unwrap();
    let errors = ErrorCapture::default();
    for _ in 0..2 {
        errors.run(|| app.update());
        assert_eq!(errors.take().len(), 1);
        assert_eq!(
            app.world().get::<Name>(business).unwrap().as_str(),
            "committed"
        );
        assert!(app.world().get::<ChildOf>(business).is_none());
        assert!(
            !app.world_mut()
                .query::<&Name>()
                .iter(app.world())
                .any(|name| name.as_str() == "attempt")
        );
    }
}

#[test]
fn multiple_style_outputs_keep_all_origins_without_recovering_when_shells_leave_view() {
    let _guard = TEST_LOCK.lock().unwrap_or_else(|error| error.into_inner());
    let (mut app, _, view, _) = common::fixture(2, 2);
    configure(&mut app);
    let cells = app
        .world_mut()
        .query_filtered::<Entity, With<WidgetryTableCell>>()
        .iter(app.world())
        .collect::<Vec<_>>();
    for &cell in &cells {
        app.world_mut().entity_mut(cell).remove::<BorderColor>();
    }
    let logs = LogCapture::default();
    let errors = ErrorCapture::default();
    let mut origins = HashSet::new();
    for _ in 0..2 {
        logs.run(|| errors.run(|| app.update()));
        let records = errors.take_records();
        assert_eq!(records.len(), 4);
        for record in records {
            let diagnostic = record.managed().unwrap().diagnostics().next().unwrap();
            origins.insert(diagnostic.origin().id().clone());
            assert_eq!(
                logs.diagnostic_records(diagnostic.origin().id(), DiagnosticPhase::Source)
                    .len(),
                1
            );
        }
    }
    assert_eq!(origins.len(), 4);
    app.world_mut().get_mut::<Node>(view).unwrap().height = px(0);
    logs.run(|| {
        errors.run(|| {
            app.update();
            app.update();
        })
    });
    errors.take();
    assert!(
        !logs
            .records()
            .iter()
            .any(|record| record.matches_field("phase", "recovered"))
    );
    app.world_mut().get_mut::<Node>(view).unwrap().height = px(144);
    logs.run(|| {
        errors.run(|| {
            app.update();
            app.update();
        })
    });
    assert!(errors.take().is_empty());
}

#[test]
fn repeated_header_intents_are_independent_and_keep_the_actual_target_and_column() {
    let _guard = TEST_LOCK.lock().unwrap_or_else(|error| error.into_inner());
    let (mut app, source, view, _) = common::fixture(1, 1);
    configure(&mut app);
    let column = app
        .world()
        .get::<WidgetryTableModel<u32>>(source)
        .unwrap()
        .column_id(0)
        .unwrap();
    app.world_mut()
        .entity_mut(view)
        .insert(WidgetryTableHeaderActions {
            sortable: [column].into(),
            ..default()
        });
    app.update();
    let button = app
        .world_mut()
        .query_filtered::<Entity, With<WidgetryTableHeaderAction>>()
        .single(app.world())
        .unwrap();
    app.world_mut()
        .entity_mut(view)
        .remove::<WidgetryTableViewState>();
    let errors = ErrorCapture::default();
    let logs = LogCapture::default();
    let mut ids = HashSet::new();
    for _ in 0..2 {
        logs.run(|| {
            errors.run(|| {
                app.world_mut()
                    .trigger(bevy::ui_widgets::Activate { entity: button });
                app.world_mut().flush();
            })
        });
        let records = errors.take_records();
        assert_eq!(records.len(), 1);
        let diagnostic = records[0].managed().unwrap().diagnostics().next().unwrap();
        assert_eq!(records[0].error.severity(), Severity::Error);
        assert_eq!(field(diagnostic, "view"), view.to_string());
        assert_eq!(field(diagnostic, "target"), button.to_string());
        assert!(field(diagnostic, "column").contains(&format!("{column:?}")));
        assert!(ids.insert(diagnostic.origin().id().clone()));
        assert_eq!(
            logs.diagnostic_records(diagnostic.origin().id(), DiagnosticPhase::Source)
                .len(),
            1
        );
    }
}

#[test]
fn invalid_colors_preserve_core_origin_and_do_not_commit() {
    let _guard = TEST_LOCK.lock().unwrap_or_else(|error| error.into_inner());
    let (mut app, _, view, _) = common::fixture(1, 1);
    let before = WidgetryTableColorOverrides::get(app.world(), view)
        .unwrap()
        .clone();
    let mut invalid = before.clone();
    invalid.cell.hovered.foreground = Some(Color::linear_rgba(f32::NAN, 0., 0., 1.));
    let logs = LogCapture::default();
    let failure = logs
        .run(|| WidgetryTableColorOverrides::set_in_world(app.world_mut(), view, invalid))
        .unwrap_err();
    assert!(matches!(failure.kind(), TableError::Core { .. }));
    assert!(
        failure
            .contexts()
            .iter()
            .flat_map(|frame| frame.fields())
            .any(|(key, value)| key == "region" && value == "cell")
    );
    assert_eq!(
        logs.diagnostic_records(failure.origin().id(), DiagnosticPhase::Source)
            .len(),
        1
    );
    assert_eq!(
        WidgetryTableColorOverrides::get(app.world(), view).unwrap(),
        &before
    );
    let target = app.world_mut().spawn_empty().id();
    assert!(
        matches!(WidgetryTableColorOverrides::get(app.world(),target).unwrap_err().kind(),TableError::InvalidColors {view:actual} if *actual==target)
    );
}

#[test]
fn replacing_destroyed_body_or_canvas_rediscovers_structure_and_recovers_the_original_fault() {
    let _guard = TEST_LOCK.lock().unwrap_or_else(|error| error.into_inner());
    for destroy_mode in ["canvas", "body", "all_parts"] {
        let (mut app, source, view, body) = common::fixture(1, 1);
        configure(&mut app);
        let destroyed = if destroy_mode != "canvas" {
            body
        } else {
            app.world().get::<Children>(body).unwrap()[0]
        };
        if destroy_mode == "all_parts" {
            let children = app.world().get::<Children>(view).unwrap().to_vec();
            for child in children {
                app.world_mut().despawn(child);
            }
        } else {
            app.world_mut().despawn(destroyed);
        }
        let errors = ErrorCapture::default();
        let logs = LogCapture::default();
        logs.run(|| {
            errors.run(|| {
                app.update();
                app.update();
            })
        });
        let records = errors.take_records();
        assert_eq!(records.len(), 2);
        let origin = records[0]
            .managed()
            .unwrap()
            .diagnostics()
            .next()
            .unwrap()
            .origin()
            .clone();
        assert!(std::sync::Arc::ptr_eq(
            &origin,
            records[1]
                .managed()
                .unwrap()
                .diagnostics()
                .next()
                .unwrap()
                .origin()
        ));
        app.world_mut().entity_mut(view).apply_scene(bsn! { @WidgetryTable::<u32> { @source:source } Node {width:px(286),height:px(144)} }).unwrap();
        logs.run(|| {
            errors.run(|| {
                app.update();
                app.update();
            })
        });
        assert!(errors.take().is_empty());
        assert_eq!(common::projection(&mut app, view).len(), 1);
        assert_eq!(
            logs.records()
                .iter()
                .filter(|record| record.matches_field("phase", "recovered")
                    && record.matches_field("error_id", &origin.id().to_string()))
                .count(),
            1
        );
    }
}

#[test]
fn resize_header_precheck_shares_the_column_failure_with_later_projection() {
    let _guard = TEST_LOCK.lock().unwrap_or_else(|error| error.into_inner());
    for frozen in [false, true] {
        let (mut app, source, view, _) = common::fixture(1, 1);
        configure(&mut app);
        app.world_mut()
            .get_mut::<WidgetryTableLayout>(view)
            .unwrap()
            .frozen_columns = usize::from(frozen);
        app.update();
        let (header, column) = app
            .world_mut()
            .query::<(Entity, &WidgetryTableColumnHeader)>()
            .iter(app.world())
            .map(|(entity, header)| (entity, header.column))
            .next()
            .unwrap();
        let target = app
            .world()
            .get::<Children>(header)
            .unwrap()
            .iter()
            .find(|&entity| {
                app.world()
                    .get::<Node>(entity)
                    .is_some_and(|node| node.width == px(6))
            })
            .unwrap();
        let pointer = bevy::picking::events::Pointer::new(
            bevy::picking::pointer::PointerId::Mouse,
            bevy::picking::pointer::Location {
                target: bevy::camera::NormalizedRenderTarget::None {
                    width: 600,
                    height: 400,
                },
                position: Vec2::ZERO,
            },
        );
        app.world_mut()
            .trigger(bevy::picking::events::PointerDragStart {
                entity: target,
                pointer,
                button: bevy::picking::pointer::PointerButton::Primary,
                hit: bevy::picking::backend::HitData::new(Entity::PLACEHOLDER, 0., None, None),
            });
        app.world_mut().flush();
        app.world_mut()
            .get_mut::<WidgetryTableModel<u32>>(source)
            .unwrap()
            .set_header(0, WidgetryTableHeaderValue::new(true))
            .unwrap();
        let errors = ErrorCapture::default();
        let logs = LogCapture::default();
        let mut origin = None;
        for _ in 0..3 {
            logs.run(|| errors.run(|| app.update()));
            let records = errors.take_records();
            assert_eq!(records.len(), 1);
            let diagnostic = records[0].managed().unwrap().diagnostics().next().unwrap();
            assert_eq!(field(diagnostic, "column"), format!("{:?}", Some(column)));
            assert_eq!(
                field(diagnostic, "region"),
                if frozen {
                    "FrozenColumnHeader"
                } else {
                    "ColumnHeader"
                }
            );
            if let Some(old) = &origin {
                assert!(std::sync::Arc::ptr_eq(old, diagnostic.origin()));
            } else {
                origin = Some(diagnostic.origin().clone());
            }
            assert_eq!(
                logs.diagnostic_records(diagnostic.origin().id(), DiagnosticPhase::Source)
                    .len(),
                1
            );
        }
    }
}

#[test]
fn detached_structural_slots_keep_ownership_origin_and_allow_reattachment_without_duplicate_shells()
{
    let _guard = TEST_LOCK.lock().unwrap_or_else(|error| error.into_inner());
    for detach_body in [false, true] {
        let (mut app, _, view, body) = common::fixture(1, 1);
        configure(&mut app);
        let entity = if detach_body {
            body
        } else {
            app.world().get::<Children>(body).unwrap()[0]
        };
        let parent = app.world().get::<ChildOf>(entity).unwrap().parent();
        let other = app.world_mut().spawn_empty().id();
        app.world_mut().entity_mut(entity).insert(ChildOf(other));
        let errors = ErrorCapture::default();
        let logs = LogCapture::default();
        let mut origin = None;
        for _ in 0..3 {
            logs.run(|| errors.run(|| app.update()));
            let records = errors.take_records();
            assert_eq!(records.len(), 1);
            let diagnostic = records[0].managed().unwrap().diagnostics().next().unwrap();
            assert!(
                matches!(fault(diagnostic),Some(TableRuntimeFault::Ownership {entity:actual,..}) if actual==entity)
            );
            if let Some(old) = &origin {
                assert!(std::sync::Arc::ptr_eq(old, diagnostic.origin()));
            } else {
                origin = Some(diagnostic.origin().clone());
            }
        }
        app.world_mut().entity_mut(entity).insert(ChildOf(parent));
        logs.run(|| errors.run(|| app.update()));
        assert!(errors.take().is_empty());
        assert_eq!(common::projection(&mut app, view).len(), 1);
        assert_eq!(
            app.world_mut()
                .query::<&WidgetryTableCell>()
                .iter(app.world())
                .count(),
            1
        );
        assert_eq!(
            logs.diagnostic_records(origin.as_ref().unwrap().id(), DiagnosticPhase::Source)
                .len(),
            1
        );
    }
}

#[test]
fn resize_shared_structure_checks_keep_the_same_root_slot_after_gesture_cancellation() {
    let _guard = TEST_LOCK.lock().unwrap_or_else(|error| error.into_inner());
    let (mut app, _, view, _) = common::fixture(1, 1);
    configure(&mut app);
    let header = app
        .world_mut()
        .query_filtered::<Entity, With<WidgetryTableColumnHeader>>()
        .single(app.world())
        .unwrap();
    let target = app
        .world()
        .get::<Children>(header)
        .unwrap()
        .iter()
        .find(|&entity| {
            app.world()
                .get::<Node>(entity)
                .is_some_and(|node| node.width == px(6))
        })
        .unwrap();
    let pointer = bevy::picking::events::Pointer::new(
        bevy::picking::pointer::PointerId::Mouse,
        bevy::picking::pointer::Location {
            target: bevy::camera::NormalizedRenderTarget::None {
                width: 600,
                height: 400,
            },
            position: Vec2::ZERO,
        },
    );
    app.world_mut()
        .trigger(bevy::picking::events::PointerDragStart {
            entity: target,
            pointer,
            button: bevy::picking::pointer::PointerButton::Primary,
            hit: bevy::picking::backend::HitData::new(Entity::PLACEHOLDER, 0., None, None),
        });
    app.world_mut().flush();
    app.world_mut()
        .entity_mut(view)
        .remove::<WidgetryTableLayout>();
    let errors = ErrorCapture::default();
    let logs = LogCapture::default();
    let mut origin = None;
    for _ in 0..3 {
        logs.run(|| errors.run(|| app.update()));
        let records = errors.take_records();
        assert_eq!(records.len(), 1);
        let diagnostic = records[0].managed().unwrap().diagnostics().next().unwrap();
        assert_eq!(field(diagnostic, "column"), "None");
        if let Some(old) = &origin {
            assert!(std::sync::Arc::ptr_eq(old, diagnostic.origin()));
        } else {
            origin = Some(diagnostic.origin().clone());
        }
    }
}

#[test]
fn successive_scroll_output_failures_keep_distinct_regions_entities_and_origins() {
    let _guard = TEST_LOCK.lock().unwrap_or_else(|error| error.into_inner());
    let (mut app, _, view, body) = common::fixture(1, 1);
    configure(&mut app);
    let columns = app
        .world()
        .get::<Children>(view)
        .unwrap()
        .iter()
        .find(|&entity| {
            app.world()
                .get::<WidgetryTableColumnHeaders>(entity)
                .is_some()
        })
        .unwrap();
    let rows = app
        .world()
        .get::<Children>(view)
        .unwrap()
        .iter()
        .find(|&entity| app.world().get::<WidgetryTableRowHeaders>(entity).is_some())
        .unwrap();
    let frozen = app
        .world()
        .get::<Children>(view)
        .unwrap()
        .iter()
        .find(|&entity| app.world().get::<WidgetryTableFrozenBody>(entity).is_some())
        .unwrap();
    let logs = LogCapture::default();
    let errors = ErrorCapture::default();
    let mut ids = HashSet::new();
    let mut regions = Vec::new();
    for (entity, region) in [
        (body, "Body"),
        (columns, "ColumnHeaders"),
        (rows, "RowHeaders"),
        (frozen, "FrozenBody"),
    ] {
        let value = app
            .world_mut()
            .entity_mut(entity)
            .take::<bevy::ui::ScrollPosition>()
            .unwrap();
        logs.run(|| errors.run(|| app.update()));
        let records = errors.take_records();
        assert_eq!(records.len(), 1);
        let diagnostic = records[0].managed().unwrap().diagnostics().next().unwrap();
        assert!(
            matches!(fault(diagnostic),Some(TableRuntimeFault::Missing {entity:actual,component:"ScrollPosition"}) if actual==entity)
        );
        assert!(ids.insert(diagnostic.origin().id().clone()));
        regions.push((field(diagnostic, "region"), region));
        assert_eq!(
            logs.diagnostic_records(diagnostic.origin().id(), DiagnosticPhase::Source)
                .len(),
            1
        );
        app.world_mut().entity_mut(entity).insert(value);
    }
    assert_eq!(ids.len(), 4);
    assert!(regions.iter().all(|(actual, expected)| actual == expected));
    logs.run(|| errors.run(|| app.update()));
    assert!(errors.take().is_empty());
}

#[test]
fn keyboard_reveal_and_plain_projection_share_the_body_scroll_failure() {
    let _guard = TEST_LOCK.lock().unwrap_or_else(|error| error.into_inner());
    let (mut app, _, view, body) = common::fixture(1, 1);
    configure(&mut app);
    app.world_mut()
        .resource_mut::<bevy::input_focus::InputFocus>()
        .set(view, bevy::input_focus::FocusCause::Navigated);
    app.world_mut().trigger(bevy::input_focus::FocusGained {
        entity: view,
        cause: bevy::input_focus::FocusCause::Navigated,
    });
    app.world_mut().flush();
    assert!(
        app.world()
            .get::<WidgetryTableState>(view)
            .unwrap()
            .focused_cell()
            .is_some()
    );
    app.world_mut()
        .entity_mut(body)
        .remove::<bevy::ui::ScrollPosition>();
    let errors = ErrorCapture::default();
    let logs = LogCapture::default();
    let mut origin = None;
    for _ in 0..3 {
        logs.run(|| errors.run(|| app.update()));
        let records = errors.take_records();
        assert_eq!(records.len(), 1);
        let diagnostic = records[0].managed().unwrap().diagnostics().next().unwrap();
        assert_eq!(field(diagnostic, "region"), "Body");
        if let Some(old) = &origin {
            assert!(std::sync::Arc::ptr_eq(old, diagnostic.origin()));
        } else {
            origin = Some(diagnostic.origin().clone());
        }
        assert_eq!(
            logs.diagnostic_records(diagnostic.origin().id(), DiagnosticPhase::Source)
                .len(),
            1
        );
    }
}
