# text_field 测试覆盖模型

本模型记录当前自动化测试保护的主要行为契约与测试职责，不逐项列举测试用例。

## States
- Editable／ReadOnly、Enabled／Disabled、Hover／Focus、Text／Caret／Selection 和编辑／粘贴队列。
- IME Composition、长度限制、计数／Hint／Auto Size、Theme 和嵌套 Editor。

## Stimuli
- BSN Scene／公开内容 API、真实 Pointer／Keyboard、EditCommand／Paste、IME、Focus／Disabled 切换和 Theme／Style 更新。

## Guards
- ReadOnly 阻止 Mutation 但允许导航／选择／复制；Disabled 阻止相应用户编辑和 Paste，不阻碍合法程序化内容更新。
- 无效 Editor／输入映射、缺资源或非法限制应拒绝，不破坏已有正文与 Selection。

## Transitions
- Focus／Hover／Disabled 外观变化、光标定位／选词／拖选、编辑提交／取消、IME 开始／结束、Auto Size 与提示／计数更新。

## Invariants
- Text／Selection 以官方 EditableText 为状态权威；样式、Root 组成和 Theme 不重置正文或编辑状态。
- 禁用期间旧 Edit／Paste 不在恢复后重放；ReadOnly 的用户 Mutation 在官方编辑消费前被阻止。
- 鼠标／自定义 Pointer 的 Caret 与 Selection 路由一致；IME 预编辑、正式提交和取消不破坏光标及正文身份。
- Unicode 字素上限在插入／替换／粘贴／IME 后持续成立；计数、Hint 和自动高度反映当前正式文本。
- 公开根与多层独立 Editor 的 Focus／InputTarget 映射正确；异常编辑器或下层写入失败保留 source／origin 和操作上下文。

## Couplings
- Pointer Focus × Keyboard；ReadOnly × Selection；Disabled × Pending Input；IME × Unicode Limit；Theme × Caret／Hint／Count。

## Coverage Map

### Integration Test

| 测试文件 | 主要测试职责 |
|---|---|
| `auto_size.rs` | 单／多行自适应高度、真实排版与滚动。 |
| `composition.rs` | 多层 Editor、Root／Frame 身份、输入映射与焦点。 |
| `count.rs` | 可选计数、Unicode 字素和编辑／IME 更新。 |
| `diagnostics.rs` | 结构、编辑器、内容与命令错误的来源和传播。 |
| `disabled.rs` | 编辑队列过滤、继承 Disabled 和恢复。 |
| `headless.rs` | 独立 Editor／Headless 输入、程序写入与多层组合。 |
| `hint.rs` | 提示文本、换行、颜色、Focus 和正式内容投影。 |
| `length.rs` | Unicode 字素限制、同步／排队内容与编辑边界。 |
| `pointer.rs` | Mouse／Custom Picking 下光标、选词和拖选。 |
| `read_only.rs` | 只读导航、Selection、复制和写入守卫。 |
| `widgetry_text_field.rs` | 公开 Scene、默认外观、真实布局、字体与 Theme。 |

### Unit Test

| 源码测试模块 | 主要测试职责 |
|---|---|
| `src/content.rs` | 编辑器身份、文本长度限制及内容提交错误。 |
