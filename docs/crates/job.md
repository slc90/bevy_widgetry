# Job

`bevy_widgetry_job` 是 ECS 无关的实例级任务控制核心。`JobManager` 可克隆并跨线程共享同一注册表，不使用进程级业务单例，也不安装任务池或日志订阅器。宿主需先初始化 Bevy 的 `IoTaskPool` 和 `AsyncComputeTaskPool`。

`spawn_io::<T, _, _, E>` 与 `spawn_cpu` 接受 `FnOnce(JobContext<T>) -> Future<Output = Result<(), E>>`，分别交给 Bevy 两类后台任务池，返回 `(JobId, JobReceiver<T>)`。首个 Receiver 与 Pending 记录先于执行体建立。当前 Receiver 只交付终止结果；数据广播尚未提供。

`JobId` 在每个 Manager 内从 1 严格递增，移除后也不复用，耗尽时拒绝提交。可选 `JobOptions::label` 保存业务标识。任务开始进入 Running；正常返回为 Completed，顶层错误或可捕捉 panic 为 Failed。仅业务顶层返回决定失败，业务已经处理的局部错误不影响状态。进程 abort 不保证转换为 Failed。

Manager 持有执行 Task，查询或接收时收割已完成 Task 后提交不可逆终态，`status` 提供分类、状态及最终错误。`JobReceiver::try_recv` 区分 Empty 与 Closed；`recv().await` 等待终态，每份 Receiver 恰好交付一次 Finished，之后流关闭。Receiver 保持 Manager 存活，移除记录不会损坏已获得的终止结果。

记录持续保留，直到显式 `remove`。活动记录返回 `NotFinished(current_state)`，未知或已删除记录返回 `NotFound`。移除不取消任务。Future 必须等待自身业务操作真正结束，不得通过 detach 让 Job 提前完成。取消、受管线程、全局关闭与 ECS 插件尚未提供。
