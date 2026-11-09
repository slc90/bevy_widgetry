# theme 测试覆盖模型

本模型记录当前自动化测试保护的主要行为契约与测试职责，不逐项列举测试用例。

## States
- 当前 Theme／Palette、已提交资源值与外部完整自定义主题。

## Stimuli
- 通过公开 World／Commands API 更新、连续替换、合法同值和延迟执行。

## Guards
- 非法嵌套颜色或 Waveform 调色板必须原子拒绝；缺少运行资源和命令错误明确传播。

## Transitions
- Theme 申请 → 完整校验 → 提交 → 观察通知；合法同值不重复通知。

## Invariants
- Observer 能读取已提交的完整 Palette；失败不修改此前有效主题，亦不发送伪变化事件。
- Preset 对比与转换遵循内建颜色契约；下层诊断与宿主错误保持可定位。

## Couplings
- Palette 内部字段 × 整体原子提交；Theme Command × Bevy Resource／Observer。

## Coverage Map

### Integration Test

| 测试文件 | 主要测试职责 |
|---|---|
| `runtime_theme.rs` | 外部完整 Palette 的同步／排队修改、非法输入与通知。 |

### Unit Test

| 源码测试模块 | 主要测试职责 |
|---|---|
| `src/lib.rs` | 内建 Preset、颜色对比和转换语义。 |
