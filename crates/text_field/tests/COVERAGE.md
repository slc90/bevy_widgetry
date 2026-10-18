# text_field Integration Test Coverage

本覆盖模型描述现有 Integration Test 重点保护的可观察行为及测试职责，不是逐项 Test Case 清单。

## States

- 普通 / ReadOnly、Enabled / Disabled、Hover / Focus。
- Text、Caret、Selection、Pending Edit / Paste 和 Theme。

## Stimuli

- BSN Scene、Widgetry 根实体内容 API、Keyboard / Pointer、Selection Drag。
- EditCommand / Paste、Theme、Focus 转移和 Disabled 恢复。

## Guards

- ReadOnly 阻止 Mutation，但允许 Navigation / Selection / Copy。
- Disabled 优先过滤全部用户编辑与 Paste，不阻止程序化内容修改。

## Transitions

- Normal / Hover / Focus / Disabled 外观转换。
- 用户选择文本、按字选取或拖动。
- Enabled → Disabled → Enabled 时不重放旧 Edit Queue。

## Invariants

- Text / Selection 是官方 EditableText State，样式不会重置内容。
- ReadOnly 的 Mutation 不到达真正编辑阶段，合法 Selection 仍改变。
- 禁用期间旧 Edit / Paste 丢弃；恢复后只接受新请求。
- 预编辑隐藏 Caret 时的程序化赋值先结束旧组合态，恢复合法光标和可见几何。
- Queue 只证明特定过滤阶段，另检查 Bevy 编辑消费后的 Text / Selection；Mouse / Custom 的定位、连续点击、Drag / Cancel 共享语义，Disabled 不消费 Selection。

## Couplings

- Pointer Focus × Keyboard；ReadOnly × Selection。
- Disabled × Pending Edits / Paste；Theme × Caret / Selection Colors。

## Coverage Map

- `widgetry_text_field.rs`：Scene、Layout、Font 策略与完整 Style / Theme。
- `disabled.rs`：编辑阻止、Queue 清理与恢复。
- `read_only.rs`：ReadOnly Mutation 边界、Navigation / Selection / Focus。
- `pointer.rs`：Mouse / Custom Picking 的 Caret、选词与 Drag Selection。
- `support/mod.rs`：跨用例共享 Fixture，不单独承担业务覆盖。
- `length.rs`：Unicode 字素簇上限、初值/根实体 Setter、队列顺序、边界合并、IME 正式/预编辑、剪贴板、官方通知与提交前长度保证。

## 覆盖边界

Native Clipboard、IME 与真实 OS Focus 仍存在 Headless 验证边界。

