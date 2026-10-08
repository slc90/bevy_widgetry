# 02 · 在 core 中统一 Hovered 和 DirectlyHovered 的 Pointer 语义

## 目标

在 core 为官方 Hovered/DirectlyHovered 提供唯一的通用 Pointer 写入者。

## 范围

负责精确替换两个 Mouse-only updater、派生语义、装配和排程；不复制 Picking backend，不修改原生 Window move/resize。

## 预期产出

共享适配插件、独立 Widget 装配覆盖、Changed 安静性及多种命中条件测试。

## 与前后方案的关系

消费上一方案的双身份基线；后续 Tooltip 和普通控件消费它提供的一致 hover 状态。

本文件是顺序执行链中的第 2 份，具体位置见 [总览](00-overview.md)。需要查看完整设计时，使用 [总方案](../master-plan.md)。

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

### 为什么需要共享适配

Bevy 0.19.1 的这两个派生状态更新系统只查询 Mouse。Widgetry 多个 crate 读取官方 Hovered 做样式或状态判断，因此只改 Tooltip 或把 BRP 改成 Custom 不够。若让每个 Widget 自己补一个 Custom-hover 布尔值，会出现不同控件各算各的、同一组件多处写入和 Mouse/Custom 两套状态。[U1] [W5]

新增一个小的共享 Pointer 适配插件，建议放在 `crates/core/src/pointer.rs`，并从 core 提供跨 crate 装配入口。继续使用官方 `Hovered`、`DirectlyHovered` 组件，不引入一套与其并行的 WidgetryHovered。这里的职责是修正通用派生状态的输入范围，不复制 Picking backend、Click 或 Drag 算法。

### 只有一个状态写入者

不能保留官方 Mouse-only updater，再在后面每帧覆盖一次。那会让 Custom hover 出现 false→true 的重复变更，影响 Changed 检测、样式更新及事件观察，也可能在排程变化时闪烁。

在所有构建期插件注册完成后、首次正常输入处理前，精确移除 `PreUpdate` 中上游 `update_is_hovered` 和 `update_is_directly_hovered` 这两个 system，再注册通用实现。可沿用本仓库 `window/input.rs` 已使用的 schedule system 移除机制与 `ScheduleCleanupPolicy::RemoveSystemsOnly`，不访问私有 Bevy 字段，不移除整个 PickingSystems::Hover，也不移除其他 hover/interaction 系统。[W13]

对没有这些官方 system 的最小 fixture，SetNotFound 是可解释的情况；其他排程错误必须报告。适配插件要幂等，多次安装多个 Widget plugin 后依然只有一套写入者。若宿主主动安装了自己的 Hovered 写入系统，这是需要显式协调的 ownership 冲突，不承诺无条件兼容第三方自定义 writer。

### 派生规则

以 HoverMap 为命中权威，遍历有效 pointer 的命中集合，而不是固定查某一个身份。有效性包括 pointer 实体仍存在、PointerLocation 有效，以及 target 没有失效；不从 raw Window cursor 计算命中。

`DirectlyHovered(entity)` 表达至少一个有效 pointer 直接命中该 entity。`Hovered(entity)` 表达有效 pointer 命中该 entity 或其 ChildOf 后代，保留官方包含后代的语义。保留遮挡、Pickable、UI 层级和目标窗口的判断结果，不重新用矩形求交覆盖 backend。

在用户约定的单操作者条件下，这个集合通常只有一个有效输入源。实现不需要对 Mouse、Custom、Touch 写分支或优先级；不因未知 Custom UUID 就拒绝。意外存在多个有效 pointer 时，hover 布尔值可按集合并集确定，但这不升级为支持多个并发 press/drag 的承诺。

只有值确实改变时才替换 immutable Hovered/DirectlyHovered 组件。保持 Changed 检测安静，不每帧重插；祖先集合只按实际命中构建，不为每个控件重复遍历整棵 UI 树。[U1]

### 装配和排程

共享插件在 `PickingSystems::Hover` 中、HoverMap 生成之后计算，在 `PostHover` 的 Tooltip 和后续交互消费者读取前完成必要 deferred flush。对新建 UI 子树、重新挂父级、pointer 退场和目标窗口销毁，状态必须在约定的下一次有效 Picking 周期更新，而不是晚到下一次鼠标移动才修复。[U2]

`crates/core/src/ui.rs` 目前只安排 UI 构造/准备阶段，没有这层 hover 适配。可以由已有 WidgetryUiPlugin 安装共享插件，但必须同时覆盖独立安装的 Widget plugin；不能只有 facade/Gallery 路径生效。核对 Button、CheckBox、RadioGroup、TextField、ScrollArea、ListView、Table、Tooltip、Window controls 等装配入口，复用一个幂等入口，不复制算法。[W14]

现有 core 依赖关系足以承载共享能力，不新增 core→具体 Widget 的反向依赖。若需要新增跨 crate 可用的插件类型，属于内部装配能力的公开入口扩展，不要求普通用户更改已有构造代码；facade 不必为了这次改动机械扩大 re-export。

### 影响和验收

这层适配安装后，会影响同一 App 内读取官方 Hovered/DirectlyHovered 的实体，并不只影响带 Widgetry 名字的节点。这一作用域必须写进 rustdoc：Mouse 单独使用时语义保持原样，Custom 的 hover 现在也能反映到官方组件。不是只覆盖某个按钮样式的局部补丁。

验收包含 Mouse 与 Custom 的直接/后代 hover、Pickable::IGNORE、遮挡、多个窗口、节点移除/重新挂接、注销后的 false、静止多帧不触发 Changed，以及单独安装各 Widget plugin 与聚合安装不产生重复系统。Native Window 原生操作 system 没有被移除、替换或加新过滤分支。此方案产出是共享适配及其排程/装配测试，而不是另一个 Picking 框架。

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
