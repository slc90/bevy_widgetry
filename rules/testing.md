# 测试规则

## 测试义务

- 新增或改变可观察行为时，必须有对应测试。
- 修复 bug 时，应增加能够复现该问题并防止回归的 regression test。
- 纯结构重构如果行为不变，不要求为了重构本身强行增加新测试。

讨论设计时重点确定测试意图、关键行为边界和重要不变量；Codex 可以自主补齐机械性的覆盖，不需要人工穷举完整 test checklist。

### Gallery 例外

`gallery/` 仅用于控件展示，不要求编写 unit test、integration test 或其他自动化测试。

对 `gallery/` 的修改只需保证能够正常编译，并通过人工运行确认展示效果和交互行为。

本例外优先于本文件中的一般测试义务。

## 测试范围

测试应验证 Widgetry 自身的行为、语义和约束。

- 不重复测试 Bevy 或第三方库已经保证的内部行为。
- 当第三方行为与 Widgetry 的真实组合关系本身就是需要验证的边界时，可以通过 integration test 覆盖真实协作。

## 测试放置

### Unit test

单个模块内部逻辑、私有实现和局部行为的测试，放在对应源码模块的：

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
- 跨多个模块的真实协作。
- 需要真实 Bevy / ECS 组合环境验证的行为。

`tests/` 是项目中允许使用传统 `mod.rs` 组织测试模块的区域。

### `test_utils`

多个 crate 或 integration test 共用的测试基础设施放入 `test_utils`。

不得为了少写几行 helper 而在多个 crate 复制同类测试基础设施。

## 可见性与测试

不得为了测试方便把私有实现改成 `pub`。

- 私有逻辑优先通过同模块 unit test 测试。
- 公共行为通过 integration test 从外部视角测试。

## `unwrap`

测试代码允许使用 `unwrap`；非测试代码仍遵守 `rules/code.md` 的禁止规则。

## Property-based testing

`proptest` 作为按需使用的增强测试工具，不要求所有测试使用。

当逻辑具有以下特征时，应优先考虑 `proptest`：

- 大量输入组合。
- 数值边界。
- 可明确表达的不变量。
- 状态空间较大。
- 一串操作组合后仍应满足某些性质。

数字输入框、SpinBox、范围 clamp、parse/format 往返、increment/decrement 状态组合等属于典型适用场景。

普通明确行为仍以常规 unit / integration test 为主。
