#![allow(clippy::disallowed_macros)]

use bevy::prelude::*;
use bevy::tasks::{IoTaskPool, TaskPoolBuilder};
use bevy_widgetry_file_dialog::*;
use bevy_widgetry_job::{JobKind, JobManager, ShutdownMessage, ShutdownPhase};
use bevy_widgetry_test_utils::scene_app;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Condvar, Mutex, mpsc};
use std::time::{Duration, Instant};

struct Backend {
    gate: Arc<(Mutex<bool>, Condvar)>,
    entered: mpsc::Sender<()>,
}

impl WidgetryFileDialogFileSystem for Backend {
    fn resolve_directory(&self, path: Option<&Path>) -> io::Result<PathBuf> {
        Ok(path.unwrap_or(Path::new("C:/fixture")).to_owned())
    }
    fn read_directory(
        &self,
        _: &Path,
    ) -> io::Result<Box<dyn Iterator<Item = io::Result<WidgetryFileDialogEntryData>> + Send>> {
        Ok(Box::new(std::iter::empty()))
    }
    fn validate(&self, _: &WidgetryFileDialogCandidate) -> io::Result<bool> {
        Ok(false)
    }
    fn create_directory(&self, _: &Path) -> io::Result<()> {
        self.entered.send(()).map_err(io::Error::other)?;
        let (lock, changed) = &*self.gate;
        let mut open = lock
            .lock()
            .map_err(|error| io::Error::other(error.to_string()))?;
        while !*open {
            open = changed
                .wait(open)
                .map_err(|error| io::Error::other(error.to_string()))?;
        }
        Ok(())
    }
    fn locations(&self) -> io::Result<Vec<WidgetryFileDialogLocation>> {
        Ok(vec![])
    }
}

struct Release(Arc<(Mutex<bool>, Condvar)>);
impl Drop for Release {
    fn drop(&mut self) {
        let (lock, changed) = &*self.0;
        let mut open = lock.lock().unwrap_or_else(|error| {
            eprintln!("folder test release recovered poisoned gate: {error}");
            error.into_inner()
        });
        *open = true;
        changed.notify_all();
    }
}

#[test]
fn slow_folder_operations_leave_a_single_io_thread_available_during_shutdown() -> Result {
    IoTaskPool::get_or_init(|| TaskPoolBuilder::new().num_threads(1).build());
    assert_eq!(IoTaskPool::get().thread_num(), 1);
    let mut app = scene_app();
    let gate = Arc::new((Mutex::new(false), Condvar::new()));
    let release = Release(gate.clone());
    let (entered, ready) = mpsc::channel();
    app.insert_resource(WidgetryFileDialogBackend(Arc::new(Backend {
        gate,
        entered,
    })));
    app.add_plugins(WidgetryFileDialogHeadlessPlugin);
    let mut roots = Vec::new();
    for _ in 0..8 {
        roots.push(app.world_mut().spawn_scene(bsn! { @WidgetryFileDialog { @initial_directory: {Some(PathBuf::from("C:/fixture"))} } })?.id());
    }
    let deadline = Instant::now() + Duration::from_secs(10);
    while roots.iter().any(|root| {
        app.world()
            .get::<WidgetryFileDialogState>(*root)
            .unwrap()
            .directory_state()
            != &WidgetryFileDialogDirectoryState::Ready
    }) {
        assert!(Instant::now() < deadline);
        app.update();
        std::thread::yield_now();
    }
    for root in roots {
        WidgetryFileDialog::apply(
            app.world_mut(),
            root,
            WidgetryFileDialogAction::NewFolder("created".into()),
        )?;
    }
    app.update();
    ready.recv_timeout(Duration::from_secs(5)).unwrap();
    let (sentinel, received) = mpsc::channel();
    IoTaskPool::get()
        .spawn(async move {
            futures_timer::Delay::new(Duration::from_millis(100)).await;
            sentinel.send(()).unwrap();
        })
        .detach();
    let shutdown = app
        .world()
        .resource::<JobManager>()
        .begin_shutdown(Duration::from_millis(20))
        .map_err(BevyError::error)?;
    // No App updates or shutdown polling can make the shared executor run this task.
    received
        .recv_timeout(Duration::from_secs(2))
        .expect("slow folder calls must leave I/O capacity for shutdown coordination");
    let status = shutdown.status();
    assert_eq!(status.phase, ShutdownPhase::TimedOut);
    assert!(status.remaining.iter().any(|job| job.label.as_deref()
        == Some("FileDialog create folder")
        && job.kind == JobKind::Thread));
    assert!(matches!(
        shutdown.try_recv(),
        Some(ShutdownMessage::TimedOut { .. })
    ));
    drop(release);
    // Managed snapshot retention stays alive until the App releases its last UI handles.
    drop(app);
    let deadline = Instant::now() + Duration::from_secs(10);
    while shutdown.status().phase != ShutdownPhase::Finished {
        assert!(Instant::now() < deadline);
        std::thread::yield_now();
    }
    assert!(matches!(
        shutdown.try_recv(),
        Some(ShutdownMessage::Completed)
    ));
    Ok(())
}
