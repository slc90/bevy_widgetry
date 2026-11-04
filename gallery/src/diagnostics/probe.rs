use super::*;
use bevy_widgetry::diagnostics::bevy::DiagnosticBatch;
use bevy_widgetry::job::{JobFailure, JobManager, JobOptions};
use std::{io, sync::Arc};

#[derive(Event)]
struct ProbeEvent;

#[track_caller]
fn injected(name: &'static str) -> GalleryFailure {
    failure(
        GalleryError::Operation(io::Error::other(name).into()),
        ErrorFrame::new("gallery.diagnostic_probe").field("case", name),
    )
}

fn failed_system() -> Result {
    Err(injected("probe-system").into_bevy_error())
}

fn failed_observer(_: On<ProbeEvent>) -> Result {
    Err(injected("probe-observer").into_bevy_error())
}

pub(crate) fn run(app: &mut App, mode: &std::ffi::OsStr) -> Result<AppExit> {
    app.add_plugins(MinimalPlugins);
    if mode == "startup" {
        return Err(injected("probe-startup").into_bevy_error());
    }
    if mode == "panic" {
        // Exercise the host's native policy in a subprocess, including the original payload.
        handler(
            BevyError::panic(
                "probe-native-panic",
                Box::new(String::from("probe-original-payload")),
            ),
            ErrorContext::Command {
                name: "probe.native_panic".into(),
            },
        );
        return Ok(AppExit::error());
    }
    if mode != "lifecycle" {
        return Err(error(
            "GALLERY_DIAGNOSTIC_PROBE must be lifecycle, startup or panic",
        ));
    }
    let handled = injected("probe-handled");
    handled.record_handled("放弃诊断验证操作", "已确认终止");
    app.add_systems(Update, failed_system);
    app.add_observer(failed_observer);
    app.world_mut().trigger(ProbeEvent);
    app.world_mut()
        .commands()
        .queue(|_: &mut World| -> Result { Err(injected("probe-command").into_bevy_error()) });
    let first = Arc::new(injected("probe-batch-a"));
    let second = Arc::new(injected("probe-batch-b"));
    for _ in 0..2 {
        let mut batch = DiagnosticBatch::default();
        batch.push_shared(first.clone());
        batch.push_shared(second.clone());
        if let Some(error) = batch.into_bevy_error() {
            handler(
                error,
                ErrorContext::Command {
                    name: "probe.batch".into(),
                },
            );
        }
    }
    handler(
        BevyError::error(io::Error::other("probe-native-error")),
        ErrorContext::Command {
            name: "probe.native".into(),
        },
    );
    let manager = JobManager::default();
    let (id, mut receiver) = manager
        .spawn_thread::<(), _>(JobOptions::default(), |_| {
            Err(JobFailure::from(injected("probe-job")))
        })
        .map_err(|failure| failure.into_bevy_error())?;
    if let bevy_widgetry::job::JobRead::Finished(bevy_widgetry::job::JobOutcome::Failed(failure)) =
        receiver
            .recv_blocking()
            .map_err(|failure| failure.into_bevy_error())?
    {
        handler(
            failure.into_bevy_error(),
            ErrorContext::Command {
                name: "probe.job".into(),
            },
        );
    } else {
        return Err(error("diagnostic probe job did not fail"));
    }
    manager
        .remove(id)
        .map_err(|failure| failure.into_bevy_error())?;
    app.update();
    info!("诊断验证退出前末条日志");
    Ok(AppExit::Success)
}
