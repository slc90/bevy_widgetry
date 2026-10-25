# Job

`bevy_widgetry_job` 是 ECS 无关的实例级任务控制核心。`JobManager` 可克隆并跨线程共享同一注册表，不使用进程级业务单例，也不安装任务池或日志订阅器。宿主需先初始化 Bevy 的 `IoTaskPool` 和 `AsyncComputeTaskPool`。

`spawn_io::<T, _, _, E>` 与 `spawn_cpu` 接受 `FnOnce(JobContext<T>) -> Future<Output = Result<(), E>>`，分别交给 Bevy 两类后台任务池，返回 `(JobId, JobReceiver<T>)`。首个 Receiver 与 Pending 记录先于执行体建立。`JobContext::send(T).await` 通过 `async-broadcast 0.7.2` 广播 `Arc<T>`，不要求 T: Clone；业务可自行定义 Batch、Progress 等结果项。

`JobId` 在每个 Manager 内从 1 严格递增，移除后也不复用，耗尽时拒绝提交。可选 `JobOptions::label` 保存业务标识。任务开始进入 Running；正常返回为 Completed，顶层错误或可捕捉 panic 为 Failed。仅业务顶层返回决定失败，业务已经处理的局部错误不影响状态。进程 abort 不保证转换为 Failed。

Manager 持有执行 Task，查询或接收时收割已完成 Task 后提交不可逆终态，`status` 提供分类、状态及最终错误。`JobReceiver::try_recv` 区分 Empty 与 Closed；`recv().await` 等待下一条 Item 或 Finished，`recv_blocking` 仅供普通同步线程使用，不能用于 ECS 主线程或池工作线程。每份 Receiver 按序排空自己的 Item 后，恰好交付一次 Finished，之后 Closed。内部意外 Overflowed 会记录错误并返回，不作为正常丢包处理。Receiver 保持 Manager 存活，移除记录不会破坏已获得的未读结果。

默认缓冲容量为 32 个结果项，通过 `JobManager::with_buffer_capacity` 设置实例默认值，`JobOptions::buffer_capacity` 按任务覆盖，零容量拒绝。容量按条目数计，不按业务 Batch 大小或字节计；最慢活跃订阅者产生背压，不覆盖未读数据。Finished 独立保存，不占数据容量，缓冲满不阻止任务终结。业务负责限制单条负载及已持有 Arc 的内存。

`subscribe::<T>(id)` 仅在 Pending/Running 接纳新订阅，类型不匹配返回 TypeMismatch，Stopping/终态返回 NotSubscribable。状态检查和订阅注册使用同一同步边界。新订阅只接收注册后的数据，不回放历史，不公开 Receiver 的 Clone。Drop 立即释放该订阅的未读引用。Manager 保留 InactiveReceiver，无活跃订阅时发送正常丢弃且不等待，后来仍可订阅未来数据。

记录持续保留，直到显式 `remove`。活动记录返回 `NotFinished(current_state)`，未知或已删除记录返回 `NotFound`。移除不取消任务。Future 必须等待自身业务操作真正结束，不得通过 detach 让 Job 提前完成。全局关闭与 ECS 插件尚未提供。

`cancel(id)` 是唯一取消申请入口，将 Pending/Running 改为 Stopping，返回 CancelRequested；重复取消 Stopping 返回 AlreadyStopping，终态返回 AlreadyFinished，未知 ID 返回 NotFound。终态和原错误不会被改写。开始执行前已取消的 Future 不再调用业务闭包，待执行句柄收割后进入 Canceled。

已经开始的执行体通过 `JobContext::is_cancelled` 或 `cancelled().await` 观察请求，在决定停止并完成必要清理后调用 `acknowledge_cancel()`，再返回 Ok。确认仅在执行期内且已有取消请求时返回 true，本身不会发布终态。真实退出并收割后，确认取消的 Ok 为 Canceled，未确认取消的 Ok 仍为 Completed；Err/panic 始终 Failed。单独读取取消标志或 send 返回 Canceled 都不构成取消确认，清理期间保持 Stopping。

`send` 优先检查取消并同时等待取消信号/缓冲空间，满缓冲可以由 cancel 主动唤醒，返回 SendError::Canceled。已经发送的 Item 仍供 Receiver 排空；最终 Finished 与注册表裁决一致。若将 SendError::Canceled 作为顶层 Err 传播，任务按 Failed 裁决。Manager 仅保证自身取消等待与发送背压可唤醒，不能中断任意第三方系统调用。

`spawn_thread::<T, _, E>` 接受同步闭包，用 `std::thread::Builder` 创建专属命名线程。`JobOptions::label` 同时作为线程名，无标识时使用带 JobId 的默认名称；名称包含空字符或 OS 创建失败会返回 ThreadSpawn，不留下活动记录，已分配 ID 不复用。线程收割需要已初始化 IoTaskPool。

线程与异步入口共享同一 Context、广播、取消确认及终态裁决。`send_blocking` 用于普通专属线程，仍可被取消唤醒，不能用于 ECS 或池工作线程。同步设备等待应由业务自行提供可中断路径。线程闭包退出仅触发收割通知；后台 Future 通过 win-custody 的安全 AsHandle 接口，以零超时查询 Windows 线程句柄，只在线程真实退出（包括 TLS 析构结束）后执行 Join，查询/接收收割该 Future 后公布终态。查询期间异步让出 I/O 工作线程，避免 TLS 清理等待同一个池时死锁。内部 OS 查询失败时记录错误并保留句柄重试，最终 Join 后通过 Infrastructure 失败诊断保留错误与执行结果。主线程不会等待活动线程 Join。即使所有外部 Manager/Receiver 都被 Drop，收割 Future 仍保留核心直到线程完成 Join，不静默 detach。
