# Job

`bevy_widgetry_job` 是 ECS 无关的实例级任务控制核心。`JobManager` 可克隆并跨线程共享同一注册表，不使用进程级业务单例，也不安装任务池或日志订阅器。宿主需先初始化 Bevy 的 `IoTaskPool` 和 `AsyncComputeTaskPool`。

`spawn_io::<T, _, _, E>` 与 `spawn_cpu` 接受 `FnOnce(JobContext<T>) -> Future<Output = Result<(), E>>`，分别交给 Bevy 两类后台任务池，返回 `(JobId, JobReceiver<T>)`。首个 Receiver 与 Pending 记录先于执行体建立。`JobContext::send(T).await` 通过 `async-broadcast 0.7.2` 广播 `Arc<T>`，不要求 T: Clone；业务可自行定义 Batch、Progress 等结果项。

`JobId` 在每个 Manager 内从 1 严格递增，移除后也不复用，耗尽时拒绝提交。可选 `JobOptions::label` 保存业务标识。任务开始进入 Running；正常返回为 Completed，顶层错误或可捕捉 panic 为 Failed。仅业务顶层返回决定失败，业务已经处理的局部错误不影响状态。进程 abort 不保证转换为 Failed。

Manager 持有执行 Task，查询或接收时收割已完成 Task 后提交不可逆终态，`status` 提供分类、状态及最终错误。`JobReceiver::try_recv` 区分 Empty 与 Closed；`recv().await` 等待下一条 Item 或 Finished，`recv_blocking` 仅供普通同步线程使用，不能用于 ECS 主线程或池工作线程。每份 Receiver 按序排空自己的 Item 后，恰好交付一次 Finished，之后 Closed。内部意外 Overflowed 会记录错误并返回，不作为正常丢包处理。Receiver 保持 Manager 存活，移除记录不会破坏已获得的未读结果。

默认缓冲容量为 32 个结果项，通过 `JobManager::with_buffer_capacity` 设置实例默认值，`JobOptions::buffer_capacity` 按任务覆盖，零容量拒绝。容量按条目数计，不按业务 Batch 大小或字节计；最慢活跃订阅者产生背压，不覆盖未读数据。Finished 独立保存，不占数据容量，缓冲满不阻止任务终结。业务负责限制单条负载及已持有 Arc 的内存。

`subscribe::<T>(id)` 仅在 Pending/Running 接纳新订阅，类型不匹配返回 TypeMismatch，Stopping/终态返回 NotSubscribable。状态检查和订阅注册使用同一同步边界。新订阅只接收注册后的数据，不回放历史，不公开 Receiver 的 Clone。Drop 立即释放该订阅的未读引用。Manager 保留 InactiveReceiver，无活跃订阅时发送正常丢弃且不等待，后来仍可订阅未来数据。

记录持续保留，直到显式 `remove`。活动记录返回 `NotFinished(current_state)`，未知或已删除记录返回 `NotFound`。移除不取消任务。Future 必须等待自身业务操作真正结束，不得通过 detach 让 Job 提前完成。取消、受管线程、全局关闭与 ECS 插件尚未提供。
