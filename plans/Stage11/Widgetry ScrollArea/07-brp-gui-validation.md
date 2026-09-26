# 使用 BRP 完成 ScrollArea GUI 自动化验证

## 目标

使用项目已经固定的 bevy_brp_runtime + bevy_brp_mcp 通道，对 ScrollArea Gallery 页面执行真实 GUI 自动化验证，覆盖 Rust unit / integration test 无法可靠替代的 pointer、wheel、drag、focus、layout、theme 和多 frame Auto convergence。

这一步只做验证和必要的缺陷反馈，不把 BRP 测试逻辑下沉成 Widgetry production API。

## 前置条件

Gallery 继续保持：

- WinitSettings::desktop_app()。
- BrpRuntimePlugin::default()。
- 当前 workspace dependency 指向 slc90/bevy_brp 的既定 tag。

BRP runtime 已提供 scroll_mouse，能够注入 x / y 和 MouseScrollUnit；同时使用现有 mouse move/click/drag、keyboard、screenshot、ECS query / watch 与 shutdown 能力。

如果执行环境中 bevy_brp_mcp 不可用，必须明确报告 GUI 验证未完成，不退回 PowerShell / Win32 桌面自动化冒充等价结果。

## 验证内容

### 启动与基线

通过 BRP 发现并启动 widget_gallery，进入 ScrollArea 页面，先截图确认页面完整、默认 theme 正常、没有意外 scrollbar / clipping。

### Vertical wheel

把 pointer 放在 Vertical Auto Viewport：

- 发送 Line y wheel，查询公开 Viewport 上的 ScrollPosition.y，确认使用官方 Line conversion 并被 clamp。
- 发送 Pixel y wheel，确认按像素 delta 变化。
- 发送 x wheel，确认 Vertical-only 不产生 X scroll。

### Horizontal / Both wheel

Horizontal Auto：发送 x wheel 验证 X；发送 y wheel不自定义映射为 X。

Both Auto：分别发送 x / y，确认两个轴独立响应。

### track click

对可见 H/V scrollbar 分别点击 thumb 之外的 track 空白区域：

```text
点击 track 空白区域 → 沿对应方向滚动一个 viewport 的距离
```

同时验证到边界时 clamp，不实现 jump-thumb-to-click。

### thumb drag

拖动 H/V thumb：

- ScrollPosition 连续变化。
- drag 期间 thumb 使用 pressed theme token。
- release 后回到 hover 或 normal state。
- layout / viewport geometry 改变后再次 drag，仍按当前 geometry 映射。

### keyboard 与 focus

点击 ScrollArea 内普通非 focusable 内容，确认 nearest root 的 TabIndex(-1) 获得 focus；点击内部更近的 focusable child 时，focus 留给 child，不由 ScrollArea 抢走。

在 ScrollArea root focused 时：

- Arrow keys 按一个 Line conversion 距离滚动对应轴。
- Vertical / Both 的 PageUp / PageDown 按一个当前 viewport height。
- Home / End 操作 Y 起点 / 终点。
- Horizontal-only 忽略 PageUp / PageDown / Home / End。

### Auto / Always / Hidden layout

截图与 ECS geometry 结合验证：

- Auto 无 overflow 时 gutter 收起。
- 单轴 overflow 只出现对应 bar。
- Both cross-axis demo 最终稳定为正确组合，没有 visible ↔ hidden 振荡。
- V 占完整第二列并跨两行；H 只占第一列第二行；没有 Corner entity。
- Always 在无 overflow 时仍保留 gutter，thumb 填满 track。
- Hidden 在 overflow 时 bar / gutter 不出现，但 wheel / keyboard 仍改变 ScrollPosition。

### Auto transition 与事件驱动收敛

操作 Gallery 的 geometry transition demo，让稳定状态在“需要 scrollbar”与“不需要 scrollbar”之间来回变化。

确认：

- 每次重新求解从最小候选集合开始。
- Vertical → Horizontal 的单调增加最终稳定。
- 旧 H/V 不会永久粘住。
- solver 的 RequestRedraw 能在 WinitSettings::desktop_app() 下自动推进后续 layout pass，不需要额外鼠标晃动或把应用改成 Continuous mode 才收敛。

必要时通过重复 ECS query / watch 观察 Node.display、ComputedNode size 与 ScrollPosition，截图只负责最终视觉确认。

### WidgetryScrollIntoView

把 demo target 放到部分可见和不可见状态后触发 WidgetryScrollIntoView：

- fully visible 时不动。
- partially visible / invisible 时启用轴 top/left 对齐。
- 结果 clamp。
- nested 场景如果 Gallery 提供，则最近 ScrollArea 处理，不带动外层。

### theme

切换 Dark / Light：

- track 仍透明。
- thumb normal / hover / dragging 使用当前 theme 的 control_border / hovered / pressed token。
- theme 切换不修改 ScrollPosition 或 Auto visibility state。

## 通过标准

BRP 验证不能只以“应用没有崩溃”为通过。每个被测输入必须检查对应可观察结果；能用 ECS state 精确判断时优先使用结构化查询，并用 screenshot 验证最终视觉。

发现实现与方案不一致时，回到对应前置小方案修复并重新运行其 Rust 测试，再重新执行受影响的 BRP 场景；不要在 Gallery 中加 workaround 掩盖 library bug。

验证结束后通过 BRP 正常关闭 Gallery。

## 预期产出

ScrollArea 的真实 GUI 行为在事件驱动 Gallery 中得到完整验证：wheel、track、drag、focus、keyboard、Auto convergence、reserved gutter、Hidden / Always、WidgetryScrollIntoView 和 theme 均与设计一致。

## 与前后方案的关系

这是唯一线性执行链的最后一步。它依赖前面所有实现与 Gallery 场景已经完成，不再引入新的产品行为。通过后再按仓库 AGENTS.md 对完整 working-tree change 执行独立 code review 与最终必要验证。
