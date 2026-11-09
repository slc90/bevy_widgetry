# job 测试覆盖模型

本模型记录当前自动化测试保护的主要行为契约与测试职责，不逐项列举测试用例。

## States
- Pending、Running、Stopping 与 Completed／Canceled／Failed 等终态；Io／Cpu／Thread 后端。
- Broadcast Subscription、缓冲背压、Job Registry、Shutdown／TimedOut 和唤醒通知状态。

## Stimuli
- 提交、取消／确认取消、发送／订阅／接收、异步错误或 panic、线程退出与 Join、Shutdown／超时及 Plugin／Wake 事件。

## Guards
- 未知或不可订阅／移除的 Job 必须按公开契约处理；有效取消确认只能发生在允许的业务执行期。
- 容量为零等非法配置不得接受；关闭／提交竞争只能纳管或拒绝，不能遗留不可收割任务。

## Transitions
- 注册 → 启动 → 自然终结或 Stopping → 终态；关闭请求 → 协调取消 → 超时／真实收割完成。

## Invariants
- Job ID 唯一，Manager 实例隔离，终态唯一且不可逆；失败和取消裁决不因观察时机或背压倒退。
- 请求取消与确认取消不同：未确认的成功为 Completed，已确认成功为 Canceled，执行错误／panic 仍为 Failed。
- Broadcast 订阅隔离，先交付已接受 Item 再唯一 Finished；慢订阅的背压不会丢弃消息，也不阻止任务进入终态。
- Thread 后端以真实 Join／TLS 析构完成作为结束依据；Shutdown 超时不提前释放运行体，迟到结束仍可完成收割。
- Shutdown 幂等、唯一截止时间和通知去重；Headless App 能继续 Update，不依赖宿主无限轮询。
- Job／Wake 原始错误、跨线程 source／origin、受管与原生 BevyError 上下文保持可追踪；诊断记录和业务结果生命周期一致。

## Couplings
- Cancel × Backpressure；Shutdown × Thread Join／TLS；Task Completion × Broadcast；Plugin／Wake × Headless Update；错误传播 × 宿主 Handler。

## Coverage Map

### Integration Test

| 测试文件 | 主要测试职责 |
|---|---|
| `broadcast.rs` | 订阅隔离、消息顺序、容量与背压。 |
| `cancellation.rs` | 取消请求、确认裁决及竞争条件。 |
| `control_diagnostics.rs` | 非法控制／订阅操作的分类、身份与宿主传播。 |
| `execution.rs` | 多后端执行、成功／失败／panic 和状态可见性。 |
| `lifecycle_logging.rs` | 子进程真实生命周期及取消／收割日志。 |
| `outcome_diagnostics.rs` | Job Outcome、原始来源／回溯与消费诊断。 |
| `plugin.rs` | 多实例／组合 Plugin、Headless、资源和唤醒协作。 |
| `shutdown.rs` | 关闭协调、截止时间、超时、迟到收割和幂等。 |
| `threads.rs` | 真实 OS Thread、Join、命名、TLS 和退出资源回收。 |

### Unit Test

| 源码测试模块 | 主要测试职责 |
|---|---|
| `src/manager.rs` | 线程收割与错误隔离的局部资源状态。 |
| `src/wake.rs` | 持续故障的去重、恢复及新故障身份。 |
| `src/lifecycle.rs` | 任务状态与取消／关闭日志时序。 |
