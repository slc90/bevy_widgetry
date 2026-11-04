#![allow(clippy::disallowed_macros, clippy::expect_used, clippy::unwrap_used)]

use bevy::log::Level;
use bevy_widgetry_internal_log::{
    widgetry_debug, widgetry_error, widgetry_info, widgetry_trace, widgetry_warn,
};
use bevy_widgetry_test_utils::LogCapture;

#[test]
fn macros_preserve_fields_levels_and_call_site() {
    let capture = LogCapture::default();
    capture.run(|| {
        widgetry_info!(count = 3, "插件注册完成");
        widgetry_warn!(path = %"图标.svg", error = ?"失败", "资源处理失败");
        widgetry_error!(entity = ?42, "内部状态缺失");
        widgetry_debug!(error_id = %"abc", action = "取消操作", "错误处理完成");
        widgetry_trace!(iteration = 7, "临时诊断");
    });
    let records = capture.records();
    assert_eq!(records.len(), 5);
    for (record, level) in records.iter().zip([
        Level::INFO,
        Level::WARN,
        Level::ERROR,
        Level::DEBUG,
        Level::TRACE,
    ]) {
        assert_eq!(record.target, "bevy_widgetry");
        assert_eq!(record.level, level);
        assert_eq!(record.file.as_deref(), Some(file!()));
        assert!(record.line.is_some());
        assert!(!record.fields.contains_key("file"));
    }
    assert_eq!(records[0].fields["count"], "3");
    assert!(records[1].fields["path"].contains("图标.svg"));
    assert_eq!(records[2].fields["entity"], "42");
    assert_eq!(records[3].fields["error_id"], "abc");
    assert!(records[3].fields["message"].contains("错误处理完成"));
    assert_eq!(records[4].fields["iteration"], "7");
    assert!(records[4].fields["message"].contains("临时诊断"));
}
