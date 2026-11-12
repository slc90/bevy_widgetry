# 应用日志

应用日志提供终端和文件两种输出，并将文件写入放到后台线程。通过 Facade 使用时开启 `app_logging` Feature。输出目录和日志生命周期由应用决定。

## 日志装配

### 创建输出并安装 Bevy 日志插件

```rust
use bevy::prelude::*;
use bevy_widgetry::app_logging::{AppLogging, log_plugin};

fn run() -> Result {
    let mut app = App::new();
    let logging = AppLogging::new("logs")
        .map_err(|failure| failure.into_bevy_error())?;
    let guard = logging.install(&mut app);

    app.add_plugins(DefaultPlugins.set(log_plugin()));
    app.add_systems(Startup, || info!("应用启动"));
    app.run();

    guard.finish().map_err(|failure| {
        bevy_widgetry::diagnostics::bevy::WidgetryEcsError::from_shared(failure)
            .into_bevy_error()
    })?;
    Ok(())
}
```

`AppLogging::new` 创建目录和带毫秒时间戳的日志文件；`install` 把输出资源交给 App，并把 Guard 返回给宿主。Guard 要保留到所有需要记录的工作结束后。

`DefaultPlugins.set(log_plugin())` 替换默认 LogPlugin 配置，不能再重复安装第二个 LogPlugin。只调用 `install` 不会安装全局 Subscriber。

### 自行组合输出层

应用已有日志装配时，可以采用 `file_layer` 和 `terminal_layer`，而不使用完整的 `log_plugin()`。两者都从 App 中读取已安装的日志输出资源，因此仍需先执行 `install`。

默认文件接收 DEBUG 及以上，终端接收 INFO 及以上，TRACE 关闭。外层过滤必须允许需要交付给各层的记录；`RUST_LOG` 等宿主过滤仍会影响最终结果。

## 输出内容

终端与文件均使用本地时间和毫秒精度，并保留产生事件时的 Rust 线程名及线程 ID。文件另外包含模块、文件名与行号，例如 `bevy_app::app (app.rs:528)`，不输出事件元数据中的源码目录，也不写 ANSI 控制字符。模块缺失时使用事件 target，文件或行号缺失时仅省略对应信息。事件字段与 Span 上下文照常保留；字段中单独携带的诊断文本不属于事件来源元数据。

```rust
info!(operation = "open", "开始打开记录");
debug!(job_id = 12_u64, phase = "finished", "后台任务已退出");
```

记录中的线程信息属于事件生产线程，不会因为异步写文件而变成日志工作线程。多行回溯可以作为正式诊断的一部分写入文件。

## 刷新与关闭

### 显式结束文件写入

`LoggingGuard::finish()` 消费 Guard，等待已接收记录写入并刷新，返回真实的输出故障。应用应先结束仍会产生日志的后台工作，再完成这一阶段。

队列采用不主动丢弃的策略。持续写入速度超过后端能力时可能形成等待，因此日志不是无限容量的内存缓冲。

Guard 的 Drop 提供最终收尾，但无法把返回值交给调用方。需要在退出前明确显示失败时，使用 `finish()`，不要只依赖 Drop。

## 输出故障

初始化失败包括时间信息、目录、文件和资源准备错误；写入期间可能发生队列、写盘、刷新或刷新超时错误。`LoggingFailure` 保留操作阶段、路径和真实原因。

文件后端自身坏掉时，不能再依靠同一个后端报告它的错误。此时使用独立应急输出，并把故障保留在 Guard 的结果中。应用仍负责最终提示或退出处理。
