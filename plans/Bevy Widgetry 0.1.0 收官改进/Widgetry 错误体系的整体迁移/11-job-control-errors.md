# 为 Job 提交与控制 API 保留结构化失败

保留 SubmitError、JobAccessError 等可匹配语义，在外面携带统一诊断；不把 normal queue/status 结果混成 ERROR。

## 修改落点与前置能力

涉及：`crates/job/src/types.rs`、`crates/job/src/manager.rs`、`crates/job/src/receiver.rs`、`crates/job/src/channel.rs`、`crates/job/src/shutdown.rs`、`直接匹配 SubmitError/JobAccessError/SendError/TryRecvError 的消费者`。

实际前置为[01 建立领域错误的统一诊断载体](01-diagnostic-payload.md)、[02 保留诊断身份地进入 Bevy 错误边界](02-bevy-error-bridge.md)、[03 让测试能观察完整错误生命周期](03-diagnostic-test-support.md)。这些引用表示本项直接使用的能力，不要求相邻编号之间存在调用关系。

## 真实失败的分类与现场

容量为零、任务池未初始化、线程创建失败、ID 耗尽、类型不匹配等保留现有分类，补充容量、JobKind、JobId、label、预期/实际类型与原始 OS source。线程创建不再只保留 error.to_string()。

公开 API 改为携带诊断的领域结果；匹配迁移到 kind，不要求诊断参与 Eq。cancel/status/remove/subscribe 的不存在或阶段不允许必须按已定 API 契约处理：真实非法操作有源头 ERROR，需要合法探测的查询返回明确 Option/状态，不能先造错误再忽略。

## 队列状态和关闭语义

Empty、订阅自然结束、协作取消、容量背压等不一律变为诊断失败。把当前作为控制流使用的 Err 状态改为明确返回状态，或在第三方适配处先分类再纳入诊断。真正的越界、丢失、协议破坏仍记录 ERROR。

维持真实线程退出收割、取消请求/确认、停止接收新任务与记录移除规则。状态变化不能因“错误已记录”被跳过。所有直接消费者在本增量适配新返回形状，不能留下暂时不编译的 match。

## 诊断承接与调用方迁移

错误采用领域分类和统一诊断包装；同一次失败的 ID、source 与原始 Backtrace 在包装/跨线程传递时保持。源头 ERROR 不输出完整栈，纯传播不重复记录；真正消费失败后 DEBUG 记录实际措施，随后诊断可以释放。只有未处理错误在 Bevy 要求的边界显式转为 Severity::Error，最终由宿主输出原栈。正常等待、取消、合法 No-op 不人为转换为错误。

本增量改变的公开签名、回调或结果形状，必须在同一个增量更新所有直接消费者、测试、examples/benches 和所需 Facade 导出。不能用兼容 String 的临时入口掩盖未迁移调用，也不能把基本编译修复推迟到最终 Gallery 增量。

## 完成条件

- 提交配置、池不可用、线程创建错误验证 kind/原source/参数/一条源头 ERROR。
- 正常 Empty、Finished、协作取消和合法关闭不产错误；错误 JobId 的操作与合法探测测试分开。
- 多订阅类型不匹配、NotFinished、ShuttingDown 路径按明确契约验收；真实错误处理 DEBUG 不由状态查询生成。
- 检查 file_dialog/waveform/waveform_file/canvas/Gallery 的全部直接调用适配并编译。

在 Windows x86_64-pc-windows-msvc / PowerShell 7 下执行本项相关测试：

```powershell
cargo test -p bevy_widgetry_job
```

对本项实际改动的包与直接消费者进行编译/Clippy 检查；触及 Gallery 时至少检查其编译。GUI、后台时序或性能敏感路径按本节具体场景复验，不重复运行未受影响的整套全仓验收。

同步 Job API 文档、types.rs rustdoc 及 COVERAGE；移除旧错误相等性测试对诊断数据的依赖。

代码、构建或测试行为变更按适用工程规则完成必要的独立 Code Review；修复和回归仍归入本增量。

## 保持的边界

只迁移错误表示、传递、处理记录及直接受影响的调用。允许破坏性 API 更改，但不改变既有正常交互、状态通知、资源归属或业务算法；不加入与该错误路径无关的重构。

## 顺序衔接

前一项：[10 记录光标资源失败与实际降级结果](10-cursor-fallback-errors.md)；后一项：[12 让后台 Job 的失败跨线程保持原诊断](12-job-outcome-diagnostics.md)。编号规定唯一执行顺序，技术依赖以上文列出的能力为准。
