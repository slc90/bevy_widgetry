# Stage 1：从官方 Button 到 Long Press —— Headless 控件与测试的第一轮实战

这一阶段看起来只是“给 Button 加一个长按”，但真正学到的东西其实比长按本身多得多。

一开始只是想弄清 Bevy 0.19 的官方 `Button` 到底是怎么工作的；做到后面，已经顺手把 **Headless Widget 的组织方式、事件链、内部状态、单元测试、集成测试、TDD、可控时间** 都摸了一遍。

回头看，Stage 1 更像是在给后面的整个控件库打地基。

## 1. 先不急着自己造 Button

最开始读的是官方 `bevy_ui_widgets::Button`。

这里最重要的不是把源码逐行记住，而是理解它为什么拆成现在这个样子。

官方 Button 大致可以看成这样：

```text
Button
  ↓
接收输入
  ↓
维护 Pressed 等状态
  ↓
必要时触发 Activate
```

输入并不是全塞进一个巨大的 system 里，而是通过不同的 observer 分开处理，例如：

```text
Pointer<Press>
Pointer<Release>
Pointer<DragEnd>
Pointer<Cancel>
键盘输入
```

每条输入路径负责自己该做的事情。

这让我第一次比较明确地感受到，Bevy 的 Headless Widget 更接近：

> 一个由 Component + Observer + System + Event 组成的行为模块。

而不是传统 GUI 里那种“一个 Button 对象，里面塞一堆回调和状态”。

### `Pressed` 不是样式

这一点也很重要。

`Pressed` 表示的是控件当前的行为状态，不是“按钮应该画成按下去的样子”。

至于最后显示成什么颜色、有没有阴影、缩放多少，那是另一层的事情。

这也是 Headless Widget 的核心味道：

```text
行为
和
外观
尽量分开
```

### `InteractionDisabled` 也属于行为契约

官方 Button 在处理 Press 时会检查 `InteractionDisabled`。

也就是说，“disabled 时不响应输入”不是皮肤层决定的，而是控件本身的行为约束。

这个细节后来还真的救了我们一次，因为 Long Press 第一版实现里把它漏掉了。

## 2. Long Press：扩展 Button，而不是重写 Button

这一阶段最后选择的是：

```rust
#[derive(Component, Debug)]
#[require(Button)]
pub struct LongPressButton {
    pub press_duration: u64,
}
```

也就是说，`LongPressButton` 本身不是另一个 Button。

它的意思更接近：

> “这个官方 Button 额外拥有 Long Press 行为。”

这比重新实现一套 Button 要自然很多。

官方 Button 已经处理好的基础行为继续复用，我们只增加自己真正需要的那一层。

## 3. Long Press 的内部状态

长按本质上不是一个瞬间事件，而是一个过程：

```text
Press
  ↓
进入等待状态
  ↓
持续计时
  ↓
达到阈值
  ↓
LongPressEvent
```

所以实现里增加了一个临时状态：

```rust
pub struct LongPressPending {
    pub timer: Timer,
}
```

这个 Component 只在“正在等待长按成立”的时候存在。

于是整个状态变化就比较直观：

```text
Press
  ↓
插入 LongPressPending
  ↓
Update 中持续 tick
  ↓
到时间
  ↓
触发 LongPressEvent
  ↓
移除 LongPressPending
```

如果中途发生：

```text
Release
Cancel
DragEnd
```

就提前移除 `LongPressPending`。

所以它们虽然是三种不同输入，但最后表达的是同一件事情：

> 这一次长按已经失效。

## 4. Disabled 的那个小坑，刚好体验了一次 TDD

最初的 Long Press Press observer 没有检查 `InteractionDisabled`。

如果只是肉眼看代码，很容易一时没注意到。

后来补了测试：

```text
InteractionDisabled
  ↓
Press
  ↓
不应该进入 LongPressPending
```

第一次跑：

```text
Red
```

然后顺着失败去找：

```text
为什么 Pending 出现了？
  ↓
是谁创建了 Pending？
  ↓
Press observer
  ↓
它有没有检查 InteractionDisabled？
  ↓
没有
```

于是只做最小修改，加上 disabled 判断。

再跑：

```text
Green
```

这算是这一阶段第一次很完整地走了一遍：

```text
Red → 最小修复 → Green
```

而且不是为了“练 TDD”硬造出来的例子，是真的有 bug。

## 5. 单元测试和集成测试，终于不只是概念上的区别了

这一阶段对测试最大的收获，可能就是把这两个视角真正分开了。

我们最后约定：

```text
src/ 里的 #[cfg(test)]
    ↓
更偏内部实现 / 内部状态

tests/
    ↓
更偏公开行为契约
```

当然 Rust 对“单元测试 / 集成测试”的严格分类还是由代码位置决定的，但在这个项目里，我们进一步给它们划了职责。

### 单元测试看内部

例如：

```text
Press → LongPressPending 出现
Release → LongPressPending 消失
Cancel → LongPressPending 消失
DragEnd → LongPressPending 消失
Disabled → 不创建 LongPressPending
配置的 duration → 正确进入内部计时状态
```

这些测试知道 `LongPressPending` 的存在。

它们是在检查：

> 内部这块零件有没有按预期工作。

### 集成测试假装完全看不见实现

集成测试只知道公开 API：

```text
LongPressButton
LongPressEvent
InteractionDisabled
Pointer 输入
```

它甚至不应该关心内部是不是用了 `Timer`。

这时测试的问题就变成：

```text
给它一个输入
  ↓
经过一段时间
  ↓
外面能观察到什么？
```

这个视角特别重要。

假设以后内部不再用 `LongPressPending`，甚至不用 `Timer`，只要公开行为没变，集成测试就不应该跟着重写。

## 6. “重复测试”不一定真的是重复

单元测试和集成测试可以覆盖同一个事实。

例如自定义长按时间：

```text
press_duration = 300ms
```

单元测试可以证明：

```text
300ms 的配置
  ↓
正确进入内部计时状态
```

集成测试则证明：

```text
300ms 的配置
  ↓
外部真的在 300ms 收到 LongPressEvent
```

表面上都在测 duration，但得到的保证不一样。

这一阶段最后留下了一个挺好用的判断：

> 如果一条测试换一个视角以后，并没有增加新的保证，那就没必要机械地再写一遍。

所以不是：

```text
每条单元测试
都必须配一条集成测试
```

而是：

```text
内部关键机制 → 单元测试
公开关键契约 → 集成测试
必要时它们自然会有重叠
```

这个感觉比死记“单元测试小、集成测试大”实用多了。

## 7. Headless 测试的最小模型

这一阶段测试基本都可以压缩成：

```text
准备
  ↓
输入
  ↓
检查结果
```

更贴近控件行为的话，就是：

```text
输入
  ↓
状态变化
  ↓
事件
```

例如官方 Button：

```text
Pointer<Press>
  ↓
Pressed
```

Long Press：

```text
Pointer<Press>
  ↓
等待
  ↓
达到阈值
  ↓
LongPressEvent
```

而 Release / Cancel / DragEnd：

```text
等待中
  ↓
取消输入
  ↓
这一次 Long Press 永久失效
```

这个模型之后应该还能一直复用到别的控件上。

## 8. `Commands` 是 deferred 的，所以测试里遇到了 `flush()`

第一次测：

```text
Pointer<Press> → Pressed
```

明明 observer 已经跑了，断言却看不到 `Pressed`。

原因是 observer 里通过 `Commands` 插入 Component，这些修改不是在那一行立刻写进 World。

所以测试里：

```rust
app.world_mut().trigger(...);
app.world_mut().flush();
```

之后再断言。

这件事在正常 App 运行时通常不需要一直手动想，但写这种极小的 Headless 测试时，它会直接暴露出来。

反而是个挺好的学习机会：以前知道 `Commands` 是 deferred，和真的因为它让测试失败一次，记忆强度完全不是一回事 😂

## 9. 测时间的时候，不要真的等

Long Press 最麻烦的地方当然是时间。

测试肯定不能写：

```rust
std::thread::sleep(...)
```

否则又慢又不稳定。

Bevy 的 `TimeUpdateStrategy::ManualDuration` 很适合这个场景。

例如：

```rust
TimeUpdateStrategy::ManualDuration(
    Duration::from_millis(100)
)
```

然后：

```rust
app.update();
```

就可以把测试理解成“手动推进一帧”。

于是我们可以自己控制：

```text
+100ms
+100ms
+100ms
...
```

整个测试几乎瞬间跑完，但控件看到的时间仍然是正常的 Bevy `Time::delta()`。

### 第一帧还有一个小陷阱

`TimePlugin` 第一次 `app.update()` 时主要是在初始化时钟。

所以第一次 update 的 delta 是 0。

也就是说，不能简单地认为：

```text
调用 5 次 update
= 一定过去 500ms
```

在我们的测试 setup 里，先单独：

```rust
app.update(); // 初始化 Time
```

然后再 Press。

之后的 update 才真正作为长按时间计算。

这个坑也是实际跑测试才发现的。

## 10. 300ms 又挖出了一个浮点精度问题

最开始：

```rust
pub press_duration: f32;
```

内部通过：

```text
毫秒
÷ 1000.0
→ f32 秒
→ Timer::from_seconds
```

来创建 Timer。

看起来没什么问题，直到集成测试配置：

```text
300ms
```

测试期望：

```text
100ms → 不触发
200ms → 不触发
300ms → 触发
```

结果 300ms 时还是 0。

原因就是经典的浮点问题：

```text
0.3
```

不能被二进制浮点数精确表示。

有意思的是，之前 750ms 的测试一直没出问题，因为：

```text
0.75
```

刚好可以被精确表示。

最后把配置改成整数毫秒：

```rust
pub press_duration: u64;
```

然后直接：

```rust
Timer::new(
    Duration::from_millis(press_duration),
    TimerMode::Once,
)
```

整个链条就没有必要经过浮点数了：

```text
300
  ↓
Duration::from_millis(300)
  ↓
Timer
```

这也是这一阶段一个很漂亮的例子：

> 单元测试能通过，不代表公开行为的边界一定没问题。

最后还是集成测试把这个问题抓了出来。

## 11. Stage 1 最终覆盖的行为契约

现在 Long Press 的核心行为已经有测试保护：

```text
未达到阈值
→ 不触发

达到阈值
→ 触发 LongPressEvent

触发以后继续等待
→ 不重复触发

Release before threshold
→ 取消

Cancel before threshold
→ 取消

DragEnd before threshold
→ 取消

InteractionDisabled
→ 不触发

自定义 press_duration
→ 实际改变触发时间
```

而且其中 Release / Cancel / DragEnd 虽然最终结果相同，仍然分别保留测试。

因为它们是三条独立输入路径。

如果哪天不小心少注册了一个 observer，只测其中一个是抓不到回归的。

## 12. 到这里，Stage 1 真正打下来的东西

如果只看代码量，这一阶段其实没有写很多东西。

但现在对 Bevy Headless Widget 的理解已经比一开始完整不少：

```text
控件不是一坨 UI 对象

而是：

Component
  +
输入 Observer
  +
内部状态
  +
Update System
  +
对外 Event
```

测试也不再只是“给代码补几个 assert”。

而是开始能区分：

```text
内部机制有没有对
```

和：

```text
外部行为契约有没有对
```

这两件事。

Long Press 本身只是第一个练习。

后面再做别的控件时，真正值得复用的应该是这一整套思路。

---

Stage 1 到这里可以收工了。

虽然中间只是多按了几次按钮，但已经顺手踩过 deferred Commands、测试时间初始化、disabled 行为遗漏、浮点 duration 精度这些坑。

还挺值的 😂
