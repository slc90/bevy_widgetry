# hold_button 测试覆盖模型

本模型记录当前自动化测试保护的主要行为契约与测试职责，不逐项列举测试用例。

## States
- Idle、Charging、Charged、Decaying／Interrupted；按住进度、成功锁存、Disabled／ReadOnly 和视觉状态。

## Stimuli
- 真实 Mouse／Custom Pointer 按下、移动、松开或取消；公开持续时间与 Props、时间推进、Theme／Override。

## Guards
- 只有有效且未禁用的交互能充能；拖出后的同一次按压不直接重新获得成功资格。
- 非法直径、颜色或无效托管结构必须拒绝且保持已提交状态。

## Transitions
- 按下 → 充能 → 达阈值触发一次 Activate；释放／取消后衰减或重按；隐藏／失焦／Disabled 中断。

## Invariants
- Progress／Attempt 先提交再触发达到阈值的通知；一次有效按住只产生一次激活。
- Headless 不依赖默认 Shader／视觉 Children；视觉圆形命中与透明角穿透遵循自己的几何规则。
- 动画仅在状态变化期间请求重绘，静止／已结束不持续唤醒；Theme／局部覆盖不改变充能状态。
- 故障诊断保留原始配置和托管部件归属，多处失败完整传播，修复后恢复。

## Couplings
- Pointer Capture × Charge；Disabled／Visibility × 会话取消；Headless × 默认 Style；充能进度 × 视觉动画。

## Coverage Map

### Integration Test

| 测试文件 | 主要测试职责 |
|---|---|
| `diagnostics.rs` | 非法尺寸、颜色、结构及受管材质诊断与恢复。 |
| `headless.rs` | 独立无默认 Style 的公开状态、时间推进、通知和输入守卫。 |
| `interaction.rs` | 真实布局／Picking、圆形命中、按住激活、拖出与中断。 |

### Unit Test

| 源码测试模块 | 主要测试职责 |
|---|---|
| `src/headless.rs` | 充能／衰减算法、边界计时、取消和输入会话。 |
| `src/style.rs` | 主题覆盖、成功闪烁与空闲重绘控制。 |
