# Internal Log

## 功能概述

提供 Widgetry 内部统一的日志 Macro，用于记录少量生命周期事实与必要的运行失败诊断。

## 当前设计思路

这是极薄的日志适配层，生产依赖只有 Tracing，不引入 Bevy 或 UI，不安装 Subscriber，也不持有业务 Log State。Macro 沿用 Tracing 调用方式，与 Bevy 的 Tracing Subscriber 兼容，同时把 Target 统一为 `bevy_widgetry`。

## 主要功能

- `widgetry_debug!`、`widgetry_info!`、`widgetry_warn!`、`widgetry_error!` 分别发出 Debug、Info、Warn、Error 记录。
- 支持普通 Message 与 Structured Field。
- 保留调用位置元数据，方便定位实际产生日志的代码。

## 行为与状态契约

日志的等级与 Target 由 Macro 确定；日志是否显示、格式和输出目的地由宿主配置的 Subscriber 决定。Macro 不改变错误传播契约；遇到真实失败时，产生日志和向上传递 `BevyError` 是不同职责。

## 边界情况与错误处理

没有 Subscriber 或 Filter 排除对应 Target 时，Macro 不会自行创建输出目的地。库的日志触发条件和去重规则由 `rules/logging.md` 约束。

## 已知限制

- 不是通用日志框架，不提供文件管理、Filter 或 Subscriber 安装。
- Debug 只用于真实错误的实际处理措施与结果，不作为正常执行轨迹。
- Gallery 的应用日志使用官方 Bevy Macro，而不是这些内部 Macro。

