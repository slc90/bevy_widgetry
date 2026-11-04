# 完善 Gallery Job 演示操作与终态日志

## 目标与设计思路

让 Gallery 的 Job 演示能够把用户的每个语义动作与该任务真实执行结局关联起来，同时继续让底层 `JobManager` 单独负责 DEBUG 生命周期。此增量只涉及 Gallery 的演示业务 Owner，不改 JobManager 的运行状态、JobId 分配或失败语义。

## 现有差距

`gallery/src/pages/job.rs` 的 `operate()` 在 `apply()` 返回 `Ok(result)` 时才调用 `info!(?action, %result, "执行 Job 演示操作")`；当操作被拒绝、失败或出现可预期的同值业务结果时，当前记录不足以表示**用户确实尝试了什么**。`start()` 会创建标记 `Gallery {mode:?} demo` 的 Io/Cpu/Thread Job 并取得 `JobId`，但日志主要从格式化 `%result` 中间接读取 ID，不利于结构化关联。

`poll()` 在演示的 Job 状态首次进入终态时，只有 `!matches!(status.state, JobState::Failed)` 才记录 `info!(job_id, outcome, "Job 演示任务执行结束")`。已有真实失败的 ERROR 不应被重复描述，但仍应能从 INFO 判断这项用户业务操作最终失败／取消／完成。现有 `select()` 在用户选择 Job 时已有 INFO，保留并避免重复。

## 业务操作与执行结果

在已确认是用户 `On<Activate>` 触发的动作后记录带 `action`、可用 `job_id`、`mode` 的 INFO；明确区分受理、操作未改变状态、业务拒绝和最终结果。对于同步完成的动作（例如切换慢消费设置、订阅／取消请求被接受），可在处理结束时用一条 INFO 带结果；对于提交一个后台 Job，则先记录任务已经成功提交，之后在终态收割时记录实际结果。取消操作 INFO 表示用户申请取消，不能写成 Canceled；真正 Canceled 以 Job 的不可逆终态及应用收到结果为准。

在 `start()` 返回成功 ID 后保留该 ID 作为所有相关日志的结构化字段，不强制新增 `operation_id`；`JobId` 对于此演示的每个独立任务已经够用。`poll()` 对 Completed、Canceled 和 Failed 都在**首次**发现真实终态时输出一条不重播 ERROR 内容的业务结果 INFO，包含 `job_id` 和 `outcome`。若用户移除了记录或离开页面，不应依据已经缺失的信息推断未观察到的成功；现有 `Orphans` 收尾继续由业务 Owner 负责。

失败本身仍通过 `JobFailure` 的源头 ERROR 和既有 handled DEBUG 表达；不能因为补 INFO 就把错误 ID、Backtrace、ECS Severity 或展示失败状态的处理分支改掉。对于输出业务拒绝但没有真实系统错误的 `WorkError`，不额外伪造 ERROR。`JobManager` 内部 Running、Stopping、Join、Canceled 等 DEBUG 由对应增量负责，Gallery 不复制相同内部阶段。

新增日志正文统一中文，动态上下文使用英文结构化字段；只记录必要的对象身份、状态和结果摘要，不打印完整用户输入、敏感内容或大体量业务数据，也不手动重复 Tracing 已提供的 `file`、`line`、`module_path`、`crate_name` 等元数据。

## 验证

使用已有 Job 演示测试与可控 JobManager 时序，检查提交与 `JobId` 关联，成功、取消、迟到完成、业务失败的最终 INFO 事实一致；重复轮询不会重复输出；重复取消仅表达实际业务答复；错误分支仍先有真实源头 ERROR 并正确 handled。以现有 `LogCapture` 和真实状态检验消息字段、等级、顺序，不依赖逐帧睡眠猜测执行结果。

同步 `docs/gallery.md` 中 Job 示例的实际诊断轨迹，保留 A/B 订阅、缓冲背压、任务撤销和演示展示行为。
