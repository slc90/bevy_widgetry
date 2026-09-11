# Stage 8 总结：TextField

> 项目：基于 Bevy 0.19 的自定义控件开发  
> Bevy 版本：0.19.1  
> Stage 8 目标：在尽量复用官方 `EditableText` 的前提下，完成一个可主题化、可测试、可放入 Gallery 的单行 `TextField`。

---

## 1. Stage 8 的核心结论

这一阶段最重要的决定不是“自己实现一个文本输入框”，而是：

```text
官方 EditableText
    ↓
复用其编辑能力
    ↓
我们只补真正缺失的 Headless 行为
    ↓
再加 Style / Theme / Gallery
```

最终采用的结构是：

```text
StyledTextField
    ↓
 TextField
    ↓
EditableText
```

`TextField` 不重新实现光标、选区、IME、剪贴板、Unicode 编辑等复杂逻辑，而是把这些全部交给 Bevy 官方的 `EditableText`。

---

## 2. 对官方 EditableText 的理解

Stage 8 中梳理了 `EditableText` 的核心职责。

它内部基于 Parley 的 `PlainEditor`，同时保存：

- 文本 buffer
- caret / cursor
- selection
- IME compose 状态
- pending edits
- pending paste
- 最大字符数
- 是否允许换行
- 可见行数 / 宽度等编辑配置

编辑输入大致经过：

```text
Keyboard / Pointer / IME
        ↓
     TextEdit
        ↓
EditableText.pending_edits
        ↓
PostUpdate
        ↓
 PlainEditor
        ↓
TextEditChange
```

其中一个重要认识是：

```text
EditableText 的“编辑状态”
≠
应用层业务 value
```

应用可以选择实时读取 `EditableText::value()`，也可以只在提交时读取。

---

## 3. TextField Headless 设计

第一版 `TextField` 明确限定为：

- 单行普通文本输入
- 不支持 multiline
- 不做 password
- 不做 number input
- 不做 search field
- 不做 validation
- 不做 formatter

核心组件：

```rust
#[derive(Component, Debug, Default)]
#[require(
    EditableText,
    TextLayout = TextLayout::no_wrap(),
    TextCursorStyle
)]
pub struct TextField;
```

这里几个设计点很关键。

### 3.1 单行语义

单行依赖两部分：

```text
EditableText::allow_newlines = false
+
TextLayout::no_wrap()
```

前者阻止硬换行，后者阻止软换行。

### 3.2 TextCursorStyle 必须显式存在

Bevy 0.19.1 的 `EditableText` 本身并不 require `TextCursorStyle`。

而 UI renderer 只有在实体上存在 `TextCursorStyle` 时，才会绘制：

- caret
- selection highlight

因此我们把它放进 `TextField` 的 required components。

---

## 4. Bevy 0.19.1 Disabled 行为修补

这一阶段发现了一个很重要的 0.19.1 行为：

`InteractionDisabled` 在该版本中并不会阻止 `EditableText` 的键盘编辑。

官方 pointer / keyboard EditableText handler 没有检查它。

因此我们增加了自己的 guard：

```rust
fn block_disabled_text_field_edits(
    mut query: Query<
        &mut EditableText,
        (With<TextField>, With<InteractionDisabled>),
    >,
) {
    for mut editable_text in &mut query {
        editable_text.pending_edits.clear();
        editable_text.pending_paste = None;
    }
}
```

并保证它在官方 `EditableTextSystems` 前运行：

```rust
app.add_systems(
    PostUpdate,
    block_disabled_text_field_edits.before(EditableTextSystems),
);
```

最终语义为：

```text
Disabled
→ 阻止用户 queued TextEdit
→ 阻止 pending paste
→ 仍允许程序直接 editor_mut().set_text(...)
```

也就是说：

```text
Disabled = 禁止用户编辑
不是 = 禁止程序更新
```

---

## 5. StyledTextField

视觉版本采用单实体结构，没有额外套一层 frame。

```text
TextField Entity
├─ StyledTextField
├─ TextField
├─ EditableText
├─ Node
├─ BackgroundColor
├─ BorderColor
├─ TextColor
├─ TextCursorStyle
└─ InteractionDisabled?
```

这样有几个好处：

- Focus 就在本实体
- Hover 就在本实体
- Border / Background 直接作用在本实体
- 不需要处理 focus-within
- 不需要额外 parent-child 状态同步

第一版布局：

```rust
Node {
    width: px(240),
    height: px(40),
    padding: UiRect::axes(px(10), px(6)),
    border: UiRect::all(px(1)),
    ..default()
}
```

---

## 6. Style 状态机

TextField 的状态优先级最终确定为：

```text
Disabled
   >
Focused
   >
Hovered
   >
Normal
```

映射到 Theme token：

```text
Normal
→ control_background
→ control_border

Hovered
→ control_background_hovered
→ control_border_hovered

Focused
→ control_background_active
→ control_border_active

Disabled
→ control_background_disabled
→ control_border_disabled
→ foreground_disabled
```

Style resolver 保持为纯函数：

```rust
fn resolve_text_field_style(
    colors: &ColorTheme,
    hovered: bool,
    focused: bool,
    disabled: bool,
) -> TextFieldStyle
```

这样状态判断和 ECS 写回彼此分离。

---

## 7. Style 更新方式

最开始使用了“每帧全量重算”来确认逻辑正确。

确认后改成了 change-driven 模式：

```text
Hovered changed ─────────┐
Disabled add/remove ─────┤
InputFocus changed ──────┼→ resolve → apply style
ThemeChanged ────────────┘
```

本地状态变化只更新受影响实体。

`InputFocus` 是全局 Resource，因此 Focus 改变时重新刷新 StyledTextField。

Theme 切换则继续沿用项目已有的 `ThemeChanged` observer 机制。

---

## 8. caret 与 selection Theme

`TextCursorStyle` 包含：

```rust
pub struct TextCursorStyle {
    pub color: Color,
    pub selection_color: Color,
    pub unfocused_selection_color: Color,
    pub selected_text_color: Option<Color>,
}
```

我们最终采用：

```text
cursor.color
→ foreground

selection_color
→ text_selection

unfocused_selection_color
→ text_selection_unfocused

selected_text_color
→ None
```

因此新增了两个专门的 Theme token：

```text
text_selection
text_selection_unfocused
```

而不是复用 ComboBox 的 `item_background_selected`。

原因是：

```text
文本选区
≠
列表 item selected
```

语义分开以后主题更容易继续演化。

---

## 9. 测试策略

Stage 8 继续保留 ECS 测试。

### Headless 测试

覆盖：

- `TextField` required components
- `EditableText`
- `TextCursorStyle`
- `TextLayout::NoWrap`
- 默认不允许换行
- Disabled 清理 queued edit
- Disabled 清理 pending paste
- Enabled 状态不误伤
- Disabled 时程序仍可直接修改文本

### Style ECS 测试

覆盖：

- Normal
- Hovered
- Focused
- Disabled
- 状态优先级
- Disabled 移除后的 fallback
- Theme 切换
- foreground
- cursor color
- selection colors

### Visual Test 调整

项目测试策略在这一阶段发生了调整：

```text
保留：
- Headless ECS tests
- Style ECS tests
- Theme / 状态测试

取消：
- screenshot baseline
- golden image
- Visual Test
```

视觉正确性改由 Gallery 直接人工查看。

这样更适合当前需求快速变化的阶段。

---

## 10. Gallery 的角色

Gallery 现在承担：

```text
“看起来好不好”
“交互是否自然”
“主题是否协调”
```

重点人工检查：

- Normal
- Hover
- Focused
- Disabled
- Dark Theme
- Light Theme
- caret
- selection
- unfocused selection
- 中文 IME
- 普通文本输入
- 长文本水平滚动

因此不再需要截图 baseline 作为视觉金标准。

---

## 11. Stage 8 中发现但延期的问题

有两个问题明确不在第一阶段继续扩展。

### 11.1 点击空白不会自动 blur

Bevy 0.19.1 的 `InputFocus` 是显式管理的：

```text
点击 EditableText
→ set focus

点击普通空白
→ 不一定自动 clear focus
```

真正清除焦点需要：

```rust
input_focus.clear();
```

这更适合以后做成全局 Focus Policy，而不是塞进 TextField。

延期到第二阶段。

### 11.2 multi-click > 3

Bevy Picking 默认在约 500ms 内累计连续点击次数。

EditableText 的逻辑大致是：

```text
1 click  → caret
2 clicks → word selection
3+       → select all
```

因此第四次、第五次快速点击仍然会继续 select all。

等超过 multi-click interval 后又会恢复正常。

第一阶段不覆盖官方 pointer handler，因此暂时接受该行为。

第二阶段可考虑：

```text
multi-click > 3
→ 重新解释为新的单击序列
```

---

## 12. 第一阶段 TextField 最终边界

最终第一版 TextField 做到：

```text
✅ 单行普通文本输入
✅ Unicode 编辑
✅ selection
✅ caret
✅ clipboard
✅ IME
✅ 水平滚动
✅ focus
✅ hover
✅ disabled
✅ Dark / Light Theme
✅ ECS tests
✅ Gallery
```

其中大部分复杂编辑能力来自 Bevy 官方 `EditableText`，我们自己的工作重点是：

```text
组合
+
行为补丁
+
状态样式
+
主题
```

而没有重新发明文本编辑器。

---

## 13. Stage 8 最大的架构收获

这一阶段最值得保留的经验是：

> 自定义控件库不应该把“自定义”理解成“全部自己实现”。

更合理的方式是：

```text
已有成熟能力
→ 尽量复用

缺失行为
→ 局部补充

视觉层
→ 独立 Style

颜色体系
→ Theme token

验证
→ ECS tests + Gallery
```

这也是后续第二阶段继续扩展控件时最值得复用的模式。

---

## 14. Stage 8 状态

```text
Stage 8 — TextField
Status: Complete ✅
```

同时，Stage 8 也标志着当前第一阶段固定控件路线基本完成。

接下来更适合先回顾第一阶段整体架构，再决定第二阶段真正需要发展的方向，而不是机械进入一个预设 Stage 9。
