#![allow(clippy::disallowed_macros, clippy::unwrap_used)]

use bevy::input_focus::{FocusCause, InputFocus};
use bevy::prelude::*;
use bevy::text::EditableText;
use bevy::ui::InteractionDisabled;
use bevy_widgetry_theme::WidgetryThemeMode;

use bevy_widgetry_file_dialog::*;
use bevy_widgetry_test_utils::{ErrorCapture, LogCapture};
use bevy_widgetry_test_utils::{add_ui_plugins, spawn_ui_camera, text_input_app};
use bevy_widgetry_test_utils::{press, primary_click, release};
use bevy_widgetry_test_utils::{press_key, switch_theme};
use std::path::PathBuf;
use std::sync::Arc;

fn input_entity(world: &World, root: Entity) -> Entity {
    world
        .get::<bevy_widgetry_core::input_target::WidgetryInputTarget>(root)
        .map(|mapping| mapping.target)
        .unwrap_or(root)
}

fn frame_entity(world: &World, root: Entity) -> Entity {
    world
        .get::<Children>(root)
        .map(|children| children[0])
        .unwrap_or(root)
}

fn loaded(app: &mut App, root: Entity, count: usize) {
    loaded_kind(app, root, count, WidgetryFileDialogEntryKind::File);
}

fn loaded_kind(app: &mut App, root: Entity, count: usize, kind: WidgetryFileDialogEntryKind) {
    loaded_at(app, root, count, kind, PathBuf::from("C:/fixture"));
}

fn loaded_at(
    app: &mut App,
    root: Entity,
    count: usize,
    kind: WidgetryFileDialogEntryKind,
    path: PathBuf,
) {
    let state = app.world().get::<WidgetryFileDialogState>(root).unwrap();
    let token = state.token();
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
    let mut app = text_input_app();
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

#[test]
fn address_display_hides_filesystem_prefix_without_changing_enter_identity() {
    use std::os::windows::ffi::OsStringExt;
    let mut units = r"\\?\C:\fixture\".encode_utf16().collect::<Vec<_>>();
    units.push(0xd800);
    for (raw, shown) in [
        (PathBuf::from(r"C:\fixture"), String::from(r"C:\fixture")),
        (
            PathBuf::from(r"\\?\C:\fixture"),
            String::from(r"C:\fixture"),
        ),
        (
            PathBuf::from(r"\\?\UNC\server\share\fixture"),
            String::from(r"\\server\share\fixture"),
        ),
        (
            PathBuf::from(r"\\?\Volume{1234}\fixture"),
            String::from(r"\\?\Volume{1234}\fixture"),
        ),
        (
            PathBuf::from(std::ffi::OsString::from_wide(&units)),
            format!("{}\u{fffd}", r"C:\fixture\"),
        ),
    ] {
        let mut app = fixture();
        let root = app
            .world_mut()
            .spawn_scene(bsn! { @WidgetryFileDialog Node {width:px(800),height:px(600)} })
            .unwrap()
            .id();
        loaded_at(
            &mut app,
            root,
            2,
            WidgetryFileDialogEntryKind::File,
            raw.clone(),
        );
        for _ in 0..3 {
            app.update();
        }
        let editor = named(&mut app, root, "FileDialogPath");
        assert_eq!(
            app.world()
                .get::<EditableText>(input_entity(app.world(), editor))
                .unwrap()
                .value()
                .to_string(),
            shown
        );
        assert_eq!(
            app.world()
                .get::<WidgetryFileDialogState>(root)
                .unwrap()
                .current_path(),
            Some(raw.as_path())
        );
        app.world_mut()
            .resource_mut::<InputFocus>()
            .set(editor, FocusCause::Navigated);
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
        assert_eq!(
            app.world()
                .get::<WidgetryFileDialogState>(root)
                .unwrap()
                .requested_path(),
            Some(raw.as_path())
        );
    }
}

#[test]
fn native_escape_from_text_field_cancels_after_bevy_blurs_the_editor() {
    let mut app = fixture();
    let root = app
        .world_mut()
        .spawn_scene(bsn! { @WidgetryFileDialog Node {width:px(800),height:px(600)} })
        .unwrap()
        .id();
    loaded(&mut app, root, 2);
    for _ in 0..3 {
        app.update();
    }
    let editor = named(&mut app, root, "FileDialogFilename");
    app.world_mut()
        .resource_mut::<InputFocus>()
        .set(editor, FocusCause::Navigated);
    bevy_widgetry_test_utils::queue_key(
        &mut app,
        bevy::input::keyboard::KeyboardInput {
            key_code: KeyCode::Escape,
            logical_key: bevy::input::keyboard::Key::Escape,
            state: bevy::input::ButtonState::Pressed,
            text: None,
            repeat: false,
            window: Entity::PLACEHOLDER,
        },
    );
    app.update();
    assert_eq!(
        app.world()
            .get::<WidgetryFileDialogState>(root)
            .unwrap()
            .result(),
        Some(&WidgetryFileDialogResult::Cancelled)
    );
}

#[test]
fn system_shell_places_navigation_together_and_mode_fields_below_entries() {
    for mode in [
        WidgetryFileDialogMode::PickFile,
        WidgetryFileDialogMode::PickFiles,
        WidgetryFileDialogMode::PickDirectory,
        WidgetryFileDialogMode::PickDirectories,
        WidgetryFileDialogMode::SaveFile,
    ] {
        let mut app = fixture();
        let root = app
            .world_mut()
            .spawn_scene(bsn! { @WidgetryFileDialog { @mode: mode }
            Node {width:px(800),height:px(600)} })
            .unwrap()
            .id();
        loaded(&mut app, root, 10);
        for _ in 0..4 {
            app.update();
        }
        let path = named(&mut app, root, "FileDialogPath");
        let search = named(&mut app, root, "FileDialogSearch");
        let back = named(&mut app, root, "FileDialogBack");
        assert!(
            app.world()
                .get::<bevy_widgetry_tooltip::WidgetryTooltip>(back)
                .is_some()
        );
        assert_eq!(
            app.world().get::<ChildOf>(path),
            app.world().get::<ChildOf>(search)
        );
        assert_eq!(
            app.world().get::<ChildOf>(path),
            app.world().get::<ChildOf>(back)
        );
        let entries = named(&mut app, root, "FileDialogEntries");
        let filename = if matches!(
            mode,
            WidgetryFileDialogMode::PickFile | WidgetryFileDialogMode::SaveFile
        ) {
            named(&mut app, root, "FileDialogFilename")
        } else {
            named(&mut app, root, "FileDialogFilenameSummary")
        };
        let top = app
            .world()
            .get::<UiGlobalTransform>(entries)
            .unwrap()
            .translation
            .y;
        let bottom = app
            .world()
            .get::<UiGlobalTransform>(filename)
            .unwrap()
            .translation
            .y;
        assert!(bottom > top);
        let has_filter = app
            .world_mut()
            .query::<(Entity, &Name)>()
            .iter(app.world())
            .any(|(entity, name)| {
                name.as_str() == "FileDialogFilter" && belongs(app.world(), entity, root)
            });
        assert_eq!(
            has_filter,
            !matches!(
                mode,
                WidgetryFileDialogMode::PickDirectory | WidgetryFileDialogMode::PickDirectories
            )
        );
        named(&mut app, root, "FileDialogColumnHeader");
    }
}

#[test]
fn pick_filename_syncs_selection_and_enter_uses_real_identity_and_validation() {
    let mut app = fixture();
    let root = app
        .world_mut()
        .spawn_scene(bsn! { @WidgetryFileDialog Node {width:px(800),height:px(600)} })
        .unwrap()
        .id();
    loaded(&mut app, root, 10);
    for _ in 0..3 {
        app.update();
    }
    let row = named(&mut app, root, "FileDialogEntryRow");
    press(&mut app, row);
    release(&mut app, row);
    app.world_mut().trigger(primary_click(row));
    app.world_mut().flush();
    finish_selection(&mut app, root);
    let editor = named(&mut app, root, "FileDialogFilename");
    assert_eq!(
        app.world()
            .get::<EditableText>(input_entity(app.world(), editor))
            .unwrap()
            .value()
            .to_string(),
        "file_000000.txt"
    );
    bevy_widgetry_text_field::WidgetryTextField::set_text_in_world(
        app.world_mut(),
        editor,
        "file_000005.txt",
    )
    .unwrap();
    app.update();
    assert!(
        app.world()
            .get::<WidgetryFileDialogState>(root)
            .unwrap()
            .filename()
            .is_empty()
    );
    app.world_mut()
        .resource_mut::<InputFocus>()
        .set(editor, FocusCause::Navigated);
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
    finish_selection(&mut app, root);
    let state = app.world().get::<WidgetryFileDialogState>(root).unwrap();
    assert!(state.result().is_none());
    let candidate = state.validation_job().unwrap().prepare().unwrap();
    assert!(app.world().get::<InteractionDisabled>(editor).is_some());
    assert_eq!(
        candidate.result(),
        &WidgetryFileDialogResult::File("C:/fixture/file_000005.txt".into())
    );
    WidgetryFileDialog::deliver(
        app.world_mut(),
        root,
        WidgetryFileDialogReply::ValidationFailed {
            token: candidate.token(),
            error: "file disappeared before confirmation".into(),
        },
    )
    .unwrap();
    app.update();
    let state = app.world().get::<WidgetryFileDialogState>(root).unwrap();
    assert!(state.result().is_none());
    assert_eq!(state.session_state(), WidgetryFileDialogSessionState::Open);
    let status = named(&mut app, root, "FileDialogStatus");
    assert!(app.world().get::<Text>(status).is_some());
}

#[test]
fn clicking_the_selected_file_restores_its_name_after_a_draft_edit() {
    let mut app = fixture();
    let root = app
        .world_mut()
        .spawn_scene(bsn! {@WidgetryFileDialog Node {width:px(800),height:px(600)}})
        .unwrap()
        .id();
    loaded(&mut app, root, 10);
    for _ in 0..3 {
        app.update();
    }
    let row = named(&mut app, root, "FileDialogEntryRow");
    for _ in 0..2 {
        press(&mut app, row);
        release(&mut app, row);
        app.world_mut().trigger(primary_click(row));
        app.world_mut().flush();
        finish_selection(&mut app, root);
        let editor = named(&mut app, root, "FileDialogFilename");
        assert_eq!(
            app.world()
                .get::<EditableText>(input_entity(app.world(), editor))
                .unwrap()
                .value()
                .to_string(),
            "file_000000.txt"
        );
        bevy_widgetry_text_field::WidgetryTextField::set_text_in_world(
            app.world_mut(),
            editor,
            "other.txt",
        )
        .unwrap();
        app.update();
    }
}

#[test]
fn filename_confirmation_revalidates_a_file_removed_after_snapshot() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().canonicalize().unwrap();
    let file = path.join("present.txt");
    std::fs::write(&file, b"present").unwrap();
    let mut app = fixture();
    let root = app
        .world_mut()
        .spawn_scene(bsn! {@WidgetryFileDialog Node {width:px(800),height:px(600)}})
        .unwrap()
        .id();
    let state = app.world().get::<WidgetryFileDialogState>(root).unwrap();
    let token = state.token();
    let snapshot = Arc::new(
        WidgetryFileDialogSnapshot::prepare(
            token,
            path.clone(),
            vec![WidgetryFileDialogEntryData {
                path: file.clone(),
                name: "present.txt".into(),
                kind: WidgetryFileDialogEntryKind::File,
                size: Some(7),
                modified: None,
                hidden: Some(false),
                system: Some(false),
            }],
            &state.query(),
            None,
        )
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
    for _ in 0..3 {
        app.update();
    }
    let editor = named(&mut app, root, "FileDialogFilename");
    bevy_widgetry_text_field::WidgetryTextField::set_text_in_world(
        app.world_mut(),
        editor,
        "present.txt",
    )
    .unwrap();
    app.update();
    let confirm = named(&mut app, root, "FileDialogConfirm");
    app.world_mut()
        .trigger(bevy::ui_widgets::Activate { entity: confirm });
    app.world_mut().flush();
    finish_selection(&mut app, root);
    let candidate = app
        .world()
        .get::<WidgetryFileDialogState>(root)
        .unwrap()
        .validation_job()
        .unwrap()
        .prepare()
        .unwrap();
    assert!(
        WidgetryFileDialogNativeFileSystem
            .validate(&candidate)
            .is_ok()
    );
    std::fs::remove_file(&file).unwrap();
    let error = WidgetryFileDialogNativeFileSystem
        .validate(&candidate)
        .unwrap_err();
    WidgetryFileDialog::deliver(
        app.world_mut(),
        root,
        WidgetryFileDialogReply::ValidationFailed {
            token: candidate.token(),
            error: error.to_string(),
        },
    )
    .unwrap();
    let state = app.world().get::<WidgetryFileDialogState>(root).unwrap();
    assert!(state.result().is_none());
    assert!(state.error().is_some());
    assert_eq!(state.confirmation(), &WidgetryFileDialogConfirmation::Idle);
}

#[test]
fn combo_escape_closes_popup_before_cancelling_dialog_from_the_closed_field() {
    for name in ["FileDialogFilter", "FileDialogSort"] {
        let mut app = fixture();
        let root = app
            .world_mut()
            .spawn_scene(bsn! {@WidgetryFileDialog Node {width:px(800),height:px(600)}})
            .unwrap()
            .id();
        for _ in 0..3 {
            app.update();
        }
        let combo = named(&mut app, root, name);
        let field = app
            .world()
            .get::<Children>(combo)
            .unwrap()
            .iter()
            .find(|entity| {
                app.world()
                    .get::<bevy_widgetry_button::WidgetryButton>(*entity)
                    .is_some()
            })
            .unwrap();
        app.world_mut()
            .resource_mut::<InputFocus>()
            .set(field, FocusCause::Navigated);
        press_key(&mut app, Entity::PLACEHOLDER, KeyCode::Enter);
        assert_ne!(app.world().resource::<InputFocus>().get(), Some(field));
        press_key(&mut app, Entity::PLACEHOLDER, KeyCode::Escape);
        assert_eq!(app.world().resource::<InputFocus>().get(), Some(field));
        assert!(
            app.world()
                .get::<WidgetryFileDialogState>(root)
                .unwrap()
                .result()
                .is_none()
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
}

#[test]
fn confirm_cannot_submit_the_previous_directory_during_navigation_or_failure() {
    let mut app = fixture();
    let root=app.world_mut().spawn_scene(bsn! {@WidgetryFileDialog {@mode:WidgetryFileDialogMode::PickDirectory} Node {width:px(800),height:px(600)}}).unwrap().id();
    loaded_kind(&mut app, root, 2, WidgetryFileDialogEntryKind::Directory);
    for _ in 0..3 {
        app.update();
    }
    let confirm = named(&mut app, root, "FileDialogConfirm");
    assert!(app.world().get::<InteractionDisabled>(confirm).is_none());
    WidgetryFileDialog::apply(
        app.world_mut(),
        root,
        WidgetryFileDialogAction::Navigate(WidgetryFileDialogNavigation::Path(
            "C:/unavailable".into(),
        )),
    )
    .unwrap();
    app.update();
    assert!(app.world().get::<InteractionDisabled>(confirm).is_some());
    let token = app
        .world()
        .get::<WidgetryFileDialogState>(root)
        .unwrap()
        .token();
    WidgetryFileDialog::deliver(
        app.world_mut(),
        root,
        WidgetryFileDialogReply::Failed {
            token,
            error: "Directory unavailable".into(),
        },
    )
    .unwrap();
    app.update();
    assert!(app.world().get::<InteractionDisabled>(confirm).is_some());
    let cancel = named(&mut app, root, "FileDialogCancel");
    assert!(app.world().get::<InteractionDisabled>(cancel).is_none());
}

#[test]
fn pick_filename_rejects_missing_directory_and_path_syntax_without_submitting_old_selection() {
    for (kind, text) in [
        (WidgetryFileDialogEntryKind::File, "missing.txt"),
        (WidgetryFileDialogEntryKind::Directory, "file_000000.txt"),
        (
            WidgetryFileDialogEntryKind::File,
            "C:/fixture/file_000000.txt",
        ),
        (WidgetryFileDialogEntryKind::File, "*.txt"),
    ] {
        let mut app = fixture();
        app.set_error_handler(ErrorCapture::handler());
        let errors = ErrorCapture::default();
        let root = app
            .world_mut()
            .spawn_scene(bsn! { @WidgetryFileDialog Node {width:px(800),height:px(600)} })
            .unwrap()
            .id();
        loaded_kind(&mut app, root, 10, kind);
        for _ in 0..3 {
            app.update();
        }
        if kind.is_file() {
            let state = app.world().get::<WidgetryFileDialogState>(root).unwrap();
            let id = state.visible()[0];
            let token = state.token();
            WidgetryFileDialog::apply(
                app.world_mut(),
                root,
                WidgetryFileDialogAction::Select {
                    id,
                    token,
                    operation: WidgetryFileDialogSelection::Replace,
                },
            )
            .unwrap();
            finish_selection(&mut app, root);
        }
        let editor = named(&mut app, root, "FileDialogFilename");
        bevy_widgetry_text_field::WidgetryTextField::set_text_in_world(
            app.world_mut(),
            editor,
            text,
        )
        .unwrap();
        errors.run(|| app.update());
        assert!(
            errors.take().is_empty(),
            "draft editing must not invoke SaveFile Filename action"
        );
        let confirm = named(&mut app, root, "FileDialogConfirm");
        errors.run(|| {
            app.world_mut()
                .trigger(bevy::ui_widgets::Activate { entity: confirm });
            app.world_mut().flush();
        });
        let state = app.world().get::<WidgetryFileDialogState>(root).unwrap();
        assert_eq!(state.confirmation(), &WidgetryFileDialogConfirmation::Idle);
        assert!(state.result().is_none());
        assert_eq!(state.session_state(), WidgetryFileDialogSessionState::Open);
        app.update();
        let status = named(&mut app, root, "FileDialogStatus");
        assert!(
            app.world()
                .get::<Text>(status)
                .unwrap()
                .0
                .contains("File name")
        );
    }
}

#[test]
fn detail_rows_show_type_and_human_size_and_folders_have_no_size() {
    for kind in [
        WidgetryFileDialogEntryKind::File,
        WidgetryFileDialogEntryKind::Directory,
    ] {
        let mut app = fixture();
        let root = app
            .world_mut()
            .spawn_scene(bsn! { @WidgetryFileDialog Node {width:px(800),height:px(600)} })
            .unwrap()
            .id();
        loaded_kind(&mut app, root, 2, kind);
        for _ in 0..3 {
            app.update();
        }
        let row = named(&mut app, root, "FileDialogEntryRow");
        let texts: Vec<_> = app
            .world_mut()
            .query::<(Entity, &Text)>()
            .iter(app.world())
            .filter(|(entity, _)| belongs(app.world(), *entity, row))
            .map(|(_, text)| text.0.clone())
            .collect();
        if kind.is_directory() {
            assert!(texts.contains(&"File folder".into()));
            assert!(!texts.iter().any(|text| text.ends_with(" B")));
        } else {
            assert!(texts.contains(&"TXT file".into()));
            assert!(texts.contains(&"0 B".into()));
        }
    }
}

#[test]
fn detail_columns_align_with_header_when_scrollbar_appears() {
    for count in [2, 100] {
        let mut app = fixture();
        let root = app
            .world_mut()
            .spawn_scene(bsn! {@WidgetryFileDialog Node {width:px(800),height:px(600)}})
            .unwrap()
            .id();
        loaded(&mut app, root, count);
        for _ in 0..4 {
            app.update();
        }
        let header = named(&mut app, root, "FileDialogColumnHeader");
        let row = named(&mut app, root, "FileDialogEntryRow");
        let text_entity = |app: &mut App, owner, label: &str| {
            app.world_mut()
                .query::<(Entity, &Text)>()
                .iter(app.world())
                .find(|(entity, text)| text.0 == label && belongs(app.world(), *entity, owner))
                .unwrap()
                .0
        };
        let heading = text_entity(&mut app, header, "Type");
        let cell = text_entity(&mut app, row, "TXT file");
        let header_x = app
            .world()
            .get::<UiGlobalTransform>(heading)
            .unwrap()
            .translation
            .x;
        let cell_x = app
            .world()
            .get::<UiGlobalTransform>(cell)
            .unwrap()
            .translation
            .x;
        assert!(
            (header_x - cell_x).abs() <= 1.1,
            "count={count}, header={header_x}, cell={cell_x}"
        );
    }
}

#[test]
fn filter_labels_include_suffixes_and_filtered_selection_clears_filename() {
    let mut app = fixture();
    let filters = vec![
        WidgetryFileDialogFilter::default(),
        WidgetryFileDialogFilter {
            id: WidgetryFileDialogFilterId("images".into()),
            label: "Images".into(),
            suffixes: vec!["png".into(), "jpg".into(), "tar.gz".into()],
            default_extension: None,
        },
    ];
    let root = app
        .world_mut()
        .spawn_scene(
            bsn! { @WidgetryFileDialog {@filters:filters} Node {width:px(800),height:px(600)} },
        )
        .unwrap()
        .id();
    loaded(&mut app, root, 10);
    for _ in 0..3 {
        app.update();
    }
    let state = app.world().get::<WidgetryFileDialogState>(root).unwrap();
    let id = state.visible()[0];
    let token = state.token();
    WidgetryFileDialog::apply(
        app.world_mut(),
        root,
        WidgetryFileDialogAction::Select {
            id,
            token,
            operation: WidgetryFileDialogSelection::Replace,
        },
    )
    .unwrap();
    finish_selection(&mut app, root);
    WidgetryFileDialog::apply(
        app.world_mut(),
        root,
        WidgetryFileDialogAction::Filter(WidgetryFileDialogFilterId("images".into())),
    )
    .unwrap();
    let state = app.world().get::<WidgetryFileDialogState>(root).unwrap();
    let snapshot = Arc::new(
        state
            .snapshot()
            .unwrap()
            .reproject(state.token(), &state.query())
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
            state: WidgetryFileDialogDirectoryState::Ready,
            selection,
        },
    )
    .unwrap();
    for _ in 0..3 {
        app.update();
    }
    let editor = named(&mut app, root, "FileDialogFilename");
    assert!(
        app.world()
            .get::<EditableText>(input_entity(app.world(), editor))
            .unwrap()
            .value()
            .to_string()
            .is_empty()
    );
    let confirm = named(&mut app, root, "FileDialogConfirm");
    assert!(app.world().get::<InteractionDisabled>(confirm).is_some());
    let filter = named(&mut app, root, "FileDialogFilter");
    let labels: Vec<_> = app
        .world_mut()
        .query::<(Entity, &Text)>()
        .iter(app.world())
        .filter(|(entity, _)| belongs(app.world(), *entity, filter))
        .map(|(_, text)| text.0.clone())
        .collect();
    assert!(labels.contains(&"Images (*.png; *.jpg; *.tar.gz)".into()));
}

#[test]
fn local_control_request_survives_internal_availability_sync() {
    let mut app = fixture();
    let root = app
        .world_mut()
        .spawn_scene(bsn! {
            @WidgetryFileDialog Node { width: px(800), height: px(600) }
        })
        .unwrap()
        .id();
    loaded(&mut app, root, 10);
    app.update();
    let search = named(&mut app, root, "FileDialogSearch");
    app.world_mut()
        .entity_mut(search)
        .insert(InteractionDisabled);
    app.update();
    assert!(app.world().get::<InteractionDisabled>(search).is_some());
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
        let mut diagnostic_focus = Vec::new();
        for _ in 0..40 {
            press_key(&mut app, Entity::PLACEHOLDER, KeyCode::Tab);
            diagnostic_focus.push(app.world().resource::<InputFocus>().get().map(|focus| {
                (
                    focus,
                    app.world()
                        .get::<Name>(focus)
                        .map(|n| n.as_str().to_owned()),
                )
            }));
            if app.world().resource::<InputFocus>().get() == Some(field) {
                reached = true;
                break;
            }
        }
        assert!(
            reached,
            "Tab must reach the activatable ComboBox field: {diagnostic_focus:?}"
        );
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
fn sidebar_selection_matches_canonical_prefix_and_keeps_original_navigation_path() {
    for (location, canonical, selected) in [
        (r"C:\fixture", r"\\?\C:\fixture", true),
        (r"C:\", r"\\?\C:\", true),
        (
            r"\\server\share\fixture",
            r"\\?\UNC\server\share\fixture",
            true,
        ),
        (r"C:\other", r"\\?\C:\fixture", false),
        (r"D:\fixture", r"\\?\C:\fixture", false),
        (
            r"\\server\other\fixture",
            r"\\?\UNC\server\share\fixture",
            false,
        ),
        (r"C:\fixture", r"\\?\Volume{fixture}\fixture", false),
    ] {
        let mut app = fixture();
        switch_theme(&mut app, WidgetryThemeMode::Dark);
        let root = app
            .world_mut()
            .spawn_scene(bsn! { @WidgetryFileDialog Node {width:px(800),height:px(600)} })
            .unwrap()
            .id();
        loaded_at(
            &mut app,
            root,
            2,
            WidgetryFileDialogEntryKind::File,
            canonical.into(),
        );
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
                    name: "Location".into(),
                    path: location.into(),
                }]),
            },
        )
        .unwrap();
        for _ in 0..3 {
            app.update();
        }
        let row = named(&mut app, root, "FileDialogLocation");
        assert_eq!(
            app.world()
                .get::<bevy::a11y::AccessibilityNode>(row)
                .unwrap()
                .0
                .is_selected(),
            Some(selected)
        );
        if selected {
            let color = WidgetryThemeMode::Dark
                .colors()
                .file_dialog
                .entry
                .selected
                .background;
            assert_eq!(app.world().get::<BackgroundColor>(row).unwrap().0, color);
        }
        app.world_mut()
            .resource_mut::<InputFocus>()
            .set(row, FocusCause::Navigated);
        bevy_widgetry_test_utils::queue_key(
            &mut app,
            bevy::input::keyboard::KeyboardInput {
                key_code: KeyCode::Enter,
                logical_key: bevy::input::keyboard::Key::Enter,
                state: bevy::input::ButtonState::Pressed,
                text: None,
                repeat: false,
                window: Entity::PLACEHOLDER,
            },
        );
        app.update();
        assert_eq!(
            app.world()
                .get::<WidgetryFileDialogState>(root)
                .unwrap()
                .requested_path(),
            Some(PathBuf::from(location).as_path())
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
    for &row in &locations {
        let label = app
            .world_mut()
            .query::<(Entity, &Text)>()
            .iter(app.world())
            .find(|(entity, _)| belongs(app.world(), *entity, row))
            .unwrap()
            .0;
        let row_y = app
            .world()
            .get::<UiGlobalTransform>(row)
            .unwrap()
            .translation
            .y;
        let label_y = app
            .world()
            .get::<UiGlobalTransform>(label)
            .unwrap()
            .translation
            .y;
        let box_height = app.world().get::<ComputedNode>(label).unwrap().size().y;
        let text_height = app
            .world()
            .get::<bevy::text::TextLayoutInfo>(label)
            .unwrap()
            .size
            .y;
        assert!(
            (label_y - box_height / 2.0 + text_height / 2.0 - row_y).abs() < 0.5,
            "navigation text must be centered within its row"
        );
    }
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
fn queued_ime_reset_does_not_block_escape() {
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
    {
        let input = input_entity(app.world(), path);
        app.world_mut().get_mut::<EditableText>(input)
    }
    .unwrap()
    .queue_edit(bevy::text::TextEdit::clear_ime_compose());
    bevy_widgetry_test_utils::queue_key(
        &mut app,
        bevy::input::keyboard::KeyboardInput {
            key_code: KeyCode::Escape,
            logical_key: bevy::input::keyboard::Key::Escape,
            state: bevy::input::ButtonState::Pressed,
            text: None,
            repeat: false,
            window: Entity::PLACEHOLDER,
        },
    );
    app.update();
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
    {
        let input = input_entity(app.world(), path);
        app.world_mut().get_mut::<EditableText>(input)
    }
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
    use bevy_widgetry_asset::BuiltinFont;
    use bevy_widgetry_core::WidgetryAppExt;
    let mut app = fixture();
    let font = app
        .world()
        .resource::<AssetServer>()
        .load::<Font>(BuiltinFont::Default.path());
    bevy_widgetry_test_utils::advance_until(
        &mut app,
        std::time::Duration::from_secs(10),
        "FileDialog 编辑选择字体",
        |world| world.resource::<Assets<Font>>().contains(&font),
    )
    .unwrap();
    app.set_default_font(bevy::text::FontSource::Handle(font));
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
    bevy_widgetry_text_field::WidgetryTextField::set_text_in_world(
        app.world_mut(),
        path,
        "editing",
    )
    .unwrap();
    app.update();
    {
        let input = input_entity(app.world(), path);
        app.world_mut().get_mut::<EditableText>(input)
    }
    .unwrap()
    .queue_edit(bevy::text::TextEdit::SelectAll);
    app.update();
    let selection = app
        .world()
        .get::<EditableText>(input_entity(app.world(), path))
        .unwrap()
        .editor()
        .raw_selection()
        .text_range();
    assert_eq!(selection, 0..7);
    let session = app
        .world()
        .get::<WidgetryFileDialogState>(root)
        .unwrap()
        .token()
        .session;
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
        app.world()
            .get::<TextFont>(input_entity(app.world(), path))
            .unwrap()
            .font_size,
        bevy::text::FontSize::Px(18.0)
    );
    assert_eq!(
        app.world()
            .get::<EditableText>(input_entity(app.world(), path))
            .unwrap()
            .value()
            .to_string(),
        "editing"
    );
    assert_eq!(
        app.world()
            .get::<EditableText>(input_entity(app.world(), path))
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
fn save_validation_overwrite_no_and_cancel_keep_the_session_contract() {
    let mut app = fixture();
    let root = app.world_mut().spawn_scene(bsn! { @WidgetryFileDialog { @mode: WidgetryFileDialogMode::SaveFile } Node {width: px(800), height: px(600)} }).unwrap().id();
    loaded(&mut app, root, 30);
    for _ in 0..3 {
        app.update();
    }
    let filename = named(&mut app, root, "FileDialogFilename");
    bevy_widgetry_text_field::WidgetryTextField::set_text_in_world(
        app.world_mut(),
        filename,
        "output.txt",
    )
    .unwrap();
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
fn root_colors_route_to_managed_parts_and_new_rows_then_clear_to_their_own_theme() {
    let mut app = fixture();
    let mut colors = WidgetryFileDialogColorOverrides::default();
    let body = Color::linear_rgb(0.51, 0.12, 0.31);
    let field = Color::linear_rgb(0.13, 0.42, 0.21);
    let text = Color::linear_rgb(0.72, 0.21, 0.33);
    let button = Color::linear_rgb(0.23, 0.12, 0.65);
    let entry = Color::linear_rgb(0.61, 0.31, 0.17);
    colors.body.normal.background = Some(body);
    colors.path_field.editable.normal.background = Some(field);
    colors.path_field.editable.normal.foreground = Some(text);
    colors.cancel_button.normal.background = Some(button);
    colors.entry.normal.background = Some(entry);
    colors.entry.normal.foreground = Some(text);
    colors.status.normal.foreground = Some(text);
    let root = app
        .world_mut()
        .spawn_scene(bsn! {
            @WidgetryFileDialog { @colors: {colors.clone()} }
            Node { width: px(800), height: px(600) }
        })
        .unwrap()
        .id();
    loaded(&mut app, root, 100);
    for _ in 0..3 {
        app.update();
    }
    let path = named(&mut app, root, "FileDialogPath");
    let cancel = named(&mut app, root, "FileDialogCancel");
    let hidden = named(&mut app, root, "FileDialogHidden");
    let area = named(&mut app, root, "FileDialogEntries");
    let status = named(&mut app, root, "FileDialogStatus");
    assert_eq!(app.world().get::<BackgroundColor>(root).unwrap().0, body);
    assert_eq!(
        app.world()
            .get::<BackgroundColor>(frame_entity(app.world(), path))
            .unwrap()
            .0,
        field
    );
    assert_eq!(
        app.world()
            .get::<TextColor>(input_entity(app.world(), path))
            .unwrap()
            .0,
        text
    );
    assert_eq!(
        app.world().get::<BackgroundColor>(cancel).unwrap().0,
        button
    );
    assert_eq!(app.world().get::<TextColor>(status).unwrap().0, text);
    let logs = LogCapture::default();
    let errors = logs.run(|| {
        [
            bevy_widgetry_button::WidgetryButtonColorOverrides::set_in_world(
                app.world_mut(),
                cancel,
                Default::default(),
            )
            .unwrap_err(),
            bevy_widgetry_text_field::WidgetryTextFieldColorOverrides::clear_in_world(
                app.world_mut(),
                path,
            )
            .unwrap_err(),
            bevy_widgetry_check_box::WidgetryCheckBoxColorOverrides::clear_in_world(
                app.world_mut(),
                hidden,
            )
            .unwrap_err(),
            bevy_widgetry_scroll_area::WidgetryScrollAreaColorOverrides::clear_in_world(
                app.world_mut(),
                area,
            )
            .unwrap_err(),
        ]
    });
    for error in errors {
        assert_eq!(error.severity(), bevy::ecs::error::Severity::Error);
        assert!(error.to_string().contains("托管"));
    }
    assert_eq!(
        logs.records()
            .iter()
            .filter(|r| r.target == "bevy_widgetry" && r.level == bevy::log::Level::ERROR)
            .count(),
        4
    );
    assert_eq!(
        WidgetryFileDialogColorOverrides::get(app.world(), root).unwrap(),
        &colors
    );
    let rows = |app: &mut App| {
        app.world_mut()
            .query::<(Entity, &Name)>()
            .iter(app.world())
            .filter(|(entity, name)| {
                name.as_str() == "FileDialogEntryRow" && belongs(app.world(), *entity, root)
            })
            .map(|(entity, _)| entity)
            .collect::<Vec<_>>()
    };
    let before = rows(&mut app);
    assert!(!before.is_empty());
    assert!(app.world_mut().query::<(Entity, &Text)>().iter(app.world()).any(|(entity, text)| text.0 == "file_000000.txt" && belongs(app.world(), entity, root)));
    let viewport = entry_viewport(&mut app, root);
    app.world_mut()
        .get_mut::<bevy::ui::ScrollPosition>(viewport)
        .unwrap()
        .0
        .y = 1e9;
    for _ in 0..3 {
        app.update();
    }
    let after = rows(&mut app);
    assert!(!after.is_empty());
    assert!(app.world_mut().query::<(Entity, &Text)>().iter(app.world()).any(|(entity, text)| text.0 == "file_000099.txt" && belongs(app.world(), entity, root)));
    assert!(!app.world_mut().query::<(Entity, &Text)>().iter(app.world()).any(|(entity, text)| text.0 == "file_000000.txt" && belongs(app.world(), entity, root)));
    switch_theme(&mut app, WidgetryThemeMode::Light);
    for row in &after {
        assert_eq!(app.world().get::<BackgroundColor>(*row).unwrap().0, entry);
        let text_colors = app
            .world_mut()
            .query::<(Entity, &TextColor)>()
            .iter(app.world())
            .filter(|(entity, _)| belongs(app.world(), *entity, *row))
            .map(|(_, color)| color.0)
            .collect::<Vec<_>>();
        assert!(!text_colors.is_empty());
        assert!(text_colors.iter().all(|color| *color == text));
    }
    assert_eq!(app.world().get::<BackgroundColor>(root).unwrap().0, body);
    assert_eq!(
        app.world()
            .get::<BackgroundColor>(frame_entity(app.world(), path))
            .unwrap()
            .0,
        field
    );
    let token = app
        .world()
        .get::<WidgetryFileDialogState>(root)
        .unwrap()
        .token();
    assert!(WidgetryFileDialogColorOverrides::clear_in_world(app.world_mut(), root).unwrap());
    app.update();
    assert_eq!(
        app.world()
            .get::<WidgetryFileDialogState>(root)
            .unwrap()
            .token(),
        token
    );
    let theme = WidgetryThemeMode::Light.colors().file_dialog;
    assert_eq!(
        app.world().get::<BackgroundColor>(root).unwrap().0,
        theme.body.normal.background
    );
    assert_eq!(
        app.world()
            .get::<BackgroundColor>(frame_entity(app.world(), path))
            .unwrap()
            .0,
        theme.path_field.editable.normal.background
    );
    assert_eq!(
        app.world()
            .get::<TextColor>(input_entity(app.world(), path))
            .unwrap()
            .0,
        theme.path_field.editable.normal.foreground
    );
    assert_eq!(
        app.world().get::<BackgroundColor>(cancel).unwrap().0,
        theme.cancel_button.normal.background
    );
    assert_eq!(
        app.world().get::<TextColor>(status).unwrap().0,
        theme.status.normal.foreground
    );
    for row in rows(&mut app) {
        assert_eq!(
            app.world().get::<BackgroundColor>(row).unwrap().0,
            theme.entry.normal.background
        );
    }
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
    bevy_widgetry_text_field::WidgetryTextField::set_text_in_world(
        app.world_mut(),
        path,
        "C:/other",
    )
    .unwrap();
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
    bevy_widgetry_text_field::WidgetryTextField::set_text_in_world(
        app.world_mut(),
        editor,
        "hello",
    )
    .unwrap();
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
        .get::<EditableText>(input_entity(app.world(), editor))
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
    switch_theme(&mut app, WidgetryThemeMode::Light);
    app.update();
    assert_eq!(
        app.world()
            .get::<EditableText>(input_entity(app.world(), editor))
            .unwrap()
            .value()
            .to_string(),
        "hello"
    );
    assert_eq!(
        app.world()
            .get::<EditableText>(input_entity(app.world(), editor))
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
