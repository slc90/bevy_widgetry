#![allow(clippy::disallowed_macros, clippy::unwrap_used)]

use bevy::{prelude::*, ui::InteractionDisabled, ui_widgets::Activate};
use bevy_widgetry_button::{
    ButtonError, WidgetryButtonHeadless, WidgetryButtonLoading, WidgetryButtonLoadingChanged,
    WidgetryButtonPlugin,
};
use bevy_widgetry_core::disabled::WidgetryEffectiveDisabled;
use bevy_widgetry_test_utils::{ErrorCapture, press, primary_click, release, scene_app};

#[derive(Resource, Default)]
struct Changes(Vec<bool>);
#[derive(Resource, Default)]
struct Activations(usize);

#[test]
fn optional_loading_has_no_visual_prerequisites_and_notifies_after_behavior_commit() {
    let mut app = scene_app();
    app.add_plugins(WidgetryButtonPlugin)
        .init_resource::<Changes>()
        .init_resource::<Activations>();
    app.add_observer(
        |event: On<WidgetryButtonLoadingChanged>, mut world: bevy::ecs::world::DeferredWorld| {
            assert_eq!(
                WidgetryButtonLoading::get(&world, event.entity)
                    .unwrap()
                    .is_active(),
                event.loading
            );
            assert_eq!(
                world
                    .get::<WidgetryEffectiveDisabled>(event.entity)
                    .unwrap()
                    .is_disabled(),
                event.loading
            );
            world.resource_mut::<Changes>().0.push(event.loading);
        },
    );
    app.add_observer(|_: On<Activate>, mut activations: ResMut<Activations>| activations.0 += 1);
    let plain = app
        .world_mut()
        .spawn_scene(bsn! { @WidgetryButtonHeadless })
        .unwrap()
        .id();
    assert!(matches!(
        WidgetryButtonLoading::set_in_world(app.world_mut(), plain, true)
            .unwrap_err()
            .kind(),
        ButtonError::MissingPart {
            part: "WidgetryButtonLoading",
            ..
        }
    ));
    let root = app
        .world_mut()
        .spawn_scene(bsn! { @WidgetryButtonHeadless WidgetryButtonLoading::default() })
        .unwrap()
        .id();
    assert!(app.world().get::<Children>(root).is_none());
    assert!(!WidgetryButtonLoading::set_in_world(app.world_mut(), root, false).unwrap());
    for active in [true, true, false, false] {
        WidgetryButtonLoading::set_in_world(app.world_mut(), root, active).unwrap();
        press(&mut app, root);
        app.world_mut().trigger(primary_click(root));
        release(&mut app, root);
    }
    assert_eq!(app.world().resource::<Changes>().0, [true, false]);
    assert_eq!(app.world().resource::<Activations>().0, 2);
    WidgetryButtonLoading::set(&mut app.world_mut().commands(), root, true);
    assert!(
        !WidgetryButtonLoading::get(app.world(), root)
            .unwrap()
            .is_active()
    );
    app.world_mut().flush();
    assert!(
        WidgetryButtonLoading::get(app.world(), root)
            .unwrap()
            .is_active()
    );
    assert!(app.world().get::<InteractionDisabled>(root).is_some());
    assert!(app.world().get::<Children>(root).is_none());
}

#[test]
fn missing_capabilities_reject_without_commit_and_commands_report_at_execution() {
    for remove_headless in [false, true] {
        let mut app = scene_app();
        app.add_plugins(WidgetryButtonPlugin);
        let root = app
            .world_mut()
            .spawn_scene(bsn! { @WidgetryButtonHeadless WidgetryButtonLoading::default() })
            .unwrap()
            .id();
        WidgetryButtonLoading::set_in_world(app.world_mut(), root, true).unwrap();
        if remove_headless {
            app.world_mut()
                .entity_mut(root)
                .remove::<WidgetryButtonHeadless>();
        } else {
            app.world_mut()
                .entity_mut(root)
                .remove::<WidgetryButtonLoading>();
        }
        let failure =
            WidgetryButtonLoading::set_in_world(app.world_mut(), root, false).unwrap_err();
        assert!(
            matches!(failure.kind(), ButtonError::MissingPart { root: target, .. } if *target == root)
        );
        assert!(app.world().get::<InteractionDisabled>(root).is_some());
        if remove_headless {
            assert!(
                app.world()
                    .get::<WidgetryButtonLoading>(root)
                    .unwrap()
                    .is_active()
            );
        }
        app.set_error_handler(ErrorCapture::handler());
        let errors = ErrorCapture::default();
        WidgetryButtonLoading::set(&mut app.world_mut().commands(), root, false);
        assert!(errors.take_records().is_empty());
        errors.run(|| app.world_mut().flush());
        let records = errors.take_records();
        assert_eq!(records.len(), 1);
        assert_eq!(
            records[0].error.severity(),
            bevy::ecs::error::Severity::Error
        );
    }
}

#[test]
fn loading_without_style_keeps_local_ancestor_and_other_intrinsic_reasons_independent() {
    let mut app = scene_app();
    app.add_plugins(WidgetryButtonPlugin);
    let parent = app
        .world_mut()
        .spawn_scene(
            bsn! { Node Children [@WidgetryButtonHeadless WidgetryButtonLoading::default()] },
        )
        .unwrap()
        .id();
    let root = app.world().get::<Children>(parent).unwrap()[0];
    app.world_mut()
        .entity_mut(parent)
        .insert(InteractionDisabled);
    app.world_mut().entity_mut(root).insert(InteractionDisabled);
    bevy_widgetry_core::disabled::set_intrinsic_disabled(app.world_mut(), root, true).unwrap();
    WidgetryButtonLoading::set_in_world(app.world_mut(), root, true).unwrap();
    WidgetryButtonLoading::set_in_world(app.world_mut(), root, false).unwrap();
    app.world_mut()
        .entity_mut(parent)
        .remove::<InteractionDisabled>();
    app.world_mut()
        .entity_mut(root)
        .remove::<InteractionDisabled>();
    app.world_mut().flush();
    assert!(
        app.world()
            .get::<WidgetryEffectiveDisabled>(root)
            .unwrap()
            .is_disabled()
    );
    WidgetryButtonLoading::set_in_world(app.world_mut(), root, true).unwrap();
    bevy_widgetry_core::disabled::set_intrinsic_disabled(app.world_mut(), root, false).unwrap();
    assert!(
        app.world()
            .get::<WidgetryEffectiveDisabled>(root)
            .unwrap()
            .is_disabled()
    );
    WidgetryButtonLoading::set_in_world(app.world_mut(), root, false).unwrap();
    assert!(
        !app.world()
            .get::<WidgetryEffectiveDisabled>(root)
            .unwrap()
            .is_disabled()
    );
    assert!(app.world().get::<Children>(root).is_none());
}
