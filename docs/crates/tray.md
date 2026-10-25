# Tray

`bevy_widgetry_tray` 提供 Windows 64 位常驻通知区域图标，经 Facade 可选 `tray` Feature 导出为 `bevy_widgetry::tray`，不在默认 Feature 集合中。

宿主在安装 `WinitPlugin` 后显式添加 `WidgetryTrayPlugin::new(include_bytes!("app.ico"), "提示文字")?`。构造时仅接受编译期嵌入的 ICO 数据，使用 Image ICO 解码器选择图层并转换为 RGBA，不读取运行时文件。Tooltip 与图标在创建时固定。

插件在主线程 Startup 创建 tray-icon 实例，以 NonSend 所有者保活，与任何窗口 Entity、可见性或最小化状态无关。收到 AppExit 或 App 释放时销毁实例。未启用 Feature 或未安装插件不会创建图标。初始化失败记录 Widgetry 诊断并以 Error Severity 传播，不保留半初始化实例。

不提供点击通知、菜单、气泡通知、动态图标或 Tooltip 更新。宿主拥有其图标素材，窗口、任务栏与 EXE 图标独立配置。

插件占用 tray-icon 的进程级事件处理入口并丢弃未使用的原生事件，避免第三方默认无界队列持续积压。宿主不得先注册其他 tray-icon 回调或消费其全局 Receiver。

原生图标、Tooltip 及退出后的移除必须在真实 Windows 通知区域验证；Headless 测试和 BRP Custom Pointer 不能代替该验证。
