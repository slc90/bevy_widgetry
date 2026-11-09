# diagnostics 测试覆盖模型

本模型记录当前自动化测试保护的主要行为契约与测试职责，不逐项列举测试用例。

## States
- 已捕获的源头诊断、追加 Context、共享 Origin／ID，以及传播、显式处理和最终宿主报告阶段。

## Stimuli
- 构造领域错误、包装原生 source、跨线程共享、经 System／Observer／Command 转发及日志捕获。

## Guards
- 普通格式化不得泄漏完整回溯；观察或 Drop 不能冒充已处理；原生 BevyError 的 Severity／Panic 策略必须保留。

## Transitions
- 错误捕获 → 追加上下文／共享 → ECS 宿主传播 → 显式处理或终结报告；最后的拥有者释放后源头身份可释放。

## Invariants
- 原始 source／downcast 链、Context、调用位置、Origin 和 ID 在传播及并发中保持一致。
- 独立失败即使文案相同也有不同身份；同一受管 Origin 的报告在并发终结时去重，但报告状态与 handled 独立。
- 源头 ERROR、纯传播静默及实际处理 DEBUG 按相同 ID 关联；宿主仍可按原始 Severity 和 payload 处理原生错误。

## Couplings
- Origin 身份 × 错误共享；ECS Handler × 受管／原生错误；Tracing Subscriber × 进程／线程边界。

## Coverage Map

### Integration Test

| 测试文件 | 主要测试职责 |
|---|---|
| `ecs_bridge.rs` | Bevy System／Observer／Command 的 Error／Panic 传播及原始对象保留。 |
| `lifecycle.rs` | 诊断产生、Context、共享、释放和自定义 reporter。 |
| `log_lifecycle.rs` | 独立子进程中真实源头、处理、兜底日志的生命周期。 |

### Unit Test

| 源码测试模块 | 主要测试职责 |
|---|---|
| `src/origin.rs` | 诊断 ID 计数与跨字长进位的局部正确性。 |
