# 内部日志

`bevy_widgetry_internal_log` 提供统一 Target 的 Tracing 宏，供库内部和基础工具记录生命周期事实。它不配置输出、不持有业务状态，也不安装 Subscriber。

## 日志记录

### 写入消息与结构化字段

```rust
use bevy_widgetry_internal_log::{widgetry_debug, widgetry_info};

widgetry_info!("资源注册完成");
widgetry_debug!(request_id = 17_u64, count = 32_usize, "批次处理完成");
```

提供 `widgetry_trace!`、`widgetry_debug!`、`widgetry_info!`、`widgetry_warn!` 和 `widgetry_error!`。字段和消息沿用 Tracing 的调用形式，Target 统一为 `bevy_widgetry`，位置元数据对应实际调用处。

### 输出对象与状态

```rust
use bevy_widgetry_internal_log::widgetry_debug;

widgetry_debug!(entity = ?entity, phase = "closed", "控件会话结束");
```

`?value` 使用 Debug，`%value` 使用 Display。生命周期日志应对应真实变化；一次查询不等于任务启动，一次取消请求也不等于任务退出。

## 输出接入

应用安装的 Tracing Subscriber 决定哪些记录可见、采用什么格式、写到哪里。Bevy 的日志接收链路可以接收这些记录，也可以组合 [应用日志](app_logging.md) 的终端与文件输出。

没有 Subscriber 时，调用宏不会自动创建日志文件。内部日志宏也不会替应用关闭或刷新输出。

## 与诊断的分工

普通日志描述已发生的事实；需要向上传播的失败使用 [Diagnostics](diagnostics.md)。单独记录 ERROR 不会让 System 返回失败，也不会自动保存一条可供调用方处理的类型化错误链。

例如“加载结果已采用”可以是一条 DEBUG；实际 I/O 失败则应先保留原始错误，再由诊断链负责传播，不只输出一段字符串。
