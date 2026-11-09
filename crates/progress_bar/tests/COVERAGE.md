# progress_bar 测试覆盖模型

本模型记录当前自动化测试保护的主要行为契约与测试职责，不逐项列举测试用例。

## States
- 确定进度 `[0,1]`、不确定模式；已提交业务状态、动画显示状态和 Theme／布局。

## Stimuli
- BSN／Headless 构造、World／Commands 设置、模式切换、时间推进、Theme／Override 与尺寸变化。

## Guards
- 无效 Root／运行时结构及 NaN、无穷或范围外进度明确拒绝；失败后旧状态不变。

## Transitions
- 增加／倒退／归零／完成后回退、两种模式切换、视觉追赶和循环动画启停。

## Invariants
- 状态 API 即时读回最后提交值；合法同值不重启动画，不产生业务 `ValueChange` 事件。
- 构造失败清理新 Root，延后错误交给宿主 Handler；不重置调用方 Name 和布局身份。
- 确定进度的视觉过渡收敛，不确定模式的循环区间有界；静止时停止额外 RequestRedraw。
- Headless 独立运行时不创建默认 Style 或动画资源；Theme／尺寸更新保留业务值。

## Couplings
- 业务进度 × 动画插值；模式 × 循环相位；Theme／Layout × 视觉填充；Headless × 默认 Style。

## Coverage Map

### Integration Test

| 测试文件 | 主要测试职责 |
|---|---|
| `animation.rs` | 确定进度过渡、不确定循环、暂停／恢复与重绘。 |
| `diagnostics.rs` | 非法状态、颜色／结构和 Scene／Command 错误传播。 |
| `headless.rs` | 公开状态、构造、读回、模式与非法输入。 |
| `independent.rs` | 独立无视觉 Headless、排队提交及无额外动画。 |
| `style.rs` | 真实布局、尺寸、Theme、Override 和视觉填充。 |

### Unit Test

| 源码测试模块 | 主要测试职责 |
|---|---|
| `src/progress.rs` | 局部状态校验、同值处理和非法结构。 |
| `src/animation.rs` | 插值收敛、循环边界与跨模式计时。 |
