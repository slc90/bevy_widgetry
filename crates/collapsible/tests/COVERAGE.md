# collapsible 测试覆盖模型

本模型记录当前自动化测试保护的主要行为契约与测试职责，不逐项列举测试用例。

## States
- Expanded／Collapsed、Header 局部 Disabled 与 Root／祖先 Disabled、内容可见性及 Focus。

## Stimuli
- BSN Scene、公开 World／Commands Set、真实 Pointer／Keyboard 输入、Theme 和样式更新。

## Guards
- 失效 Root、错误结构或缺失必要部件拒绝更新；Disabled／Hidden 阻止相应用户输入，但不禁止程序化折叠。
- Header Content 内部业务输入不应冒充折叠动作；合法同值请求不产生事件。

## Transitions
- 初始展开／折叠 → Toggle → 内容显示或隐藏；折叠时按作用域回收深层焦点，重新展开不自动恢复旧焦点。

## Invariants
- Expansion Authority 先提交再通知，初始化和合法同值静默。
- 折叠仅改变受管内容的可见投影，不销毁业务实体，也不丢失 Grid、编辑草稿、滚动位置或显式隐藏子项。
- Header 局部禁用与 Root 禁用相互独立；箭头／Card 的 Theme 投影不破坏调用方内容和业务状态。
- 非法结构和颜色产生可追溯的领域错误；持续故障去重，修复后实际恢复。

## Couplings
- Header Disabled × 内嵌编辑器；展开状态 × Focus 回收；Theme × 箭头及内容样式。

## Coverage Map

### Integration Test

| 测试文件 | 主要测试职责 |
|---|---|
| `diagnostics.rs` | 结构与样式故障聚合、原因链、真实错误传播及恢复。 |
| `disabled.rs` | Header 局部禁用 API、继承禁用与输入隔离。 |
| `input.rs` | 真实 Button／Keyboard 折叠、Focus 与 Disabled 守卫。 |
| `state.rs` | 公开展开状态、同步／排队提交和内容投影。 |
| `style.rs` | 默认布局、箭头方向、Theme／Override 与颜色错误。 |

### Unit Test

| 源码测试模块 | 主要测试职责 |
|---|---|
| `src/state.rs` | 局部状态与结构校验：缺失 Expanded、HeaderDisabled 或 Parts 的分类和实体身份；Disabled 失败时的原始诊断与既定提交顺序。 |
