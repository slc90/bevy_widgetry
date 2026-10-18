# Text Field

## 功能概述

提供可编辑的 `WidgetryTextField` 与只读的 `WidgetryReadOnlyTextField`，用于文本输入、查看、选择和复制。

## 当前设计思路

文本内容、Caret 与 Selection 使用 Bevy 官方 `EditableText`，Widgetry 负责控件 Scene、交互限制和样式投影。Read-only 和 Disabled 是不同的限制：前者保留选择 / 复制，后者阻止用户编辑相关输入。

## 主要功能

- 单行与可配置多行的文字编辑。
- ReadOnly 模式可导航、选择及复制，但过滤会修改文本的操作。
- Disabled 过滤用户编辑请求及未处理的 Edit / Paste Queue。
- Pointer / Keyboard Focus、Caret、Selection 与 Theme 相关颜色同步。
- 构造 Props 的 `value` 初始化正文，`max_length: Option<usize>` 指定固定的 Unicode 扩展字素簇上限；默认不限制，零表示空文本。
- `WidgetryTextField` / `WidgetryReadOnlyTextField` 提供以根实体为目标的 `text`、`set_text_in_world` 和排队 `set_text`；超限内容保留可容纳的完整字素簇前缀。
- `Node` 及官方文本布局、字体配置仍可使用 Bevy 组件设置。

## 行为与状态契约

ReadOnly 和 Disabled 不禁止通过 Widgetry 内容 API 程序化赋值。Disabled 对用户输入的限制优先于 ReadOnly，禁用期间旧 Edit / Paste 请求不能在重新启用后补执行。Theme、Hover 和 Focus 变化只应影响外观，不重置文字、Caret 或 Selection。

`EditableText` 的正文写入由 TextField 内部托管，外部直接写编辑器、替换组件或排入编辑命令不属于受支持的程序化内容入口。长度上限只在构造时确定，不支持运行时修改。构造时仍可初始化官方编辑组件，但初值会在首次渲染前规范化，`max_characters` 始终关闭，避免与字素簇约束并存。

用户输入在官方编辑提交前按队列顺序约束。先处理选择、删除，再按实际试替换全文确定最长合法插入前缀；键入、粘贴和 IME 正式提交共用该语义。预编辑仍交给官方编辑器，既不提前裁剪，也不计入正式正文。普通非 Widgetry `EditableText` 不受影响。

同步设置返回 `Ok(true)` / `Ok(false)` 表示正文是否改变，目标类型不符或编辑组件缺失返回 Error severity 的错误；Commands 在执行时验证。Setter 丢弃旧编辑及待处理粘贴，结束旧组合态，优先保留新正文中仍合法的光标与选区，否则折叠至末尾。官方 `TextEditChange` 保留其正文或光标变化语义，不另建 Widgetry 文本 Authority 或事件转接。

```rust
bsn! {
    @WidgetryTextField { @value: {"姓名".to_owned()}, @max_length: {Some(20)} }
    Node { width: px(240) }
}
```

## 边界情况与错误处理

文本输入依赖宿主安装相应 Bevy Keyboard、Clipboard、IME 环境。Invalid / Pending 编辑请求遵循官方 Text Input Pipeline 的处理时机，不应由 Widgetry 另行保存一份文本 Authority。

## 已知限制

- 不提供自有 Text Model；公开内容 API 委托唯一的官方编辑状态。
- 有长度上限时不能组合 `EditableTextFilter`：Bevy 未公开该组件的过滤函数，组合输入会明确报错并丢弃待处理请求；无上限时沿用官方过滤器管线。
- Clipboard 与 Native IME 的 OS 端行为不能全部通过 Headless 测试覆盖。
- ReadOnly 仍允许用户 Selection 和 Copy，不应等同 Disabled。

