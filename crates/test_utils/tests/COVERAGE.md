# test_utils 测试覆盖模型

本模型记录当前自动化测试保护的主要行为契约与测试职责，不逐项列举测试用例。

## States
- Mouse／Custom Pointer 的位置、Hover／Pressed／Drag／Cancel，Window／Target 身份和 Picking 命中。
- 日志 target／嵌套捕获作用域、错误源头／处理阶段与诊断生命周期。

## Stimuli
- 构造并入队 Pointer 事件、推进真实 UI Picking／输入 Pipeline、销毁 Target／Window，捕获 Tracing 和宿主错误。

## Guards
- Fixture 入队不得隐式推进 App，也不得直接伪造最终 Hover／Activate；Disabled 目标不应产生 Activate。

## Transitions
- Pointer 进入／离开、Click／Double Click、Drag／Cancel 和目标失效后的重新命中；诊断从源头到实际处理／兜底。

## Invariants
- Pointer ID、位置、Payload、Target 在真实 Backend 中保持一致；Parent／Child 事件消费不重复激活。
- Window／Target 隔离，销毁 Entity 不破坏 Pipeline；测试使用精确 target／dispatcher 捕获真实异步事件。
- 错误记录按 ID、原因和阶段关联，线程／子进程捕获相互隔离；真实处理与纯传播明确区分。

## Couplings
- Pointer 构造 × 原生 Picking／Pressed；嵌套事件消费 × 业务目标；Tracing Dispatcher × 线程／错误生命周期。

## Coverage Map

### Integration Test

| 测试文件 | 主要测试职责 |
|---|---|
| `diagnostic_capture.rs` | 日志 target、线程与嵌套作用域、错误 Context 捕获。 |
| `diagnostic_lifecycle.rs` | 独立子进程的真实源头／处理／兜底日志和宿主错误关联。 |
| `pointer_pipeline.rs` | 真实 Bevy Picking Pipeline 的 Mouse／Custom 输入 Fixture。 |

### Unit Test

| 源码测试模块 | 主要测试职责 |
|---|---|
| `src/benchmark.rs` | Benchmark Baseline 命名与目录安全、禁止覆盖既有数据、命令参数保留及真实文本 Fixture。 |
| `src/benchmark/artifact.rs` | Benchmark 诊断保留原始错误原因、源头回溯和 ECS Error Severity。 |
| `src/logging.rs` | 跨线程 Tracing Callsite 注册情况下的局部日志捕获与作用域隔离。 |
| `src/scene.rs` | 键盘输入入队与 Window 身份、UI Camera 尺寸／缩放、条件等待和超时辅助行为。 |
| `src/theme.rs` | Theme 辅助 API 的正常／同值调用、缺失资源与无效调色板、排队校验和错误身份／通知语义。 |
