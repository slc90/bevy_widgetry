# 将 ScrollArea 接入 Widget Gallery

## 目标

新增正式 ScrollArea Gallery 页面，用真实 Widgetry public API 展示 v1 的主要能力，同时提供稳定、可重复的 BRP GUI 自动化操作场景。

Gallery 仍是消费者，不向库层下沉 demo 专属 state 或调试 helper。

## Gallery 接入位置

按现有结构增加 gallery/src/pages/scroll_area.rs，并同步：

- gallery/src/pages.rs 的 module 与 scene 导出。
- gallery/src/gallery.rs 的 GalleryPage enum、sidebar navigation button、PageHost page。
- gallery/src/main.rs 中 WidgetryScrollAreaPlugin 的装配。
- 如果 ScrollArea page 需要响应 demo 按钮或 ThemeChanged，再按现有页面模式增加 ScrollAreaDemoPlugin；没有运行时页面行为时不为了形式创建 plugin。

## 页面场景

页面应让以下 v1 行为可以直接肉眼和 BRP 验证：

### Vertical Auto

固定 Viewport 尺寸，放入明显超高内容。用于 wheel y、PageUp/PageDown、Home/End、V thumb drag、V track click。

### Horizontal Auto

固定尺寸，放入明显超宽内容。用于 wheel x、Left/Right、H thumb drag、H track click。

### Both Auto / cross-axis feedback

构造能明确触发“一个 gutter 出现后诱发另一轴 overflow”的场景，展示最终 H+V 稳定状态以及 V 全高、H 只占剩余宽度的无 Corner geometry。

### Always / Hidden

至少能同时看到：

- Always 在无 overflow 时仍保留 bar + gutter，thumb 填满 track。
- Hidden 在有 overflow 时不显示 bar / gutter，但 wheel / keyboard / 程序化滚动仍有效。

### Auto transition

提供一个简单的 Gallery demo 操作，使 root 或 content geometry 能在“需要 scrollbar”和“不需要 scrollbar”之间切换，用于验证稳定状态发生外部 geometry change 后会重新从最小集合求解并撤掉旧 Auto bar。

这是 Auto policy 的真实能力展示，不增加 ScrollArea library runtime config；变化来自普通 Bevy layout / demo content。

### WidgetryScrollIntoView

提供明确的 trigger control 和远处 target：

- target 使用稳定 Name。
- trigger 只负责对 target 发送 WidgetryScrollIntoView。
- BRP 可以先把 target 置于部分可见或不可见，再点击 trigger 检查 top/left alignment。

## BRP 可发现性

页面关键 demo root / trigger / target 使用 BSN `#Name` 提供稳定 Name，例如按语义命名 VerticalAuto、HorizontalAuto、BothAuto、Always、Hidden、ScrollIntoViewTarget 等。

Name 只加在 Gallery 自己负责的 demo entity 上；不要为了 BRP 给 ScrollArea library 的大量内部 entity 增加无业务意义的 public marker 或 debug API。

内部 Viewport 已有公开 WidgetryScrollAreaViewport marker，必要时 BRP 可通过 root hierarchy + component 查询定位 ScrollPosition。

## 视觉要求

Gallery 不覆盖 ScrollArea 的默认 track/thumb theme，以便页面真实展示 library style：

- track transparent。
- thumb normal / hover / dragging 使用既定 ColorTheme token。
- Dark / Light theme 切换后立即更新。

页面自有说明文字和 demo layout 按现有 Gallery 风格处理，不为 ScrollArea 新增 Gallery asset。

## 验证边界

Gallery 本身不增加 unit / integration test。保证可编译，并在下一方案通过 BRP 做运行时验证。

## 预期产出

Gallery 出现可导航的 ScrollArea 页面，覆盖 Vertical / Horizontal / Both、Auto / Always / Hidden、动态 Auto transition 与 WidgetryScrollIntoView，关键 entity 可稳定被 BRP 定位。

## 与前后方案的关系

前置是 facade public API 已完成。这个页面是下一方案 BRP GUI 自动化测试的唯一运行时测试载体；下一方案不再修改 ScrollArea 设计，只验证这里暴露的真实行为。
