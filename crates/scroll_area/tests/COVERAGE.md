# scroll_area Integration Test Coverage

本覆盖模型描述现有 Integration Test 重点保护的可观察行为及测试职责，不是逐项 Test Case 清单。

## States

- Axis / Scrollbar Policy、Viewport / Content Layout、Scroll Offset。
- Keyboard Enabled、Disabled、Thumb Dragging、Nested ScrollArea。

## Stimuli

- Public Scene、内容尺寸变化、Mouse / Custom Input、Keyboard、Wheel、IntoView。
- Drag Cancel、Theme、Viewport / Layout 更新。

## Guards

- Scroll Offset 受有效 Content / Viewport 范围限制。
- Keyboard=false 只关闭 Keyboard；Disabled 不阻止程序化滚动。

## Transitions

- Auto / Always / Hidden 随 Overflow 与布局变化。
- Thumb Drag Start / Update / Cancel，Scroll Into View 目标 Reveal。
- 内容 Resize 后位置调整与恢复。

## Invariants

- Scrollbar Geometry 与合法 Offset 同步。
- 最近 Viewport 消费 IntoView；唯一 Content 归属稳定。
- 静止 UI 不产生无效连续布局 / Redraw。
- Custom Pointer 不获取真实 OS Mouse Cursor Lease。
- Mouse Hover / Drag / 终止分别恢复 Grab / Grabbing / 当前 Hover Cursor，祖先禁用仍允许程序化滚动与 Thumb Geometry 更新；原生 Scroll Request 不被投影回写。
- 祖先禁用时 Thumb 使用 Disabled 配色，恢复时回到 Normal，同时保留滚动位置与几何。输入暂停造成的 Scrollbar 暂存不阻断配色。

## Couplings

- Axis × Scrollbar Policy 与 Gutter / Overflow。
- Disabled × Keyboard / Wheel / Drag；IntoView × 嵌套 Viewport。

## Coverage Map

- `bsn_scroll_area.rs`：公开 Scene、真实 Layout、Keyboard / Wheel / IntoView、Drag / Disabled / Cursor 组合。
- `headless.rs`、`layout.rs`、`style.rs` 源码 Unit Test 负责数值、Solver 与 Thumb / Theme 细节。
- `style.rs` Unit Test 保护横向 / 纵向 / 双轴配色、Hover / Drag 状态移除、Disabled 与 Override 清除、Theme、动态 Thumb / ScrollArea、Background 修复和 StyleOwner 交接，以及稳定帧零配色与单 Thumb 更新范围。

