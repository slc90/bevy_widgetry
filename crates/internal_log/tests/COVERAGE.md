# internal_log 测试覆盖模型

本模型记录当前自动化测试保护的主要行为契约与测试职责，不逐项列举测试用例。

## States
- 当前线程的 Subscriber 捕获状态、各等级 Log Record 及其字段。

## Stimuli
- 调用统一五级日志 Macro，携带中文正文、英文结构化字段和调用位置。

## Guards
- Macro 不自行安装或替换全局 Subscriber。

## Transitions
- 不同等级日志调用映射到对应的 Tracing 记录。

## Invariants
- 日志固定 Target 为 `bevy_widgetry`，正确保留 Level、Message、Field 和源文件位置；TRACE 仍遵循相同契约。

## Couplings
- Macro 展开 × 宿主 Subscriber 的捕获与过滤。

## Coverage Map

### Integration Test

| 测试文件 | 主要测试职责 |
|---|---|
| `macros.rs` | 统一日志 Macro 的等级、Target、字段和调用位置。 |
