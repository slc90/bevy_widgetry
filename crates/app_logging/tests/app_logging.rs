//! 覆盖 AppLogging 从目录准备到 install、LogPlugin 输出和 guard 释放的 lifecycle。
//! 验证新建目录、毫秒文件名、源码位置、统一 target 路由与退出前日志刷新。
//! 未 install 的 factory 返回 None，初始化失败返回 Error severity 并保留失败信息。

// contract 测试需要通过断言报告失败，仅在测试 scope 允许生产代码禁用的断言 macro。
#![allow(clippy::disallowed_macros)]

use bevy::ecs::error::Severity;
use bevy::log::{LogPlugin, info};
use bevy::prelude::*;
use bevy_widgetry_app_logging::{AppLogging, file_layer, terminal_layer};
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
    let directory = TestDirectory::new()?;
    let logs = directory.0.join("nested/logs");
    let logging = AppLogging::new(&logs)?;
    let files = fs::read_dir(&logs)?.collect::<std::io::Result<Vec<_>>>()?;
    assert_eq!(files.len(), 1);
    let path = files[0].path();
    let name = files[0].file_name().to_string_lossy().into_owned();
    assert_eq!(name.len(), 27);
    assert_eq!(&name[23..], ".log");
    for (index, byte) in name.bytes().take(23).enumerate() {
        match index {
            4 | 7 | 13 | 16 | 19 => assert_eq!(byte, b'-'),
            10 => assert_eq!(byte, b'_'),
            _ => assert!(byte.is_ascii_digit()),
        }
    }

    let mut app = App::new();
    let guard = logging.install(&mut app);
    app.add_plugins(LogPlugin {
        fmt_layer: terminal_layer,
        custom_layer: file_layer,
        ..default()
    });
    let line = line!() + 1;
    info!("应用日志输出验证");
    info!(target: "bevy_widgetry", "Widgetry target 输出验证");
    drop(app);
    info!("guard 释放前的最后一条日志");
    drop(guard);

    let contents = fs::read_to_string(path)?;
    assert!(contents.contains(&format!("app_logging.rs:{line}")));
    assert!(contents.contains("应用日志输出验证"));
    assert!(contents.contains("Widgetry target 输出验证"));
    assert!(contents.contains("guard 释放前的最后一条日志"));
    assert!(!contents.contains('\u{1b}'));
    assert!(!contents.contains("bevy_widgetry"));
    let timestamp = contents.lines().next().ok_or("日志为空")?;
    assert_eq!(timestamp.as_bytes()[10], b' ');
    assert_eq!(timestamp.as_bytes()[19], b'.');
    assert!(timestamp.as_bytes()[20..23].iter().all(u8::is_ascii_digit));
    Ok(())
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
    let logging = AppLogging::new(&logs)?;
    assert_eq!(fs::read_dir(&logs)?.count(), 1);

    let mut app = App::new();
    let guard = logging.install(&mut app);
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
    assert_eq!(failure.severity(), Severity::Error);
    assert!(failure.to_string().contains(&original.to_string()));
    assert_eq!(fs::read_to_string(&occupied)?, "原有文件");
    Ok(())
}
