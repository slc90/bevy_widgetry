# Internal Log

## 功能概述

提供 Widgetry 内部统一的日志 Macro，用于记录少量生命周期事实与必要的运行失败诊断。

## 当前设计思路

这是极薄的日志适配层，生产依赖只有 Tracing，不引入 Bevy 或 UI，不安装 Subscriber，也不持有业务 Log State。Macro 沿用 Tracing 调用方式，与 Bevy 的 Tracing Subscriber 兼容，同时把 Target 统一为 `bevy_widgetry`。

## 主要功能

- `widgetry_trace!`、`widgetry_debug!`、`widgetry_info!`、`widgetry_warn!`、`widgetry_error!` 分别发出 Trace、Debug、Info、Warn、Error 记录。
- 支持普通 Message 与 Structured Field。
- 保留调用位置元数据，方便定位实际产生日志的代码。

## 行为与状态契约

日志的等级与 Target 由 Macro 确定；日志是否显示、格式和输出目的地由宿主配置的 Subscriber 决定。Macro 不改变错误传播契约；遇到真实失败时，产生日志和向上传递 `BevyError` 是不同职责。

## 边界情况与错误处理

没有 Subscriber 或 Filter 排除对应 Target 时，Macro 不会自行创建输出目的地。库的日志触发条件和去重规则由 `rules/logging.md` 约束。

## 已知限制

- 不是通用日志框架，不提供文件管理、Filter 或 Subscriber 安装。
- Debug 用于有诊断价值的程序生命周期、内部执行阶段、状态转换及真实错误的实际处理，不记录普通函数调用和逐帧同步。
- Widgetry Plugin 在 `build()` 完成时使用 Debug 记录一次注册完成；泛型 Plugin 每个成功注册的实例分别记录，重复注册不重复输出。
- Trace 仅用于临时开发诊断，正式提交前必须移除实现代码中的调用；宏定义保留，由 Workspace Clippy `disallowed_macros` 禁止遗留调用。
- Gallery 的应用日志使用官方 Bevy Macro，而不是这些内部 Macro。

