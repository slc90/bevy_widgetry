# Splash

`SplashPlugin` 提供独立原生 Splash 窗口、固定几何、首帧安全显示与主窗交接。仅安装插件不创建 Splash、不隐藏主窗口。Facade 的 `splash` Feature 默认关闭，经 `splash` 模块导出；它不自动启用 `ui_scale` 或 `window` Feature。Gallery 显式开启 Feature，按本次运行的启动选择装配插件、独立 Window 与应用自有 Camera/图片；主窗在启用时从原生创建起隐藏，不启用时正常独立显示。

宿主调用 `Splash::start(&mut World, main, Window)`，传入已存在的普通主窗口 Entity 及完整 Window 基础配置，返回独立 Splash Window Entity。输入物理宽高非零；错误不创建半成品或修改主窗口。不能将已有 Splash 当成主窗口。插件不创建 Camera、UI、图片、动画或默认内容；宿主自行绑定渲染目标并管理内容生命周期。

创建时强制初始隐藏、无原生边框、不可 Resize、不可最大化、Windowed、DPI Override `Some(1.0)`，保留传入的物理尺寸；原子 Bundle 中带 Core 的 `WindowScaleExcluded`。普通窗口的全局 Scale 与系统 DPI 改变不重定义该尺寸。宿主直接改写这些托管 Window 属性时，Geometry 阶段恢复约束；首次显示由库负责，不直接设置 Splash 的 `Window.visible=true` 绕过握手。

窗口以主窗口**中心所在的显示器**居中，使用整块显示器物理边界，支持负坐标。原生主窗口存在时读取实际 outer position/size；创建前按明确的 At、Centered Entity/Primary 或 Winit 原生 Index 定位，不将 ECS 迭代顺序当成 Monitor Index。主窗口中心位于显示器间隙时选择距离最近的显示器。位置、尺寸约束和 DPI 换算在 PostUpdate 的 `SplashSystems::Geometry`、Camera 更新前收敛；静态帧不重复标记 Window 已变化，实际尺寸/比例修正提前发布 Bevy Window 消息以协调 Camera。

Monitor 元数据缺失，或 Automatic/Current 的原生主窗口位置尚不可知时正常等待；Splash 保持初始隐藏，不伪造居中坐标。`SplashWindow` 只读接口提供主窗口、目标物理尺寸和当前是否已定位。目标尺寸大于显示器时返回 `SplashError::TooLarge`，不自动缩放到工作区。`SplashWindow` 及其排除标记属于内部托管结构，不支持外部移除/替换来改变策略。

主窗口最小化时，不使用停放在屏幕外的原生矩形选显示器，而保留此前已确认的显示器身份，并按其当前边界/DPI重新居中。首次启动时主窗口已最小化、或已确认显示器元数据消失时正常等待；主窗口恢复后重新按实际中心定位。真实几何失败清除当前定位有效性与旧确认信息，不将陈旧位置当成恢复依据。

领域 API 返回 `SplashFailure = WidgetryFailure<SplashError>`，保留分类、主窗口/尺寸/显示器字段、来源及 Backtrace；实际原生位置查询的 `NotSupportedError` 作为 source 保留。ECS 边界以 Error Severity 传播，多个窗口错误聚合，同一持续故障保留身份；等待元数据不记恢复，实际完成定位后才清除故障。库不安装宿主 Handler，不决定恢复、退出或显示主窗口。

宿主确认自身内容可正确绘制后，调用同步 `Splash::ready(world, splash)` 或触发排队执行的 `SplashReady { splash }`。同步接口返回是否首次提交内容就绪，重复信号合法且不再次显示；失效同步请求返回 `UnknownSplash`，Event 对不属于当前会话的目标正常过滤。`Splash::phase` 只读返回 `WaitingForContent`、`WaitingForFrame` 或 `Visible`，不存在的会话为 `None`。等待元数据或缺少 RenderApp/Camera 都是正常等待，Ready 不替代真实渲染。

`SplashSystems::Presentation` 在几何收敛后为当前 Window Entity、物理尺寸和比例建立首帧 Ticket。RenderApp 检查同一目标的可写 Camera/ViewTarget、该帧 Swap Chain 获取及 Render 后消费，再执行 Windows `DwmFlush` 同步。库不检查特定图片、UI 批次或内容类型；`SplashReady` 仍由宿主承担内容语义。匹配当前 Ticket 且已定位后才在主线程显示原生窗口、提交 ECS 可见状态，并发送一次 `SplashShown { splash, main }`。订阅者能读取已经提交的 `Visible` 状态。几何快照过期或会话销毁会取消旧 Ticket，旧结果不能解锁新请求。

隐藏窗口可在本项目 Windows/DX12 链路完成首帧渲染；握手等待期间通过 Winit WakeUp 推进桌面 Reactive 模式，即使所有窗口都隐藏也能闭环。显示完成或呈现失败后停止该请求的主动唤醒，不安装全局持续刷新。Splash 与 Window 插件幂等安装 Core 共享的 DX12 多窗口 Camera 提交和无 Camera 初始 Present 保护；Splash 不依赖 Window 控件。透明 DX12 设备初始化仍由宿主选择。

宿主通过同步 `Splash::finished` / `Splash::main_ready` 或排队的 `SplashFinished { splash }` / `MainWindowReady { splash }` 提交播放结束和主业务内容就绪。三类信号相互独立，支持任意顺序与合法重复；主窗关联取自会话，事件的 `splash` 指返回的会话窗口 Entity。主窗必须在原生创建前由宿主设为隐藏，插件不替宿主事后隐藏已经可见的主窗。未启用会话的普通窗口不受影响。

Splash 显示后收到 Finished，但未收到主内容 Ready 时进入 `WaitingForMainContent`，继续保留宿主画面；两者收到后进入 `WaitingForMainFrame`，为主窗当前物理尺寸/比例建立独立帧 Ticket。主窗先 Ready 不提前结束动画。宿主负责保持 Splash 末帧可渲染，系统正常曝光/Redraw仍可重绘；库不生成末帧、不添加加载超时。

主窗隐藏业务帧通过同一呈现检查后，库在 Windows 主线程 Show 并请求 Focus，进入 `HandingOff`。为避免 DWM Show 动画在交接中露出后景，交接期间临时设置 `DWMWA_TRANSITIONS_FORCEDISABLED=true`，Show 后 DwmFlush 并设回 false，恢复系统常规动画。该 DWM 属性不可读取，启动交接托管这项属性，宿主不应同时写入自定义值。交接再创建新 Ticket，确认主窗在显示后实际完成新帧呈现，并核查原生/ECS可见性，才隐藏及销毁 Splash Window、释放会话并触发一次 `SplashCompleted { splash, main }`。结束后 `Splash::phase` 返回 `None`；原生 Focus 请求仍受 Windows 前台策略约束。主窗帧快照在 Core 共享的全局缩放 Sync 标记之后取得，缩放中间尺寸不能被误当成交接凭据。

`Splash::cancel(world, splash)` 是与完成交接不同的终止入口：取消在途 Ticket，隐藏/销毁 Splash Window 并删除会话，不打开、聚焦或销毁主窗，也不发 Completed。窗口的 Camera 与 Content 仍由宿主拥有；正常 Completed 后或主动取消时，宿主自行清理自身资源，库不按 RenderTarget 搜索并越权删除其他对象。未知同步请求返回领域失败，已结束会话收到排队信号时正常过滤。

呈现同步真实失败以 `SplashError::Presentation` 保留 HRESULT 原因；原生句柄查找和 DWM 动画设置失败分别保留 Win32/HRESULT source。Render 线程创建的原始诊断共享至主线程 ECS 边界，多个请求及 Show 同步/动画恢复的独立失败聚合，失败会话停止推进但保留当前阶段。该诊断不是已处理；宿主决定保留、取消、降级或退出。Show 后同步或动画恢复失败时，主窗可能已经可见，库保留真实 Window 状态及 Splash，不擅自回滚显示或销毁会话。

`cargo run -p bevy_widgetry_splash --example splash_windows` 用宿主 Camera ClearColor 提供内容；S 声明 Splash 内容就绪，F 声明播放结束，M 声明主内容就绪，C 取消；1/2/3 设置普通窗口的 1.0/1.25/2.0，Splash 保持480×270。添加 `-- --hidden-main` 可从原生创建时隐藏主窗，配合 BRP 输入验证全隐藏 Reactive 启动与交接。示例使用宿主 Window 的 DX12 呈现配置，仅安装 SplashPlugin，不额外安装 Window 控件插件；示例自行在 Completed/取消时清理它的 Splash Camera。
