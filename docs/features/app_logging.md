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

终端与文件均使用本地时间和毫秒精度，并保留产生事件时的 Rust 线程名及线程 ID。文件另外包含模块、文件名与行号，例如 `bevy_app::app (app.rs:528)`，不输出事件元数据中的源码目录，也不写 ANSI 控制字符。模块缺失时使用事件 target，文件或行号缺失时仅省略对应信息。事件字段与 Span 上下文照常保留。

```rust
info!(operation = "open", "开始打开记录");
debug!(job_id = 12_u64, phase = "finished", "后台任务已退出");
```

记录中的线程信息属于事件生产线程，不会因为异步写文件而变成日志工作线程。多行回溯可以作为正式诊断的一部分写入文件。

### 诊断文本的安全显示

`safe_location(origin.location())` 与 `safe_backtrace(origin.backtrace())` 提供借用的 Display 视图。位置保留可识别的仓库内相对路径或文件名及行列号；回溯显示全部可用帧，保留符号、地址、帧顺序和行列号，将源码目录改为安全相对标识。Cargo registry 来源保留 crate 与版本，工具链来源保留 `library/...`；不明源码位置使用文件名或 `[source]`。

`safe_diagnostic(value)` 接收 Display 值，也可包装 `format_args!("{error:?}")`，用于最终报告与日志尚未安装时的应急 stderr。上述视图不修改原始 Location、Backtrace、错误身份或传播上下文，也不重新捕获回溯。Release 下能解析多少源码位置仍取决于可用符号。

文件字段在入队前使用相同规则处理嵌套回溯、Debug 来源位置、带行号的源码路径与已知 Cargo／工具链源码路径。普通配置、资源和用户文件的运行时绝对路径保留；此能力不对任意业务字符串中的绝对路径作无差别删除，也不清除 EXE 或 PDB 内的编译路径。终端的第三方任意字段由宿主决定，项目管理的应急出口使用安全视图独立输出。

## 刷新与关闭

### 显式结束文件写入

`LoggingGuard::finish()` 消费 Guard，等待已接收记录写入并刷新，返回真实的输出故障。应用应先结束仍会产生日志的后台工作，再完成这一阶段。

队列采用不主动丢弃的策略。持续写入速度超过后端能力时可能形成等待，因此日志不是无限容量的内存缓冲。

Guard 的 Drop 提供最终收尾，但无法把返回值交给调用方。需要在退出前明确显示失败时，使用 `finish()`，不要只依赖 Drop。

## 输出故障

初始化失败包括时间信息、目录、文件和资源准备错误；写入期间可能发生队列、写盘、刷新或刷新超时错误。`LoggingFailure` 保留操作阶段、路径和真实原因。

文件后端自身坏掉时，不能再依靠同一个后端报告它的错误。此时使用独立应急输出，并把故障保留在 Guard 的结果中。应用仍负责最终提示或退出处理。
