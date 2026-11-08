const WRITER_QUEUE_CAPACITY: usize = 1024;

use crate::{
    error::{LoggingFailure, LoggingSource, LoggingStage, failure},
    writer::{self, FileWriter, LoggingGuard},
};
use bevy::log::{BoxedFmtLayer, BoxedLayer};
use bevy::prelude::*;
use std::fs::{self, OpenOptions};
use std::path::Path;
use std::sync::Arc;
use time::{OffsetDateTime, UtcOffset, macros::format_description};
use tracing_subscriber::fmt::time::OffsetTime;
use tracing_subscriber::{filter::filter_fn, prelude::*};

pub struct AppLogging {
    output: LogOutput,
    guard: LoggingGuard,
}

#[derive(Resource)]
struct LogOutput {
    offset: UtcOffset,
    writer: FileWriter,
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
            .with_writer(std::io::stderr)
            .with_filter(filter_fn(|metadata| {
                *metadata.level() <= bevy::log::Level::INFO
            })),
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
            .with_writer(output.writer.clone())
            .with_filter(filter_fn(|metadata| {
                *metadata.level() <= bevy::log::Level::DEBUG
            })),
    ))
}

impl AppLogging {
    pub fn new(path: impl AsRef<Path>) -> Result<Self, LoggingFailure> {
        let directory = path.as_ref();
        let offset = UtcOffset::current_local_offset().map_err(|source| {
            failure(
                LoggingStage::LocalOffset,
                directory,
                LoggingSource::Offset(source),
            )
        })?;
        let name = OffsetDateTime::now_utc()
            .to_offset(offset)
            .format(format_description!(
                "[year]-[month]-[day]_[hour]-[minute]-[second]-[subsecond digits:3].log"
            ))
            .map_err(|source| {
                failure(
                    LoggingStage::Timestamp,
                    directory,
                    LoggingSource::Format(source),
                )
            })?;
        fs::create_dir_all(directory).map_err(|source| {
            failure(
                LoggingStage::Directory,
                directory,
                LoggingSource::Io(Arc::new(source)),
            )
        })?;
        let path = directory.join(name);
        let file = open_log_file(&path)?;
        let (writer, guard) = writer::prepare(file, &path, WRITER_QUEUE_CAPACITY);
        Ok(Self {
            output: LogOutput { offset, writer },
            guard,
        })
    }

    pub fn install(self, app: &mut App) -> LoggingGuard {
        app.insert_resource(self.output);
        self.guard
    }
}

pub fn log_plugin() -> bevy::log::LogPlugin {
    bevy::log::LogPlugin {
        level: bevy::log::Level::DEBUG,
        fmt_layer: terminal_layer,
        custom_layer: file_layer,
        ..default()
    }
}

fn open_log_file(path: &Path) -> Result<std::fs::File, LoggingFailure> {
    OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .map_err(|source| {
            failure(
                LoggingStage::File,
                path,
                LoggingSource::Io(Arc::new(source)),
            )
        })
}

#[cfg(test)]
#[allow(clippy::disallowed_macros, clippy::unwrap_used)]
mod tests {
    use super::*;
    use std::error::Error;
    #[test]
    fn an_existing_file_is_rejected_with_its_original_io_error_and_path() {
        let path = std::env::current_exe().unwrap();
        let failure = open_log_file(&path).unwrap_err();
        assert_eq!(failure.kind().stage, LoggingStage::File);
        assert_eq!(failure.kind().path, path);
        assert!(failure.kind().source().unwrap().is::<std::io::Error>());
    }
}
