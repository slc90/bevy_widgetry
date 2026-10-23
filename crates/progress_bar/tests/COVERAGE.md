# ProgressBar Coverage Model

`headless.rs` 从 crate 外部通过真实 BSN、World 和 Commands 验证状态与初始化。

- States：确定进度比例 / 不确定模式；默认、非零初值、直接不确定初始化。
- Stimuli：同步设置、排队设置、BSN 新建和应用到已有实体；入队后目标销毁、排队构造后设置。
- Guards：有限且属于 `[0, 1]`；有效目标及必需运行时状态。NaN、正负无穷、上下越界和无效目标均明确拒绝。
- Transitions：增加、倒退、归零、完成后减小、双向模式切换、同值 No-op。
- Invariants：读回已提交状态；失败不修改；无 ValueChange 业务事件；调用方 Name 保留；初始化失败清理新建根；延迟失败进入宿主 Handler 且为 Error Severity，并只记录一次诊断。

私有状态校验与无效结构、同值不标脏由 `progress.rs` Unit Test 补充。尚不涉及样式、布局、动画、用户输入或 Gallery。
