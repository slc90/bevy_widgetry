use bevy::prelude::*;
use bevy_widgetry_file_dialog::*;
use bevy_widgetry_test_utils::{
    benchmark::artifact::{Artifact, error},
    scene_app,
};
use serde_json::{Value, json};
use std::collections::HashMap;
use std::hint::black_box;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};

struct Synthetic(usize);

fn fixture(items: usize) -> App {
    let mut app = scene_app();
    app.insert_resource(WidgetryFileDialogBackend(Arc::new(Synthetic(items))));
    app.add_plugins(WidgetryFileDialogHeadlessPlugin);
    app
}

fn state(app: &App, root: Entity) -> Result<&WidgetryFileDialogState> {
    app.world()
        .get::<WidgetryFileDialogState>(root)
        .ok_or_else(|| error("FileDialog state missing"))
}

fn measure(app: &mut App, samples: &mut Vec<f64>) {
    let start = Instant::now();
    app.update();
    samples.push(start.elapsed().as_secs_f64() * 1000.0);
}

fn summary(mut samples: Vec<f64>) -> Value {
    samples.sort_by(f64::total_cmp);
    let len = samples.len();
    json!({ "samples": len, "median_ms": samples.get(len / 2), "p95_ms": samples.get(len * 95 / 100), "max_ms": samples.last(), "over_2ms": samples.iter().filter(|value| **value > 2.0).count(), "over_16_7ms": samples.iter().filter(|value| **value > 16.7).count() })
}

fn main() -> Result {
    let artifact = Artifact::new(
        "file_dialog-runtime",
        "bench; headless App; 2 I/O + 1 CPU; no rendering",
    )?;
    let mut scenarios = Vec::new();
    for items in [1_000, 10_000, 100_000] {
        let mut app = fixture(items);
        let mut first = Vec::new();
        let mut final_projection = Vec::new();
        let mut updates = Vec::new();
        let mut idle = Vec::new();
        let mut cancel = Vec::new();
        let mut close = Vec::new();
        let mut peaks = (0, 0, 0);
        for sample in 0..12 {
            let start = Instant::now();
            let root = app.world_mut().spawn_scene(bsn! { @WidgetryFileDialog { @mode: WidgetryFileDialogMode::PickFiles, @initial_directory: {Some(PathBuf::from("C:/fixture"))} } })?.id();
            let mut first_seen = false;
            loop {
                if start.elapsed() > Duration::from_secs(60) {
                    return Err(error("runtime benchmark timed out"));
                }
                measure(&mut app, &mut updates);
                let current = state(&app, root)?;
                if let WidgetryFileDialogDirectoryState::Failed(failure) = current.directory_state()
                {
                    return Err(error(failure));
                }
                if !first_seen && !current.entries().is_empty() {
                    if sample > 1 {
                        first.push(start.elapsed().as_secs_f64() * 1000.0);
                    }
                    first_seen = true;
                }
                let status = app.world().resource::<WidgetryFileDialogRuntimeStatus>();
                peaks.0 = peaks.0.max(status.workers);
                peaks.1 = peaks.1.max(status.queued);
                peaks.2 = peaks.2.max(status.retained_snapshots);
                if current.directory_state() == &WidgetryFileDialogDirectoryState::Ready {
                    black_box(current.entries().len());
                    if sample > 1 {
                        final_projection.push(start.elapsed().as_secs_f64() * 1000.0);
                    }
                    break;
                }
                thread::yield_now();
            }
            for _ in 0..200 {
                measure(&mut app, &mut idle);
            }
            let start = Instant::now();
            WidgetryFileDialog::apply(app.world_mut(), root, WidgetryFileDialogAction::Cancel)?;
            cancel.push(start.elapsed().as_secs_f64() * 1000.0);
            let start = Instant::now();
            app.world_mut().despawn(root);
            app.update();
            close.push(start.elapsed().as_secs_f64() * 1000.0);
        }
        scenarios.push(json!({ "items": items, "directory_runs": 12, "warmup_runs": 2, "first_batch_ms": first, "final_projection_ms": final_projection, "update": summary(updates), "idle": summary(idle), "cancel": summary(cancel), "close": summary(close), "peak_workers": peaks.0, "peak_queued": peaks.1, "peak_retained": peaks.2 }));
    }
    let connection = wmi::WMIConnection::new().map_err(error)?;
    let process: Vec<HashMap<String, Value>> = connection.raw_query(format!("SELECT PeakWorkingSetSize,WorkingSetSize,ThreadCount FROM Win32_Process WHERE ProcessId={}", std::process::id())).map_err(error)?;
    let result = json!({ "scenarios": scenarios, "process": process, "boundaries": "first batch and final include BSN, admission, I/O synthetic iterator, CPU projection and App reply processing; update excludes rendering; cancel/close measure App thread only; samples include two warmup loads in update counters" });
    artifact.write_json("runtime.json", &result)?;
    println!("{result}");
    Ok(())
}

impl WidgetryFileDialogFileSystem for Synthetic {
    fn resolve_directory(&self, path: Option<&Path>) -> io::Result<PathBuf> {
        path.map(ToOwned::to_owned)
            .ok_or_else(|| io::Error::other("synthetic path missing"))
    }

    fn read_directory(
        &self,
        path: &Path,
    ) -> io::Result<Box<dyn Iterator<Item = io::Result<WidgetryFileDialogEntryData>> + Send>> {
        let path = path.to_owned();
        Ok(Box::new((0..self.0).rev().map(move |index| {
            let name = format!("file_{index:06}.txt");
            Ok(WidgetryFileDialogEntryData {
                path: path.join(&name),
                name: name.into(),
                kind: WidgetryFileDialogEntryKind::File,
                size: Some(index as u64),
                modified: None,
                hidden: Some(false),
                system: Some(false),
            })
        })))
    }

    fn validate(&self, _candidate: &WidgetryFileDialogCandidate) -> io::Result<bool> {
        Ok(true)
    }

    fn create_directory(&self, _path: &Path) -> io::Result<()> {
        Err(io::Error::other(
            "synthetic benchmark does not create directories",
        ))
    }

    fn locations(&self) -> io::Result<Vec<WidgetryFileDialogLocation>> {
        Ok(vec![])
    }
}
