use bevy::log::{BoxedFmtLayer, BoxedLayer, LogPlugin};
use bevy::prelude::*;
use std::fs::{self, OpenOptions};
use std::path::Path;
use time::{OffsetDateTime, UtcOffset, macros::format_description};
use tracing_appender::non_blocking::{NonBlocking, WorkerGuard};
use tracing_subscriber::fmt::time::OffsetTime;

/// 在启动工作线程之前获取时区并创建文件，失败时阻止 Gallery 启动。
pub(crate) struct GalleryLogging {
    /// 两个输出层共享的启动时本地时区与文件 writer。
    output: LogOutput,
    /// 必须由 main 持有到 app.run 返回，确保退出时刷新队列。
    guard: WorkerGuard,
}

/// 通过 App 向 Bevy LogPlugin 的函数指针回调传递配置。
#[derive(Resource)]
struct LogOutput {
    /// 本次运行固定使用的本机 UTC offset。
    offset: UtcOffset,
    /// 后台线程文件输出，两层共用 Bevy 的过滤器。
    writer: NonBlocking,
}

/// 使用 Bevy 初始化唯一 subscriber，终端和文件仅在呈现方式上不同。
pub(crate) fn log_plugin() -> LogPlugin {
    LogPlugin {
        fmt_layer: terminal_layer,
        custom_layer: file_layer,
        ..default()
    }
}

/// 使用固定启动 offset 输出毫秒时间，与文件名保持相同时区。
fn timer(
    offset: UtcOffset,
) -> OffsetTime<&'static [time::format_description::BorrowedFormatItem<'static>]> {
    OffsetTime::new(
        offset,
        format_description!("[year]-[month]-[day] [hour]:[minute]:[second].[subsecond digits:3]"),
    )
}

/// 终端保留等级颜色和结构化字段，隐藏 target 与源码位置。
fn terminal_layer(app: &mut App) -> Option<BoxedFmtLayer> {
    let output = app.world().resource::<LogOutput>();
    Some(Box::new(
        tracing_subscriber::fmt::layer()
            .with_timer(timer(output.offset))
            .with_target(false)
            .with_file(false)
            .with_line_number(false)
            .with_writer(std::io::stderr),
    ))
}

/// 文件输出纯文本并附带调用位置，使用同一全局 EnvFilter。
fn file_layer(app: &mut App) -> Option<BoxedLayer> {
    let output = app.world().resource::<LogOutput>();
    Some(Box::new(
        tracing_subscriber::fmt::layer()
            .with_timer(timer(output.offset))
            .with_target(false)
            .with_file(true)
            .with_line_number(true)
            .with_ansi(false)
            .with_writer(output.writer.clone()),
    ))
}

impl GalleryLogging {
    /// 使用仅新建语义避免覆盖旧日志；时区、目录、文件或格式错误直接交回 main。
    pub(crate) fn new() -> Result<Self> {
        let offset = UtcOffset::current_local_offset()?;
        let name = OffsetDateTime::now_utc()
            .to_offset(offset)
            .format(format_description!(
                "[year]-[month]-[day]_[hour]-[minute]-[second]-[subsecond digits:3].log"
            ))?;
        let directory = Path::new(env!("CARGO_MANIFEST_DIR")).join("logs");
        fs::create_dir_all(&directory)?;
        let file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(directory.join(name))?;
        let (writer, guard) = tracing_appender::non_blocking(file);
        Ok(Self {
            output: LogOutput { offset, writer },
            guard,
        })
    }

    /// 把 layer 所需配置交给 App，并把刷新 guard 的所有权交回 main。
    pub(crate) fn install(self, app: &mut App) -> WorkerGuard {
        app.insert_resource(self.output);
        self.guard
    }
}
