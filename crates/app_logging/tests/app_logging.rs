#![allow(clippy::disallowed_macros)]

const EXPECTED_LOG_FILENAME_LEN: usize = 27;
const EXPECTED_FILENAME_TIMESTAMP_LEN: usize = 23;
const YEAR_END: usize = 4;
const MONTH_END: usize = 7;
const DATE_END: usize = 10;
const HOUR_END: usize = 13;
const MINUTE_END: usize = 16;
const SECOND_END: usize = 19;
const MILLISECOND_RANGE: std::ops::Range<usize> = 20..23;
const PRODUCER_THREAD_NAME: &str = "logging-producer";
const NAMED_THREAD_MESSAGE: &str = "命名线程日志验证";
const UNNAMED_THREAD_MESSAGE: &str = "未命名线程日志验证";

use bevy::ecs::error::Severity;
use bevy::log::{debug, error, info, trace, warn};
use bevy::prelude::*;
use bevy_widgetry_app_logging::{AppLogging, LoggingStage, file_layer, log_plugin, terminal_layer};
use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

static NEXT_DIRECTORY: AtomicUsize = AtomicUsize::new(0);

struct TestDirectory(PathBuf);

impl TestDirectory {
    fn new() -> Result<Self> {
        let path = std::env::temp_dir().join(format!(
            "widgetry-app-logging-{}-{}-{}",
            std::process::id(),
            SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos(),
            NEXT_DIRECTORY.fetch_add(1, Ordering::Relaxed),
        ));
        fs::create_dir(&path)?;
        Ok(Self(path))
    }
}

impl Drop for TestDirectory {
    fn drop(&mut self) {
        if let Err(error) = fs::remove_dir_all(&self.0) {
            eprintln!("清理日志测试目录失败：{}: {error}", self.0.display());
        }
    }
}

#[test]
fn official_log_plugin_writes_source_and_flushes_on_guard_drop() -> Result {
    const CHILD: &str = "WIDGETRY_LOGGING_PLUGIN_CHILD";
    if std::env::var_os(CHILD).is_none() {
        let output = std::process::Command::new(std::env::current_exe()?)
            .args([
                "--exact",
                "official_log_plugin_writes_source_and_flushes_on_guard_drop",
                "--nocapture",
            ])
            .env(CHILD, "1")
            .env_remove("RUST_LOG")
            .output()?;
        assert!(
            output.status.success(),
            "{}\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        let terminal = String::from_utf8_lossy(&output.stderr);
        let named = event_prefix(&terminal, NAMED_THREAD_MESSAGE)?;
        assert!(named.contains(PRODUCER_THREAD_NAME));
        assert!(named.contains("ThreadId("));
        assert!(event_prefix(&terminal, UNNAMED_THREAD_MESSAGE)?.contains("ThreadId("));
        return Ok(());
    }
    let directory = TestDirectory::new()?;
    let logs = directory.0.join("nested/logs");
    let logging = AppLogging::new(&logs).map_err(|failure| failure.into_bevy_error())?;
    let files = fs::read_dir(&logs)?.collect::<std::io::Result<Vec<_>>>()?;
    assert_eq!(files.len(), 1);
    let path = files[0].path();
    let name = files[0].file_name().to_string_lossy().into_owned();
    assert_eq!(name.len(), EXPECTED_LOG_FILENAME_LEN);
    assert_eq!(&name[EXPECTED_FILENAME_TIMESTAMP_LEN..], ".log");
    for (index, byte) in name
        .bytes()
        .take(EXPECTED_FILENAME_TIMESTAMP_LEN)
        .enumerate()
    {
        match index {
            YEAR_END | MONTH_END | HOUR_END | MINUTE_END | SECOND_END => assert_eq!(byte, b'-'),
            DATE_END => assert_eq!(byte, b'_'),
            _ => assert!(byte.is_ascii_digit()),
        }
    }

    let mut app = App::new();
    let guard = logging.install(&mut app);
    app.add_plugins(log_plugin());
    let line = line!() + 1;
    info!("应用日志输出验证");
    info!(target: "bevy_widgetry", "Widgetry target 输出验证");
    debug!(target: "bevy_widgetry", error_id = "probe", phase = "handled", action = "fallback", "必要处理诊断");
    debug!(target: "ordinary_module", "普通模块DEBUG过滤验证");
    debug!(target: "bevy_widgetry", "库生命周期DEBUG验证");
    warn!(target: "ordinary_module", "宿主风险WARN验证");
    trace!(target: "ordinary_module", "宿主TRACE默认关闭验证");
    trace!(target: "bevy_widgetry", "库TRACE默认关闭验证");
    error!(target: "widget_gallery", error_id = "probe", phase = "unhandled", backtrace = %"frame1\nframe2", "最终报告验证");
    let named_thread_id = std::thread::Builder::new()
        .name(PRODUCER_THREAD_NAME.into())
        .spawn(|| {
            info!("{}", NAMED_THREAD_MESSAGE);
            format!("{:0>2?}", std::thread::current().id())
        })?
        .join()
        .map_err(|_| "命名日志生产线程意外panic")?;
    let unnamed_thread_id = std::thread::spawn(|| {
        info!("{}", UNNAMED_THREAD_MESSAGE);
        format!("{:0>2?}", std::thread::current().id())
    })
    .join()
    .map_err(|_| "未命名日志生产线程意外panic")?;
    drop(app);
    info!("guard 释放前的最后一条日志");
    drop(guard);

    let contents = fs::read_to_string(path)?;
    let named = event_prefix(&contents, NAMED_THREAD_MESSAGE)?;
    assert!(named.contains(PRODUCER_THREAD_NAME));
    assert!(named.contains(&named_thread_id));
    assert!(event_prefix(&contents, UNNAMED_THREAD_MESSAGE)?.contains(&unnamed_thread_id));
    assert!(contents.contains(&format!("app_logging.rs:{line}")));
    assert!(contents.contains("应用日志输出验证"));
    assert!(contents.contains("Widgetry target 输出验证"));
    assert!(contents.contains("guard 释放前的最后一条日志"));
    assert!(contents.contains("必要处理诊断"));
    assert!(contents.contains("frame1\nframe2"));
    assert!(contents.contains("普通模块DEBUG过滤验证"));
    assert!(contents.contains("库生命周期DEBUG验证"));
    assert!(contents.contains("宿主风险WARN验证"));
    assert!(!contents.contains("宿主TRACE默认关闭验证"));
    assert!(!contents.contains("库TRACE默认关闭验证"));
    assert!(!contents.contains('\u{1b}'));
    assert!(contents.contains("app_logging (app_logging.rs:"));
    assert!(!contents.contains("crates\\app_logging\\tests\\"));
    let timestamp = contents.lines().next().ok_or("日志为空")?;
    assert_eq!(timestamp.as_bytes()[DATE_END], b' ');
    assert_eq!(timestamp.as_bytes()[SECOND_END], b'.');
    assert!(
        timestamp.as_bytes()[MILLISECOND_RANGE]
            .iter()
            .all(u8::is_ascii_digit)
    );
    Ok(())
}

fn event_prefix<'a>(contents: &'a str, message: &str) -> Result<&'a str> {
    contents
        .lines()
        .find_map(|line| line.split_once(message).map(|(prefix, _)| prefix))
        .ok_or_else(|| "缺少线程日志验证事件".into())
}

#[test]
fn factories_without_install_return_none() {
    let mut app = App::new();
    assert!(terminal_layer(&mut app).is_none());
    assert!(file_layer(&mut app).is_none());
}

#[test]
fn install_exposes_prepared_layers_and_returns_guard() -> Result {
    let directory = TestDirectory::new()?;
    let logs = directory.0.join("logs");
    let logging = AppLogging::new(&logs).map_err(|failure| failure.into_bevy_error())?;
    assert_eq!(fs::read_dir(&logs)?.count(), 1);

    let mut app = App::new();
    let guard = logging.install(&mut app);
    assert!(app.get_error_handler().is_none());
    assert!(terminal_layer(&mut app).is_some());
    assert!(file_layer(&mut app).is_some());
    assert_eq!(fs::read_dir(&logs)?.count(), 1);
    drop(app);
    drop(guard);
    Ok(())
}

#[test]
fn directory_failure_keeps_original_error_and_error_severity() -> Result {
    let directory = TestDirectory::new()?;
    let occupied = directory.0.join("occupied");
    fs::write(&occupied, "原有文件")?;
    let path = occupied.join("logs");
    let original = fs::create_dir_all(&path).err().ok_or("目录创建应失败")?;
    let failure = AppLogging::new(&path).err().ok_or("日志准备应失败")?;
    assert_eq!(failure.kind().stage, LoggingStage::Directory);
    assert_eq!(failure.kind().path, path);
    let failure = failure.into_bevy_error();
    assert_eq!(failure.severity(), Severity::Error);
    assert!(failure.to_string().contains(&original.to_string()));
    assert_eq!(fs::read_to_string(&occupied)?, "原有文件");
    Ok(())
}
