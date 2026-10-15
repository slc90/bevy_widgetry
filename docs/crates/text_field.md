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
- 调用方可设置 `EditableText` 和 `Node` 的内容与布局。

## 行为与状态契约

ReadOnly 不禁止程序化修改 `EditableText`。Disabled 对用户输入的限制优先于 ReadOnly，禁用期间旧 Edit / Paste 请求不能在重新启用后补执行。Theme、Hover 和 Focus 变化只应影响外观，不重置文字、Caret 或 Selection。

## 边界情况与错误处理

文本输入依赖宿主安装相应 Bevy Keyboard、Clipboard、IME 环境。Invalid / Pending 编辑请求遵循官方 Text Input Pipeline 的处理时机，不应由 Widgetry 另行保存一份文本 Authority。

## 已知限制

- 不提供自有 Text Model 或替代 Bevy 编辑状态的 API。
- Clipboard 与 Native IME 的 OS 端行为不能全部通过 Headless 测试覆盖。
- ReadOnly 仍允许用户 Selection 和 Copy，不应等同 Disabled。

