# Widgetry ScrollArea 分解方案总览

## 整体目标

在 Bevy 0.19.1 上为 bevy_widgetry 增加通用 ScrollArea Widget，支持任意嵌套 UI 内容、Vertical / Horizontal / Both 三种滚动轴、Auto / Always / Hidden scrollbar policy、wheel / trackpad、scrollbar drag、点击 track 空白区域、keyboard、程序化 ScrollPosition 和 WidgetryScrollIntoView，并按项目现有 headless / style 分工、BSN、TDD、Gallery 与 BRP GUI 验证规则接入整个 workspace。

本次只实现已经确认的 v1 范围，不扩展 overlay scrollbar、惯性滚动、scroll chaining、Shift+wheel 自定义映射、内容拖拽、disabled、独立 scroll changed event、额外 accessibility 设计、运行时修改 axis / policy / thickness、外部 min_thumb_length 配置等能力。

## 全局检查后的必要修正

全局检查发现三处必须在实施方案中修正或补全，否则会与既定行为冲突：

1. **TabNavigationPlugin 必须进入需要 pointer focus 的 Widget plugin 装配。** Bevy 0.19.1 的 click-to-focus 与 AcquireFocus observer 位于 TabNavigationPlugin，而不是 InputFocusPlugin。ScrollArea 既要求 root 的 TabIndex(-1) 可通过 pointer 获得 focus，又依赖 focused keyboard input，因此 WidgetryScrollAreaPlugin 应确保 TabNavigationPlugin 已安装。去除 WidgetryFocusPlugin 后，WidgetryTextFieldPlugin 也应改为确保 TabNavigationPlugin；WidgetryRadioGroupPlugin 保留现有 TabNavigationPlugin，只移除 WidgetryFocusPlugin。
2. **Viewport marker 需要成为公开 API。** 已确认程序化滚动直接使用原生 ScrollPosition，且不增加 scroll_to wrapper。由于 ScrollPosition 位于内部 Viewport 而不是 root，必须提供稳定的公开查询入口。因此使用公开 WidgetryScrollAreaViewport marker；Content marker 与 runtime config 仍保持 crate-private。
3. **Auto visibility 需要跨真实 UI layout pass 收敛。** Grid gutter 改变会反过来改变 Viewport 和任意用户内容的 layout，不能假设同一个 ui_layout_system pass 内完成 fixed-point。实现应使用每个 ScrollArea 自己的 private convergence state，在连续 layout pass 中按 Vertical → Horizontal 单调增加 Auto scrollbar，并在未稳定时发送 RequestRedraw，保证 Gallery 的 WinitSettings::desktop_app() 事件驱动模式继续推进到稳定状态。

另外，Viewport 使用独立 scrollbar entity 和 Grid gutter，因此保持 Node.scrollbar_width = 0，不使用 Bevy 内建 scrollbar reservation，避免重复扣减可视区域。

## 唯一执行顺序

1. [01-remove-custom-focus.md](01-remove-custom-focus.md) — 删除 Widgetry 自定义 focus policy，统一切换到 Bevy 官方 focus / TabNavigation 行为，并修复直接受影响的 TextField、RadioGroup、facade 与测试。
2. [02-scroll-area-headless.md](02-scroll-area-headless.md) — 建立 ScrollArea crate、公开行为 type、Viewport/Content 语义结构、wheel/keyboard/WidgetryScrollIntoView 等 headless 基础。
3. [03-scroll-area-auto-layout.md](03-scroll-area-auto-layout.md) — 完成 scrollbar policy、reserved gutter、双轴 Auto fixed-point 与跨 layout pass 收敛。
4. [04-scroll-area-style.md](04-scroll-area-style.md) — 完成 WidgetryScrollArea SceneComponent、Props、BSN hierarchy、Content patch 规则、scrollbar 视觉与完整 public plugin 装配。
5. [05-public-integration.md](05-public-integration.md) — 完成 facade public API、public API integration test 和 architecture 文档的最终同步。
6. [06-gallery-integration.md](06-gallery-integration.md) — 将 ScrollArea 作为正式 Gallery 页面接入，提供可人工和 BRP 操作的代表性场景。
7. [07-brp-gui-validation.md](07-brp-gui-validation.md) — 使用 Gallery 已有 bevy_brp_runtime 完成真实 wheel、drag、track、keyboard、layout、theme 与 ScrollIntoView 的 GUI 自动化验证。

## 跨方案执行约束

所有代码阶段继续遵守仓库 AGENTS.md 与 rules/：使用 Type-Driven Development + Test-Driven Development；新增或改变可观察行为必须先有对应 Rust 自动化测试；Gallery 不增加 Rust test，但涉及 GUI 的行为最终必须走 BRP；不为了测试扩大 visibility；不新增与当前目标无关的 abstraction；代码、注释和 Markdown 以中文为主要叙述语言并保留技术术语英文原词。

这条链中的 Rust 自动化测试分为 module unit test 与 crate integration test；BRP GUI 自动化是第三层运行时验证，不替代前两者。
