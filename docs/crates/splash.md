# Splash

`SplashPlugin` 提供独立原生 Splash 窗口的创建与固定几何策略。仅安装插件不创建 Splash、不隐藏主窗口。Facade 的 `splash` Feature 默认关闭，经 `splash` 模块导出；它不自动启用 `ui_scale` 或 `window` Feature。Gallery 目前仅显式开启编译 Feature，仍使用应用原有覆盖层 Splash，尚未接入独立窗口。

宿主调用 `Splash::start(&mut World, main, Window)`，传入已存在的普通主窗口 Entity 及完整 Window 基础配置，返回独立 Splash Window Entity。输入物理宽高非零；错误不创建半成品或修改主窗口。不能将已有 Splash 当成主窗口。插件不创建 Camera、UI、图片、动画或默认内容；宿主自行绑定渲染目标并管理内容生命周期。

创建时强制初始隐藏、无原生边框、不可 Resize、不可最大化、Windowed、DPI Override `Some(1.0)`，保留传入的物理尺寸；原子 Bundle 中带 Core 的 `WindowScaleExcluded`。普通窗口的全局 Scale 与系统 DPI 改变不重定义该尺寸。宿主直接改写这些托管 Window 属性时，Geometry 阶段恢复约束；首次显示由库负责，不直接设置 Splash 的 `Window.visible=true` 绕过握手。

窗口以主窗口**中心所在的显示器**居中，使用整块显示器物理边界，支持负坐标。原生主窗口存在时读取实际 outer position/size；创建前按明确的 At、Centered Entity/Primary 或 Winit 原生 Index 定位，不将 ECS 迭代顺序当成 Monitor Index。主窗口中心位于显示器间隙时选择距离最近的显示器。位置、尺寸约束和 DPI 换算在 PostUpdate 的 `SplashSystems::Geometry`、Camera 更新前收敛；静态帧不重复标记 Window 已变化，实际尺寸/比例修正提前发布 Bevy Window 消息以协调 Camera。

Monitor 元数据缺失，或 Automatic/Current 的原生主窗口位置尚不可知时正常等待；Splash 保持初始隐藏，不伪造居中坐标。`SplashWindow` 只读接口提供主窗口、目标物理尺寸和当前是否已定位。目标尺寸大于显示器时返回 `SplashError::TooLarge`，不自动缩放到工作区。`SplashWindow` 及其排除标记属于内部托管结构，不支持外部移除/替换来改变策略。

主窗口最小化时，不使用停放在屏幕外的原生矩形选显示器，而保留此前已确认的显示器身份，并按其当前边界/DPI重新居中。首次启动时主窗口已最小化、或已确认显示器元数据消失时正常等待；主窗口恢复后重新按实际中心定位。真实几何失败清除当前定位有效性与旧确认信息，不将陈旧位置当成恢复依据。

领域 API 返回 `SplashFailure = WidgetryFailure<SplashError>`，保留分类、主窗口/尺寸/显示器字段、来源及 Backtrace；实际原生位置查询的 `NotSupportedError` 作为 source 保留。ECS 边界以 Error Severity 传播，多个窗口错误聚合，同一持续故障保留身份；等待元数据不记恢复，实际完成定位后才清除故障。库不安装宿主 Handler，不决定恢复、退出或显示主窗口。

宿主确认自身内容可正确绘制后，调用同步 `Splash::ready(world, splash)` 或触发排队执行的 `SplashReady { splash }`。同步接口返回是否首次提交内容就绪，重复信号合法且不再次显示；失效同步请求返回 `UnknownSplash`，Event 对不属于当前会话的目标正常过滤。`Splash::phase` 只读返回 `WaitingForContent`、`WaitingForFrame` 或 `Visible`，不存在的会话为 `None`。等待元数据或缺少 RenderApp/Camera 都是正常等待，Ready 不替代真实渲染。

`SplashSystems::Presentation` 在几何收敛后为当前 Window Entity、物理尺寸和比例建立首帧 Ticket。RenderApp 检查同一目标的可写 Camera/ViewTarget、该帧 Swap Chain 获取及 Render 后消费，再执行 Windows `DwmFlush` 同步。库不检查特定图片、UI 批次或内容类型；`SplashReady` 仍由宿主承担内容语义。匹配当前 Ticket 且已定位后才在主线程显示原生窗口、提交 ECS 可见状态，并发送一次 `SplashShown { splash, main }`。订阅者能读取已经提交的 `Visible` 状态。几何快照过期或会话销毁会取消旧 Ticket，旧结果不能解锁新请求。

隐藏窗口可在本项目 Windows/DX12 链路完成首帧渲染；握手等待期间通过 Winit WakeUp 推进桌面 Reactive 模式，即使所有窗口都隐藏也能闭环。显示完成或呈现失败后停止该请求的主动唤醒，不安装全局持续刷新。Splash 与 Window 插件幂等安装 Core 共享的 DX12 多窗口 Camera 提交和无 Camera 初始 Present 保护；Splash 不依赖 Window 控件。透明 DX12 设备初始化仍由宿主选择。

呈现同步真实失败以 `SplashError::Presentation` 保留 HRESULT 原因。Render 线程创建的原始诊断共享至主线程 ECS 边界，多个请求聚合，失败会话停止推进但保留等待阶段，不自动回退、销毁、退出或显示主窗口。该诊断不是已处理；宿主决定处理策略。当前尚不提供主窗口就绪与交接。

`cargo run -p bevy_widgetry_splash --example splash_windows` 用宿主 Camera ClearColor 提供内容；S 声明内容就绪，1/2/3 设置普通窗口的 1.0/1.25/2.0，Splash 保持480×270。添加 `-- --hidden-main` 可从原生创建时隐藏主窗，配合 BRP 触发 Ready 验证全隐藏 Reactive 启动。示例使用宿主 Window 的 DX12 呈现配置，仅安装 SplashPlugin，不额外安装 Window 控件插件。
