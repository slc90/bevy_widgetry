# Tray

`bevy_widgetry_tray` 提供 Windows 64 位常驻通知区域图标，经 Facade 可选 `tray` Feature 导出为 `bevy_widgetry::tray`，不在默认 Feature 集合中。

宿主在安装 `WinitPlugin` 后显式添加 `WidgetryTrayPlugin::new(include_bytes!("app.ico"), "提示文字")?`。构造时仅接受编译期嵌入的 ICO 数据，使用 Image ICO 解码器选择图层并转换为 RGBA，不读取运行时文件。Tooltip 与图标在创建时固定。

插件在主线程 Startup 创建 tray-icon 实例，以 NonSend 所有者保活，与任何窗口 Entity、可见性或最小化状态无关。收到 AppExit 或 App 释放时销毁实例。未启用 Feature 或未安装插件不会创建图标。初始化失败记录 Widgetry 诊断并以 Error Severity 传播，不保留半初始化实例。

宿主通过 `MessageReader<WidgetryTrayClick>` 接收本实例的托盘左键单击。只转发 `Click + Left + Up`，按下、右键、悬停及第三方 DoubleClick 不产生额外通知。回调先将点击排队，再发 `WinitUserEvent::WakeUp` 唤醒 desktop_app 反应式循环，PreUpdate 转成 ECS 消息；所有窗口隐藏时仍有效。库不操作窗口，宿主自行决定如何响应。

插件占用 tray-icon 的进程级事件处理入口，通过 Weak 路由按图标 ID 分发，丢弃未使用的原生事件。实例销毁先撤销路由，再释放图标和桥接发送端；全局回调不捕获 App/World 或过期发送端，支持后续实例重新注册。宿主不得先注册其他 tray-icon 回调或消费其全局 Receiver。

每实例点击队列容量 64，溢出拒绝新点击；每次被拒绝的点击与每次唤醒失败都是独立操作失败，源头创建 `TrayFailure = WidgetryFailure<TrayError>` 并记录一次 ERROR。独立的无界诊断通道不等待主线程消费、不因点击过载而丢弃失败，实例收件箱保留诊断直到 PreUpdate 批量传播。每轮只取开始消费时的诊断数量，避免并发生产者使更新无法结束。AppExit 释放实例后再次递送当前剩余诊断。正常消费不会重复通知，无订阅者也不影响图标存在。不通过定时轮询或后台常驻任务唤醒。

普通构造 API 返回领域失败，保留 ICO 解码、RGBA 转换、原生创建及注册确认的真实 source、图标尺寸和原生 ID；只有 Startup / PreUpdate / 退出 System 才转换为 Error Severity。回调唤醒失败保留底层错误对象及原始 origin，跨线程递送不重建 ID 或回溯。路由锁中毒属于独立领域错误：恢复仍然有效的 HashMap，实际完成注册、查找或撤销后清除中毒标记并记录处理 DEBUG，不把 Drop 当成 Bevy 执行边界。失效 Weak、非本实例事件和关闭后的点击过滤保持静默。

单元回归覆盖连续唤醒失败、后台点击过载不阻塞、真实 Startup 与 PreUpdate 宿主接收、原始 source / origin、路由中毒恢复及正常点击过滤。原生通知区域输入不由这些测试代替。

不提供菜单、气泡通知、双击 API、动态图标或 Tooltip 更新。宿主拥有其图标素材，窗口、任务栏与 EXE 图标独立配置。

原生图标、Tooltip 及退出后的移除必须在真实 Windows 通知区域验证；Headless 测试和 BRP Custom Pointer 不能代替该验证。
