#![allow(clippy::disallowed_macros, clippy::unwrap_used)]

use bevy::prelude::*;
use bevy_widgetry_test_utils::{DiagnosticPhase, LogCapture, scene_app};
use bevy_widgetry_tooltip::*;
use std::error::Error;

static TEST_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

#[test]
fn color_api_keeps_core_origin_and_rejects_invalid_and_managed_targets() {
    let _guard = TEST_LOCK.lock().unwrap();
    let mut app = scene_app();
    app.add_plugins(WidgetryTooltipPlugin);
    let root = bevy_widgetry_core::scene::spawn_scene(
        app.world_mut(),
        bsn! {
            @WidgetryTooltip { @content: {TooltipContentFactory::new(|| bsn_list!{Node})} }
        },
    )
    .unwrap();
    let logs = LogCapture::default();
    logs.run(|| {
        let mut colors = WidgetryTooltipColorOverrides::default();
        colors.popup.normal.foreground = Some(Color::linear_rgba(f32::NAN,0.,0.,1.));
        let error = WidgetryTooltipColorOverrides::set_in_world(app.world_mut(), root, colors).unwrap_err();
        assert!(matches!(error.kind(), TooltipError::Core { source: bevy_widgetry_core::diagnostics::CoreError::InvalidColor {..} }));
        assert!(error.kind().source().unwrap().is::<bevy_widgetry_core::diagnostics::CoreError>());
        assert_eq!(logs.diagnostic_records(error.origin().id(), DiagnosticPhase::Source).len(), 1);
        assert!(error.contexts().iter().any(|frame| frame.operation() == "tooltip.set_colors"));
        let invalid = WidgetryTooltipColorOverrides::get(app.world(), Entity::PLACEHOLDER).unwrap_err();
        assert!(matches!(invalid.kind(), TooltipError::InvalidTarget {entity} if *entity == Entity::PLACEHOLDER));
        app.world_mut().entity_mut(root).insert(bevy_widgetry_core::color::WidgetryStyleOwner::<WidgetryTooltip>::new(Entity::PLACEHOLDER));
        let managed = WidgetryTooltipColorOverrides::clear_in_world(app.world_mut(), root).unwrap_err();
        assert!(matches!(managed.kind(), TooltipError::ManagedColor {entity, owner} if *entity == root && *owner == Entity::PLACEHOLDER));
    });
}

#[test]
fn missing_content_scene_keeps_classification_and_each_request_has_a_new_origin() {
    let _guard = TEST_LOCK.lock().unwrap();
    let mut app = scene_app();
    app.add_plugins(WidgetryTooltipPlugin);
    let logs = LogCapture::default();
    logs.run(|| {
        let mut previous = None;
        for _ in 0..2 {
            let error =
                bevy_widgetry_core::scene::spawn_scene(app.world_mut(), bsn! { @WidgetryTooltip })
                    .unwrap_err()
                    .into_bevy_error();
            let diagnostic = error
                .downcast_ref::<bevy_widgetry_diagnostics::bevy::WidgetryEcsError>()
                .unwrap()
                .diagnostics()
                .next()
                .unwrap();
            assert_eq!(error.severity(), bevy::ecs::error::Severity::Error);
            let mut source = diagnostic.source();
            let mut found = false;
            while let Some(error) = source {
                if matches!(
                    error.downcast_ref::<TooltipError>(),
                    Some(TooltipError::MissingContent { .. })
                ) {
                    found = true;
                    break;
                }
                source = error.source();
            }
            assert!(found);
            let id = diagnostic.origin().id().clone();
            assert_ne!(previous.as_ref(), Some(&id));
            previous = Some(id.clone());
            assert_eq!(
                logs.diagnostic_records(&id, DiagnosticPhase::Source).len(),
                1
            );
            assert!(
                logs.diagnostic_records(&id, DiagnosticPhase::Handled)
                    .is_empty()
            );
        }
    });
}
