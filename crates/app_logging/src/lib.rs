//! 提供桌面 App 的 terminal 与文件日志输出配置。
//! 调用方指定日志目录，并通过官方 Bevy LogPlugin 使用配套 layer factory。
//!
//! AppLogging 在构造时创建目录和带本地时间戳的日志文件。
//! terminal 输出到 stderr，文件输出保留 source file 和 line number 并禁用 ANSI。
//! 两种输出使用本地时间，精度均为毫秒。
//!
//! 调用方应先 install，再装配 LogPlugin，并持有返回的 WorkerGuard 直到 App 运行结束。
//! 未 install 时，layer factory 返回 None。
//! 目录、文件和本地时区的准备失败通过 Result 返回。

mod logging;

pub use logging::{AppLogging, file_layer, terminal_layer};
pub use tracing_appender::non_blocking::WorkerGuard;
