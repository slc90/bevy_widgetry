use bevy::log::Level;
use bevy_widgetry_log::{widgetry_error, widgetry_info, widgetry_warn};
use bevy_widgetry_test_utils::LogCapture;

// 三种宏固定同一 target，保留结构化语法，并把位置归属到调用者而不是宏 crate。
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
