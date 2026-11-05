#![allow(clippy::disallowed_macros)]

use super::*;
use crate::TrayMenuEntry;
use bevy_widgetry_test_utils::{DiagnosticPhase, ErrorCapture, LogCapture};

fn app() -> App {
    let icon = TrayIconBuilder::new()
        .with_icon(Icon::from_rgba(vec![255; 16 * 16 * 4], 16, 16).unwrap())
        .with_menu_on_left_click(false)
        .build()
        .unwrap();
    let (bridge, inbox) = Bridge::new(icon.id().clone(), || Ok(()));
    let registration = Registration::new(icon.id().clone(), &bridge);
    let mut app = App::new();
    app.add_message::<WidgetryTrayClick>()
        .add_message::<WidgetryTrayMenuClick>()
        .add_message::<AppExit>()
        .insert_resource(inbox)
        .insert_non_send(TrayOwner(Some(TrayInstance {
            _registration: registration,
            menu_registration: None,
            session: None,
            icon,
            menu: None,
            bridge,
            applied: None,
            failed_request: None,
            last_sync_error: None,
        })))
        .add_systems(PreUpdate, bridge::forward_clicks)
        .add_systems(
            Last,
            (
                synchronize_menu.in_set(WidgetryTraySystems::Sync),
                release_on_exit.after(WidgetryTraySystems::Sync),
            ),
        );
    app
}

fn assert_applied(app: &App) {
    let model = app
        .world()
        .get_resource::<WidgetryTrayMenu>()
        .filter(|menu| !menu.entries().is_empty());
    let owner = app.world().non_send::<TrayOwner>();
    let instance = owner.0.as_ref().unwrap();
    assert_eq!(instance.applied.as_ref(), model);
    if let (Some(native), Some(model)) = (&instance.menu, model) {
        crate::native::verify_menu(&native.root, model.entries()).unwrap();
    } else {
        assert!(instance.menu.is_none());
    }
}

fn activate(app: &App, path: &[u32]) {
    let owner = app.world().non_send::<TrayOwner>();
    let instance = owner.0.as_ref().unwrap();
    crate::native::activate_item(&instance.icon, &instance.menu.as_ref().unwrap().root, path);
}

fn native_id(app: &App) -> tray_icon::menu::MenuId {
    app.world()
        .non_send::<TrayOwner>()
        .0
        .as_ref()
        .unwrap()
        .menu
        .as_ref()
        .unwrap()
        .routes[0]
        .0
        .clone()
}

#[test]
fn attaches_updates_reorders_and_removes_without_recreating_the_tray_icon() {
    let mut app = app();
    let icon_id = app
        .world()
        .non_send::<TrayOwner>()
        .0
        .as_ref()
        .unwrap()
        .icon
        .id()
        .clone();
    app.update();
    assert_applied(&app);
    app.insert_resource(
        WidgetryTrayMenu::new(vec![
            TrayMenuEntry::item("open", "Open"),
            TrayMenuEntry::submenu("More", vec![TrayMenuEntry::check("check", "Check", false)]),
        ])
        .unwrap(),
    );
    app.update();
    assert_applied(&app);
    let first = native_id(&app);
    app.update();
    assert_eq!(native_id(&app), first);
    activate(&app, &[0]);
    app.update();
    assert_eq!(native_id(&app), first);
    {
        let mut menu = app.world_mut().resource_mut::<WidgetryTrayMenu>();
        menu.set_text("open", "Intermediate").unwrap();
        menu.set_text("open", "Restore").unwrap();
        menu.set_enabled("open", false).unwrap();
        menu.set_checked("check", true).unwrap();
        menu.edit(|entries| {
            entries.insert(1, TrayMenuEntry::Separator);
            if let TrayMenuEntry::Submenu { entries, .. } = &mut entries[2] {
                entries.insert(0, TrayMenuEntry::item("new", "New"));
            }
            entries.swap(0, 2);
        })
        .unwrap();
    }
    app.update();
    assert_applied(&app);
    assert_ne!(native_id(&app), first);
    assert_eq!(
        app.world()
            .non_send::<TrayOwner>()
            .0
            .as_ref()
            .unwrap()
            .icon
            .id(),
        &icon_id
    );
    app.world_mut()
        .resource_mut::<WidgetryTrayMenu>()
        .edit(|entries| entries.clear())
        .unwrap();
    app.update();
    assert_applied(&app);
    app.world_mut()
        .resource_mut::<WidgetryTrayMenu>()
        .replace(vec![TrayMenuEntry::item("again", "Again")])
        .unwrap();
    app.update();
    assert_applied(&app);
    app.world_mut().remove_resource::<WidgetryTrayMenu>();
    app.update();
    assert_applied(&app);
    assert_eq!(
        app.world()
            .non_send::<TrayOwner>()
            .0
            .as_ref()
            .unwrap()
            .icon
            .id(),
        &icon_id
    );
    app.world_mut().write_message(AppExit::Success);
    app.update();
    assert!(app.world().non_send::<TrayOwner>().0.is_none());
}

#[test]
fn queued_activations_cannot_execute_items_deleted_or_disabled_in_the_desired_tree() {
    let mut app = app();
    app.insert_resource(
        WidgetryTrayMenu::new(vec![
            TrayMenuEntry::item("remove", "Remove"),
            TrayMenuEntry::submenu("Nested", vec![TrayMenuEntry::item("nested", "Nested")]),
        ])
        .unwrap(),
    );
    app.update();
    let mut cursor = app
        .world()
        .resource::<Messages<WidgetryTrayMenuClick>>()
        .get_cursor();
    activate(&app, &[0]);
    app.world_mut()
        .resource_mut::<WidgetryTrayMenu>()
        .edit(|entries| {
            entries.remove(0);
        })
        .unwrap();
    app.update();
    assert_eq!(
        cursor
            .read(app.world().resource::<Messages<WidgetryTrayMenuClick>>())
            .count(),
        0
    );
    activate(&app, &[0, 0]);
    app.world_mut()
        .resource_mut::<WidgetryTrayMenu>()
        .edit(|entries| {
            if let TrayMenuEntry::Submenu { enabled, .. } = &mut entries[0] {
                *enabled = false;
            }
        })
        .unwrap();
    app.update();
    assert_eq!(
        cursor
            .read(app.world().resource::<Messages<WidgetryTrayMenuClick>>())
            .count(),
        0
    );
    assert_applied(&app);
}

#[derive(Resource)]
struct Accept(bool);

fn host(
    mut events: MessageReader<WidgetryTrayMenuClick>,
    decision: Res<Accept>,
    mut menu: ResMut<WidgetryTrayMenu>,
) -> Result {
    for event in events.read() {
        if event.id == "check" && decision.0 {
            menu.set_checked("check", true)
                .map_err(TrayFailure::into_bevy_error)?;
        }
    }
    Ok(())
}

#[test]
fn check_activation_converges_to_host_acceptance_rejection_or_no_resource_write() {
    for decision in [None, Some(false), Some(true)] {
        let mut app = app();
        app.insert_resource(
            WidgetryTrayMenu::new(vec![TrayMenuEntry::check("check", "Check", false)]).unwrap(),
        );
        if let Some(accepted) = decision {
            app.insert_resource(Accept(accepted))
                .add_systems(Update, host);
        }
        app.update();
        let mut cursor = app
            .world()
            .resource::<Messages<WidgetryTrayMenuClick>>()
            .get_cursor();
        assert!(!app.world().is_resource_changed::<WidgetryTrayMenu>());
        activate(&app, &[0]);
        {
            let owner = app.world().non_send::<TrayOwner>();
            assert!(
                owner
                    .0
                    .as_ref()
                    .unwrap()
                    .menu
                    .as_ref()
                    .unwrap()
                    .root
                    .items()[0]
                    .as_check_menuitem()
                    .unwrap()
                    .is_checked()
            );
        }
        app.update();
        assert_applied(&app);
        assert!(
            matches!(app.world().resource::<WidgetryTrayMenu>().entry("check"), Some(TrayMenuEntry::Check { checked, .. }) if *checked == decision.unwrap_or(false))
        );
        assert_eq!(
            cursor
                .read(app.world().resource::<Messages<WidgetryTrayMenuClick>>())
                .map(|event| event.id.as_str())
                .collect::<Vec<_>>(),
            ["check"]
        );
        app.update();
        assert_eq!(
            cursor
                .read(app.world().resource::<Messages<WidgetryTrayMenuClick>>())
                .count(),
            0
        );
    }
}

#[test]
fn failed_projection_retains_desired_and_applied_states_then_recovers_on_new_request() {
    let mut app = app();
    app.insert_resource(WidgetryTrayMenu::new(vec![TrayMenuEntry::item("open", "Open")]).unwrap());
    app.set_error_handler(ErrorCapture::handler());
    app.update();
    let original = native_id(&app);
    let errors = ErrorCapture::default();
    let logs = LogCapture::default();
    app.world_mut()
        .resource_mut::<WidgetryTrayMenu>()
        .set_text("open", "invalid\0label")
        .unwrap();
    errors.run(|| logs.run(|| app.update()));
    let records = errors.take_records();
    assert_eq!(records.len(), 1);
    assert_eq!(
        records[0].error.severity(),
        bevy::ecs::error::Severity::Error
    );
    let origin = records[0]
        .managed()
        .unwrap()
        .diagnostics()
        .next()
        .unwrap()
        .origin()
        .clone();
    assert_eq!(native_id(&app), original);
    assert!(
        matches!(app.world().resource::<WidgetryTrayMenu>().entry("open"), Some(TrayMenuEntry::Item { text, .. }) if text.contains('\0'))
    );
    errors.run(|| logs.run(|| app.update()));
    assert!(errors.take_records().is_empty());
    assert_eq!(
        logs.diagnostic_records(origin.id(), DiagnosticPhase::Source)
            .len(),
        1
    );
    assert!(
        logs.diagnostic_records(origin.id(), DiagnosticPhase::Handled)
            .is_empty()
    );
    app.world_mut()
        .resource_mut::<WidgetryTrayMenu>()
        .set_text("open", "Recovered")
        .unwrap();
    errors.run(|| logs.run(|| app.update()));
    assert!(errors.take_records().is_empty());
    assert_applied(&app);
    assert_eq!(
        logs.diagnostic_records(origin.id(), DiagnosticPhase::Handled)
            .len(),
        1
    );
    assert!(
        app.world()
            .non_send::<TrayOwner>()
            .0
            .as_ref()
            .unwrap()
            .failed_request
            .is_none()
    );
}
