//! 覆盖 facade 宏在调用方 package 中嵌入 PNG 并安装 Plugin 的组合边界。
//! 无效嵌入内容在安装前返回 Severity::Error，不需要运行时文件或 AssetServer。

// 断言用于让 contract 违反时测试失败，生产代码的 disallowed_macros 不适用于此测试文件。
#![allow(clippy::disallowed_macros)]

use bevy::ecs::error::Severity;
use bevy::prelude::*;

#[test]
fn facade_embeds_the_calling_package_png() {
    let mut app = App::new();
    app.add_plugins(bevy_widgetry::window::taskbar_icon!("tests/fixtures/taskbar.png").unwrap());
}

#[test]
fn facade_rejects_embedded_non_png() {
    let error = bevy_widgetry::window::taskbar_icon!("tests/taskbar_icon.rs")
        .err()
        .unwrap();
    assert_eq!(error.severity(), Severity::Error);
}
