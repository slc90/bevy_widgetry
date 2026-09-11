# Bevy 0.19 Headless Button：从输入事件到语义激活的完整学习记录

> 本文总结本阶段对 Bevy 0.19.1 官方 Headless Button 的学习与验证过程，重点是理解控件行为链，而不是复刻完整实现。

## 1. Headless Button 的核心思想

Headless Button 的重点不是“长得像按钮”，而是“具备按钮的行为与语义”。

在 Bevy 里，官方 `Button` 本身只是一个 marker component：

```rust
pub struct Button;
```

它不保存颜色、尺寸、边框，也不直接保存 `pressed: bool` 之类的内部状态。

因此可以把官方 Button 拆成几部分来看：

```text
Button
= 控件身份

Pressed
= Pointer 交互中的临时按压状态

InteractionDisabled
= 是否拒绝用户交互

ActivateOnPress
= 修改激活时机

ButtonPlugin
= 注册控件行为规则

Activate
= 对外输出的高层“激活”语义事件
```

这体现了 Bevy 很典型的 ECS 组合式设计：身份、状态、行为修饰和输出语义都通过不同的 Component / Event / Plugin 组合完成。

---

## 2. 从底层输入到 Button 的事件层级

Button 的 observer 并不负责处理最底层的鼠标坐标、触摸输入或 hit test。

更完整的输入链大致是：

```text
操作系统 / winit 输入
        ↓
鼠标 / 触摸等输入
        ↓
PointerInput
        ↓
Picking backend 做命中检测
        ↓
HoverMap / PointerState
        ↓
pointer_events
        ↓
Pointer<Press>
Pointer<Click>
Pointer<Release>
Pointer<DragEnd>
Pointer<Cancel>
...
        ↓
Button observer
        ↓
Pressed / Activate
```

其中有一个很重要的层次区别：

```text
PointerInput
= “这个 pointer 自己发生了什么”

Pointer<Press>
= “这个 pointer 对某个 Entity 做了什么”

Button observer
= “如果这个 Entity 是 Button，该怎么响应”
```

所以在自定义 UI 控件时，通常应该从 `Pointer<Press>`、`Pointer<Click>` 这层开始，而不是自己重新处理鼠标坐标和命中检测。

---

## 3. Pointer 不是 Mouse 的同义词

`Pointer` 是 Bevy Picking 提供的统一输入抽象。

它可以表示：

```text
Mouse
Touch
Custom Pointer
```

因此：

```rust
On<Pointer<Click>>
```

并不等于“只监听鼠标点击”。

可以把抽象层次记成：

```text
Mouse / Touch / Custom
        ↓
      Pointer
        ↓
Press / Click / Drag / ...
        ↓
      Button
        ↓
     Activate
```

`Activate` 再进一步把“具体怎么操作的”抽象掉，让业务层只关心：

> 用户想执行这个控件的动作。

---

## 4. ButtonPlugin 的角色

官方 `ButtonPlugin` 的核心工作非常简单：

```rust
app.add_observer(button_on_key_event)
    .add_observer(button_on_pointer_down)
    .add_observer(button_on_pointer_up)
    .add_observer(button_on_pointer_click)
    .add_observer(button_on_pointer_drag_end)
    .add_observer(button_on_pointer_cancel);
```

也就是说：

```text
ButtonPlugin
= 把 Button 的通用行为规则注册进 App
```

真正的行为都在这些 observer 中实现。

---

## 5. Pointer Press：进入 Pressed 状态

Pointer 按下时，官方 Button 会检查目标 Entity 是否是 Button，以及当前状态：

```text
Pointer<Press>
    ↓
目标是 Button？
    ↓
是否 disabled？
    ↓
是否已经 Pressed？
    ↓
insert Pressed
```

`Pressed` 不是 `Button` 结构体里的字段，而是独立 Component。

这意味着：

```text
Entity
├─ Button
└─ Pressed
```

表示这个 Button 当前正处于 Pointer 按压交互中。

如果同时带有 `ActivateOnPress`：

```text
Press
  ↓
insert Pressed
  ↓
立即 Activate
```

普通 Button 则不会在 Press 阶段立即激活。

---

## 6. Pointer Click：产生 Activate

普通 Button 的主要激活链是：

```text
Press
  ↓
Pressed
  ↓
Click
  ↓
Activate
```

Click observer 会检查：

```text
Pressed == true
InteractionDisabled == false
ActivateOnPress == false
```

满足时才：

```rust
commands.trigger(Activate { ... });
```

这里对 `Pressed` 的检查更适合理解成：

> 一个状态守卫 / 状态一致性保护。

正常 Picking 流程里，Click 到来时 Button 通常本来就应该处于 `Pressed` 状态。

---

## 7. Click 与 Release 的真实顺序

这一部分深入到了 Bevy Picking 的源码。

一个容易产生误解的地方是：

> 物理上“松开鼠标”这个输入动作决定了 Click 是否成立，但 Bevy 在这一帧里实际会先生成 Click，再生成 Release。

即：

```text
PointerAction::Release
        ↓
先 Trigger Click
        ↓
再 Trigger Release
```

因此 Button 可以在 Click observer 中看到：

```text
Pressed == true
```

然后在 Release observer 中再清掉它。

正常顺序可以记成：

```text
Press
  ↓
Pressed
  ↓
Click
  ↓
Activate
  ↓
Release
  ↓
remove Pressed
```

---

## 8. Commands 的 deferred 行为

`commands.trigger(...)` 并不是立刻执行 observer。

它先进入 CommandQueue：

```text
commands.trigger()
    ≠ 立即运行 Observer

它先进入 CommandQueue
    ↓
到同步点 / ApplyDeferred 时执行
```

但还有第二层需要注意：

当某个 Trigger Command 真正执行时，它触发的 observer 里面还可能继续产生新的 Commands，例如：

```text
Trigger Click
    ↓
button_on_pointer_click
    ↓
commands.trigger(Activate)
```

这些 observer 内部新产生的 Commands 会在当前 command 执行后的 `world.flush()` 中继续被处理。

因此可以把 release 那一帧理解成：

```text
pointer_events
    ↓
queue Trigger Click
queue Trigger Release
    ↓
ApplyDeferred
    ↓
Trigger Click
    ↓
Button Click observer
    ↓
queue Activate
    ↓
递归 flush
    ↓
Activate observers
    ↓
Trigger Release
    ↓
Button Release observer
    ↓
queue remove Pressed
    ↓
递归 flush
    ↓
Pressed removed
```

这解释了为什么 deferred Commands 并不会破坏 Button 所依赖的状态顺序。

---

## 9. Release / DragEnd / Cancel 都负责清理 Pressed

除了正常 Release，官方 Button 还处理：

```text
Release
DragEnd
Cancel
```

三者的共同目的都是：

```text
如果 Button 还处于 Pressed
    ↓
remove Pressed
```

这是为了避免交互没有走“普通点击完成”路径时，Button 永远卡在 Pressed 状态。

因此 `Pressed` 更适合理解为：

> 一段 Pointer 按压交互生命周期中的临时状态。

而不是简单的“鼠标左键当前是不是物理按下”。

---

## 10. DragEnd 的传播验证

在 MiniButton 验证过程中发现：

如果使用全局 observer：

```rust
app.add_observer(handle_drag_end);
```

而又不调用：

```rust
event.propagate(false);
```

那么同一个 `Pointer<DragEnd>` 可能让这个全局 observer 执行多次。

原因不是 Picking 生成了多个 DragEnd，而是 Pointer 事件默认会沿 Entity 层级传播。

例如：

```text
MiniButton
    ↓
Window
```

于是：

```text
事件只产生 1 次

observer 可能运行多次
= 因为事件传播到了不同 Entity
```

这也验证了：

```text
事件产生次数
≠
observer 执行次数
```

---

## 11. propagate(false) 的真正作用

`event.propagate(false)` 的含义是：

> 当前 Entity 上的 observer 都执行完后，不再继续沿传播路径进入下一个 Entity。

它不是：

```text
只允许当前一个 observer 执行
```

同一个 Entity 上的其他匹配 observer 依然可以运行。

另外，如果是全局 observer，`propagate(false)` 的位置非常重要。

错误思路：

```rust
event.propagate(false);

if let Ok(...) = query.get(event.entity) {
    ...
}
```

因为这样会导致：

```text
任何 Entity 的 Pointer 事件
    ↓
先被全局 observer 截断传播
    ↓
然后才检查是不是 MiniButton
```

正确思路应该是：

```text
先确认当前 Entity 是自己负责的控件
    ↓
再决定是否停止传播
```

---

## 12. 键盘激活链

官方 Button 的键盘入口是：

```rust
On<FocusedInput<KeyboardInput>>
```

键盘没有鼠标坐标，因此它需要 Focus 来决定输入目标：

```text
KeyboardInput
    ↓
InputFocus
    ↓
FocusedInput<KeyboardInput>
    ↓
当前 focused Entity
```

Button 会检查：

```text
当前 focused entity 是 Button
并且不是 InteractionDisabled
并且：
repeat == false
state == Pressed
key == Enter 或 Space
```

满足时直接：

```text
Activate
```

所以键盘链是：

```text
Focus 在 Button 上
    ↓
Enter / Space Pressed
    ↓
Activate
```

它没有使用 `Pressed` Component。

因此官方 Button 的 `Pressed` 主要服务于 Pointer 交互状态，而键盘激活直接产生语义事件。

---

## 13. 为什么 Activate 很重要

Pointer 和 Keyboard 最终汇合成：

```text
Pointer
   │
   ├─ Press
   ├─ Click
   └─ ...
        │
        ▼
     Activate
        ▲
        │
Enter / Space
        │
    Keyboard
```

于是业务层只需要：

```rust
.observe(|activate: On<Activate>| {
    ...
})
```

而不必关心：

```text
鼠标点击？
触摸？
键盘 Enter？
键盘 Space？
```

这就是高层语义事件的价值。

---

## 14. InteractionDisabled 的设计

`InteractionDisabled` 同样只是一个 marker component：

```text
正常 Button：

Entity
└─ Button

Disabled Button：

Entity
├─ Button
└─ InteractionDisabled
```

它本身不会自动：

```text
变灰
停止渲染
失去 Focus
停止被 Picking 命中
```

真正让 Button 不响应的是 Button 的 observer 主动检查：

```text
Has<InteractionDisabled>
```

然后拒绝：

```text
insert Pressed
trigger Activate
```

因此：

```text
InteractionDisabled
= 行为状态

变灰、透明度、颜色变化
= Style 层自己决定
```

这和 Headless 思想完全一致。

---

## 15. app.add_observer 与 entity.observe

这一阶段还专门验证了两种 observer 注册方式。

### app.add_observer

```rust
app.add_observer(...)
```

更适合：

```text
注册全局规则
```

例如官方 ButtonPlugin：

```text
任何 Pointer<Press>
    ↓
observer 都可能运行
    ↓
再通过 Query<..., With<Button>>
筛选是不是 Button
```

### entity.observe

```rust
commands
    .spawn(...)
    .observe(...)
```

更像：

```text
这个具体 Entity 自己的事件回调
```

例如：

```text
这个 Button 被 MiniActivate
    ↓
执行这个按钮自己的业务逻辑
```

两者底层都属于 Observer 系统，只是监听范围不同。

---

## 16. 两种 observer 可以同时触发

通过实际实验验证：

```text
app.add_observer(...)
+
entity.observe(...)
```

如果两者都匹配同一个事件，那么都会执行。

同理，如果两套规则各自都：

```rust
commands.trigger(Activate { ... });
```

那么就真的会产生两个独立的 `Activate`。

因此：

```text
一个控件最好只有一套“什么时候算激活”的语义规则
```

业务层应该消费 `Activate`，而不是再从 Press / Click 重新制造一次相同的激活语义。

---

## 17. MiniButton 练习的目标

为了确认不是“看懂源码但自己串不起来”，实现了一个最小 `MiniButton`。

目标只关注：

```text
输入
↓
状态
↓
语义事件
↓
状态清理
```

最终链路：

```text
Pointer<Press>
    ↓
insert Pressed

Pointer<Click>
    ↓
确认 Pressed
    ↓
trigger MiniActivate

Pointer<Release>
Pointer<DragEnd>
Pointer<Cancel>
    ↓
remove Pressed
```

业务层只监听：

```text
MiniActivate
```

而不关心底层 Pointer 事件。

这完成了：

```text
低层输入
    ↓
控件状态机
    ↓
高层语义事件
    ↓
业务逻辑
```

的完整闭环。

---

## 18. MiniButton 中验证到的几个额外细节

在实现 MiniButton 时还实际踩到了几个有价值的问题。

### Pointer 类型导错

`Pointer` 这个名字可能被 IDE 自动导成：

```rust
core::fmt::Pointer
```

而我们需要的是 Bevy Picking 的：

```rust
bevy::picking::...::Pointer
```

因此遇到：

```text
expected a type, found a trait
```

时，应优先检查导入路径。

### Query 不一定需要 mut

如果 Query 只是：

```rust
Query<(Entity, Has<Pressed>), With<MiniButton>>
```

并没有 `&mut T`，那么本身不需要 `mut Query`，也不一定需要 `get_mut()`。

这属于代码权限可以进一步收紧的地方，但不影响本阶段行为验证。

### 测试代码可以保持“不优雅”

MiniButton 的目的不是生产级实现，而是学习。

因此临时：

```text
info!
test_cancel system
手工触发 Pointer<Cancel>
```

这些都可以保留，只要它们能帮助观察事件链和状态变化。

---

## 19. Cancel 的一个特殊发现

实际尝试直接注入：

```text
PointerInput::Cancel
```

时发现，在 Bevy 0.19.1 默认 Picking 流程中：

```text
generate_hovermap
    ↓
看到 PointerAction::Cancel
    ↓
这个 pointer 的 hit 被过滤
    ↓
HoverMap 中可能已经没有目标
    ↓
pointer_events 处理 Cancel 时
又依赖当前 HoverMap 来决定给谁发 Pointer<Cancel>
```

因此这种人工注入方式并不能稳定地产生我们想验证的 `Pointer<Cancel>` observer。

最终采用了更直接的验证方式：

```text
人工构造 Pointer<Cancel>
    ↓
直接 target MiniButton
    ↓
验证 MiniButton 的 Cancel observer
    ↓
确认 Pressed 被清理
```

这样把测试范围限制在 MiniButton 自己负责的行为上，而不是继续深入 Picking 的 Cancel 生成细节。

---

## 20. 本阶段最终形成的控件设计模型

经过官方 Button 源码阅读和 MiniButton 实践，可以把一个 Headless 控件理解成四层：

```text
输入层
Pointer / Keyboard / Focus
        ↓
行为层
Observer / Plugin
        ↓
状态层
Pressed / Disabled / 其他 Component
        ↓
语义输出层
Activate / ValueChange / 自定义事件
        ↓
业务层
真正的应用逻辑
```

其中最重要的分工是：

```text
底层输入负责告诉控件“用户做了什么”

控件行为负责维护自己的交互状态

控件把具体输入转换成更高层语义

业务代码只消费语义事件
```

这就是这次 Headless Button 学习过程中最核心的收获。

---

## 21. 阶段结论

到这里，官方 Headless Button 的主要行为已经完整读过并实际验证：

```text
Button 身份                  ✓
Pointer Press               ✓
Pressed 状态                ✓
Click → Activate            ✓
Release 清理                ✓
DragEnd 清理                ✓
Cancel 清理                 ✓
Keyboard Focus              ✓
Enter / Space → Activate    ✓
InteractionDisabled         ✓
ActivateOnPress             ✓
Observer 注册方式           ✓
Pointer 事件传播            ✓
Commands deferred 顺序      ✓
MiniButton 独立实现验证      ✓
```

因此这一阶段已经从：

```text
“知道官方有一个 Headless Button”
```

推进到了：

```text
“能够解释它为什么这样设计，并自己写出一个最小版本验证核心机制”
```

这可以视为 Headless Button 学习阶段的完整闭环。
