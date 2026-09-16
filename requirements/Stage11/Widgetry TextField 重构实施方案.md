# Widgetry TextField 重构实施方案

## 目标

重构 `bevy_widgetry_text_field`，删除当前学习性质的自定义 headless 单行 TextField，直接以 Bevy 0.19.1 官方 `EditableText` 作为编辑能力基础。

本次主要处理：

* TextField API 统一为 `WidgetryXXX` 命名
* BSN 化
* 默认 Style / Layout
* 单行限制移除
* disabled 兼容逻辑迁移
* 全局 pointer focus 清理规则
* Gallery 与测试同步更新

不要修改 Theme token 体系，不新增 read-only 能力，不实现额外文本编辑 abstraction。

---

# 1. Public API

删除旧公开类型：

```rust
TextField
TextFieldPlugin
StyledTextField
StyledTextFieldPlugin
```

新增：

```rust
WidgetryTextField
WidgetryTextFieldPlugin
```

不要新增：

```rust
WidgetryTextFieldProps
```

`EditableText` 的配置直接由调用方 patch 官方组件。

---

# 2. 删除自定义 headless 层

当前：

```text
crates/text_field/src/
├── headless.rs
├── lib.rs
└── style.rs
```

修改为：

```text
crates/text_field/src/
├── lib.rs
└── style.rs
```

删除：

```text
crates/text_field/src/headless.rs
```

其中原有 disabled workaround 移入新的 TextField 实现模块。

`lib.rs` 最终只公开新的 Widgetry API，例如：

```rust
pub use style::{WidgetryTextField, WidgetryTextFieldPlugin};
```

---

# 3. WidgetryTextField BSN Scene

`WidgetryTextField` 改为 `SceneComponent`，使用方式：

```rust
bsn! {
    @WidgetryTextField
}
```

Scene 结构保持单实体，核心组成：

```rust
EditableText
Hovered(false)

Node {
    min_height: px(32),
    padding: UiRect::axes(px(10), px(6)),
    border: UiRect::all(px(1)),
    border_radius: BorderRadius::all(px(4)),
}

BackgroundColor
BorderColor
TextCursorStyle::default()
```

根据项目现有 BSN 写法调整具体语法。

### 不要主动提供这些组件

```rust
TextFont
TextLayout
LineHeight
TabIndex
```

`TextColor` 由 `EditableText` required component 提供，Widgetry 样式系统只修改它。

Bevy 0.19.1 `EditableText` 已 required：

```text
TextLayout
TextFont
TextColor
LineHeight
FontHinting
EditableTextGeneration
```

因此不要重复声明 Widgetry 默认。

---

# 4. 移除 Widgetry 单行限制

删除旧行为：

```rust
TextLayout::no_wrap()
```

不要替换成新的 Widgetry `TextLayout` 默认。

使用 `EditableText::default()` 时继续保持 Bevy 官方默认：

```text
visible_lines = Some(1.0)
allow_newlines = false
```

需要多行时由调用方直接配置：

```rust
EditableText {
    visible_lines: Some(4.0),
    allow_newlines: true,
    ..
}
```

以下内容全部属于官方 `EditableText` 配置，不包装为 Widgetry props：

```text
initial text
visible_lines
visible_width
allow_newlines
max_characters
EditableTextFilter
```

---

# 5. Layout

删除旧固定：

```rust
width: px(240)
height: px(40)
```

默认改为：

```rust
Node {
    min_height: px(32),
    padding: UiRect::axes(px(10), px(6)),
    border: UiRect::all(px(1)),
    border_radius: BorderRadius::all(px(4)),
    ..
}
```

不要设置固定 `height`。

`EditableText.visible_lines` 应能够通过 Bevy 官方 intrinsic measure 自然增加控件高度。

不要给默认 width。

---

# 6. Style

保持现有状态优先级：

```text
Disabled > Focused > Hovered > Normal
```

保持现有 Theme 映射：

```text
Normal
  background = control_background
  border     = control_border
  foreground = foreground

Hovered
  background = control_background_hovered
  border     = control_border_hovered
  foreground = foreground

Focused
  background = control_background_active
  border     = control_border_active
  foreground = foreground

Disabled
  background = control_background_disabled
  border     = control_border_disabled
  foreground = foreground_disabled
```

样式系统继续负责：

```rust
BackgroundColor
BorderColor
TextColor

TextCursorStyle {
    color,
    selection_color,
    unfocused_selection_color,
    selected_text_color,
}
```

其中：

```rust
cursor.color = foreground;
cursor.selection_color = colors.text_selection;
cursor.unfocused_selection_color = colors.text_selection_unfocused;
cursor.selected_text_color = None;
```

本次不要修改 Theme token 定义。

---

# 7. InteractionDisabled workaround

保留当前 Bevy 0.19.1 workaround。

但查询范围改为只影响 Widgetry TextField：

```rust
Query<
    &mut EditableText,
    (
        With<WidgetryTextField>,
        With<InteractionDisabled>,
    ),
>
```

继续在：

```rust
PostUpdate
```

且位于：

```rust
EditableTextSystems
```

之前执行。

行为：

```rust
editable_text.pending_edits.clear();
editable_text.pending_paste = None;
```

要求：

* disabled WidgetryTextField 不接受用户编辑
* 不影响程序代码直接 `set_text()`
* 不影响裸 `EditableText`
* 不重新设计 disabled 机制

---

# 8. 新增 WidgetryFocusPlugin

新增：

```text
crates/core/src/focus.rs
```

公开：

```rust
WidgetryFocusPlugin
```

并在：

```text
crates/core/src/lib.rs
```

导出。

同时通过 facade 的共享 style/core API 导出，使外部用户可以访问。

## 行为

全局处理：

```text
Primary Pointer<Press>
```

规则：

```text
目标实体有 EditableText
→ WidgetryFocusPlugin 不处理
→ 由 Bevy 官方 EditableText 自己 set / switch InputFocus

目标实体没有 EditableText
→ InputFocus::clear()
```

必须得到以下行为：

```text
EditableText A 已 focus
点击 EditableText B
→ A FocusLost
→ B FocusGained
```

```text
EditableText A 已 focus
点击普通 Node / WidgetryButton / 其他非 EditableText
→ A FocusLost
→ InputFocus 清空
```

```text
点击当前 EditableText
→ 保持正常 focus
```

```text
非 Primary Pointer<Press>
→ 不执行该全局清焦规则
```

不要加入：

```text
Tab navigation
focus marker
focus allowlist abstraction
方向键导航
通用 focus manager
```

### 实现注意

Button 自己会停止 pointer event propagation，因此实现后必须通过集成测试确认：

```text
已 focus TextField
→ 点击 WidgetryButton
→ TextField 确实失焦
```

不要只根据 observer 理论行为假设这个场景成立。

---

# 9. WidgetryTextFieldPlugin

最终职责：

```text
WidgetryTextFieldPlugin
├── 自动安装 ThemePlugin
├── 自动安装 WidgetryFocusPlugin
├── 注册 TextField Style 系统
└── 注册 InteractionDisabled workaround
```

使用项目现有模式：

```rust
if !app.is_plugin_added::<...>() {
    app.add_plugins(...);
}
```

不要自动安装：

```text
WidgetryFontPlugin
EditableTextInputPlugin
```

字体由调用方/App 决定。

官方 `EditableText` 输入插件属于应用的 Bevy UI widgets 配置责任。

---

# 10. TabIndex

`WidgetryTextField` 默认不要添加：

```rust
TabIndex(...)
```

保持官方 `EditableText` 本身的状态，即默认没有 `TabIndex`。

调用方需要 Tab 导航时自行添加。

---

# 11. Gallery

修改：

```text
gallery/src/pages/text_field.rs
```

旧的两个普通 `StyledTextField` 示例删除。

最终保留三个示例。

## 普通单行

```rust
@WidgetryTextField
EditableText::new("Single line")
```

## 多行高度

```rust
@WidgetryTextField
EditableText {
    visible_lines: Some(4.0),
    allow_newlines: true,
    ..
}
```

可放一个适合观察换行/高度的初始字符串。

该示例主要验证：

```text
visible_lines = 4
→ TextField 自然变高
→ Widgetry 没有固定 height
```

## Disabled

```rust
@WidgetryTextField
EditableText::new("Disabled")
InteractionDisabled
```

Gallery 人工检查：

```text
默认尺寸
padding / border / radius
hover
focused
失焦
多行高度
disabled 外观
disabled 无法编辑
```

---

# 12. TextField tests

当前：

```text
crates/text_field/tests/
├── disabled.rs
└── styled_text_field.rs
```

改为：

```text
crates/text_field/tests/
├── disabled.rs
└── widgetry_text_field.rs
```

即：

```text
styled_text_field.rs
→ widgetry_text_field.rs
```

## `disabled.rs`

迁移旧测试到：

```rust
WidgetryTextField
WidgetryTextFieldPlugin
```

继续验证：

* disabled pending edits 被清理
* disabled pending paste 被清理
* enabled TextField 不受影响
* disabled 不阻止程序化 `set_text()`
* workaround 不影响裸 `EditableText`

## `widgetry_text_field.rs`

迁移已有 Style 测试：

* 初始 Normal
* Hovered
* Focused
* Disabled
* `Disabled > Focused > Hovered > Normal`
* disabled 移除后恢复正确状态
* focus 变化后样式更新
* ThemeChanged 后刷新
* cursor / selection colors

删除旧断言：

```text
Widgetry 强制 TextLayout::no_wrap()
Widgetry 自己强制单行
```

增加：

* `@WidgetryTextField` 可以通过 BSN spawn
* entity 存在 `WidgetryTextField`
* entity 存在 `EditableText`
* Node 默认没有固定 width
* Node 默认没有固定 height
* `min_height == 32px`
* caller 提供的 `EditableText.visible_lines` 不被覆盖
* caller 提供的 `allow_newlines` 不被覆盖
* 不隐式安装 Widgetry font fallback

测试重点是 Widgetry 自己的 contract，不要重复测试 Bevy 内部文本编辑实现。

---

# 13. Focus tests

新增：

```text
crates/core/tests/focus.rs
```

测试 `WidgetryFocusPlugin` 自己定义的 policy。

至少覆盖：

### 非 EditableText primary press

```text
InputFocus = A
primary press 非 EditableText
→ InputFocus = None
```

### EditableText → EditableText

```text
InputFocus = A
primary press B
B 是 EditableText
→ WidgetryFocusPlugin 不提前 clear
→ 最终 B 获得 focus
```

### 点击当前 EditableText

```text
InputFocus = A
primary press A
→ A 保持正常 focus
```

### 非 primary

```text
InputFocus = A
secondary/middle press 非 EditableText
→ 不触发 Widgetry 清焦规则
```

### Button integration

使用真实 `WidgetryButton`：

```text
EditableText A 已 focus
primary press WidgetryButton
→ A 最终失焦
```

这个测试用于防止 Button 停止 pointer propagation 后破坏 Widgetry 全局失焦规则。

不要扩展成 Bevy InputFocus 全量测试。

---

# 14. Facade / public API tests

修改：

```text
crates/bevy_widgetry/tests/public_api.rs
```

删除：

```rust
StyledTextFieldPlugin
```

相关 import 和使用。

当前“UI plugin 自动安装默认字体 fallback”的测试中，不再把 TextField plugin 作为会安装字体 fallback 的对象。

增加 TextField facade/BSN 覆盖，例如验证：

```rust
WidgetryTextField
WidgetryTextFieldPlugin
```

可以只通过：

```rust
bevy_widgetry::text_field
```

使用。

并确认：

```text
WidgetryTextFieldPlugin
不会改变调用方默认字体策略
```

同步公开：

```rust
WidgetryFocusPlugin
```

到约定的 facade 入口。

---

# 15. 不要实现的内容

本次明确不要加入：

```text
WidgetryTextFieldProps
Widgetry 自定义 headless TextField
multiline abstraction
read-only EditableText
selectable read-only Text
placeholder
validation
password masking
Theme token 重构
Tab navigation policy
通用 focus manager
```

不要因为实现过程中发现这些邻近需求而扩 scope。

---

# 16. 最终验收标准

完成后应满足：

1. 仓库中不再存在旧公开类型：

```text
TextField
TextFieldPlugin
StyledTextField
StyledTextFieldPlugin
```

2. TextField 对外 API 为：

```text
WidgetryTextField
WidgetryTextFieldPlugin
```

3. `@WidgetryTextField` 可独立构造完整可编辑控件。

4. Widgetry 不再强制：

```text
no_wrap
240px width
40px height
Widgetry font fallback
```

5. `visible_lines` 能控制多行高度。

6. disabled 用户编辑仍被阻止。

7. 点击非 `EditableText` 后当前文本编辑 focus 被清除。

8. 点击另一个 `EditableText` 能正常完成 A → B focus 切换。

9. 点击 `WidgetryButton` 能让当前 EditableText 失焦。

10. Gallery 包含：

```text
普通单行
多行高度
disabled
```

11. 测试目录最终至少包含：

```text
crates/text_field/tests/
├── disabled.rs
└── widgetry_text_field.rs

crates/core/tests/
└── focus.rs
```

12. 相关 public API tests 同步通过。

实现完成后运行与本次修改直接相关的 crate tests / workspace tests，并修复由本次 API 重构造成的编译失败；不要顺手处理与本任务无关的问题。
