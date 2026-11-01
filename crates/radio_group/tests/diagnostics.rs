#![allow(clippy::disallowed_macros, clippy::unwrap_used)]

use bevy::prelude::*;
use bevy::ui::Checked;
use bevy_widgetry_radio_group::RadioGroupError;
use bevy_widgetry_radio_group::{
    WidgetryRadioGroup, WidgetryRadioGroupPlugin, WidgetryRadioOption,
};
use bevy_widgetry_test_utils::{DiagnosticPhase, ErrorCapture, LogCapture, scene_app};

#[test]
fn every_invalid_group_and_option_reaches_the_host_without_polling_or_recreating_the_source() {
    let mut app = scene_app();
    app.add_plugins(WidgetryRadioGroupPlugin);
    app.set_error_handler(ErrorCapture::handler());
    let groups: Vec<_> = (0..2)
        .map(|_| {
            app.world_mut()
                .spawn_scene(bsn! { @WidgetryRadioGroup })
                .unwrap()
                .id()
        })
        .collect();
    let parent = app.world_mut().spawn(Node::default()).id();
    let options: Vec<_> = (0..2)
        .map(|_| {
            app.world_mut()
                .spawn_scene(bsn! { @WidgetryRadioOption ChildOf(parent) })
                .unwrap()
                .id()
        })
        .collect();
    let healthy = app
        .world_mut()
        .spawn_scene(
            bsn! { @WidgetryRadioGroup Children [@WidgetryRadioOption-- @WidgetryRadioOption] },
        )
        .unwrap()
        .id();
    let healthy_first = app.world().get::<Children>(healthy).unwrap()[0];
    let errors = ErrorCapture::default();
    let logs = LogCapture::default();
    logs.run(|| {
        errors.run(|| {
            app.world_mut().run_schedule(PreUpdate);
            let records = errors.take_records();
            assert_eq!(records.len(), 1);
            let diagnostics: Vec<_> = records[0]
                .managed()
                .map(|managed| managed.diagnostics().collect())
                .unwrap_or_default();
            assert_eq!(diagnostics.len(), 4);
            assert_eq!(
                records[0].error.severity(),
                bevy::ecs::error::Severity::Error
            );
            let mut ids: Vec<_> = diagnostics
                .iter()
                .map(|diagnostic| {
                    match diagnostic
                        .source()
                        .unwrap()
                        .downcast_ref::<RadioGroupError>()
                        .unwrap()
                    {
                        RadioGroupError::EmptyGroup { group } => assert!(groups.contains(group)),
                        RadioGroupError::InvalidParent {
                            option,
                            parent: actual,
                        } => {
                            assert!(options.contains(option));
                            assert_eq!(*actual, Some(parent));
                        }
                        _ => panic!("wrong owner classification"),
                    }
                    assert_eq!(
                        logs.diagnostic_records(diagnostic.origin().id(), DiagnosticPhase::Source)
                            .len(),
                        1
                    );
                    diagnostic.origin().id().clone()
                })
                .collect();
            ids.sort_by_key(ToString::to_string);
            ids.dedup();
            assert_eq!(ids.len(), 4);
            assert!(app.world().get::<Checked>(healthy_first).is_some());
            app.world_mut().run_schedule(PreUpdate);
            assert!(errors.take_records().is_empty());
            for root in &groups {
                app.world_mut().entity_mut(*root).insert(WidgetryRadioGroup);
            }
            for option in &options {
                app.world_mut()
                    .entity_mut(*option)
                    .insert(WidgetryRadioOption);
            }
            app.world_mut().run_schedule(PreUpdate);
            let records = errors.take_records();
            let mut repeated: Vec<_> = records[0]
                .managed()
                .unwrap()
                .diagnostics()
                .map(|diagnostic| {
                    assert_eq!(
                        logs.diagnostic_records(diagnostic.origin().id(), DiagnosticPhase::Source)
                            .len(),
                        1
                    );
                    diagnostic.origin().id().clone()
                })
                .collect();
            repeated.sort_by_key(ToString::to_string);
            assert_eq!(ids, repeated);
            for &root in &groups {
                app.world_mut()
                    .spawn_scene(bsn! { @WidgetryRadioOption ChildOf(root) })
                    .unwrap();
            }
            for option in &options {
                app.world_mut().entity_mut(*option).insert(ChildOf(healthy));
            }
            app.world_mut().run_schedule(PreUpdate);
            assert!(errors.take_records().is_empty());
            assert!(app.world().get::<Checked>(healthy_first).is_some());
            assert_eq!(
                logs.records()
                    .iter()
                    .filter(|record| record.matches_field("phase", "recovered"))
                    .count(),
                4
            );
        })
    });
}

#[test]
fn style_batches_keep_each_option_identity_across_theme_and_repair_without_idle_polling() {
    let mut app = scene_app();
    app.add_plugins(WidgetryRadioGroupPlugin);
    app.set_error_handler(ErrorCapture::handler());
    app.edit_schedule(PostUpdate, |schedule| {
        schedule.set_executor(bevy::ecs::schedule::SingleThreadedExecutor::new());
    });
    let group = app.world_mut().spawn_scene(bsn! { @WidgetryRadioGroup Children [@WidgetryRadioOption-- @WidgetryRadioOption-- @WidgetryRadioOption] }).unwrap().id();
    app.update();
    app.update();
    let options = app.world().get::<Children>(group).unwrap().to_vec();
    let indicators: Vec<_> = options
        .iter()
        .map(|option| app.world().get::<Children>(*option).unwrap()[0])
        .collect();
    let healthy_before = *app.world().get::<BorderColor>(indicators[2]).unwrap();
    for indicator in &indicators[..2] {
        app.world_mut()
            .entity_mut(*indicator)
            .remove::<BorderColor>();
    }
    let logs = LogCapture::default();
    let errors = ErrorCapture::default();
    logs.run(|| errors.run(|| {
        app.update();
        let records = errors.take_records();
        assert_eq!(records.len(), 1);
        let diagnostics: Vec<_> = records[0].managed().unwrap().diagnostics().collect();
        assert_eq!(diagnostics.len(), 2);
        let mut ids: Vec<_> = diagnostics.iter().map(|diagnostic| {
            assert!(matches!(diagnostic.source().unwrap().downcast_ref::<RadioGroupError>(), Some(RadioGroupError::MissingPart { group: Some(target), option: Some(option), entity, part: "indicator.BorderColor" }) if *target == group && options[..2].contains(option) && indicators[..2].contains(entity)));
            diagnostic.origin().id().clone()
        }).collect();
        ids.sort_by_key(ToString::to_string);
        assert_eq!(*app.world().get::<BorderColor>(indicators[2]).unwrap(), healthy_before);
        app.update();
        assert!(errors.take_records().is_empty());
        bevy_widgetry_theme::WidgetryThemeMode::set_in_world(app.world_mut(), bevy_widgetry_theme::WidgetryThemeMode::Light).unwrap();
        let records = errors.take_records();
        assert_eq!(records.len(), 1);
        let mut repeated: Vec<_> = records[0].managed().unwrap().diagnostics().map(|diagnostic| {
            assert_eq!(logs.diagnostic_records(diagnostic.origin().id(), DiagnosticPhase::Source).len(), 1);
            diagnostic.origin().id().clone()
        }).collect();
        repeated.sort_by_key(ToString::to_string);
        assert_eq!(ids, repeated);
        for indicator in &indicators[..2] { app.world_mut().entity_mut(*indicator).insert(BorderColor::all(Color::NONE)); }
        app.update();
        assert!(errors.take_records().is_empty());
        assert_eq!(logs.records().iter().filter(|record| record.matches_field("phase", "recovered")).count(), 2);
    }));
}
