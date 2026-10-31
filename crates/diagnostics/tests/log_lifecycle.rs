#![allow(clippy::disallowed_macros)]

use bevy_widgetry_diagnostics::{ErrorFrame, WidgetryFailure};
use std::{io, sync::Arc};

#[derive(Debug)]
struct Outer(io::Error);
impl std::fmt::Display for Outer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("文档失败")
    }
}
impl std::error::Error for Outer {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(&self.0)
    }
}

#[derive(Clone, Default)]
struct Buffer(Arc<std::sync::Mutex<Vec<u8>>>);

impl io::Write for Buffer {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        self.0
            .lock()
            .map_err(|_| io::Error::other("测试日志缓冲区锁中毒"))?
            .extend_from_slice(bytes);
        Ok(bytes.len())
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

#[test]
fn source_logs_once_propagation_is_silent_and_handling_is_explicit() {
    let buffer = Buffer::default();
    let writer = buffer.clone();
    let subscriber = tracing_subscriber::fmt()
        .without_time()
        .with_ansi(false)
        .with_max_level(tracing::Level::DEBUG)
        .with_writer(move || writer.clone())
        .finish();
    tracing::subscriber::with_default(subscriber, || {
        let failure = WidgetryFailure::new(
            Outer(io::Error::other("磁盘失败")),
            ErrorFrame::new("读取").field("path", "abc.bin"),
        );
        let id = failure.origin().id().to_string();
        let failure = Arc::new(failure.with_context(ErrorFrame::new("展示")));
        let before_handling = String::from_utf8(buffer.0.lock().unwrap().clone()).unwrap();
        assert_eq!(before_handling.lines().count(), 1);
        assert!(before_handling.contains("ERROR bevy_widgetry"));
        assert!(before_handling.contains("操作失败"));
        assert!(before_handling.contains("磁盘失败"));
        assert!(before_handling.contains("abc.bin"));
        assert!(!before_handling.contains("Backtrace"));
        failure.record_handled("取消读取", "已保留旧文档");
        let another_owner = Arc::clone(&failure);
        another_owner.record_handled("取消读取", "已保留旧文档");
        #[cfg(feature = "bevy")]
        {
            use bevy_widgetry_diagnostics::bevy::{Diagnostic, WidgetryEcsError};
            let shared: Arc<dyn Diagnostic> = another_owner;
            let _boundary = WidgetryEcsError::from_shared(shared)
                .with_context(ErrorFrame::new("执行边界"))
                .into_bevy_error();
        }
        let logged = String::from_utf8(buffer.0.lock().unwrap().clone()).unwrap();
        let lines: Vec<_> = logged.lines().collect();
        assert_eq!(lines.len(), 2);
        assert!(lines[1].contains("DEBUG bevy_widgetry"));
        assert!(lines[1].contains("action=\"取消读取\""));
        assert!(lines[1].contains("outcome=\"已保留旧文档\""));
        assert!(
            lines
                .iter()
                .all(|line| line.contains(&format!("error_id={id}")))
        );
        assert!(!lines[1].contains("磁盘失败"));
    });
}
