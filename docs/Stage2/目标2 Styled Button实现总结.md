# Stage 2：Styled Button 开发总结

## 目标

Stage 2 的目标是基于 Bevy 官方的 Headless `Button`，实现一个最小但完整的 `StyledButton`。

当前阶段只关注控件自身的基础视觉表现，不引入 Theme、Design Tokens、Variant 等更高层抽象。

---

## 当前设计

`StyledButton` 不重新实现 Button 行为，而是直接复用 Bevy 官方的 Headless `Button`。

整体结构可以理解为：

```text
StyledButton
├─ Button
├─ Hovered
├─ Node
├─ BackgroundColor
├─ BorderColor
└─ Propagate<ForegroundColor>
```

其中：

- `Button`：负责官方的按钮交互行为。
- `Hovered`：由 Bevy Picking 更新，用于表示 hover 状态。
- `Node`：负责按钮静态布局，例如 padding、border。
- `BackgroundColor`：由按钮状态动态决定。
- `BorderColor`：目前作为静态视觉属性。
- `Propagate<ForegroundColor>`：向按钮的子节点传播前景色。

`Pressed` 和 `InteractionDisabled` 不作为 required component，因为它们属于“存在即表示状态”的动态组件。

---

## 当前支持的视觉状态

Stage 2 最终保留四种视觉状态：

```text
Default
Hover
Pressed
Disabled
```

Focus 相关功能已经从本阶段移除。

移除内容包括：

```text
TabIndex
InputFocus
InputFocusVisible
Focus -> Outline resolver
Outline
```

原因是当前 `StyledButton` 不考虑键盘导航，而鼠标点击场景下 focus ring 对这个阶段没有实际价值。

---

## BackgroundColor 状态解析

按钮背景色由一个独立 resolver 决定：

```rust
fn resolve_button_background(
    hovered: bool,
    pressed: bool,
    disabled: bool,
) -> Color {
    if disabled {
        Color::srgb(0.15, 0.15, 0.15)
    } else if pressed {
        Color::srgb(0.85, 0.12, 0.12)
    } else if hovered {
        Color::srgb(0.20, 0.65, 0.95)
    } else {
        Color::srgb(0.30, 0.30, 0.30)
    }
}
```

当前优先级为：

```text
Disabled
  >
Pressed
  >
Hovered
  >
Default
```

当前颜色故意拉大差异，主要用于开发阶段快速确认状态切换是否正确：

```text
Default   -> 灰色
Hover     -> 亮蓝色
Pressed   -> 红色
Disabled  -> 深灰色
```

---

## 增量样式更新

背景色更新不是每帧扫描全部按钮，而是根据 ECS 的变化检测机制进行增量更新。

使用：

```text
Changed<Hovered>
Added<Pressed>
Added<InteractionDisabled>
RemovedComponents<Pressed>
RemovedComponents<InteractionDisabled>
```

核心思路是：

```text
Added / Changed / RemovedComponents
        ↓
决定“什么时候、哪个 Entity 需要重新计算”
        ↓
读取按钮当前完整状态
        ↓
resolve_button_background(...)
        ↓
更新 BackgroundColor
```

`RemovedComponents<T>` 在这里主要用于找出“哪个 Entity 刚刚失去了某个状态”，最终样式仍然由当前完整状态重新解析，而不是根据事件直接猜测结果。

---

## ForegroundColor

实现了一个通用前景色组件：

```rust
#[derive(Component, Clone, Copy, Debug, PartialEq)]
pub struct ForegroundColor(pub Color);
```

默认值目前为黑色。

按钮根节点通过：

```rust
Propagate<ForegroundColor>
```

向子节点传播前景色。

传播使用 Bevy 官方：

```text
HierarchyPropagatePlugin<ForegroundColor>
```

而不是自己实现继承系统。

---

## ForegroundColor -> TextColor

Text 自己使用的是 `TextColor`，因此增加了一个 adapter：

```text
ForegroundColor
        ↓
apply_foreground_color_to_text
        ↓
TextColor
```

adapter 会处理同时具有：

```text
ForegroundColor + TextColor
```

的 Entity，因此既适用于普通 `Text`，也自然适用于需要 `TextColor` 的文字节点。

传播流程放在：

```text
PostUpdate
└─ UiSystems::Propagate
   ├─ PropagateSet<ForegroundColor>
   └─ apply_foreground_color_to_text
```

其中 adapter 明确：

```rust
.after(PropagateSet::<ForegroundColor>::default())
```

确保先完成 `ForegroundColor` 的层级传播，再同步到 `TextColor`。

已经通过黑色和白色前景色实际验证，这条传播链可以正常工作。

---

## SystemSet 相关理解

本阶段顺便梳理了 Bevy 的 `SystemSet`：

```text
Schedule
= Update / PostUpdate 等大的执行阶段

SystemSet
= Schedule 内部用于组织和排序 system 的逻辑分组
```

一个 SystemSet 也可以放入另一个 SystemSet：

```rust
PropagateSet::<ForegroundColor>::default()
    .in_set(UiSystems::Propagate)
```

可以理解为：

```text
UiSystems::Propagate
└─ PropagateSet<ForegroundColor>
   └─ ForegroundColor propagation systems
```

`.after(...)` 同样可以针对整个 SystemSet，而不一定只能指定单个 system。

---

## Example 验证

当前 example 中同时放置：

```text
普通 StyledButton
Disabled StyledButton
```

普通按钮用于实际测试：

```text
Default
Hover
Pressed
```

Disabled 按钮通过：

```rust
InteractionDisabled
```

持续展示 Disabled 状态。

目前四种状态均已经实际跑通。

---

## 当前 Stage 2 状态

目前已完成：

- 基于官方 Headless `Button` 构建 `StyledButton`
- 默认静态布局和视觉组件
- Hover 状态
- Pressed 状态
- Disabled 状态
- 状态背景色 resolver
- 增量状态更新
- `ForegroundColor`
- 官方 hierarchy propagation
- `ForegroundColor -> TextColor` adapter
- example 实际验证
- 移除当前阶段没有价值的 Focus 相关设计

因此，Stage 2 的 Styled Button 主体功能可以认为已经完成。

---

## 下一步

下一步进入 Stage 2 的剩余工作：

```text
Goal 3：Style Tests
```

重点将放在验证：

- 各状态是否解析到正确背景色
- 状态优先级是否正确
- 状态添加和移除后是否能重新计算
- ForegroundColor 是否能正确传播并同步到 TextColor

暂时不继续扩充新的 StyledButton 功能。
