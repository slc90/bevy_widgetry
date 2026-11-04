# 完善 BRP Canvas 阶段导出的操作及结果日志

## 目标与设计思路

把 Gallery 自己暴露的 BRP Canvas 会话／阶段操作当成真实的应用业务操作，而不是后台框架输入。调用请求有明确业务语义，用 INFO 表示接受或拒绝以及最终业务结果；只有真实的文件处理、阶段推进和任务取消用 DEBUG。复用已有 Stage ID、JobId、文件状态，不增加日志轮询或改变 BRP 返回结构。

## 现有事实与缺口

`gallery/src/canvas_experiment.rs` 已对恢复存档、打开独立绘画窗口、关闭窗口保留会话等操作写了 INFO。`gallery/src/canvas_experiment/stages.rs` 的 `export()` 在预留 Stage、冻结 Document、建立 SVG／PNG 工作后写 `info!(session, stage, status, "通过 BRP 发起绘画阶段导出")`，`advance()` 在队列全部推进完成、快照 Document 被销毁时写 `info!(session, stage, status, "绘画阶段导出已收尾")`。

Stage 自身的 `FileState` 已明确区分 `Queued`、`Exporting { request, job, temporary }`、`Succeeded`、`Failed`、`Canceled`；其 BRP 摘要还含文件路径、Scale、尺寸、JobId 和整体 `exporting/succeeded/partial_failure/failed/canceled`。因此不应另造监视状态；应直接在这些已有不可逆转换点记录真实事实。

## 请求、后台阶段与最终结果

对已识别、通过参数校验的 BRP 业务操作，在应用 Owner 入口记录可追踪的语义请求。操作被拒绝（例如已有 Stage 在执行、会话不存在、文档为空）时，记录真实的拒绝结果，但不能记为 Stage 已接受或导出成功。这里的业务响应仍遵循原有 `BrpResult`／领域诊断处理；日志不代替 API 返回内容。

在 `export()` 真正完成 Stage 编号预留后，保留一次“阶段导出已接受”的 INFO，包含 `session`、`stage`、当前整体状态；不把此时队列尚未启动的 PNG 写成完成。`advance()` 和现有 PNG 完成 Observer 中，在文件从 `Queued`→`Exporting`、`Exporting`→终态等具有诊断价值的**实际变化**时，按需记录 DEBUG，包含 Stage、文件、`request`／`job_id`。连续轮询、同一队列反复检查和每个 Progress 不记录。

阶段最终达到不再 Pending 的总体终态时，记录一次带 `session`、`stage`、成功数量、失败数量、取消数量与真实最终 `status` 的业务 INFO。此结果既适用于全部成功，也要表达 `partial_failure`、`failed`、`canceled`；`advance()` 中释放 Snapshot 是内部清理，使用 DEBUG 或保持静默，不能拿“快照已销毁”假装业务导出成功。若 Stage 导出在 Gallery 退出时被请求取消，取消请求只能写 DEBUG，真正 Canceled 要等现有终态处理，不得提前宣布文件取消或删除。

其他 BRP 会话创建、打开、保存以及绘图批次属于独立的业务操作路径，分别由对应增量检查，避免在本增量中扩大 Stage 导出的修改范围。纯只读 `stage` 查询和 Benchmark 高频探针仍不增加普通 INFO。

新增日志正文统一中文，动态上下文使用英文结构化字段；只记录必要的对象身份、状态和结果摘要，不打印完整用户输入、敏感内容或大体量业务数据，也不手动重复 Tracing 已提供的 `file`、`line`、`module_path`、`crate_name` 等元数据。

## 验证与边界

使用既有 BRP Canvas 工作会话与阶段导出用例，覆盖 Stage 接受、队列顺序、多个 PNG Scale、成功、部分失败、失败、取消、重复查询、并发阶段被拒绝、后台完成时页面退出。验证请求 INFO 与实际最终 INFO 通过 `session + stage` 关联，阶段 DEBUG 与同一 `job_id` 相关，且整个过程没有每帧重复日志。现有 Snapshot 冻结、编号不复用、临时文件原子发布、Job 取消与错误报告不变；不得为了日志修改 API、添加新的调度系统或延长资源寿命。

同步 `docs/gallery.md` 的 Canvas BRP 会话与阶段导出描述，保持文档只陈述已经实现的日志行为。
