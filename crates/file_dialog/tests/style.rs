//! Coverage Map：contract.rs 负责业务 state，runtime.rs 负责后台，本文件负责真实 BSN 与 UI adapter。
//! state 维度为 shell/session、viewport/visible range、selection/active、editor focus 与 theme。
//! stimuli 为构造、reply、scroll、pointer、focused keyboard、EditableText edit 与 ThemeChanged。
//! guards 为 disabled、IME composition、press identity/token 与 bounded viewport。
//! invariants 为 Props 只初始化一次、独立实例、业务 authority 先提交、可见 rows 有界且保留 editor cursor。

// 测试断言保护 UI contract，不适用生产 macro 禁令。
#![allow(clippy::disallowed_macros, clippy::unwrap_used)]

use bevy::input_focus::{FocusCause, InputFocus};
use bevy::prelude::*;
use bevy::text::EditableText;
use bevy_widgetry_core::ThemeMode;
use bevy_widgetry_file_dialog::*;
use bevy_widgetry_test_utils::{ErrorCapture, LogCapture};
use bevy_widgetry_test_utils::{add_ui_plugins, scene_app, spawn_ui_camera};
use bevy_widgetry_test_utils::{press, primary_click, release};
use bevy_widgetry_test_utils::{press_key, switch_theme};
use std::path::PathBuf;
use std::sync::Arc;

fn loaded(app: &mut App, root: Entity, count: usize) {
    loaded_kind(app, root, count, WidgetryFileDialogEntryKind::File);
}

fn loaded_kind(app: &mut App, root: Entity, count: usize, kind: WidgetryFileDialogEntryKind) {
    let state = app.world().get::<WidgetryFileDialogState>(root).unwrap();
    let token = state.token();
    let path = PathBuf::from("C:/fixture");
    let data = (0..count)
        .map(|index| {
            let name = format!("file_{index:06}.txt");
            WidgetryFileDialogEntryData {
                path: path.join(&name),
                name: name.into(),
                kind,
                size: Some(index as u64),
                modified: None,
                hidden: Some(false),
                system: Some(false),
            }
        })
        .collect();
    let snapshot = Arc::new(
        WidgetryFileDialogSnapshot::prepare(token, path.clone(), data, &state.query(), None)
            .unwrap(),
    );
    WidgetryFileDialog::deliver(
        app.world_mut(),
        root,
        WidgetryFileDialogReply::Started { token, path },
    )
    .unwrap();
    let selection = app
        .world()
        .get::<WidgetryFileDialogState>(root)
        .unwrap()
        .projection_selection_job(snapshot.clone())
        .unwrap()
        .prepare()
        .unwrap();
    WidgetryFileDialog::deliver(
        app.world_mut(),
        root,
        WidgetryFileDialogReply::Snapshot {
            snapshot,
            state: WidgetryFileDialogDirectoryState::Ready,
            selection,
        },
    )
    .unwrap();
}

fn fixture() -> App {
    let mut app = scene_app();
    add_ui_plugins(&mut app);
    spawn_ui_camera(&mut app, UVec2::new(1000, 700), 1.0);
    app.world_mut()
        .spawn((Window::default(), bevy::window::PrimaryWindow));
    app.insert_resource(WidgetryFileDialogRuntimeOptions {
        automatic: false,
        ..default()
    });
    app.add_plugins(WidgetryFileDialogPlugin);
    app.add_plugins(bevy::input_focus::InputDispatchPlugin);
    app
}

fn finish_selection(app: &mut App, root: Entity) {
    if let Some(job) = app
        .world()
        .get::<WidgetryFileDialogState>(root)
        .unwrap()
        .selection_job()
    {
        let reply = job.prepare().unwrap();
        WidgetryFileDialog::deliver(
            app.world_mut(),
            root,
            WidgetryFileDialogReply::Selection(reply),
        )
        .unwrap();
    }
    app.update();
}

#[test]
fn single_modes_space_and_modified_click_use_legal_selection_operations() {
    for (mode, kind) in [
        (
            WidgetryFileDialogMode::PickFile,
            WidgetryFileDialogEntryKind::File,
        ),
        (
            WidgetryFileDialogMode::PickDirectory,
            WidgetryFileDialogEntryKind::Directory,
        ),
        (
            WidgetryFileDialogMode::SaveFile,
            WidgetryFileDialogEntryKind::File,
        ),
    ] {
        let mut app = fixture();
        app.set_error_handler(ErrorCapture::handler());
        let errors = ErrorCapture::default();
        let root = app
            .world_mut()
            .spawn_scene(
                bsn! { @WidgetryFileDialog { @mode: mode } Node {width: px(800), height: px(600)} },
            )
            .unwrap()
            .id();
        loaded_kind(&mut app, root, 20, kind);
        for _ in 0..3 {
            app.update();
        }
        let area = named(&mut app, root, "FileDialogEntries");
        app.world_mut()
            .resource_mut::<InputFocus>()
            .set(area, FocusCause::Navigated);
        errors.run(|| press_key(&mut app, Entity::PLACEHOLDER, KeyCode::ArrowDown));
        finish_selection(&mut app, root);
        errors.run(|| press_key(&mut app, Entity::PLACEHOLDER, KeyCode::Space));
        assert!(
            errors.take().is_empty(),
            "single mode Space must be a legal selection"
        );
        finish_selection(&mut app, root);
        assert_eq!(
            app.world()
                .get::<WidgetryFileDialogState>(root)
                .unwrap()
                .selected()
                .len(),
            usize::from(mode.accepts(kind))
        );
        let row = named(&mut app, root, "FileDialogEntryRow");
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(KeyCode::ControlLeft);
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(KeyCode::ShiftLeft);
        errors.run(|| {
            press(&mut app, row);
            release(&mut app, row);
            app.world_mut().trigger(primary_click(row));
            app.world_mut().flush();
        });
        assert!(
            errors.take().is_empty(),
            "single mode modified click must use Replace"
        );
        finish_selection(&mut app, root);
        errors.run(|| press_key(&mut app, Entity::PLACEHOLDER, KeyCode::KeyA));
        assert!(
            errors.take().is_empty(),
            "single mode Ctrl+A must be ignored"
        );
    }
    let mut app = fixture();
    app.set_error_handler(ErrorCapture::handler());
    let errors = ErrorCapture::default();
    let root = app
        .world_mut()
        .spawn_scene(bsn! { @WidgetryFileDialog Node {width: px(800), height: px(600)} })
        .unwrap()
        .id();
    loaded_kind(&mut app, root, 20, WidgetryFileDialogEntryKind::Directory);
    for _ in 0..3 {
        app.update();
    }
    let area = named(&mut app, root, "FileDialogEntries");
    app.world_mut()
        .resource_mut::<InputFocus>()
        .set(area, FocusCause::Navigated);
    press_key(&mut app, Entity::PLACEHOLDER, KeyCode::ArrowDown);
    finish_selection(&mut app, root);
    errors.run(|| press_key(&mut app, Entity::PLACEHOLDER, KeyCode::Space));
    assert!(
        errors.take().is_empty(),
        "Space on an incompatible entry must be ignored"
    );
}

#[test]
fn invalid_folder_enter_does_not_discard_following_escape_in_the_same_update() {
    let mut app = fixture();
    app.set_error_handler(ErrorCapture::handler());
    let errors = ErrorCapture::default();
    let root = app
        .world_mut()
        .spawn_scene(bsn! { @WidgetryFileDialog Node {width: px(800), height: px(600)} })
        .unwrap()
        .id();
    loaded(&mut app, root, 10);
    for _ in 0..3 {
        app.update();
    }
    let open = named(&mut app, root, "FileDialogNewFolder");
    app.world_mut()
        .trigger(bevy::ui_widgets::Activate { entity: open });
    app.world_mut().flush();
    for key_code in [KeyCode::Enter, KeyCode::Escape] {
        bevy_widgetry_test_utils::queue_key(
            &mut app,
            bevy::input::keyboard::KeyboardInput {
                key_code,
                logical_key: bevy::input::keyboard::Key::Unidentified(
                    bevy::input::keyboard::NativeKey::Unidentified,
                ),
                state: bevy::input::ButtonState::Pressed,
                text: None,
                repeat: false,
                window: Entity::PLACEHOLDER,
            },
        );
    }
    errors.run(|| app.update());
    assert_eq!(errors.take().len(), 1);
    let panel = named(&mut app, root, "FileDialogFolderPanel");
    assert_eq!(
        app.world().get::<Node>(panel).unwrap().display,
        Display::None
    );
    assert!(
        app.world()
            .get::<WidgetryFileDialogState>(root)
            .unwrap()
            .result()
            .is_none()
    );
}

#[test]
fn tab_focus_opens_filter_and_sort_popups_and_disabled_fields_are_skipped() {
    for label in ["FileDialogFilter", "FileDialogSort"] {
        let mut app = fixture();
        let root = app.world_mut().spawn_scene(bsn! { @WidgetryFileDialog Node {width: px(800), height: px(600)} bevy::input_focus::tab_navigation::TabGroup::new(0) }).unwrap().id();
        for _ in 0..4 {
            app.update();
        }
        let combo = named(&mut app, root, label);
        let field = app
            .world()
            .get::<Children>(combo)
            .unwrap()
            .iter()
            .next()
            .unwrap();
        let path = named(&mut app, root, "FileDialogPath");
        app.world_mut()
            .resource_mut::<InputFocus>()
            .set(path, FocusCause::Navigated);
        let mut reached = false;
        for _ in 0..40 {
            press_key(&mut app, Entity::PLACEHOLDER, KeyCode::Tab);
            if app.world().resource::<InputFocus>().get() == Some(field) {
                reached = true;
                break;
            }
        }
        assert!(reached, "Tab must reach the activatable ComboBox field");
        press_key(&mut app, Entity::PLACEHOLDER, KeyCode::Enter);
        for _ in 0..3 {
            app.update();
        }
        let popup = app
            .world()
            .get::<Children>(combo)
            .unwrap()
            .iter()
            .nth(1)
            .unwrap();
        assert_ne!(
            app.world().get::<Visibility>(popup),
            Some(&Visibility::Hidden)
        );
        app.world_mut()
            .entity_mut(root)
            .insert(bevy::ui::InteractionDisabled);
        for _ in 0..3 {
            app.update();
        }
        assert!(
            app.world()
                .get::<bevy::ui::InteractionDisabled>(field)
                .is_some()
        );
        assert_eq!(
            app.world()
                .get::<bevy::input_focus::tab_navigation::TabIndex>(field)
                .unwrap()
                .0,
            -1
        );
    }
}

#[test]
fn sidebar_locations_and_pins_follow_disabled_and_resolved_tab_state() {
    let mut app = fixture();
    let root = app
        .world_mut()
        .spawn_scene(bsn! { @WidgetryFileDialog Node {width: px(800), height: px(600)} })
        .unwrap()
        .id();
    let token = app
        .world()
        .get::<WidgetryFileDialogState>(root)
        .unwrap()
        .token();
    WidgetryFileDialog::deliver(
        app.world_mut(),
        root,
        WidgetryFileDialogReply::Locations {
            token,
            outcome: Ok(vec![WidgetryFileDialogLocation {
                name: "Fixture".into(),
                path: PathBuf::from("C:/fixture"),
            }]),
        },
    )
    .unwrap();
    WidgetryFileDialog::apply(
        app.world_mut(),
        root,
        WidgetryFileDialogAction::Pin(PathBuf::from("C:/pinned")),
    )
    .unwrap();
    for _ in 0..3 {
        app.update();
    }
    let locations: Vec<_> = app
        .world_mut()
        .query::<(Entity, &Name)>()
        .iter(app.world())
        .filter(|(entity, name)| {
            name.as_str() == "FileDialogLocation" && belongs(app.world(), *entity, root)
        })
        .map(|(entity, _)| entity)
        .collect();
    assert_eq!(locations.len(), 2);
    for resolved in [false, true] {
        if resolved {
            app.world_mut()
                .entity_mut(root)
                .remove::<bevy::ui::InteractionDisabled>();
            app.update();
            for entity in &locations {
                assert!(
                    app.world()
                        .get::<bevy::ui::InteractionDisabled>(*entity)
                        .is_none()
                );
                assert_eq!(
                    app.world()
                        .get::<bevy::input_focus::tab_navigation::TabIndex>(*entity)
                        .unwrap()
                        .0,
                    0
                );
            }
            WidgetryFileDialog::apply(app.world_mut(), root, WidgetryFileDialogAction::Cancel)
                .unwrap();
        } else {
            app.world_mut()
                .entity_mut(root)
                .insert(bevy::ui::InteractionDisabled);
        }
        for _ in 0..3 {
            app.update();
        }
        for entity in &locations {
            assert!(
                app.world()
                    .get::<bevy::ui::InteractionDisabled>(*entity)
                    .is_some()
            );
            assert_eq!(
                app.world()
                    .get::<bevy::input_focus::tab_navigation::TabIndex>(*entity)
                    .unwrap()
                    .0,
                -1
            );
        }
    }
}

#[test]
fn queued_navigation_space_and_enter_wait_for_their_active_selection_reply() {
    for activation in [false, true] {
        let mut app = fixture();
        let root = app.world_mut().spawn_scene(bsn! { @WidgetryFileDialog { @mode: WidgetryFileDialogMode::PickFiles } Node {width: px(800), height: px(600)} }).unwrap().id();
        loaded(&mut app, root, 3);
        for _ in 0..3 {
            app.update();
        }
        let expected = app
            .world()
            .get::<WidgetryFileDialogState>(root)
            .unwrap()
            .snapshot()
            .unwrap()
            .visible()[if activation { 2 } else { 1 }];
        let area = named(&mut app, root, "FileDialogEntries");
        app.world_mut()
            .resource_mut::<InputFocus>()
            .set(area, FocusCause::Navigated);
        let keys = if activation {
            vec![KeyCode::End, KeyCode::Enter]
        } else {
            vec![
                KeyCode::ArrowDown,
                KeyCode::Space,
                KeyCode::ArrowDown,
                KeyCode::Space,
            ]
        };
        for key_code in keys {
            bevy_widgetry_test_utils::queue_key(
                &mut app,
                bevy::input::keyboard::KeyboardInput {
                    key_code,
                    logical_key: bevy::input::keyboard::Key::Unidentified(
                        bevy::input::keyboard::NativeKey::Unidentified,
                    ),
                    state: bevy::input::ButtonState::Pressed,
                    text: None,
                    repeat: false,
                    window: Entity::PLACEHOLDER,
                },
            );
        }
        app.update();
        for _ in 0..3 {
            app.update();
        }
        for _ in 0..6 {
            finish_selection(&mut app, root);
        }
        let state = app.world().get::<WidgetryFileDialogState>(root).unwrap();
        assert_eq!(state.active(), Some(expected));
        assert!(state.selected().contains(&expected));
        if activation {
            assert_eq!(state.selected().len(), 1);
            assert_eq!(
                state.confirmation(),
                &WidgetryFileDialogConfirmation::Validating
            );
            let validation = state
                .validation_job()
                .expect("Enter must activate the new last entry");
            assert_eq!(
                validation.prepare().unwrap().result(),
                &WidgetryFileDialogResult::Files(
                    vec![PathBuf::from("C:/fixture/file_000002.txt")].into()
                )
            );
        } else {
            assert_eq!(state.selected().len(), 2);
        }
    }
}

#[test]
fn deferred_entry_input_is_discarded_on_focus_query_change_and_escape_cancels_immediately() {
    for change in [0, 1, 2] {
        let mut app = fixture();
        let root = app.world_mut().spawn_scene(bsn! { @WidgetryFileDialog { @mode: WidgetryFileDialogMode::PickFiles } Node {width: px(800), height: px(600)} }).unwrap().id();
        loaded(&mut app, root, 3);
        for _ in 0..3 {
            app.update();
        }
        let area = named(&mut app, root, "FileDialogEntries");
        app.world_mut()
            .resource_mut::<InputFocus>()
            .set(area, FocusCause::Navigated);
        press_key(&mut app, Entity::PLACEHOLDER, KeyCode::ArrowDown);
        press_key(&mut app, Entity::PLACEHOLDER, KeyCode::Space);
        assert!(
            app.world()
                .get::<WidgetryFileDialogState>(root)
                .unwrap()
                .selection_pending()
        );
        match change {
            0 => {
                let search = named(&mut app, root, "FileDialogSearch");
                app.world_mut()
                    .resource_mut::<InputFocus>()
                    .set(search, FocusCause::Navigated);
            }
            1 => {
                WidgetryFileDialog::apply(
                    app.world_mut(),
                    root,
                    WidgetryFileDialogAction::Search("000001".into()),
                )
                .unwrap();
            }
            _ => {
                press_key(&mut app, Entity::PLACEHOLDER, KeyCode::Escape);
            }
        }
        for _ in 0..3 {
            app.update();
        }
        finish_selection(&mut app, root);
        let state = app.world().get::<WidgetryFileDialogState>(root).unwrap();
        assert!(state.selected().is_empty());
        if change == 2 {
            assert_eq!(state.result(), Some(&WidgetryFileDialogResult::Cancelled));
        }
    }
}

#[test]
fn tab_skips_hidden_panels_and_disabled_controls() {
    let mut app = fixture();
    let root = app.world_mut().spawn_scene(bsn! { @WidgetryFileDialog Node {width: px(800), height: px(600)} bevy::input_focus::tab_navigation::TabGroup::new(0) }).unwrap().id();
    for _ in 0..3 {
        app.update();
    }
    let path = named(&mut app, root, "FileDialogPath");
    app.world_mut()
        .resource_mut::<InputFocus>()
        .set(path, FocusCause::Navigated);
    for _ in 0..40 {
        press_key(&mut app, Entity::PLACEHOLDER, KeyCode::Tab);
        let focused = app.world().resource::<InputFocus>().get().unwrap();
        assert!(
            app.world()
                .get::<bevy::ui::InteractionDisabled>(focused)
                .is_none()
        );
        let mut ancestor = focused;
        loop {
            assert!(
                app.world()
                    .get::<Node>(ancestor)
                    .is_none_or(|node| node.display != Display::None),
                "Tab must not enter a hidden panel"
            );
            let Some(parent) = app.world().get::<ChildOf>(ancestor) else {
                break;
            };
            ancestor = parent.parent();
        }
    }
}

#[test]
fn press_is_invalidated_by_a_new_stream_snapshot_even_with_the_same_entry_id() {
    let mut app = fixture();
    let root = app
        .world_mut()
        .spawn_scene(bsn! { @WidgetryFileDialog Node {width: px(800), height: px(600)} })
        .unwrap()
        .id();
    loaded(&mut app, root, 10);
    for _ in 0..3 {
        app.update();
    }
    let row = named(&mut app, root, "FileDialogEntryRow");
    press(&mut app, row);
    let state = app
        .world()
        .get::<WidgetryFileDialogState>(root)
        .unwrap()
        .clone();
    let previous = state.snapshot().unwrap();
    let data = previous
        .entries()
        .iter()
        .map(|entry| WidgetryFileDialogEntryData {
            path: entry.path().to_owned(),
            name: entry.name().to_owned(),
            kind: entry.kind(),
            size: Some(999),
            modified: None,
            hidden: Some(false),
            system: Some(false),
        })
        .collect();
    let snapshot = Arc::new(
        WidgetryFileDialogSnapshot::prepare(
            state.token(),
            previous.path().to_owned(),
            data,
            &state.query(),
            Some(previous),
        )
        .unwrap(),
    );
    let selection = state
        .projection_selection_job(snapshot.clone())
        .unwrap()
        .prepare()
        .unwrap();
    WidgetryFileDialog::deliver(
        app.world_mut(),
        root,
        WidgetryFileDialogReply::Snapshot {
            snapshot,
            selection,
            state: WidgetryFileDialogDirectoryState::Ready,
        },
    )
    .unwrap();
    app.update();
    release(&mut app, row);
    app.world_mut().trigger(primary_click(row));
    app.world_mut().flush();
    assert!(
        app.world()
            .get::<WidgetryFileDialogState>(root)
            .unwrap()
            .selection_job()
            .is_none()
    );
}

#[test]
fn invalid_style_is_reported_once_and_does_not_starve_another_dialog() {
    let mut app = fixture();
    app.set_error_handler(ErrorCapture::handler());
    let first = app
        .world_mut()
        .spawn_scene(bsn! { @WidgetryFileDialog })
        .unwrap()
        .id();
    let second = app
        .world_mut()
        .spawn_scene(bsn! { @WidgetryFileDialog })
        .unwrap()
        .id();
    app.world_mut()
        .entity_mut(first)
        .insert(WidgetryFileDialogStyle {
            row_height: f32::NAN,
            ..default()
        });
    let errors = ErrorCapture::default();
    let logs = LogCapture::default();
    logs.run(|| {
        errors.run(|| {
            for _ in 0..3 {
                app.update();
            }
        })
    });
    named(&mut app, second, "FileDialogCancel");
    let captured = errors.take();
    assert_eq!(captured.len(), 1);
    assert_eq!(captured[0].severity(), bevy::ecs::error::Severity::Error);
    assert!(
        captured[0]
            .to_string()
            .contains("invalid FileDialog style dimensions")
    );
    assert_eq!(
        logs.records()
            .iter()
            .filter(|record| record.target == "bevy_widgetry"
                && record.level == bevy::log::Level::ERROR)
            .count(),
        1
    );
    app.world_mut()
        .get_mut::<WidgetryFileDialogStyle>(first)
        .unwrap()
        .row_height = 28.0;
    app.update();
    named(&mut app, first, "FileDialogCancel");
}

#[test]
fn new_folder_escape_dismisses_editor_before_cancel_and_pins_are_navigable() {
    let mut app = fixture();
    let root = app
        .world_mut()
        .spawn_scene(bsn! { @WidgetryFileDialog Node { width: px(800), height: px(600) } })
        .unwrap()
        .id();
    loaded(&mut app, root, 10);
    for _ in 0..3 {
        app.update();
    }
    let pin = named(&mut app, root, "FileDialogPin");
    app.world_mut()
        .trigger(bevy::ui_widgets::Activate { entity: pin });
    app.world_mut().flush();
    app.update();
    assert!(
        app.world_mut()
            .query::<&Name>()
            .iter(app.world())
            .any(|name| name.as_str() == "FileDialogLocation")
    );
    let new_folder = named(&mut app, root, "FileDialogNewFolder");
    app.world_mut()
        .trigger(bevy::ui_widgets::Activate { entity: new_folder });
    app.world_mut().flush();
    press_key(&mut app, Entity::PLACEHOLDER, KeyCode::Escape);
    assert_eq!(
        app.world()
            .get::<WidgetryFileDialogState>(root)
            .unwrap()
            .session_state(),
        WidgetryFileDialogSessionState::Open
    );
    let panel = named(&mut app, root, "FileDialogFolderPanel");
    assert_eq!(
        app.world().get::<Node>(panel).unwrap().display,
        Display::None
    );
    press_key(&mut app, Entity::PLACEHOLDER, KeyCode::Escape);
    assert_eq!(
        app.world()
            .get::<WidgetryFileDialogState>(root)
            .unwrap()
            .result(),
        Some(&WidgetryFileDialogResult::Cancelled)
    );
}

#[test]
fn ime_pending_composition_does_not_submit_or_cancel_dialog() {
    let mut app = fixture();
    let root = app
        .world_mut()
        .spawn_scene(bsn! { @WidgetryFileDialog Node { width: px(800), height: px(600) } })
        .unwrap()
        .id();
    app.update();
    let path = named(&mut app, root, "FileDialogPath");
    app.world_mut()
        .resource_mut::<InputFocus>()
        .set(path, FocusCause::Navigated);
    app.world_mut()
        .get_mut::<EditableText>(path)
        .unwrap()
        .queue_edit(bevy::text::TextEdit::ImeSetCompose {
            value: "文".into(),
            cursor: None,
        });
    press_key(&mut app, Entity::PLACEHOLDER, KeyCode::Escape);
    assert_eq!(
        app.world()
            .get::<WidgetryFileDialogState>(root)
            .unwrap()
            .session_state(),
        WidgetryFileDialogSessionState::Open
    );
}

#[test]
fn runtime_style_updates_sidebar_font_and_rows_without_resetting_editors() {
    let mut app = fixture();
    let root = app
        .world_mut()
        .spawn_scene(bsn! { @WidgetryFileDialog Node { width: px(800), height: px(600) } })
        .unwrap()
        .id();
    loaded(&mut app, root, 100);
    for _ in 0..3 {
        app.update();
    }
    let path = named(&mut app, root, "FileDialogPath");
    app.world_mut()
        .get_mut::<EditableText>(path)
        .unwrap()
        .editor_mut()
        .set_text("editing");
    app.world_mut()
        .get_mut::<WidgetryFileDialogStyle>(root)
        .unwrap()
        .font_size = 18.0;
    app.world_mut()
        .get_mut::<WidgetryFileDialogStyle>(root)
        .unwrap()
        .row_height = 36.0;
    app.world_mut()
        .get_mut::<WidgetryFileDialogStyle>(root)
        .unwrap()
        .sidebar_width = 200.0;
    app.update();
    let sidebar = named(&mut app, root, "FileDialogSidebar");
    assert_eq!(app.world().get::<Node>(sidebar).unwrap().width, px(200));
    assert_eq!(
        app.world().get::<TextFont>(path).unwrap().font_size,
        bevy::text::FontSize::Px(18.0)
    );
    assert_eq!(
        app.world()
            .get::<EditableText>(path)
            .unwrap()
            .value()
            .to_string(),
        "editing"
    );
}

#[test]
fn save_validation_overwrite_no_and_cancel_keep_the_session_contract() {
    let mut app = fixture();
    let root = app.world_mut().spawn_scene(bsn! { @WidgetryFileDialog { @mode: WidgetryFileDialogMode::SaveFile } Node {width: px(800), height: px(600)} }).unwrap().id();
    loaded(&mut app, root, 30);
    for _ in 0..3 {
        app.update();
    }
    let filename = named(&mut app, root, "FileDialogFilename");
    app.world_mut()
        .get_mut::<EditableText>(filename)
        .unwrap()
        .editor_mut()
        .set_text("output.txt");
    app.update();
    app.world_mut()
        .resource_mut::<InputFocus>()
        .set(filename, FocusCause::Navigated);
    bevy_widgetry_test_utils::queue_key(
        &mut app,
        bevy::input::keyboard::KeyboardInput {
            key_code: KeyCode::Enter,
            logical_key: bevy::input::keyboard::Key::Enter,
            state: bevy::input::ButtonState::Pressed,
            text: Some("\r".into()),
            repeat: false,
            window: Entity::PLACEHOLDER,
        },
    );
    app.update();
    let job = app
        .world()
        .get::<WidgetryFileDialogState>(root)
        .unwrap()
        .validation_job()
        .unwrap();
    let candidate = job.prepare().unwrap();
    WidgetryFileDialog::deliver(
        app.world_mut(),
        root,
        WidgetryFileDialogReply::Validated {
            candidate,
            exists: true,
        },
    )
    .unwrap();
    app.update();
    let panel = named(&mut app, root, "FileDialogOverwritePanel");
    assert_eq!(
        app.world().get::<Node>(panel).unwrap().display,
        Display::Flex
    );
    let no = named(&mut app, root, "FileDialogOverwriteNo");
    app.world_mut()
        .trigger(bevy::ui_widgets::Activate { entity: no });
    app.world_mut().flush();
    assert_eq!(
        app.world()
            .get::<WidgetryFileDialogState>(root)
            .unwrap()
            .confirmation(),
        &WidgetryFileDialogConfirmation::Idle
    );
    assert!(
        app.world()
            .get::<WidgetryFileDialogState>(root)
            .unwrap()
            .result()
            .is_none()
    );
    let cancel = named(&mut app, root, "FileDialogCancel");
    app.world_mut()
        .trigger(bevy::ui_widgets::Activate { entity: cancel });
    app.world_mut().flush();
    assert_eq!(
        app.world()
            .get::<WidgetryFileDialogState>(root)
            .unwrap()
            .result(),
        Some(&WidgetryFileDialogResult::Cancelled)
    );
}

#[test]
fn real_sort_popup_choice_and_hidden_toggle_update_headless_query() {
    let mut app = fixture();
    let root = app
        .world_mut()
        .spawn_scene(bsn! { @WidgetryFileDialog Node {width: px(800), height: px(600)} })
        .unwrap()
        .id();
    loaded(&mut app, root, 30);
    for _ in 0..3 {
        app.update();
    }
    let sort = named(&mut app, root, "FileDialogSort");
    let field = app
        .world()
        .get::<Children>(sort)
        .unwrap()
        .iter()
        .next()
        .unwrap();
    press(&mut app, field);
    release(&mut app, field);
    app.world_mut().trigger(primary_click(field));
    app.world_mut().flush();
    for _ in 0..3 {
        app.update();
    }
    let popup = app
        .world()
        .get::<Children>(sort)
        .unwrap()
        .iter()
        .nth(1)
        .unwrap();
    let list = app
        .world()
        .get::<Children>(popup)
        .unwrap()
        .iter()
        .next()
        .unwrap();
    let row = app
        .world_mut()
        .query::<(Entity, &bevy_widgetry_list_view::WidgetryListViewItem)>()
        .iter(app.world())
        .find(|(entity, item)| item.index == 1 && belongs(app.world(), *entity, list))
        .unwrap()
        .0;
    press(&mut app, row);
    release(&mut app, row);
    app.world_mut().trigger(primary_click(row));
    app.world_mut().flush();
    assert_eq!(
        app.world()
            .get::<WidgetryFileDialogState>(root)
            .unwrap()
            .preferences()
            .sort,
        WidgetryFileDialogSort::NameDescending
    );
    let hidden = named(&mut app, root, "FileDialogHidden");
    press(&mut app, hidden);
    release(&mut app, hidden);
    app.world_mut().trigger(primary_click(hidden));
    app.world_mut().flush();
    assert!(
        app.world()
            .get::<WidgetryFileDialogState>(root)
            .unwrap()
            .preferences()
            .show_hidden
    );
}

fn named(app: &mut App, root: Entity, label: &str) -> Entity {
    let candidates: Vec<_> = app
        .world_mut()
        .query::<(Entity, &Name)>()
        .iter(app.world())
        .filter(|(_, name)| name.as_str() == label)
        .map(|(entity, _)| entity)
        .collect();
    candidates
        .into_iter()
        .find(|entity| {
            let mut parent = *entity;
            while let Some(relation) = app.world().get::<ChildOf>(parent) {
                parent = relation.parent();
                if parent == root {
                    return true;
                }
            }
            false
        })
        .unwrap()
}

fn belongs(world: &World, mut entity: Entity, ancestor: Entity) -> bool {
    while let Some(parent) = world.get::<ChildOf>(entity) {
        entity = parent.parent();
        if entity == ancestor {
            return true;
        }
    }
    false
}

fn entry_viewport(app: &mut App, root: Entity) -> Entity {
    let area = named(app, root, "FileDialogEntries");
    app.world()
        .get::<Children>(area)
        .unwrap()
        .iter()
        .find(|entity| {
            app.world()
                .get::<bevy_widgetry_scroll_area::WidgetryScrollAreaViewport>(*entity)
                .is_some()
        })
        .unwrap()
}

#[test]
fn shell_is_present_while_directory_loading_and_cancel_is_available() {
    let mut app = fixture();
    let root = app
        .world_mut()
        .spawn_scene(bsn! { @WidgetryFileDialog Node { width: px(800), height: px(600) } })
        .unwrap()
        .id();
    app.update();
    let names: Vec<_> = app
        .world_mut()
        .query::<&Name>()
        .iter(app.world())
        .map(|name| name.as_str().to_owned())
        .collect();
    for name in [
        "FileDialogPath",
        "FileDialogSearch",
        "FileDialogEntries",
        "FileDialogStatus",
        "FileDialogCancel",
    ] {
        assert!(names.iter().any(|value| value == name), "missing {name}");
    }
    assert_eq!(
        app.world()
            .get::<WidgetryFileDialogState>(root)
            .unwrap()
            .directory_state(),
        &WidgetryFileDialogDirectoryState::Loading
    );
}

#[test]
fn large_directory_materializes_only_visible_rows_and_scrolls_to_last_entry() {
    let mut app = fixture();
    let root = app
        .world_mut()
        .spawn_scene(bsn! { @WidgetryFileDialog Node { width: px(800), height: px(600) } })
        .unwrap()
        .id();
    loaded(&mut app, root, 10_000);
    for _ in 0..3 {
        app.update();
    }
    let rows = app
        .world_mut()
        .query::<&Name>()
        .iter(app.world())
        .filter(|name| name.as_str() == "FileDialogEntryRow")
        .count();
    assert!((1..=30).contains(&rows), "rows={rows}");
    let viewport = entry_viewport(&mut app, root);
    app.world_mut()
        .get_mut::<bevy::ui::ScrollPosition>(viewport)
        .unwrap()
        .0
        .y = 1e9;
    for _ in 0..3 {
        app.update();
    }
    assert!(
        app.world_mut()
            .query::<&Text>()
            .iter(app.world())
            .any(|text| text.0 == "file_009999.txt")
    );
    assert!(
        app.world_mut()
            .query::<&Name>()
            .iter(app.world())
            .filter(|name| name.as_str() == "FileDialogEntryRow")
            .count()
            <= 30
    );
}

#[test]
fn pointer_selection_commits_authority_and_recycled_press_cannot_select_new_identity() {
    let mut app = fixture();
    let root = app.world_mut().spawn_scene(bsn! { @WidgetryFileDialog { @mode: WidgetryFileDialogMode::PickFiles } Node { width: px(800), height: px(600) } }).unwrap().id();
    loaded(&mut app, root, 1000);
    for _ in 0..3 {
        app.update();
    }
    let row = app
        .world_mut()
        .query::<(Entity, &Name)>()
        .iter(app.world())
        .find(|(_, name)| name.as_str() == "FileDialogEntryRow")
        .unwrap()
        .0;
    press(&mut app, row);
    release(&mut app, row);
    app.world_mut().trigger(primary_click(row));
    app.world_mut().flush();
    let job = app
        .world()
        .get::<WidgetryFileDialogState>(root)
        .unwrap()
        .selection_job();
    assert!(job.is_some(), "pointer did not submit selection");
    WidgetryFileDialog::deliver(
        app.world_mut(),
        root,
        WidgetryFileDialogReply::Selection(job.unwrap().prepare().unwrap()),
    )
    .unwrap();
    assert_eq!(
        app.world()
            .get::<WidgetryFileDialogState>(root)
            .unwrap()
            .selected()
            .len(),
        1
    );
    press(&mut app, row);
    let viewport = entry_viewport(&mut app, root);
    app.world_mut()
        .get_mut::<bevy::ui::ScrollPosition>(viewport)
        .unwrap()
        .0
        .y = 15000.0;
    app.update();
    release(&mut app, row);
    app.world_mut().trigger(primary_click(row));
    app.world_mut().flush();
    assert!(
        app.world()
            .get::<WidgetryFileDialogState>(root)
            .unwrap()
            .selection_job()
            .is_none()
    );
}

#[test]
fn focused_keyboard_moves_active_and_editor_enter_navigates_without_losing_other_view() {
    let mut app = fixture();
    let first = app
        .world_mut()
        .spawn_scene(bsn! { @WidgetryFileDialog Node { width: px(450), height: px(600) } })
        .unwrap()
        .id();
    let second = app
        .world_mut()
        .spawn_scene(bsn! { @WidgetryFileDialog Node { width: px(450), height: px(600) } })
        .unwrap()
        .id();
    loaded(&mut app, first, 100);
    for _ in 0..3 {
        app.update();
    }
    let area = named(&mut app, first, "FileDialogEntries");
    app.world_mut()
        .resource_mut::<InputFocus>()
        .set(area, FocusCause::Navigated);
    press_key(&mut app, Entity::PLACEHOLDER, KeyCode::End);
    let job = app
        .world()
        .get::<WidgetryFileDialogState>(first)
        .unwrap()
        .selection_job();
    assert!(job.is_some(), "focused navigation did not submit a job");
    WidgetryFileDialog::deliver(
        app.world_mut(),
        first,
        WidgetryFileDialogReply::Selection(job.unwrap().prepare().unwrap()),
    )
    .unwrap();
    let state = app.world().get::<WidgetryFileDialogState>(first).unwrap();
    assert_eq!(state.active(), state.visible().last().copied());
    assert!(state.selected().is_empty());
    let path = named(&mut app, first, "FileDialogPath");
    app.world_mut()
        .get_mut::<EditableText>(path)
        .unwrap()
        .editor_mut()
        .set_text("C:/other");
    app.world_mut()
        .resource_mut::<InputFocus>()
        .set(path, FocusCause::Navigated);
    press_key(&mut app, Entity::PLACEHOLDER, KeyCode::Enter);
    assert_eq!(
        app.world()
            .get::<WidgetryFileDialogState>(first)
            .unwrap()
            .requested_path(),
        Some(PathBuf::from("C:/other").as_path())
    );
    assert!(
        app.world()
            .get::<WidgetryFileDialogState>(second)
            .unwrap()
            .requested_path()
            .is_none()
    );
}

#[test]
fn search_editor_updates_query_and_theme_keeps_text_cursor_and_session() {
    let mut app = fixture();
    let root = app
        .world_mut()
        .spawn_scene(bsn! { @WidgetryFileDialog Node { width: px(800), height: px(600) } })
        .unwrap()
        .id();
    loaded(&mut app, root, 30);
    for _ in 0..3 {
        app.update();
    }
    let editor = named(&mut app, root, "FileDialogSearch");
    app.world_mut()
        .get_mut::<EditableText>(editor)
        .unwrap()
        .editor_mut()
        .set_text("hello");
    app.update();
    assert_eq!(
        app.world()
            .get::<WidgetryFileDialogState>(root)
            .unwrap()
            .search(),
        "hello"
    );
    let selection = app
        .world()
        .get::<EditableText>(editor)
        .unwrap()
        .editor()
        .raw_selection()
        .text_range();
    let session = app
        .world()
        .get::<WidgetryFileDialogState>(root)
        .unwrap()
        .token()
        .session;
    switch_theme(&mut app, ThemeMode::Light);
    app.update();
    assert_eq!(
        app.world()
            .get::<EditableText>(editor)
            .unwrap()
            .value()
            .to_string(),
        "hello"
    );
    assert_eq!(
        app.world()
            .get::<EditableText>(editor)
            .unwrap()
            .editor()
            .raw_selection()
            .text_range(),
        selection
    );
    assert_eq!(
        app.world()
            .get::<WidgetryFileDialogState>(root)
            .unwrap()
            .token()
            .session,
        session
    );
}

#[test]
fn filter_sort_visibility_new_folder_and_overwrite_controls_exist_without_loading_dependency() {
    let mut app = fixture();
    let root = app.world_mut().spawn_scene(bsn! { @WidgetryFileDialog { @mode: WidgetryFileDialogMode::SaveFile } Node { width: px(800), height: px(600) } }).unwrap().id();
    app.update();
    for label in [
        "FileDialogFilter",
        "FileDialogSort",
        "FileDialogHidden",
        "FileDialogSystem",
        "FileDialogNewFolder",
        "FileDialogPin",
        "FileDialogFilename",
    ] {
        named(&mut app, root, label);
    }
    let cancel = named(&mut app, root, "FileDialogCancel");
    app.world_mut()
        .trigger(bevy::ui_widgets::Activate { entity: cancel });
    app.world_mut().flush();
    assert_eq!(
        app.world()
            .get::<WidgetryFileDialogState>(root)
            .unwrap()
            .result(),
        Some(&WidgetryFileDialogResult::Cancelled)
    );
}
