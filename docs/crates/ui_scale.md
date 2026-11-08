# Ui Scale

`WidgetryUiScalePlugin` 将 `WidgetryUiScale` 的应用比例统一应用到普通 Bevy Window。默认值为 1.0，与系统显示器 DPI 独立；宿主通过 `WidgetryUiScale::new` 初始化资源，或在运行期通过 `set` 修改。只接受有限正数，不限制为 Gallery 的四档。

管理器在 Window 加入 ECS 时应用当前比例，已有窗口由 PostUpdate 的 `WidgetryUiScaleSystems::Sync` 接管。宿主的 Update 配置变化在该阶段传播。Sync 排在 Bevy Camera 更新之前，并即时发送原生尺寸/比例消息，使显式 Viewport 在本帧渲染提取前更新；之后 winit 仍可能报告相同变化，窗口消息不是 Widgetry 业务状态通知。插件没有安装时，不改变普通 Window。Facade 的 `ui_scale` Feature 默认关闭，通过同名模块导出插件、资源和错误。

每个窗口保存逻辑尺寸基准和上次请求物理尺寸；应用缩放只改变物理尺寸，用户 Resize 则更新后续缩放的逻辑基准。窗口原有及宿主运行期修改的 Resize Constraints 限制逻辑尺寸，放宽约束可恢复设计尺寸。合法同值 `set` 返回 false，不再次写 Window；资源值是配置，窗口真正几何在同步阶段收敛，不提供独立变化通知。

`WindowScaleExcluded` 定义在 Core，由此 crate 和 Facade 根导出。与 Window 同批创建的标记也有效；管理器完全跳过其 Override、尺寸和策略。给已管理窗口加入标记会丢弃管理状态，保持当前窗口配置，移除标记后以当时窗口的逻辑尺寸重新接管。移除 Window 后重建不会继承旧实例的尺寸基准。

非法比例返回 `UiScaleFailure = WidgetryFailure<UiScaleError>`，保留请求值、源头位置、Backtrace 和唯一身份，源头记录一次 ERROR；拒绝时保留原配置。宿主仅在 ECS 边界将失败 `into_bevy_error()`；插件不安装宿主 Error Handler。

目前提供普通窗口的基础缩放；Windows 工作区夹紧、跨屏系统 DPI 时序和最大化/最小化协调尚未实现。原生像素尺寸至少为 1，超过 u32 像素范围时使用可表示上限。

`cargo run -p bevy_widgetry_ui_scale --example ui_scale_windows` 运行主窗口、普通 Popup、排除窗口。按 1 / 2 / 3 设置 1.0 / 1.25 / 2.0，观察前两个窗口内容逻辑范围保持不变而物理尺寸变化，排除窗口保持原配置；示例启用 BRP。
