#![allow(clippy::disallowed_macros)]

use super::*;
use bevy::window::{PrimaryWindow, RequestRedraw, WindowCloseRequested};
use bevy_widgetry::tray::WidgetryTrayClick;

fn app() -> App {
    let mut app = App::new();
    app.init_resource::<GalleryTrayState>()
        .init_resource::<Language>()
        .add_message::<WidgetryTrayClick>()
        .add_message::<WidgetryTrayMenuClick>()
        .add_message::<RequestRedraw>()
        .add_systems(Update, restore_from_tray);
    install(&mut app).unwrap();
    app
}

fn command(app: &mut App, id: &str) {
    app.world_mut()
        .write_message(WidgetryTrayMenuClick { id: id.into() });
}

#[test]
fn menu_restore_reuses_hidden_session_and_preserves_minimize_requests() {
    for use_menu in [false, true] {
        let mut app = app();
        let primary = app
            .world_mut()
            .spawn((
                Window {
                    visible: false,
                    ..default()
                },
                PrimaryWindow,
            ))
            .id();
        let prehidden = app
            .world_mut()
            .spawn(Window {
                visible: false,
                ..default()
            })
            .id();
        let mut minimized = Window {
            visible: false,
            ..default()
        };
        minimized.set_minimized(true);
        let minimized = app.world_mut().spawn(minimized).id();
        let stale = app.world_mut().spawn_empty().id();
        app.world_mut().despawn(stale);
        app.world_mut()
            .resource_mut::<GalleryTrayState>()
            .hidden_windows = vec![primary, minimized, stale];
        if use_menu {
            command(&mut app, RESTORE);
        } else {
            app.world_mut().write_message(WidgetryTrayClick);
        }
        app.update();
        assert!(app.world().get::<Window>(primary).unwrap().visible);
        assert!(!app.world().get::<Window>(prehidden).unwrap().visible);
        assert!(app.world().get::<Window>(minimized).unwrap().visible);
        assert_eq!(
            app.world_mut()
                .get_mut::<Window>(minimized)
                .unwrap()
                .internal
                .take_minimize_request(),
            Some(true)
        );
        assert!(
            app.world()
                .resource::<GalleryTrayState>()
                .hidden_windows
                .is_empty()
        );
        assert_eq!(
            app.world()
                .resource::<GalleryTrayState>()
                .focus_after_restore,
            Some(primary)
        );
        app.world_mut().get_mut::<Window>(primary).unwrap().visible = false;
        if use_menu {
            command(&mut app, RESTORE);
        } else {
            app.world_mut().write_message(WidgetryTrayClick);
        }
        app.update();
        assert!(!app.world().get::<Window>(primary).unwrap().visible);
    }
}

#[test]
fn demo_actions_and_language_changes_preserve_stable_ids_and_host_state() {
    let mut app = app();
    let menu = app.world().resource::<WidgetryTrayMenu>();
    assert!(matches!(
        menu.entry(CHECK),
        Some(TrayMenuEntry::Check { checked: true, .. })
    ));
    assert!(matches!(
        menu.entry(DISABLED),
        Some(TrayMenuEntry::Item { enabled: false, .. })
    ));
    assert!(menu.entry(DYNAMIC).is_none());
    for id in [CHECK, TOGGLE_ENABLED, RENAME, ADD, ADD] {
        command(&mut app, id);
    }
    app.update();
    {
        let menu = app.world().resource::<WidgetryTrayMenu>();
        assert!(matches!(
            menu.entry(CHECK),
            Some(TrayMenuEntry::Check { checked: false, .. })
        ));
        assert!(matches!(
            menu.entry(DISABLED),
            Some(TrayMenuEntry::Item { enabled: true, .. })
        ));
        let Some(TrayMenuEntry::Submenu { entries, .. }) = menu.entries().get(2) else {
            panic!("demo submenu missing");
        };
        assert_eq!(
            entries
                .iter()
                .filter(|entry| matches!(entry, TrayMenuEntry::Item { id, .. } if id == DYNAMIC))
                .count(),
            1
        );
    }
    for language in [Language::SimplifiedChinese, Language::English] {
        *app.world_mut().resource_mut::<Language>() = language;
        app.update();
        let menu = app.world().resource::<WidgetryTrayMenu>();
        assert!(
            matches!(menu.entry(RENAME), Some(TrayMenuEntry::Item { text, .. }) if text == label(language, TrayKey::Renamed))
        );
        assert!(
            matches!(menu.entry(DYNAMIC), Some(TrayMenuEntry::Item { text, .. }) if text == label(language, TrayKey::Dynamic))
        );
        assert!(matches!(
            menu.entry(CHECK),
            Some(TrayMenuEntry::Check { checked: false, .. })
        ));
        assert!(matches!(
            menu.entry(DISABLED),
            Some(TrayMenuEntry::Item { enabled: true, .. })
        ));
    }
    for id in [
        REMOVE,
        REMOVE,
        ADD,
        DYNAMIC,
        CHECK,
        TOGGLE_ENABLED,
        RENAME,
        REMOVE,
    ] {
        command(&mut app, id);
    }
    app.update();
    let menu = app.world().resource::<WidgetryTrayMenu>();
    assert!(menu.entry(DYNAMIC).is_none());
    assert!(matches!(
        menu.entry(CHECK),
        Some(TrayMenuEntry::Check { checked: true, .. })
    ));
    assert!(matches!(
        menu.entry(DISABLED),
        Some(TrayMenuEntry::Item { enabled: false, .. })
    ));
    assert!(
        matches!(menu.entry(RENAME), Some(TrayMenuEntry::Item { text, .. }) if text == label(Language::English, TrayKey::Rename))
    );
}

#[test]
fn menu_exit_starts_normal_shutdown_once_without_opening_close_confirmation() {
    use bevy_widgetry::job::JobManagerPlugin;
    use bevy_widgetry_test_utils::LogCapture;
    for existing_dialog in [false, true] {
        let mut app = app();
        app.add_plugins((MinimalPlugins, JobManagerPlugin))
            .add_message::<WindowCloseRequested>()
            .add_systems(Update, close_requests);
        super::super::shutdown::install(&mut app);
        let primary = app
            .world_mut()
            .spawn((
                Window {
                    visible: false,
                    ..default()
                },
                PrimaryWindow,
            ))
            .id();
        app.world_mut()
            .resource_mut::<GalleryTrayState>()
            .hidden_windows
            .push(primary);
        let original_dialog =
            existing_dialog.then(|| app.world_mut().spawn(super::super::CloseDialog).id());
        app.world_mut().resource_mut::<GalleryTrayState>().dialog = original_dialog;
        command(&mut app, EXIT);
        command(&mut app, EXIT);
        app.world_mut()
            .write_message(WindowCloseRequested { window: primary });
        let logs = LogCapture::with_targets(["widget_gallery::close::shutdown"]);
        logs.run(|| app.update());
        assert!(app.world().resource::<GalleryTrayState>().exiting);
        assert!(
            app.world()
                .resource::<GalleryTrayState>()
                .dialog
                .is_none_or(|dialog| Some(dialog) == original_dialog)
        );
        assert!(
            app.world_mut()
                .query_filtered::<Entity, With<super::super::CloseDialog>>()
                .iter(app.world())
                .all(|dialog| Some(dialog) == original_dialog)
        );
        assert_eq!(
            logs.records()
                .iter()
                .filter(|record| {
                    record.fields.get("message").is_some_and(|message| {
                        message.contains("Gallery 开始等待全部后台任务真实退出")
                    })
                })
                .count(),
            1
        );
        for _ in 0..8 {
            if app.should_exit().is_some() {
                break;
            }
            app.update();
        }
        assert_eq!(app.should_exit(), Some(AppExit::Success));
    }
}
