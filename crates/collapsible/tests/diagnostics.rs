#![allow(clippy::disallowed_macros, clippy::unwrap_used)]

use bevy::prelude::*;
use bevy_widgetry_collapsible::{
    CollapsibleError, CollapsibleFailure, CollapsibleStructureError as Problem,
    WidgetryCollapsible, WidgetryCollapsibleChanged, WidgetryCollapsibleColorOverrides,
    WidgetryCollapsiblePlugin,
};
use bevy_widgetry_core::icon::{IconError, WidgetryIcon};
use bevy_widgetry_diagnostics::bevy::{Diagnostic, WidgetryEcsError};
use bevy_widgetry_test_utils::{DiagnosticPhase, ErrorCapture, LogCapture, scene_app};
use std::{error::Error, sync::Arc};

#[derive(Resource, Default)]
struct Changes(usize);

fn fixture() -> App {
    let mut app = scene_app();
    app.add_plugins(WidgetryCollapsiblePlugin);
    app.set_error_handler(ErrorCapture::handler());
    app.init_resource::<Changes>().add_observer(
        |_: On<WidgetryCollapsibleChanged>, mut changes: ResMut<Changes>| changes.0 += 1,
    );
    app.edit_schedule(PreUpdate, |schedule| {
        schedule.set_executor(bevy::ecs::schedule::SingleThreadedExecutor::new());
    });
    app.edit_schedule(PostUpdate, |schedule| {
        schedule.set_executor(bevy::ecs::schedule::SingleThreadedExecutor::new());
    });
    app
}

fn original(diagnostic: &dyn Diagnostic) -> &CollapsibleFailure {
    let mut cause: &(dyn Error + 'static) = diagnostic;
    loop {
        if let Some(failure) = cause.downcast_ref::<CollapsibleFailure>() {
            return failure;
        }
        cause = cause.source().unwrap();
    }
}
fn parts(world: &World, root: Entity) -> [Entity; 4] {
    let children = world.get::<Children>(root).unwrap();
    let header = children[0];
    let inner = world.get::<Children>(header).unwrap();
    [header, children[1], inner[0], inner[1]]
}
fn arrow(world: &World, root: Entity) -> Entity {
    let mut pending = vec![parts(world, root)[3]];
    let mut found = None;
    while let Some(entity) = pending.pop() {
        if world.get::<WidgetryIcon>(entity).is_some() {
            found = Some(entity);
            break;
        }
        if let Some(children) = world.get::<Children>(entity) {
            pending.extend(children.iter());
        }
    }
    found.unwrap()
}

#[test]
fn all_pending_broken_roots_reach_the_initialization_handler() {
    let mut app = fixture();
    let roots: Vec<_> = (0..2)
        .map(|_| {
            app.world_mut()
                .spawn_scene(bsn! { @WidgetryCollapsible })
                .unwrap()
                .id()
        })
        .collect();
    for root in &roots {
        let content = app.world().get::<Children>(*root).unwrap()[1];
        app.world_mut().entity_mut(content).remove::<Node>();
    }
    let healthy = app
        .world_mut()
        .spawn_scene(bsn! { @WidgetryCollapsible })
        .unwrap()
        .id();
    let canceled = app
        .world_mut()
        .spawn_scene(bsn! { @WidgetryCollapsible })
        .unwrap()
        .id();
    app.world_mut().despawn(canceled);
    let errors = ErrorCapture::default();
    let logs = LogCapture::default();
    logs.run(|| errors.run(|| app.world_mut().run_schedule(PreUpdate)));
    let records = errors.take_records();
    assert_eq!(records.len(), 1);
    assert_eq!(
        records[0].error.severity(),
        bevy::ecs::error::Severity::Error
    );
    let diagnostics: Vec<_> = records[0].managed().unwrap().diagnostics().collect();
    assert_eq!(diagnostics.len(), 2);
    assert!(!Arc::ptr_eq(
        diagnostics[0].origin(),
        diagnostics[1].origin()
    ));
    for diagnostic in diagnostics {
        assert!(
            matches!(original(diagnostic).kind(), CollapsibleError::Structure { source: Problem::MissingPart { root, entity, part: "content.Node" } } if roots.contains(root) && app.world().get::<Children>(*root).unwrap()[1] == *entity)
        );
        assert!(Arc::ptr_eq(
            original(diagnostic).origin(),
            diagnostic.origin()
        ));
        assert!(
            diagnostic
                .contexts()
                .iter()
                .any(|frame| frame.operation() == "collapsible.initialize")
        );
        assert_eq!(
            logs.diagnostic_records(diagnostic.origin().id(), DiagnosticPhase::Source)
                .len(),
            1
        );
    }
    errors.run(|| app.world_mut().run_schedule(PreUpdate));
    assert!(errors.take_records().is_empty());
    assert!(WidgetryCollapsible::expanded(app.world(), healthy).unwrap());
    assert_eq!(app.world().resource::<Changes>().0, 0);
}

#[test]
fn each_owned_part_preserves_missing_component_and_parent_identity() {
    let mut app = fixture();
    let root = app
        .world_mut()
        .spawn_scene(bsn! { @WidgetryCollapsible })
        .unwrap()
        .id();
    app.update();
    let [header, content, header_content, button] = parts(app.world(), root);
    let foreign = app.world_mut().spawn_scene(bsn! { Node }).unwrap().id();
    for (entity, parent, role, node_part) in [
        (header, root, "header", "header.Node"),
        (content, root, "content", "content.Node"),
        (
            header_content,
            header,
            "header_content",
            "header_content.Node",
        ),
        (button, header, "button", "button.Node"),
    ] {
        let node = app.world_mut().entity_mut(entity).take::<Node>().unwrap();
        let failure =
            WidgetryCollapsible::set_expanded_in_world(app.world_mut(), root, false).unwrap_err();
        assert!(
            matches!(failure.kind(), CollapsibleError::Structure { source: Problem::MissingPart { root: actual, entity: actual_entity, part } } if *actual == root && *actual_entity == entity && *part == node_part)
        );
        app.world_mut().entity_mut(entity).insert(node);
        app.world_mut().entity_mut(entity).insert(ChildOf(foreign));
        let failure =
            WidgetryCollapsible::set_expanded_in_world(app.world_mut(), root, false).unwrap_err();
        assert!(
            matches!(failure.kind(), CollapsibleError::Structure { source: Problem::InvalidParent { root: actual, entity: actual_entity, part, expected, actual: Some(actual_parent) } } if *actual == root && *actual_entity == entity && *part == role && *expected == parent && *actual_parent == foreign)
        );
        app.world_mut().entity_mut(entity).insert(ChildOf(parent));
    }
    app.world_mut()
        .entity_mut(button)
        .remove::<bevy_widgetry_button::WidgetryButton>();
    let failure = WidgetryCollapsible::expanded(app.world(), root).unwrap_err();
    assert!(
        matches!(failure.kind(), CollapsibleError::Structure { source: Problem::InvalidButton { root: actual, button: actual_button } } if *actual == root && *actual_button == button)
    );
    app.world_mut()
        .entity_mut(button)
        .insert(bevy_widgetry_button::WidgetryButton);
    assert!(WidgetryCollapsible::expanded(app.world(), root).unwrap());
    assert_eq!(app.world().resource::<Changes>().0, 0);
}

#[test]
fn icon_failure_keeps_source_origin_and_request_without_committing_or_moving_focus() {
    use bevy::input_focus::{FocusCause, InputFocus};
    let mut app = fixture();
    let root = app
        .world_mut()
        .spawn_scene(bsn! { @WidgetryCollapsible })
        .unwrap()
        .id();
    app.update();
    let [_, content, _, button] = parts(app.world(), root);
    let focused = app
        .world_mut()
        .spawn_scene(bsn! { Node ChildOf(content) })
        .unwrap()
        .id();
    app.world_mut()
        .resource_mut::<InputFocus>()
        .set(focused, FocusCause::Navigated);
    let icon = arrow(app.world(), root);
    let saved = app
        .world_mut()
        .entity_mut(icon)
        .take::<WidgetryIcon>()
        .unwrap();
    let logs = LogCapture::default();
    logs.run(|| {
        let failure = WidgetryCollapsible::set_expanded_in_world(app.world_mut(), root, false).unwrap_err();
        assert!(matches!(failure.kind(), CollapsibleError::Icon { source: IconError::InvalidTarget { entity, missing: "WidgetryIcon", path: Some(_) } } if *entity == icon));
        assert!(failure.source().unwrap().source().unwrap().is::<IconError>());
        assert!(failure.contexts().iter().any(|frame| frame.operation() == "collapsible.set_expanded" && frame.fields().iter().any(|(key, value)| key == "expanded" && value == "false")));
        let origin = failure.origin().clone();
        let boundary = failure.into_bevy_error();
        assert_eq!(boundary.severity(), bevy::ecs::error::Severity::Error);
        let diagnostic = boundary.downcast_ref::<WidgetryEcsError>().unwrap().diagnostics().next().unwrap();
        assert!(Arc::ptr_eq(&origin, diagnostic.origin()));
        assert_eq!(logs.diagnostic_records(origin.id(), DiagnosticPhase::Source).len(), 1);
        assert!(logs.diagnostic_records(origin.id(), DiagnosticPhase::Handled).is_empty());
        let errors = ErrorCapture::default();
        WidgetryCollapsible::set_expanded(&mut app.world_mut().commands(), root, false);
        errors.run(|| app.world_mut().flush());
        let records = errors.take_records();
        assert_eq!(records.len(), 1);
        assert!(matches!(records[0].context, bevy::ecs::error::ErrorContext::Command { .. }));
        let diagnostic = records[0].managed().unwrap().diagnostics().next().unwrap();
        assert!(matches!(original(diagnostic).kind(), CollapsibleError::Icon { source: IconError::InvalidTarget { entity, .. } } if *entity == icon));
        assert_eq!(logs.diagnostic_records(diagnostic.origin().id(), DiagnosticPhase::Source).len(), 1);
    });
    assert!(WidgetryCollapsible::expanded(app.world(), root).unwrap());
    assert_eq!(
        app.world().get::<Node>(content).unwrap().display,
        Display::Flex
    );
    assert_eq!(app.world().resource::<InputFocus>().get(), Some(focused));
    assert_eq!(app.world().resource::<Changes>().0, 0);
    app.world_mut().entity_mut(icon).insert(saved);
    assert!(WidgetryCollapsible::set_expanded_in_world(app.world_mut(), root, false).unwrap());
    assert_eq!(app.world().resource::<InputFocus>().get(), Some(button));
    assert_eq!(app.world().resource::<Changes>().0, 1);
}

#[test]
fn missing_chevron_is_distinct_from_an_icon_failure_and_same_value_remains_silent() {
    let mut app = fixture();
    let root = app
        .world_mut()
        .spawn_scene(bsn! { @WidgetryCollapsible })
        .unwrap()
        .id();
    app.update();
    let button = parts(app.world(), root)[3];
    let icon = arrow(app.world(), root);
    app.world_mut().despawn(icon);
    assert!(!WidgetryCollapsible::set_expanded_in_world(app.world_mut(), root, true).unwrap());
    let failure =
        WidgetryCollapsible::set_expanded_in_world(app.world_mut(), root, false).unwrap_err();
    assert!(
        matches!(failure.kind(), CollapsibleError::Structure { source: Problem::MissingPart { root: actual, entity, part: "Chevron" } } if *actual == root && *entity == button)
    );
    assert!(WidgetryCollapsible::expanded(app.world(), root).unwrap());
    assert_eq!(app.world().resource::<Changes>().0, 0);
}

#[test]
fn scene_color_validation_preserves_core_source_and_one_origin() {
    let mut app = fixture();
    let logs = LogCapture::default();
    logs.run(|| {
        let colors = WidgetryCollapsibleColorOverrides { border: Some(Color::linear_rgba(0.0, 0.0, 0.0, f32::NAN)), ..default() };
        let failure = bevy_widgetry_core::scene::spawn_scene(app.world_mut(), bsn! { @WidgetryCollapsible { @colors: colors } }).unwrap_err();
        let diagnostic = failure.diagnostics().diagnostics().next().unwrap();
        assert!(matches!(original(diagnostic).kind(), CollapsibleError::Core { source: bevy_widgetry_core::diagnostics::CoreError::InvalidColor { color } } if color.to_linear().alpha.is_nan()));
        assert!(Arc::ptr_eq(original(diagnostic).origin(), diagnostic.origin()));
        assert_eq!(logs.diagnostic_records(diagnostic.origin().id(), DiagnosticPhase::Source).len(), 1);
    });
}

#[test]
fn style_failures_keep_each_root_origin_until_actual_repair() {
    use bevy_widgetry_theme::WidgetryThemeMode;
    let mut app = fixture();
    let roots: Vec<_> = (0..2)
        .map(|_| {
            app.world_mut()
                .spawn_scene(bsn! { @WidgetryCollapsible })
                .unwrap()
                .id()
        })
        .collect();
    app.update();
    for root in &roots {
        app.world_mut().entity_mut(*root).remove::<BorderColor>();
    }
    let logs = LogCapture::default();
    let errors = ErrorCapture::default();
    logs.run(|| errors.run(|| {
        WidgetryThemeMode::set_in_world(app.world_mut(), WidgetryThemeMode::Light).unwrap();
        app.update();
        let first = errors.take_records();
        let first: Vec<_> = first.iter().flat_map(|record| record.managed().unwrap().diagnostics()).collect();
        assert_eq!(first.len(), 2);
        for diagnostic in &first {
            assert!(matches!(original(*diagnostic).kind(), CollapsibleError::Structure { source: Problem::MissingPart { root, entity, part: "BorderColor" } } if roots.contains(root) && root == entity));
        }
        WidgetryThemeMode::set_in_world(app.world_mut(), WidgetryThemeMode::Dark).unwrap();
        app.update();
        let repeated = errors.take_records();
        let repeated: Vec<_> = repeated.iter().flat_map(|record| record.managed().unwrap().diagnostics()).collect();
        assert_eq!(repeated.len(), 2);
        for diagnostic in repeated { assert!(first.iter().any(|prior| Arc::ptr_eq(prior.origin(), diagnostic.origin()))); }
        for diagnostic in &first { assert_eq!(logs.diagnostic_records(diagnostic.origin().id(), DiagnosticPhase::Source).len(), 1); }
        for root in &roots { app.world_mut().entity_mut(*root).insert(BorderColor::default()); }
        WidgetryThemeMode::set_in_world(app.world_mut(), WidgetryThemeMode::Light).unwrap();
        app.update();
        assert!(errors.take_records().is_empty());
        assert_eq!(logs.records().iter().filter(|record| record.matches_field("phase", "recovered")).count(), 2);
        assert_eq!(app.world().resource::<Changes>().0, 0);
    }));
}
