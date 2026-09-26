# Widgetry Tooltip 设计方案

## 1. 目标

为 `bevy_widgetry` 增加一个基于 Bevy 0.19.1 的 Tooltip 控件。

总体原则：

- 参考 Bevy 官方 Headless Tooltip 设计，但按当前项目架构实现。
- 不实现官方设计之外的扩展能力。
- 对外只暴露带 Widgetry style 的 `WidgetryTooltip`。
- headless Tooltip 完全属于 `bevy_widgetry_tooltip` crate 内部实现细节。
- 未来 Bevy 官方 Tooltip merge 后，再根据届时真实 API 决定替换内部 headless 实现，不为未来 API 提前增加兼容层。

---

# 2. 新增 crate

新增独立 crate：

```text
crates/tooltip/
├── Cargo.toml
└── src/
    ├── lib.rs
    ├── headless.rs
    └── style.rs
```

职责：

```text
headless.rs
├── Tooltip
├── TooltipPlugin
├── TooltipState
├── ShowTooltip
├── HideTooltip
└── hover / timer 状态机

style.rs
├── WidgetryTooltip
├── WidgetryTooltipProps
├── TooltipContentFactory
├── TooltipPopup
├── popup scene
├── show / hide observers
└── theme refresh
```

Workspace 增加：

```text
crates/tooltip
```

Facade 增加：

```rust
pub mod tooltip {
    pub use bevy_widgetry_tooltip::*;
}
```

---

# 3. Public API

Tooltip crate 对外仅暴露：

```rust
pub struct WidgetryTooltip;

pub struct WidgetryTooltipProps {
    pub content: TooltipContentFactory,
}

#[derive(Clone)]
pub struct TooltipContentFactory(/* private */);

pub struct WidgetryTooltipPlugin;
```

不暴露：

```rust
Tooltip
TooltipPlugin
TooltipState
ShowTooltip
HideTooltip
TooltipPopup
```

这些均使用 private 或 `pub(crate)` visibility。

---

# 4. Tooltip 内容模型

Tooltip 支持任意 BSN `SceneList` 内容，不限制为文本。

```rust
impl TooltipContentFactory {
    pub fn new<S, F>(factory: F) -> Self
    where
        S: SceneList + 'static,
        F: Fn() -> S + Send + Sync + 'static;
}
```

factory 必须使用 `Fn`，而不是 `FnOnce`，因为 Tooltip 会经历：

```text
show
→ spawn content

hide
→ despawn content

show again
→ rebuild content
```

factory 只负责调用方提供的内容：

```text
TooltipContentFactory
└── arbitrary children
```

它不负责 Tooltip popup 外壳。

---

# 5. WidgetryTooltip SceneComponent

`WidgetryTooltip` 是 anchor entity 上持久存在的 ECS identity，同时保存运行时仍需要的 content factory。

概念结构：

```rust
#[derive(SceneComponent, FromTemplate, Clone)]
#[scene(WidgetryTooltipProps)]
pub struct WidgetryTooltip {
    content: TooltipContentFactory,
}
```

Scene 展开后，同一个 anchor entity 上存在：

```text
Anchor
├── WidgetryTooltip { content }
└── Tooltip               // internal marker
```

初始 Scene **不生成 Tooltip popup**。

Popup 只在真正显示 Tooltip 时动态创建。

## Props Default

Bevy 0.19.1 的 `SceneComponent` 要求：

```rust
type Props: Default;
```

但 Tooltip content 是必填项，因此采用和当前 ComboBox 相同的原则：

```text
Default
→ 仅满足 SceneComponent 构造机制
→ 可以暂时处于无效状态
→ scene(props) 立即验证
→ 缺少必填 content 时 ERROR + panic
```

`TooltipContentFactory` 内部使用 private missing sentinel 表达该状态，不公开一个具有误导语义的 `Default` factory。

---

# 6. 内部 Headless Tooltip

内部类型大致为：

```rust
#[derive(Component, Default, Clone)]
pub(crate) struct Tooltip;

pub(crate) struct TooltipPlugin;

#[derive(Resource)]
pub(crate) struct TooltipState {
    // 当前 candidate / visible anchor
    // show timer
    // cooldown state/timer
}

#[derive(EntityEvent)]
pub(crate) struct ShowTooltip {
    pub source: Entity,
}

#[derive(EntityEvent)]
pub(crate) struct HideTooltip {
    pub source: Entity,
}
```

`source` 始终表示 Tooltip anchor，而不是 popup entity。

事件中不携带：

- popup entity；
- content factory；
- position；
- style。

---

# 7. Hover 与 ancestor lookup

运行时按照 Bevy 官方 Tooltip 设计：

```text
HoverMap
   ↓
当前 hovered entity
   ↓
沿 ancestor 向上查找
   ↓
第一个拥有 Tooltip marker 的 entity
   ↓
resolved tooltip anchor
```

因此 Tooltip 可以挂在复合 Widget root 上，而实际 hover 的可能是内部 descendant。

Tooltip 完全无视：

```rust
InteractionDisabled
```

disabled Button / ComboBox / TextField 等仍然可以正常显示 Tooltip。

---

# 8. Tooltip 状态与 timing

内部固定时长：

```text
cold warmup     = 200ms
warm delay      = 50ms
cooldown        = 300ms
```

这些都是内部常量，不作为 public configurable API。

## 首次 Tooltip

```text
pointer 停留
→ 200ms
→ ShowTooltip
```

pointer 在显示前移动：

```text
移动
→ warmup timer reset
```

---

## Tooltip 已经显示

Tooltip 显示后：

```text
pointer movement
→ 重新 resolve tooltip anchor
```

### resolved anchor 没变化

```text
A → A
```

Tooltip 保持显示。

例如：

- 在大型控件内部移动；
- 从 anchor 本身移动到其 descendant；
- descendant 之间移动。

不能因为单纯 pointer movement 就 hide。

### 切换到另一个 Tooltip

```text
A → B
```

立即：

```text
HideTooltip(A)
```

然后 B 在 warm mode 下使用：

```text
50ms
```

后显示。

### 离开所有 Tooltip

```text
A → none
```

立即：

```text
HideTooltip(A)
```

并进入 cooldown。

---

# 9. Warm / cooldown

cooldown 从 **Hide** 开始，而不是 Show。

因此：

```text
Show A
→ A 可以无限保持显示

Hide A
→ cooldown 300ms 开始
```

如果新的 Tooltip candidate 在 cooldown 期间出现，则采用：

```text
50ms warm delay
```

如果已经超过 cooldown 后才出现新的 candidate：

```text
200ms cold delay
```

同一时间只允许存在一个 Tooltip。

---

# 10. System / Observer 组织

遵循项目当前习惯：

```text
持续状态 / timer / HoverMap
→ system

离散事件 / lifecycle
→ observer
```

## TooltipPlugin

核心状态机保持为一个 system，不人为拆成多个互相排序的 system：

```rust
app.init_resource::<TooltipState>()
    .add_observer(tooltip_removed)
    .add_systems(
        PreUpdate,
        update_tooltip.in_set(PickingSystems::PostHover),
    );
```

使用 `PickingSystems::PostHover`，保证本帧：

```text
Pointer input
→ picking
→ HoverMap 更新
→ Tooltip state machine
```

`update_tooltip` 统一完成：

- pointer movement 判断；
- HoverMap lookup；
- ancestor resolution；
- show timer；
- cooldown；
- ShowTooltip；
- HideTooltip。

## Lifecycle observer

例如：

```rust
tooltip_removed
```

在当前 candidate / visible anchor 的 Tooltip component 被移除或实体销毁时清理状态和视觉 popup。

---

# 11. WidgetryTooltipPlugin

对外只需要注册：

```rust
WidgetryTooltipPlugin
```

它内部确保注册：

```text
TooltipPlugin
PopoverPlugin
ThemePlugin
```

概念：

```rust
impl Plugin for WidgetryTooltipPlugin {
    fn build(&self, app: &mut App) {
        if !app.is_plugin_added::<TooltipPlugin>() {
            app.add_plugins(TooltipPlugin);
        }

        if !app.is_plugin_added::<PopoverPlugin>() {
            app.add_plugins(PopoverPlugin);
        }

        if !app.is_plugin_added::<ThemePlugin>() {
            app.add_plugins(ThemePlugin);
        }

        app.add_observer(show_tooltip)
            .add_observer(hide_tooltip)
            .add_observer(refresh_tooltip_theme);
    }
}
```

---

# 12. Popup 生命周期

Tooltip popup 不作为隐藏 entity 常驻。

Show 时：

```text
ShowTooltip(anchor)
→ 获取 anchor 上的 WidgetryTooltip
→ content.build()
→ spawn TooltipPopup as direct child of anchor
```

Hide 时：

```text
HideTooltip(anchor)
→ 找到该 anchor 的 TooltipPopup
→ recursive despawn
```

因此：

```text
hidden
≠ Visibility::Hidden

hidden
= entity 不存在
```

Popup entity ID 属于 styled layer 的实现细节，不进入 headless state/event API。

---

# 13. Tooltip popup hierarchy

显示时结构：

```text
Anchor
├── WidgetryTooltip
├── Tooltip
└── TooltipPopup
    ├── Popover
    ├── OverrideClip
    ├── GlobalZIndex(z_index::TOOLTIP)
    ├── Pickable::IGNORE
    └── arbitrary children from content.build()
```

Popup 必须是 anchor 的 direct child，因为 Bevy `Popover` 使用 parent 作为定位 anchor。

不额外保存：

```text
anchor: Entity
```

到 Popup component 中。

---

# 14. Tooltip style

Tooltip popup 使用现有 popup theme token，不新增 Tooltip 专用颜色。

```text
background = theme.popup_background
border     = theme.popup_border
foreground = theme.foreground
```

布局：

```rust
Node {
    padding: UiRect::axes(px(8), px(6)),
    border: UiRect::all(px(1)),
    border_radius: BorderRadius::all(px(4)),
    ..
}
```

确定：

```text
border       = 1px
border radius = 4px
padding-x     = 8px
padding-y     = 6px
```

不添加：

- shadow；
- 固定 width；
- 固定 height；
- max-width；
- Tooltip arrow。

Tooltip 内容自己的布局完全由调用方的 `SceneList` 决定。

ThemeChanged 时同步刷新：

- background；
- border；
- foreground。

---

# 15. Popover placement

Tooltip 自己不实现任何窗口边缘判断。

全部交给 Bevy 官方：

```rust
Popover
```

候选顺序：

```text
1. Bottom / Center
2. Top / Center
3. Right / Center
4. Left / Center
```

统一：

```text
gap           = 6px
window_margin = 8px
```

Popover 负责：

- 优先使用第一个完全放得下的位置；
- 空间不足时自动切换方向；
- 全部候选都无法完整显示时选择裁剪最少的位置；
- anchor 移动时动态重新计算。

---

# 16. Global Z-index 重构

新增：

```text
crates/core/src/z_index.rs
```

统一定义全库 overlay layer token：

```rust
pub const LOCAL_OVERLAY: i32 = 10;
pub const POPUP: i32 = 100;
pub const TOOLTIP: i32 = 200;
pub const MODAL: i32 = 100_000;
```

语义层级：

```text
LOCAL_OVERLAY   10
POPUP          100
TOOLTIP        200
MODAL       100_000
```

不使用具体 Widget 名称命名这些 token。

Facade 的 `style` module 导出：

```rust
pub use bevy_widgetry_core::z_index;
```

Gallery 和 Widget 都统一使用 token，不再直接写裸数字。

---

# 17. Modal 与 Tooltip

当前 `WidgetryModalWindow` 使用独立 native Window + UI camera。

因此：

```text
Parent Window
├── ComboBox popup = 100
├── Tooltip        = 200
└── Modal blocker  = 100_000

Modal Window
├── ComboBox popup = 100
└── Tooltip        = 200
```

Parent modal blocker 不会遮住 modal native window 内自己的 Tooltip。

同时 parent window 原有 Tooltip 会被 blocker 压住，这是预期行为。

---

# 18. ComboBox Popup 顺带重构

当前 ComboBox popup 手写：

```text
absolute
top: 100%
```

改成 Bevy `Popover`。

ComboBox 只提供两个候选方向：

```text
1. Bottom / Start
2. Top / Start
```

参数：

```text
gap           = 0
window_margin = 8px
```

保留：

```text
width: 100%
```

因此 Popup：

- 正常显示在 ComboBox 下方；
- 底部空间不足时自动翻到上方；
- 始终与 ComboBox 左边缘对齐；
- 不会跑到左右两侧。

`WidgetryComboBoxPlugin` 自己确保注册：

```rust
PopoverPlugin
```

不能依赖 Tooltip plugin。

ComboBox z-index 改为：

```rust
GlobalZIndex(z_index::POPUP)
```

---

# 19. Gallery

新增：

```text
gallery/src/pages/tooltip.rs
```

Navigation 增加：

```text
Tooltip
```

`main.rs` 注册：

```rust
WidgetryTooltipPlugin
```

Tooltip page 展示四组内容。

## Basic

普通文本：

```text
[ Hover me ]
```

验证基本 Tooltip。

## Rich Content

Tooltip 内使用：

- icon；
- 多段 Text；
- 自定义 layout。

验证：

```text
TooltipContentFactory
→ arbitrary SceneList
```

以及 foreground/theme 传播。

## Disabled

```text
[ Disabled ]
```

Button 带：

```rust
InteractionDisabled
```

但 Tooltip 仍然正常显示。

明确展示 Tooltip 无视 disabled。

## Placement

保留三种：

```text
Placement

        [ Center ]

                         [ Right ]

        [ Bottom ]
```

用于肉眼观察：

### Center

正常空间下默认：

```text
Bottom
```

### Right

靠近页面/window 右侧，观察 Popover 自动避免右边界裁剪。

### Bottom

靠近 window 底部，验证：

```text
Bottom 放不下
→ Top
```

不在 Gallery 强行制造 Top / Left window-edge 场景。

完整方向行为由测试覆盖。

Gallery 当前：

```rust
GlobalZIndex(10)
```

同步替换成：

```rust
GlobalZIndex(z_index::LOCAL_OVERLAY)
```

---

# 20. 测试范围

## Tooltip crate

至少覆盖：

### Scene / API

- valid content factory 正常构造；
- arbitrary SceneList；
- 缺少 content 时 ERROR + panic；
- WidgetryTooltip 和 internal Tooltip marker 位于同一 anchor。

### Hover resolution

- 直接 hover anchor；
- hover descendant；
- ancestor Tooltip 正常解析；
- 无 Tooltip ancestor 不显示。

### Timing

- cold 首次等待 200ms；
- pointer movement 重置 show timer；
- Show 后 pointer 在同 anchor 内移动不 Hide；
- A → B 立即 Hide A；
- warm B 使用 50ms；
- Hide 后 300ms cooldown；
- cooldown 结束后恢复 200ms。

### Lifecycle

- 同时只存在一个 Tooltip；
- Hide 后 popup recursive despawn；
- anchor despawn；
- Tooltip component remove；
- repeated show/hide/show 可以重复调用 factory。

### Disabled

- `InteractionDisabled` 不阻止 Tooltip。

### Styling

- popup background/border；
- radius / padding；
- `Pickable::IGNORE`；
- `OverrideClip`；
- `z_index::TOOLTIP`；
- ThemeChanged refresh。

### Placement

对 Popover 配置至少检查：

```text
Bottom
Top
Right
Left
```

候选顺序、alignment、gap 和 window margin。

不需要重新测试 Bevy Popover 自身完整几何算法。

---

# 21. ComboBox 测试调整

现有测试中类似：

```rust
node.position_type == Absolute
node.top == percent(100)
GlobalZIndex == 100
```

需要更新。

改为验证：

```text
Popover component 存在
positions == [
    Bottom / Start / gap 0,
    Top / Start / gap 0,
]
window_margin == 8
width == 100%
GlobalZIndex == z_index::POPUP
```

其余 ComboBox behavior 保持不变。

---

# 22. Facade public API tests

`crates/bevy_widgetry/tests/public_api.rs` 增加 Tooltip public surface：

```text
WidgetryTooltip
WidgetryTooltipProps
TooltipContentFactory
WidgetryTooltipPlugin
```

验证消费者仅通过：

```rust
bevy_widgetry::tooltip
```

即可构造 Tooltip。

同时验证：

```rust
bevy_widgetry::style::z_index
```

可用。

不得从 facade 暴露：

```text
Tooltip
TooltipPlugin
TooltipState
ShowTooltip
HideTooltip
TooltipPopup
```

---

# 23. 明确不实现的内容

本阶段不增加：

- keyboard focus Tooltip；
- touch / long press；
- interactive Tooltip；
- nested Tooltip；
- InfoTip；
- manual public show/hide；
- public ShowTooltip / HideTooltip；
- per-Tooltip delay；
- public timing configuration；
- show/hide animation；
- Tooltip arrow；
- text-only special API；
- accessibility-derived automatic Tooltip；
- Tooltip 专属 theme color；
- shadow；
- max-width；
- 自定义定位算法。

这些能力未来如有需求单独设计，不作为当前 Tooltip 的隐式扩展。

---

# 24. 最终架构

```text
Consumer
   │
   ▼
@WidgetryTooltip
   │
   ├── WidgetryTooltip { content }
   └── internal Tooltip
            │
            ▼
     TooltipPlugin
     HoverMap + timers
            │
       Show / Hide
            │
            ▼
   Widgetry styled layer
            │
            ▼
      TooltipPopup
      ├── Popover
      ├── z_index::TOOLTIP
      ├── popup theme
      └── content.build()
```

对外只有 styled Widget。

Headless layer 是当前 Bevy 0.19.1 下的内部实现，未来升级 Bevy 时可以根据官方最终 Tooltip API 独立替换，而不影响 `WidgetryTooltip` 的消费者接口。
