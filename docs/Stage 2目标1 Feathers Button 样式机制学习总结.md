# Stage 2 Goal 1：Feathers Button 样式机制学习总结

## 1. 核心认识

Feathers Button 并没有重新实现 Button 的交互行为，而是在官方 Headless Button 之上增加了布局、视觉样式以及样式更新逻辑。

可以概括为：

```text
Headless Button
    +
视觉 Components
    +
Style System
```

其中最重要的设计思想是：

> **不要在事件发生时到处手动修改界面，而是维护状态，再由统一的样式逻辑把状态投影成 UI。**

可以近似理解为：

```text
UI = f(state)
```

在 Bevy ECS 中：

```text
state → Components
f     → System / style resolver
UI    → visual Components
```

---

## 2. Button 的基本结构

Feathers Button 的根 Entity 上同时包含行为、状态、布局和视觉相关组件。

大致可以分成：

```text
Button Entity
├─ 行为
│  └─ Button
│
├─ 状态 / 配置
│  ├─ Hovered
│  ├─ Pressed
│  ├─ InteractionDisabled
│  └─ ButtonVariant
│
├─ 布局 / 几何
│  └─ Node
│
├─ 视觉
│  ├─ ThemeBackgroundColor
│  ├─ InheritableThemeTextColor
│  └─ FocusIndicator
│
└─ 内容层级
   └─ Children
```

`Children` 本身不是视觉样式，它表达的是 Entity 层级和内容结构。

---

## 3. 状态来源

### Hovered

`Hovered` 是一个长期存在的 Component。

需要主动挂到 Entity 上：

```rust
Hovered::default()
```

之后 Bevy Picking 根据鼠标位置修改其中的 bool。

### Pressed

`Pressed` 是 marker Component。

Headless Button 在鼠标按下时插入：

```text
insert(Pressed)
```

松开、取消等情况下移除：

```text
remove::<Pressed>()
```

### InteractionDisabled

同样适合使用“存在 / 不存在”表达状态：

```text
存在 → Disabled
不存在 → Enabled
```

因此 Style System 查询时可以使用：

```rust
Has<Pressed>
Has<InteractionDisabled>
```

---

## 4. 集中解析状态

Feathers 不让 Hover、Pressed、Disabled 分别去修改 BackgroundColor。

而是读取完整状态，然后统一计算最终样式：

```text
Hovered
Pressed
Disabled
    ↓
Style Resolver
    ↓
最终 Style
```

例如优先级：

```text
Disabled
   ↓
Pressed
   ↓
Hovered
   ↓
Normal
```

因此：

```text
Hovered = true
Pressed = true
Disabled = true
```

最终只会得到：

```text
Disabled Style
```

这样可以避免多个 System 同时修改同一个视觉 Component，导致样式互相覆盖或依赖 System 执行顺序。

---

## 5. Node 为什么通常不动态修改

Feathers Button 的一些基础布局参数直接定义在 Node 中，例如：

```text
height
padding
justify_content
align_items
border_radius
```

这些属性通常不会因为：

```text
Hover
Pressed
Disabled
```

而改变。

交互状态一般改变的是：

```text
颜色
文字颜色
Cursor
Focus Outline
```

而不是布局。

这样也能避免鼠标 Hover 或 Press 时整个 UI 因为尺寸变化而抖动。

---

## 6. ThemeBackgroundColor 与 BackgroundColor

两者并不冲突。

```text
ThemeBackgroundColor
→ 表示应该使用哪个主题颜色

UiTheme
→ 将 token 转换成实际 Color

BackgroundColor
→ 最终真正用于渲染
```

因此：

```text
ThemeBackgroundColor
        ↓
     UiTheme
        ↓
BackgroundColor
```

目前我们的 Stage 2 不需要实现 Theme，可以直接修改 `BackgroundColor`。

Theme 留到 Stage 5 再加入。

---

## 7. FocusIndicator

`FocusIndicator` 不是：

```text
这个控件能不能获得 Focus
```

而是：

```text
这个控件获得可见 Focus 后
是否使用公共 Focus 机制显示 Outline
```

因此可以理解成一个视觉 opt-in marker：

```text
FocusIndicator
+ 当前有可见 Focus
        ↓
      Outline
```

真正当前 Focus 在哪个 Entity 上，由 `InputFocus` 管理。

---

## 8. Style System 的执行时机

Style System 应该在它依赖的状态已经更新之后执行。

逻辑顺序类似：

```text
Picking
   ↓
Hovered / Pressed 更新
   ↓
Style System
   ↓
Visual Components 更新
```

否则 Style System 可能读到上一帧的状态。

---

## 9. 不必每帧处理所有 Button

最开始可以写：

```rust
Query<
    (
        &Hovered,
        Has<Pressed>,
        Has<InteractionDisabled>,
        &mut BackgroundColor,
    ),
    With<Button>,
>
```

但这会每帧遍历所有 Button。

可以通过 Query Filter 改成只处理状态发生变化的 Button：

```rust
Or<(
    Changed<Hovered>,
    Added<Pressed>,
    Added<InteractionDisabled>,
)>
```

这样优化发生在 Query 层：

```text
Filter
→ 决定哪些 Entity 需要处理

System
→ 负责真正的样式计算
```

System 内部的样式解析代码不需要因此改变。

---

## 10. Component Removal 需要单独处理

`Added<Pressed>` 能检测：

```text
Pressed 不存在
→ insert Pressed
```

但检测不到：

```text
Pressed 存在
→ remove Pressed
```

因此需要：

```rust
RemovedComponents<Pressed>
RemovedComponents<InteractionDisabled>
```

例如：

```rust
for entity in removed_pressed
    .read()
    .chain(removed_disabled.read())
{
    // 重新计算这个 Entity 的样式
}
```

`.chain()` 只是把两个 Entity iterator 首尾连接起来：

```text
Pressed 被移除的 Entity
        +
Disabled 被移除的 Entity
        ↓
统一处理
```

---

## 11. 最小实现

### Style Resolver

```rust
fn resolve_button_color(
    hovered: bool,
    pressed: bool,
    disabled: bool,
) -> Color {
    if disabled {
        Color::srgb(0.12, 0.12, 0.12)
    } else if pressed {
        Color::srgb(0.15, 0.35, 0.65)
    } else if hovered {
        Color::srgb(0.25, 0.50, 0.85)
    } else {
        Color::srgb(0.25, 0.25, 0.25)
    }
}
```

### 添加 / 变化

```rust
fn update_button_style(
    mut buttons: Query<
        (
            &Hovered,
            Has<Pressed>,
            Has<InteractionDisabled>,
            &mut BackgroundColor,
        ),
        (
            With<Button>,
            Or<(
                Changed<Hovered>,
                Added<Pressed>,
                Added<InteractionDisabled>,
            )>,
        ),
    >,
) {
    for (hovered, pressed, disabled, mut background) in &mut buttons {
        background.0 =
            resolve_button_color(hovered.get(), pressed, disabled);
    }
}
```

### 移除

```rust
fn update_button_style_remove(
    mut removed_pressed: RemovedComponents<Pressed>,
    mut removed_disabled: RemovedComponents<InteractionDisabled>,

    mut buttons: Query<
        (
            &Hovered,
            Has<Pressed>,
            Has<InteractionDisabled>,
            &mut BackgroundColor,
        ),
        With<Button>,
    >,
) {
    for entity in removed_pressed
        .read()
        .chain(removed_disabled.read())
    {
        let Ok((hovered, pressed, disabled, mut background)) =
            buttons.get_mut(entity)
        else {
            continue;
        };

        background.0 =
            resolve_button_color(hovered.get(), pressed, disabled);
    }
}
```

最终结构可以概括为：

```text
状态添加 / 状态变化 ──→ update_button_style ───┐
                                              │
                                              ↓
                                      style resolver
                                              ↑
                                              │
状态移除 ───────────→ update_button_style_remove ┘
                                              │
                                              ↓
                                      Visual Components
```

## 12. 当前得到的设计原则

Stage 2 后续实现 Styled Button 时，可以先遵循下面这个原则：

> **事件负责产生状态变化，Style System 负责观察状态，Resolver 负责把完整状态统一映射成最终 UI。**

即：

```text
Input / Event
     ↓
State Components
     ↓
Style Resolver
     ↓
Visual Components
```

现阶段先直接使用具体 Color，不引入 Theme、Design Token、复杂 Variant 等抽象。

先把这条最基础的状态到视觉的数据流做好。