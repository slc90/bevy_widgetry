# 完善 Gallery Canvas 文件导出的分阶段日志

## 目标与设计思路

让 Gallery Canvas 页面上的 SVG／PNG 导出具备完整、可信的业务 INFO，尤其是异步 PNG 的“发起、拒绝、最终结果”，以及必要的执行 DEBUG。用户请求和实际文件导出结果必须分开，不能用某个按钮事件代替后台完成事实。此增量只涉及页面级导出，不包括单独的 BRP Canvas 阶段导出工作流。

## 现有记录点

在 `gallery/src/pages/canvas.rs` 中，`operate()` 执行 `Action::Svg | Action::Png => export(...)`，之后统一记录 `info!(?action, "执行 Canvas 演示操作")`。`export()` 对同时存在 `PendingExport` 的 PNG 直接返回 Busy UI 状态，没有单独业务日志。同步 SVG 完成后记录 `info!(?path, "Canvas SVG 文件导出完成")`；异步 PNG 请求被接受后记录 `info!(?request, ?path, "已发起 Canvas PNG 导出")`，`png_finished` 只有成功结果写 INFO，失败结果只更新 UI／领域诊断。

因此当前用户尝试导出但被 Busy、路径创建失败、PNG 被取消或真正导出失败时，容易缺少完整的业务操作结果。统一“执行 Canvas 演示操作”与具体导出日志还可能重复描述请求，需要明确每条负责的语义。

## 操作与实际完成

在应用层明确接收 SVG／PNG 导出动作时记录用户意图 INFO，包括格式、必要参数、输出目标；对于同步 SVG，完成后可以用一条带 `result` 的 INFO 表达处理结局，或保持请求与结果两条但不得重复写同一成功事实。同步失败仍需有真实业务结果，不得以错误日志取代“用户操作已返回失败”的记录。

异步 PNG 被接受时用 INFO 表明“请求已提交”，包含已有的 `request` Entity 身份、导出路径和关键尺寸／Scale；在实际 `CanvasPngExportFinished` 观察到终态后再记录另一条 INFO，区分成功、取消和失败。用户重复提交而已有 PendingExport 时也应记录被拒绝／Busy 的操作结果，但不能声称启动了第二次导出。失败仍由 `CanvasExportError` 现有 `JobFailure`、源头 ERROR 和 `record_handled` 处理 DEBUG 负责，INFO 只表达业务终态，不复制错误堆栈。

在掌握程序运行事实的现有阶段点记录适量 DEBUG（例如后台导出实际受理与结果归档），关联 `request`；若底层受管 JobManager 已经记录相同任务的 Running／Finished，就不在页面重复写另一份相同的执行事实。`PendingExport` 自身已经提供自然的关联身份，不为一次页面导出引入通用日志服务。

新增日志正文统一中文，动态上下文使用英文结构化字段；只记录必要的对象身份、状态和结果摘要，不打印完整用户输入、敏感内容或大体量业务数据，也不手动重复 Tracing 已提供的 `file`、`line`、`module_path`、`crate_name` 等元数据。

## 验证与边界

覆盖 SVG 成功与失败、PNG 请求提交与异步真正成功、重复点击 Busy、PNG 取消、后台失败及页面收尾。确认失败有领域 ERROR 和用户结果 INFO 但不重复 ERROR；新 DEBUG 不会把排队当成写盘完成；同一 `request` 的最终结果只输出一次；可用结构化字段过滤同一次导出。测试或现有 GUI／BRP 验证不得改变 Canvas Document、GPU Readback、文件发布和 UI 本来的结果行为。

同步 `docs/gallery.md` 中 Canvas 导出示例对实际完成／取消的描述，不修改 Widgetry Canvas 通用 Pointer、几何编辑或逐帧渲染日志。
