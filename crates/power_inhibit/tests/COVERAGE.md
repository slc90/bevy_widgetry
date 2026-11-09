# power_inhibit 测试覆盖模型

本模型记录当前自动化测试保护的主要行为契约与测试职责，不逐项列举测试用例。

## States
- DISPLAY／SYSTEM 两种抑制请求、原生句柄的开启／关闭、Resource 存活与 App 生命周期。

## Stimuli
- 公开 Resource API 的开关、重建、App 释放，以及原生 FFI 失败或清理失败。

## Guards
- 原生创建／设置／清理失败必须保留错误和句柄状态，不能报告虚假成功；重复请求遵循幂等语义。

## Transitions
- 创建句柄 → 开启／关闭两类请求 → Resource 移除或 App 结束时释放；部分失败与回滚后允许后续重试。

## Invariants
- 每类请求计数和句柄生命周期保持一致；释放后不得继续使用旧 Win32 句柄。
- 主操作与 Cleanup 失败分别保留原生 source、origin、请求状态及宿主传播身份；Drop 的错误处理在句柄释放后发生。
- 无窗口 Plugin／时钟依赖时，公开 Resource 仍可独立工作。

## Couplings
- DISPLAY × SYSTEM；原生操作 × 回滚／Drop；Resource 生命周期 × 跨线程句柄。

## Coverage Map

### Integration Test

| 测试文件 | 主要测试职责 |
|---|---|
| `resource.rs` | 公开 Resource 的抑制切换、重建、跨生命周期行为。 |

### Unit Test

| 源码测试模块 | 主要测试职责 |
|---|---|
| `src/power/tests.rs` | 私有 Win32 FFI 句柄、失败注入、回滚、重试及资源释放。 |
