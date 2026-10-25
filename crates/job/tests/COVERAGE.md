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
- 背压取消：取消唤醒满缓冲发送，SendError::Canceled 不自动确认取消；直接作为顶层 Err 传播时 Failed，已发 Item 和移除后的 Finished 保留。
- 线程：命名、广播、无订阅者仍完成、成功/取消确认/错误/panic 共用裁决、TLS 析构延迟时保持活动状态、后台真实 Join 后才 Finished。
- 收割：最后外部 Handle Drop 后核心仍被保留，Join 后可释放；创建线程的 OS 失败清除 Pending 记录且 ID 不复用，非法名称拒绝；多次查询/取消无状态倒退。
- Windows TLS 回归：业务主函数已结束但 TLS 等待同一个单工作线程 I/O 池时，收割必须让出该池，使其他 I/O Job 能推进并完成 TLS 清理。
- ECS 注入及应用关闭尚未实现。
