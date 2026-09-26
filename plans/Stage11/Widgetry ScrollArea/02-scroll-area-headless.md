# 建立 ScrollArea headless 行为与公开滚动模型

## 目标

新增独立 bevy_widgetry_scroll_area crate，先建立 ScrollArea 的语义 type、固定 hierarchy contract、原生 ScrollPosition 数据流、wheel / keyboard / WidgetryScrollIntoView 行为，不在这一阶段完成 Auto gutter 求解和最终 visual style。

## 范围

本阶段新增 crates/scroll_area，并把它加入 workspace。crate 内优先采用 lib.rs + headless.rs 的职责组织；不因为文件长度机械拆 module。

由于新增 workspace member 属于 architecture 事实，本阶段必须同步 docs/architecture.md 的 workspace 列表和 dependency graph，使文档与当前代码状态一致。后续 style 完成后再补充最终 role 描述。

## 公开 type

### ScrollAxis

```rust
pub enum ScrollAxis {
    Horizontal,
    Vertical,
    Both,
}
```

Default = Vertical。没有 None。axis 是 construction-time structural config，不支持运行时修改。

### ScrollbarPolicy / ScrollbarVisibility

本阶段先定义公开配置 type，实际 Auto layout 在下一方案完成：

```rust
pub enum ScrollbarPolicy {
    Auto,
    Always,
    Hidden,
}

pub struct ScrollbarVisibility {
    pub horizontal: ScrollbarPolicy,
    pub vertical: ScrollbarPolicy,
}
```

两轴默认 Auto。不存在的轴对应 policy 无效。

### WidgetryScrollIntoView

公开 bubbling EntityEvent：

```rust
#[derive(Copy, Clone, Debug, PartialEq, EntityEvent)]
#[entity_event(propagate)]
pub struct WidgetryScrollIntoView {
    pub entity: Entity,
}
```

调用方对 ScrollArea descendant target 触发事件；最近的 Widgetry ScrollArea Viewport 处理并停止继续向外层 ScrollArea 传播。

### WidgetryScrollAreaViewport

Viewport marker 作为公开 API。理由不是让调用方定制内部 layout，而是给已经确认的原生程序化滚动提供稳定查询入口：

```text
Query<..., With<WidgetryScrollAreaViewport>>
→ 取得同 entity 的 ScrollPosition
```

不增加 WidgetryScrollArea::scroll_to wrapper。ScrollPosition 仍是唯一权威滚动 state。

Content marker、runtime config、内部求解 state 均不公开。

## 固定语义 hierarchy

最终 hierarchy contract 为：

```text
ScrollArea Root
├─ Viewport + WidgetryScrollAreaViewport + bevy_ui_widgets::ScrollArea + ScrollPosition
│  └─ Content + private ScrollAreaContent
├─ Vertical Scrollbar    // axis 包含 Vertical 时才存在
└─ Horizontal Scrollbar  // axis 包含 Horizontal 时才存在
```

内部 system 通过 direct child relationship 定位部件，不缓存 Entity 引用，不递归误取 nested ScrollArea。

axis 决定 scrollbar entity 是否存在；policy 只决定已经存在的 scrollbar 是否参与 layout / 显示。

## Viewport overflow

未启用轴必须 clip，而不是沿用 Bevy ScrollArea 示例中的 Visible：

```text
Vertical   → X = Clip,   Y = Scroll
Horizontal → X = Scroll, Y = Clip
Both       → X = Scroll, Y = Scroll
```

使用独立 scrollbar entity 和外部 Grid gutter，因此 Viewport 的 Node.scrollbar_width 保持 0，不使用 Bevy 自带的 scrollbar reservation。

## ScrollPosition 数据流

所有滚动入口统一写 Viewport 上的原生 ScrollPosition：

```text
wheel / trackpad
scrollbar drag / 点击 track 空白区域
keyboard
WidgetryScrollIntoView
调用方程序化修改
        ↓
Viewport ScrollPosition
```

不增加自定义 ScrollChanged event。调用方读取 ScrollPosition、直接修改 ScrollPosition、使用 Changed<ScrollPosition> 观察变化。

## wheel / trackpad

直接使用官方 Bevy ScrollArea 与 ScrollAreaPlugin：

- MouseScrollUnit::Line 使用 Bevy MouseScrollUnit::SCROLL_UNIT_CONVERSION_FACTOR。
- Pixel 直接使用原始值。
- event.x 只作用 X，event.y 只作用 Y。
- 不增加 Shift+wheel 映射。
- v1 不支持 scroll chaining；收到当前 ScrollArea 的 wheel 后沿用官方消费语义。

## keyboard

ScrollArea root 获得 focus 后处理 FocusedInput<KeyboardInput>：

```text
↑ / ↓ / ← / →
→ 对应轴移动一个 Bevy Line conversion 距离

PageUp / PageDown
→ Y 轴移动一个当前 viewport 高度

Home / End
→ Y 轴移动到起点 / 终点
```

Horizontal-only 忽略 PageUp / PageDown / Home / End；Both 中这些键保持 Y 语义。所有结果按当前最大滚动范围 clamp。

本阶段只实现 headless keyboard handler；root 的 TabIndex(-1) 在 style Scene 中挂载。

## WidgetryScrollIntoView 语义

与 Bevy 官方 ScrollIntoView 保持不同事件，不修改官方事件的语义：

- target 已完全可见：不移动。
- target 部分可见或完全不可见：在启用的轴上把 target top / left 对齐 viewport top / left。
- target 大于 viewport：同样按 top / left 对齐。
- 最终 ScrollPosition clamp 到有效范围。
- nested ScrollArea 由最近的 Widgetry Viewport 处理。

## scrollbar 官方行为边界

实际 Scrollbar / ScrollbarThumb entity 在 style Scene 中创建，但 headless contract 固定复用 Bevy 官方 Scrollbar：

- thumb drag 连续控制对应 ScrollPosition。
- 点击 track 空白区域 → 沿对应方向滚动一个 viewport 的距离。
- content 不超出 viewport 时，官方 thumb 填满 track，drag / track click 最终无实际滚动。
- 不实现 jump-thumb-to-click。

## v1 非目标

本阶段不增加：scroll chaining、内容拖拽滚动、touch pan / inertia、smooth scroll、Shift+wheel 自定义映射、disabled、独立 scroll notification、额外 accessibility 语义、运行时 axis / policy / thickness 修改。

## 自动化测试

按当前行为逐个 TDD：

- ScrollAxis 到 Viewport overflow 的映射。
- keyboard 各键与 axis 的组合、步长和 clamp。
- WidgetryScrollIntoView 的 fully visible / partially visible / invisible / oversized target 语义及 nearest ancestor 行为。
- Scene hierarchy 相关测试只验证本阶段已经存在的 headless contract，不为了测试把 private marker 改成 pub。

官方 ScrollArea wheel 和官方 Scrollbar 内部算法不原样复制测试；只在 Widgetry 的真实组合边界需要时做 integration test。

## 预期产出

得到可以被 style Scene 驱动的完整 ScrollArea headless 基础：公开滚动 type 稳定、Viewport 可从外部 query、滚动 state 唯一、keyboard 与 WidgetryScrollIntoView 已定义并有测试。

## 与前后方案的关系

前置是官方 focus 迁移已完成。下一方案只在此基础上解决 functional geometry：policy、gutter 和 Auto convergence，不改变这里已经确定的滚动语义。
