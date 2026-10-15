# app_logging

## 功能概述

为桌面应用准备 Terminal 与 File 日志输出环境。宿主可以选择使用这一能力，也可以自行配置日志系统。

## 当前设计思路

库负责输出层所需的文件、Writer 和本地时间，不替宿主安装全局 Subscriber。`AppLogging` 的准备与 `LogPlugin` 的装配彼此分离，避免把日志输出策略写死在 Widgetry 库中。

宿主可启用 Facade 的独立非默认 `app_logging` Feature，通过 `bevy_widgetry::app_logging` 使用 `AppLogging`、`file_layer`、`terminal_layer` 和 `WorkerGuard`。实现仍由本 crate 承担，启用 Feature 不会自动安装 Subscriber 或替宿主持有 Guard。

## 主要功能

- `AppLogging::new` 准备指定目录及带本地时间戳的日志文件。
- `install` 提供配套输出层可使用的资源，并返回需要由应用持有的 `WorkerGuard`。
- `terminal_layer` 输出到 stderr；`file_layer` 输出到文件并保留源码位置。
- 两种输出采用本地时间和毫秒精度；文件输出不包含 ANSI 控制序列。

## 行为与状态契约

宿主先 `install`，再通过官方 Bevy `LogPlugin` 接入输出层，并持有 `WorkerGuard` 直到应用退出，以保证排队日志得以刷新。没有 `install` 时，Layer Factory 返回 `None`，而不是自行初始化另一套日志系统。

## 边界情况与错误处理

日志目录、文件准备及本地时间相关失败通过 `Result` 报告，保留失败原因与 `Severity::Error`；宿主决定后续行为。

## 已知限制

- 不安装 Subscriber，也不提供 Widgetry 日志开关或 Plugin。
- 不决定宿主的过滤策略和日志目录。
- 属于应用级输出基础设施，不是 `internal_log` crate 的诊断 Macro 替代品。
