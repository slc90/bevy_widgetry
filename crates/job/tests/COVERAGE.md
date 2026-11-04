# Job 测试覆盖

- 执行状态：登记后启动、Running、Completed、Failed；终态不可逆，活动记录不可删除。
- 后端：真实 Bevy IoTaskPool 和 AsyncComputeTaskPool。
- 刺激：提交、跨线程查询、显式移除、Future 顶层错误、可捕捉 panic。
- 身份：并发提交不重复、Manager 实例相互独立；ID 耗尽由模块内边界测试覆盖。
- 接收：零数据任务只交付一次 Finished，之后 Closed；诊断与注册表保持一致。
- 广播：独立快慢订阅、新订阅无历史、类型匹配、Pending/终态订阅守卫、首条数据注册边界、无活动订阅正常丢弃与重新订阅。
- 背压：满缓冲发送 Pending、不覆盖、Drop 慢订阅恢复、默认/单 Job 容量配置、零容量拒绝。
- 结束：满缓冲仍进入终态、先排空 Item 后一次 Finished、移除记录不损坏未读失败结果；无 Clone 大型 Batch 与业务进度共享 Arc 并保持顺序。
- 取消：Pending/Running → Stopping，启动前业务体跳过，重复取消和未知 ID；运行体延迟退出时保持 Stopping；读取消标志和等待通知不构成确认。
- 裁决：未确认取消的 Ok 为 Completed，有效确认的 Ok 为 Canceled；无请求/执行期外确认被拒绝；确认后的 Err/panic 仍 Failed。自然成功/错误与取消并发时终态唯一且不可逆。
- 背压取消：取消唤醒满缓冲发送，SendStatus::Canceled 不创建错误或自动确认取消；正常返回且不确认时 Completed，已发 Item 和移除后的 Finished 保留。
- 线程：命名、广播、无订阅者仍完成、成功/取消确认/错误/panic 共用裁决、TLS 析构延迟时保持活动状态、后台真实 Join 后才 Finished。
- 收割：最后外部 Handle Drop 后核心仍被保留，Join 后可释放；创建线程的 OS 失败清除 Pending 记录且 ID 不复用，非法名称拒绝；多次查询/取消无状态倒退。
- Windows TLS 回归：业务主函数已结束但 TLS 等待同一个单工作线程 I/O 池时，收割必须让出该池，使其他 I/O Job 能推进并完成 TLS 清理。
- 关闭：三种后端并行取消，Pending/Running 全部请求停止，关闭/提交竞争要么纳管要么拒绝；无 Job 立即完成，终态记录不阻碍完成。
- 协调：唯一截止时间、重复请求幂等、最多一次 TimedOut 和一次 Completed、超时后继续真实收割、后台无宿主轮询也能计时；结果缓冲满和未读 Item 不阻碍完成，三类终态都计入退出。
- Headless App 在收尾等待期间仍能推进 Update；无限 Duration 不溢出；通知队列独立于业务接收器且克隆关闭句柄共享消费。
- 延迟轮询截止时间回归：最后任务已退出且当前剩余集合为空时，真实结束晚于截止时间仍先 TimedOut 后 Completed，结束早于截止时间则只 Completed；保留超时身份与当时 Stopping 快照。
- 初始关闭收割后才读取时钟；登记线程与关闭交错后，OS 启动拒绝不会成为永久未完成的跟踪或产生虚假超时。
- Plugin：两个组合消费者幂等安装、宿主预插入 Resource 保持、后台线程同一 Handle、独立 App 隔离、Headless 无 Winit 代理轮询及逐帧消费预算。
- 唤醒：高频通知合并、Headless 待处理通知在代理连接后补发、消费期间通知在清标记后重新投递、闲置订阅不连续刷新、错误持续状态去重及恢复后新故障。
- lifecycle_logging.rs：独立子进程安装真实全局捕获器，覆盖 Io/Cpu/Thread 的登记→Running→执行体结束→真实收割顺序、JobId/种类/Label 关联、取消请求去重、确认取消/迟到完成/失败终态与重复观察静默；TLS 析构阻塞时无提前 Join/Finished 日志，空关闭与重复关闭各只记录一次开始和完成。Lifecycle 单测覆盖 Pending 取消无 Running，Shutdown 单测覆盖超时后完成及重复推进无日志风暴。

- 控制诊断：零容量、缺失 CPU 池与请求现场、订阅类型名、严格未知 ID、原生 OS 线程创建 source；错误分类可匹配且不比较诊断载体。
- 合法探测与接收生命周期：find_status/cancel_if_present/remove_if_present 的缺失记录无日志，Empty/Finished/Closed 与协作取消无 ERROR；完成后发送是真实 Closed 失败。
- 传播边界：非法取消诊断进入真实 Bevy Command，保持原 origin、Severity::Error、一条源头 ERROR，纯传播不生成 handled。

- outcome_diagnostics.rs：真实OS线程、I/O池、CPU池的原始source/origin/强制捕栈；两订阅者与status共享结果且观察不handled，消费一次DEBUG、多原因组合不重捕栈、外部错误与PanicObserved明确区分。Manager模块验证真实线程Join与注入退出查询失败并存、各origin保留及恢复记录，锁中毒恢复只记录一次。Wake模块验证持续故障同一身份、恢复DEBUG与后续新故障身份。

原生BevyError多层context与原始原因完整保留，Display源头输出排除Bevy附加回溯；原Bevy对象可按原类型查询。

已受管BevyError经外层ContextExt再进入Job时，多个原报告共享origin、保留真实source与新增多层说明，并通过真实Command交给宿主；禁用或真实Bevy回溯均不重复源头日志或产生handled。

Native回归同时保护默认过滤回溯、BEVY_BACKTRACE=full模式，以及无回溯和真实回溯下应用自定义Debug/Display正文的编号Rust路径与伪context行，覆盖0..2层原生context；正文在首次纳入时计算一次，快照展示不反复格式化原Bevy回溯。
