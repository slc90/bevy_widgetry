# ui_scale 测试覆盖模型

本模型记录当前自动化测试保护的主要行为契约与测试职责，不逐项列举测试用例。

## States
- 当前全局 UI Scale、Window 逻辑尺寸基准、普通／排除窗口、现有／新建 Window、显示器／DPI 几何状态。

## Stimuli
- 初始资源、Scale 更新／同值、用户 Resize、排除标记增删、Window 重建、Viewport 和 Monitor 状态变化。

## Guards
- Scale 必须是有限正数；非法输入拒绝并保留旧状态。排除窗口不能被全局缩放误写。

## Transitions
- Scale 修改 → 普通窗口尺寸／约束收敛；用户 Resize 更新设计基准；排除标记切换后重新投影。

## Invariants
- 普通窗口按当前 Scale 变换并保留逻辑设计尺寸；用户真实 Resize 成为后续计算的基准。
- 排除窗口在初次创建及运行期不被误改写；同值 Scale／稳定帧不触发冗余 `Changed<Window>`。
- Window／Camera／Viewport 在当前调度帧完成尺寸同步；修改 Resize Constraints 后 Clamp 一致，不丢失原设计基准。
- Win32 显示器工作区和 Native 查询错误保留源头身份，实际恢复前不伪报恢复；宿主可观察到原始诊断。

## Couplings
- UI Scale × 设计尺寸／Constraints；DPI／Monitor × 原生窗口；Viewport × 相机当帧尺寸。

## Coverage Map

### Integration Test

| 测试文件 | 主要测试职责 |
|---|---|
| `camera.rs` | 真实 Bevy Camera／Viewport 与缩放窗口的同帧收敛。 |
| `scaling.rs` | App／World 全局 Scale、排除窗口和变化通知。 |
| `validation.rs` | 非法 Scale、初始配置和错误传播。 |

### Unit Test

| 源码测试模块 | 主要测试职责 |
|---|---|
| `src/geometry.rs` | Monitor 几何、逻辑／物理尺寸和原生位置计算。 |
| `src/native.rs` | DPI／工作区更新、约束与原生状态转换。 |
