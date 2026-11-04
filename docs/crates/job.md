# Job

`bevy_widgetry_job` 是 ECS 无关的实例级任务控制核心。`JobManager` 可克隆并跨线程共享同一注册表，不使用进程级业务单例，也不安装任务池或日志订阅器。宿主需先初始化 Bevy 的 `IoTaskPool` 和 `AsyncComputeTaskPool`。

`spawn_io::<T, _, _>` 与 `spawn_cpu` 接受 `FnOnce(JobContext<T>) -> Future<Output = Result<(), JobFailure>>`，分别交给 Bevy 两类后台任务池，返回 `(JobId, JobReceiver<T>)`。首个 Receiver 与 Pending 记录先于执行体建立。`JobContext::send(T).await` 通过 `async-broadcast 0.7.2` 广播 `Arc<T>`，不要求 T: Clone；业务可自行定义 Batch、Progress 等结果项。

`JobId` 在每个 Manager 内从 1 严格递增，移除后也不复用，耗尽时拒绝提交。可选 `JobOptions::label` 保存业务标识。任务开始进入 Running；正常返回为 Completed，顶层错误或可捕捉 panic 为 Failed。仅业务顶层返回决定失败，业务已经处理的局部错误不影响状态。进程 abort 不保证转换为 Failed。

Manager 持有执行 Task，查询或接收时收割已完成 Task 后提交不可逆终态，`status` 提供分类、状态及最终错误。接收 API 返回 `Result<JobRead<T>, ReceiveFailure>`，`try_recv` 的 Empty 与 Finished 后的 Closed 都是正常状态；`recv().await` 等待下一条 Item 或 Finished，`recv_blocking` 仅供普通同步线程使用，不能用于 ECS 主线程或池工作线程。每份 Receiver 按序排空自己的 Item 后，恰好交付一次 Finished，之后 Closed。内部意外 Overflowed 会记录错误并返回，不作为正常丢包处理。Receiver 保持 Manager 存活，移除记录不会破坏已获得的未读结果。

Job 的实际生命周期通过 `bevy_widgetry` Target 的 DEBUG 记录，包含 `job_id`、`kind`、可用 `label`、`state` 和 `stage`。登记只表示 Pending 记录已建立并等待执行，不保证 OS 线程创建成功；创建失败仍返回原诊断并删除预留记录，不会虚构 Running 或终态。实际进入 Running、首次取消请求、执行体结束和真实收割后的 Completed/Canceled/Failed 各记录一次，重复查询、轮询和取消不会重复记录。工作体结束不代表专属线程回收；`joined` 事件只在 OS 线程退出及真实 Join 之后产生，随后收割才发布终态。失败终态 DEBUG 不重播源头错误或 Backtrace。

提交与控制失败分别使用 `SubmitFailure`、`JobAccessFailure`，发送与接收的真实故障使用 `SendFailure`、`ReceiveFailure`；它们均是携带领域分类的 `WidgetryFailure`。调用方通过 `kind()` 匹配分类，不比较诊断本身。提交错误保留请求分类、label、容量及可用的 JobId，线程创建保留原始 `io::Error`；订阅类型不匹配保留预期与实际 Rust 类型名，阶段拒绝保留状态快照。源头只创建一次 ERROR，传播保留原诊断身份；实际停止操作、交付失败回复等处理后调用 `record_handled`，未处理错误显式 `into_bevy_error()` 进入宿主边界。

`status`、`outcome`、`cancel`、`remove` 对未知 ID 报真实访问错误。合法查询使用 `find_status -> Option<JobStatus>`；允许记录已被移除的清理使用原子 `cancel_if_present` 或 `remove_if_present`，不存在时返回 None 且无诊断。`remove_if_present` 对仍活动的记录仍返回 NotFinished，不能用于忽略未完成任务。正常 Empty、Closed、取消和等待背压不创建诊断；Closed/Overflowed 发送及接收丢失才是错误。

默认缓冲容量为 32 个结果项，通过 `JobManager::with_buffer_capacity` 设置实例默认值，`JobOptions::buffer_capacity` 按任务覆盖，零容量拒绝。容量按条目数计，不按业务 Batch 大小或字节计；最慢活跃订阅者产生背压，不覆盖未读数据。Finished 独立保存，不占数据容量，缓冲满不阻止任务终结。业务负责限制单条负载及已持有 Arc 的内存。

`subscribe::<T>(id)` 仅在 Pending/Running 接纳新订阅，类型不匹配返回 TypeMismatch，Stopping/终态返回 NotSubscribable。状态检查和订阅注册使用同一同步边界。新订阅只接收注册后的数据，不回放历史，不公开 Receiver 的 Clone。Drop 立即释放该订阅的未读引用。Manager 保留 InactiveReceiver，无活跃订阅时发送正常丢弃且不等待，后来仍可订阅未来数据。

记录持续保留，直到显式 `remove`。活动记录返回携带当前完整状态的 `NotFinished`，未知或已删除记录返回 `NotFound`。移除不取消任务。Future 必须等待自身业务操作真正结束，不得通过 detach 让 Job 提前完成。

`cancel(id)` 是严格取消申请入口，将 Pending/Running 改为 Stopping，返回 CancelRequested；重复取消 Stopping 返回 AlreadyStopping，终态返回 AlreadyFinished，未知 ID 返回 NotFound。终态和原错误不会被改写。开始执行前已取消的 Future 不再调用业务闭包，待执行句柄收割后进入 Canceled。

已经开始的执行体通过 `JobContext::is_cancelled` 或 `cancelled().await` 观察请求，在决定停止并完成必要清理后调用 `acknowledge_cancel()`，再返回 Ok。确认仅在执行期内且已有取消请求时返回 true，本身不会发布终态。真实退出并收割后，确认取消的 Ok 为 Canceled，未确认取消的 Ok 仍为 Completed；Err/panic 始终 Failed。单独读取取消标志或 send 返回 Canceled 都不构成取消确认，清理期间保持 Stopping。

`send` 优先检查取消并同时等待取消信号/缓冲空间，满缓冲可以由 cancel 主动唤醒，返回 `Ok(SendStatus::Canceled)`。已经发送的 Item 仍供 Receiver 排空；最终 Finished 与注册表裁决一致。正常交付返回 `SendStatus::Sent`，取消状态不构成顶层 Err，也不隐式确认取消。Manager 仅保证自身取消等待与发送背压可唤醒，不能中断任意第三方系统调用。

`spawn_thread::<T, _>` 接受返回 `Result<(), JobFailure>` 的同步闭包，用 `std::thread::Builder` 创建专属命名线程。`JobOptions::label` 同时作为线程名，无标识时使用带 JobId 的默认名称；名称包含空字符或 OS 创建失败会返回 ThreadSpawn，不留下活动记录，已分配 ID 不复用。线程收割需要已初始化 IoTaskPool。

线程与异步入口共享同一 Context、广播、取消确认及终态裁决。`send_blocking` 用于普通专属线程，仍可被取消唤醒，不能用于 ECS 或池工作线程。同步设备等待应由业务自行提供可中断路径。线程闭包退出仅触发收割通知；后台 Future 通过 win-custody 的安全 AsHandle 接口，以零超时查询 Windows 线程句柄，只在线程真实退出（包括 TLS 析构结束）后执行 Join，查询/接收收割该 Future 后公布终态。查询期间异步让出 I/O 工作线程，避免 TLS 清理等待同一个池时死锁。内部 OS 查询失败保留原 `io::Error`，保持句柄并重试；真实 Join 后记录恢复措施，并将查询失败与业务失败的各份诊断一同保存在 Failed 报告中。主线程不会等待活动线程 Join。即使所有外部 Manager/Receiver 都被 Drop，收割 Future 仍保留核心直到线程完成 Join，不静默 detach。

`JobOutcome::Failed(JobFailure)` 共享统一诊断报告。受管 `WidgetryFailure<E>` 或 `WidgetryEcsError` 直接转换，保留原 source、origin 与首次回溯；外部错误通过 `JobFailure::external(error, frame)` 首次纳入诊断，不能只返回格式化文本。`combine` 保存多份独立原因，按错误身份避免同份诊断重复加入；快照相等性比较诊断身份，不比较消息或回溯。`diagnostics()` 可检查各份原始诊断。克隆、status、outcome、Finished 和多个订阅者只是共享观察，不记录 handled；真正恢复、放弃或交付失败状态的 Owner 才调用 `record_handled`，未处理报告通过 `into_bevy_error()` 保持 Error Severity 进入宿主。

可捕获 panic 使用 `JobError::PanicObserved`，其回溯明确来自 catch 之后的观察点，不能代表展开前的 panic 源头栈；Job 不安装全局 panic hook。内部锁中毒沿既有恢复策略取得受保护状态，记录诊断、清除中毒标志并记录实际恢复。主循环 wake 保存原始错误，持续同一故障只建立一次诊断，恢复后记录生命周期 DEBUG 和实际处理 DEBUG；之后的新故障具有新身份。线程退出查询失败后成功 Join 的恢复事件同样使用 DEBUG，仍保留此前失败的源头诊断及最终 Failed 结果。

`begin_shutdown(timeout)` 可从任意线程非阻塞调用，返回 ShutdownHandle。在同一次注册表提交中停止接纳新任务并向所有活动 Job 请求取消。之后三个提交入口均返回 ShuttingDown。重复关闭返回同一个协调过程，不重置首次请求的全局截止时间。

首次接受关闭时记录一次 DEBUG（`stage = shutdown_started`），带活跃任务数和 `timeout_ms`；所有执行体真实结束并完成收割后记录一次 `shutdown_finished`。空集合立即完成，超时后继续等待也只在最终完成时记录，不对协调轮询、结果项或进度逐次输出。

ShutdownHandle 的 `status` 返回 ShuttingDown / TimedOut / Finished 及当前剩余 Job 身份和状态，`try_recv` 取走一次性 TimedOut（含超时发生时的剩余快照）或 Completed。所有克隆句柄共享这份通知队列，重复调用 begin_shutdown 不重复生成通知；队列最多保留两条，独立于业务数据容量。空集合立即 Completed；保留的终态记录、未读 Item 和未消费 Finished 都不阻碍关闭完成。

关闭协调在既有 Bevy 后台池运行，不依赖宿主轮询才计时或收割。超时只通知一次，不释放执行句柄，不强杀线程，也不替宿主退出应用；之后全部执行体退出并完成收割，仍发布一次 Completed。完成集合可以同时包含 Completed、Canceled 和 Failed。宿主应在收尾期间继续运行事件循环，决定如何展示超时及是否强制退出。

协调器保留关闭开始时的活动身份与独立完成时间。即使轮询延迟、记录已被移除，或最后一个任务在下一次轮询前已经退出，只要实际完成超过原始截止时间，仍按顺序交付 TimedOut 和 Completed。线程的完成时间在真实 Join 后记录；已在截止时间前结束的任务不会因轮询较晚被误报超时。关闭完成后释放该次跟踪引用。

`JobManagerPlugin` 通过 init_resource 在 App 内安装唯一 JobManager，保留宿主预先插入的容量配置和记录。后台直接克隆 Resource 获得同一控制核心。组合插件使用 `app.is_plugin_added::<JobManagerPlugin>()` 确保只安装一次。没有窗口或 EventLoopProxy 时，Headless 任务和显式轮询仍可使用；任务池由宿主的 DefaultPlugins / MinimalPlugins 等初始化。

插件在 First 收割任务、连接 Winit 代理并采样通知代数，在 Last 清除 Pending 标志并复检期间的新通知。Item、任务完成和关闭超时/完成通过 `WinitUserEvent::WakeUp` 跨线程请求帧更新；密集通知合并，清标记前后的新通知不会丢失。接收器实际读取 Item 且仍有积压或待交付 Finished 时请求后续消费帧；闲置慢订阅不会仅因持有未读数据强制应用持续刷新。唤醒错误经统一日志和宿主 Bevy Error Handler 传播，不安装日志订阅器。

消费者通常在 Update / PostUpdate 按自己的预算调用 try_recv。若在 First 消费，应排在 `JobManagerSystems::BeginFrame` 之后；若在 Last 消费，应排在 `JobManagerSystems::EndFrame` 之前，以便把消费延续请求纳入该帧的清标记复检。插件不提供通用 JobStateChanged 事件或百分比进度 API。

原生外来BevyError保留原对象及完整Display原因/context块，源头输出剔除Bevy附加的编号回溯帧；Job诊断自身的原始回溯继续由统一origin持有。已受管的BevyError继续共享原领域诊断，并将外层ContextExt说明作为额外上下文保留到每个诊断及宿主边界，不生成新origin或重复源头日志。
