# Splash 覆盖模型

- `window.rs`：插件未开启会话时不影响主窗口；显式启动、独立 Window 身份、初始隐藏、固定物理尺寸/Override1.0、不可 Resize/无边框及全局缩放排除。
- 几何状态包括等待 Monitor 元数据与已定位；覆盖负坐标、主窗口隐藏、以主窗口中心而非主显示器/最大重叠选择显示器、显示器 Entity 定位、DPI/外部几何改写后的固定尺寸修复、静态同帧不通知 Window 消费者。Index 依赖 Winit 真实索引，不使用 ECS 顺序模拟。
- 输入边界包括未安装 Plugin、失效主窗口、零物理尺寸和目标超过显示器；失败不创建半成品、不改变主窗口。领域身份与 ECS Severity/Error Handler、多个窗口故障聚合、持续错误去重、元数据等待不误报恢复由错误测试保护。
- Headless 只验证策略及 ECS 组合；原生固定大小、位置、Windows 样式和真实 DPI 由任务级 DX12/BRP/只读原生状态另行核查，不冒充原生手势证据。
- `geometry.rs` 单元测试保护最小化时的已确认显示器保留、显示器移动后的重新居中、首次无确认和元数据消失时等待；原生停放矩形被排除的路径另有任务级复现证据。
- `window.rs` 的首帧契约覆盖独立会话的内容等待→帧等待、同步 Ready 与排队 Event、重复信号、未知目标过滤、失效同步请求、缺失元数据/RenderApp时不显示、不改变主窗、Despawn清理及不提前通知。真正可见路径由 Windows/DX12 BRP 验证，Headless 不伪造 Present 成功。
- `presentation.rs` 单元测试覆盖内容和当前窗口/尺寸/比例帧双门槛、旧Ticket取消与过期成功隔离、显示一次，以及呈现错误停止依赖推进。真实Extract参数验证Presentation同步后到达的Ready即使尚无Ticket也请求下一次更新，无会话时停止唤醒。两个工作线程诊断通过共享原始对象进入真实ECS Handler一次聚合，保留HRESULT source、origin、Severity与源头一次ERROR；纯传播不记handled。
- 主窗交接：单元测试确定覆盖三信号的六种到达顺序、重复、Finished保持末帧等待主Ready、主Ready先到不跳过播放、主窗过期帧与比例快照隔离、Show后重新取帧、不凭隐藏帧直接结束HandingOff，以及错误阻止交接。ECS组合验证主窗帧快照读取同帧已收敛的全局Scale。
- `window.rs` 验证没有实际帧时三信号均到达仍不能显示主窗，取消后不可重入、不发Completed、不改变主窗及其他会话、保留宿主Camera/内容。`native.rs` 验证Show失败仍恢复常规动画且不执行依赖的Flush，同时保留Flush与恢复的双故障原始身份/source。真实DWM Show/Focus和像素时序由Windows/DX12专项证据覆盖。
- 三种排队信号均覆盖真实 `trigger → cancel → flush` 组合：命令执行时会话已结束则过滤，无伪错误或重入；同步请求同一失效目标仍返回 `UnknownSplash`。
