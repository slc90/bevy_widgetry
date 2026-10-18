# Text Field

## 功能概述

提供可编辑的 `WidgetryTextField` 与只读的 `WidgetryReadOnlyTextField`，用于文本输入、查看、选择和复制。

## 当前设计思路

文本内容、Caret 与 Selection 使用内部编辑实体上的 Bevy 官方 `EditableText`，Widgetry 根负责外部身份及纵向布局，Frame 负责边框、背景和横向组合。根不再携带 `EditableText` / `TextInput`。Read-only 和 Disabled 是不同的限制：前者保留选择 / 复制，后者阻止用户编辑相关输入。

## 主要功能

- 单行与可配置多行的文字编辑。
- ReadOnly 模式可导航、选择及复制，但过滤会修改文本的操作。
- Disabled 过滤用户编辑请求及未处理的 Edit / Paste Queue。
- Pointer / Keyboard Focus、Caret、Selection 与 Theme 相关颜色同步。
- 构造 Props 的 `value` 初始化正文，`max_length: Option<usize>` 指定固定的 Unicode 扩展字素簇上限；默认不限制，零表示空文本。
- `WidgetryTextField` / `WidgetryReadOnlyTextField` 提供以根实体为目标的 `text`、`set_text_in_world` 和排队 `set_text`；超限内容保留可容纳的完整字素簇前缀。
- `Node` 及官方文本布局、字体配置仍可使用 Bevy 组件设置。
- 可选 `WidgetryTextFieldPrefix` / `WidgetryTextFieldSuffix` 容器通过 `@children` 接收任意 BSN Children；每类最多一个，声明顺序不影响 Prefix → Editor → Suffix 的位置。重复、同一容器双重标记及不属于槽的直接内容会交给宿主 Error Handler，结构变化时重新验证。
- 两侧保留自身宽度，编辑器使用剩余宽度；过宽内容不自动缩放、隐藏或建立降级布局。槽内可有多个装饰节点、按钮或其他控件，其文本不进入正文和长度计算。
- 构造属性 `show_count` 默认关闭。开启后在 Frame 边框外下方右对齐显示正式文本的 Unicode 扩展字素簇数；无上限显示 `11`，有上限显示 `11 / 100`，零上限显示 `0 / 0`。普通、只读、单行、多行均支持，Prefix/Suffix 与 IME 预编辑不计入。

## 行为与状态契约

ReadOnly 和 Disabled 不禁止通过 Widgetry 内容 API 程序化赋值。Disabled 对用户输入的限制优先于 ReadOnly，禁用期间旧 Edit / Paste 请求不能在重新启用后补执行。Theme、Hover 和 Focus 变化只应影响外观，不重置文字、Caret 或 Selection。

`EditableText` 的正文写入由 TextField 内部托管，外部直接写编辑器、替换组件或排入编辑命令不属于受支持的程序化内容入口。长度上限只在构造时确定，不支持运行时修改。通过 Props 的 `value`、`allow_newlines`、`visible_lines`（默认一行）和 `visible_width` 设置初始内容与原有编辑配置；不再向根添加 `EditableText`。`max_characters` 始终关闭，避免与字素簇约束并存。

根上的 `Node` 控制整体尺寸，根的 `TextFont`、`TextLayout`、`LineHeight`、`FontHinting` 和 `EditableTextFilter` 配置投影到内部编辑实体。主题将 Background / Border 写入 Frame，Text / Caret / Selection 颜色写入 Editor，样式变化不重置编辑状态。

计数节点使用 Widgetry 文本主题，不接受拾取或焦点，不改变编辑区可用宽度；其高度计入 Root。数字直接派生自唯一的官方正式文本，创建和同步 Setter 后立即正确，用户编辑在官方提交后同帧更新计数的 `Text`，显示排版沿用 Bevy 的 UI 文本管线。`show_count` 仅在构造时确定。

`editor` 提供以根为目标的官方编辑状态只读访问，`focus_in_world` / 排队 `focus` 将根映射为真实 InputFocus 目标。根的 `AcquireFocus`、Tab 顺序及 Window InitialFocus 继续使用根作为外部身份；Tab 在真实编辑器、槽中焦点控件与其他控件之间导航。槽内按钮保留自身的点击与焦点，装饰内容不会自动聚焦编辑器，调用方可自行设置拾取穿透或显式调用根聚焦 API。根或祖先的 Disabled 通过继承约束全部内部交互；ReadOnly 仍可选择、导航和复制。

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

