//! Coverage Map：本文件负责 ECS API、deferred command、通知与 session/scope 隔离。
//! snapshot.rs 的 unit test 负责 identity/filter/sort，本文件负责 navigation/selection/save 的跨 state contract。
//! storage.rs 负责 mutation merge 与 persistence revision，model.rs 负责 result once-only。
//! state 维度为 session、directory、selection、confirmation 和 storage。
//! stimuli 为 BSN 构造、World API、Commands 与 backend reply。
//! guard 为有效 root/session/revision 和 mode/kind，observer 必须读取已提交 authority。
//! 相同操作不重复通知，错误不改变 state，scope merge 不回滚其它 session 的偏好。

// 测试允许断言，workspace 的 macro 禁令只约束生产代码。
#![allow(clippy::disallowed_macros)]

use bevy::ecs::error::Severity;
use bevy::prelude::*;
use bevy_widgetry_file_dialog::*;
use bevy_widgetry_test_utils::{ErrorCapture, LogCapture, scene_app};
use proptest::prelude::*;
use std::sync::Arc;

#[derive(Resource, Default)]
struct Results(Vec<WidgetryFileDialogResult>);

// 测试 fixture 不属于生产代码，捕获 observer 可以使用 unwrap 校验必需 state。
#[allow(clippy::unwrap_used)]
fn app() -> App {
    let mut app = scene_app();
    app.insert_resource(WidgetryFileDialogRuntimeOptions {
        automatic: false,
        ..Default::default()
    });
    app.add_plugins(WidgetryFileDialogHeadlessPlugin);
    app.init_resource::<Results>();
    app.add_observer(
        |event: On<WidgetryFileDialogResultEvent>,
         states: Query<&WidgetryFileDialogState>,
         mut results: ResMut<Results>| {
            assert_eq!(
                states.get(event.entity).unwrap().result(),
                Some(&event.result)
            );
            results.0.push(event.result.clone());
        },
    );
    app
}

// 测试 fixture 构造失败必须立即终止当前测试，允许使用 unwrap。
#[allow(clippy::unwrap_used)]
fn dialog(app: &mut App, props: WidgetryFileDialogProps) -> Entity {
    app.world_mut().spawn_scene(bsn! { @WidgetryFileDialog { @mode: {props.mode},
        @initial_directory: {props.initial_directory}, @fallback_directory: {props.fallback_directory},
        @storage_scope: {props.storage_scope}, @filters: {props.filters},
        @allow_different_extension: {props.allow_different_extension} } }).unwrap().id()
}

#[test]
fn cancel_commits_before_notification_and_resolves_once() {
    let mut app = app();
    let root = dialog(&mut app, WidgetryFileDialogProps::default());
    assert!(
        WidgetryFileDialog::apply(app.world_mut(), root, WidgetryFileDialogAction::Cancel).unwrap()
    );
    assert!(
        !WidgetryFileDialog::apply(app.world_mut(), root, WidgetryFileDialogAction::Cancel)
            .unwrap()
    );
    assert_eq!(
        app.world().resource::<Results>().0,
        [WidgetryFileDialogResult::Cancelled]
    );
}

#[test]
fn invalid_root_is_error_without_notification() {
    let mut app = app();
    let logs = LogCapture::default();
    let root = app.world_mut().spawn_empty().id();
    let error = logs
        .run(|| WidgetryFileDialog::apply(app.world_mut(), root, WidgetryFileDialogAction::Cancel))
        .unwrap_err();
    assert_eq!(error.severity(), Severity::Error);
    assert!(error.to_string().contains("invalid FileDialog root"));
    assert_eq!(logs.records().len(), 1);
    assert!(app.world().resource::<Results>().0.is_empty());
}

#[test]
fn deferred_command_waits_for_apply_and_reports_error_to_host() {
    let mut app = app();
    app.set_error_handler(ErrorCapture::handler());
    let root = dialog(&mut app, WidgetryFileDialogProps::default());
    let errors = ErrorCapture::default();
    WidgetryFileDialog::queue(
        &mut app.world_mut().commands(),
        root,
        WidgetryFileDialogAction::Cancel,
    );
    WidgetryFileDialog::queue(
        &mut app.world_mut().commands(),
        Entity::PLACEHOLDER,
        WidgetryFileDialogAction::Cancel,
    );
    assert_eq!(
        app.world()
            .get::<WidgetryFileDialogState>(root)
            .unwrap()
            .result(),
        None
    );
    errors.run(|| app.world_mut().flush());
    assert_eq!(app.world().resource::<Results>().0.len(), 1);
    assert_eq!(errors.take()[0].severity(), Severity::Error);
}

#[test]
fn navigation_commits_history_only_on_started_and_rejects_stale_reply() {
    let mut app = app();
    let root = dialog(
        &mut app,
        WidgetryFileDialogProps {
            initial_directory: Some("C:/one".into()),
            ..default()
        },
    );
    let token = app
        .world()
        .get::<WidgetryFileDialogState>(root)
        .unwrap()
        .token();
    assert!(
        WidgetryFileDialog::deliver(
            app.world_mut(),
            root,
            WidgetryFileDialogReply::Started {
                token,
                path: "C:/one".into()
            }
        )
        .unwrap()
    );
    assert!(
        WidgetryFileDialog::apply(
            app.world_mut(),
            root,
            WidgetryFileDialogAction::Navigate(WidgetryFileDialogNavigation::Path("../two".into()))
        )
        .unwrap()
    );
    let state = app.world().get::<WidgetryFileDialogState>(root).unwrap();
    assert_eq!(
        state.requested_path(),
        Some(std::path::Path::new("C:/one/../two"))
    );
    assert_eq!(state.history(), [std::path::PathBuf::from("C:/one")]);
    assert!(state.entries().is_empty());
    assert!(
        !WidgetryFileDialog::deliver(
            app.world_mut(),
            root,
            WidgetryFileDialogReply::Started {
                token,
                path: "C:/late".into()
            }
        )
        .unwrap()
    );
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
            error: "not found".into(),
        },
    )
    .unwrap();
    assert_eq!(
        app.world()
            .get::<WidgetryFileDialogState>(root)
            .unwrap()
            .history()
            .len(),
        1
    );
}

// 测试 fixture 的 backend 准备失败必须终止当前测试，允许使用 unwrap。
#[allow(clippy::unwrap_used)]
fn loaded(
    app: &mut App,
    root: Entity,
    names: &[(&str, WidgetryFileDialogEntryKind)],
) -> Arc<WidgetryFileDialogSnapshot> {
    let state = app.world().get::<WidgetryFileDialogState>(root).unwrap();
    let token = state.token();
    let query = state.query();
    let previous = state.snapshot().cloned();
    let path = std::path::PathBuf::from("C:/fixture");
    let data = names
        .iter()
        .map(|(name, kind)| WidgetryFileDialogEntryData {
            path: path.join(name),
            name: (*name).into(),
            kind: *kind,
            size: None,
            modified: None,
            hidden: Some(false),
            system: Some(false),
        })
        .collect();
    let snapshot = Arc::new(
        WidgetryFileDialogSnapshot::prepare(token, path.clone(), data, &query, previous.as_deref())
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
            snapshot: snapshot.clone(),
            state: WidgetryFileDialogDirectoryState::Ready,
            selection,
        },
    )
    .unwrap();
    snapshot
}

// 测试 fixture 的 selection completion 失败必须终止当前测试，允许使用 unwrap。
#[allow(clippy::unwrap_used)]
fn complete_selection(app: &mut App, root: Entity) {
    let job = app
        .world()
        .get::<WidgetryFileDialogState>(root)
        .unwrap()
        .selection_job()
        .unwrap();
    WidgetryFileDialog::deliver(
        app.world_mut(),
        root,
        WidgetryFileDialogReply::Selection(job.prepare().unwrap()),
    )
    .unwrap();
}

#[test]
fn bulk_intents_merge_in_order_and_stale_selection_does_not_apply() {
    let mut app = app();
    let root = dialog(
        &mut app,
        WidgetryFileDialogProps {
            mode: WidgetryFileDialogMode::PickFiles,
            ..default()
        },
    );
    let snapshot = loaded(
        &mut app,
        root,
        &[
            ("a.txt", WidgetryFileDialogEntryKind::File),
            ("b.txt", WidgetryFileDialogEntryKind::File),
            ("folder", WidgetryFileDialogEntryKind::Directory),
        ],
    );
    WidgetryFileDialog::apply(app.world_mut(), root, WidgetryFileDialogAction::SelectAll).unwrap();
    let job = app
        .world()
        .get::<WidgetryFileDialogState>(root)
        .unwrap()
        .selection_job()
        .unwrap();
    assert!(
        app.world()
            .get::<WidgetryFileDialogState>(root)
            .unwrap()
            .selected()
            .is_empty()
    );
    let first = snapshot
        .entries()
        .iter()
        .find(|entry| entry.name() == "a.txt")
        .unwrap()
        .id();
    WidgetryFileDialog::apply(
        app.world_mut(),
        root,
        WidgetryFileDialogAction::Select {
            id: first,
            operation: WidgetryFileDialogSelection::Toggle,
            token: snapshot.token(),
        },
    )
    .unwrap();
    assert!(
        !WidgetryFileDialog::deliver(
            app.world_mut(),
            root,
            WidgetryFileDialogReply::Selection(job.prepare().unwrap())
        )
        .unwrap()
    );
    complete_selection(&mut app, root);
    let state = app.world().get::<WidgetryFileDialogState>(root).unwrap();
    assert_eq!(state.selected().len(), 1);
    assert!(!state.selected().contains(&first));
    assert_eq!(state.active(), Some(first));
}

#[test]
fn query_change_removes_invisible_selection_and_same_search_is_noop() {
    let mut app = app();
    let root = dialog(
        &mut app,
        WidgetryFileDialogProps {
            mode: WidgetryFileDialogMode::PickFiles,
            ..default()
        },
    );
    loaded(
        &mut app,
        root,
        &[
            ("a.txt", WidgetryFileDialogEntryKind::File),
            ("b.txt", WidgetryFileDialogEntryKind::File),
        ],
    );
    WidgetryFileDialog::apply(app.world_mut(), root, WidgetryFileDialogAction::SelectAll).unwrap();
    complete_selection(&mut app, root);
    assert!(
        WidgetryFileDialog::apply(
            app.world_mut(),
            root,
            WidgetryFileDialogAction::Search("b".into())
        )
        .unwrap()
    );
    assert!(
        !WidgetryFileDialog::apply(
            app.world_mut(),
            root,
            WidgetryFileDialogAction::Search("b".into())
        )
        .unwrap()
    );
    assert!(
        app.world()
            .get::<WidgetryFileDialogState>(root)
            .unwrap()
            .projection_pending()
    );
    loaded(
        &mut app,
        root,
        &[
            ("a.txt", WidgetryFileDialogEntryKind::File),
            ("b.txt", WidgetryFileDialogEntryKind::File),
        ],
    );
    let state = app.world().get::<WidgetryFileDialogState>(root).unwrap();
    assert_eq!(state.selected().len(), 1);
    assert_eq!(state.visible().len(), 1);
    assert_eq!(
        state
            .snapshot()
            .unwrap()
            .entry(*state.selected().first().unwrap())
            .unwrap()
            .name(),
        "b.txt"
    );
}

#[test]
fn filename_edit_invalidates_overwrite_and_decline_keeps_session_open() {
    let mut app = app();
    let root = dialog(
        &mut app,
        WidgetryFileDialogProps {
            mode: WidgetryFileDialogMode::SaveFile,
            filters: vec![WidgetryFileDialogFilter {
                id: WidgetryFileDialogFilterId("text".into()),
                label: "Text".into(),
                suffixes: vec!["txt".into()],
                default_extension: Some("txt".into()),
            }],
            ..default()
        },
    );
    loaded(&mut app, root, &[]);
    assert!(
        WidgetryFileDialog::apply(
            app.world_mut(),
            root,
            WidgetryFileDialogAction::Filename("report".into())
        )
        .unwrap()
    );
    assert!(
        WidgetryFileDialog::apply(app.world_mut(), root, WidgetryFileDialogAction::Confirm)
            .unwrap()
    );
    assert_eq!(
        app.world()
            .get::<WidgetryFileDialogState>(root)
            .unwrap()
            .confirmation(),
        &WidgetryFileDialogConfirmation::Validating
    );
    let candidate = app
        .world()
        .get::<WidgetryFileDialogState>(root)
        .unwrap()
        .validation_job()
        .unwrap()
        .prepare()
        .unwrap();
    assert_eq!(
        candidate.result(),
        &WidgetryFileDialogResult::SavePath("C:/fixture/report.txt".into())
    );
    let token = candidate.token();
    WidgetryFileDialog::deliver(
        app.world_mut(),
        root,
        WidgetryFileDialogReply::Validated {
            candidate,
            exists: true,
        },
    )
    .unwrap();
    WidgetryFileDialog::apply(
        app.world_mut(),
        root,
        WidgetryFileDialogAction::Overwrite {
            token,
            accept: false,
        },
    )
    .unwrap();
    assert_eq!(
        app.world()
            .get::<WidgetryFileDialogState>(root)
            .unwrap()
            .session_state(),
        WidgetryFileDialogSessionState::Open
    );
    WidgetryFileDialog::apply(app.world_mut(), root, WidgetryFileDialogAction::Confirm).unwrap();
    let stale = app
        .world()
        .get::<WidgetryFileDialogState>(root)
        .unwrap()
        .validation_job()
        .unwrap()
        .prepare()
        .unwrap();
    WidgetryFileDialog::apply(
        app.world_mut(),
        root,
        WidgetryFileDialogAction::Filename("new.md".into()),
    )
    .unwrap();
    assert!(
        !WidgetryFileDialog::deliver(
            app.world_mut(),
            root,
            WidgetryFileDialogReply::Validated {
                candidate: stale,
                exists: true
            }
        )
        .unwrap()
    );
    assert!(
        WidgetryFileDialog::apply(
            app.world_mut(),
            root,
            WidgetryFileDialogAction::Overwrite {
                token,
                accept: true
            }
        )
        .is_err()
    );
    WidgetryFileDialog::apply(app.world_mut(), root, WidgetryFileDialogAction::Confirm).unwrap();
    let candidate = app
        .world()
        .get::<WidgetryFileDialogState>(root)
        .unwrap()
        .validation_job()
        .unwrap()
        .prepare()
        .unwrap();
    WidgetryFileDialog::deliver(
        app.world_mut(),
        root,
        WidgetryFileDialogReply::Validated {
            candidate,
            exists: false,
        },
    )
    .unwrap();
    assert_eq!(
        app.world().resource::<Results>().0,
        [WidgetryFileDialogResult::SavePath(
            "C:/fixture/new.md".into()
        )]
    );
}

#[test]
fn shared_scope_merges_deltas_and_cancel_does_not_update_picked_directory() {
    let mut app = app();
    let props = WidgetryFileDialogProps {
        storage_scope: Some("open".into()),
        ..default()
    };
    let first = dialog(&mut app, props.clone());
    let second = dialog(&mut app, props.clone());
    loaded(&mut app, first, &[]);
    loaded(&mut app, second, &[]);
    WidgetryFileDialog::apply(
        app.world_mut(),
        first,
        WidgetryFileDialogAction::Pin("C:/one".into()),
    )
    .unwrap();
    WidgetryFileDialog::apply(
        app.world_mut(),
        second,
        WidgetryFileDialogAction::Pin("C:/two".into()),
    )
    .unwrap();
    WidgetryFileDialog::apply(
        app.world_mut(),
        first,
        WidgetryFileDialogAction::ShowHidden(true),
    )
    .unwrap();
    WidgetryFileDialog::apply(
        app.world_mut(),
        second,
        WidgetryFileDialogAction::ShowSystem(true),
    )
    .unwrap();
    WidgetryFileDialog::apply(app.world_mut(), first, WidgetryFileDialogAction::Cancel).unwrap();
    let store = app.world().resource::<WidgetryFileDialogStorage>();
    let snapshot = store.snapshot("open").unwrap();
    assert_eq!(
        snapshot.pinned,
        [
            std::path::PathBuf::from("C:/one"),
            std::path::PathBuf::from("C:/two")
        ]
    );
    assert!(snapshot.show_hidden && snapshot.show_system);
    assert_eq!(snapshot.last_picked_dir, None);
    let third = dialog(&mut app, props);
    let state = app.world().get::<WidgetryFileDialogState>(third).unwrap();
    assert_eq!(state.preferences().pinned.len(), 2);
    assert_eq!(
        state.requested_path(),
        Some(std::path::Path::new("C:/fixture"))
    );
}

#[test]
fn activation_is_independent_of_selection_and_folder_cancel_is_not_a_result() {
    let mut app = app();
    let root = dialog(&mut app, WidgetryFileDialogProps::default());
    let snapshot = loaded(
        &mut app,
        root,
        &[("folder", WidgetryFileDialogEntryKind::Directory)],
    );
    let id = snapshot.visible()[0];
    assert!(
        WidgetryFileDialog::apply(
            app.world_mut(),
            root,
            WidgetryFileDialogAction::Activate {
                id,
                token: snapshot.token()
            }
        )
        .unwrap()
    );
    assert_eq!(
        app.world()
            .get::<WidgetryFileDialogState>(root)
            .unwrap()
            .requested_path(),
        Some(std::path::Path::new("C:/fixture/folder"))
    );
    assert!(app.world().resource::<Results>().0.is_empty());
}

#[test]
fn new_folder_is_only_an_admission_until_backend_completion() {
    let mut app = app();
    let root = dialog(&mut app, WidgetryFileDialogProps::default());
    loaded(&mut app, root, &[]);
    assert!(
        WidgetryFileDialog::apply(
            app.world_mut(),
            root,
            WidgetryFileDialogAction::NewFolder("created".into())
        )
        .unwrap()
    );
    assert!(app.world().resource::<Results>().0.is_empty());
    assert_eq!(
        app.world()
            .get::<WidgetryFileDialogState>(root)
            .unwrap()
            .directory_state(),
        &WidgetryFileDialogDirectoryState::Ready
    );
}

#[test]
fn single_directory_uses_current_but_multiple_directory_requires_selection() {
    let mut app = app();
    let single = dialog(
        &mut app,
        WidgetryFileDialogProps {
            mode: WidgetryFileDialogMode::PickDirectory,
            ..default()
        },
    );
    loaded(&mut app, single, &[]);
    WidgetryFileDialog::apply(app.world_mut(), single, WidgetryFileDialogAction::Confirm).unwrap();
    let candidate = app
        .world()
        .get::<WidgetryFileDialogState>(single)
        .unwrap()
        .validation_job()
        .unwrap()
        .prepare()
        .unwrap();
    WidgetryFileDialog::deliver(
        app.world_mut(),
        single,
        WidgetryFileDialogReply::Validated {
            candidate,
            exists: true,
        },
    )
    .unwrap();
    assert_eq!(
        app.world().resource::<Results>().0,
        [WidgetryFileDialogResult::Directory("C:/fixture".into())]
    );
    let multiple = dialog(
        &mut app,
        WidgetryFileDialogProps {
            mode: WidgetryFileDialogMode::PickDirectories,
            ..default()
        },
    );
    loaded(&mut app, multiple, &[]);
    assert!(
        WidgetryFileDialog::apply(app.world_mut(), multiple, WidgetryFileDialogAction::Confirm)
            .is_err()
    );
}

#[test]
fn result_order_follows_visible_snapshot_and_observer_reads_final_authority() {
    let mut app = app();
    let root = dialog(
        &mut app,
        WidgetryFileDialogProps {
            mode: WidgetryFileDialogMode::PickFiles,
            ..default()
        },
    );
    loaded(
        &mut app,
        root,
        &[
            ("z.txt", WidgetryFileDialogEntryKind::File),
            ("a.txt", WidgetryFileDialogEntryKind::SymlinkFile),
            ("unknown", WidgetryFileDialogEntryKind::Unknown),
        ],
    );
    WidgetryFileDialog::apply(app.world_mut(), root, WidgetryFileDialogAction::SelectAll).unwrap();
    assert!(
        WidgetryFileDialog::apply(app.world_mut(), root, WidgetryFileDialogAction::Confirm)
            .is_err()
    );
    complete_selection(&mut app, root);
    assert!(
        !WidgetryFileDialog::apply(app.world_mut(), root, WidgetryFileDialogAction::SelectAll)
            .unwrap()
    );
    WidgetryFileDialog::apply(app.world_mut(), root, WidgetryFileDialogAction::Confirm).unwrap();
    let candidate = app
        .world()
        .get::<WidgetryFileDialogState>(root)
        .unwrap()
        .validation_job()
        .unwrap()
        .prepare()
        .unwrap();
    WidgetryFileDialog::deliver(
        app.world_mut(),
        root,
        WidgetryFileDialogReply::Validated {
            candidate: candidate.clone(),
            exists: true,
        },
    )
    .unwrap();
    assert!(
        !WidgetryFileDialog::deliver(
            app.world_mut(),
            root,
            WidgetryFileDialogReply::Validated {
                candidate,
                exists: true
            }
        )
        .unwrap()
    );
    assert!(
        !WidgetryFileDialog::apply(app.world_mut(), root, WidgetryFileDialogAction::Confirm)
            .unwrap()
    );
    assert_eq!(
        app.world().resource::<Results>().0,
        [WidgetryFileDialogResult::Files(
            vec![
                std::path::PathBuf::from("C:/fixture/a.txt"),
                std::path::PathBuf::from("C:/fixture/z.txt")
            ]
            .into()
        )]
    );
}

#[test]
fn ctrl_a_does_not_absorb_future_entries_and_queued_navigation_accumulates() {
    let mut app = app();
    let root = dialog(
        &mut app,
        WidgetryFileDialogProps {
            mode: WidgetryFileDialogMode::PickFiles,
            ..default()
        },
    );
    loaded(
        &mut app,
        root,
        &[("a.txt", WidgetryFileDialogEntryKind::File)],
    );
    WidgetryFileDialog::apply(app.world_mut(), root, WidgetryFileDialogAction::SelectAll).unwrap();
    loaded(
        &mut app,
        root,
        &[
            ("a.txt", WidgetryFileDialogEntryKind::File),
            ("b.txt", WidgetryFileDialogEntryKind::File),
            ("c.txt", WidgetryFileDialogEntryKind::File),
        ],
    );
    assert_eq!(
        app.world()
            .get::<WidgetryFileDialogState>(root)
            .unwrap()
            .selected()
            .len(),
        1
    );
    for _ in 0..3 {
        WidgetryFileDialog::apply(
            app.world_mut(),
            root,
            WidgetryFileDialogAction::MoveActive(WidgetryFileDialogMove::Next),
        )
        .unwrap();
    }
    complete_selection(&mut app, root);
    let state = app.world().get::<WidgetryFileDialogState>(root).unwrap();
    assert_eq!(
        state
            .snapshot()
            .unwrap()
            .entry(state.active().unwrap())
            .unwrap()
            .name(),
        "c.txt"
    );
}

#[test]
fn storage_capacity_failure_does_not_rollback_result_or_cancel() {
    let mut app = app();
    for index in 0..256 {
        app.world_mut()
            .resource_mut::<WidgetryFileDialogStorage>()
            .import(
                format!("scope{index}"),
                WidgetryFileDialogStorageSnapshot::default(),
            )
            .unwrap();
    }
    let root = dialog(
        &mut app,
        WidgetryFileDialogProps {
            mode: WidgetryFileDialogMode::PickDirectory,
            storage_scope: Some("new".into()),
            ..default()
        },
    );
    loaded(&mut app, root, &[]);
    WidgetryFileDialog::apply(app.world_mut(), root, WidgetryFileDialogAction::Confirm).unwrap();
    let candidate = app
        .world()
        .get::<WidgetryFileDialogState>(root)
        .unwrap()
        .validation_job()
        .unwrap()
        .prepare()
        .unwrap();
    WidgetryFileDialog::deliver(
        app.world_mut(),
        root,
        WidgetryFileDialogReply::Validated {
            candidate,
            exists: true,
        },
    )
    .unwrap();
    assert_eq!(app.world().resource::<Results>().0.len(), 1);
    assert!(matches!(
        app.world()
            .get::<WidgetryFileDialogState>(root)
            .unwrap()
            .storage_state(),
        WidgetryFileDialogStorageState::Failed(_)
    ));
    let cancel = dialog(
        &mut app,
        WidgetryFileDialogProps {
            storage_scope: Some("another".into()),
            ..default()
        },
    );
    WidgetryFileDialog::apply(app.world_mut(), cancel, WidgetryFileDialogAction::Cancel).unwrap();
    assert_eq!(app.world().resource::<Results>().0.len(), 2);
}

proptest! {
    #[test]
    fn navigation_selection_query_sequences_preserve_identity_and_result_uniqueness(operations in prop::collection::vec(0u8..8, 1..60)) {
        let mut app = app();
        let root = dialog(&mut app, WidgetryFileDialogProps { mode: WidgetryFileDialogMode::PickFiles, ..default() });
        let names = [("a.txt", WidgetryFileDialogEntryKind::File), ("b.png", WidgetryFileDialogEntryKind::File),
            ("c.txt", WidgetryFileDialogEntryKind::File), ("folder", WidgetryFileDialogEntryKind::Directory)];
        loaded(&mut app, root, &names);
        for operation in operations {
            let state = app.world().get::<WidgetryFileDialogState>(root).unwrap();
            let id = state.visible().iter().find(|id| state.snapshot().unwrap().entry(**id).unwrap().kind().is_file()).copied();
            let token = state.token();
            let action = match operation {
                0 => id.map(|id| WidgetryFileDialogAction::Select { id, token, operation: WidgetryFileDialogSelection::Toggle }),
                1 => Some(WidgetryFileDialogAction::SelectAll),
                2 => id.map(|id| WidgetryFileDialogAction::Select { id, token, operation: WidgetryFileDialogSelection::Replace }),
                3 => Some(WidgetryFileDialogAction::ClearSelection),
                4 => Some(WidgetryFileDialogAction::Search("a".into())),
                5 => Some(WidgetryFileDialogAction::Search(String::new())),
                6 => Some(WidgetryFileDialogAction::Sort(WidgetryFileDialogSort::NameDescending)),
                _ => Some(WidgetryFileDialogAction::Navigate(WidgetryFileDialogNavigation::Refresh)),
            };
            if let Some(action) = action { WidgetryFileDialog::apply(app.world_mut(), root, action).unwrap(); }
            if app.world().get::<WidgetryFileDialogState>(root).unwrap().selection_pending() { complete_selection(&mut app, root); }
            loaded(&mut app, root, &names);
            let state = app.world().get::<WidgetryFileDialogState>(root).unwrap();
            prop_assert!(state.selected().iter().all(|id| state.visible().contains(id)));
            prop_assert!(state.selected().iter().all(|id| state.mode().accepts(state.snapshot().unwrap().entry(*id).unwrap().kind())));
            prop_assert!(state.anchor().is_none_or(|id| state.visible().contains(&id)));
            prop_assert!(app.world().resource::<Results>().0.is_empty());
        }
        WidgetryFileDialog::apply(app.world_mut(), root, WidgetryFileDialogAction::Cancel).unwrap();
        WidgetryFileDialog::apply(app.world_mut(), root, WidgetryFileDialogAction::Cancel).unwrap();
        prop_assert_eq!(app.world().resource::<Results>().0.len(), 1);
    }
}

#[test]
fn scene_patch_keeps_runtime_state_and_reopen_rejects_previous_session_reply() {
    let mut app = app();
    let root = dialog(&mut app, WidgetryFileDialogProps::default());
    loaded(&mut app, root, &[]);
    WidgetryFileDialog::apply(
        app.world_mut(),
        root,
        WidgetryFileDialogAction::Search("kept".into()),
    )
    .unwrap();
    let token = app
        .world()
        .get::<WidgetryFileDialogState>(root)
        .unwrap()
        .token();
    app.world_mut()
        .entity_mut(root)
        .apply_scene(bsn! { @WidgetryFileDialog { @mode: WidgetryFileDialogMode::SaveFile } })
        .unwrap();
    let state = app.world().get::<WidgetryFileDialogState>(root).unwrap();
    assert_eq!(state.search(), "kept");
    assert_eq!(state.token(), token);
    assert_eq!(state.mode(), WidgetryFileDialogMode::PickFile);
    WidgetryFileDialog::apply(app.world_mut(), root, WidgetryFileDialogAction::Cancel).unwrap();
    WidgetryFileDialog::apply(app.world_mut(), root, WidgetryFileDialogAction::Reopen).unwrap();
    assert_ne!(
        app.world()
            .get::<WidgetryFileDialogState>(root)
            .unwrap()
            .token()
            .session,
        token.session
    );
    assert!(
        !WidgetryFileDialog::deliver(
            app.world_mut(),
            root,
            WidgetryFileDialogReply::Started {
                token,
                path: "C:/old".into()
            }
        )
        .unwrap()
    );
}

#[test]
fn loaded_selection_can_validate_while_directory_is_still_streaming() {
    let mut app = app();
    let root = dialog(&mut app, WidgetryFileDialogProps::default());
    let snapshot = loaded(
        &mut app,
        root,
        &[("a.txt", WidgetryFileDialogEntryKind::File)],
    );
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
            snapshot: snapshot.clone(),
            state: WidgetryFileDialogDirectoryState::Loading,
            selection,
        },
    )
    .unwrap();
    WidgetryFileDialog::apply(
        app.world_mut(),
        root,
        WidgetryFileDialogAction::Select {
            id: snapshot.visible()[0],
            token: snapshot.token(),
            operation: WidgetryFileDialogSelection::Replace,
        },
    )
    .unwrap();
    complete_selection(&mut app, root);
    assert!(
        WidgetryFileDialog::apply(app.world_mut(), root, WidgetryFileDialogAction::Confirm)
            .unwrap()
    );
    assert_eq!(
        app.world()
            .get::<WidgetryFileDialogState>(root)
            .unwrap()
            .directory_state(),
        &WidgetryFileDialogDirectoryState::Loading
    );
}

#[test]
fn shift_range_anchor_repair_and_invalid_kind_keep_selection_contract() {
    let mut app = app();
    let root = dialog(
        &mut app,
        WidgetryFileDialogProps {
            mode: WidgetryFileDialogMode::PickFiles,
            ..default()
        },
    );
    let snapshot = loaded(
        &mut app,
        root,
        &[
            ("a.txt", WidgetryFileDialogEntryKind::File),
            ("b.txt", WidgetryFileDialogEntryKind::File),
            ("c.txt", WidgetryFileDialogEntryKind::File),
            ("folder", WidgetryFileDialogEntryKind::Directory),
        ],
    );
    let first = snapshot
        .entries()
        .iter()
        .find(|entry| entry.name() == "a.txt")
        .unwrap()
        .id();
    let last = snapshot
        .entries()
        .iter()
        .find(|entry| entry.name() == "c.txt")
        .unwrap()
        .id();
    WidgetryFileDialog::apply(
        app.world_mut(),
        root,
        WidgetryFileDialogAction::Select {
            id: first,
            token: snapshot.token(),
            operation: WidgetryFileDialogSelection::Replace,
        },
    )
    .unwrap();
    complete_selection(&mut app, root);
    WidgetryFileDialog::apply(
        app.world_mut(),
        root,
        WidgetryFileDialogAction::Select {
            id: last,
            token: snapshot.token(),
            operation: WidgetryFileDialogSelection::Range,
        },
    )
    .unwrap();
    complete_selection(&mut app, root);
    assert_eq!(
        app.world()
            .get::<WidgetryFileDialogState>(root)
            .unwrap()
            .selected()
            .len(),
        3
    );
    let logs = LogCapture::default();
    let folder = snapshot
        .entries()
        .iter()
        .find(|entry| entry.kind().is_directory())
        .unwrap()
        .id();
    let before = app
        .world()
        .get::<WidgetryFileDialogState>(root)
        .unwrap()
        .token();
    let error = logs
        .run(|| {
            WidgetryFileDialog::apply(
                app.world_mut(),
                root,
                WidgetryFileDialogAction::Select {
                    id: folder,
                    token: snapshot.token(),
                    operation: WidgetryFileDialogSelection::Replace,
                },
            )
        })
        .unwrap_err();
    assert_eq!(error.severity(), Severity::Error);
    assert_eq!(logs.records().len(), 1);
    assert_eq!(
        app.world()
            .get::<WidgetryFileDialogState>(root)
            .unwrap()
            .token(),
        before
    );
    loaded(
        &mut app,
        root,
        &[("c.txt", WidgetryFileDialogEntryKind::File)],
    );
    let state = app.world().get::<WidgetryFileDialogState>(root).unwrap();
    assert_eq!(state.anchor(), None);
    assert_eq!(state.selected().len(), 1);
    assert!(
        WidgetryFileDialog::apply(
            app.world_mut(),
            root,
            WidgetryFileDialogAction::Select {
                id: first,
                token: snapshot.token(),
                operation: WidgetryFileDialogSelection::Replace
            }
        )
        .is_err()
    );
}

#[test]
fn folder_result_retries_recovers_and_cancelled_root_ignores_late_completion() {
    let mut app = app();
    let root = dialog(&mut app, WidgetryFileDialogProps::default());
    loaded(&mut app, root, &[]);
    WidgetryFileDialog::apply(
        app.world_mut(),
        root,
        WidgetryFileDialogAction::NewFolder("created".into()),
    )
    .unwrap();
    let request = app
        .world()
        .get::<WidgetryFileDialogState>(root)
        .unwrap()
        .folder_request()
        .unwrap();
    WidgetryFileDialog::deliver(
        app.world_mut(),
        root,
        WidgetryFileDialogReply::FolderCreated {
            request,
            outcome: Err("permission denied".into()),
        },
    )
    .unwrap();
    assert_eq!(
        app.world()
            .get::<WidgetryFileDialogState>(root)
            .unwrap()
            .error(),
        Some("permission denied")
    );
    WidgetryFileDialog::apply(
        app.world_mut(),
        root,
        WidgetryFileDialogAction::NewFolder("created".into()),
    )
    .unwrap();
    let request = app
        .world()
        .get::<WidgetryFileDialogState>(root)
        .unwrap()
        .folder_request()
        .unwrap();
    WidgetryFileDialog::deliver(
        app.world_mut(),
        root,
        WidgetryFileDialogReply::FolderCreated {
            request: request.clone(),
            outcome: Ok(()),
        },
    )
    .unwrap();
    loaded(
        &mut app,
        root,
        &[("created", WidgetryFileDialogEntryKind::Directory)],
    );
    let state = app.world().get::<WidgetryFileDialogState>(root).unwrap();
    assert_eq!(
        state
            .snapshot()
            .unwrap()
            .entry(state.active().unwrap())
            .unwrap()
            .name(),
        "created"
    );
    assert!(state.selected().is_empty());
    WidgetryFileDialog::apply(app.world_mut(), root, WidgetryFileDialogAction::Cancel).unwrap();
    app.world_mut().despawn(root);
    assert!(
        !WidgetryFileDialog::deliver(
            app.world_mut(),
            root,
            WidgetryFileDialogReply::FolderCreated {
                request,
                outcome: Ok(())
            }
        )
        .unwrap()
    );
    assert_eq!(
        app.world().resource::<Results>().0,
        [WidgetryFileDialogResult::Cancelled]
    );
}

#[test]
fn scope_recent_directory_and_pin_operations_follow_actual_commit_order() {
    let mut app = app();
    let props = WidgetryFileDialogProps {
        mode: WidgetryFileDialogMode::PickDirectory,
        storage_scope: Some("shared".into()),
        ..default()
    };
    let first = dialog(&mut app, props.clone());
    let second = dialog(&mut app, props);
    loaded(&mut app, first, &[]);
    loaded(&mut app, second, &[]);
    WidgetryFileDialog::apply(
        app.world_mut(),
        first,
        WidgetryFileDialogAction::Pin("C:/pin".into()),
    )
    .unwrap();
    WidgetryFileDialog::apply(
        app.world_mut(),
        second,
        WidgetryFileDialogAction::Pin("C:/pin".into()),
    )
    .unwrap();
    WidgetryFileDialog::apply(
        app.world_mut(),
        second,
        WidgetryFileDialogAction::Unpin("C:/pin".into()),
    )
    .unwrap();
    assert!(
        WidgetryFileDialog::apply(
            app.world_mut(),
            first,
            WidgetryFileDialogAction::Pin("C:/pin".into())
        )
        .unwrap()
    );
    assert_eq!(
        app.world()
            .resource::<WidgetryFileDialogStorage>()
            .snapshot("shared")
            .unwrap()
            .pinned,
        [std::path::PathBuf::from("C:/pin")]
    );
    for (root, path) in [(first, "C:/one"), (second, "C:/two")] {
        WidgetryFileDialog::apply(
            app.world_mut(),
            root,
            WidgetryFileDialogAction::Navigate(WidgetryFileDialogNavigation::Path(path.into())),
        )
        .unwrap();
        let token = app
            .world()
            .get::<WidgetryFileDialogState>(root)
            .unwrap()
            .token();
        WidgetryFileDialog::deliver(
            app.world_mut(),
            root,
            WidgetryFileDialogReply::Started {
                token,
                path: path.into(),
            },
        )
        .unwrap();
    }
    WidgetryFileDialog::apply(app.world_mut(), first, WidgetryFileDialogAction::Confirm).unwrap();
    let candidate = app
        .world()
        .get::<WidgetryFileDialogState>(first)
        .unwrap()
        .validation_job()
        .unwrap()
        .prepare()
        .unwrap();
    WidgetryFileDialog::deliver(
        app.world_mut(),
        first,
        WidgetryFileDialogReply::Validated {
            candidate,
            exists: true,
        },
    )
    .unwrap();
    assert_eq!(
        app.world()
            .resource::<WidgetryFileDialogStorage>()
            .snapshot("shared")
            .unwrap()
            .last_picked_dir,
        Some("C:/one".into())
    );
    WidgetryFileDialog::apply(
        app.world_mut(),
        second,
        WidgetryFileDialogAction::Navigate(WidgetryFileDialogNavigation::Refresh),
    )
    .unwrap();
    let token = app
        .world()
        .get::<WidgetryFileDialogState>(second)
        .unwrap()
        .token();
    WidgetryFileDialog::deliver(
        app.world_mut(),
        second,
        WidgetryFileDialogReply::Started {
            token,
            path: "C:/two".into(),
        },
    )
    .unwrap();
    assert_eq!(
        app.world()
            .resource::<WidgetryFileDialogStorage>()
            .snapshot("shared")
            .unwrap()
            .last_visited_dir,
        Some("C:/two".into())
    );
}

#[test]
fn history_back_forward_refresh_branch_and_root_up_have_distinct_semantics() {
    let mut app = app();
    let root = dialog(
        &mut app,
        WidgetryFileDialogProps {
            initial_directory: Some("C:/one".into()),
            ..default()
        },
    );
    for path in ["C:/one", "C:/two", "C:/three"] {
        if path != "C:/one" {
            WidgetryFileDialog::apply(
                app.world_mut(),
                root,
                WidgetryFileDialogAction::Navigate(WidgetryFileDialogNavigation::Path(path.into())),
            )
            .unwrap();
        }
        let token = app
            .world()
            .get::<WidgetryFileDialogState>(root)
            .unwrap()
            .token();
        WidgetryFileDialog::deliver(
            app.world_mut(),
            root,
            WidgetryFileDialogReply::Started {
                token,
                path: path.into(),
            },
        )
        .unwrap();
    }
    WidgetryFileDialog::apply(
        app.world_mut(),
        root,
        WidgetryFileDialogAction::Navigate(WidgetryFileDialogNavigation::Back),
    )
    .unwrap();
    let token = app
        .world()
        .get::<WidgetryFileDialogState>(root)
        .unwrap()
        .token();
    WidgetryFileDialog::deliver(
        app.world_mut(),
        root,
        WidgetryFileDialogReply::Started {
            token,
            path: "C:/two".into(),
        },
    )
    .unwrap();
    WidgetryFileDialog::apply(
        app.world_mut(),
        root,
        WidgetryFileDialogAction::Navigate(WidgetryFileDialogNavigation::Forward),
    )
    .unwrap();
    let token = app
        .world()
        .get::<WidgetryFileDialogState>(root)
        .unwrap()
        .token();
    WidgetryFileDialog::deliver(
        app.world_mut(),
        root,
        WidgetryFileDialogReply::Started {
            token,
            path: "C:/three".into(),
        },
    )
    .unwrap();
    WidgetryFileDialog::apply(
        app.world_mut(),
        root,
        WidgetryFileDialogAction::Navigate(WidgetryFileDialogNavigation::Back),
    )
    .unwrap();
    let token = app
        .world()
        .get::<WidgetryFileDialogState>(root)
        .unwrap()
        .token();
    WidgetryFileDialog::deliver(
        app.world_mut(),
        root,
        WidgetryFileDialogReply::Started {
            token,
            path: "C:/two".into(),
        },
    )
    .unwrap();
    for navigation in [
        WidgetryFileDialogNavigation::Refresh,
        WidgetryFileDialogNavigation::Path("C:/four".into()),
    ] {
        WidgetryFileDialog::apply(
            app.world_mut(),
            root,
            WidgetryFileDialogAction::Navigate(navigation),
        )
        .unwrap();
        let state = app.world().get::<WidgetryFileDialogState>(root).unwrap();
        let token = state.token();
        let path = state.requested_path().unwrap().to_owned();
        WidgetryFileDialog::deliver(
            app.world_mut(),
            root,
            WidgetryFileDialogReply::Started { token, path },
        )
        .unwrap();
    }
    assert_eq!(
        app.world()
            .get::<WidgetryFileDialogState>(root)
            .unwrap()
            .history(),
        [
            std::path::PathBuf::from("C:/one"),
            std::path::PathBuf::from("C:/two"),
            std::path::PathBuf::from("C:/four")
        ]
    );
    assert!(
        !WidgetryFileDialog::apply(
            app.world_mut(),
            root,
            WidgetryFileDialogAction::Navigate(WidgetryFileDialogNavigation::Forward)
        )
        .unwrap()
    );
    WidgetryFileDialog::apply(
        app.world_mut(),
        root,
        WidgetryFileDialogAction::Navigate(WidgetryFileDialogNavigation::Path("C:/".into())),
    )
    .unwrap();
    let token = app
        .world()
        .get::<WidgetryFileDialogState>(root)
        .unwrap()
        .token();
    WidgetryFileDialog::deliver(
        app.world_mut(),
        root,
        WidgetryFileDialogReply::Started {
            token,
            path: "C:/".into(),
        },
    )
    .unwrap();
    assert!(
        !WidgetryFileDialog::apply(
            app.world_mut(),
            root,
            WidgetryFileDialogAction::Navigate(WidgetryFileDialogNavigation::Up)
        )
        .unwrap()
    );
}

#[test]
fn validation_error_recovers_and_accepting_overwrite_resolves_exact_candidate_once() {
    let mut app = app();
    let root = dialog(
        &mut app,
        WidgetryFileDialogProps {
            mode: WidgetryFileDialogMode::SaveFile,
            ..default()
        },
    );
    loaded(&mut app, root, &[]);
    WidgetryFileDialog::apply(
        app.world_mut(),
        root,
        WidgetryFileDialogAction::Filename("name.txt".into()),
    )
    .unwrap();
    WidgetryFileDialog::apply(app.world_mut(), root, WidgetryFileDialogAction::Confirm).unwrap();
    let token = app
        .world()
        .get::<WidgetryFileDialogState>(root)
        .unwrap()
        .token();
    WidgetryFileDialog::deliver(
        app.world_mut(),
        root,
        WidgetryFileDialogReply::ValidationFailed {
            token,
            error: "parent vanished".into(),
        },
    )
    .unwrap();
    assert_eq!(
        app.world()
            .get::<WidgetryFileDialogState>(root)
            .unwrap()
            .error(),
        Some("parent vanished")
    );
    assert!(app.world().resource::<Results>().0.is_empty());
    WidgetryFileDialog::apply(app.world_mut(), root, WidgetryFileDialogAction::Confirm).unwrap();
    let candidate = app
        .world()
        .get::<WidgetryFileDialogState>(root)
        .unwrap()
        .validation_job()
        .unwrap()
        .prepare()
        .unwrap();
    let token = candidate.token();
    WidgetryFileDialog::deliver(
        app.world_mut(),
        root,
        WidgetryFileDialogReply::Validated {
            candidate,
            exists: true,
        },
    )
    .unwrap();
    assert!(app.world().resource::<Results>().0.is_empty());
    WidgetryFileDialog::apply(
        app.world_mut(),
        root,
        WidgetryFileDialogAction::Overwrite {
            token,
            accept: true,
        },
    )
    .unwrap();
    assert_eq!(
        app.world().resource::<Results>().0,
        [WidgetryFileDialogResult::SavePath(
            "C:/fixture/name.txt".into()
        )]
    );
    assert!(
        !WidgetryFileDialog::apply(app.world_mut(), root, WidgetryFileDialogAction::Cancel)
            .unwrap()
    );
}

#[test]
fn entry_kind_replacement_repairs_selection_and_invalidates_old_validation() {
    let mut app = app();
    let root = dialog(&mut app, WidgetryFileDialogProps::default());
    let snapshot = loaded(&mut app, root, &[("a", WidgetryFileDialogEntryKind::File)]);
    WidgetryFileDialog::apply(
        app.world_mut(),
        root,
        WidgetryFileDialogAction::Select {
            id: snapshot.visible()[0],
            token: snapshot.token(),
            operation: WidgetryFileDialogSelection::Replace,
        },
    )
    .unwrap();
    complete_selection(&mut app, root);
    WidgetryFileDialog::apply(app.world_mut(), root, WidgetryFileDialogAction::Confirm).unwrap();
    let candidate = app
        .world()
        .get::<WidgetryFileDialogState>(root)
        .unwrap()
        .validation_job()
        .unwrap()
        .prepare()
        .unwrap();
    loaded(
        &mut app,
        root,
        &[("a", WidgetryFileDialogEntryKind::Directory)],
    );
    assert!(
        app.world()
            .get::<WidgetryFileDialogState>(root)
            .unwrap()
            .selected()
            .is_empty()
    );
    assert_eq!(
        app.world()
            .get::<WidgetryFileDialogState>(root)
            .unwrap()
            .anchor(),
        None
    );
    assert!(
        !WidgetryFileDialog::deliver(
            app.world_mut(),
            root,
            WidgetryFileDialogReply::Validated {
                candidate,
                exists: true
            }
        )
        .unwrap()
    );
}

#[test]
fn delayed_projection_only_invalidates_confirmation_when_repair_changes_selection() {
    for keep_entry in [true, false] {
        let mut app = app();
        let root = dialog(&mut app, WidgetryFileDialogProps::default());
        let previous = loaded(
            &mut app,
            root,
            &[("a.txt", WidgetryFileDialogEntryKind::File)],
        );
        WidgetryFileDialog::apply(
            app.world_mut(),
            root,
            WidgetryFileDialogAction::Select {
                id: previous.visible()[0],
                token: previous.token(),
                operation: WidgetryFileDialogSelection::Replace,
            },
        )
        .unwrap();
        let state = app.world().get::<WidgetryFileDialogState>(root).unwrap();
        let data = if keep_entry {
            vec![WidgetryFileDialogEntryData {
                path: previous.entries()[0].path().to_owned(),
                name: previous.entries()[0].name().to_owned(),
                kind: WidgetryFileDialogEntryKind::File,
                size: None,
                modified: None,
                hidden: Some(false),
                system: Some(false),
            }]
        } else {
            Vec::new()
        };
        let snapshot = Arc::new(
            WidgetryFileDialogSnapshot::prepare(
                state.token(),
                previous.path().to_owned(),
                data,
                &state.query(),
                Some(&previous),
            )
            .unwrap(),
        );
        let selection = state
            .projection_selection_job(snapshot.clone())
            .unwrap()
            .prepare()
            .unwrap();
        complete_selection(&mut app, root);
        WidgetryFileDialog::apply(app.world_mut(), root, WidgetryFileDialogAction::Confirm)
            .unwrap();
        let state = app.world().get::<WidgetryFileDialogState>(root).unwrap();
        let candidate = state.validation_job().unwrap().prepare().unwrap();
        let token = state.token();
        assert!(
            WidgetryFileDialog::deliver(
                app.world_mut(),
                root,
                WidgetryFileDialogReply::Snapshot {
                    snapshot,
                    state: WidgetryFileDialogDirectoryState::Ready,
                    selection
                },
            )
            .unwrap()
        );
        let state = app.world().get::<WidgetryFileDialogState>(root).unwrap();
        if keep_entry {
            assert_eq!(state.token(), token);
            assert_eq!(
                state.confirmation(),
                &WidgetryFileDialogConfirmation::Validating
            );
        } else {
            assert!(state.selected().is_empty());
            assert_eq!(state.confirmation(), &WidgetryFileDialogConfirmation::Idle);
            assert!(state.token().selection_revision > token.selection_revision);
        }
        assert_eq!(
            WidgetryFileDialog::deliver(
                app.world_mut(),
                root,
                WidgetryFileDialogReply::Validated {
                    candidate,
                    exists: true
                },
            )
            .unwrap(),
            keep_entry
        );
        assert_eq!(
            app.world().resource::<Results>().0.len(),
            usize::from(keep_entry)
        );
    }
}

#[test]
fn activation_target_repair_commits_snapshot_and_ends_automatic_confirmation() {
    for replacement in [None, Some(WidgetryFileDialogEntryKind::Directory)] {
        let mut app = app();
        let root = dialog(&mut app, WidgetryFileDialogProps::default());
        let previous = loaded(
            &mut app,
            root,
            &[("a.txt", WidgetryFileDialogEntryKind::File)],
        );
        WidgetryFileDialog::apply(
            app.world_mut(),
            root,
            WidgetryFileDialogAction::Activate {
                id: previous.visible()[0],
                token: previous.token(),
            },
        )
        .unwrap();
        let stale_selection = app
            .world()
            .get::<WidgetryFileDialogState>(root)
            .unwrap()
            .selection_job()
            .unwrap()
            .prepare()
            .unwrap();
        let names: Vec<_> = replacement
            .into_iter()
            .map(|kind| ("a.txt", kind))
            .collect();
        let snapshot = loaded(&mut app, root, &names);
        let state = app.world().get::<WidgetryFileDialogState>(root).unwrap();
        assert!(Arc::ptr_eq(state.snapshot().unwrap(), &snapshot));
        assert!(state.selected().is_empty());
        assert!(!state.selection_pending());
        assert_eq!(state.confirmation(), &WidgetryFileDialogConfirmation::Idle);
        assert!(state.validation_job().is_none());
        assert!(app.world().resource::<Results>().0.is_empty());
        assert!(
            !WidgetryFileDialog::deliver(
                app.world_mut(),
                root,
                WidgetryFileDialogReply::Selection(stale_selection),
            )
            .unwrap()
        );
    }
}

#[test]
fn pending_selection_repair_rejects_other_prepared_replies_without_cancelling_confirmation() {
    let mut app = app();
    let root = dialog(
        &mut app,
        WidgetryFileDialogProps {
            mode: WidgetryFileDialogMode::PickFiles,
            ..default()
        },
    );
    let previous = loaded(
        &mut app,
        root,
        &[
            ("a.txt", WidgetryFileDialogEntryKind::File),
            ("b.txt", WidgetryFileDialogEntryKind::File),
        ],
    );
    WidgetryFileDialog::apply(app.world_mut(), root, WidgetryFileDialogAction::SelectAll).unwrap();
    let state = app.world().get::<WidgetryFileDialogState>(root).unwrap();
    let stale_selection = state.selection_job().unwrap().prepare().unwrap();
    let entry = previous
        .entries()
        .iter()
        .find(|entry| entry.name() == "b.txt")
        .unwrap();
    let snapshot = Arc::new(
        WidgetryFileDialogSnapshot::prepare(
            state.token(),
            previous.path().to_owned(),
            vec![WidgetryFileDialogEntryData {
                path: entry.path().to_owned(),
                name: entry.name().to_owned(),
                kind: entry.kind(),
                size: None,
                modified: None,
                hidden: Some(false),
                system: Some(false),
            }],
            &state.query(),
            Some(&previous),
        )
        .unwrap(),
    );
    let selection = state
        .projection_selection_job(snapshot.clone())
        .unwrap()
        .prepare()
        .unwrap();
    let late_snapshot = Arc::new(snapshot.reproject(state.token(), &state.query()).unwrap());
    let late_selection = state
        .projection_selection_job(late_snapshot.clone())
        .unwrap()
        .prepare()
        .unwrap();
    let late_reply = WidgetryFileDialogReply::Snapshot {
        snapshot: late_snapshot,
        state: WidgetryFileDialogDirectoryState::Ready,
        selection: late_selection,
    };
    let reply = WidgetryFileDialogReply::Snapshot {
        snapshot,
        state: WidgetryFileDialogDirectoryState::Ready,
        selection,
    };
    assert!(WidgetryFileDialog::deliver(app.world_mut(), root, reply.clone()).unwrap());
    assert!(
        !WidgetryFileDialog::deliver(
            app.world_mut(),
            root,
            WidgetryFileDialogReply::Selection(stale_selection)
        )
        .unwrap()
    );
    WidgetryFileDialog::apply(app.world_mut(), root, WidgetryFileDialogAction::Confirm).unwrap();
    let state = app.world().get::<WidgetryFileDialogState>(root).unwrap();
    assert_eq!(state.selected().len(), 1);
    let token = state.token();
    let candidate = state.validation_job().unwrap().prepare().unwrap();
    assert!(!WidgetryFileDialog::deliver(app.world_mut(), root, late_reply).unwrap());
    let state = app.world().get::<WidgetryFileDialogState>(root).unwrap();
    assert_eq!(state.token(), token);
    assert_eq!(
        state.confirmation(),
        &WidgetryFileDialogConfirmation::Validating
    );
    assert!(
        WidgetryFileDialog::deliver(
            app.world_mut(),
            root,
            WidgetryFileDialogReply::Validated {
                candidate,
                exists: true
            }
        )
        .unwrap()
    );
    assert_eq!(app.world().resource::<Results>().0.len(), 1);
}

#[test]
fn failed_directory_ends_pending_activation_without_rolling_back_selection_completion() {
    let mut app = app();
    let root = dialog(&mut app, WidgetryFileDialogProps::default());
    let snapshot = loaded(
        &mut app,
        root,
        &[("a.txt", WidgetryFileDialogEntryKind::File)],
    );
    WidgetryFileDialog::apply(
        app.world_mut(),
        root,
        WidgetryFileDialogAction::Activate {
            id: snapshot.visible()[0],
            token: snapshot.token(),
        },
    )
    .unwrap();
    let state = app.world().get::<WidgetryFileDialogState>(root).unwrap();
    let selection = state.selection_job().unwrap().prepare().unwrap();
    let token = state.token();
    WidgetryFileDialog::deliver(
        app.world_mut(),
        root,
        WidgetryFileDialogReply::Failed {
            token,
            error: "directory access lost".into(),
        },
    )
    .unwrap();
    assert!(
        WidgetryFileDialog::deliver(
            app.world_mut(),
            root,
            WidgetryFileDialogReply::Selection(selection),
        )
        .unwrap()
    );
    let state = app.world().get::<WidgetryFileDialogState>(root).unwrap();
    assert!(!state.selection_pending());
    assert!(state.selected().contains(&snapshot.visible()[0]));
    assert_eq!(state.confirmation(), &WidgetryFileDialogConfirmation::Idle);
    assert!(state.validation_job().is_none());
    assert!(matches!(
        state.directory_state(),
        WidgetryFileDialogDirectoryState::Failed(_)
    ));
    loaded(
        &mut app,
        root,
        &[("a.txt", WidgetryFileDialogEntryKind::File)],
    );
    let state = app.world().get::<WidgetryFileDialogState>(root).unwrap();
    assert_eq!(state.confirmation(), &WidgetryFileDialogConfirmation::Idle);
    assert!(app.world().resource::<Results>().0.is_empty());
}
