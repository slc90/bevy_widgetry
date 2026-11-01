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
- `diagnostics.rs`：多个 Root 的 Viewport 损坏聚合、重复检查共享 origin、恢复后新失败、Bar / Thumb / 已测量 ComputedNode 损坏、滚动条目标 / 方向与轴策略归属、组合样式消费共享诊断、IntoView 队列边界、无布局 / 空内容 / 零尺寸 / 非所属事件静默，以及宽度配置经 Scene 传播和颜色 API 的独立请求身份。
- `colors.rs` / `error.rs` Unit Test：多个颜色 State 损坏在 Theme 刷新与修复时的生命周期，Core 原 source 与 origin 经领域包装和 ECS 边界保留。
- `diagnostics.rs` 还保护几何损坏期间 Root Override 与 Thumb Hover 被跳过后，几何恢复自动重做样式，无需新的输入。
- `headless.rs`、`layout.rs`、`style.rs` 源码 Unit Test 负责数值、Solver 与 Thumb / Theme 细节。
- `style.rs` Unit Test 保护横向 / 纵向 / 双轴配色、Hover / Drag 状态移除、Disabled 与 Override 清除、Theme、动态 Thumb / ScrollArea、Background 修复和 StyleOwner 交接，以及稳定帧零配色与单 Thumb 更新范围。

