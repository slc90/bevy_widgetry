# tooltip 测试覆盖模型

本模型记录当前自动化测试保护的主要行为契约与测试职责，不逐项列举测试用例。

## States
- 无候选／Cold Wait／Warm Wait／Visible，Anchor／Owner 身份与 Popup 生命周期。
- Theme、局部覆盖、Disabled 和动态内容准备状态。

## Stimuli
- 真实／受控 Hover、时间推进、Anchor／Owner 销毁、Content Factory、Theme／Override 与 Window Focus 变化。

## Guards
- 必须具备有效 Content Factory；同一 Anchor 持续悬停不能重复创建；Disabled Anchor 仍允许 Tooltip 显示。

## Transitions
- Hover Wait → Show → 离开／销毁后 Cleanup；Anchor／Pointer 更换和 Warm Window 影响后续显示。

## Invariants
- 同时至多一个 Popup，Factory 仅在真正显示时执行；失效 Anchor 或隐藏后不残留孤儿 Entity。
- 新 Popup 在真实 Picking 前完成忽略命中标记和 Text／Icon 内容准备。
- Headless 不要求默认字体／视觉 Plugin，默认 Plugin 与自定义实例共存不抢夺归属。
- Theme／Override 更新可见内容不重建业务子树；故障 Popup 清理、诊断身份和恢复行为符合托管约束。

## Couplings
- Hover Owner × 冷／暖等待；Factory × Popup 生命周期；Theme × Anchor／动态内容；Window 失焦 × Hide。

## Coverage Map

### Integration Test

| 测试文件 | 主要测试职责 |
|---|---|
| `diagnostics.rs` | 非法颜色／内容请求、Owner／Scene 失败与诊断传播。 |
| `independent.rs` | 独立 Headless、异位 Popup、过期请求和销毁／失焦边界。 |
| `tooltip.rs` | 公开 Scene、真实 Hover／Timer、Popup、Theme 和内容准备。 |

### Unit Test

| 源码测试模块 | 主要测试职责 |
|---|---|
| `src/headless.rs` | 精确计时、Owner／Anchor 资格与私有 Guard。 |
| `src/style.rs` | Popup 配色、动态部件归属、局部工作量及故障恢复。 |
