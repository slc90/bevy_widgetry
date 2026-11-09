# cursor 测试覆盖模型

本模型记录当前自动化测试保护的主要行为契约与测试职责，不逐项列举测试用例。

## States
- System／Custom／Static／Animated、Default／Hover／Dragging 及 Native 控制权；Asset Loading／Failed／Recovered。
- 每窗口 Pointer、Hover、Capture、Lease 与真实时间动画相位。

## Stimuli
- Pointer 命中、拖动与取消、窗口失焦、Owner 销毁、Theme 与 Cursor 声明／图集变化。

## Guards
- 仅 Mouse 更新 OS Cursor；Custom／Touch Pointer 不抢占原生指针。
- 有效声明按既定优先级选取，Native Move／Resize 期间让权；失效资产或非法帧必须安全回退。

## Transitions
- Hover／Lease 取得与释放、Loading／Disabled／ReadOnly 覆盖和恢复、静态与动画主题切换。

## Invariants
- 各 Window 的 Cursor 状态与资源身份隔离，Owner／Window 失效后不残留 Lease 或无用动画唤醒。
- Loading 优先 Wait，Disabled 优先 NotAllowed，ReadOnly 默认 Cell；恢复后选取当前有效声明。
- 同值主题不重置动画相位；新语义从初帧启动，动画按真实时间推进，停用时停止重绘请求。
- 图集故障、后备与恢复使用可关联的真实诊断，持续故障不产生重复源头记录。

## Couplings
- Hover／Capture × Lease；Native 控制权 × Cursor 声明；动画 × Asset／Window 生命周期。

## Coverage Map

### Integration Test

| 测试文件 | 主要测试职责 |
|---|---|
| `animation.rs` | 合成图集的帧时序、循环、资产异常、后备与唤醒控制。 |
| `coordination.rs` | 多层声明优先级、Disabled／Loading、窗口隔离及 Native／Lease 协作。 |
| `static_theme.rs` | 静态图像、热点、惰性加载、主题更新与失败恢复。 |

### Unit Test

| 源码测试模块 | 主要测试职责 |
|---|---|
| `src/presentation/diagnostics/tests.rs` | RGBA 图集有效性、资产身份、原始来源及诊断关联。 |
