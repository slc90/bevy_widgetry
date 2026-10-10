# 07｜将 Tooltip 页接入按需生命周期

## 目标

使 Tooltip 演示页的锚点与悬停 UI 随页面进入/退出，同时保留控件自身的应用级指针状态处理。

## 范围

`gallery/src/pages/tooltip.rs` 的 BSN Scene 与鼠标悬停行为；只检查 `crates/tooltip` 既有清理契约，不修改 crate。

## 预期产出

离开 Tooltip 后无可见或可触发的陈旧页面锚点，再进入时示例从默认状态显示。

## 与前后方案的关系

承接公共 UI-only 页面模式；下一份开始迁入带独立 Model 的页面。

## 方案正文

`tooltip::scene()` 直接描述 Basic、Rich、Disabled、Placement 等演示锚点，不由 Gallery Plugin 构建数据源。进入页时照原 Scene 包装挂载；退出时销毁页面根及所有提示锚点。

Tooltip Widget 插件自己的 `TooltipState` 是应用级 Resource，`tooltip_removed(On<Remove<Tooltip>>)` 已负责清除被移除锚点对应的 candidate/visible 并发出 HideTooltip。**不要**在 `OnExit(Tooltip)` 中删除该共享 Resource，也不将其错误划归 Gallery 页面所有。退出时若指针恰好停在 Tooltip 上，依据现有 `Remove<Tooltip>` 清理机制验证不会留出旧锚点内容；如果鼠标移入新页，新的 Tooltip 仍可正常显示。

保留 `TooltipNav` 和原页面中的内容与 Name，若其他页面的控件带 Tooltip，不应因退出 Tooltip 示例页而关闭其全局注册机制。

**源代码：** [tooltip.rs](https://github.com/slc90/bevy_widgetry/blob/main/gallery/src/pages/tooltip.rs)、[crates/tooltip/headless.rs](https://github.com/slc90/bevy_widgetry/blob/main/crates/tooltip/src/headless.rs)。
