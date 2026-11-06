#![allow(clippy::disallowed_macros, clippy::unwrap_used)]

use bevy::prelude::*;
use bevy::ui::Checked;
use bevy_widgetry_core::{color::WidgetryStyleOwner, diagnostics::CoreError};
use bevy_widgetry_radio_group::{
    RadioGroupError, RadioGroupFailure, WidgetryRadioGroup, WidgetryRadioGroupColorOverrides,
    WidgetryRadioGroupHeadless, WidgetryRadioGroupPlugin, WidgetryRadioOption,
    WidgetryRadioOptionColorOverrides,
};
use bevy_widgetry_test_utils::{DiagnosticPhase, ErrorCapture, LogCapture, scene_app};
use std::error::Error;

#[test]
fn group_and_option_colors_preserve_core_source_origin_configuration_and_owner() {
    let mut app = scene_app();
    app.add_plugins(WidgetryRadioGroupPlugin);
    let group = app
        .world_mut()
        .spawn_scene(bsn! { @WidgetryRadioGroup Children [@WidgetryRadioOption] })
        .unwrap()
        .id();
    let option = app.world().get::<Children>(group).unwrap()[0];
    let logs = LogCapture::default();
    logs.run(|| {
        assert!(!WidgetryRadioGroupColorOverrides::clear_in_world(app.world_mut(), group).unwrap());
        assert!(!WidgetryRadioOptionColorOverrides::clear_in_world(app.world_mut(), option).unwrap());
        assert!(logs.records().is_empty());
        let mut group_colors = WidgetryRadioGroupColorOverrides::default();
        group_colors.container.focused.border = Some(Color::linear_rgba(f32::NAN, 0.0, 0.0, 1.0));
        let group_error = WidgetryRadioGroupColorOverrides::set_in_world(app.world_mut(), group, group_colors).unwrap_err();
        let mut option_colors = WidgetryRadioOptionColorOverrides::default();
        option_colors.checked.disabled.dot = Some(Color::linear_rgba(f32::NAN, 0.0, 0.0, 1.0));
        let option_error = WidgetryRadioOptionColorOverrides::set_in_world(app.world_mut(), option, option_colors).unwrap_err();
        for (failure, state, field) in [(group_error, "container.focused", "border"), (option_error, "option.checked.disabled", "dot")] {
            assert!(matches!(failure.kind(), RadioGroupError::Core { source: CoreError::InvalidColor { .. } }));
            assert!(failure.kind().source().unwrap().is::<CoreError>());
            let context = format!("{:?}", failure.contexts());
            assert!(context.contains(state) && context.contains(field));
            let origin = failure.origin().clone();
            let error = failure.into_bevy_error();
            let diagnostic = error.downcast_ref::<bevy_widgetry_diagnostics::bevy::WidgetryEcsError>().unwrap().diagnostics().next().unwrap();
            assert!(std::sync::Arc::ptr_eq(diagnostic.origin(), &origin));
            assert_eq!(logs.diagnostic_records(origin.id(), DiagnosticPhase::Source).len(), 1);
            assert!(logs.diagnostic_records(origin.id(), DiagnosticPhase::Handled).is_empty());
        }
        assert_eq!(*WidgetryRadioGroupColorOverrides::get(app.world(), group).unwrap(), WidgetryRadioGroupColorOverrides::default());
        assert_eq!(*WidgetryRadioOptionColorOverrides::get(app.world(), option).unwrap(), WidgetryRadioOptionColorOverrides::default());
        app.world_mut().entity_mut(option).insert(WidgetryStyleOwner::<WidgetryRadioOption>::new(group));
        let failure = WidgetryRadioOptionColorOverrides::clear_in_world(app.world_mut(), option).unwrap_err();
        assert!(matches!(failure.kind(), RadioGroupError::ManagedColors { entity, owner } if *entity == option && *owner == group));
    });
}

#[test]
fn independent_selection_requests_and_color_commands_keep_typed_identity_at_the_host() {
    let mut app = scene_app();
    app.add_plugins(WidgetryRadioGroupPlugin);
    app.set_error_handler(ErrorCapture::handler());
    let group = app
        .world_mut()
        .spawn_scene(bsn! { @WidgetryRadioGroup Children [@WidgetryRadioOption] })
        .unwrap()
        .id();
    let option = app.world().get::<Children>(group).unwrap()[0];
    app.update();
    let stale = app.world_mut().spawn_empty().id();
    app.world_mut().despawn(stale);
    let logs = LogCapture::default();
    let errors = ErrorCapture::default();
    for _ in 0..2 {
        WidgetryRadioGroupHeadless::set_selected(&mut app.world_mut().commands(), group, 10);
    }
    WidgetryRadioGroupColorOverrides::clear(&mut app.world_mut().commands(), stale);
    WidgetryRadioOptionColorOverrides::clear(&mut app.world_mut().commands(), stale);
    logs.run(|| errors.run(|| app.world_mut().flush()));
    let records = errors.take_records();
    assert_eq!(records.len(), 4);
    let mut ids = Vec::new();
    for (i, record) in records.iter().enumerate() {
        assert_eq!(record.error.severity(), bevy::ecs::error::Severity::Error);
        let diagnostic = record.managed().unwrap().diagnostics().next().unwrap();
        let kind = diagnostic
            .source()
            .unwrap()
            .downcast_ref::<RadioGroupError>()
            .unwrap();
        if i < 2 {
            assert!(
                matches!(kind, RadioGroupError::IndexOutOfBounds { group: target, index: 10, len: 1 } if *target == group)
            );
        } else {
            assert!(
                matches!(kind, RadioGroupError::InvalidTarget { entity, .. } if *entity == stale)
            );
        }
        assert_eq!(
            logs.diagnostic_records(diagnostic.origin().id(), DiagnosticPhase::Source)
                .len(),
            1
        );
        assert!(
            logs.diagnostic_records(diagnostic.origin().id(), DiagnosticPhase::Handled)
                .is_empty()
        );
        ids.push(diagnostic.origin().id().clone());
    }
    assert_ne!(ids[0], ids[1]);
    assert!(app.world().get::<Checked>(option).is_some());
}

#[test]
fn initial_group_and_option_color_errors_keep_core_identity_through_scene_cleanup() {
    let mut app = scene_app();
    app.add_plugins(WidgetryRadioGroupPlugin);
    for option in [false, true] {
        let count = app.world().entities().len();
        let logs = LogCapture::default();
        logs.run(|| {
            let scene = if option {
                let mut colors = WidgetryRadioOptionColorOverrides::default();
                colors.unchecked.hovered.foreground =
                    Some(Color::linear_rgba(f32::NAN, 0.0, 0.0, 1.0));
                Box::new(bsn! { @WidgetryRadioOption { @colors: {colors.clone()} } })
                    as Box<dyn Scene>
            } else {
                let mut colors = WidgetryRadioGroupColorOverrides::default();
                colors.option.checked.normal.background =
                    Some(Color::linear_rgba(f32::NAN, 0.0, 0.0, 1.0));
                Box::new(bsn! { @WidgetryRadioGroup { @colors: {colors.clone()} } })
                    as Box<dyn Scene>
            };
            let error = bevy_widgetry_core::scene::spawn_scene(app.world_mut(), scene).unwrap_err();
            let diagnostic = error.diagnostics().diagnostics().next().unwrap();
            let mut source: &(dyn Error + 'static) = diagnostic;
            while !source.is::<RadioGroupFailure>() {
                source = source.source().unwrap();
            }
            let failure = source.downcast_ref::<RadioGroupFailure>().unwrap();
            assert!(matches!(failure.kind(), RadioGroupError::Core { .. }));
            assert!(std::sync::Arc::ptr_eq(
                failure.origin(),
                diagnostic.origin()
            ));
            assert_eq!(
                logs.diagnostic_records(failure.origin().id(), DiagnosticPhase::Source)
                    .len(),
                1
            );
            assert_eq!(app.world().entities().len(), count);
        });
    }
}
