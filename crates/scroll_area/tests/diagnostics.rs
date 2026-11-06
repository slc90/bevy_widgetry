#![allow(clippy::disallowed_macros, clippy::unwrap_used)]
use bevy::prelude::*;
use bevy::ui::ScrollPosition;
use bevy::ui_widgets::{ControlOrientation, Scrollbar, ScrollbarThumb};
use bevy_widgetry_scroll_area::{
    ScrollAreaError, ScrollAxis, ScrollbarPolicy, WidgetryScrollArea, WidgetryScrollAreaPlugin,
    WidgetryScrollIntoView,
};
use bevy_widgetry_test_utils::{DiagnosticPhase, ErrorCapture, LogCapture, scene_app};
use std::sync::Arc;

fn app() -> App {
    let mut app = scene_app();
    app.add_plugins(WidgetryScrollAreaPlugin);
    app.set_error_handler(ErrorCapture::handler());
    app.edit_schedule(PostUpdate, |schedule| {
        schedule.set_executor(bevy::ecs::schedule::SingleThreadedExecutor::new());
    });
    app
}
fn root(app: &mut App) -> Entity {
    app.world_mut()
        .spawn_scene(bsn! { @WidgetryScrollArea })
        .unwrap()
        .id()
}
fn diagnostics(
    records: &[bevy_widgetry_test_utils::CapturedError],
) -> Vec<&dyn bevy_widgetry_diagnostics::bevy::Diagnostic> {
    records
        .iter()
        .flat_map(|record| {
            assert_eq!(record.error.severity(), bevy::ecs::error::Severity::Error);
            record.managed().unwrap().diagnostics()
        })
        .collect()
}

#[test]
fn broken_owned_viewports_are_reported_for_every_root() {
    let mut app = app();
    let roots: Vec<_> = (0..2).map(|_| root(&mut app)).collect();
    app.update();
    let viewports: Vec<_> = roots
        .iter()
        .map(|root| app.world().get::<Children>(*root).unwrap()[0])
        .collect();
    for viewport in &viewports {
        app.world_mut()
            .entity_mut(*viewport)
            .remove::<ScrollPosition>();
    }
    let errors = ErrorCapture::default();
    let logs = LogCapture::default();
    logs.run(|| errors.run(|| {
        app.update(); let first = errors.take_records(); let first = diagnostics(&first); assert_eq!(first.len(), 2);
        for diagnostic in &first {
            assert!(matches!(diagnostic.source().unwrap().downcast_ref::<ScrollAreaError>(), Some(ScrollAreaError::MissingPart { context, entity, part: "ScrollPosition" }) if roots.contains(&context.root) && context.viewport == Some(*entity) && context.content.is_some() && viewports.contains(entity) && context.axis == Some(ScrollAxis::Vertical) && context.viewport_size == Some(Vec2::ZERO)));
            assert_eq!(logs.diagnostic_records(diagnostic.origin().id(), DiagnosticPhase::Source).len(), 1);
        }
        app.update(); let second = errors.take_records(); let second = diagnostics(&second); assert_eq!(second.len(), 2);
        for diagnostic in &second { assert!(first.iter().any(|original| Arc::ptr_eq(original.origin(), diagnostic.origin()))); }
        for viewport in &viewports { app.world_mut().entity_mut(*viewport).insert(ScrollPosition::default()); }
        app.update(); assert!(errors.take_records().is_empty());
        assert_eq!(logs.records().iter().filter(|record| record.matches_field("phase", "recovered")).count(), 2);
        app.world_mut().entity_mut(viewports[0]).remove::<ScrollPosition>();
        app.update(); let again = errors.take_records(); let again = diagnostics(&again); assert_eq!(again.len(), 1);
        assert!(first.iter().all(|original| original.origin().id() != again[0].origin().id()));
    }));
}

#[test]
fn scrollbar_damage_keeps_axis_policy_owner_and_shared_style_identity() {
    let mut app = app();
    let root = root(&mut app);
    app.update();
    let bar = app.world().get::<Children>(root).unwrap()[1];
    let original = app.world_mut().entity_mut(bar).take::<Scrollbar>().unwrap();
    let errors = ErrorCapture::default();
    let logs = LogCapture::default();
    logs.run(|| errors.run(|| {
        app.update(); let records = errors.take_records(); let first = diagnostics(&records); assert_eq!(first.len(), 1);
        assert!(matches!(first[0].source().unwrap().downcast_ref::<ScrollAreaError>(), Some(ScrollAreaError::MissingPart { context, entity, part: "Scrollbar" }) if context.root == root && *entity == bar && context.axis == Some(ScrollAxis::Vertical) && context.policy == Some(ScrollbarPolicy::Auto)));
        let colors = bevy_widgetry_theme::WIDGETRY_DARK_THEME.scroll_area;
        let from_owner = bevy_widgetry_scroll_area::internal::apply_owned_scroll_colors(app.world_mut(), root, &colors).unwrap_err();
        let repeated = from_owner.diagnostics().next().unwrap();
        assert!(Arc::ptr_eq(first[0].origin(), repeated.origin()));
        assert_eq!(logs.diagnostic_records(repeated.origin().id(), DiagnosticPhase::Source).len(), 1);
        app.world_mut().entity_mut(bar).insert(original); app.update(); assert!(errors.take_records().is_empty());
    }));
}

#[test]
fn wrong_scrollbar_target_keeps_actual_and_expected_identity() {
    let mut app = app();
    let root = root(&mut app);
    app.update();
    let viewport = app.world().get::<Children>(root).unwrap()[0];
    let bar = app.world().get::<Children>(root).unwrap()[1];
    let other = app.world_mut().spawn(Node::default()).id();
    {
        let mut scrollbar = app.world_mut().get_mut::<Scrollbar>(bar).unwrap();
        scrollbar.target = other;
        scrollbar.orientation = ControlOrientation::Horizontal;
    }
    let errors = ErrorCapture::default();
    errors.run(|| app.update());
    let records = errors.take_records();
    let failures = diagnostics(&records);
    assert!(!failures.is_empty());
    for failure in &failures {
        assert!(
            matches!(failure.source().unwrap().downcast_ref::<ScrollAreaError>(), Some(ScrollAreaError::InvalidScrollbar { context, entity, target, axis: ScrollAxis::Horizontal }) if context.root == root && context.viewport == Some(viewport) && *entity == bar && *target == other && context.axis == Some(ScrollAxis::Vertical))
        );
        assert_eq!(failure.origin().id(), failures[0].origin().id());
    }
}

#[test]
fn waiting_layout_zero_geometry_empty_content_and_unrelated_events_are_quiet() {
    let mut app = app();
    let root = root(&mut app);
    let viewport = app.world().get::<Children>(root).unwrap()[0];
    app.world_mut()
        .entity_mut(viewport)
        .remove::<ComputedNode>();
    let outside = app.world_mut().spawn(Node::default()).id();
    let errors = ErrorCapture::default();
    let logs = LogCapture::default();
    logs.run(|| {
        errors.run(|| {
            for _ in 0..2 {
                app.world_mut()
                    .trigger(WidgetryScrollIntoView { entity: outside });
                app.update();
            }
            assert!(errors.take_records().is_empty());
            app.world_mut()
                .entity_mut(viewport)
                .insert(ComputedNode::default());
            app.update();
            assert!(errors.take_records().is_empty());
            assert!(
                logs.records()
                    .iter()
                    .all(|record| record.level != bevy::log::Level::ERROR)
            );
        })
    });
}

#[test]
fn invalid_thickness_propagates_through_scene_with_original_origin() {
    use bevy_widgetry_core::scene::WidgetrySceneCommandsExt;
    for thickness in [f32::NAN, f32::INFINITY, -1.0] {
        let mut app = app();
        let errors = ErrorCapture::default();
        let logs = LogCapture::default();
        let root = app.world_mut().commands().spawn_scene_with_error_handler(bsn! { @WidgetryScrollArea { @axis: ScrollAxis::Both, @scrollbar_thickness: thickness } }).id();
        logs.run(|| errors.run(|| app.world_mut().flush()));
        let records = errors.take_records();
        let failures = diagnostics(&records);
        assert_eq!(failures.len(), 1);
        let mut cause: &(dyn std::error::Error + 'static) = failures[0];
        while !cause.is::<bevy_widgetry_scroll_area::ScrollAreaFailure>() {
            cause = cause.source().unwrap();
        }
        let original = cause
            .downcast_ref::<bevy_widgetry_scroll_area::ScrollAreaFailure>()
            .unwrap();
        assert!(
            matches!(original.kind(), ScrollAreaError::InvalidThickness { root: actual, axis: ScrollAxis::Both, thickness: value, .. } if *actual == root && value.to_bits() == thickness.to_bits())
        );
        assert!(Arc::ptr_eq(original.origin(), failures[0].origin()));
        assert_eq!(
            logs.diagnostic_records(failures[0].origin().id(), DiagnosticPhase::Source)
                .len(),
            1
        );
        assert!(!app.world().entities().contains(root));
    }
}

#[test]
fn invalid_color_preserves_core_source_and_independent_request_ids() {
    use bevy_widgetry_scroll_area::WidgetryScrollAreaColorOverrides as Colors;
    let mut app = app();
    let root = root(&mut app);
    let mut colors = Colors::default();
    colors.vertical.thumb.dragged.background = Some(Color::linear_rgba(f32::NAN, 0.0, 0.0, 1.0));
    let errors: Vec<_> = (0..2)
        .map(|_| Colors::set_in_world(app.world_mut(), root, colors.clone()).unwrap_err())
        .collect();
    assert_ne!(errors[0].origin().id(), errors[1].origin().id());
    assert!(matches!(
        errors[0].kind(),
        ScrollAreaError::Core {
            source: bevy_widgetry_core::diagnostics::CoreError::InvalidColor { .. }
        }
    ));
    assert_eq!(Colors::get(app.world(), root).unwrap(), &Colors::default());
}

#[test]
fn missing_thumb_and_previously_measured_nodes_are_real_failures() {
    for remove_thumb in [false, true] {
        let mut app = app();
        let root = root(&mut app);
        app.update();
        let bar = app.world().get::<Children>(root).unwrap()[1];
        let thumb = app.world().get::<Children>(bar).unwrap()[0];
        let viewport = app.world().get::<Children>(root).unwrap()[0];
        if remove_thumb {
            app.world_mut().entity_mut(thumb).remove::<ScrollbarThumb>();
        } else {
            app.world_mut()
                .entity_mut(viewport)
                .remove::<ComputedNode>();
        }
        let errors = ErrorCapture::default();
        errors.run(|| app.update());
        let records = errors.take_records();
        let failures = diagnostics(&records);
        assert!(!failures.is_empty());
        for failure in failures {
            assert!(
                matches!(failure.source().unwrap().downcast_ref::<ScrollAreaError>(), Some(ScrollAreaError::MissingPart { context, entity, part }) if context.root == root && *entity == if remove_thumb { thumb } else { viewport } && *part == if remove_thumb { "ScrollbarThumb" } else { "ComputedNode" })
            );
        }
    }
}

#[test]
fn owned_into_view_failure_is_reported_at_command_boundary_before_update() {
    let mut app = app();
    let root = app
        .world_mut()
        .spawn_scene(bsn! { @WidgetryScrollArea { @children: bsn_list! { Node Name("target") } } })
        .unwrap()
        .id();
    app.update();
    let viewport = app.world().get::<Children>(root).unwrap()[0];
    let content = app.world().get::<Children>(viewport).unwrap()[0];
    let target = app.world().get::<Children>(content).unwrap()[0];
    app.world_mut()
        .entity_mut(viewport)
        .remove::<ScrollPosition>();
    let errors = ErrorCapture::default();
    let logs = LogCapture::default();
    logs.run(|| {
        errors.run(|| {
            app.world_mut()
                .trigger(WidgetryScrollIntoView { entity: target });
            app.world_mut().flush();
            let records = errors.take_records();
            let first = diagnostics(&records);
            assert_eq!(first.len(), 1);
            app.update();
            let second = errors.take_records();
            let second = diagnostics(&second);
            assert_eq!(second.len(), 1);
            assert!(Arc::ptr_eq(first[0].origin(), second[0].origin()));
            assert_eq!(
                logs.diagnostic_records(first[0].origin().id(), DiagnosticPhase::Source)
                    .len(),
                1
            );
        })
    });
}

#[test]
fn geometry_recovery_retries_root_and_thumb_style_work_without_another_input() {
    use bevy_widgetry_scroll_area::WidgetryScrollAreaColorOverrides as Colors;
    for hover_only in [false, true] {
        let mut app = app();
        let root = root(&mut app);
        app.update();
        let viewport = app.world().get::<Children>(root).unwrap()[0];
        let bar = app.world().get::<Children>(root).unwrap()[1];
        let thumb = app.world().get::<Children>(bar).unwrap()[0];
        app.world_mut()
            .entity_mut(viewport)
            .remove::<ScrollPosition>();
        let expected = if hover_only {
            app.world_mut()
                .entity_mut(thumb)
                .insert(bevy::picking::hover::Hovered(true));
            bevy_widgetry_theme::WIDGETRY_DARK_THEME
                .scroll_area
                .vertical
                .thumb
                .hovered
                .background
        } else {
            let mut colors = Colors::default();
            colors.vertical.thumb.normal.background = Some(Color::srgb(0.75, 0.25, 0.5));
            Colors::set_in_world(app.world_mut(), root, colors).unwrap();
            Color::srgb(0.75, 0.25, 0.5)
        };
        let errors = ErrorCapture::default();
        errors.run(|| app.update());
        assert!(!errors.take_records().is_empty());
        app.world_mut()
            .entity_mut(viewport)
            .insert(ScrollPosition::default());
        errors.run(|| {
            app.update();
            app.update();
        });
        assert!(errors.take_records().is_empty());
        assert_eq!(
            app.world().get::<BackgroundColor>(thumb).unwrap().0,
            expected
        );
    }
}
