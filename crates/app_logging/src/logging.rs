use bevy::ecs::error::Severity;
use bevy::log::{BoxedFmtLayer, BoxedLayer};
use bevy::prelude::*;
use std::fs::{self, OpenOptions};
use std::path::Path;
use time::{OffsetDateTime, UtcOffset, macros::format_description};
use tracing_appender::non_blocking::{NonBlocking, WorkerGuard};
use tracing_subscriber::fmt::time::OffsetTime;

pub struct AppLogging {
    output: LogOutput,
    guard: WorkerGuard,
}

#[derive(Resource)]
struct LogOutput {
    offset: UtcOffset,
    writer: NonBlocking,
}

fn timer(
    offset: UtcOffset,
) -> OffsetTime<&'static [time::format_description::BorrowedFormatItem<'static>]> {
    OffsetTime::new(
        offset,
        format_description!("[year]-[month]-[day] [hour]:[minute]:[second].[subsecond digits:3]"),
    )
}

pub fn terminal_layer(app: &mut App) -> Option<BoxedFmtLayer> {
    let output = app.world().get_resource::<LogOutput>()?;
    Some(Box::new(
        tracing_subscriber::fmt::layer()
            .with_timer(timer(output.offset))
            .with_target(false)
            .with_file(false)
            .with_line_number(false)
            .with_writer(std::io::stderr),
    ))
}

pub fn file_layer(app: &mut App) -> Option<BoxedLayer> {
    let output = app.world().get_resource::<LogOutput>()?;
    Some(Box::new(
        tracing_subscriber::fmt::layer()
            .with_timer(timer(output.offset))
            .with_target(false)
            .with_file(true)
            .with_line_number(true)
            .with_ansi(false)
            .with_writer(output.writer.clone()),
    ))
}

impl AppLogging {
    pub fn new(path: impl AsRef<Path>) -> Result<Self> {
        let directory = path.as_ref();
        (|| -> Result<Self> {
            let offset = UtcOffset::current_local_offset()?;
            let name = OffsetDateTime::now_utc()
                .to_offset(offset)
                .format(format_description!(
                    "[year]-[month]-[day]_[hour]-[minute]-[second]-[subsecond digits:3].log"
                ))?;
            fs::create_dir_all(directory)?;
            let file = OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(directory.join(name))?;
            let (writer, guard) = tracing_appender::non_blocking(file);
            Ok(Self {
                output: LogOutput { offset, writer },
                guard,
            })
        })()
        .map_err(|error| {
            error!(path = %directory.display(), error = %error, "应用日志准备失败");
            error.with_severity(Severity::Error)
        })
    }

    pub fn install(self, app: &mut App) -> WorkerGuard {
        app.insert_resource(self.output);
        self.guard
    }
}
