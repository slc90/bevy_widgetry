# Widgetry ReadOnly TextField 实施方案

## 目标

在现有 `bevy_widgetry_text_field` 中增加独立的只读文本控件：

```rust
WidgetryReadOnlyTextField
```

它基于 Bevy 0.19.1 官方 `EditableText`，与现有 `WidgetryTextField` 共享布局、样式、focus、selection 和文本配置能力，但禁止所有用户文本修改行为。

不使用普通 `Text` 代替，因为只读控件仍需要文本选择、光标导航和复制能力。

## Public API

继续使用同一个插件：

```rust
WidgetryTextFieldPlugin
```

公开：

```rust
WidgetryTextField
WidgetryReadOnlyTextField
WidgetryTextFieldPlugin
```

不新增：

```rust
WidgetryReadOnlyTextFieldPlugin
ReadOnly // 不作为公共 API
```

调用方式：

```rust
@WidgetryTextField
```

以及：

```rust
@WidgetryReadOnlyTextField
```

Readonly 是控件初始化时确定的身份，不提供运行时从普通 TextField 切换为 ReadOnly 的公开机制。

## 内部结构

增加两个 crate 内部 marker：

```rust
#[derive(Component)]
struct TextFieldBase;

#[derive(Component)]
struct ReadOnly;
```

结构：

```text
WidgetryTextField
├─ EditableText
├─ TextFieldBase
└─ 现有 layout / style components

WidgetryReadOnlyTextField
├─ EditableText
├─ TextFieldBase
├─ ReadOnly
└─ 与 WidgetryTextField 相同的 layout / style components
```

所有两个控件共有的 system 使用：

```rust
With<TextFieldBase>
```

而不是反复写：

```rust
Or<(
    With<WidgetryTextField>,
    With<WidgetryReadOnlyTextField>,
)>
```

## ReadOnly 行为

Readonly 只禁止**用户修改文本内容**。

继续允许：

```text
focus
cursor movement
selection
Ctrl/Cmd + A
Ctrl/Cmd + C
鼠标单击定位
拖动选择
双击选词
多击选择
Shift + Click
```

禁止：

```text
Cut
Paste
Insert
Backspace
BackspaceWord
Delete
DeleteWord
IME Preedit / composition
IME Commit
```

对应 Bevy 0.19.1 `TextEdit` 中需要过滤：

```rust
TextEdit::Cut
TextEdit::Paste
TextEdit::Insert(_)
TextEdit::Backspace
TextEdit::BackspaceWord
TextEdit::Delete
TextEdit::DeleteWord
TextEdit::ImeSetCompose { .. }
TextEdit::ImeCommit { .. }
```

其余 `TextEdit` 全部保留。

同时清除：

```rust
editable_text.pending_paste = None;
```

避免异步 clipboard paste 已经进入 pending 状态后修改 ReadOnly 内容。

`Cut` 在 ReadOnly 中完全无效，不退化成 Copy。

IME composition 和 commit 均禁止，不显示临时 preedit。

## System

增加 ReadOnly 输入过滤 system，在：

```rust
PostUpdate
```

运行，并保证：

```rust
before(EditableTextSystems)
```

查询：

```rust
Query<
    &mut EditableText,
    (
        With<TextFieldBase>,
        With<ReadOnly>,
    ),
>
```

职责仅为：

```text
过滤会修改文本内容的 TextEdit
清除 pending_paste
```

现有 disabled workaround 调整为使用公共 marker：

```rust
Query<
    &mut EditableText,
    (
        With<TextFieldBase>,
        With<InteractionDisabled>,
    ),
>
```

Disabled 继续清空整个：

```rust
pending_edits
pending_paste
```

## 状态语义

行为优先级：

```text
InteractionDisabled > ReadOnly > Normal
```

语义：

```text
Normal
→ 可编辑
→ 可选择
→ 可复制

ReadOnly
→ 不可编辑
→ 可 focus
→ 可移动光标
→ 可选择
→ 可复制

InteractionDisabled
→ 不可编辑
→ 不可选择
→ 不可复制
→ 不参与正常文本交互

ReadOnly + InteractionDisabled
→ 按 InteractionDisabled 处理
```

不需要为 `ReadOnly + InteractionDisabled` 增加额外特殊状态。

## Style

ReadOnly **不增加视觉状态**。

现有样式优先级保持：

```text
Disabled > Focused > Hovered > Normal
```

`WidgetryReadOnlyTextField` 与普通 `WidgetryTextField` 使用完全相同的：

```text
Node layout
padding
border
radius
background
foreground
cursor
selection color
hover style
focus style
disabled style
```

Readonly 获得 focus 后正常显示 Focused 样式。

只有 `InteractionDisabled` 使用 Disabled 配色。

## EditableText 配置

`WidgetryReadOnlyTextField` 与普通 TextField 一样允许调用方 patch 官方 `EditableText`：

```text
initial text
visible_lines
visible_width
allow_newlines
max_characters
EditableTextFilter
以及其他官方配置
```

Readonly 只限制用户输入。

程序代码仍可以：

```rust
editable.editor_mut().set_text(...)
```

更新内容。

## Plugin

`WidgetryTextFieldPlugin` 统一负责：

```text
WidgetryTextField
WidgetryReadOnlyTextField
共同 style systems
ReadOnly 输入过滤
InteractionDisabled workaround
ThemePlugin
WidgetryFocusPlugin
```

不增加第二个 plugin。

## Tests

新增 ReadOnly 行为测试，至少覆盖：

```text
Insert 被阻止
Backspace / Delete 被阻止
Cut 被阻止
Paste 被阻止
IME composition 被阻止
IME commit 被阻止

Copy 被保留
SelectAll 被保留
cursor navigation 被保留
鼠标 selection command 被保留

程序化 set_text 仍然有效
```

同时覆盖：

```text
ReadOnly + InteractionDisabled
→ Disabled 清空所有 command
```

现有 TextField style tests 应同时确认 ReadOnly：

```text
Normal
Hovered
Focused
Disabled
theme switch
selection colors
```

与普通 TextField 使用相同样式。

不重复测试 Bevy 自己的文本编辑实现，只验证 Widgetry 定义的过滤 contract。

## Gallery

修改：

```text
gallery/src/pages/text_field.rs
```

TextField 页面在现有示例基础上**增加 `WidgetryReadOnlyTextField` 示例**。

最终页面至少包含四类：

```text
普通单行 TextField
多行 TextField
ReadOnly TextField
Disabled TextField
```

ReadOnly 示例例如：

```rust
@WidgetryReadOnlyTextField
template_value(EditableText::new(
    "Read-only text: select and copy me"
))
Node { margin: UiRect::bottom(px(16)) }
```

ReadOnly 示例主要用于人工验证：

```text
可以获得 focus
可以鼠标定位光标
可以拖选文本
可以 Ctrl/Cmd + A
可以 Ctrl/Cmd + C

不能输入文字
不能 Backspace / Delete
不能 Cut
不能 Paste
不能进行 IME composition / commit

hover / focus / selection 外观与普通 WidgetryTextField 一致
```

现有普通单行、多行和 Disabled 示例继续保留，不用 ReadOnly 替换任何现有示例。

## Facade

`bevy_widgetry::text_field::*` 已直接 re-export 整个功能 crate，因此不需要改变 facade 结构。

public API test 增加：

```rust
WidgetryReadOnlyTextField
```

的 BSN 构造验证，并确认 entity 同样包含官方 `EditableText`。

## 不做

本次不加入：

```text
普通 Text / WidgetryText
公开 ReadOnly component
运行时 Editable ↔ ReadOnly 切换
Readonly 专属 theme token
Readonly 专属 plugin
placeholder
validation
password masking
新的文本 abstraction
```

## 验收

完成后应满足：

1. `@WidgetryReadOnlyTextField` 可以通过 BSN 构造。
2. ReadOnly 与普通 TextField layout / style 完全一致。
3. ReadOnly 可以 focus、移动光标、选择和复制文本。
4. 所有用户 mutation 都不能修改内容。
5. Cut 完全无效。
6. IME preedit 和 commit 均不能进入 ReadOnly。
7. 程序化修改内容仍然有效。
8. `InteractionDisabled` 的语义和现有外观保持不变。
9. `ReadOnly + InteractionDisabled` 最终表现为 Disabled。
10. 不增加新的 public marker 或 plugin。
11. Gallery 的 TextField 页面增加 ReadOnly 示例，并保留普通单行、多行和 Disabled 示例。
12. facade / public API tests 同步覆盖新控件。
