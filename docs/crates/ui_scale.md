# Ui Scale

`WidgetryUiScalePlugin` 将 `WidgetryUiScale` 的应用比例统一应用到普通 Bevy Window。默认值为 1.0，与系统显示器 DPI 独立；宿主通过 `WidgetryUiScale::new` 初始化资源，或在运行期通过 `set` 修改。只接受有限正数，不限制为 Gallery 的四档。

管理器在 Window 加入 ECS 时应用当前比例，已有窗口由 PostUpdate 的 `WidgetryUiScaleSystems::Sync` 接管。宿主的 Update 配置变化在该阶段传播。Sync 排在 Bevy Camera 更新之前，并即时发送原生尺寸/比例消息，使显式 Viewport 在本帧渲染提取前更新；之后 winit 仍可能报告相同变化，窗口消息不是 Widgetry 业务状态通知。插件没有安装时，不改变普通 Window。Facade 的 `ui_scale` Feature 默认关闭，通过同名模块导出插件、资源和错误。

每个窗口保存逻辑尺寸基准和上次请求物理尺寸；应用缩放只改变物理尺寸，用户 Resize 则更新后续缩放的逻辑基准。窗口原有及宿主运行期修改的 Resize Constraints 限制逻辑尺寸，放宽约束可恢复设计尺寸。合法同值 `set` 返回 false，不再次写 Window；资源值是配置，窗口真正几何在同步阶段收敛，不提供独立变化通知。

`WindowScaleExcluded` 定义在 Core，由此 crate 和 Facade 根导出。与 Window 同批创建的标记也有效；管理器完全跳过其 Override、尺寸和策略。给已管理窗口加入标记会暂停管理、保持当前窗口配置，移除标记后以当时窗口的逻辑尺寸重新接管；保留内部约束单位信息，避免重新接管时重复转换。移除 Window 后重建不会继承旧实例的尺寸基准。

非法比例返回 `UiScaleFailure = WidgetryFailure<UiScaleError>`，保留请求值、源头位置、Backtrace 和唯一身份，源头记录一次 ERROR；拒绝时保留原配置。宿主仅在 ECS 边界将失败 `into_bevy_error()`；插件不安装宿主 Error Handler。

Windows 协调在主线程读取真实原生显示器、最大化/最小化状态及边框尺寸，以 winsafe 获取扣除任务栏的工作区。创建前使用显式 Monitor Entity、Winit Monitor Index，或目标矩形的重叠面积/中心距离选择显示器，支持负坐标。运行期优先使用实际原生显示器；宿主新位置请求先按目标显示器协调，避免在 winit 执行移动前被旧工作区夹回。

系统 DPI/显示器元数据与原生 Resize 到达顺序不一致时，保留长期逻辑基准；工作区暂时不足时夹紧物理尺寸和坐标，计入原生边框，再在空间恢复后恢复设计尺寸。有效的显式坐标保持原位。最大化、最小化和非 Windowed 状态暂停物理尺寸/位置/约束写回，仍保存当前应用 Scale，恢复后沿用原设计尺寸。静态状态不重复写 Window；在 App 的更新帧读取原生工作区，以识别任务栏/显示器元数据变化。

初始 Resize Constraints 以应用逻辑尺寸为设计输入；内部保存原始约束，将 Window 上的约束换算为 winit 所需的系统逻辑像素，并受可用工作区限制。运行期宿主改变某一约束字段时，该新值作为该字段的应用逻辑约束，其他字段保留原设计单位。尚未创建 Native Window、没有 Monitor 元数据是合法等待；真实工作区查询失败以 `UiScaleError::WorkArea` 保留窗口、坐标及 winsafe 原始 source，持续同一故障复用诊断身份并只首次传播，独立窗口错误聚合到 ECS Handler。库不替宿主决定恢复或退出策略。原生像素尺寸至少为 1，超过 u32 像素范围时使用可表示上限。

`cargo run -p bevy_widgetry_ui_scale --example ui_scale_windows` 运行主窗口、普通 Popup、排除窗口。按 1 / 2 / 3 设置 1.0 / 1.25 / 2.0，观察前两个窗口内容逻辑范围保持不变而物理尺寸变化，排除窗口保持原配置；M / I / R 对主窗口发出原生最大化 / 最小化 / 恢复请求。示例启用 BRP。

当前 Bevy 0.20 / DX12 实机验证中，示例最小化时会触发 `ResizeBuffers failed / window is in use` 并退出；不安装缩放插件也复现。最小化的暂停/恢复几何契约由 Headless 测试保护，原生最小化恢复的端到端验收尚受该渲染故障阻碍。
