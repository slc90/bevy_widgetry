# 05 · 接入新版 BRP 并完成 Gallery 真实交互交接

## 目标

将固定新版 BRP 接入 Gallery，验证真实鼠标、Custom 与串行交接的完整体验。

## 范围

负责依赖与 MCP 版本一致、GUI 场景、按需更新、事实文档和工程验证；原生拖窗/resize 保留人工回归。

## 预期产出

一致的消费版本、真实 GUI 与人工原生记录，以及准确的已测/未测说明。

## 与前后方案的关系

这是 Widgetry 执行链的末尾；依赖 BRP 的固定消费产物和前四份本地适配，不用浮动 main 替代版本交接。

本文件是顺序执行链中的第 5 份，具体位置见 [总览](00-overview.md)。需要查看完整设计时，使用 [总方案](../master-plan.md)。

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

### 固定消费版本

本方案的实际跨仓库前提是 BRP 已完成其六份小方案，给出 `<POINTER_RELEASE>` 对应的固定 tag/commit、配套 MCP binary、普通输入兼容边界和 `brp_extras/pointer_control` 的真实 contract。不得用当前不存在的版本号假装已经完成依赖升级，也不在消费者里覆盖旧 tag。[W1] [B1]

根 `Cargo.toml` 中的 runtime Git tag、MCP 安装命令注释、`[workspace.metadata.tools]` 版本，以及 `Cargo.lock` 必须一致。Gallery 保持消费 `bevy_brp_runtime`；控件 crate 不引用 Extras 或 BRP 的 pointer state。`gallery/src/main.rs` 目前使用 `BrpRuntimePlugin::default()` 和 `WinitSettings::desktop_app()`，若新版装配契约不变，这里无需为了迁移而额外加一套插件。[W1] [W2]

### 实际 GUI 验收

沿用现有 GUI 规则，通过 MCP 发现并启动 Gallery，取得截图基线，以真实 move/click/drag/scroll 驱动控件，结合截图、ECS state、日志和事件检查结果。不能靠直接修改 selection、Hovered 或 ScrollPosition 通过验收，也不把 powershell/Win32 移鼠标当替代输入。[W16]

本次选取直接受影响的场景：普通按钮 hover 与 activation、CheckBox/Radio、TextField 定位/双击/选择、Tooltip 延迟与退场、ScrollArea thumb/wheel、ListView 行选择、Table 列宽和取消、ComboBox 弹层、一个多窗口 modal/FileDialog 场景。长期细节覆盖留在前面增加的 unit/integration tests，不把 Gallery 验收扩成每次必须跑全历史功能的庞大脚本。

至少完成 Mouse 单独操作、Custom 单独操作和 Mouse→Custom→Mouse 串行交接。交接前允许物理光标静止在另一个控件，验证不会留下双 hover、Tooltip owner 混乱或第一次真人点击落到错误位置。BRP 自动化结束时调用新 control method 的 release，并确认状态已经 inactive，而不是收到接受响应就开始下一轮真人动作。

确认 BRP 工作时系统鼠标仍可留在 App 外；目标 App 的普通交互不靠改 Window.focused 或 OS cursor 完成。后台、不同 DPI、两个窗口的 target 路由按实际可运行环境记录。Native Window 标题栏 move 和窗口边缘 resize 不作为虚拟输入测试目标，也不宣称已自动阻止 Custom Press 触及那里。

### 保持按需更新和视觉稳定

不修改 Gallery 的全局 update mode 来掩盖 timer 或排程问题。hover 稳定后 Changed<Hovered> 应安静；Tooltip 只在等待和必要状态变化时请求 redraw；BRP 的 activity 在动作及退场完成后归零。不能用空闲时持续高频刷新换取表面上的“响应正常”。[W16]

观察必要的中间状态，而不只是最终截图：首次 hover 是否闪一下、按下是否残留、popup 是否先出现于旧窗口、取消后的下一轮是否被旧 release 干扰。可重复的瞬时错误提炼成永久测试。没有测量不宣称延迟改善，截图往返耗时不是控件处理性能指标。

### 文档、工程验证与完成条件

更新 core 共享适配的 rustdoc，明确全 App 的官方 Hovered/DirectlyHovered 语义扩展、写入者 ownership 和独立 Widget plugin 的装配方式。Tooltip 文档明确单 owner、单 popup 和注销规则；GUI 调试说明记录新输入的释放方式、raw mouse 消费者边界以及原生操作例外。`docs/architecture.md` 只在共享模块或实际职责事实需要记录时同步，不把计划当现状。

实施时遵守本仓库 AGENTS 和相关 scope、development、code、architecture、widget-api、testing、documentation、gui-debugging 规则；涉及热路径性能声明再执行相应 benchmark 规范。代码落地前创建项目要求的进度记录，完成后按规则处理。执行定向 crate 测试、`cargo fmt --all -- --check`、`cargo check --workspace`、`cargo clippy --workspace --all-targets`、`cargo build --workspace`、`cargo test --workspace`，再完成本次真实 Gallery 场景。按仓库要求进行独立 Code Review；无环境或无独立 reviewer 能力的项明确列为未完成，不由施工者自审代替。[W17]

最终产出是固定新 BRP 版本的 Gallery、事实一致的说明、双身份永久测试以及真实 GUI/人工原生回归记录。完成的判断是：真实鼠标原来的普通体验不退化，Custom 的普通交互和 hover 确实生效，退场状态闭合，受保护的原生 move/resize 不变，而不是“仓库里再也搜不到 Mouse”。

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
