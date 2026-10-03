//! State：当前 thread 的 subscriber 与捕获到的 structured log record。
//! Stimuli：widgetry_error!/widgetry_warn!/widgetry_info! 及 structured field 输入。
//! Transitions：每次调用追加对应 severity 的 record。
//! Invariants：target、调用位置、message 和 structured field 保留；macro 不初始化全局 subscriber。

// 测试断言需要在 contract 不满足时立即失败；生产代码的 panic lint 会拒绝这些表达式，因此仅在本测试 scope 允许所列 lint。
#![allow(clippy::disallowed_macros, clippy::expect_used, clippy::unwrap_used)]

use bevy::log::Level;
use bevy_widgetry_log::{widgetry_error, widgetry_info, widgetry_warn};
use bevy_widgetry_test_utils::LogCapture;

#[test]
fn macros_preserve_fields_levels_and_call_site() {
    let capture = LogCapture::default();
    capture.run(|| {
        widgetry_info!(count = 3, "插件注册完成");
        widgetry_warn!(path = %"图标.svg", error = ?"失败", "资源处理失败");
        widgetry_error!(entity = ?42, "内部状态缺失");
    });
    let records = capture.records();
    assert_eq!(records.len(), 3);
    for (record, level) in records.iter().zip([Level::INFO, Level::WARN, Level::ERROR]) {
        assert_eq!(record.target, "bevy_widgetry");
        assert_eq!(record.level, level);
        assert_eq!(record.file.as_deref(), Some(file!()));
        assert!(record.line.is_some());
        assert!(!record.fields.contains_key("file"));
    }
    assert_eq!(records[0].fields["count"], "3");
    assert!(records[1].fields["path"].contains("图标.svg"));
    assert_eq!(records[2].fields["entity"], "42");
}
