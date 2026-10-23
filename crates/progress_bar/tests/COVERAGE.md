# ProgressBar Coverage Model

`headless.rs` 从 crate 外部通过真实 BSN、World 和 Commands 验证状态与初始化。

- States：确定进度比例 / 不确定模式；默认、非零初值、直接不确定初始化。
- Stimuli：同步设置、排队设置、BSN 新建和应用到已有实体；入队后目标销毁、排队构造后设置。
- Guards：有限且属于 `[0, 1]`；有效目标及必需运行时状态。NaN、正负无穷、上下越界和无效目标均明确拒绝。
- Transitions：增加、倒退、归零、完成后减小、双向模式切换、同值 No-op。
- Invariants：读回已提交状态；失败不修改；无 ValueChange 业务事件；调用方 Name 保留；初始化失败清理新建根；延迟失败进入宿主 Handler 且为 Error Severity，并只记录一次诊断。

私有状态校验与无效结构、同值不标脏由 `progress.rs` Unit Test 补充。不涉及动画、用户输入或 Gallery。

`style.rs` 验证真实 UI Layout 的 0/65/100% 内侧填充、12px 默认高度、父级宽度和根高度变更、Props 高度校验、调用方 Name、无 Text/TabIndex；四套 Theme 原位刷新、局部部分覆盖及清除、颜色错误与延迟错误传播，均保留真实进度。

`animation.rs` Integration Test 覆盖即时状态与 150ms 视觉追赶、倒退/重置、连续目标从可见位置重定位、每帧新目标仍持续推进、合法同值不重启、过渡中尺寸/Theme/局部覆盖、空闲首帧与静止后停止重绘；Unit Test 检查 Ease-Out 减速和精确收敛。

循环动画覆盖初始直接可见、向右移动与跨周期、双向模式即时交接、同值保持相位、退出停止重绘、四套 Theme / UI Scale / 局部覆盖及尺寸变化；状态始终为调用方最后提交值。Unit Test 检查窗口有界和跨模式丢弃旧计时。
