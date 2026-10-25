# Job 测试覆盖

- 执行状态：登记后启动、Running、Completed、Failed；终态不可逆，活动记录不可删除。
- 后端：真实 Bevy IoTaskPool 和 AsyncComputeTaskPool。
- 刺激：提交、跨线程查询、显式移除、Future 顶层错误、可捕捉 panic。
- 身份：并发提交不重复、Manager 实例相互独立；ID 耗尽由模块内边界测试覆盖。
- 接收：零数据任务只交付一次 Finished，之后 Closed；诊断与注册表保持一致。
- 广播：独立快慢订阅、新订阅无历史、类型匹配、Pending/终态订阅守卫、首条数据注册边界、无活动订阅正常丢弃与重新订阅。
- 背压：满缓冲发送 Pending、不覆盖、Drop 慢订阅恢复、默认/单 Job 容量配置、零容量拒绝。
- 结束：满缓冲仍进入终态、先排空 Item 后一次 Finished、移除记录不损坏未读失败结果；无 Clone 大型 Batch 与业务进度共享 Arc 并保持顺序。
- ECS 注入、取消、线程及应用关闭尚未实现。
