# app_logging Integration Test Coverage

本覆盖模型描述现有 Integration Test 重点保护的可观察行为及测试职责，不是逐项 Test Case 清单。

## States

- 输出未安装 / 已安装；目标目录不存在 / 可创建 / 不可写。
- Terminal 与 File Layer 准备及日志 Guard 生命周期。

## Stimuli

- 构造 `AppLogging`、调用 `install`，装配 Bevy `LogPlugin` 并写入日志。
- 未安装时调用 Layer Factory；结束时释放 Guard。

## Guards

- 目录与文件准备失败必须返回错误，不伪造已安装状态。

## Transitions

- 准备目录及文件 → 安装输出层 → 记录日志 → Guard 释放后刷新。
- 缺少安装时 Factory 返回 None；准备失败走 Error 传播。

## Invariants

- 文件输出保留日志来源位置、时间戳和结构化信息。
- 普通准备失败保留领域 stage/path/origin/source，ECS 显式转换保留 Error Severity；退出前日志能够刷新。

## Couplings

- 应用目录与输出 Layer 的装配顺序。
- Bevy `LogPlugin`、文件 Writer 与 Guard 的完成时机。

## Coverage Map

- `app_logging.rs`：完整应用日志准备、集成输出、失败与清理生命周期。


完整 LogPlugin 的全局 Subscriber 测试在无 RUST_LOG 的子进程执行，文件保留库和宿主普通 DEBUG、必要处理诊断、INFO/WARN/ERROR、真实多行栈、最终日志与退出 flush；两个 Target 的 TRACE 默认不落盘。writer 模块单测使用容量1与慢writer、多生产者检验背压下200条完整事件不丢弃，覆盖写失败/零写/flush失败、队列断连与受控flush超时，不通过失效后端递归报告。初始化与 install 不配置 ECS Handler。
