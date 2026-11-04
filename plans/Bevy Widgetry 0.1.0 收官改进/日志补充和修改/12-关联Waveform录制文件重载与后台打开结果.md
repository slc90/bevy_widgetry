# 关联 Waveform 录制文件重载与后台打开结果

## 目标与设计思路

让 Gallery 的 Waveform 本地录制回放能追踪“用户或程序请求打开哪份文件、前一次任务为何取消、这次打开真正得到什么结果”。业务请求与结果使用 INFO；受管 Job 的执行、取消请求、实际终态和清理过程使用 DEBUG。保持目前对过期结果、强引用所有权、Revision 和失败处理的全部保障。

## 现有执行路径

`gallery/src/pages/waveform/disk.rs` 中 `DiskReplay::new()` 会自动 `reload()`；用户按钮也会调用 `reload()`，并写 `info!("请求重新加载本地 Waveform 回放记录")`。`reload()` 增加 `revision`、取消之前的 `OpenJob`，然后通过 `JobManager::spawn_thread` 启动打开操作。`poll()` 只有在收到 `JobRead::Finished` 后才决定接纳最新版本的打开结果，成功时写 `info!(path, "本地 Waveform 回放记录加载完成")`；失败采用原有受管诊断；真正的 `JobOutcome::Canceled` 当前没有业务日志。`cancel_open()` 还启动一个后台收尾 Future，以等待过期接收器并回收 Job。

这些分支说明必须把“接受任务”“终态到达”“最新版本真正被提交”区分开：Job 成功不等于旧 Revision 的数据就可以覆盖当前回放，用户申请重载也不等于文件已经读好。

## 业务意图与终态记录

在用户触发 Reload 的应用层入口记录 INFO，附上文件路径、即将处理的请求标识或版本；不能只写无字段消息。程序在 `DiskReplay::new()` 自动打开文件不是用户点击，可视为具有独立业务意义的自动加载，必要时 INFO 标明 `origin = "automatic"`，不能伪称用户操作。对于已存在录制文件的实际提交，记录返回的 `JobId` 与 Revision，以便与底层 JobManager DEBUG 关联；不存在文件时记录真实“未找到/未启动”业务结果，而不是启动成功。

保持 `JobManager` 对线程实际执行与取消的 DEBUG；Waveform Owner 在旧请求被新 Revision 取代时可以追加一次 DEBUG 表明“旧版本已放弃／请求取消”，但不能把取消请求当成 `JobOutcome::Canceled`。后台收尾的真实回收才允许记录完成；若已交付的旧版本结果最终到达，也不能写成当前业务加载成功。

当 `poll()` 收到最新版本的有效 `Completed` 且 Source Owner／Weak 升级和元信息校验均通过、`self.source` 已提交后，记录一次业务结果 INFO，包含 `path`、`job_id`、`revision`、采样率／通道等必要摘要。对于手动或重要自动请求的 `Canceled`、失败和结果被放弃，表达其真实业务终态；失败原始原因仍在源头 ERROR／handled DEBUG，不重播 Backtrace。若需要区分首次自动加载和用户重载的结果来源，只在 `DiskReplay` 既有请求上下文中保留最小来源信息，不增加全局日志状态。

新增日志正文统一中文，动态上下文使用英文结构化字段；只记录必要的对象身份、状态和结果摘要，不打印完整用户输入、敏感内容或大体量业务数据，也不手动重复 Tracing 已提供的 `file`、`line`、`module_path`、`crate_name` 等元数据。

## 验证与边界

利用 `disk.rs` 已有重载／取消／并发测试：旧版本任务晚于新版本结束时不能出现虚假业务成功；取消请求和真正 Canceled 区分；打开成功却没有有效 Source 时仍是 ERROR；真实成功只在最新版本数据已提交后记录；重复 Poll 不重复结果日志；页面退出或关闭触发的清理不被误判为用户取消。已有 Source 强所有权、防死锁锁序与 `JobFailure` 诊断共享行为不得变更。

同步 `docs/gallery.md` 中 Waveform 录制文件的后台加载行为描述。普通逐帧波形渲染、播放进度更新、波形窗口滚动不加入 INFO/DEBUG；此增量只处理磁盘录制文件加载业务流程。
