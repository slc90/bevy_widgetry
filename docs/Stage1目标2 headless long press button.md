# Bevy 0.19 Headless Button 扩展实践：Long Press 行为设计与实现学习记录

> 本文记录 Stage 1 / 目标 2 的学习过程：在 **不重写官方 Button** 的前提下，为 `bevy_ui_widgets::Button` 增加 Long Press 行为。
>
> 本阶段只关注 Headless 行为本身，不涉及 Style，也不扩展额外需求。

## 1. 目标

这一阶段要解决的问题很明确：

- 基于官方 `Button` 增加 Long Press 能力；
- 为一次长按交互维护必要的临时状态；
- 达到长按阈值时触发一个新的 Entity Event；
- 在 `Release` / `DragEnd` / `Cancel` 时取消尚未完成的长按；
- 将这些行为封装在自己的 Plugin 中。

重点不是重新实现 Button，而是学习如何用 Bevy 的 ECS 组合方式给已有 Headless Widget 增加新行为。

---

## 2. Long Press 与官方 Button 的关系

Long Press 不需要继承或复制官方 Button 的实现。

更合适的理解是：

```text
Button Entity
├─ Button                  官方：这个 Entity 是按钮
└─ LongPressButton         自己：这个按钮额外支持长按
```

`LongPressButton` 可以通过：

```rust
#[require(Button)]
```

声明它依赖官方 `Button`。

这与 Bevy 中 `MenuButton` 通过 required components 组合 `Button`、`ActivateOnPress` 的思路类似：控件能力通过 Component 组合到同一个 Entity 上，而不是通过传统继承扩展。

当前实现中：

```rust
#[derive(Component, Debug)]
#[require(Button)]
pub struct LongPressButton {
    pub press_duration: f32,
}
```

其中 `press_duration` 表示长按阈值，当前使用毫秒作为单位，默认值为 `500.0`。

---

## 3. 配置状态与运行时状态分离

Long Press 需要区分两类状态。

### `LongPressButton`

它是长期存在的配置 Component：

```text
LongPressButton
= 这个 Button 支持 Long Press
+ 长按阈值是多少
```

### `LongPressPending`

它是一次按压生命周期中的临时状态：

```rust
#[derive(Component)]
pub struct LongPressPending {
    pub timer: Timer,
}
```

它表达的是：

> 当前有一次长按候选正在等待成立。

所以一次交互大致是：

```text
按下前
Button
LongPressButton

        ↓ Pointer<Press>

按下后
Button
LongPressButton
LongPressPending
```

`LongPressPending` 中保存一个 `Timer`，用于记录这一次按压距离长按阈值还有多久。

---

## 4. 为什么 Long Press 需要普通 System

官方 Button 的很多行为都可以直接由 Observer 处理：

```text
Pointer Event
    ↓
Observer
    ↓
立即改变状态 / 触发事件
```

但 Long Press 多了一个“时间经过”的条件。

`Pointer<Press>` 只能告诉我们：

> 长按候选开始了。

它不会在 500ms 后自动再产生一个事件告诉我们长按已经成立。

因此 Long Press 需要两类机制配合：

```text
Observer
= 负责交互生命周期的开始和结束

Update System
= 负责时间推进
```

完整关系是：

```text
Pointer<Press>
      ↓
创建 LongPressPending
      ↓
Update System 每帧 tick Timer
      ↓
达到阈值
      ↓
LongPressEvent
```

---

## 5. Press：创建一次长按候选

`Pointer<Press>` 到来以后，Observer 查询目标 Entity 是否带有 `LongPressButton`。

如果有，就根据配置创建 `LongPressPending`：

```rust
commands.entity(entity).insert(LongPressPending {
    timer: Timer::from_seconds(
        long_press_button.press_duration / 1000.0,
        TimerMode::Once,
    ),
});
```

这里不需要检查官方的 `Pressed`。

原因是二者属于不同的行为层：

```text
Pointer<Press>
├─ 官方 Button 逻辑 → 管理 Pressed
└─ Long Press 逻辑  → 管理 LongPressPending
```

Long Press 已经直接收到了 `Pointer<Press>`，因此没有必要再通过 `Pressed` 二次确认交互是否开始。

同样，Long Press 扩展也不应该负责修改官方的 `Pressed`。

---

## 6. Update System：推进 Timer 并触发事件

Update System 只需要查询正在进行的长按：

```rust
Query<(Entity, &mut LongPressPending)>
```

它不需要再次查询 `LongPressButton`，因为长按阈值在 Press 发生时已经被写进本次交互的 `Timer` 中。

因此可以把它理解为：

```text
LongPressButton
= 配置

Press 时
配置 → 本次交互的运行时状态

LongPressPending
= 这次交互自己的计时状态
```

每一帧先推进 Timer：

```rust
pending.timer.tick(time.delta());
```

然后检查这一帧是否刚刚达到阈值：

```rust
if pending.timer.just_finished() {
    commands.trigger(LongPressEvent { entity });
    commands.entity(entity).remove::<LongPressPending>();
}
```

顺序必须是：

```text
先 tick
   ↓
再 just_finished()
```

不能先检查 `just_finished()` 再 `tick()`。

如果顺序反过来，Timer 即使在当前帧刚刚达到阈值，也要到下一帧才会被发现；不仅会晚一帧，还可能在两帧之间发生 `Release` 时让本应成立的 Long Press 被提前取消。

触发成功后移除 `LongPressPending`，也自然保证了一次 Press 生命周期只会触发一次 Long Press。

---

## 7. Release / DragEnd / Cancel：取消尚未完成的长按

Long Press 的提前结束路径与官方 Button 清理 `Pressed` 的思路一致。

只不过这里清理的是自己的运行时状态：

```text
Release ──┐
DragEnd ──┼─→ remove LongPressPending
Cancel  ──┘
```

Observer 可以直接只查询带有 `LongPressPending` 的 Entity：

```rust
Query<Entity, With<LongPressPending>>
```

如果目标 Entity 当前存在 Pending，就移除它：

```rust
commands.entity(entity).remove::<LongPressPending>();
```

这里同样不需要操作官方 `Pressed`。

职责边界保持为：

```text
官方 Button
→ 管理 Pressed

Long Press 扩展
→ 管理 LongPressPending
```

---

## 8. `LongPressEvent`：对外输出新的语义事件

Long Press 达到阈值以后，需要向外输出一个高层语义事件。

它与之前学习时使用的 `MiniActivate` 没有本质区别：

```rust
#[derive(EntityEvent)]
pub struct LongPressEvent {
    pub entity: Entity,
}
```

Update System 到时间后：

```rust
commands.trigger(LongPressEvent { entity });
```

外部只需要 observe 这个事件：

```rust
.observe(|long_press: On<LongPressEvent>| {
    // 响应 Long Press
});
```

这样调用者不需要知道内部用了 `Timer`、`LongPressPending` 或几个 Observer，只关心：

> 用户完成了一次长按操作。

---

## 9. `LongPressPlugin`：把整个行为封装起来

Long Press 的内部行为由自己的 Plugin 统一注册：

```rust
pub struct LongPressPlugin;

impl Plugin for LongPressPlugin {
    fn build(&self, app: &mut App) {
        app.add_observer(handle_long_press_button_on_press)
            .add_observer(handle_long_press_button_on_release)
            .add_observer(handle_long_press_button_on_drag_end)
            .add_observer(handle_long_press_button_on_cancel)
            .add_systems(Update, update_long_press);
    }
}
```

于是主程序只需要：

```rust
.add_plugins(LongPressPlugin)
```

而不需要自己注册 Long Press 内部的 system。

这和之前 `MiniButtonPlugin` 的组织方式一样，只是这次多了一个负责计时的普通 `Update` system。

---

## 10. 最终行为链

整个实现可以压缩成下面这一张图：

```text
                         Pointer<Press>
                               ↓
                    LongPressButton ?
                               ↓
                 insert LongPressPending
                         (Timer Once)
                               ↓
                     ┌─────────┴─────────┐
                     │                   │
                 Update System       提前结束
                     │                   │
             timer.tick(delta)      Release
                     │              DragEnd
             just_finished()?       Cancel
                     │                   │
                    yes                  │
                     ↓                   │
              LongPressEvent            │
                     │                   │
                     └─────────┬─────────┘
                               ↓
                 remove LongPressPending
```

---

## 11. 本阶段得到的几个关键认识

### 1. 扩展官方 Headless Widget，不等于重新实现它

Long Press 不需要复制 Button 的 Pressed、Activate 等逻辑。

只需要在同一个 Entity 上组合自己的 Component，并监听已经存在的 Pointer 事件。

### 2. 输入事件和随时间发展的行为需要不同机制

```text
Observer
= 接住瞬时事件

System
= 推进持续状态
```

Long Press 正好是二者配合的一个很小、很完整的例子。

### 3. 每层行为只管理自己的状态

```text
Button → Pressed
Long Press → LongPressPending
```

不要因为它们在同一个 Entity 上，就让扩展层顺手修改官方 Button 的内部状态。

### 4. 配置与一次交互的运行时状态可以分开

`LongPressButton.press_duration` 是长期配置；按下以后，把这个配置转成一个具体的 `Timer` 放进 `LongPressPending`。

之后 Update System 只推进本次交互，不需要反复读取配置。

---

## 12. 当前 Stage 1 / 目标 2 的完成状态

目前基础实现已经覆盖本阶段学习目标：

- [x] 基于官方 `Button` 扩展 Long Press；
- [x] 使用 required component 保证 Long Press Button 同时具有官方 Button；
- [x] Press 时建立 `LongPressPending`；
- [x] 使用一次性 `Timer` 保存长按计时状态；
- [x] 使用普通 `Update` system 推进计时；
- [x] 到达阈值时触发 `LongPressEvent`；
- [x] Long Press 成功后清理 Pending；
- [x] Release / DragEnd / Cancel 时取消 Pending；
- [x] 通过 `LongPressPlugin` 注册全部内部行为；
- [x] example 中只依赖公开组件、Plugin 和 Event 即可使用。

本阶段到这里结束即可。

下一阶段按原规划进入 **Stage 1 / 目标 3：Headless Widget 测试**。
