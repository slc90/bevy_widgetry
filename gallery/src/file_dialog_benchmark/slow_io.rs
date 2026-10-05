use bevy::prelude::*;
use bevy_remote::{BrpError, BrpResult, RemoteMethodSystemId, RemoteMethods};
use bevy_widgetry::file_dialog::*;
use serde_json::{Value, json};
use std::io;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Condvar, Mutex};
use std::time::Duration;

#[derive(Resource, Clone)]
struct Gate(Arc<(Mutex<bool>, Condvar)>);

struct SlowFileSystem {
    delay: Duration,
    gate: Option<Gate>,
}

fn release(In(_params): In<Option<Value>>, gate: Res<Gate>) -> BrpResult {
    let (lock, changed) = &*gate.0;
    *lock.lock().map_err(BrpError::resource_error)? = true;
    changed.notify_all();
    Ok(json!({"released":true}))
}

pub(super) fn install(app: &mut App) -> Result {
    let delay = std::env::var("GALLERY_FILE_DIALOG_BENCH_IO_DELAY_MS")
        .ok()
        .map(|value| value.parse::<u64>())
        .transpose()?;
    let gated = std::env::var_os("GALLERY_FILE_DIALOG_BENCH_IO_GATE").is_some();
    if delay.is_none() && !gated {
        return Ok(());
    }
    let gate = gated.then(|| Gate(Arc::new((Mutex::new(false), Condvar::new()))));
    if let Some(gate) = &gate {
        app.insert_resource(gate.clone());
        let handler = app.world_mut().register_system(release);
        app.world_mut().resource_mut::<RemoteMethods>().insert(
            "benchmark/file_dialog_release_io",
            RemoteMethodSystemId::Instant(handler),
        );
    }
    app.insert_resource(WidgetryFileDialogBackend(Arc::new(SlowFileSystem {
        delay: Duration::from_millis(delay.unwrap_or(0)),
        gate,
    })));
    Ok(())
}

impl WidgetryFileDialogFileSystem for SlowFileSystem {
    fn resolve_directory(&self, path: Option<&Path>) -> io::Result<PathBuf> {
        WidgetryFileDialogNativeFileSystem.resolve_directory(path)
    }

    fn read_directory(
        &self,
        path: &Path,
    ) -> io::Result<Box<dyn Iterator<Item = io::Result<WidgetryFileDialogEntryData>> + Send>> {
        if let Some(gate) = &self.gate {
            let (lock, changed) = &*gate.0;
            let mut released = lock
                .lock()
                .map_err(|error| io::Error::other(error.to_string()))?;
            while !*released {
                released = changed
                    .wait(released)
                    .map_err(|error| io::Error::other(error.to_string()))?;
            }
        }
        std::thread::sleep(self.delay);
        WidgetryFileDialogNativeFileSystem.read_directory(path)
    }

    fn validate(&self, candidate: &WidgetryFileDialogCandidate) -> io::Result<bool> {
        WidgetryFileDialogNativeFileSystem.validate(candidate)
    }

    fn create_directory(&self, path: &Path) -> io::Result<()> {
        WidgetryFileDialogNativeFileSystem.create_directory(path)
    }

    fn locations(&self) -> io::Result<Vec<WidgetryFileDialogLocation>> {
        WidgetryFileDialogNativeFileSystem.locations()
    }
}
