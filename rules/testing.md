# 测试规则

## 测试义务

- 新增或改变可观察行为时，必须有对应测试。
- 修复 bug 时，应增加能够复现该问题并防止回归的 regression test。
- 纯结构重构如果行为不变，不要求为了重构本身强行增加新测试。

讨论设计时重点确定测试意图、关键行为边界和重要 invariant；Codex 可以自主补齐机械性的覆盖，不需要人工穷举完整 test checklist。

### Gallery 例外

`gallery/` 仅用于 Widget 展示，不要求编写 unit test、integration test 或其他自动化测试。

对 `gallery/` 的修改应保证能够正常编译。

涉及展示效果或交互行为时，应按照 `rules/gui-debugging.md` 通过 Widget Gallery 进行运行时验证；BRP 是 Codex 执行此类验证的默认方式。

本例外优先于本文件中的一般测试义务。

## 测试范围

测试应验证 Widgetry 自身的行为、语义和约束。

- 不重复测试 Bevy 或第三方库已经保证的内部行为。
- 当第三方行为与 Widgetry 的真实组合关系本身就是需要验证的边界时，可以通过 integration test 覆盖真实协作。

### 验证职责

长期自动化测试只有两类：

- Unit test 证明具有独立语义的局部 contract。
- Integration test 证明模块的可观察业务 contract。

BRP GUI 验证是当前任务完成后的真实用户场景运行时验收，不属于长期自动化测试覆盖体系，不用于判断模块自动化测试是否全面。具体要求见 rules/gui-debugging.md。

测试全面性来自重要的业务 state、stimulus、transition、guard、invariant 和必要 coupling，而不是测试数量、源码 function 数量或 state 的完整笛卡尔积。

## Unit test 覆盖

Unit test 面向具有独立局部 contract 的 function、type、algorithm，以及独立的小型 state 对象或 model。

不要求每个 private function 都有 unit test，也不以 private implementation coverage 判断测试是否全面。

对于一个具有独立 contract 的单元，应根据实际语义考虑：

- 正常输入。
- 边界输入。
- guard / branch 的不同语义分支。
- no-op。
- invalid input。
- invariant。
- 有 state 对象的有意义操作序列。

例如 WidgetryListModel 应验证 stable identity、revision、disabled metadata、move/remove/clear 等自身 contract；ListView 在 model mutation 后如何修复 selection 则属于 integration test。

## Integration test 覆盖

### 可观察业务状态模型

复杂模块的 integration test 应以可观察业务状态机为基础设计覆盖，而不是按 public API 或源码 function 逐项补测试。

状态模型只描述可观察业务 state，默认不包含 cache、dirty flag、内部 revision cache 或其他纯 implementation state；除非该内部 state 本身承担必须保护的 scheduling / incremental-update contract。

### 独立 state 维度与 coupling

复杂模块应把互相独立的业务 state 拆成不同维度，例如：

```text
selection: none / selected
active: none / active
enabled: enabled / disabled
focus: focused / unfocused
model: empty / non-empty
```

每个维度应覆盖自身重要 transition；不同维度之间只覆盖存在明确业务 coupling 的组合，不得机械展开完整笛卡尔积。

例如 selection × model、selection × enabled、active × model 可以存在需要保护的业务 coupling。如果两个维度之间没有定义业务影响，不因为它们能够组合而强制增加测试。

### 从 stimulus 推导覆盖

设计 integration coverage 时，应先识别所有能够改变可观察业务 state 的 stimulus，至少检查：

- 用户输入：pointer、keyboard、focus、scroll。
- public API。
- 外部 model / resource / component 变化。
- theme 等环境 state 变化。
- lifecycle：spawn、despawn、first update、rebuild。
- Widgetry 与 Bevy / 其他 Widget 的真实组合输入。

每一种重要 stimulus 都应在测试覆盖模型中有明确归属。

### Transition 与 guard

测试不能只覆盖最终 state。对于每个有意义的 transition，应根据其实际 contract 考虑：

- normal path。
- boundary path。
- no-op / repeated operation。
- rejected / invalid input。
- stale external state。

不要求对不存在相应语义的 transition 人工制造这些情况。例如 set_selected(id) 如果存在相应 contract，应分别考虑 valid id、same id、missing / stale id、disabled interaction context。

如果某个 transition 存在 guard，应覆盖 guard 的主要语义分支，不能只覆盖 guard == true 就认为该 transition 已完整测试。

### Invariant

模块的重要 invariant 应显式识别，例如：

- selected id 必须有效或为 None。
- active id 必须有效或为 None。
- programmatic selection 不产生用户通知。
- physical row 只是 logical state 的 projection。
- stable identity 不因 index move 改变。

Invariant 不应只在独立的 invariant test 中验证一次。所有可能破坏某 invariant 的 transition，都应在执行后验证该 invariant。

允许提取 assert_selection_invariants(...)、assert_active_invariants(...)、assert_projection_invariants(...) 等共享 helper；不要求所有测试无条件调用一个巨大的 assert_everything()。

### 测试模块的 Coverage Model

复杂测试模块应在测试文件开头使用 //! module-level documentation 记录覆盖模型，说明相关 state 维度、stimuli、guards、invariants 与 couplings。例如：

```rust
//! State dimensions:
//! - selection 与 model 的可观察 state。
//!
//! Events / stimuli:
//! - 用户输入与外部 model mutation。
//!
//! Guards:
//! - 用户输入受 enabled state 限制。
//!
//! Invariants:
//! - selected id 必须有效或为 None。
//!
//! Couplings:
//! - model mutation 后修复 selection。
```

这份说明描述测试设计边界，不要求维护完整 transition table；具体 transition 由实际 Rust test case 表达。简单模块不要求为了形式完整强行写空洞的状态模型。

### 多测试文件的 Coverage Map

当一个复杂 Widget 的 integration test 拆成多个文件时，应有一个主要测试模块维护轻量级 Coverage Map，说明各测试文件负责的行为领域和重要跨领域 invariant。

行为领域可以包括 logical state / selection、model mutation、interaction、virtualization、accessibility、rendering / layout。Coverage Map 不重复完整状态模型。

每一种重要 stimulus 应有明确的主要测试模块 owner。跨模块行为可以联合验证，但不得形成多个文件各测一点、最终没人负责完整 contract 的情况。

## 测试放置

### Unit test

具有独立局部 contract 的内部逻辑与局部行为测试，放在对应源码 module 的：

```rust
#[cfg(test)]
mod tests {
    // ...
}
```

测试应尽量贴近被测实现。

### Integration test

以下测试放在 crate 的 `tests/`：

- 从 crate 外部视角验证公共行为。
- 跨多个 module 的真实协作。
- 需要真实 Bevy / ECS 组合环境验证的行为。

`tests/` 是项目中允许使用传统 `mod.rs` 组织测试 module 的区域。

### test_utils

多个 crate 或 integration test 共用的测试基础设施放入 test_utils。

不得为了少写几行 helper 而在多个 crate 复制同类测试基础设施。

## Visibility 与测试

不得为了测试方便把私有实现改成 pub。

- 私有逻辑优先通过同 module 的 unit test 测试。
- 公共行为通过 integration test 从外部视角测试。

## 断言与错误传播

测试代码允许使用 assert 系列、panic、unwrap 和 expect；非测试代码仍遵守 rules/code.md 的禁止规则。

错误回归测试应验证返回的错误内容与 Severity::Error、Widgetry 日志，以及失败后必须保持的业务 state。system、observer 和 command 使用捕获错误的宿主 handler 验证真实 ECS 传播路径，不以 catch_unwind 或 should_panic 作为错误传播的成功标准。

## Property-based testing

proptest 作为按需使用的增强测试工具，不要求所有测试使用。

当逻辑具有以下特征时，应优先考虑 proptest：

- 大量输入组合。
- 数值边界。
- 可明确表达的 invariant。
- state space 较大。
- 一串操作组合后仍应满足某些性质。

数字 TextField、SpinBox、范围 clamp、parse/format round trip、increment/decrement state 组合等属于典型适用场景。

已知且明确的重要 contract 应使用普通 #[test] / rstest 编写 deterministic test。

对于多步操作序列，可以生成任意合法 operation sequence，逐步应用操作，并在每一步执行对应的 assert_xxx_invariants()。

Property test 不替代明确的 deterministic regression test。

proptest 的 dev-dependency 在实际任务首次使用时按需添加，不因规则推荐而提前添加 dependency。
