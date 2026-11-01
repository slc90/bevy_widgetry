#![allow(clippy::disallowed_macros, clippy::unwrap_used)]

use bevy::ecs::schedule::ScheduleLabel;
use bevy::prelude::*;
use bevy_widgetry_diagnostics::bevy::{Diagnostic, WidgetryEcsError};
use bevy_widgetry_tabs::{
    TabsError, TabsFailure, TabsStructureError, WidgetryTab, WidgetryTabColorOverrides,
    WidgetryTabs, WidgetryTabsPlugin,
};
use bevy_widgetry_test_utils::{DiagnosticPhase, ErrorCapture, LogCapture, scene_app};
use std::{error::Error, sync::Arc};

fn item() -> impl Scene {
    bsn! { @WidgetryTab { @header: bsn! { Node }, @content: bsn! { Node } } }
}
fn scene(selected: usize) -> impl Scene {
    bsn! { @WidgetryTabs { @selected_index: selected, @children: {Some(Box::new(vec![item(), item()]) as Box<dyn SceneList>)} } }
}
fn fixture() -> App {
    let mut app = scene_app();
    app.add_plugins(WidgetryTabsPlugin);
    app.set_error_handler(ErrorCapture::handler());
    for schedule in [PreUpdate.intern(), PostUpdate.intern()] {
        app.edit_schedule(schedule, |schedule| {
            schedule.set_executor(bevy::ecs::schedule::SingleThreadedExecutor::new());
        });
    }
    app
}
fn original(diagnostic: &dyn Diagnostic) -> &TabsFailure {
    let mut cause: &(dyn Error + 'static) = diagnostic;
    loop {
        if let Some(failure) = cause.downcast_ref::<TabsFailure>() {
            return failure;
        }
        cause = cause.source().unwrap();
    }
}

#[test]
fn all_pending_roots_keep_distinct_initial_fields_and_origins_without_repeated_initialization() {
    let mut app = fixture();
    let empty = app
        .world_mut()
        .spawn_scene(bsn! { @WidgetryTabs })
        .unwrap()
        .id();
    let invalid = app.world_mut().spawn_scene(scene(2)).unwrap().id();
    let healthy = app.world_mut().spawn_scene(scene(0)).unwrap().id();
    let canceled = app.world_mut().spawn_scene(scene(0)).unwrap().id();
    app.world_mut().despawn(canceled);
    let errors = ErrorCapture::default();
    let logs = LogCapture::default();
    logs.run(|| errors.run(|| {
        app.update();
        let records = errors.take_records();
        assert_eq!(records.len(), 1);
        assert_eq!(records[0].error.severity(), bevy::ecs::error::Severity::Error);
        let failures: Vec<_> = records[0].managed().unwrap().diagnostics().collect();
        assert_eq!(failures.len(), 2);
        assert!(!Arc::ptr_eq(failures[0].origin(), failures[1].origin()));
        assert!(failures.iter().any(|diagnostic| matches!(original(*diagnostic).kind(), TabsError::InvalidIndex { root, index: 2, count: 2, initial: true } if *root == invalid)));
        assert!(failures.iter().any(|diagnostic| matches!(original(*diagnostic).kind(), TabsError::Structure { source: TabsStructureError::Empty { root, .. } } if *root == empty)));
        for diagnostic in failures {
            assert!(Arc::ptr_eq(original(diagnostic).origin(), diagnostic.origin()));
            assert!(diagnostic.contexts().iter().any(|frame| frame.operation() == "tabs.initialize"));
            assert_eq!(logs.diagnostic_records(diagnostic.origin().id(), DiagnosticPhase::Source).len(), 1);
            assert!(logs.diagnostic_records(diagnostic.origin().id(), DiagnosticPhase::Handled).is_empty());
        }
        for _ in 0..3 { app.update(); }
        assert!(errors.take_records().is_empty());
        assert_eq!(WidgetryTabs::selected_index(app.world(), healthy).unwrap(), 0);
    }));
}

#[test]
fn structural_rejections_preserve_actual_entities_parents_and_fixed_pair_order() {
    let mut app = fixture();
    let root = app.world_mut().spawn_scene(scene(0)).unwrap().id();
    app.update();
    let headers = [
        WidgetryTabs::header(app.world(), root, 0).unwrap(),
        WidgetryTabs::header(app.world(), root, 1).unwrap(),
    ];
    let pages = [
        WidgetryTabs::page(app.world(), root, 0).unwrap(),
        WidgetryTabs::page(app.world(), root, 1).unwrap(),
    ];
    let container = app.world().get::<ChildOf>(pages[1]).unwrap().parent();
    let foreign = app.world_mut().spawn(Node::default()).id();
    app.world_mut()
        .entity_mut(pages[1])
        .insert(ChildOf(foreign));
    let failure = WidgetryTabs::set_selected_in_world(app.world_mut(), root, 1).unwrap_err();
    assert!(
        matches!(failure.kind(), TabsError::Structure { source: TabsStructureError::InvalidParent { root: actual, entity, part: "page", expected, actual: Some(parent) } } if *actual == root && *entity == pages[1] && *expected == container && *parent == foreign)
    );
    assert!(
        failure
            .contexts()
            .iter()
            .any(|frame| frame.operation() == "tabs.set_selected.validate")
    );
    app.world_mut()
        .entity_mut(pages[1])
        .insert(ChildOf(container));
    app.world_mut().entity_mut(pages[1]).remove::<Node>();
    let failure = WidgetryTabs::set_selected_in_world(app.world_mut(), root, 1).unwrap_err();
    assert!(
        matches!(failure.kind(), TabsError::Structure { source: TabsStructureError::MissingPart { root: actual, entity, part: "Node" } } if *actual == root && *entity == pages[1])
    );
    app.world_mut().entity_mut(pages[1]).insert(Node::default());
    let bar = app.world().get::<Children>(root).unwrap()[0];
    app.world_mut().entity_mut(bar).insert(ChildOf(foreign));
    let failure = WidgetryTabs::set_selected_in_world(app.world_mut(), root, 1).unwrap_err();
    assert!(
        matches!(failure.kind(), TabsError::Structure { source: TabsStructureError::InvalidParent { root: actual, entity, part: "bar", expected, actual: Some(parent) } } if *actual == root && *entity == bar && *expected == root && *parent == foreign)
    );
    app.world_mut().entity_mut(bar).insert(ChildOf(root));
    let strip = app.world().get::<ChildOf>(headers[0]).unwrap().parent();
    app.world_mut()
        .entity_mut(strip)
        .insert_children(0, &[headers[1], headers[0]]);
    let failure = WidgetryTabs::set_selected_in_world(app.world_mut(), root, 1).unwrap_err();
    assert!(
        matches!(failure.kind(), TabsError::FixedPairs { root: actual, expected, actual: pairs } if *actual == root && *expected == vec![[headers[0], pages[0]], [headers[1], pages[1]]] && *pairs == vec![[headers[1], pages[1]], [headers[0], pages[0]]])
    );
    app.world_mut()
        .entity_mut(strip)
        .insert_children(0, &headers);
    assert_eq!(WidgetryTabs::selected_index(app.world(), root).unwrap(), 0);
}

#[test]
fn removing_the_selected_last_pair_reports_fixed_relationships_before_index_validity() {
    let mut app = fixture();
    let root = app.world_mut().spawn_scene(scene(1)).unwrap().id();
    app.update();
    let headers = [
        WidgetryTabs::header(app.world(), root, 0).unwrap(),
        WidgetryTabs::header(app.world(), root, 1).unwrap(),
    ];
    let pages = [
        WidgetryTabs::page(app.world(), root, 0).unwrap(),
        WidgetryTabs::page(app.world(), root, 1).unwrap(),
    ];
    let strip = app.world().get::<ChildOf>(headers[1]).unwrap().parent();
    let ellipsis = app
        .world()
        .get::<Children>(strip)
        .unwrap()
        .iter()
        .find(|entity| {
            app.world()
                .get::<bevy_widgetry_core::color::WidgetryStyleOwner<WidgetryTab>>(*entity)
                .is_some_and(|owner| owner.entity == headers[1])
        })
        .unwrap();
    let foreign = app.world_mut().spawn(Node::default()).id();
    for entity in [headers[1], ellipsis] {
        app.world_mut().entity_mut(entity).insert(ChildOf(foreign));
    }
    let failure = WidgetryTabs::set_selected_in_world(app.world_mut(), root, 0).unwrap_err();
    assert!(
        matches!(failure.kind(), TabsError::FixedPairs { root: actual, expected, actual: pairs } if *actual == root && *expected == vec![[headers[0], pages[0]], [headers[1], pages[1]]] && *pairs == vec![[headers[0], pages[0]] ])
    );
    for entity in [headers[1], ellipsis] {
        app.world_mut().entity_mut(entity).insert(ChildOf(strip));
    }
    assert_eq!(WidgetryTabs::selected_index(app.world(), root).unwrap(), 1);
}

#[test]
fn removing_all_pairs_keeps_original_relationships_for_queries_and_selection_requests() {
    let mut app = fixture();
    let root = app.world_mut().spawn_scene(bsn! {
        @WidgetryTabs { @children: bsn! { @WidgetryTab { @header: bsn! { Node }, @content: bsn! { Node } } } }
    }).unwrap().id();
    app.update();
    let header = WidgetryTabs::header(app.world(), root, 0).unwrap();
    let page = WidgetryTabs::page(app.world(), root, 0).unwrap();
    let strip = app.world().get::<ChildOf>(header).unwrap().parent();
    let children: Vec<_> = app.world().get::<Children>(strip).unwrap().iter().collect();
    let foreign = app.world_mut().spawn(Node::default()).id();
    for entity in &children {
        app.world_mut().entity_mut(*entity).insert(ChildOf(foreign));
    }
    for failure in [
        WidgetryTabs::selected_index(app.world(), root).unwrap_err(),
        WidgetryTabs::set_selected_in_world(app.world_mut(), root, 0).unwrap_err(),
    ] {
        assert!(
            matches!(failure.kind(), TabsError::FixedPairs { root: actual, expected, actual: pairs } if *actual == root && *expected == vec![[header, page]] && pairs.is_empty())
        );
    }
    for entity in children {
        app.world_mut().entity_mut(entity).insert(ChildOf(strip));
    }
    assert_eq!(WidgetryTabs::selected_index(app.world(), root).unwrap(), 0);
}

#[test]
fn queued_rejection_and_scene_failures_keep_original_diagnostics_and_core_sources() {
    let mut app = fixture();
    let root = app.world_mut().spawn_scene(scene(0)).unwrap().id();
    app.update();
    let errors = ErrorCapture::default();
    let logs = LogCapture::default();
    logs.run(|| {
        WidgetryTabs::set_selected(&mut app.world_mut().commands(), root, 2);
        errors.run(|| app.world_mut().flush());
        let records = errors.take_records();
        assert_eq!(records.len(), 1);
        assert!(matches!(records[0].context, bevy::ecs::error::ErrorContext::Command { .. }));
        assert_eq!(records[0].error.severity(), bevy::ecs::error::Severity::Error);
        let diagnostic = records[0].managed().unwrap().diagnostics().next().unwrap();
        assert!(matches!(original(diagnostic).kind(), TabsError::InvalidIndex { root: actual, index: 2, count: 2, initial: false } if *actual == root));
        assert!(Arc::ptr_eq(original(diagnostic).origin(), diagnostic.origin()));
        assert_eq!(logs.diagnostic_records(diagnostic.origin().id(), DiagnosticPhase::Source).len(), 1);
        let failure = bevy_widgetry_core::scene::spawn_scene(app.world_mut(), bsn! { @WidgetryTab { @header: bsn! { Node } } }).unwrap_err();
        let diagnostic = failure.diagnostics().diagnostics().next().unwrap();
        assert!(matches!(original(diagnostic).kind(), TabsError::IncompleteTab { has_header: true, has_content: false, .. }));
        assert!(Arc::ptr_eq(original(diagnostic).origin(), diagnostic.origin()));
        assert_eq!(logs.diagnostic_records(diagnostic.origin().id(), DiagnosticPhase::Source).len(), 1);
        let invalid = WidgetryTabColorOverrides { focus_border: Some(Color::linear_rgba(f32::NAN, 0.0, 0.0, 1.0)), ..default() };
        let failure = bevy_widgetry_core::scene::spawn_scene(app.world_mut(), bsn! { @WidgetryTab { @colors: invalid, @header: bsn! { Node }, @content: bsn! { Node } } }).unwrap_err();
        let diagnostic = failure.diagnostics().diagnostics().next().unwrap();
        assert!(matches!(original(diagnostic).kind(), TabsError::Core { source: bevy_widgetry_core::diagnostics::CoreError::InvalidColor { color } } if color.to_linear().red.is_nan()));
        assert!(Arc::ptr_eq(original(diagnostic).origin(), diagnostic.origin()));
        assert!(original(diagnostic).source().unwrap().source().unwrap().is::<bevy_widgetry_core::diagnostics::CoreError>());
        assert_eq!(logs.diagnostic_records(diagnostic.origin().id(), DiagnosticPhase::Source).len(), 1);
    });
}

#[test]
fn style_batches_keep_each_owner_origin_across_theme_refresh_recovery_and_recurrence() {
    let mut app = fixture();
    let roots: Vec<_> = (0..2)
        .map(|_| app.world_mut().spawn_scene(scene(0)).unwrap().id())
        .collect();
    app.update();
    let headers: Vec<_> = roots
        .iter()
        .map(|root| WidgetryTabs::header(app.world(), *root, 0).unwrap())
        .collect();
    for header in &headers {
        app.world_mut()
            .entity_mut(*header)
            .remove::<BackgroundColor>();
    }
    let errors = ErrorCapture::default();
    let logs = LogCapture::default();
    logs.run(|| errors.run(|| {
        app.update();
        let first = errors.take_records();
        assert_eq!(first.len(), 1);
        let first: Vec<_> = first[0].managed().unwrap().diagnostics().collect();
        assert_eq!(first.len(), 2);
        for diagnostic in &first {
            assert!(matches!(original(*diagnostic).kind(), TabsError::Structure { source: TabsStructureError::MissingPart { root, entity, part: "BackgroundColor" } } if roots.iter().zip(&headers).any(|(r, h)| r == root && h == entity)));
        }
        bevy_widgetry_theme::WidgetryThemeMode::set_in_world(app.world_mut(), bevy_widgetry_theme::WidgetryThemeMode::Light).unwrap();
        app.update();
        let repeated = errors.take_records();
        assert_eq!(repeated.len(), 1);
        let repeated: Vec<_> = repeated[0].managed().unwrap().diagnostics().collect();
        assert_eq!(repeated.len(), 2);
        for diagnostic in repeated { assert!(first.iter().any(|prior| Arc::ptr_eq(prior.origin(), diagnostic.origin()))); }
        for diagnostic in &first { assert_eq!(logs.diagnostic_records(diagnostic.origin().id(), DiagnosticPhase::Source).len(), 1); }
        for header in &headers { app.world_mut().entity_mut(*header).insert(BackgroundColor::default()); }
        app.update();
        assert!(errors.take_records().is_empty());
        for diagnostic in &first { assert_eq!(logs.records().iter().filter(|record| record.matches_field("phase", "recovered") && record.matches_field("error_id", &diagnostic.origin().id().to_string())).count(), 1); }
        app.world_mut().entity_mut(headers[0]).remove::<BackgroundColor>();
        app.update();
        let next = errors.take_records();
        assert_eq!(next.len(), 1);
        let diagnostic = next[0].error.downcast_ref::<WidgetryEcsError>().unwrap().diagnostics().next().unwrap();
        assert!(!first.iter().any(|prior| Arc::ptr_eq(prior.origin(), diagnostic.origin())));
    }));
}
