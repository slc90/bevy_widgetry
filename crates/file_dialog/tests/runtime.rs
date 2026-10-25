#![allow(clippy::disallowed_macros)]

use bevy::prelude::*;
use bevy_widgetry_file_dialog::*;
use bevy_widgetry_job::JobManager;
use bevy_widgetry_test_utils::scene_app;
use std::collections::BTreeMap;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, mpsc};
use std::thread::{self, ThreadId};
use std::time::{Duration, Instant};

struct Controlled {
    gates: Mutex<BTreeMap<PathBuf, mpsc::Receiver<()>>>,
    calls: mpsc::Sender<(&'static str, PathBuf, ThreadId)>,
    count: usize,
}

struct Release(Option<mpsc::Sender<()>>);

struct ControlledEntries {
    entries: std::vec::IntoIter<io::Result<WidgetryFileDialogEntryData>>,
    calls: mpsc::Sender<(&'static str, PathBuf, ThreadId)>,
    path: PathBuf,
    entered: bool,
}

#[derive(Resource, Default)]
struct ObserverFlushes(Vec<std::result::Result<(), String>>);

#[test]
fn cached_query_jobs_preserve_pending_snapshot_and_do_not_rescan() -> Result {
    let (mut app, calls, _) = fixture(&[], 4096);
    let root = dialog(&mut app, "cached")?;
    pump(&mut app, |world| {
        world
            .get::<WidgetryFileDialogState>(root)
            .is_some_and(|state| {
                state.directory_state() == &WidgetryFileDialogDirectoryState::Ready
            })
    })?;
    let initial_reads = calls
        .try_iter()
        .filter(|(operation, _, _)| *operation == "read")
        .count();
    assert_eq!(initial_reads, 1);
    let previous = app
        .world()
        .get::<WidgetryFileDialogState>(root)
        .unwrap()
        .snapshot()
        .unwrap()
        .clone();
    let search = WidgetryFileDialogAction::Search("004095".into());
    assert!(WidgetryFileDialog::apply(
        app.world_mut(),
        root,
        search.clone()
    )?);
    assert!(!WidgetryFileDialog::apply(app.world_mut(), root, search)?);
    let pending = app.world().get::<WidgetryFileDialogState>(root).unwrap();
    assert!(pending.projection_pending());
    assert!(Arc::ptr_eq(pending.snapshot().unwrap(), &previous));
    pump(&mut app, |world| {
        world
            .get::<WidgetryFileDialogState>(root)
            .is_some_and(|state| !state.projection_pending())
    })?;
    let projected = app.world().get::<WidgetryFileDialogState>(root).unwrap();
    assert_eq!(projected.entries().len(), 4096);
    assert_eq!(projected.visible().len(), 1);
    assert_eq!(projected.entries()[0].id(), previous.entries()[0].id());
    assert!(Arc::strong_count(projected.snapshot().unwrap()) >= 2);
    assert_eq!(
        calls
            .try_iter()
            .filter(|(operation, _, _)| *operation == "read")
            .count(),
        0
    );
    Ok(())
}

fn path(name: &str) -> PathBuf {
    PathBuf::from("C:/fixture").join(name)
}

fn fixture(
    names: &[&str],
    count: usize,
) -> (
    App,
    mpsc::Receiver<(&'static str, PathBuf, ThreadId)>,
    Vec<Release>,
) {
    let (calls, receiver) = mpsc::channel();
    let mut gates = BTreeMap::new();
    let mut releases = Vec::new();
    for name in names {
        let (release, gate) = mpsc::channel();
        gates.insert(path(name), gate);
        releases.push(Release(Some(release)));
    }
    let mut app = scene_app();
    app.insert_resource(WidgetryFileDialogBackend(Arc::new(Controlled {
        gates: Mutex::new(gates),
        calls,
        count,
    })));
    app.add_plugins(WidgetryFileDialogHeadlessPlugin);
    (app, receiver, releases)
}

fn dialog(app: &mut App, name: &str) -> Result<Entity> {
    Ok(app.world_mut().spawn_scene(bsn! { @WidgetryFileDialog { @initial_directory: {Some(path(name))}, @mode: WidgetryFileDialogMode::PickFiles } })?.id())
}

fn pump(app: &mut App, mut done: impl FnMut(&World) -> bool) -> Result {
    let start = Instant::now();
    while !done(app.world()) {
        if start.elapsed() > Duration::from_secs(10) {
            return Err(BevyError::error("controlled runtime did not complete"));
        }
        app.update();
        thread::yield_now();
    }
    Ok(())
}

impl Drop for Release {
    fn drop(&mut self) {
        if let Some(sender) = self.0.take()
            && let Err(error) = sender.send(())
        {
            eprintln!("controlled backend receiver already ended: {error}");
        }
    }
}

impl Iterator for ControlledEntries {
    type Item = io::Result<WidgetryFileDialogEntryData>;

    fn next(&mut self) -> Option<Self::Item> {
        if !self.entered {
            self.entered = true;
            if let Err(error) =
                self.calls
                    .send(("iterate", self.path.clone(), thread::current().id()))
            {
                return Some(Err(io::Error::other(error)));
            }
        }
        self.entries.next()
    }
}

impl Controlled {
    fn record(&self, operation: &'static str, path: &Path) -> io::Result<()> {
        self.calls
            .send((operation, path.to_owned(), thread::current().id()))
            .map_err(io::Error::other)
    }
}

impl WidgetryFileDialogFileSystem for Controlled {
    fn resolve_directory(&self, path: Option<&Path>) -> io::Result<PathBuf> {
        let path = path.ok_or_else(|| io::Error::other("fixture requires path"))?;
        self.record("resolve", path)?;
        Ok(path.to_owned())
    }

    fn read_directory(
        &self,
        path: &Path,
    ) -> io::Result<Box<dyn Iterator<Item = io::Result<WidgetryFileDialogEntryData>> + Send>> {
        self.record("read", path)?;
        if path.file_name().is_some_and(|name| name == "failed") {
            return Err(io::Error::new(
                io::ErrorKind::PermissionDenied,
                "controlled directory read failure",
            ));
        }
        let gate = self
            .gates
            .lock()
            .map_err(|error| io::Error::other(error.to_string()))?
            .remove(path);
        if let Some(gate) = gate {
            gate.recv().map_err(io::Error::other)?;
        }
        let mut entries: Vec<_> = (0..self.count)
            .rev()
            .map(|index| {
                let name = format!("{index:06}.txt");
                Ok(WidgetryFileDialogEntryData {
                    path: path.join(&name),
                    name: name.into(),
                    kind: WidgetryFileDialogEntryKind::File,
                    size: Some(index as u64),
                    modified: None,
                    hidden: Some(false),
                    system: Some(false),
                })
            })
            .collect();
        if path.file_name().is_some_and(|name| name == "partial") {
            entries.push(Err(io::Error::other("entry disappeared")));
        }
        Ok(Box::new(ControlledEntries {
            entries: entries.into_iter(),
            calls: self.calls.clone(),
            path: path.to_owned(),
            entered: false,
        }))
    }

    fn validate(&self, candidate: &WidgetryFileDialogCandidate) -> io::Result<bool> {
        self.record("validate", &path("validation"))?;
        assert!(matches!(
            candidate.result(),
            WidgetryFileDialogResult::Files(_)
        ));
        Ok(true)
    }

    fn create_directory(&self, path: &Path) -> io::Result<()> {
        self.record("create", path)?;
        let gate = self
            .gates
            .lock()
            .map_err(|error| io::Error::other(error.to_string()))?
            .remove(path);
        if let Some(gate) = gate {
            gate.recv().map_err(io::Error::other)?;
        }
        self.record("created", path)
    }

    fn locations(&self) -> io::Result<Vec<WidgetryFileDialogLocation>> {
        self.record("locations", &path("locations"))?;
        Ok(vec![])
    }

    fn load_preferences(
        &self,
        path: &Path,
    ) -> io::Result<BTreeMap<String, WidgetryFileDialogStorageSnapshot>> {
        self.record("load", path)?;
        WidgetryFileDialogNativeFileSystem.load_preferences(path)
    }

    fn save_preferences(
        &self,
        path: &Path,
        scopes: &BTreeMap<String, WidgetryFileDialogStorageSnapshot>,
    ) -> io::Result<()> {
        self.record("save", path)?;
        if path.file_name().is_some_and(|name| name == "failed.json") {
            return Err(io::Error::new(
                io::ErrorKind::PermissionDenied,
                "controlled preference write failure",
            ));
        }
        WidgetryFileDialogNativeFileSystem.save_preferences(path, scopes)
    }
}

#[test]
fn observer_flush_survives_multiple_replies_in_one_update() -> Result {
    let (mut app, calls, releases) = fixture(&["a", "b"], 0);
    let directory = tempfile::tempdir().map_err(BevyError::error)?;
    app.insert_resource(WidgetryFileDialogPersistence {
        path: directory.path().join("preferences.json"),
    });
    app.init_resource::<ObserverFlushes>();
    app.add_observer(
        |event: On<WidgetryFileDialogChangeEvent>,
         states: Query<&WidgetryFileDialogState>,
         mut commands: Commands| {
            if states
                .get(event.entity)
                .is_ok_and(|state| state.current_path().is_some())
            {
                commands.queue(|world: &mut World| {
                    let outcome = WidgetryFileDialog::flush_preferences(world)
                        .map_err(|error| error.to_string());
                    world.resource_mut::<ObserverFlushes>().0.push(outcome);
                });
            }
        },
    );
    dialog(&mut app, "a")?;
    dialog(&mut app, "b")?;
    app.update();
    let mut blocked = 0;
    pump(&mut app, |_| {
        blocked += calls
            .try_iter()
            .filter(|(operation, _, _)| *operation == "read")
            .count();
        blocked == 2
    })?;
    drop(releases);
    let mut started = 0;
    while started < 2 {
        let (operation, _, _) = calls
            .recv_timeout(Duration::from_secs(10))
            .map_err(BevyError::error)?;
        if operation == "iterate" {
            started += 1;
        }
    }
    app.world_mut()
        .resource_mut::<WidgetryFileDialogRuntimeOptions>()
        .update_budget = Duration::from_secs(1);
    app.update();
    let flushes = &app.world().resource::<ObserverFlushes>().0;
    assert!(flushes.len() >= 2, "{flushes:?}");
    assert!(
        flushes.iter().all(std::result::Result::is_ok),
        "{flushes:?}"
    );
    pump(&mut app, |world| {
        matches!(
            world
                .resource::<WidgetryFileDialogRuntimeStatus>()
                .persistence,
            WidgetryFileDialogStorageState::Saved(_)
        )
    })?;
    assert!(directory.path().join("preferences.json").exists());
    Ok(())
}

#[test]
fn headless_installs_manager_and_manual_mode_does_not_read() -> Result {
    let (mut app, calls, _) = fixture(&[], 1);
    assert!(app.world().contains_resource::<JobManager>());
    app.world_mut()
        .resource_mut::<WidgetryFileDialogRuntimeOptions>()
        .automatic = false;
    let root = dialog(&mut app, "manual")?;
    for _ in 0..3 {
        app.update();
    }
    assert!(calls.try_iter().next().is_none());
    assert_eq!(
        app.world()
            .get::<WidgetryFileDialogState>(root)
            .unwrap()
            .directory_state(),
        &WidgetryFileDialogDirectoryState::Loading
    );
    Ok(())
}

#[test]
fn directory_read_failure_and_entry_limit_are_reported_after_job_completion() -> Result {
    for (name, count, error) in [
        ("failed", 0, Some("controlled directory read failure")),
        ("limit", 100_000, None),
        ("over_limit", 100_001, Some("capacity exceeded")),
    ] {
        let (mut app, _calls, _releases) = fixture(&[], count);
        let root = dialog(&mut app, name)?;
        pump(&mut app, |world| {
            world
                .get::<WidgetryFileDialogState>(root)
                .is_some_and(|state| {
                    matches!(
                        state.directory_state(),
                        WidgetryFileDialogDirectoryState::Ready
                            | WidgetryFileDialogDirectoryState::Failed(_)
                    )
                })
        })?;
        let state = app.world().get::<WidgetryFileDialogState>(root).unwrap();
        match error {
            Some(message) => assert!(
                matches!(state.directory_state(), WidgetryFileDialogDirectoryState::Failed(error) if error.contains(message))
            ),
            None => assert_eq!(state.entries().len(), count),
        }
    }
    Ok(())
}

#[test]
fn file_dialog_reuses_an_explicit_plugin_and_a_preconfigured_manager() -> Result {
    use bevy_widgetry_job::{JobManagerPlugin, JobMessage, JobOptions, JobOutcome};
    let mut app = scene_app();
    let manager = JobManager::with_buffer_capacity(1).map_err(BevyError::error)?;
    let (id, mut receiver) = manager
        .spawn_io::<(), _, _, String>(JobOptions::default(), |_| async { Ok(()) })
        .map_err(BevyError::error)?;
    app.insert_resource(manager);
    app.add_plugins((JobManagerPlugin, WidgetryFileDialogHeadlessPlugin));
    assert!(matches!(
        receiver.recv_blocking(),
        Ok(JobMessage::Finished(JobOutcome::Completed))
    ));
    assert_eq!(
        app.world()
            .resource::<JobManager>()
            .outcome(id)
            .map_err(BevyError::error)?,
        Some(JobOutcome::Completed)
    );
    Ok(())
}

#[test]
fn cancel_and_reopen_reject_the_old_blocked_directory_job() -> Result {
    let (mut app, calls, releases) = fixture(&["a"], 257);
    let root = dialog(&mut app, "a")?;
    pump(&mut app, |_| {
        calls
            .try_iter()
            .any(|(operation, _, _)| operation == "read")
    })?;
    let old = app
        .world()
        .get::<WidgetryFileDialogState>(root)
        .unwrap()
        .token();
    WidgetryFileDialog::apply(app.world_mut(), root, WidgetryFileDialogAction::Cancel)?;
    WidgetryFileDialog::apply(app.world_mut(), root, WidgetryFileDialogAction::Reopen)?;
    WidgetryFileDialog::apply(
        app.world_mut(),
        root,
        WidgetryFileDialogAction::Navigate(WidgetryFileDialogNavigation::Path(path("fresh"))),
    )?;
    let current = app
        .world()
        .get::<WidgetryFileDialogState>(root)
        .unwrap()
        .token();
    assert_ne!(old.session, current.session);
    pump(&mut app, |world| {
        world
            .get::<WidgetryFileDialogState>(root)
            .is_some_and(|state| {
                state.directory_state() == &WidgetryFileDialogDirectoryState::Ready
            })
    })?;
    drop(releases);
    // Let the old backend return while the reopened session remains alive.
    for _ in 0..20 {
        app.update();
        thread::yield_now();
    }
    let state = app.world().get::<WidgetryFileDialogState>(root).unwrap();
    assert_eq!(state.token(), current);
    assert_eq!(state.current_path(), Some(path("fresh").as_path()));
    assert!(
        state
            .entries()
            .iter()
            .all(|entry| entry.path().starts_with(path("fresh")))
    );
    Ok(())
}

#[test]
fn first_update_starts_fixed_service_and_keeps_loading_available() -> Result {
    let mut app = scene_app();
    app.add_plugins(WidgetryFileDialogHeadlessPlugin);
    let root = app
        .world_mut()
        .spawn_scene(bsn! { @WidgetryFileDialog })?
        .id();
    app.update();
    assert_eq!(
        app.world()
            .resource::<WidgetryFileDialogRuntimeStatus>()
            .workers,
        3
    );
    assert_eq!(
        app.world()
            .get::<WidgetryFileDialogState>(root)
            .map(WidgetryFileDialogState::directory_state),
        Some(&WidgetryFileDialogDirectoryState::Loading)
    );
    WidgetryFileDialog::apply(app.world_mut(), root, WidgetryFileDialogAction::Cancel)?;
    assert_eq!(
        app.world()
            .get::<WidgetryFileDialogState>(root)
            .and_then(WidgetryFileDialogState::result),
        Some(&WidgetryFileDialogResult::Cancelled)
    );
    Ok(())
}

#[test]
fn one_blocked_slot_does_not_block_local_navigation_and_cancel() -> Result {
    let (mut app, calls, mut releases) = fixture(&["slow"], 257);
    let slow = dialog(&mut app, "slow")?;
    let fast = dialog(&mut app, "fast")?;
    pump(&mut app, |world| {
        world
            .get::<WidgetryFileDialogState>(fast)
            .is_some_and(|state| {
                state.directory_state() == &WidgetryFileDialogDirectoryState::Ready
            })
    })?;
    assert_eq!(
        app.world()
            .get::<WidgetryFileDialogState>(slow)
            .map(WidgetryFileDialogState::directory_state),
        Some(&WidgetryFileDialogDirectoryState::Loading)
    );
    assert_eq!(
        app.world()
            .get::<WidgetryFileDialogState>(fast)
            .map(|state| state.entries().len()),
        Some(257)
    );
    WidgetryFileDialog::apply(app.world_mut(), slow, WidgetryFileDialogAction::Cancel)?;
    releases.clear();
    pump(&mut app, |world| {
        world
            .resource::<WidgetryFileDialogRuntimeStatus>()
            .active_sessions
            == 1
    })?;
    assert_eq!(
        app.world()
            .get::<WidgetryFileDialogState>(slow)
            .and_then(WidgetryFileDialogState::result),
        Some(&WidgetryFileDialogResult::Cancelled)
    );
    for (_, _, thread) in calls.try_iter() {
        assert_ne!(thread, thread::current().id());
    }
    Ok(())
}

#[test]
fn two_blocked_slots_allow_cancel_and_latest_navigation_only() -> Result {
    let (mut app, calls, mut releases) = fixture(&["a", "b"], 12);
    let a = dialog(&mut app, "a")?;
    let b = dialog(&mut app, "b")?;
    let mut entered = 0;
    pump(&mut app, |_| {
        entered += calls.try_iter().filter(|(op, _, _)| *op == "read").count();
        entered == 2
    })?;
    WidgetryFileDialog::apply(
        app.world_mut(),
        a,
        WidgetryFileDialogAction::Navigate(WidgetryFileDialogNavigation::Path(path("c"))),
    )?;
    app.update();
    WidgetryFileDialog::apply(
        app.world_mut(),
        a,
        WidgetryFileDialogAction::Navigate(WidgetryFileDialogNavigation::Path(path("d"))),
    )?;
    app.update();
    WidgetryFileDialog::apply(app.world_mut(), b, WidgetryFileDialogAction::Cancel)?;
    assert_eq!(
        app.world()
            .resource::<WidgetryFileDialogRuntimeStatus>()
            .workers,
        3
    );
    releases.clear();
    pump(&mut app, |world| {
        world
            .get::<WidgetryFileDialogState>(a)
            .is_some_and(|state| {
                state.directory_state() == &WidgetryFileDialogDirectoryState::Ready
            })
    })?;
    assert_eq!(
        app.world()
            .get::<WidgetryFileDialogState>(a)
            .and_then(WidgetryFileDialogState::current_path),
        Some(path("d").as_path())
    );
    assert_eq!(
        app.world()
            .get::<WidgetryFileDialogState>(b)
            .and_then(WidgetryFileDialogState::result),
        Some(&WidgetryFileDialogResult::Cancelled)
    );
    Ok(())
}

#[test]
fn partial_stream_projection_selection_validation_and_idle_queries() -> Result {
    let (mut app, calls, _) = fixture(&[], 300);
    let root = dialog(&mut app, "partial")?;
    pump(&mut app, |world| {
        world
            .get::<WidgetryFileDialogState>(root)
            .is_some_and(|state| {
                matches!(
                    state.directory_state(),
                    WidgetryFileDialogDirectoryState::Partial { .. }
                ) && state.entries().len() == 300
            })
    })?;
    WidgetryFileDialog::apply(
        app.world_mut(),
        root,
        WidgetryFileDialogAction::Search("00000".into()),
    )?;
    pump(&mut app, |world| {
        world
            .get::<WidgetryFileDialogState>(root)
            .is_some_and(|state| !state.projection_pending())
    })?;
    assert_eq!(
        app.world()
            .get::<WidgetryFileDialogState>(root)
            .map(|state| state.visible().len()),
        Some(10)
    );
    WidgetryFileDialog::apply(app.world_mut(), root, WidgetryFileDialogAction::SelectAll)?;
    pump(&mut app, |world| {
        world
            .get::<WidgetryFileDialogState>(root)
            .is_some_and(|state| !state.selection_pending())
    })?;
    assert_eq!(
        app.world()
            .get::<WidgetryFileDialogState>(root)
            .map(|state| state.selected().len()),
        Some(10)
    );
    WidgetryFileDialog::apply(app.world_mut(), root, WidgetryFileDialogAction::Confirm)?;
    pump(&mut app, |world| {
        world
            .get::<WidgetryFileDialogState>(root)
            .is_some_and(|state| state.result().is_some())
    })?;
    assert!(app.world().get::<WidgetryFileDialogState>(root).is_some_and(|state| matches!(state.result(), Some(WidgetryFileDialogResult::Files(paths)) if paths.len() == 10)));
    for (_, _, thread) in calls.try_iter() {
        assert_ne!(thread, thread::current().id());
    }
    for _ in 0..10 {
        app.update();
    }
    assert_eq!(calls.try_iter().count(), 0);
    Ok(())
}

#[test]
fn consumer_message_limit_and_session_capacity_are_explicit() -> Result {
    let (mut app, _calls, _) = fixture(&[], 500);
    app.insert_resource(WidgetryFileDialogRuntimeOptions {
        max_sessions: 1,
        max_messages_per_update: 1,
        ..Default::default()
    });
    let first = dialog(&mut app, "first")?;
    let extra = dialog(&mut app, "extra")?;
    pump(&mut app, |world| {
        world
            .get::<WidgetryFileDialogState>(first)
            .is_some_and(|state| {
                state.directory_state() == &WidgetryFileDialogDirectoryState::Ready
            })
    })?;
    assert!(app.world().get::<WidgetryFileDialogState>(extra).is_some_and(|state| matches!(state.directory_state(), WidgetryFileDialogDirectoryState::Failed(error) if error.contains("capacity"))));
    assert!(
        app.world()
            .resource::<WidgetryFileDialogRuntimeStatus>()
            .applied_last_update
            <= 1
    );
    Ok(())
}

#[test]
fn persistence_is_opt_in_and_write_failure_cannot_rollback_result() -> Result {
    let temporary = tempfile::tempdir().map_err(BevyError::error)?;
    let (mut app, calls, _) = fixture(&[], 2);
    let root = app.world_mut().spawn_scene(bsn! { @WidgetryFileDialog { @initial_directory: {Some(path("files"))}, @storage_scope: {Some("open".into())}, @mode: WidgetryFileDialogMode::PickFiles } })?.id();
    pump(&mut app, |world| {
        world
            .get::<WidgetryFileDialogState>(root)
            .is_some_and(|state| {
                state.directory_state() == &WidgetryFileDialogDirectoryState::Ready
            })
    })?;
    assert!(
        calls
            .try_iter()
            .all(|(op, _, _)| op != "load" && op != "save")
    );
    app.insert_resource(WidgetryFileDialogPersistence {
        path: temporary.path().join("failed.json"),
    });
    WidgetryFileDialog::apply(app.world_mut(), root, WidgetryFileDialogAction::SelectAll)?;
    pump(&mut app, |world| {
        world
            .get::<WidgetryFileDialogState>(root)
            .is_some_and(|state| !state.selection_pending())
    })?;
    WidgetryFileDialog::apply(app.world_mut(), root, WidgetryFileDialogAction::Confirm)?;
    pump(&mut app, |world| {
        world
            .get::<WidgetryFileDialogState>(root)
            .is_some_and(|state| state.result().is_some())
            && matches!(
                world
                    .resource::<WidgetryFileDialogRuntimeStatus>()
                    .persistence,
                WidgetryFileDialogStorageState::Failed(_)
            )
    })?;
    assert!(app.world().get::<WidgetryFileDialogState>(root).is_some_and(|state| matches!(state.result(), Some(WidgetryFileDialogResult::Files(paths)) if paths.len() == 2)));
    assert!(!temporary.path().join("failed.json").exists());
    for (_, _, thread) in calls.try_iter() {
        assert_ne!(thread, thread::current().id());
    }
    Ok(())
}

#[test]
fn persistence_restores_future_session_without_redirecting_current_session() -> Result {
    let temporary = tempfile::tempdir().map_err(BevyError::error)?;
    let file = temporary.path().join("preferences.json");
    let stored = WidgetryFileDialogStorageSnapshot {
        last_visited_dir: Some(path("stored")),
        show_hidden: true,
        ..Default::default()
    };
    WidgetryFileDialogNativeFileSystem
        .save_preferences(&file, &BTreeMap::from([("open".into(), stored)]))
        .map_err(BevyError::error)?;
    let (mut app, _calls, _releases) = fixture(&["explicit"], 0);
    app.insert_resource(WidgetryFileDialogPersistence { path: file });
    let root = app.world_mut().spawn_scene(bsn! { @WidgetryFileDialog { @initial_directory: {Some(path("explicit"))}, @storage_scope: {Some("open".into())} } })?.id();
    pump(&mut app, |world| {
        world
            .resource::<WidgetryFileDialogStorage>()
            .snapshot("open")
            .is_some_and(|prefs| prefs.show_hidden)
    })?;
    assert_eq!(
        app.world()
            .get::<WidgetryFileDialogState>(root)
            .and_then(WidgetryFileDialogState::requested_path),
        Some(path("explicit").as_path())
    );
    let second = app
        .world_mut()
        .spawn_scene(bsn! { @WidgetryFileDialog { @storage_scope: {Some("open".into())} } })?
        .id();
    assert_eq!(
        app.world()
            .get::<WidgetryFileDialogState>(second)
            .and_then(WidgetryFileDialogState::requested_path),
        Some(path("stored").as_path())
    );
    assert!(
        app.world()
            .get::<WidgetryFileDialogState>(second)
            .is_some_and(|state| state.preferences().show_hidden)
    );
    Ok(())
}

#[test]
fn close_releases_session_and_background_retains_large_snapshot_ownership() -> Result {
    let (mut app, _calls, _) = fixture(&[], 2000);
    let root = dialog(&mut app, "large")?;
    pump(&mut app, |world| {
        world
            .get::<WidgetryFileDialogState>(root)
            .is_some_and(|state| {
                state.directory_state() == &WidgetryFileDialogDirectoryState::Ready
            })
    })?;
    let snapshot = app
        .world()
        .get::<WidgetryFileDialogState>(root)
        .and_then(WidgetryFileDialogState::snapshot)
        .ok_or_else(|| BevyError::error("snapshot missing"))?;
    assert!(Arc::strong_count(snapshot) >= 2);
    // Query projection shares entry data, so retained snapshot references must not be counted as UI owners during reclamation.
    for query in ["000", "0000", ""] {
        WidgetryFileDialog::apply(
            app.world_mut(),
            root,
            WidgetryFileDialogAction::Search(query.into()),
        )?;
        pump(&mut app, |world| {
            world
                .get::<WidgetryFileDialogState>(root)
                .is_some_and(|state| !state.projection_pending())
        })?;
    }
    app.world_mut().despawn(root);
    pump(&mut app, |world| {
        let status = world.resource::<WidgetryFileDialogRuntimeStatus>();
        status.active_sessions == 0 && status.retained_snapshots == 0
    })?;
    assert_eq!(
        app.world()
            .resource::<WidgetryFileDialogRuntimeStatus>()
            .workers,
        3
    );
    Ok(())
}

#[test]
fn blocked_folder_request_is_admitted_once_and_cancel_does_not_wait_for_creation() -> Result {
    let (mut app, calls, releases) = fixture(&["folder/created"], 3);
    let root = dialog(&mut app, "folder")?;
    pump(&mut app, |world| {
        world
            .get::<WidgetryFileDialogState>(root)
            .is_some_and(|state| {
                state.directory_state() == &WidgetryFileDialogDirectoryState::Ready
            })
    })?;
    calls.try_iter().for_each(drop);
    WidgetryFileDialog::apply(
        app.world_mut(),
        root,
        WidgetryFileDialogAction::NewFolder("created".into()),
    )?;
    let mut observed = Vec::new();
    pump(&mut app, |_| {
        observed.extend(calls.try_iter());
        observed
            .iter()
            .any(|(operation, _, _)| *operation == "create")
    })?;
    assert!(
        WidgetryFileDialog::apply(
            app.world_mut(),
            root,
            WidgetryFileDialogAction::NewFolder("created".into())
        )
        .is_err()
    );
    for _ in 0..16 {
        app.update();
    }
    observed.extend(calls.try_iter());
    assert_eq!(
        observed
            .iter()
            .filter(|(operation, _, _)| *operation == "create")
            .count(),
        1
    );
    assert!(
        !observed
            .iter()
            .any(|(operation, _, _)| *operation == "created")
    );
    assert!(
        app.world()
            .get::<WidgetryFileDialogState>(root)
            .unwrap()
            .folder_request()
            .is_some()
    );
    WidgetryFileDialog::apply(app.world_mut(), root, WidgetryFileDialogAction::Cancel)?;
    app.update();
    assert_eq!(
        app.world()
            .resource::<WidgetryFileDialogRuntimeStatus>()
            .active_sessions,
        0
    );
    WidgetryFileDialog::apply(app.world_mut(), root, WidgetryFileDialogAction::Reopen)?;
    let token = app
        .world()
        .get::<WidgetryFileDialogState>(root)
        .unwrap()
        .token();
    drop(releases);
    pump(&mut app, |world| {
        observed.extend(calls.try_iter());
        observed
            .iter()
            .any(|(operation, _, _)| *operation == "created")
            && world
                .get::<WidgetryFileDialogState>(root)
                .is_some_and(|state| {
                    state.directory_state() == &WidgetryFileDialogDirectoryState::Ready
                })
    })?;
    let state = app.world().get::<WidgetryFileDialogState>(root).unwrap();
    assert_eq!(state.token().session, token.session);
    assert_eq!(state.token().generation, token.generation);
    assert!(state.folder_request().is_none());
    assert!(state.error().is_none());
    assert_eq!(
        observed
            .iter()
            .filter(|(operation, _, _)| *operation == "create")
            .count(),
        1
    );
    Ok(())
}

#[test]
fn folder_completion_refreshes_without_reusing_previous_generation_snapshot() -> Result {
    let (mut app, calls, _) = fixture(&[], 300);
    let root = dialog(&mut app, "folder")?;
    pump(&mut app, |world| {
        world
            .get::<WidgetryFileDialogState>(root)
            .is_some_and(|state| {
                state.directory_state() == &WidgetryFileDialogDirectoryState::Ready
            })
    })?;
    let previous = app
        .world()
        .get::<WidgetryFileDialogState>(root)
        .ok_or_else(|| BevyError::error("state missing"))?
        .token()
        .generation;
    WidgetryFileDialog::apply(
        app.world_mut(),
        root,
        WidgetryFileDialogAction::NewFolder("created".into()),
    )?;
    pump(&mut app, |world| {
        world
            .get::<WidgetryFileDialogState>(root)
            .is_some_and(|state| {
                state.token().generation != previous
                    && state.directory_state() == &WidgetryFileDialogDirectoryState::Ready
            })
    })?;
    assert_eq!(
        calls.try_iter().filter(|(op, _, _)| *op == "read").count(),
        2
    );
    Ok(())
}

#[test]
fn result_completion_releases_session_in_the_same_update_with_or_without_root_despawn() -> Result {
    for despawn in [false, true] {
        let (mut app, _calls, _) = fixture(&[], 3);
        let root = dialog(&mut app, "result")?;
        let count = Arc::new(AtomicUsize::new(0));
        let observed = count.clone();
        app.world_mut().entity_mut(root).observe(
            move |event: On<WidgetryFileDialogResultEvent>, mut commands: Commands| {
                observed.fetch_add(1, Ordering::Relaxed);
                if despawn {
                    commands.entity(event.entity).despawn();
                }
            },
        );
        pump(&mut app, |world| {
            world
                .get::<WidgetryFileDialogState>(root)
                .is_some_and(|state| {
                    state.directory_state() == &WidgetryFileDialogDirectoryState::Ready
                })
        })?;
        WidgetryFileDialog::apply(app.world_mut(), root, WidgetryFileDialogAction::SelectAll)?;
        pump(&mut app, |world| {
            world
                .get::<WidgetryFileDialogState>(root)
                .is_some_and(|state| !state.selection_pending())
        })?;
        WidgetryFileDialog::apply(app.world_mut(), root, WidgetryFileDialogAction::Confirm)?;
        pump(&mut app, |_| count.load(Ordering::Relaxed) == 1)?;
        assert_eq!(
            app.world()
                .resource::<WidgetryFileDialogRuntimeStatus>()
                .active_sessions,
            0
        );
        if despawn {
            assert!(app.world().get_entity(root).is_err());
        } else {
            assert!(
                app.world()
                    .get::<WidgetryFileDialogState>(root)
                    .is_some_and(|state| state.result().is_some())
            );
        }
    }
    Ok(())
}
