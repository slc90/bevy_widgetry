# Job 测试覆盖

- 执行状态：登记后启动、Running、Completed、Failed；终态不可逆，活动记录不可删除。
- 后端：真实 Bevy IoTaskPool 和 AsyncComputeTaskPool。
- 刺激：提交、跨线程查询、显式移除、Future 顶层错误、可捕捉 panic。
- 身份：并发提交不重复、Manager 实例相互独立；ID 耗尽由模块内边界测试覆盖。
- 接收：零数据任务只交付一次 Finished，之后 Closed；诊断与注册表保持一致。
- ECS 注入、数据广播、取消、线程及应用关闭尚未实现。
