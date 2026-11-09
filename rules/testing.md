# 测试规则

## 测试原则

新增或修改可观察行为时，必须有对应测试。修复 Bug 时，应增加能够复现问题并防止回归的确定性 Regression Test。

纯内部重构不要求人为增加测试，但必须确保现有测试能够保护受影响的行为；覆盖不足时应先补充必要测试。

测试应验证 Widgetry 自身的行为、语义和约束：

- 不重复验证 Bevy 或第三方库已经保证的内部行为，但应覆盖 Widgetry 与其真实组合时的重要契约。
- 不编写仅将静态 `const` 与另一份相同的硬编码字面值进行比较的冗余测试。具有独立意义的数值关系、颜色约束和其他 Invariant 仍应测试。
- 不以测试数量、源码 Function 数量或完整 State 笛卡尔积衡量覆盖是否充分。
- 已知且明确的重要行为应使用确定性测试验证，不依赖随机测试碰巧覆盖。

### Gallery 例外

`gallery/` 作为展示应用，不强制编写 Unit Test 或 Integration Test，但必须保证修改后能够正常编译。

涉及可观察 GUI 行为时，应按照 `rules/gui-debugging.md` 进行针对性的 BRP 运行时验证。

Gallery 的行为测试例外不免除适用的性能 Benchmark 要求。

## Unit Test 与 Integration Test

### Unit Test

Unit Test 验证具有独立语义的局部 Contract，包括 Function、Type、Algorithm，以及独立的小型 State 对象或 Model。

根据实际语义覆盖正常输入、边界、Guard、No-op、无效输入、Invariant 和有意义的操作序列。

不要求每个 Private Function 都有 Unit Test，也不以内部实现覆盖率代替行为验证。

Unit Test 放在对应源码 Module 中：

```rust
#[cfg(test)]
mod tests {
}
```

### Integration Test

Integration Test 从 Crate 外部视角验证可观察业务行为、跨 Module 协作及真实 Bevy / ECS 组合。

Integration Test 放在对应 Crate 的 `tests/` 目录中，可以根据实际需要使用 `mod.rs` 组织测试 Module。

复杂模块应根据实际业务状态机设计测试覆盖，重点包括：

- **States**：相互独立的可观察状态维度，例如 Selection、Active、Focus、Disabled 和 Model 状态。
- **Stimuli**：用户输入、公开 API、外部 Model / Resource / Component 变化、Theme 变化、生命周期及真实组合输入。
- **Guards**：决定操作是否允许或如何处理的条件，包括 Disabled、Read-only、无效目标及其他业务限制。
- **Transitions**：重要状态转移，以及相关的正常、边界、重复、拒绝和过期输入路径。
- **Invariants**：跨操作必须保持的性质，例如 Stable ID、有效 Selection、状态提交与变化通知顺序。
- **Couplings**：不同状态维度之间有明确业务联系的组合，例如 Model 删除引起的 Selection Repair。

测试模型只包含具有实际业务意义的状态。Cache、Dirty Flag 等内部实现状态通常不纳入，除非它们承担需要明确保护的行为契约。

各状态维度应覆盖自身的重要转移；只有存在真实业务耦合时才增加组合验证，不机械展开完整笛卡尔积。

重要 Invariant 应在可能破坏它的操作之后验证，而不是只通过一个独立测试检查一次。可以提取共享断言 Helper，但不要求所有测试检查全部 Invariant。

初始化、Model Repair 和自动状态修复应遵循各 Widget 的既有契约，不为测试覆盖而改变实际通知行为。

### Coverage Model

包含 Integration Test 的 Crate，应在 `tests/COVERAGE.md` 中维护行为覆盖模型，集中描述测试所保护的重要业务契约及测试职责，而不是记录具体测试用例或测试开发历史。

根据实际模块复杂度，按需描述 States、Stimuli、Guards、Transitions、Invariants 和 Couplings。简单模块只记录有实际意义的维度和约束，不为了填满所有栏目制造内容，也不要求维护完整的 Transition Table。

Coverage Model 应保持稳定、简洁，重点描述当前测试保护了什么，而不是测试如何实现。

#### Coverage Map

Coverage Map 用于说明测试文件或源码测试模块的主要职责，分为 Integration Test 和 Unit Test 两部分。

- 使用 Markdown 表格组织，以测试文件或源码测试模块为单位记录职责。
- 同一测试文件或模块只出现一次，相关职责应合并描述。
- 每项使用简短、概括性的说明，不逐条罗列测试场景、输入组合、断言或回归案例。
- 重要 Stimulus 和业务契约应有明确的主要覆盖位置，但不要求每个测试函数都映射到文档。
- Unit Test 与 Integration Test 应分别归类，不混合描述。

#### 内容与维护

- `COVERAGE.md` 只描述当前有效的行为覆盖模型，不记录测试新增、修改、Bug 修复或其他开发历史。
- 新增测试如果仅加强已有行为的验证，不要求修改 Coverage Model。
- 重要业务契约、实际覆盖范围或测试文件主要职责发生变化时，应同步更新对应内容。
- 不重复记录相同测试文件的职责，不因新增回归测试而持续追加具体场景说明。
- 属于测试契约的重要条件和约束应归入对应行为维度；其他验证方式或任务的说明不纳入 Coverage Model。
- 不为了统一文档形式而强制所有 Crate 使用完全相同的章节结构。

Unit Test 不单独创建 Coverage Markdown。

## 测试组织与基础设施

私有实现优先通过所在 Module 的 Unit Test 验证，公共行为通过 Integration Test 从外部视角验证。

不得仅为了测试方便扩大生产代码的 Visibility。

跨 Crate 共享的测试与 Benchmark 基础设施应放在 `test_utils`，但不为简单且局部的 Helper 提前建立通用抽象。

测试代码允许使用断言、`panic`、`unwrap` 和 `expect`，具体由 Workspace lint 配置管理。

### 测试并行执行

测试用例必须支持并行执行，不得通过`--test-threads=1`、`RUST_TEST_THREADS=1`或其他方式强制串行运行测试。

测试之间不得依赖执行顺序，也不得通过共享可变状态产生相互干扰。

对于涉及全局状态、环境变量、文件系统或其他共享资源的测试，应通过状态隔离、独立资源或独立子进程解决冲突，不得以串行执行作为解决方案。

### 错误路径验证

错误回归测试应根据实际契约检查：

- 领域 API 按自身契约检查 `kind`、真实 `source` 原因链、共享 `origin`、`error_id` 和必要的源头 ERROR；不要求普通领域 API 返回 `BevyError` 或具有 Severity。
- 在 System、Observer、Command 等 Bevy 执行边界检查 `Severity::Error`、原始诊断保留及宿主 Handler 调用；外来 Bevy 错误保留其原生 Severity/Panic 策略。
- 源头一次 ERROR、纯传播无重复 ERROR、实际处理一次 DEBUG，以及最终未处理的宿主报告，通过相同 `error_id` 关联；源头和处理事件不输出完整栈，兜底检查原始 origin/源头标识，不比较完整回溯字符串或固定栈帧序号。
- 失败后需要保持的业务 State 与 Invariant。
- System、Observer 和 Command 的实际错误传播行为。

涉及 Bevy Error Handler 的路径，应通过捕获错误的宿主 Handler 验证，不以 `catch_unwind` 或 `should_panic` 代替正确的错误传播测试。

共享 `LogCapture` 默认只捕获 `bevy_widgetry` target；宿主诊断使用显式 target 选择。`ErrorCapture::take_records()` 保留真实 `BevyError` 与 `ErrorContext`，受管错误按类型识别而非匹配文案。

后台线程或 future 必须在实际执行线程或每次 poll 时显式传递测试 dispatcher；主线程的 thread-local Subscriber 不会自动传播。未传 dispatcher 时未捕获事件不能证明生产者没有发出日志。需要安装真实全局 Subscriber 的测试应在独立子进程中执行，避免不同测试反复安装或互相污染。

正常 Guard、合法 No-op 和明确允许的缺失状态，不应为了形成错误测试而人为改成失败。

## Property-based Testing

`proptest` 用于补充具有大量输入组合、数值边界、复杂 State Space 或多步操作序列的测试，不要求所有模块使用。

适用场景包括：

- 数值解析、范围限制与 Round-trip。
- 复杂 Model 操作及数据结构 Invariant。
- 多步 State Transition。
- 多种合法操作组合后的持续一致性。

对于操作序列，可以生成合法的随机操作，逐步应用，并在每一步检查相关 Invariant。

Property Test 不替代已知行为的确定性测试，也不要求为了使用 `proptest` 而提前增加依赖。

### 失败诊断与回归

Property Test 失败时，必须区分：

- **原始失败**：初始随机输入或操作序列及其触发的问题，包括重放所需的原始 Seed 等信息。
- **Shrinking 反例**：缩减后仍然触发失败的输入或操作序列。

Shrinking 只保证缩减后的输入仍能触发失败，不保证缩减前后属于同一根因。尤其在多步操作序列中，缩减可能暴露另一种缺陷。

因此，诊断和修复 Property Test 失败时：

1. 保留能够重放原始失败的 Seed 及必要诊断信息，并记录 Shrinking 后的反例。
2. 分别分析原始失败与缩减反例，不直接假定二者根因相同。
3. 修复后重新执行原始失败案例，不能仅凭最小反例通过就认定原始缺陷已经消失。
4. 确认 Bug 后补充可读的确定性 Regression Test，保护对应的具体行为。
5. 妥善保留仍有回归价值的失败 Seed，确保相关 Property Test 可以持续验证。

不要求所有 Property Test 为保证同一根因而拆分成独立测试。是否拆分应由实际测试语义和诊断需要决定。

使用 `proptest` 的失败持久化机制时，应正确利用其保存的 Seed 和重放能力，不人为制造失败持久化文件。没有已提交的失败 Seed 文件本身不构成测试缺陷。

## 其他验证

性能 Benchmark 负责测量和比较实际性能，不作为第三类行为测试。可确定性表达的工作量 Invariant 仍通过 Unit Test 或 Integration Test 保护，不在普通行为测试中直接断言受机器噪声影响的耗时。

具体性能要求遵循 `rules/benchmark.md`。

BRP GUI 验证属于任务级运行时验收，不取代长期行为测试，也不要求为每个历史 GUI 场景建立重复验证。

具体运行时验证遵循 `rules/gui-debugging.md`。
