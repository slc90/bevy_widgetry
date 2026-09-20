# Widgetry RadioGroup 设计方案

## 目标

新增标准 RadioGroup 控件，行为建立在 Bevy 0.19.1 官方 headless：

- `RadioGroup`
- `RadioButton`
- `radio_self_update`
- `Checked`
- `ValueChange<Entity>`

之上。

Widgetry 不重新实现 Radio 的点击、键盘导航、互斥选择、focus / accessibility 等行为，只负责：

- 默认视觉样式
- BSN 公开 API
- `ValueChange<Entity>` → `ValueChange<usize>` 的公开值转换
- root disabled 状态向内部 RadioButton 的同步
- 程序化选择 API

不支持卡片式、多选式等其他选择控件形态；这些以后应作为独立控件设计。

---

# Crate 结构

新增：

```text
crates/radio_group/
├─ Cargo.toml
├─ src/
│  ├─ lib.rs
│  ├─ group.rs
│  ├─ group_style.rs
│  ├─ option.rs
│  └─ option_style.rs
└─ tests/
   └─ widgetry_radio_group.rs
```

package：

```text
bevy_widgetry_radio_group
```

facade：

```rust
bevy_widgetry::radio_group
```

公开：

```rust
WidgetryRadioGroup
WidgetryRadioOption
WidgetryRadioGroupPlugin
```

---

# 公开 API

基本使用方式：

```rust
bsn! {
    @WidgetryRadioGroup
    Children [
        (
            @WidgetryRadioOption
            Children [
                (Text("Low"))
            ]
        ),
        (
            @WidgetryRadioOption
            Children [
                (Text("Medium"))
            ]
        ),
        (
            @WidgetryRadioOption
            Children [
                (Text("High"))
            ]
        ),
    ]
}
```

`WidgetryRadioOption` 的显示内容完全由调用方通过 `Children` 提供。

不提供：

```text
value: T
RadioGroup<T>
RadioOption<T>
WidgetryRadioGroupProps
WidgetryRadioOptionProps
```

选择值统一使用 direct child 的 index。

用户选择发生变化时，由 `WidgetryRadioGroup` root 发出：

```rust
ValueChange<usize>
```

其中：

```text
0 = 第一个 WidgetryRadioOption
1 = 第二个 WidgetryRadioOption
...
```

`is_final` 原样继承官方 `ValueChange<Entity>`。

---

# 程序化选择

公开：

```rust
impl WidgetryRadioGroup {
    pub fn set_selected(
        commands: &mut Commands,
        entity: Entity,
        selected: usize,
    );
}
```

语义与 `WidgetryComboBox::set_selected()` 保持一致：

- 有效 index：静默修改当前选择
- 已经选中该 index：no-op
- index 越界：no-op
- entity 无效：no-op
- disabled group：仍允许程序化修改
- 不发送 `ValueChange<Entity>`
- 不发送 `ValueChange<usize>`

程序化修改直接维护内部 `Checked`：

```text
目标 option    -> insert Checked
其他 options  -> remove Checked
```

不通过官方 `ValueChange<Entity>`，避免把程序化修改当成用户操作。

---

# 用户选择行为

用户交互：

```text
mouse / keyboard
      ↓
官方 RadioButton / RadioGroup
      ↓
ValueChange<Entity>
      ├─ radio_self_update
      │      ↓
      │   维护 Checked
      │
      └─ Widgetry observer
             ↓
       Entity -> child index
             ↓
       ValueChange<usize>
```

Widgetry 不重新实现用户交互下的互斥选择逻辑。

`Checked` 属于内部状态，不作为 Widgetry 公开 API。

点击已经选中的 option 时沿用官方行为，不重复产生 selection change。

---

# 初始选择

RadioGroup 必须至少包含一个 option。

初始化完成后：

```text
index 0 -> Checked
其他    -> unchecked
```

初始化过程不发送任何 `ValueChange`。

外部预先插入的 `Checked` 不属于公开初始化 API；Widgetry 初始化时统一将 index 0 设置为选中项。

---

# 层级结构约束

合法结构：

```text
WidgetryRadioGroup
├─ WidgetryRadioOption
├─ WidgetryRadioOption
└─ WidgetryRadioOption
```

要求：

1. `WidgetryRadioGroup` direct children 不得为空。
2. 所有 direct child 必须是 `WidgetryRadioOption`。
3. `WidgetryRadioOption` 必须是 `WidgetryRadioGroup` 的 direct child。
4. `WidgetryRadioOption` 不允许脱离 Group 单独使用。
5. 不支持初始化完成后动态新增、删除或重排 options。

发现非法结构时，在初始化阶段：

```text
widgetry_error! 记录相关信息
        ↓
panic!
```

空 Group 的日志至少包含 root entity。

非法 child 或单独存在的 Option，日志应包含相关 root / child entity。

---

# WidgetryRadioOption Scene

`WidgetryRadioOption::scene()` 自带：

```text
WidgetryRadioOption
RadioButton
Hovered(false)
Node
Propagate<ForegroundColor>
Children
└─ indicator
   └─ dot
```

调用方声明的：

```rust
Children [...]
```

通过 BSN Scene composition 追加到内建 indicator 后面。

最终结构：

```text
WidgetryRadioOption + RadioButton
├─ indicator
│  └─ dot
└─ user children...
```

内部可使用 private marker：

```rust
RadioIndicator
RadioDot
```

供 Style system 精确定位。

---

# Option 默认布局与尺寸

Option root：

```rust
Node {
    flex_direction: FlexDirection::Row,
    align_items: AlignItems::Center,
    column_gap: px(6),
    min_height: px(24),
}
```

整个 `WidgetryRadioOption` / `RadioButton` 都属于点击区域。

indicator：

```rust
Node {
    width: px(16),
    height: px(16),
    border: UiRect::all(px(2)),
    border_radius: BorderRadius::MAX,
    align_items: AlignItems::Center,
    justify_content: JustifyContent::Center,
}
```

dot：

```rust
Node {
    width: px(8),
    height: px(8),
    border_radius: BorderRadius::MAX,
}
```

Option：

- 不提供自己的背景
- 不提供自己的外框
- 不实现卡片式 Radio 样式
- 不实现 pressed 状态样式
- 不提供独立 focus 样式

---

# Option 状态样式

支持：

```text
normal
hovered
checked
checked + hovered
disabled
```

默认颜色：

| State             | Indicator border          | Dot                              | Foreground            |
| ----------------- | ------------------------- | -------------------------------- | --------------------- |
| normal            | `control_border`          | transparent                      | `foreground`          |
| hovered           | `control_border_hovered`  | transparent                      | `foreground`          |
| checked           | `control_border_active`   | `control_border_active`          | `foreground`          |
| checked + hovered | `control_border_hovered`  | `control_border_active`          | `foreground`          |
| disabled          | `control_border_disabled` | checked 时 `foreground_disabled` | `foreground_disabled` |

checked 状态只通过标准圆圈和圆点表达，不给整行增加 selected background。

---

# WidgetryRadioGroup Scene

Group root 自带：

```text
WidgetryRadioGroup
RadioGroup
TabIndex::default()
Node
BackgroundColor
BorderColor
```

默认布局：

```rust
Node {
    flex_direction: FlexDirection::Column,
    align_items: AlignItems::Stretch,
    row_gap: px(6),

    padding: UiRect::all(px(8)),
    border: UiRect::all(px(1)),
    border_radius: BorderRadius::all(px(4)),
}
```

调用方可以通过普通 `Node` patch 修改布局，例如横向或 Grid。

不额外增加 orientation enum。

---

# Group 状态样式

默认：

```text
normal:
    background = control_background
    border     = control_border

focused:
    background = control_background
    border     = control_border_active

disabled:
    background = control_background_disabled
    border     = control_border_disabled
```

focus 只改变 border。

Group 使用 `TabIndex::default()` 参与官方 Tab navigation。

`InteractionDisabled` 不修改 `TabIndex`；disabled Group 保持 Bevy 官方现有的 focus / Tab 行为，不额外实现“退出 Tab 顺序”。

---

# Disabled 行为

调用方只在 Group root 上设置：

```rust
InteractionDisabled
```

不公开单个 Option disabled。

Group disabled 时，将 `InteractionDisabled` 镜像到所有 direct `WidgetryRadioOption`。

需要覆盖：

```text
root 新增 InteractionDisabled
root 移除 InteractionDisabled
初始化时 root 已经 disabled
```

只同步到 Option root，不递归向用户提供的 children 添加 `InteractionDisabled`。

由官方 `RadioButton` 自己负责禁止鼠标和键盘改选。

程序化：

```rust
WidgetryRadioGroup::set_selected(...)
```

不受 disabled 限制。

---

# Plugin

```rust
pub struct WidgetryRadioGroupPlugin;
```

自动安装缺失依赖：

```text
RadioGroupPlugin
TabNavigationPlugin
WidgetryFocusPlugin
ThemePlugin
ForegroundColorPlugin
```

均沿用项目现有模式：

```rust
if !app.is_plugin_added::<...>() {
    app.add_plugins(...);
}
```

另外注册：

```rust
radio_self_update
```

以及 Widgetry 自己的：

```text
初始化层级结构与默认选择
disabled 状态同步
ValueChange<Entity> -> ValueChange<usize>
Group style
Option style
ThemeChanged refresh
```

---

# 文件职责

## `lib.rs`

负责：

- module 声明
- public re-export
- `WidgetryRadioGroupPlugin`
- Plugin 依赖和 system / observer 注册

## `group.rs`

负责：

- `WidgetryRadioGroup`
- `scene()`
- `set_selected()`
- 层级结构初始化与校验
- 默认选择
- `ValueChange<Entity>` → `ValueChange<usize>`
- root → option disabled 同步

## `group_style.rs`

负责：

- Group normal / focus / disabled 样式
- Group `ThemeChanged` 刷新

## `option.rs`

负责：

- `WidgetryRadioOption`
- `scene()`
- indicator / dot 层级结构
- private indicator / dot marker

## `option_style.rs`

负责：

- hover
- checked
- disabled
- indicator border
- dot background
- propagated foreground
- `ThemeChanged`

---

# Workspace / facade

Workspace 增加：

```text
crates/radio_group
```

并加入 root `Cargo.toml` members。

`crates/bevy_widgetry` 增加依赖：

```text
bevy_widgetry_radio_group
```

facade 增加：

```rust
pub mod radio_group {
    pub use bevy_widgetry_radio_group::*;
}
```

`docs/architecture.md` 同步：

- workspace structure
- RadioGroup crate 职责
- dependency graph
- facade → radio_group
- radio_group → core
- radio_group → log
- radio_group dev → test_utils

RadioGroup 不需要依赖 asset / button 等其他 Widget crate。

---

# Tests

`tests/widgetry_radio_group.rs` 至少覆盖：

- BSN 构造完整层级结构
- Option 支持任意用户内容
- 默认第 0 项 `Checked`
- 用户点击后保持唯一选中项
- 用户操作发送一次 root `ValueChange<usize>`
- `is_final` 正确透传
- 重选当前 option 不重复通知
- `set_selected()` 静默
- `set_selected()` 更新唯一 `Checked`
- `set_selected()` 同值 no-op
- `set_selected()` 越界 no-op
- invalid entity no-op
- disabled root 禁止用户改选
- disabled 时程序化 `set_selected()` 仍有效
- disabled 样式
- hover / checked 样式
- Group focus border 样式
- theme 切换
- 横向 `Node` patch
- Grid `Node` patch
- 空 Group：error log + panic
- 非 Option direct child：error log + panic
- standalone Option：error log + panic

facade `public_api.rs` 增加 RadioGroup 公开入口和 BSN 构造验证。

---

# Gallery

不新增独立 Gallery page。

在现有 Button page 增加一个 Radio Button section：

```text
Button
  Normal
  Disabled

Radio Button
  Horizontal
  Grid
  Disabled
```

Radio 示例共三个。

## Horizontal

横向排列，例如：

```text
● Apple   ○ Banana   ○ Orange
```

验证 `Node` 横向 patch 和正常选择行为。

## Grid

二维排列，例如：

```text
● Apple     ○ Banana
○ Orange    ○ Grape
```

验证 RadioGroup 不依赖固定的纵向布局。

## Disabled

使用默认纵向布局：

```text
● Apple
○ Banana
○ Orange
```

root 挂 `InteractionDisabled`，验证整体 disabled 行为和视觉效果。

正常示例监听：

```rust
ValueChange<usize>
```

并记录选中的 index。

Gallery 使用 facade：

```rust
bevy_widgetry::radio_group::...
```

并注册 `WidgetryRadioGroupPlugin`。

---

# 明确不做

本次不包括：

- typed `T` value
- option 自定义业务 value
- RadioOption 单独使用
- 单个 Option disabled
- 空 Group
- 无选中项状态
- 动态新增 / 删除 / 重排 option
- card-style selection
- multi-select
- Option selected background
- pressed 状态样式
- 自定义 Radio orientation API
- disabled 时移除 `TabIndex`
- 重新实现官方 Radio keyboard / focus / accessibility 行为
