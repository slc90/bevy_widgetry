#![allow(clippy::disallowed_macros, clippy::unwrap_used)]

use bevy::prelude::*;
use bevy::ui::Checked;
use bevy_widgetry_segmented::{
    SegmentedError, WidgetrySegmented, WidgetrySegmentedOption, WidgetrySegmentedPlugin,
};
use bevy_widgetry_test_utils::{DiagnosticPhase, ErrorCapture, LogCapture, scene_app};

#[test]
fn every_invalid_group_and_option_reaches_the_host_without_polling_or_recreating_the_source() {
    let mut app = scene_app();
    app.add_plugins(WidgetrySegmentedPlugin);
    app.set_error_handler(ErrorCapture::handler());
    let groups: Vec<_> = (0..2)
        .map(|_| {
            app.world_mut()
                .spawn_scene(bsn! { @WidgetrySegmented })
                .unwrap()
                .id()
        })
        .collect();
    let parent = app.world_mut().spawn(Node::default()).id();
    let options: Vec<_> = (0..2)
        .map(|_| {
            app.world_mut()
                .spawn_scene(bsn! { @WidgetrySegmentedOption ChildOf(parent) })
                .unwrap()
                .id()
        })
        .collect();
    let healthy = app
        .world_mut()
        .spawn_scene(
            bsn! { @WidgetrySegmented Children [@WidgetrySegmentedOption-- @WidgetrySegmentedOption] },
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
                        .downcast_ref::<SegmentedError>()
                        .unwrap()
                    {
                        SegmentedError::EmptyGroup { group } => assert!(groups.contains(group)),
                        SegmentedError::InvalidParent {
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
                app.world_mut().entity_mut(*root).insert(WidgetrySegmented);
            }
            for option in &options {
                app.world_mut()
                    .entity_mut(*option)
                    .insert(WidgetrySegmentedOption);
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
                    .spawn_scene(bsn! { @WidgetrySegmentedOption ChildOf(root) })
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
    app.add_plugins(WidgetrySegmentedPlugin);
    app.set_error_handler(ErrorCapture::handler());
    app.edit_schedule(PostUpdate, |schedule| {
        schedule.set_executor(bevy::ecs::schedule::SingleThreadedExecutor::new());
    });
    let group = app.world_mut().spawn_scene(bsn! { @WidgetrySegmented Children [@WidgetrySegmentedOption-- @WidgetrySegmentedOption-- @WidgetrySegmentedOption] }).unwrap().id();
    app.update();
    app.update();
    let options = app.world().get::<Children>(group).unwrap().to_vec();
    let healthy_before = *app.world().get::<BackgroundColor>(options[2]).unwrap();
    for option in &options[..2] {
        app.world_mut()
            .entity_mut(*option)
            .remove::<BackgroundColor>();
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
            assert!(matches!(diagnostic.source().unwrap().downcast_ref::<SegmentedError>(), Some(SegmentedError::MissingPart { group: Some(target), option: Some(option), entity, part: "BackgroundColor" }) if *target == group && options[..2].contains(option) && options[..2].contains(entity)));
            diagnostic.origin().id().clone()
        }).collect();
        ids.sort_by_key(ToString::to_string);
        assert_eq!(*app.world().get::<BackgroundColor>(options[2]).unwrap(), healthy_before);
        app.update();
        assert!(errors.take_records().is_empty());
        bevy_widgetry_theme::WidgetryTheme::set_in_world(app.world_mut(), bevy_widgetry_theme::WIDGETRY_LIGHT_THEME).unwrap();
        app.update();
        let records = errors.take_records();
        assert_eq!(records.len(), 1);
        let mut repeated: Vec<_> = records[0].managed().unwrap().diagnostics().map(|diagnostic| {
            assert_eq!(logs.diagnostic_records(diagnostic.origin().id(), DiagnosticPhase::Source).len(), 1);
            diagnostic.origin().id().clone()
        }).collect();
        repeated.sort_by_key(ToString::to_string);
        assert_eq!(ids, repeated);
        for option in &options[..2] { app.world_mut().entity_mut(*option).insert(BackgroundColor::default()); }
        app.update();
        assert!(errors.take_records().is_empty());
        assert_eq!(logs.records().iter().filter(|record| record.matches_field("phase", "recovered")).count(), 2);
    }));
}

#[test]
fn independent_commands_keep_requested_index_and_each_new_identity() {
    let mut app = scene_app();
    app.set_error_handler(ErrorCapture::handler());
    let root = app
        .world_mut()
        .spawn_scene(bsn! { @WidgetrySegmented Children [@WidgetrySegmentedOption] })
        .unwrap()
        .id();
    let errors = ErrorCapture::default();
    let logs = LogCapture::default();
    logs.run(|| {
        errors.run(|| {
            for _ in 0..2 {
                WidgetrySegmented::set_selected(&mut app.world_mut().commands(), root, 7);
            }
            app.world_mut().flush();
        })
    });
    let records = errors.take_records();
    assert_eq!(records.len(), 2);
    let diagnostics: Vec<_> = records
        .iter()
        .map(|record| {
            assert_eq!(record.error.severity(), bevy::ecs::error::Severity::Error);
            record.managed().unwrap().diagnostics().next().unwrap()
        })
        .collect();
    for diagnostic in &diagnostics {
        assert!(
            matches!(diagnostic.source().unwrap().downcast_ref::<SegmentedError>(), Some(SegmentedError::IndexOutOfBounds { group, index: 7, len: 1 }) if *group == root)
        );
        assert_eq!(
            logs.diagnostic_records(diagnostic.origin().id(), DiagnosticPhase::Source)
                .len(),
            1
        );
    }
    assert_ne!(diagnostics[0].origin().id(), diagnostics[1].origin().id());
}

#[test]
fn unchanged_initial_checked_conflict_reuses_origin_until_recovery() {
    let mut app = scene_app();
    app.add_plugins(WidgetrySegmentedPlugin);
    app.set_error_handler(ErrorCapture::handler());
    let group = app.world_mut().spawn_scene(bsn! { @WidgetrySegmented Children [@WidgetrySegmentedOption Checked-- @WidgetrySegmentedOption Checked] }).unwrap().id();
    let options = app.world().get::<Children>(group).unwrap().to_vec();
    let logs = LogCapture::default();
    let errors = ErrorCapture::default();
    logs.run(|| errors.run(|| {
        app.world_mut().run_schedule(PreUpdate);
        let first = errors.take_records(); let first = first[0].managed().unwrap().diagnostics().next().unwrap();
        assert!(matches!(first.source().unwrap().downcast_ref::<SegmentedError>(), Some(SegmentedError::InitialCheckedConflict { group: root, options: checked }) if *root == group && *checked == options));
        app.world_mut().entity_mut(options[0]).insert(Checked);
        app.world_mut().run_schedule(PreUpdate);
        let second = errors.take_records(); let second = second[0].managed().unwrap().diagnostics().next().unwrap();
        assert!(std::sync::Arc::ptr_eq(first.origin(), second.origin()));
        assert_eq!(logs.diagnostic_records(first.origin().id(), DiagnosticPhase::Source).len(), 1);
        app.world_mut().entity_mut(options[0]).remove::<Checked>();
        app.world_mut().run_schedule(PreUpdate); assert!(errors.take_records().is_empty());
        assert!(app.world().get::<Checked>(options[1]).is_some());
        assert_eq!(logs.records().iter().filter(|record| record.matches_field("phase", "recovered")).count(), 1);
    }));
}
