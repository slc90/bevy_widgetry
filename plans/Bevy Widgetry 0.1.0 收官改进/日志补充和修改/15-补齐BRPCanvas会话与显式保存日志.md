# 补齐 BRP Canvas 会话与显式保存日志

## 目标与设计思路

让 BRP Canvas 的会话创建、打开／恢复以及显式保存这一组业务动作具备可信的 INFO 结果。不能因为操作是由远程 BRP 发起，就误认为没有用户或业务语义；也不能因为内部 `persist()` 返回了控制权，就自动称作磁盘保存成功。

## 已有 API 与实际语义

`gallery/src/canvas_experiment.rs` 的 `create()` 创建会话目录、Document、窗口，随后执行 `storage::persist()` 并向调用方返回会话摘要；目前这条业务操作没有会话级结果 INFO，仅 `open_window()` 会记录“通过 BRP 打开独立绘画窗口”。`open()` 同样通过 `open_window()` 执行会话打开；`restore()` 在替换 Document 后有 `info!(session, "通过 BRP 恢复绘画工作存档")`。`close_windows()` 在实际关闭独立绘画窗口时记录 INFO，虽然这一记录本身主要描述程序状态变化。

`gallery/src/canvas_experiment/storage.rs` 的 `save()` 是明确的 BRP `gallery/canvas_save` 业务请求，但只调用 `persist()` 和返回 `describe()`，没有业务 INFO。`persist()` 也被会话创建、绘图批次和 Stage 导出路径复用；它通过 `dirty` 和 `save_error` 暴露是否真实保存，不能在公共 `persist()` 里直接为所有调用者写“用户点击保存”。

## 信息该由哪一层记录

在 BRP 会话 API 层记录成功接受与最终结果，保留 `session`、`operation`、`origin = "brp"` 和真实 `saved`／`error` 等摘要字段，不记录整份形状列表或请求全文。`create()` 必须在会话真正插入 `Sessions` 并有可返回的 ID 后才能写“会话已创建”；`open()` 在已经拥有活动窗口时应记录真实“不需要重开”结果，不重复宣称创建新窗口；`restore()` 必须在切换 Document 已成功且会话提交之后才写“存档已恢复”。

`storage::save()` 的业务结果应在 `persist()` 返回、实际 `dirty` 与 `save_error` 已经确定后记录 INFO，区分“已保存”和“保存失败、仍保持编辑状态”。如果发生真实 I/O 错误，继续沿现有源头诊断与错误处理传播，不把真实失败降成 WARN，也不能用业务 INFO 掩盖它。自动触发的持久化不会被误记为“远程用户显式保存”。

程序内部窗口实际创建、销毁和窗口状态切换，若有诊断价值可以记录 DEBUG；用户或 BRP 创建／打开请求的 INFO 不能替代它。`close_windows()` 遇到 OS 窗口关闭请求与会话自然失效时，应把实际窗口清理记录调整为 DEBUG；只在应用层确知独立的外部请求时才补充语义操作 INFO，不能猜测触发者一定是用户。

新增日志正文统一中文，动态上下文使用英文结构化字段；只记录必要的对象身份、状态和结果摘要，不打印完整用户输入、敏感内容或大体量业务数据，也不手动重复 Tracing 已提供的 `file`、`line`、`module_path`、`crate_name` 等元数据。

## 验证与边界

使用现有 Canvas BRP 会话与存档场景，验证 create 成功、打开已存在会话、从磁盘恢复、显式 save 成功、显式 save 失败但编辑状态保留，以及直接关闭窗口不销毁会话。INFO 要体现真实操作、来源和持久化状态；不因自动保存产生误导性的用户日志，不在失败时写成功。原有会话目录唯一性、存档替换、Document 释放、Window Owner 和 BRP 返回字段不改变。

同步 `docs/gallery.md` 对 `gallery/canvas_create`、`gallery/canvas_open`、`gallery/canvas_restore`、`gallery/canvas_save` 的可观察行为描述。绘图批次与 Stage 导出另行独立处理，本增量不对只读 `query` 增加通常无诊断价值的 INFO。
