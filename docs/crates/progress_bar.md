# progress_bar

ProgressBar 是只读展示控件，保存应用已经掌握的任务进度，不参与任务计算和输入处理。

通过 BSN `@WidgetryProgressBar { @state: ... }` 构造，省略时初始化为 `WidgetryProgressState::Determinate { value: 0.0 }`。可直接使用非零初值或 `Indeterminate`。Props 只消费一次，随后由私有持久 Component 保存状态；不保留旧的确定值供跨模式恢复。

安装 `WidgetryProgressBarPlugin` 注册反射类型。Facade 默认启用 `progress_bar` Feature，通过同名模块导出 Widget、Props、状态枚举和 Plugin；可关闭默认集合后单独启用。

`WidgetryProgressBar::state(world, entity)` 读回当前真实状态。`set_state_in_world` 同步校验并提交，返回 `Result<bool, BevyError>`：状态改变为 `Ok(true)`，合法同值为 `Ok(false)`，无效输入或目标为错误。`set_state` 通过 Commands 排队，执行时才校验和提交，错误交给宿主 Handler。

确定值必须有限且属于 `[0.0, 1.0]`，允许增加、减小、重置以及双向切换模式。NaN、无穷、越界、无效 Entity、非目标 Widget 或缺少运行时状态均拒绝并报告 Error，不 Clamp；失败保留原状态。外部不直接替换或删除内部状态 Component。

进度作为应用提供的展示输入，不发送 `ValueChange` 或其他业务变化通知；同值更新不标记状态改变。外部文字应读取已提交状态。Scene 构造失败遵循 Core 的 Scene 错误处理路径，清理新建预约实体。

视觉层采用水平从左向右的圆角 Track / Fill，外框保留完整 1px Border，内层裁剪确保填充不会覆盖边框。默认高 `12px`、宽度填满父级，Props `height` 接受有限正数，BSN 根 Node 可覆盖尺寸。几何使用内层可用宽度的比例，响应父级和高度变化。0% 无填充，100% 填满内部区域。

`WidgetryTheme::progress_bar` 为四套 Theme 提供 Track、Fill、Border 配色，运行时切换原位刷新。Props `colors` 和 `WidgetryProgressBarColorOverrides` 的 get / set_in_world / set / clear_in_world / clear 提供局部覆盖；未覆盖部分继续随 Theme 更新，无效颜色按 Core 契约拒绝。Plugin 自动安装共享 UI 能力，Headless 状态 API 仍可独立查询。

确定值更新后，真实进度立即提交，填充从当前可见比例以约 `150ms` Cubic Ease-Out 追赶目标，允许倒退和重置。连续更新从中途可见位置接续；初次显示非零进度直接投影。同值不重启，Theme 和尺寸变化不重播，到达目标后停止持续重绘。空闲后的首次更新不把之前的空闲时间计入新动画。

不确定模式以约 `1.2s` 周期、约轨道 30% 长度的高亮段向右循环，运动区域始终与完整内侧圆角画布相交，边框和轨道静止。循环相位只属于视觉层，不保留旧的业务进度。确定→不确定立即取消插值并开始循环；不确定→确定立即显示新真实比例，不伪造从零增长；重复不确定设置不重启相位。退出后不残留循环重绘。尺寸、Theme 和局部覆盖变化不重置循环。无内置 Text、Thumb、端点刻线或圆点，不接受指针与键盘输入，也无 Hover / Pressed / Focus 外观态。
