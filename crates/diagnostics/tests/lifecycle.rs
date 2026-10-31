#![allow(clippy::disallowed_macros)]

use bevy_widgetry_diagnostics::{ErrorFrame, WidgetryFailure};
use std::{backtrace::BacktraceStatus, error::Error, io, sync::Arc};

#[test]
fn propagation_preserves_origin_source_and_owned_context() {
    let line = line!() + 1;
    let failure = WidgetryFailure::new(
        io::Error::other("读取失败"),
        ErrorFrame::new("读取文件").field("path", "测试.bin"),
    );
    assert_eq!(failure.origin().location().line(), line);
    assert_eq!(failure.origin().location().file(), file!());
    let origin = Arc::clone(failure.origin());
    let wrapped = failure
        .map_kind(Outer)
        .with_context(ErrorFrame::new("打开文档").field("request", 7));
    assert!(Arc::ptr_eq(&origin, wrapped.origin()));
    assert_eq!(wrapped.contexts().len(), 2);
    assert_eq!(
        wrapped.contexts()[1].fields()[0],
        ("request".to_owned(), "7".to_owned())
    );
    assert!(wrapped.source().unwrap().downcast_ref::<Outer>().is_some());
    assert!(
        wrapped
            .source()
            .unwrap()
            .source()
            .unwrap()
            .downcast_ref::<io::Error>()
            .is_some()
    );
    let shared = Arc::new(wrapped);
    let other = Arc::clone(&shared);
    let observed = std::thread::spawn(move || other.origin().id().clone())
        .join()
        .unwrap();
    assert_eq!(&observed, origin.id());
    let weak = Arc::downgrade(&origin);
    drop(origin);
    assert!(weak.upgrade().is_some());
    drop(shared);
    assert!(weak.upgrade().is_none());
}

#[derive(Debug)]
struct Outer(io::Error);

impl std::fmt::Display for Outer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("文档失败")
    }
}

impl Error for Outer {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        Some(&self.0)
    }
}

#[test]
fn independent_and_concurrent_failures_have_distinct_ids() {
    let barrier = Arc::new(std::sync::Barrier::new(8));
    let threads: Vec<_> = (0..8)
        .map(|_| {
            let barrier = Arc::clone(&barrier);
            std::thread::spawn(move || {
                barrier.wait();
                (0..32)
                    .map(|_| {
                        WidgetryFailure::new(io::Error::other("同样失败"), ErrorFrame::new("操作"))
                            .origin()
                            .id()
                            .clone()
                    })
                    .collect::<Vec<_>>()
            })
        })
        .collect();
    let ids: Vec<_> = threads
        .into_iter()
        .flat_map(|thread| thread.join().unwrap())
        .collect();
    let unique: std::collections::HashSet<_> = ids.iter().collect();
    assert_eq!(unique.len(), ids.len());
}

#[test]
fn backtrace_is_captured_without_environment() {
    const CHILD: &str = "WIDGETRY_DIAGNOSTICS_CAPTURE_CHILD";
    if std::env::var_os(CHILD).is_some() {
        assert!(std::env::var_os("RUST_BACKTRACE").is_none());
        assert!(std::env::var_os("RUST_LIB_BACKTRACE").is_none());
        let failure = WidgetryFailure::new(io::Error::other("失败"), ErrorFrame::new("子进程"));
        assert_eq!(
            failure.origin().backtrace().status(),
            BacktraceStatus::Captured
        );
        let ordinary = format!("{failure} {failure:?} {:?}", failure.origin());
        assert!(!ordinary.contains("Backtrace"));
        assert!(!ordinary.contains("stack backtrace"));
        return;
    }
    let output = std::process::Command::new(std::env::current_exe().unwrap())
        .args([
            "--exact",
            "backtrace_is_captured_without_environment",
            "--nocapture",
        ])
        .env(CHILD, "1")
        .env_remove("RUST_BACKTRACE")
        .env_remove("RUST_LIB_BACKTRACE")
        .env_remove("BEVY_BACKTRACE")
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{} {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}
