# test_utils Integration Test Coverage

本覆盖模型描述现有 Integration Test 重点保护的可观察行为及测试职责，不是逐项 Test Case 清单。

## States

- Mouse / Custom Pointer 的位置、Hover、Press、Drag、Cancel 和失效。
- 日志 target 选择、线程/嵌套捕获作用域，错误源头、处理与兜底阶段及诊断拥有者生命周期。
- 多 Window / Target 与真实 Picking 命中状态。

## Stimuli

- Pointer Event Constructor、Queue、按帧推进真实 UI Picking Backend。
- Moving / Press / Release / Drag / Cancel、Target Despawn 与 Window Identity。

## Guards

- Disabled 目标不能 Activation。
- 输入队列不能隐式推进 App；测试不得直接伪造最终 Hover / Activate 结果。

## Transitions

- Pointer 进入 / 离开、正常 Click / Double Click、Drag 终点和取消。
- Window / Target 失效后重新计算命中。

## Invariants

- PointerId、Location、Payload、Target 被正确转交。
- Parent / Child 的事件消费不造成重复业务操作。
- 不同 Native Target 隔离，失效 Entity 不破坏 Pipeline。

## Couplings

- 输入构造 × 实际 Picking 与官方 Pressed 生命周期。

## Coverage Map

- `pointer_pipeline.rs`：受控 Mouse / Custom Input 与 Bevy Picking Backend 的集成 Fixture Contract。
- `diagnostic_capture.rs`：精确 target、DEBUG 与字段/ID/阶段查询、显式线程和 future dispatcher、不同线程捕获隔离、嵌套错误作用域及真实原生/受管错误 context。
- `diagnostic_lifecycle.rs`：无回溯环境变量的独立子进程安装真实全局 Subscriber，通过 scene_app + Commands 捕获批量诊断和 ErrorContext；源头/兜底按 ID 关联，纯传播静默、显式处理释放、同文案独立错误与原始源头栈。

## 覆盖边界

Fixture 验证的是 ECS / UI Pipeline，不涉及真实操作系统 Mouse。

