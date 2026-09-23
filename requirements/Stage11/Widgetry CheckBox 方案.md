# Widgetry CheckBox / TriStateCheckbox 实施方案

## 1. 目标

新增独立的 `check_box` Widget crate，同时提供：

* 基于 Bevy 0.19.1 官方 `Checkbox` headless 行为的 `WidgetryCheckBox`
* Widgetry 自己实现的真正三态 `WidgetryTriStateCheckbox`
* 两者共用 Indicator、SVG asset 和 Style
* Gallery 增加独立 Checkbox 展示页
* 普通与三态 Checkbox 都保留正确的 Accessibility 语义

普通二态不重新实现 Bevy 已有行为。

三态行为尽量保持与官方 Checkbox 一致，只将二态 toggle 扩展为三态 cycle。

Accessibility 本次只实现 Checkbox 当前必要的基础语义，不进一步扩展统一 Label、Name、AI Agent 等设计。

---

# 2. Workspace 与 crate

新增：

```text
crates/check_box/
├── Cargo.toml
├── src/
│   ├── lib.rs
│   ├── checkbox.rs
│   ├── tri_state.rs
│   ├── indicator.rs
│   └── style.rs
└── tests/
```

Package：

```toml
name = "bevy_widgetry_check_box"
```

生产依赖：

```text
bevy
accesskit
bevy_widgetry_asset
bevy_widgetry_core
bevy_widgetry_log
```

测试依赖：

```text
bevy_widgetry_test_utils
```

Root workspace 增加：

```text
crates/check_box
```

Bevy 0.19.1 不再从 `bevy_a11y` re-export AccessKit，因此 workspace dependency 增加：

```toml
accesskit = { version = "0.24", default-features = false }
```

版本与 Bevy 0.19.1 自己使用的 AccessKit 保持一致。

---

# 3. 公开 API

Facade 新增：

```rust
pub mod check_box {
    pub use bevy_widgetry_check_box::*;
}
```

公开类型：

```rust
pub struct WidgetryCheckBox;

pub struct WidgetryTriStateCheckbox;

pub enum WidgetryCheckState {
    Unchecked,
    Checked,
    Indeterminate,
}

pub struct WidgetryCheckBoxPlugin;
```

不提供：

```text
WidgetrySetCheckState
WidgetryCycleCheckState
```

程序化状态修改使用 `WidgetryTriStateCheckbox` 的方法。

---

# 4. 三态程序化 API

```rust
impl WidgetryTriStateCheckbox {
    pub fn set_state(
        commands: &mut Commands,
        entity: Entity,
        state: WidgetryCheckState,
    );

    pub fn cycle_state(
        commands: &mut Commands,
        entity: Entity,
    );
}
```

## `set_state`

直接设置指定状态：

```text
Unchecked
Checked
Indeterminate
```

目标状态等于当前状态时：

```text
no-op
```

## `cycle_state`

根据当前状态前进一步：

```text
Unchecked
    ↓
Checked
    ↓
Indeterminate
    ↓
Unchecked
```

## 共同规则

程序化操作：

```text
set_state
cycle_state
```

均：

* 直接修改真实状态
* 不发送 `ValueChange`
* `InteractionDisabled` 时仍允许
* 无效 entity 或非 TriStateCheckbox 时 no-op

内部共用一个实际写状态的 private helper。

不要在 `cycle_state()` 内再次调用公开 `set_state(commands, ...)`，避免 Commands queue 中再次 queue。

---

# 5. 普通二态 Checkbox

`WidgetryCheckBox` 是 `SceneComponent`。

内部直接使用 Bevy 官方：

```rust
Checkbox
Checked
Pressed
InteractionDisabled
ActivateOnPress
ValueChange<bool>
checkbox_self_update
```

默认没有：

```rust
Checked
```

因此默认：

```text
Unchecked
```

显式初始选中：

```rust
@WidgetryCheckBox
Checked
{
    ...
}
```

用户交互产生：

```rust
ValueChange<bool>
```

通过官方：

```rust
checkbox_self_update
```

自动维护 `Checked`。

Widgetry 不重新实现官方 Checkbox 的 pointer / keyboard 状态机。

---

# 6. 三态状态模型

```rust
#[derive(
    Component,
    Default,
    Clone,
    Copy,
    Debug,
    PartialEq,
    Eq,
    Reflect,
)]
pub enum WidgetryCheckState {
    #[default]
    Unchecked,
    Checked,
    Indeterminate,
}
```

使用单一 enum。

不使用：

```text
Checked + Indeterminate
```

两个独立状态 Component。

因此不存在 Checked 与 Indeterminate 同时成立的非法组合。

默认：

```text
WidgetryCheckState::Unchecked
```

---

# 7. 三态循环函数

集中定义纯函数：

```rust
fn next_check_state(
    state: WidgetryCheckState,
) -> WidgetryCheckState {
    match state {
        WidgetryCheckState::Unchecked => {
            WidgetryCheckState::Checked
        }
        WidgetryCheckState::Checked => {
            WidgetryCheckState::Indeterminate
        }
        WidgetryCheckState::Indeterminate => {
            WidgetryCheckState::Unchecked
        }
    }
}
```

以下路径都复用它：

```text
Pointer
Keyboard
cycle_state()
```

不在不同 observer 中重复三态 match。

---

# 8. 三态 headless 身份

`WidgetryTriStateCheckbox` 同时作为：

* SceneComponent
* ECS 身份
* 三态交互行为目标

要求存在：

```rust
WidgetryCheckState
AccessibilityNode
```

默认状态：

```text
Unchecked
```

Accessibility Role：

```text
CheckBox
```

三态不挂官方：

```rust
Checkable
```

因为官方 `Checkable` 的 checked 同步建立在官方 `Checked` Component 上，而三态真实状态来源是：

```rust
WidgetryCheckState
```

不应同时维护两套 checked state。

---

# 9. Pointer 行为

三态 Pointer 行为尽量与官方 Checkbox 保持一致。

默认：

```text
Press
    ↓
插入 Pressed

Click
    ↓
next_check_state(current)
    ↓
ValueChange<WidgetryCheckState>

Release / DragEnd / Cancel
    ↓
移除 Pressed
```

Press 时保持官方 Checkbox 的 focus 行为：

* 设置 input focus
* 使用 `FocusCause::Pressed`
* 隐藏 keyboard focus visible 状态
* 按官方 Checkbox 相同方式处理传播

---

# 10. ActivateOnPress

支持官方：

```rust
ActivateOnPress
```

默认不挂。

没有 `ActivateOnPress`：

```text
Press
    ↓
Pressed

Click
    ↓
状态变化
```

存在 `ActivateOnPress`：

```text
Press
    ↓
Pressed
    ↓
立即产生下一状态

Click
    ↓
不再次产生状态变化
```

一次 pointer 操作最多循环一次。

---

# 11. Keyboard 行为

聚焦后的三态 Checkbox 支持：

```text
Enter
Space
```

均：

```text
current state
    ↓
next_check_state()
    ↓
ValueChange<WidgetryCheckState>
```

只处理：

```rust
ButtonState::Pressed
```

并要求：

```rust
event.repeat == false
```

Keyboard repeat 不重复切换状态。

---

# 12. InteractionDisabled

三态 Widget 存在：

```rust
InteractionDisabled
```

时：

```text
Pointer
Enter
Space
```

均不改变状态，也不发送 `ValueChange`。

程序化：

```text
set_state()
cycle_state()
```

仍允许执行。

因此 disabled 的语义是：

> 禁止用户交互，不禁止应用代码修改状态。

与当前 Widgetry ComboBox / RadioGroup 的程序化 API 保持一致。

---

# 13. ValueChange 语义

`ValueChange` 专门表示：

> 用户实际交互造成的值变化。

用户：

```text
Click / Press activation
Enter
Space
    ↓
ValueChange<WidgetryCheckState>
    ↓
self update
    ↓
修改 WidgetryCheckState
```

事件：

```rust
ValueChange {
    source,
    value: next_state,
    is_final: true,
}
```

Checkbox 没有 Slider 那种 intermediate value，因此：

```rust
is_final: true
```

程序化：

```text
set_state()
cycle_state()
```

不产生：

```rust
ValueChange<WidgetryCheckState>
```

目标状态与当前状态相同时不产生无意义状态写入。

---

# 14. 三态 self-update

提供内部：

```rust
widgetry_tri_state_checkbox_self_update
```

作用类似官方：

```rust
checkbox_self_update
```

接受：

```rust
ValueChange<WidgetryCheckState>
```

更新：

```rust
WidgetryCheckState
```

只处理属于：

```rust
WidgetryTriStateCheckbox
```

的事件来源。

用户输入路径：

```text
Input
    ↓
ValueChange
    ↓
self update
    ↓
WidgetryCheckState
```

程序化 API 则直接写状态。

---

# 15. Accessibility

## 普通二态

普通 `WidgetryCheckBox` 直接使用 Bevy 官方：

```rust
Checkbox
```

因此继续使用官方：

```text
Role::CheckBox
Checkable
Checked
```

以及 Bevy 已有的 Accessibility 同步。

Widgetry 不重复实现。

## 三态

三态自行提供：

```text
Role::CheckBox
```

并从真实状态：

```rust
WidgetryCheckState
```

派生 accessibility toggled state：

```text
Unchecked
    -> Toggled::False

Checked
    -> Toggled::True

Indeterminate
    -> Toggled::Mixed
```

同步由：

```text
Changed<WidgetryCheckState>
```

驱动。

这样无论状态来源是：

* 用户 ValueChange + self-update
* `set_state`
* `cycle_state`
* 外部直接修改 `WidgetryCheckState`

Accessibility 都能同步。

Accessibility 更新不塞进 self-update observer 中，避免只覆盖用户交互路径。

本次只保证：

```text
Role::CheckBox
False / True / Mixed
InteractionDisabled 的官方 disabled 语义
```

不进一步设计：

```text
Name
Label relationship
Description
统一 Widgetry accessibility abstraction
AI Agent 专用语义
```

这些后续单独讨论。

---

# 16. Scene 结构

普通和三态结构一致：

```text
Checkbox root
├── CheckBoxIndicator
│   └── CheckBoxMark
└── 调用方 Children...
```

Root 默认：

```rust
Node {
    flex_direction: FlexDirection::Row,
    align_items: AlignItems::Center,
    column_gap: px(6),
    min_height: px(24),
}
```

并传播：

```rust
Propagate(ForegroundColor)
```

调用方：

```rust
@WidgetryCheckBox
Children [
    Text("Enable Shadows")
]
```

或：

```rust
@WidgetryTriStateCheckbox
Children [
    Text("Select All")
]
```

调用方 Children 自动追加在内建 Indicator 后。

不增加：

```text
label prop
children prop
slot abstraction
builder callback
```

继续使用 BSN Children composition。

---

# 17. Indicator

普通和三态共用：

```rust
fn checkbox_indicator_scene() -> impl Scene;
```

该 helper 不需要独立 ECS 身份，因此不是 SceneComponent。

Private marker：

```rust
struct CheckBoxIndicator;
struct CheckBoxMark;
```

结构：

```text
CheckBoxIndicator
└── CheckBoxMark @WidgetryIcon
```

---

# 18. Indicator 几何

Indicator：

```text
width          18px
height         18px
border         1px
border-radius  4px
flex-shrink    0
```

内部：

```text
align-items      center
justify-content  center
```

Mark：

```text
12 × 12
```

Gallery 实际运行后如果视觉偏小，再统一调整尺寸。

---

# 19. SVG 切换方案

Indicator 内始终只有：

```text
一个 WidgetryIcon
```

状态变化时切换 SVG，与现有 ComboBox 箭头采用同类机制。

## Unchecked

```text
CheckBoxMark
    Visibility::Hidden
```

当前 SVG 是哪个不重要。

## Checked

```text
WidgetryIcon::set_svg(
    CheckboxCheck
)

Visibility::Inherited
```

## Indeterminate

```text
WidgetryIcon::set_svg(
    CheckboxIndeterminate
)

Visibility::Inherited
```

因此：

```text
Checked
    ↓
Indeterminate
```

只修改同一个 `WidgetryIcon` 的 SVG。

不创建第二个 mark entity。

---

# 20. Visibility::Hidden

当前结构：

```text
Indicator
└── CheckBoxMark
```

始终只有一个固定：

```text
12 × 12
```

child。

因此：

```rust
Visibility::Hidden
```

虽然仍参与 layout，但这里正好希望 Indicator 内部始终保留相同 mark 占位。

不需要：

```rust
Node.display = Display::None
```

---

# 21. 普通 Checkbox SVG 状态

普通二态：

```text
Unchecked
    ↓
CheckBoxMark Hidden

Checked
    ↓
checkbox_check.svg
    ↓
Visible
```

普通二态不会进入：

```text
CheckboxIndeterminate
```

状态。

---

# 22. 三态 SVG 状态

三态：

```text
Unchecked
    ↓
Hidden

Checked
    ↓
checkbox_check.svg
    ↓
Visible

Indeterminate
    ↓
checkbox_indeterminate.svg
    ↓
Visible
```

恢复 `Unchecked` 时：

```text
只隐藏 mark
```

无需强制切回某个 SVG。

---

# 23. 共用视觉状态

Style 内部定义：

```rust
enum CheckBoxVisualState {
    Unchecked,
    Checked,
    Indeterminate,
}
```

普通：

```text
Has<Checked>
    false -> Unchecked
    true  -> Checked
```

三态：

```text
WidgetryCheckState::Unchecked
    -> Unchecked

WidgetryCheckState::Checked
    -> Checked

WidgetryCheckState::Indeterminate
    -> Indeterminate
```

之后进入同一 Style resolver。

---

# 24. Style 优先级

严格使用：

```text
disabled
    >
pressed
    >
hovered
    >
active
    >
normal
```

其中：

```text
active =
    Checked
    或
    Indeterminate
```

状态 SVG 与颜色优先级分开处理。

不要散落大量：

```text
checked && hovered
checked && pressed
indeterminate && hovered
indeterminate && disabled
```

组合 case。

---

# 25. Background / Border

Normal：

```text
background = control_background
border     = control_border
```

Active：

```text
background = control_background_active
border     = control_border_active
```

Hovered：

```text
background = control_background_hovered
border     = control_border_hovered
```

Pressed：

```text
background = control_background_pressed
border     = control_border_pressed
```

Disabled：

```text
background = control_background_disabled
border     = control_border_disabled
```

Checked 和 Indeterminate 在颜色层面都属于 active。

区别只在 SVG。

---

# 26. Foreground

Root 传播：

```rust
Propagate(ForegroundColor)
```

Enabled：

```text
foreground
```

Disabled：

```text
foreground_disabled
```

外部：

```rust
Text(...)
```

会跟随 disabled 状态。

Style 不递归修改调用方任意 child，只更新 root 的传播源。

---

# 27. Mark 颜色

可见 mark：

```text
Enabled
    -> control_border_active

Disabled
    -> foreground_disabled
```

Checked 与 Indeterminate 使用相同 mark 色。

SVG 使用：

```text
currentColor
```

最终由 `WidgetryIcon` 调色。

---

# 28. SVG asset

新增：

```text
crates/asset/src/assets/icons/
├── checkbox_check.svg
└── checkbox_indeterminate.svg
```

---

# 29. checkbox_check.svg

```xml
<svg xmlns="http://www.w3.org/2000/svg"
     width="16"
     height="16"
     viewBox="0 0 16 16"
     color="white">
  <path
    d="M3.5 8 6.75 11.25 12.5 5.5"
    fill="none"
    stroke="currentColor"
    stroke-width="1.5"
    stroke-linecap="round"
    stroke-linejoin="round"/>
</svg>
```

---

# 30. checkbox_indeterminate.svg

```xml
<svg xmlns="http://www.w3.org/2000/svg"
     width="16"
     height="16"
     viewBox="0 0 16 16"
     color="white">
  <path
    d="M4 8H12"
    fill="none"
    stroke="currentColor"
    stroke-width="1.5"
    stroke-linecap="round"/>
</svg>
```

---

# 31. SVG 规范

保持现有 Widgetry icon 约定：

```text
16 × 16 viewBox
currentColor
1.5px stroke
round linecap
round linejoin
```

Checkbox 实际最大 raster：

```rust
Some(UVec2::new(12, 12))
```

---

# 32. BuiltinIcon

新增：

```rust
BuiltinIcon::CheckboxCheck
BuiltinIcon::CheckboxIndeterminate
```

同步补：

```text
BuiltinIcon::path()
WidgetryAssetPlugin embedded 注册
embedded asset test
```

命名使用 Checkbox 语义，不使用：

```text
minus.svg
Minus
```

`BuiltinIcon` 继续作为 workspace 内部 API，不由 facade 导出。

---

# 33. WidgetryCheckBoxPlugin

只提供：

```rust
WidgetryCheckBoxPlugin
```

同时负责：

```text
WidgetryCheckBox
WidgetryTriStateCheckbox
```

自动确保：

```text
CheckboxPlugin
ThemePlugin
ForegroundColorPlugin
WidgetryAssetPlugin
WidgetryIconPlugin
```

不因为 Checkbox 自动增加：

```text
TabNavigationPlugin
WidgetryFocusPlugin
```

默认可继续：

```rust
TabIndex(-1)
```

如果调用方以后主动纳入 Tab navigation，则由应用自身输入和导航设施负责。

Plugin 文档说明：

* 在 Bevy `AssetPlugin` 后注册
* BSN 构造依赖 Scene 设施
* 不安装字体 fallback

---

# 34. Style 更新入口

普通 Checkbox 响应：

```text
Added<WidgetryCheckBox>
Changed<Hovered>
Added<Pressed>
Removed<Pressed>
Added<InteractionDisabled>
Removed<InteractionDisabled>
Added<Checked>
Removed<Checked>
ThemeChanged
```

三态：

```text
Added<WidgetryTriStateCheckbox>
Changed<Hovered>
Added<Pressed>
Removed<Pressed>
Added<InteractionDisabled>
Removed<InteractionDisabled>
Changed<WidgetryCheckState>
ThemeChanged
```

最终调用同一套：

```text
resolve visual state
resolve colors
apply_checkbox_style
sync mark
```

---

# 35. SVG 同步

Mark SVG 同步集中处理。

类似 ComboBox：

```text
logical state
    ↓
决定 BuiltinIcon
    ↓
WidgetryIcon::set_svg(...)
```

只在目标 SVG 实际发生变化时调用 `set_svg`。

状态变化驱动：

```text
Checked added / removed
WidgetryCheckState changed
```

Hover / Pressed 等纯颜色变化不重复切 SVG。

---

# 36. Gallery

新增：

```text
gallery/src/pages/check_box.rs
```

`pages.rs`：

```rust
mod check_box;
```

并导出：

```rust
pub(crate) use check_box::scene as check_box;
```

如果页面标题 foreground 需要响应 ThemeChanged，可提供：

```rust
CheckBoxDemoPlugin
```

只管理 Gallery 自己的样式。

---

# 37. GalleryPage

增加：

```rust
enum GalleryPage {
    Button,
    CheckBox,
    ComboBox,
    TextField,
    Window,
}
```

Sidebar：

```text
Button
CheckBox
ComboBox
TextField
Window
```

PageHost 增加：

```text
CheckBoxPage
```

`main.rs` 注册：

```rust
WidgetryCheckBoxPlugin
```

---

# 38. Gallery 页面

```text
CheckBox

Normal
  □ Unchecked
  ☑ Checked

Disabled
  □ Unchecked
  ☑ Checked
  ⊟ Indeterminate

Tri-State
  ☐ Tri-State Checkbox
```

---

# 39. Normal

两个普通 Checkbox：

```text
Unchecked
Checked
```

都是真实可交互控件。

初始 Checked 直接添加：

```rust
Checked
```

---

# 40. Disabled

三只：

```text
普通 Unchecked
普通 Checked
三态 Indeterminate
```

全部：

```rust
InteractionDisabled
```

用于同时检查：

```text
disabled unchecked
disabled checked
disabled indeterminate
```

---

# 41. Tri-State

只放一个真实可交互三态：

```text
Tri-State Checkbox
```

交互：

```text
Unchecked
    ↓
Checked
    ↓
Indeterminate
    ↓
Unchecked
```

不额外增加：

```text
Tri-State States
```

静态展示区域。

---

# 42. Gallery 日志

普通：

```rust
On<ValueChange<bool>>
```

三态：

```rust
On<ValueChange<WidgetryCheckState>>
```

用于人工确认真实用户交互事件。

Gallery 不增加自动化测试。

只要求：

```text
cargo check
实际运行
Dark / Light Theme 检查
交互检查
SVG 尺寸检查
```

---

# 43. Facade

`crates/bevy_widgetry/Cargo.toml`：

```toml
bevy_widgetry_check_box = { path = "../check_box" }
```

`crates/bevy_widgetry/src/lib.rs`：

```rust
pub mod check_box {
    pub use bevy_widgetry_check_box::*;
}
```

消费者只需：

```rust
use bevy_widgetry::check_box::*;
```

---

# 44. Architecture 文档

更新：

```text
docs/architecture.md
```

Workspace 结构增加：

```text
crates/check_box/
```

Workspace Role：

> 基于 Bevy 官方 Checkbox 提供 styled binary Checkbox，并提供 Widgetry 自己的 tri-state Checkbox；两者共享 indicator、theme style 与内建 SVG asset，并保持对应 Checkbox Accessibility 语义。

Dependency Graph：

```text
bevy_widgetry
    -> check_box

check_box
    -> core
    -> asset
    -> log

check_box
    -. dev .-> test_utils
```

`accesskit` 是外部 dependency，不作为内部 crate dependency graph 节点处理。

---

# 45. 测试原则

重点测试：

> Widgetry 自己新增的行为与 Bevy 组合边界。

不重复完整测试 Bevy 官方 Checkbox 内部实现。

Gallery 按现有规则不写自动化测试。

---

# 46. 状态模型测试

```text
WidgetryCheckState::default()
    == Unchecked
```

以及：

```text
Unchecked -> Checked
Checked -> Indeterminate
Indeterminate -> Unchecked
```

---

# 47. 三态 Scene 测试

构造：

```rust
@WidgetryTriStateCheckbox
```

确认：

```text
存在 WidgetryTriStateCheckbox
存在 WidgetryCheckState
默认 == Unchecked
存在 AccessibilityNode
Role == CheckBox
```

并确认内部：

```text
CheckBoxIndicator
唯一一个 CheckBoxMark
```

---

# 48. 普通 Checkbox Scene 测试

构造：

```rust
@WidgetryCheckBox
```

确认：

```text
存在 WidgetryCheckBox
存在官方 Checkbox
默认不存在 Checked
```

即默认：

```text
Unchecked
```

不重新测试 Bevy 官方完整 pointer state machine。

---

# 49. set_state 测试

覆盖显式状态设置：

```text
Unchecked
Checked
Indeterminate
```

确认：

```text
InteractionDisabled
```

存在时仍可以设置。

同值：

```text
no-op
```

并验证：

```text
不发送 ValueChange<WidgetryCheckState>
```

---

# 50. cycle_state 测试

连续：

```text
Unchecked
    ↓
Checked
    ↓
Indeterminate
    ↓
Unchecked
```

Disabled 时仍允许。

并验证：

```text
不发送 ValueChange<WidgetryCheckState>
```

---

# 51. 用户 ValueChange 测试

三态激活：

```text
Unchecked
    -> ValueChange(Checked)

Checked
    -> ValueChange(Indeterminate)

Indeterminate
    -> ValueChange(Unchecked)
```

确认：

```rust
is_final == true
```

Self-update 后：

```text
WidgetryCheckState == ValueChange.value
```

Disabled 时：

```text
不发送 ValueChange
状态不变化
```

---

# 52. Keyboard 测试

如果现有 test infrastructure 能稳定注入 focused keyboard input，则覆盖：

```text
Space
Enter
```

均循环一次。

并确认：

```text
repeat == true
```

时不触发。

若现有测试设施不适合完整 input pipeline，则不为了这一项扩张 test infrastructure，由 observer 核心测试与 Gallery 人工验证补足。

---

# 53. ActivateOnPress 测试

组合：

```text
WidgetryTriStateCheckbox
+
ActivateOnPress
```

确认：

```text
Press
    -> 只循环一次

后续 Click
    -> 不再次循环
```

---

# 54. Style resolver 测试

测试优先级：

```text
disabled
    >
pressed
    >
hovered
    >
active
    >
normal
```

重点覆盖：

```text
checked + hover
indeterminate + hover
checked + pressed
disabled + checked
disabled + indeterminate
```

不需要穷举所有组合。

---

# 55. SVG 状态测试

检查同一个 `CheckBoxMark` entity。

Unchecked：

```text
Visibility::Hidden
```

Checked：

```text
Visibility != Hidden
WidgetryIcon SVG == CheckboxCheck
```

Indeterminate：

```text
Visibility != Hidden
WidgetryIcon SVG == CheckboxIndeterminate
```

---

# 56. SVG 单 entity 切换测试

测试：

```text
Checked
    ↓
Indeterminate
```

之后：

```text
CheckBoxMark entity id 不变
```

只发生：

```text
WidgetryIcon SVG 改变
```

不能 spawn 第二个 mark。

再测试：

```text
Indeterminate
    ↓
Unchecked
```

只变成：

```text
Visibility::Hidden
```

mark entity 仍存在。

---

# 57. Accessibility 测试

三态基础映射：

```text
Unchecked
    -> Toggled::False

Checked
    -> Toggled::True

Indeterminate
    -> Toggled::Mixed
```

另外验证一次程序化状态修改：

```text
set_state(...)
```

之后 Accessibility 同步更新。

这用于证明 Accessibility 监听的是：

```text
Changed<WidgetryCheckState>
```

而不是只绑定用户 self-update 路径。

本次不增加更深入的 accessibility 测试矩阵。

---

# 58. Asset 测试

扩展：

```text
builtin_icons_resolve_to_registered_embedded_assets
```

加入：

```rust
BuiltinIcon::CheckboxCheck
BuiltinIcon::CheckboxIndeterminate
```

验证：

* embedded source 正确
* path 唯一
* 内容非空
* SVG 内容彼此不同
* 与现有 BuiltinIcon 不冲突

---

# 59. Facade 测试

增加：

```rust
use bevy_widgetry::check_box::{
    WidgetryCheckBox,
    WidgetryCheckBoxPlugin,
    WidgetryTriStateCheckbox,
    WidgetryCheckState,
};
```

验证消费者可以：

```text
构造普通 Checkbox
构造 TriStateCheckbox
读取 WidgetryCheckState
调用 set_state
调用 cycle_state
注册 WidgetryCheckBoxPlugin
```

无需直接依赖功能 crate。

---

# 60. 普通 Checkbox 最小 integration test

普通 Checkbox 不重复测试 Bevy 官方全部逻辑。

只验证：

```text
WidgetryCheckBoxPlugin
+
WidgetryCheckBox
+
官方 Checkbox
+
checkbox_self_update
```

组合正确。

测试目标是防止 Widgetry 接线遗漏，不是重新测试 Bevy 的 `checkbox.rs`。

---

# 61. 本次不深入的 Accessibility 事项

本次已经保留并实现：

```text
Role::CheckBox
Unchecked -> False
Checked -> True
Indeterminate -> Mixed
```

但以下问题留待后续专门讨论：

```text
Accessible Name
Label 与 Checkbox 的关联
Description
统一 Widgetry Accessibility API
不同 Widget 的语义规范
UI Automation
AI Agent 如何消费 Accessibility Tree
```

因此这次不是“不做 Accessibility”，而是：

> 先保证 Checkbox / TriStateCheckbox 的基础状态语义正确，后续再展开完整 Accessibility 设计。

---

# 62. 本次明确不做

本次不增加：

* Accessibility 的统一上层抽象
* 新 Theme accent palette
* Checkbox 专属 label type
* initial state props
* `Checked + Indeterminate` 双 Component
* `WidgetrySetCheckState` event
* `WidgetryCycleCheckState` event
* 独立 TriState plugin
* 两个常驻 mark entity
* `Node.display` 切换 mark
* runtime spawn / despawn mark
* Gallery `Tri-State States` 静态区域
* 与 Checkbox 无关的 core 重构
* 与 Checkbox 无关的 focus / tab navigation 重构

---

# 63. 最终核心结构

普通：

```text
WidgetryCheckBox
    │
    ├── Bevy Checkbox
    ├── Checked
    ├── checkbox_self_update
    ├── Bevy Accessibility
    │
    └── shared indicator/style
              │
              └── one WidgetryIcon
                    ├── Hidden when unchecked
                    └── checkbox_check.svg when checked
```

三态：

```text
WidgetryTriStateCheckbox
    │
    ├── WidgetryCheckState
    │     ├── Unchecked
    │     ├── Checked
    │     └── Indeterminate
    │
    ├── Pointer / Keyboard
    │     ↓
    │   ValueChange<WidgetryCheckState>
    │     ↓
    │   self update
    │
    ├── set_state / cycle_state
    │     ↓
    │   silent direct state mutation
    │
    ├── AccessibilityNode
    │     ├── False
    │     ├── True
    │     └── Mixed
    │
    └── shared indicator/style
              │
              └── one WidgetryIcon
                    ├── Hidden
                    ├── checkbox_check.svg
                    └── checkbox_indeterminate.svg
```

总体原则：

```text
二态尽量复用 Bevy
三态只实现 Bevy 当前缺少的能力
视觉层最大程度共享
程序化修改与用户 ValueChange 清晰分离
Accessibility 保持正确，但本次不扩展成更大的设计议题
```
