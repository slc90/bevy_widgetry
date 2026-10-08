# 03 · 让 Tooltip 跟随有效 Pointer，而不是固定 Mouse

## 目标

让单个 Tooltip 跟随有效 Pointer，而不是固定读取 Mouse。

## 范围

负责私有 owner、target、延迟与退场状态；保留内容 factory、theme、Popover 布局和单 popup 契约。

## 预期产出

设备无关的 Tooltip 状态机、原有时间边界与交接回归。

## 与前后方案的关系

承接共享 hover 与测试基础；下一方案继续处理其他普通控件的按下和拖动终止。两者按此顺序验收，不要求互相依赖私有状态。

本文件是顺序执行链中的第 3 份，具体位置见 [总览](00-overview.md)。需要查看完整设计时，使用 [总方案](../master-plan.md)。

## 基线与共同约束

版本：方案草案 1，2026-10-05。

核对基线：`slc90/bevy_widgetry@f32f5d0452cf8d63b6d506dc1060236e70367dd5`（本次读取时的 `main`）。配套 BRP 基线为 `slc90/bevy_brp@7eb70b5d24a19b8e7480b11bc1726af4e80cce6c`。Widgetry 固定 Bevy `=0.19.1`，Gallery 当前消费 BRP `v0.2.2`。源码通过 GitHub 连接器核对；上游行为核对到 Bevy `v0.19.1`，不把当前讨论中的判断当作已经验证的实现。[W1] [W2]

这是待实施方案。本次只生成文档，没有改仓库，没有运行编译、测试或真实 Gallery。正文中的共享 Pointer 适配、owner 状态和新的测试入口是方案设计；现状与设计分开说明。

### 共同边界

普通鼠标类交互统一消费 Bevy 的 `PointerInput`、`Pointer<T>`、`PointerId`、HoverMap 和相关派生状态。不在控件中识别 BRP UUID，不把 Mouse 事件另接一条业务路径；真实鼠标仍通过 `PointerId::Mouse` 进入同一套交互。keyboard、IME、剪贴板和应用业务输入不被强行改成 Pointer。

用户限定 BRP 与真人不会同时操作。不设计多人并发，也不设 Mouse 永久优先规则。但源交接、指针注销、旧 hover、按下和拖动的清理仍是必要范围；静止的旧指针也可能留在 HoverMap。BRP 负责虚拟源的激活、退场与真人交接，Widgetry 负责通用 Pointer 消费，不修改宿主 raw mouse 资源或操作系统光标。

**Native Window move/resize 保持现状。** `crates/window/src/title_bar/drag.rs` 与 `crates/window/src/title_bar/resize.rs` 中的原生拖窗、窗口 resize、现有真实按键收尾和系统 cursor 处理不改；不增加 Custom 拒绝分支，不加程序化虚拟拖窗/缩放，不替换 Win32/winit 行为。BRP 验收不操作这些拖拽区域。现有 observer 仍可能接到 Custom Press，保持代码不变不是新增了一道隔离保护。[W3] [W4]

Table 列宽、滚动条 thumb、文本选择属于普通控件拖动，仍在范围内。窗口中的普通按钮、最小化/最大化/关闭 controls、modal blocker 和 FileDialog 正文也没有因位于 Window crate 就全部被排除。统一交互不等于重写所有控件，不改 selection/value/focus 的既有业务契约，不用直接触发 Activate、修改 Value 或 ScrollPosition 冒充输入验证。

生产 Widget crate 不新增 BRP 依赖，也不 fork Bevy。共享适配留在现有 `core`，测试工具留在 `test_utils`，外部 BRP 只由 Gallery 消费。执行总顺序固定为配套 BRP 01 → 02 → 03 → 04 → 05 → 06，再执行本仓库 01 → 02 → 03 → 04 → 05。只有最后接入需要 BRP 已形成固定版本，前面的排序是统一交接顺序，不是假设技术硬依赖。

共享状态遵循唯一写入者原则：统一适配官方 Hovered/DirectlyHovered，而不是保留 Mouse-only writer 再每帧覆盖。普通会话只由其 PointerId 对应的终止或失效信号清理，取消不能产生成功 Click/Drop，正常 Release 也不能因提前清除 Pressed 而丢失 Activate。真实鼠标回归与 Custom 回归使用同一行为断言，但直接触发目标事件、真实 Picking 和桌面验收是不同层次的证据。

## 方案正文

### 当前行为应保留什么

`crates/tooltip/src/headless.rs::resolve_anchor` 固定读取 Mouse 的 HoverMap，`update_tooltip` 也只把 Mouse 的 Move 视为活动。当前延迟为 cold 200ms、warm 50ms、离开后 warm 保留 300ms，基于 Time<Real>。同一 App 同时显示一个 Tooltip；沿命中节点的祖先寻找 anchor；disabled 目标仍可显示说明。[W10]

`style.rs` 通过 Popover 选择下、上、右、左四个位置，保留窗口边距、theme 配色、内容 factory 和子孙 Pickable::IGNORE。这些不是 Pointer 身份问题，不重做布局或改成光标跟随气泡。[W15]

### 单一 owner，但不按设备偏心

给 Tooltip 私有状态增加当前 owner 的 PointerId 和 target 身份。新的有效 PointerInput 活动可选择当前 owner，不识别 BRP UUID，不保留之前讨论中的“Mouse 优先、Custom 备用”。正常使用不并发，选择只是记录当前操作者，不是实现多人仲裁。

有 owner 后，只用该 pointer 的 HoverMap 解析 anchor；不能取所有 HoverMap entry 的第一个，因为 HashMap 迭代顺序不是输入顺序。没有 owner 且只有唯一有效 hover pointer 时可采用它，以支持初始已静止的场景；多个候选且没有新的活动时不随机展示，等待可辨认的输入。这个退化规则不承诺多操作者体验。

活动排序基于本地读到的输入顺序，不声称不同 PointerId 的输入在 Bevy 中存在更强的全局排序保障。对同一输入源，Move 的位置/target 与有效实体要对应。一个 pointer 换窗口也视为上下文改变，旧窗口的 timer、candidate 与 popup 不能带到新窗口。

### 生命周期和时间

同一 owner 内继续沿用 cold/warm/cooldown 规则，owner 的 Move 按原规则重置尚未展示的等待；已经显示时不因为无关 pointer 的历史消息关闭或重启。owner 改变时清理旧 candidate/visible 与等待计时，为新 owner 建立新的等待，不把旧 pointer 已经等了 190ms 的时间借给新 pointer。

Cancel、PointerLocation 失效、pointer 实体移除、target 窗口消失及 anchor 移除，都必须清理相应 Tooltip 状态。不能只等 Pointer<Out>：注销可能没有可用位置，且高层 Cancel 的派发范围也受上游管线限制。清理做到幂等，不重复调用内容 factory，不留旧 popup。[U3] [U5]

保留 `PickingSystems::PostHover` 的消费位置，并确保上一方案的通用 Hovered 状态与本帧 HoverMap 已经就绪。需要等待时由 Tooltip 现有 RequestRedraw 续帧；展示稳定或没有 candidate 后停止多余 redraw，不改 Gallery 全局 update mode。

### 改动与验收

确认修改 `crates/tooltip/src/headless.rs` 及其测试；`style.rs` 只因必要的私有事件/状态连接或测试而调整，不改 public props 和内容 factory 契约。更新 `lib.rs` 的输入与生命周期说明，不把“Pointer 统一”描述成可以并发显示多个 Tooltip。

测试以 Mouse/Custom 参数化覆盖原有 199ms/200ms 等边界、warm/cooldown、从子节点找到 anchor、disabled、同 anchor 内移动、移到另一 anchor、pointer 注销、换窗口及连续 Mouse→Custom→Mouse。真实 Gallery 还要观察 popup 的出现和退场，不以手工 ShowTooltip 事件代替输入验收。该方案交付一个跟随有效 Pointer 的单 Tooltip 状态机。

## 来源

来源链接固定到本次核对的 commit 或 Bevy `v0.19.1`。下列源码事实不等于完整运行验证。GitHub fork 搜索曾返回 incomplete results，所以本方案明确保留实施时的本地完整符号扫描；没有把未读到的每个文件宣布为已审查或无需修改。

[W1]: https://github.com/slc90/bevy_widgetry/blob/f32f5d0452cf8d63b6d506dc1060236e70367dd5/Cargo.toml
[W2]: https://github.com/slc90/bevy_widgetry/blob/f32f5d0452cf8d63b6d506dc1060236e70367dd5/gallery/src/main.rs
[W3]: https://github.com/slc90/bevy_widgetry/blob/f32f5d0452cf8d63b6d506dc1060236e70367dd5/crates/window/src/title_bar/drag.rs
[W4]: https://github.com/slc90/bevy_widgetry/blob/f32f5d0452cf8d63b6d506dc1060236e70367dd5/crates/window/src/title_bar/resize.rs
[W5]: https://github.com/slc90/bevy_widgetry/blob/f32f5d0452cf8d63b6d506dc1060236e70367dd5/crates/button/src/style.rs
[W6]: https://github.com/slc90/bevy_widgetry/blob/f32f5d0452cf8d63b6d506dc1060236e70367dd5/crates/check_box/src/checkbox.rs
[W7]: https://github.com/slc90/bevy_widgetry/blob/f32f5d0452cf8d63b6d506dc1060236e70367dd5/crates/combo_box/src/popup.rs
[W8]: https://github.com/slc90/bevy_widgetry/blob/f32f5d0452cf8d63b6d506dc1060236e70367dd5/crates/list_view/src/behavior.rs
[W9]: https://github.com/slc90/bevy_widgetry/blob/f32f5d0452cf8d63b6d506dc1060236e70367dd5/crates/table/src/resize.rs
[W10]: https://github.com/slc90/bevy_widgetry/blob/f32f5d0452cf8d63b6d506dc1060236e70367dd5/crates/tooltip/src/headless.rs
[W11]: https://github.com/slc90/bevy_widgetry/blob/f32f5d0452cf8d63b6d506dc1060236e70367dd5/crates/test_utils/src/pointer.rs
[W12]: https://github.com/slc90/bevy_widgetry/blob/f32f5d0452cf8d63b6d506dc1060236e70367dd5/crates/test_utils/src/lib.rs
[W13]: https://github.com/slc90/bevy_widgetry/blob/f32f5d0452cf8d63b6d506dc1060236e70367dd5/crates/window/src/input.rs
[W14]: https://github.com/slc90/bevy_widgetry/blob/f32f5d0452cf8d63b6d506dc1060236e70367dd5/crates/core/src/ui.rs
[W15]: https://github.com/slc90/bevy_widgetry/blob/f32f5d0452cf8d63b6d506dc1060236e70367dd5/crates/tooltip/src/style.rs
[W16]: https://github.com/slc90/bevy_widgetry/blob/f32f5d0452cf8d63b6d506dc1060236e70367dd5/rules/gui-debugging.md
[W17]: https://github.com/slc90/bevy_widgetry/blob/f32f5d0452cf8d63b6d506dc1060236e70367dd5/AGENTS.md
[U1]: https://github.com/bevyengine/bevy/blob/v0.19.1/crates/bevy_picking/src/hover.rs#L320-L455
[U2]: https://github.com/bevyengine/bevy/blob/v0.19.1/crates/bevy_picking/src/lib.rs#L256-L450
[U3]: https://github.com/bevyengine/bevy/blob/v0.19.1/crates/bevy_picking/src/events.rs#L650-L1215
[U4]: https://github.com/bevyengine/bevy/blob/v0.19.1/crates/bevy_ui_widgets/src/text_input.rs#L157-L261
[U5]: https://github.com/bevyengine/bevy/blob/v0.19.1/crates/bevy_picking/src/hover.rs#L151-L210
[U6]: https://github.com/bevyengine/bevy/blob/v0.19.1/crates/bevy_ui_widgets/src/button.rs
[B1]: https://github.com/slc90/bevy_brp/blob/7eb70b5d24a19b8e7480b11bc1726af4e80cce6c/docs/architecture.md
