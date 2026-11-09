# scroll_area 测试覆盖模型

本模型记录当前自动化测试保护的主要行为契约与测试职责，不逐项列举测试用例。

## States
- Axis、Scrollbar Policy、Viewport／Content 几何、Offset；Keyboard Enabled、Disabled、Thumb Drag 与嵌套 ScrollArea。

## Stimuli
- BSN 构造、内容 Resize、Keyboard／Wheel、Mouse／Custom Pointer Drag、IntoView、Theme／Override。

## Guards
- Offset 限于当前 Viewport／Content 合法范围；Keyboard=false 只关闭键盘滚动。
- Disabled 阻止用户滚动或拖动，但不阻止合法程序化滚动及几何刷新。

## Transitions
- Auto／Always／Hidden 随 Overflow 切换，Thumb Drag 启动／取消、IntoView 定位及 Resize 后 Clamp。

## Invariants
- Thumb 几何、可见区域与 Offset 同步，Content Owner 稳定；IntoView 由最近符合条件的 Viewport 消费。
- 嵌套 ScrollArea 的输入归属正确；Custom Pointer 不获取真实 OS Cursor Lease，Mouse Hover／Drag 终止后恢复适当 Cursor。
- Disabled／Theme／Override 变化不丢失合法 Offset；稳定布局不产生重复重绘，几何损坏恢复后可重建样式投影。
- 结构及颜色失败保留真实业务归属和原始诊断，其他 Root 正常运作。

## Couplings
- Axis × Scrollbar／Overflow；Disabled × Thumb／Keyboard／Wheel；嵌套 IntoView × Viewport；Theme × 动态 Scrollbar。

## Coverage Map

### Integration Test

| 测试文件 | 主要测试职责 |
|---|---|
| `bsn_scroll_area.rs` | 公开 Scene、真实 Layout、Input、IntoView、Drag 和 Cursor 组合。 |
| `diagnostics.rs` | 受管部件、Viewport／Thumb、颜色和队列错误的诊断及恢复。 |

### Unit Test

| 源码测试模块 | 主要测试职责 |
|---|---|
| `src/headless.rs` | Offset、范围和输入状态的局部规则。 |
| `src/layout.rs` | 轴／Scrollbar Policy 的几何求解。 |
| `src/style.rs` | Thumb／Theme／Owner 配色、局部工作量和故障恢复。 |
| `src/colors.rs` | 颜色状态校验与归属。 |
| `src/error.rs` | 错误包装、原因链和宿主传播身份。 |
