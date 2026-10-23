# progress_bar

ProgressBar 是只读展示控件，保存应用已经掌握的任务进度，不参与任务计算和输入处理。

通过 BSN `@WidgetryProgressBar { @state: ... }` 构造，省略时初始化为 `WidgetryProgressState::Determinate { value: 0.0 }`。可直接使用非零初值或 `Indeterminate`。Props 只消费一次，随后由私有持久 Component 保存状态；不保留旧的确定值供跨模式恢复。

安装 `WidgetryProgressBarPlugin` 注册反射类型。Facade 默认启用 `progress_bar` Feature，通过同名模块导出 Widget、Props、状态枚举和 Plugin；可关闭默认集合后单独启用。

`WidgetryProgressBar::state(world, entity)` 读回当前真实状态。`set_state_in_world` 同步校验并提交，返回 `Result<bool, BevyError>`：状态改变为 `Ok(true)`，合法同值为 `Ok(false)`，无效输入或目标为错误。`set_state` 通过 Commands 排队，执行时才校验和提交，错误交给宿主 Handler。

确定值必须有限且属于 `[0.0, 1.0]`，允许增加、减小、重置以及双向切换模式。NaN、无穷、越界、无效 Entity、非目标 Widget 或缺少运行时状态均拒绝并报告 Error，不 Clamp；失败保留原状态。外部不直接替换或删除内部状态 Component。

进度作为应用提供的展示输入，不发送 `ValueChange` 或其他业务变化通知；同值更新不标记状态改变。外部文字应读取已提交状态。Scene 构造失败遵循 Core 的 Scene 错误处理路径，清理新建预约实体。

当前只交付 Headless 状态和程序化 API，尚无轨道、填充、主题样式或动画，也不内置 Text、Thumb、指针与键盘输入。
