# splash 测试覆盖模型

本模型记录当前自动化测试保护的主要行为契约与测试职责，不逐项列举测试用例。

## States
- 独立 Splash 会话、窗口 Hidden／Visible、显示器元数据、布局／首帧准备及主窗口交接状态。

## Stimuli
- 显式启动和关闭会话、Monitor／DPI 信息变化、内容 Ready、帧呈现信号、主窗 Ready、取消与异常。

## Guards
- 未安装／未启用会话不得改变主窗；非法目标、无尺寸或缺少必要帧证明时不提前 Show。
- Splash 的窗口尺寸与原生样式策略不由普通 UI Scale 或外部改写破坏。

## Transitions
- 创建独立窗口 → 等待位置与内容／帧 → 显示 → 等待主窗就绪并完成交接；取消／失败进入资源清理。

## Invariants
- 独立 Splash 保留自己的 Window 身份、固定尺寸、显示器选择及用户不可 Resize 约束；主窗与宿主资源不受错误会话损坏。
- 首帧和交接只依赖当前有效的内容／窗口／比例快照，过期 Ticket／重复信号不得触发重复显示。
- 真实帧不具备时，即使其他准备信号已到，也不能把未呈现画面当作完成；取消后不重入或伪造 Completed。
- 显示与 DWM／原生操作失败保留真实错误身份，适用的后续恢复仍执行，组合失败完整传播。

## Couplings
- Monitor／DPI × Window 几何；内容／帧 Ready × Show；主窗 Ready × Splash Finished；Cancel × 延迟命令。

## Coverage Map

### Integration Test

| 测试文件 | 主要测试职责 |
|---|---|
| `window.rs` | Splash 的公开窗口会话、几何、首帧、主窗交接与错误场景。 |

### Unit Test

| 源码测试模块 | 主要测试职责 |
|---|---|
| `src/geometry.rs` | 显示器选择、几何收敛与元数据等待。 |
| `src/presentation.rs` | 帧资格、Ticket／信号去重和主窗交接状态机。 |
| `src/native.rs` | 原生 Show／Flush 错误及资源恢复。 |
