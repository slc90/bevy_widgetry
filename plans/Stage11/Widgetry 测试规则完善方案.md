# Widgetry 测试规则完善方案

## 目标

完善 Widgetry 当前测试规范，明确：

- 单元测试怎样判断覆盖充分；
- 集成测试怎样系统性覆盖复杂 Widget 的业务行为；
- BRP GUI 验证在整个测试体系中的职责；
- 如何避免只覆盖 happy path、状态组合爆炸以及只检查最终稳定状态的问题。

本次只修改规则文档，不修改源码、测试代码、Cargo 依赖或 architecture 文档。

---

## 一、修改 `rules/testing.md`

### 1. 明确三种验证职责

长期自动化测试只有两类：

```text
Unit test
→ 局部 contract coverage

Integration test
→ 模块 observable behavior coverage
```

BRP GUI 验证不属于长期测试覆盖体系，不用于判断模块自动化测试是否全面。

---

## 2. Unit test 覆盖规则

Unit test 面向具有独立局部 contract 的：

- function；
- type；
- algorithm；
- 独立的小型状态对象或 model。

不要求每个 private function 都有 unit test，也不以 private implementation coverage 判断测试是否全面。

对于一个具有独立 contract 的单元，应根据实际语义考虑：

- 正常输入；
- 边界输入；
- guard / branch 的不同语义分支；
- no-op；
- invalid input；
- invariant；
- 有状态对象的有意义操作序列。

例如 `WidgetryListModel` 应验证 stable identity、revision、disabled metadata、move/remove/clear 等自身 contract；ListView 在 model mutation 后如何修复 selection 则属于 integration test。

---

## 3. Property-based testing

保留并加强现有 `proptest` 规则。

普通 `#[test]` / `rstest` 用于已知且明确的重要行为：

```text
明确 contract
→ deterministic test
```

`proptest` 用于：

- 大量输入组合；
- 数值边界；
- 可明确表达的 invariant；
- state space 较大；
- 多步操作序列后仍应保持的性质。

典型模式：

```text
生成任意合法 operation sequence
        ↓
逐步应用操作
        ↓
每一步执行 assert_xxx_invariants()
```

Property test 不替代明确的 deterministic regression test。

本次不新增 Cargo dependency；实际任务首次使用 `proptest` 时，再由该任务添加所需 dev-dependency。

---

## 4. Integration test 使用可观察状态模型设计覆盖

复杂模块的 integration test 应以**可观察业务状态机**为基础设计测试，而不是按 public API 或源码函数逐项补测试。

状态模型只描述可观察业务状态。

默认不包含：

- cache；
- dirty flag；
  -内部 revision cache；
- 其他纯 implementation state。

除非该内部状态本身承担必须保护的 scheduling / incremental-update contract。

---

## 5. 独立状态维度

复杂模块应把互相独立的业务状态拆成不同维度，例如：

```text
selection: none / selected
active: none / active
enabled: enabled / disabled
focus: focused / unfocused
model: empty / non-empty
```

不得机械展开成完整笛卡尔积。

测试规则为：

```text
每个状态维度
→ 覆盖自身重要 transition

不同状态维度之间
→ 只覆盖存在明确业务耦合的组合
```

例如：

```text
selection × model
selection × enabled
active × model
```

如果两个维度之间没有定义业务影响，不因为它们能够组合而强制增加测试。

---

## 6. 从 stimulus 推导测试

设计 integration coverage 时，应先识别所有能够改变可观察业务状态的 stimulus。

至少检查：

- 用户输入；
  - pointer；
  - keyboard；
  - focus；
  - scroll；
- public API；
- 外部 model / resource / component 变化；
- theme 等环境状态变化；
- lifecycle；
  - spawn；
  - despawn；
  - first update；
  - rebuild；
- Widgetry 与 Bevy / 其他 Widget 的真实组合输入。

每一种重要 stimulus 都应在测试覆盖模型中有明确归属。

---

## 7. Transition coverage

测试不能只覆盖最终状态。

对于每个有意义的 transition，应根据其实际 contract 考虑：

- normal path；
- boundary path；
- no-op / repeated operation；
- rejected / invalid input；
- stale external state。

不要求不存在相应语义的 transition 人工制造这些情况。

例如一个 `set_selected(id)` 如果存在相应 contract，应分别考虑：

```text
valid id
same id
missing / stale id
disabled interaction context
```

---

## 8. Guard coverage

如果某个 transition 存在 guard，应覆盖 guard 的主要语义分支。

不能只覆盖：

```text
guard == true
```

然后认为该 transition 已完整测试。

---

## 9. Invariant coverage

模块的重要 invariant 应显式识别。

例如：

```text
selected id 必须有效或为 None
active id 必须有效或为 None
programmatic selection 不产生用户通知
physical row 只是 logical state 的 projection
stable identity 不因 index move 改变
```

Invariant 不应只在独立的“invariant test”中验证一次。

所有可能破坏某 invariant 的 transition，都应在执行后验证该 invariant。

允许提取共享 helper：

```rust
assert_selection_invariants(...)
assert_active_invariants(...)
assert_projection_invariants(...)
```

不要求所有测试无条件调用一个巨大的 `assert_everything()`。

---

## 10. 测试模块级 Coverage Model 注释

复杂测试模块应在测试文件开头使用 `//!` module-level documentation 记录其覆盖模型。

典型内容：

```rust
//! State dimensions:
//! - ...
//!
//! Events / stimuli:
//! - ...
//!
//! Guards:
//! - ...
//!
//! Invariants:
//! - ...
//!
//! Couplings:
//! - ...
```

这份说明用于描述测试设计边界，不要求维护完整 transition table。

具体 transition 由实际 Rust test case 表达。

简单模块不要求为了形式完整强行写空洞的状态模型。

---

## 11. 多测试文件的 Coverage Map

当一个复杂 Widget 的 integration test 拆成多个文件时，应有一个主要测试模块维护轻量级 Coverage Map。

例如：

```text
logical state / selection
model mutation
interaction
virtualization
accessibility
rendering / layout
```

Coverage Map 只说明各测试文件负责的行为领域和重要跨领域 invariant，不重复完整状态模型。

每一种重要 stimulus 应有明确的主要测试模块 owner。

跨模块行为可以联合验证，但不得形成多个文件各测一点、最终没人负责完整 contract 的情况。

---

## 二、修改 `rules/gui-debugging.md`

## 1. 明确 BRP 的定位

BRP GUI 验证是：

> 当前任务完成后的真实用户场景运行时验收。

它不是长期 exhaustive regression suite。

BRP 场景根据：

- 本次新增行为；
- 本次修改行为；
- 本次变更直接影响的 GUI 行为；

进行选择。

不要求每次重新验证 Widget 的全部历史功能。

长期回归保护仍由 unit test / integration test 承担。

---

## 2. 必须走真实用户路径

BRP 验证某种 GUI 行为时，应真实执行对应用户路径。

例如：

```text
keyboard behavior
→ 真实取得 focus
→ 发送 keyboard input

pointer behavior
→ 真实 pointer movement / click

scroll behavior
→ 真实 wheel / drag
```

不得直接修改 ECS state 后把结果视为等价的用户行为验证。

---

## 3. 验证用户可观察结果

BRP 优先验证：

- 实际视觉结果；
- 实际交互效果；
- focus；
- scrolling；
- popup；
- selection；
- layout。

需要精确判断内部语义时，可以结合 ECS / Component / Resource state。

不要求重复 integration test 中所有内部 invariant。

---

## 4. 增加 Temporal Behavior 验证规则

动态 GUI 行为不得默认：

```text
执行输入
→ 等待完全稳定
→ 只检查最终状态
```

对于容易产生瞬时错误的变化，应关注操作后的中间过程和收敛过程。

典型问题包括：

- 一帧或短暂空白；
- 短暂消失；
- flicker；
- 旧内容残留；
- 数百毫秒后才刷新；
- popup 先出现在错误位置后再跳正；
- text / icon 晚一帧出现；
- background 已变化但 icon / foreground 尚未同步；
- dynamic subtree 创建后直到下一帧才真正可见。

重点关注对象包括：

- dynamic subtree；
- text；
- icon；
- popup；
- focus transition；
- theme change；
- visibility；
- layout；
- virtualized row；
- asset / text / icon materialization。

需要时验证流程可以为：

```text
baseline
↓
execute input
↓
尽快观察操作后状态
↓
必要时观察 intermediate state
↓
确认 final stable state
```

不要求所有 GUI 测试机械增加多次截图；只在当前行为存在 temporal risk 时使用。

---

## 5. BRP 发现问题后的回归闭环

如果 BRP 发现可稳定复现的 GUI / temporal regression，并且该问题能够合理通过 headless unit / integration test 表达，应增加永久 regression test。

关系为：

```text
BRP
→ 发现真实运行问题
→ 提炼可自动化 contract
→ 沉淀为长期 regression test
```

BRP 本身不承担以后持续重复执行同一场景的责任。

---

## 三、修改 `rules/development.md`

只做少量流程衔接，不复制 `testing.md` 的具体规则。

### Test-Driven Development 部分

在复杂状态模块进入具体 test case 之前，先识别当前行为涉及的：

```text
state dimensions
stimuli
guards
invariants
cross-state couplings
```

再据此选择当前 Red 阶段需要的 unit / integration test。

不要求一次设计完整功能的全部状态空间，仍然保持当前小步：

```text
Type
→ Red
→ Green
→ Refactor
```

循环。

### GUI 验证部分

明确：

> BRP GUI 验证按照本次任务的真实用户场景选择，不要求重新执行模块完整长期测试覆盖。

对于动态 UI 行为，应根据 `rules/gui-debugging.md` 额外关注 temporal behavior。

---

## 四、不修改的内容

本次不修改：

- `AGENTS.md`
- `docs/architecture.md`
- Workspace / crate `Cargo.toml`
- `crates/test_utils`
- 任何生产源码
- 任何现有测试代码
- Gallery 实现

原因：

- `AGENTS.md` 当前已经能够正确引导测试任务读取 `rules/testing.md`，GUI 任务读取 `rules/gui-debugging.md`；
- 测试 architecture 与 dependency graph 没有发生实际变化；
- `proptest` 只是规则允许和推荐的按需工具，本次没有实际测试使用它，因此不提前增加依赖。

---

## 最终测试模型

```text
                 Long-term regression
                 ────────────────────

Unit Test
│
├─ local contract
├─ branches / guards
├─ boundaries
├─ no-op / invalid
├─ invariants
└─ property / operation sequences when useful


Integration Test
│
├─ observable state dimensions
├─ stimuli
├─ transitions
├─ guards
├─ invariants
├─ explicit cross-state couplings
└─ coverage map for complex modules


                 Current-task runtime acceptance
                 ───────────────────────────────

BRP GUI Validation
│
├─ changed / affected real user scenarios
├─ real pointer / keyboard / focus / scroll path
├─ visual + structured state when useful
└─ temporal behavior / transient regressions
```

整体原则：

> Unit test 证明局部 contract；Integration test 证明模块长期业务 contract；BRP 证明本次修改在真实 GUI 使用过程中确实工作。

> 测试全面性来自完整的业务状态、stimulus、transition、guard、invariant 和必要 coupling，而不是测试数量、源码函数数量或状态笛卡尔积。
