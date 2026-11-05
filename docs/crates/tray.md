# Tray

`bevy_widgetry_tray` 提供 Windows 64 位常驻通知区域图标，经 Facade 可选 `tray` Feature 导出为 `bevy_widgetry::tray`，不在默认 Feature 集合中。

宿主在安装 `WinitPlugin` 后显式添加 `WidgetryTrayPlugin::new(include_bytes!("app.ico"), "提示文字")?`。构造时仅接受编译期嵌入的 ICO 数据，使用 Image ICO 解码器选择图层并转换为 RGBA，不读取运行时文件。Tooltip 与图标在创建时固定。

插件在主线程 Startup 创建 tray-icon 实例，以 NonSend 所有者保活，与任何窗口 Entity、可见性或最小化状态无关。收到 AppExit 或 App 释放时销毁实例。未启用 Feature 或未安装插件不会创建图标。初始化失败记录 Widgetry 诊断并以 Error Severity 传播，不保留半初始化实例。

宿主通过 `MessageReader<WidgetryTrayClick>` 接收本实例的托盘左键单击。只转发 `Click + Left + Up`，按下、右键、悬停及第三方 DoubleClick 不产生额外通知。回调先将点击排队，再发 `WinitUserEvent::WakeUp` 唤醒 desktop_app 反应式循环，PreUpdate 转成 ECS 消息；所有窗口隐藏时仍有效。库不操作窗口，宿主自行决定如何响应。

插件占用 tray-icon 及其 menu 模块的进程级事件处理入口，通过 Weak 路由按图标 ID 或内部原生 MenuId 分发，丢弃未使用的原生事件。实例销毁先撤销路由，再释放菜单、图标和桥接发送端；全局回调不捕获 App/World 或过期发送端，支持后续实例重新注册。宿主不得先注册其他 tray-icon / MenuEvent 回调或消费其全局 Receiver。

每实例点击队列容量 64，溢出拒绝新点击；每次被拒绝的点击与每次唤醒失败都是独立操作失败，源头创建 `TrayFailure = WidgetryFailure<TrayError>` 并记录一次 ERROR。独立的无界诊断通道不等待主线程消费、不因点击过载而丢弃失败，实例收件箱保留诊断直到 PreUpdate 批量传播。每轮只取开始消费时的诊断数量，避免并发生产者使更新无法结束。AppExit 释放实例后再次递送当前剩余诊断。正常消费不会重复通知，无订阅者也不影响图标存在。不通过定时轮询或后台常驻任务唤醒。

普通构造与菜单编辑 API 返回领域失败，保留 ICO 解码、RGBA 转换、原生创建及注册确认的真实 source、图标尺寸和原生 ID；到 Startup / PreUpdate / Last 同步及退出 System 才转换为 Error Severity。回调唤醒失败保留底层错误对象及原始 origin，跨线程递送不重建 ID 或回溯。路由锁中毒属于独立领域错误：恢复仍然有效的 HashMap，实际完成注册、查找或撤销后清除中毒标记并记录处理 DEBUG，不把 Drop 当成 Bevy 执行边界。失效 Weak、非本实例事件和关闭后的点击过滤保持静默。

单元回归覆盖连续唤醒失败、后台点击过载不阻塞、真实 Startup 与 PreUpdate 宿主接收、原始 source / origin、路由中毒恢复及正常点击过滤。原生通知区域输入不由这些测试代替。

宿主可在 Startup 前或运行期间插入 `WidgetryTrayMenu::new(Vec<TrayMenuEntry>)?`，配置普通项、复选项、分隔线和嵌套子菜单。`TrayMenuEntry::item(id, text)` 和 `TrayMenuEntry::check(id, text, checked)` 的业务 ID 在整个菜单树内必须唯一；构造失败保留重复 ID 诊断，原生 ID 则由库内部独立生成，不泄露到宿主 API。各可交互节点和子菜单支持 `enabled`，构造时可用 `with_enabled(false)`；禁用子菜单会阻止其后代激活。未配置 Resource、清空菜单或移除 Resource 均保持只有图标的状态。

`entries()` 和 `entry(id)` 提供只读查询；`set_text`、`set_enabled`、`set_checked` 按稳定 ID 修改可交互项。未知 ID 和在普通项上设置勾选状态返回领域失败，不修改资源。`replace` 替换整棵树；`edit` 把树副本交给宿主闭包，允许按父子关系增删、插入、替换、排序，以及修改子菜单标题和属性，整棵树校验成功后才提交。编辑失败保留原状态，合法同值编辑返回 `false`。

菜单在主线程 `Last` 的 `WidgetryTraySystems::Sync` 中同步，早于退出清理。普通 Update / PostUpdate 的修改在同一轮生效；宿主如在 Last 写入，应显式排在该 SystemSet 之前。连续修改仅投影最终期望状态。库对比成功应用的快照，内容未变化时不重复构造原生菜单；实际变化时先构造和验证完整替代菜单，随后撤销旧 ID 路由、替换原生菜单并注册新 ID，不重建托盘图标。删除或禁用已排队激活的业务项（包括禁用其祖先）会在 PreUpdate 递送前重新过滤，未同步完成的期望删除同样有效。

Resource 始终是宿主期望状态的唯一来源。复选项原生点击可能立即切换勾选，但消息只报告业务 ID，宿主自行接受或拒绝并修改 `checked`。每次复选项激活都会标记校准，即使消息队列过载或宿主完全没有写 Resource，也会在 Last 从宿主期望状态重建投影；普通项激活不会触发无意义的重建。同步失败不提交新快照，也不替换先前仍有效的菜单和路由，期望 Resource 保持原值；下一次 Resource 写入、不同的期望状态或复选项激活会重试。相同失败请求不会在空闲更新中循环重试和报错，后续成功会关联最近错误 ID 记录处理 DEBUG。无法原样表示的原生标签（例如含 NUL 的文本）作为投影失败报告。

同步不增加计时轮询，也不承诺外部线程修改数据会自行唤醒 Bevy。菜单和点击继续使用既有反应式唤醒。首次附加菜单后，输入过滤器随图标保活；清空菜单时仍保留其原生按下/释放配对，防止旧菜单的遗留释放被当成新的恢复点击。

菜单只由右键展开，左键弹菜单选项显式关闭。宿主通过 `MessageReader<WidgetryTrayMenuClick>` 读取激活项的业务 `id`，子菜单标题和分隔线不产生消息。菜单激活使用独立容量 64 的有界队列，唤醒反应式循环，PreUpdate 转发；队列过载和唤醒失败保留独立诊断。未知、其他实例、已撤销路由的迟到激活（包括已排队尚未转发的激活）静默丢弃。

右键菜单展开后，用左键点击托盘只关闭菜单，不发送恢复消息；下一次独立左键才发送 `WidgetryTrayClick`。Escape、点击外部或选择菜单项后，新点击均正常。私有 Windows FFI 使用 subclass 观察菜单开闭，以主线程 WH_MSGFILTER 观察原生菜单输入；在托盘图标内开始的关闭手势，其按下和释放均在菜单循环内消费，释放时 EndMenu，避免送达 Explorer 后再反向推断异步通知属于哪次手势。其他位置的点击、普通菜单选择和键盘输入保留系统处理。有菜单时，左键释放额外要求对应一次菜单关闭时的原生托盘按下；关闭按下已被消费，即使 Escape 或焦点变化提前结束菜单，遗留的孤立释放也不会恢复窗口。没有固定延迟、全局鼠标监听、后台线程或额外轮询。FFI 的 unsafe 仅限该私有模块，其余 Tray 代码继续禁止 unsafe。

底层菜单库的创建/插入接口并非都报告 Win32 失败，因此完成构造后会从真实 HMENU 读取结构、文字、启用和勾选状态，确认成功才注册路由。原生查询失败保留当次 Win32 查询的真实错误；投影不一致报告独立领域错误，不伪造底层库未返回的错误原因。

不提供气泡通知、双击 API、动态图标或 Tooltip 更新。宿主拥有其图标素材，窗口、任务栏与 EXE 图标独立配置。

原生图标、Tooltip 及退出后的移除必须在真实 Windows 通知区域验证；Headless 测试和 BRP Custom Pointer 不能代替该验证。
