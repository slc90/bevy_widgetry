#![allow(clippy::disallowed_macros, clippy::unwrap_used)]
use bevy::prelude::*;
use bevy_widgetry_diagnostics::bevy::Diagnostic;
use bevy_widgetry_splitter::{
    SplitterError, SplitterFailure, SplitterLayoutError, WidgetrySplitter, WidgetrySplitterPlugin,
};
use bevy_widgetry_test_utils::{
    ErrorCapture, LogCapture, add_ui_plugins, scene_app, spawn_ui_camera,
};
use std::{error::Error, sync::Arc};

#[test]
fn failed_layout_request_reaches_command_host_as_one_original_diagnostic() {
    let mut app = scene_app();
    add_ui_plugins(&mut app);
    spawn_ui_camera(&mut app, UVec2::splat(1200), 1.0);
    app.add_plugins(WidgetrySplitterPlugin);
    app.set_error_handler(ErrorCapture::handler());
    let root = app
        .world_mut()
        .spawn_scene(bsn! { @WidgetrySplitter Node { width: px(1000), height: px(500) } })
        .unwrap()
        .id();
    for _ in 0..12 {
        app.update();
    }
    let handle = app.world().get::<Children>(root).unwrap()[1];
    app.world_mut().despawn(handle);
    WidgetrySplitter::set_split_ratio(&mut app.world_mut().commands(), root, 0.3);
    let errors = ErrorCapture::default();
    let logs = LogCapture::default();
    logs.run(|| errors.run(|| app.world_mut().flush()));
    let records = errors.take_records();
    assert_eq!(records.len(), 1);
    assert_eq!(
        records[0].error.severity(),
        bevy::ecs::error::Severity::Error
    );
    let count = records[0]
        .managed()
        .map(|managed| managed.diagnostics().count())
        .unwrap_or_default();
    assert_eq!(count, 1);
    let diagnostic = records[0].managed().unwrap().diagnostics().next().unwrap();
    assert!(
        matches!(domain(diagnostic), SplitterError::Layout { root: actual, parts: Some(parts), source, .. } if *actual == root && parts[1] == handle && matches!(source.as_ref(), SplitterLayoutError::Query { source: bevy::ecs::query::QueryEntityError::NotSpawned(error), .. } if error.entity() == handle))
    );
    assert!(diagnostic.contexts().iter().any(|frame| {
        frame.operation() == "splitter.commit_ratio"
            && frame
                .fields()
                .iter()
                .any(|(field, value)| field == "ratio" && value == "0.3")
    }));
    assert_eq!(
        logs.records()
            .iter()
            .filter(|record| record.level == bevy::log::Level::ERROR)
            .count(),
        1
    );
}

fn domain<'a>(diagnostic: &'a dyn Diagnostic) -> &'a SplitterError {
    let mut error: &'a (dyn Error + 'static) = diagnostic;
    loop {
        if let Some(kind) = error.downcast_ref::<SplitterError>() {
            return kind;
        }
        error = error.source().unwrap();
    }
}
fn fixture() -> App {
    let mut app = scene_app();
    add_ui_plugins(&mut app);
    spawn_ui_camera(&mut app, UVec2::splat(1200), 1.0);
    app.add_plugins(WidgetrySplitterPlugin);
    app.set_error_handler(ErrorCapture::handler());
    app.edit_schedule(PostUpdate, |schedule| {
        schedule.set_executor(bevy::ecs::schedule::SingleThreadedExecutor::new());
    });
    app
}
#[test]
fn all_broken_layout_owners_keep_numeric_fields_and_origins_until_actual_recovery() {
    let mut app = fixture();
    let roots: Vec<_> = (0..2).map(|_| app.world_mut().spawn_scene(bsn! { @WidgetrySplitter { @pane_a: bsn! { Node { min_width: px(400), max_width: px(300) } } } Node { width: px(1000), height: px(100) } }).unwrap().id()).collect();
    let errors = ErrorCapture::default();
    let logs = LogCapture::default();
    logs.run(|| {
        errors.run(|| {
            for _ in 0..12 {
                app.update();
            }
            let records = errors.take_records();
            assert!(!records.is_empty());
            let mut origins = std::collections::HashMap::new();
            for record in &records {
                assert_eq!(record.error.severity(), bevy::ecs::error::Severity::Error);
                let failures: Vec<_> = record.managed().unwrap().diagnostics().collect();
                assert_eq!(failures.len(), 2);
                for failure in failures {
                    let SplitterError::Layout {
                        root,
                        parts: Some(parts),
                        source,
                        ..
                    } = domain(failure)
                    else {
                        panic!("missing layout context")
                    };
                    assert!(roots.contains(root));
                    assert_eq!(parts[0], app.world().get::<Children>(*root).unwrap()[0]);
                    assert!(matches!(
                        source.as_ref(),
                        SplitterLayoutError::ConflictingPanes {
                            a: [Val::Px(400.0), Val::Px(300.0)],
                            ..
                        }
                    ));
                    let original = origins
                        .entry(*root)
                        .or_insert_with(|| failure.origin().clone());
                    assert!(Arc::ptr_eq(original, failure.origin()));
                }
            }
            assert_eq!(origins.len(), 2);
            assert_eq!(
                logs.records()
                    .iter()
                    .filter(|record| record.matches_field("phase", "source"))
                    .count(),
                2
            );
            for root in &roots {
                let a = app.world().get::<Children>(*root).unwrap()[0];
                app.world_mut().get_mut::<Node>(a).unwrap().min_width = px(0);
            }
            for _ in 0..12 {
                app.update();
            }
            assert!(errors.take_records().is_empty());
            assert_eq!(
                logs.records()
                    .iter()
                    .filter(|record| record.matches_field("phase", "recovered"))
                    .count(),
                2
            );
        })
    });
}

#[test]
fn style_batches_keep_each_line_identity_across_theme_refresh_and_repair() {
    let mut app = fixture();
    let roots: Vec<_> = (0..2)
        .map(|_| {
            app.world_mut()
                .spawn_scene(bsn! { @WidgetrySplitter Node { width: px(500), height: px(100) } })
                .unwrap()
                .id()
        })
        .collect();
    for _ in 0..12 {
        app.update();
    }
    let lines: Vec<_> = roots
        .iter()
        .map(|root| {
            let handle = app.world().get::<Children>(*root).unwrap()[1];
            app.world().get::<Children>(handle).unwrap()[0]
        })
        .collect();
    for line in &lines {
        app.world_mut()
            .entity_mut(*line)
            .remove::<BackgroundColor>();
    }
    let errors = ErrorCapture::default();
    let logs = LogCapture::default();
    logs.run(|| errors.run(|| {
        app.update(); let first = errors.take_records(); assert_eq!(first.len(), 1);
        let first: Vec<_> = first[0].managed().unwrap().diagnostics().collect(); assert_eq!(first.len(), 2);
        for failure in &first { assert!(matches!(domain(*failure), SplitterError::Layout { root, source, .. } if roots.contains(root) && matches!(source.as_ref(), SplitterLayoutError::MissingPart { entity, part: "BackgroundColor" } if lines.contains(entity)))); }
        bevy_widgetry_theme::WidgetryThemeMode::set_in_world(app.world_mut(), bevy_widgetry_theme::WidgetryThemeMode::Light).unwrap();
        app.update(); let repeated = errors.take_records();
        let repeated: Vec<_> = repeated[0].managed().unwrap().diagnostics().collect(); assert_eq!(repeated.len(), 2);
        for failure in repeated { assert!(first.iter().any(|original| Arc::ptr_eq(original.origin(), failure.origin()))); }
        assert_eq!(logs.records().iter().filter(|record| record.matches_field("phase", "source")).count(), 2);
        for line in &lines { app.world_mut().entity_mut(*line).insert(BackgroundColor::default()); }
        app.update(); assert!(errors.take_records().is_empty());
        assert_eq!(logs.records().iter().filter(|record| record.matches_field("phase", "recovered")).count(), 2);
    }));
}

#[test]
fn scene_dimension_failure_keeps_root_values_and_source_origin() {
    let mut app = fixture();
    let logs = LogCapture::default();
    logs.run(|| {
        let failure = bevy_widgetry_core::scene::spawn_scene(
            app.world_mut(),
            bsn! { @WidgetrySplitter { @handle_thickness: 8.0, @line_thickness: 9.0 } },
        )
        .unwrap_err();
        let diagnostic = failure.diagnostics().diagnostics().next().unwrap();
        let mut cause: &(dyn Error + 'static) = diagnostic;
        while !cause.is::<SplitterFailure>() {
            cause = cause.source().unwrap();
        }
        let original = cause.downcast_ref::<SplitterFailure>().unwrap();
        assert!(matches!(
            original.kind(),
            SplitterError::InvalidDimension {
                field: "line_thickness",
                value: 9.0,
                handle_thickness: 8.0,
                ..
            }
        ));
        assert!(Arc::ptr_eq(original.origin(), diagnostic.origin()));
        assert_eq!(
            logs.records()
                .iter()
                .filter(|record| record.matches_field("phase", "source"))
                .count(),
            1
        );
    });
}

#[test]
fn color_domain_failure_and_boundary_preserve_core_cause_and_original_origin() {
    use bevy_widgetry_splitter::WidgetrySplitterColorOverrides as Colors;
    let mut app = fixture();
    let root = app
        .world_mut()
        .spawn_scene(bsn! { @WidgetrySplitter })
        .unwrap()
        .id();
    let logs = LogCapture::default();
    logs.run(|| {
        let colors = Colors { hovered: Some(Color::linear_rgba(f32::NAN, 0.0, 0.0, 1.0)), ..default() };
        let failure = Colors::set_in_world(app.world_mut(), root, colors).unwrap_err();
        assert!(matches!(failure.kind(), SplitterError::Core { source: bevy_widgetry_core::diagnostics::CoreError::InvalidColor { color } } if color.to_linear().red.is_nan()));
        let origin = failure.origin().clone();
        let boundary = failure.into_bevy_error();
        assert_eq!(boundary.severity(), bevy::ecs::error::Severity::Error);
        let diagnostic = boundary.downcast_ref::<bevy_widgetry_diagnostics::bevy::WidgetryEcsError>().unwrap().diagnostics().next().unwrap();
        assert!(Arc::ptr_eq(&origin, diagnostic.origin()));
        assert!(matches!(diagnostic.source().unwrap().source().unwrap().downcast_ref::<bevy_widgetry_core::diagnostics::CoreError>(), Some(bevy_widgetry_core::diagnostics::CoreError::InvalidColor { .. })));
        assert_eq!(logs.diagnostic_records(origin.id(), bevy_widgetry_test_utils::DiagnosticPhase::Source).len(), 1);
        assert!(logs.diagnostic_records(origin.id(), bevy_widgetry_test_utils::DiagnosticPhase::Handled).is_empty());
        assert_eq!(Colors::get(app.world(), root).unwrap(), &Colors::default());
    });
}

#[test]
fn persistent_nan_constraints_keep_the_exact_pane_value_and_one_origin() {
    let mut app = scene_app();
    app.add_plugins(WidgetrySplitterPlugin);
    app.set_error_handler(ErrorCapture::handler());
    app.edit_schedule(PostUpdate, |schedule| {
        schedule.set_executor(bevy::ecs::schedule::SingleThreadedExecutor::new());
    });
    let nan = f32::from_bits(0x7fc0_0001);
    let root = app
        .world_mut()
        .spawn_scene(bsn! { @WidgetrySplitter { @pane_a: bsn! { Node { min_width: px(nan) } } } })
        .unwrap()
        .id();
    let pane = app.world().get::<Children>(root).unwrap()[0];
    let errors = ErrorCapture::default();
    let logs = LogCapture::default();
    logs.run(|| errors.run(|| {
        for _ in 0..3 { app.update(); }
        let records = errors.take_records(); assert_eq!(records.len(), 3);
        let mut origin = None;
        for record in &records {
            let diagnostic = record.managed().unwrap().diagnostics().next().unwrap();
            let SplitterError::Layout { root: actual, source, .. } = domain(diagnostic) else { panic!("missing layout context") };
            assert_eq!(*actual, root);
            assert!(matches!(source.as_ref(), SplitterLayoutError::AtPart { entity, part: "pane_a.constraints", source } if *entity == pane && matches!(source.as_ref(), SplitterLayoutError::InvalidConstraint { value: Val::Px(value) } if value.to_bits() == nan.to_bits())));
            let first = origin.get_or_insert_with(|| diagnostic.origin().clone()); assert!(Arc::ptr_eq(first, diagnostic.origin()));
        }
        assert_eq!(logs.records().iter().filter(|record| record.matches_field("phase", "source")).count(), 1);
        app.world_mut().get_mut::<Node>(pane).unwrap().min_width = px(f32::from_bits(0x7fc0_0002));
        app.update(); let changed = errors.take_records(); let changed = changed[0].managed().unwrap().diagnostics().next().unwrap();
        assert!(!Arc::ptr_eq(origin.as_ref().unwrap(), changed.origin()));
        assert_eq!(logs.records().iter().filter(|record| record.matches_field("phase", "source")).count(), 2);
    }));
}
