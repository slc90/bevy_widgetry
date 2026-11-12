const FLUSH_TIMEOUT: Duration = Duration::from_secs(5);

use crate::error::{self, LoggingError, LoggingFailure, LoggingSource, LoggingStage};
use crate::{safe_backtrace, safe_diagnostic, safe_location};
use bevy_widgetry_diagnostics::WidgetryFailure;
use std::{
    io::{self, Write},
    path::{Path, PathBuf},
    sync::{Arc, Condvar, Mutex, MutexGuard},
    time::{Duration, Instant},
};
use tracing_appender::non_blocking::{NonBlocking, NonBlockingBuilder, WorkerGuard};
use tracing_subscriber::fmt::MakeWriter;

#[derive(Default)]
struct State {
    accepted: usize,
    written: usize,
    flushed: usize,
    failure: Option<Arc<LoggingFailure>>,
}
struct Health {
    path: PathBuf,
    state: Mutex<State>,
    changed: Condvar,
}
impl Health {
    fn lock(&self) -> MutexGuard<'_, State> {
        self.state
            .lock()
            .unwrap_or_else(|poison| poison.into_inner())
    }
    #[track_caller]
    fn fail(&self, stage: LoggingStage, source: LoggingSource) -> Arc<LoggingFailure> {
        let mut state = self.lock();
        if let Some(failure) = &state.failure {
            return failure.clone();
        }
        let failure = Arc::new(WidgetryFailure::new_with_source_reporter(
            LoggingError {
                stage,
                path: self.path.clone(),
                source,
            },
            error::frame(stage, &self.path),
            |failure| {
                eprintln!(
                    "ERROR phase=source error_id={} error={} 日志后端失败，使用应急输出",
                    failure.origin().id(),
                    safe_diagnostic(failure)
                )
            },
        ));
        state.failure = Some(failure.clone());
        self.changed.notify_all();
        failure
    }
    #[track_caller]
    fn io(&self, stage: LoggingStage, source: io::Error) -> io::Error {
        let kind = source.kind();
        let message = source.to_string();
        self.fail(stage, LoggingSource::Io(Arc::new(source)));
        io::Error::new(kind, message)
    }
}

struct Observed<W> {
    inner: W,
    health: Arc<Health>,
}
impl<W: Write> Write for Observed<W> {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        let count = self
            .inner
            .write(bytes)
            .map_err(|error| self.health.io(LoggingStage::Write, error))?;
        if count == 0 && !bytes.is_empty() {
            return Err(self.health.io(
                LoggingStage::Write,
                io::Error::from(io::ErrorKind::WriteZero),
            ));
        }
        self.health.lock().written += count;
        Ok(count)
    }
    fn flush(&mut self) -> io::Result<()> {
        if let Err(error) = self.inner.flush() {
            // Keep the failure in the guard's result channel while completing the worker's shutdown handshake.
            // tracing-appender otherwise retries a consumed Shutdown indefinitely after a flush error.
            drop(self.health.io(LoggingStage::Flush, error));
            return Ok(());
        }
        let mut state = self.health.lock();
        state.flushed = state.written;
        self.health.changed.notify_all();
        Ok(())
    }
}

#[derive(Clone)]
pub(crate) struct FileWriter {
    writer: NonBlocking,
    health: Arc<Health>,
}
impl Write for FileWriter {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        // Never hold the health lock while waiting for queue capacity.
        let count = self
            .writer
            .write(bytes)
            .map_err(|error| self.health.io(LoggingStage::Queue, error))?;
        self.health.lock().accepted += count;
        Ok(count)
    }
    fn flush(&mut self) -> io::Result<()> {
        self.writer.flush()
    }
}
impl<'a> MakeWriter<'a> for FileWriter {
    type Writer = Self;
    fn make_writer(&'a self) -> Self {
        self.clone()
    }
}

pub struct LoggingGuard {
    worker: Option<WorkerGuard>,
    health: Arc<Health>,
}
impl LoggingGuard {
    pub fn finish(mut self) -> Result<(), Arc<LoggingFailure>> {
        self.finish_inner(FLUSH_TIMEOUT)
    }
    fn finish_inner(&mut self, timeout: Duration) -> Result<(), Arc<LoggingFailure>> {
        if self.worker.is_some() {
            let deadline = Instant::now() + timeout;
            let mut state = self.health.lock();
            let accepted = state.accepted;
            while state.failure.is_none() && state.flushed < accepted {
                let Some(remaining) = deadline.checked_duration_since(Instant::now()) else {
                    break;
                };
                let (next, _) = self
                    .health
                    .changed
                    .wait_timeout(state, remaining)
                    .unwrap_or_else(|poison| poison.into_inner());
                state = next;
            }
            let pending = accepted.saturating_sub(state.flushed);
            let timed_out = pending != 0 && state.failure.is_none();
            drop(state);
            if timed_out {
                self.health.fail(
                    LoggingStage::FlushTimeout,
                    LoggingSource::PendingBytes(pending),
                );
            }
            drop(self.worker.take());
        }
        match &self.health.lock().failure {
            Some(failure) => Err(failure.clone()),
            None => Ok(()),
        }
    }
}
impl Drop for LoggingGuard {
    fn drop(&mut self) {
        if self.worker.is_some()
            && let Err(failure) = self.finish_inner(FLUSH_TIMEOUT)
        {
            eprintln!(
                "ERROR phase=unhandled error_id={} origin={} backtrace={} 日志输出未能完整保存",
                failure.origin().id(),
                safe_location(failure.origin().location()),
                safe_backtrace(failure.origin().backtrace())
            );
        }
    }
}

pub(crate) fn prepare<W: Write + Send + 'static>(
    inner: W,
    path: &Path,
    capacity: usize,
) -> (FileWriter, LoggingGuard) {
    let health = Arc::new(Health {
        path: path.to_owned(),
        state: Mutex::default(),
        changed: Condvar::new(),
    });
    let (writer, worker) = NonBlockingBuilder::default()
        .buffered_lines_limit(capacity)
        .lossy(false)
        .finish(Observed {
            inner,
            health: health.clone(),
        });
    (
        FileWriter {
            writer,
            health: health.clone(),
        },
        LoggingGuard {
            worker: Some(worker),
            health,
        },
    )
}

#[cfg(test)]
#[allow(clippy::disallowed_macros, clippy::unwrap_used)]
mod tests {
    const SLOW_WRITE_DELAY: Duration = Duration::from_millis(1);
    const FLUSH_TEST_TIMEOUT: Duration = Duration::from_millis(10);

    use super::*;

    struct Slow(Arc<Mutex<Vec<u8>>>);
    impl Write for Slow {
        fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
            std::thread::sleep(SLOW_WRITE_DELAY);
            self.0.lock().unwrap().extend_from_slice(bytes);
            Ok(bytes.len())
        }
        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }

    #[test]
    fn small_queue_backpressure_preserves_every_line_and_guard_waits_for_flush() {
        let output = Arc::new(Mutex::new(Vec::new()));
        let (writer, guard) = prepare(Slow(output.clone()), Path::new("slow.log"), 1);
        let producers: Vec<_> = (0..4)
            .map(|producer| {
                let mut writer = writer.clone();
                std::thread::spawn(move || {
                    for index in 0..50 {
                        writer
                            .write_all(format!("diagnostic-{producer}-{index}\n").as_bytes())
                            .unwrap();
                    }
                })
            })
            .collect();
        for producer in producers {
            producer.join().unwrap();
        }
        guard.finish().unwrap();
        assert_eq!(writer.writer.error_counter().dropped_lines(), 0);
        let bytes = output.lock().unwrap();
        let contents = std::str::from_utf8(&bytes).unwrap();
        assert_eq!(contents.lines().count(), 200);
        for producer in 0..4 {
            for index in 0..50 {
                assert!(
                    contents
                        .lines()
                        .any(|line| line == format!("diagnostic-{producer}-{index}"))
                );
            }
        }
    }

    struct Broken {
        flush: bool,
        zero: bool,
    }
    impl Write for Broken {
        fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
            if self.zero {
                Ok(0)
            } else if self.flush {
                Ok(bytes.len())
            } else {
                Err(io::Error::new(
                    io::ErrorKind::PermissionDenied,
                    "writer rejected bytes",
                ))
            }
        }
        fn flush(&mut self) -> io::Result<()> {
            if self.flush {
                Err(io::Error::other("flush failed"))
            } else {
                Ok(())
            }
        }
    }

    #[test]
    fn write_flush_and_zero_write_failures_reach_the_guard_without_recursive_logging() {
        for (flush, zero, stage) in [
            (false, false, LoggingStage::Write),
            (true, false, LoggingStage::Flush),
            (false, true, LoggingStage::Write),
        ] {
            let (mut writer, guard) = prepare(Broken { flush, zero }, Path::new("broken.log"), 1);
            writer.write_all(b"diagnostic\n").unwrap();
            let failure = guard.finish().unwrap_err();
            assert_eq!(failure.kind().stage, stage);
            assert_eq!(failure.kind().path, Path::new("broken.log"));
            assert!(matches!(failure.kind().source, LoggingSource::Io(_)));
            assert_eq!(
                failure.origin().backtrace().status(),
                std::backtrace::BacktraceStatus::Captured
            );
            assert!(Arc::ptr_eq(
                &failure,
                writer.health.lock().failure.as_ref().unwrap()
            ));
        }
    }

    #[test]
    fn emergency_output_is_independent_and_does_not_expose_build_sources() {
        const CHILD: &str = "WIDGETRY_LOGGING_EMERGENCY_CHILD";
        if let Some(mode) = std::env::var_os(CHILD) {
            let (mut writer, guard) = prepare(
                Broken {
                    flush: mode == "flush",
                    zero: false,
                },
                Path::new(r"D:\runtime logs\broken.log"),
                1,
            );
            writer.write_all(b"diagnostic\n").unwrap();
            drop(guard);
            return;
        }
        for mode in ["write", "flush"] {
            let output = std::process::Command::new(std::env::current_exe().unwrap())
                .args(["--exact", "writer::tests::emergency_output_is_independent_and_does_not_expose_build_sources", "--nocapture"])
                .env(CHILD, mode)
                .output().unwrap();
            let stderr = String::from_utf8_lossy(&output.stderr);
            assert!(output.status.success(), "{stderr}");
            assert!(stderr.contains("phase=source error_id="), "{stderr}");
            assert!(stderr.contains("phase=unhandled error_id="), "{stderr}");
            assert!(
                stderr.contains(r"path=D:\runtime logs\broken.log"),
                "{stderr}"
            );
            assert!(
                stderr.contains("origin=crates/app_logging/src/writer.rs:"),
                "{stderr}"
            );
            assert!(!stderr.contains(env!("CARGO_MANIFEST_DIR")), "{stderr}");
            assert!(!stderr.contains(r".cargo\registry\src"), "{stderr}");
            assert!(!stderr.contains(r".rustup\toolchains"), "{stderr}");
        }
    }

    #[test]
    fn disconnected_writer_reports_the_real_queue_error() {
        let output = Arc::new(Mutex::new(Vec::new()));
        let (mut writer, guard) = prepare(Slow(output), Path::new("closed.log"), 1);
        guard.finish().unwrap();
        assert!(writer.write_all(b"too late").is_err());
        assert_eq!(
            writer.health.lock().failure.as_ref().unwrap().kind().stage,
            LoggingStage::Queue
        );
    }

    #[test]
    fn a_blocked_backend_returns_an_observable_flush_timeout() {
        struct Blocked(Arc<(Mutex<bool>, Condvar)>);
        impl Write for Blocked {
            fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
                let (lock, changed) = &*self.0;
                let mut ready = lock.lock().unwrap();
                while !*ready {
                    ready = changed.wait(ready).unwrap();
                }
                Ok(bytes.len())
            }
            fn flush(&mut self) -> io::Result<()> {
                Ok(())
            }
        }
        let gate = Arc::new((Mutex::new(false), Condvar::new()));
        let (mut writer, mut guard) = prepare(Blocked(gate.clone()), Path::new("blocked.log"), 1);
        writer.write_all(b"diagnostic").unwrap();
        let result = guard.finish_inner(FLUSH_TEST_TIMEOUT);
        *gate.0.lock().unwrap() = true;
        gate.1.notify_all();
        let failure = result.unwrap_err();
        assert_eq!(failure.kind().stage, LoggingStage::FlushTimeout);
        assert!(matches!(failure.kind().source, LoggingSource::PendingBytes(bytes) if bytes != 0));
    }
}
