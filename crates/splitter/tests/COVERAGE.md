# splitter 测试覆盖模型

本模型记录当前自动化测试保护的主要行为契约与测试职责，不逐项列举测试用例。

## States
- 水平／垂直主轴、保存比例、Pane 最小／最大约束、布局尺寸及 Resizable／Disabled。
- Pointer Drag、Focus、Theme／Override 和嵌套 Splitter。

## Stimuli
- BSN 构造、尺寸／约束变化、公开 Ratio World／Commands、真实 Pointer Drag、Theme 与窗口 Scale。

## Guards
- 非法尺寸、NaN 约束、缺失 Handle／Pane、无有效布局或失效 Root 必须拒绝，不改变已提交比例。
- Disabled／Resizable=false 阻止对应拖动，但合法程序化比例更新继续可用。

## Transitions
- 初始化约束收敛 → Ratio Clamp；主动拖动／设置 → 比例更新；父级 Resize 和 Pane 约束变化 → 被动修正。

## Invariants
- 约束求解在真实布局与 Border／Padding 下保持两侧可达范围和有限比例；首次初始化的被动调整不产生虚假通知。
- 主动提交先于通知，合法同值与被动 Resize 静默；失败保留当前比例、Pane 和用户内容。
- 指针离开区域、Clamp 到端点、取消／失焦／销毁后 Capture 正确回收；嵌套实例不串用手势。
- Theme／Override、细线尺寸和独立命中区域不改变 Pane 几何与比例。
- 持续布局／结构故障按真实恢复时机去重；原始浮点位、Owner 和宿主错误链保留。

## Couplings
- Min／Max × Pane／Root Layout；Pointer × Capture／Resizable；DPI／UI Scale × Ratio；Theme × Handle 样式。

## Coverage Map

### Integration Test

| 测试文件 | 主要测试职责 |
|---|---|
| `box_layout.rs` | Box Pane 的 Padding／Border、分数约束与初次比例收敛。 |
| `constraints.rs` | 两侧 Min／Max、百分比／像素约束及动态布局。 |
| `diagnostics.rs` | 结构、比例、样式和布局故障的身份、传播和恢复。 |
| `input.rs` | 真实 Pointer Drag、Clamp、取消／失焦与嵌套隔离。 |
| `layout.rs` | 主轴布局、初始／被动求解和 Pane 身份。 |
| `ratio_api.rs` | 公开 World／Commands Ratio、通知和非法操作。 |
| `style.rs` | Handle 几何、Theme／Override、Disabled／Resizable 和 Cursor。 |

### Unit Test

| 源码测试模块 | 主要测试职责 |
|---|---|
| `src/input.rs` | 多手势 Owner 的 Capture／Pressed 清理、局部失败隔离，以及移动命令中主错误与清理错误的共同传播。 |
