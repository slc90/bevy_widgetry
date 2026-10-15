# log Integration Test Coverage

本覆盖模型描述现有 Integration Test 重点保护的可观察行为及测试职责，不是逐项 Test Case 清单。

## States

- 当前 Thread 的 Subscriber 状态和已捕获的 Log Record。

## Stimuli

- 调用 `widgetry_info!`、`widgetry_warn!`、`widgetry_error!` 并传入 Structured Field。

## Guards

- Macro 不擅自安装全局 Subscriber。

## Transitions

- 不同 Severity 的调用产生相应日志记录。

## Invariants

- 固定 Target `bevy_widgetry`；正确保留 Message、Level、Field 与调用位置。

## Couplings

- Macro 展开与宿主 Subscriber 的捕获机制。

## Coverage Map

- `macros.rs`：统一 Macro 的等级、Target、元数据和 Structured Field。

