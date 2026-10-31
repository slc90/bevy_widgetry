# 保留 FileDialog 偏好持久化与结果递送双重失败

使偏好读写通过Reply和JobOutcome传递同一诊断，保留已经完成的磁盘事实，不再拼接业务错误和递送错误。

## 修改落点与前置能力

涉及：`crates/file_dialog/src/preferences_job.rs`、`crates/file_dialog/src/persistence.rs`、`crates/file_dialog/src/storage.rs`、`crates/file_dialog/src/runtime.rs 的preferences分支`。

实际前置为[12 让后台 Job 的失败跨线程保持原诊断](12-job-outcome-diagnostics.md)、[44 为 FileDialog 状态与公开操作建立领域错误](44-file-dialog-contract-errors.md)、[45 保留目录枚举与快照流水线中的真实失败](45-file-dialog-directory-errors.md)。这些引用表示本项直接使用的能力，不要求相邻编号之间存在调用关系。

## 一次失败不能在两个结果通道重生

Reply::Loaded/Saved目前保存String，工作闭包又克隆completion到JobOutcome。改为共享不可变诊断，同一失败在两个通道里拥有相同origin，不重复源头ERROR或捕栈。

load/save错误保留path、scope/revisions、操作阶段和原source。不存在的首次偏好文件按现有默认加载语义处理，损坏文件、权限和实际写失败属于真实错误。UI/状态里显示的文本只是派生展示，不是错误传输。

## 晚取消与递送失败

晚取消不能撤销已经完成的加载或持久化写入。发送reply失败与实际IO失败可能同时发生，分别保存两个诊断，不用format!("cause; result delivery failed")。

revision提交、failed_revisions去重、flush准入和重试规则保持现有约束。只有业务Owner完成采用默认值/保留旧偏好/拒绝本次保存等明确处理后才DEBUG；仅接收Failed不算处理。

## 诊断承接与调用方迁移

错误采用领域分类和统一诊断包装；同一次失败的 ID、source 与原始 Backtrace 在包装/跨线程传递时保持。源头 ERROR 不输出完整栈，纯传播不重复记录；真正消费失败后 DEBUG 记录实际措施，随后诊断可以释放。只有未处理错误在 Bevy 要求的边界显式转为 Severity::Error，最终由宿主输出原栈。正常等待、取消、合法 No-op 不人为转换为错误。

本增量改变的公开签名、回调或结果形状，必须在同一个增量更新所有直接消费者、测试、examples/benches 和所需 Facade 导出。不能用兼容 String 的临时入口掩盖未迁移调用，也不能把基本编译修复推迟到最终 Gallery 增量。

## 完成条件

- 加载/保存失败经Reply和JobOutcome观察到相同ID与原source。
- 真正IO失败与结果递送失败并存时两份诊断可见；同一次IO故障没有双源头日志。
- 晚取消后的磁盘事实、revision、首次缺文件和损坏文件区别测试保留。
- 偏好处理恢复记录与持续失败恢复INFO分开，正常自动保存没有额外日志。

在 Windows x86_64-pc-windows-msvc / PowerShell 7 下执行本项相关测试：

```powershell
cargo test -p bevy_widgetry_file_dialog
```

对本项实际改动的包与直接消费者进行编译/Clippy 检查；触及 Gallery 时至少检查其编译。GUI、后台时序或性能敏感路径按本节具体场景复验，不重复运行未受影响的整套全仓验收。

同步偏好读写、flush、取消与双重失败覆盖。

代码、构建或测试行为变更按适用工程规则完成必要的独立 Code Review；修复和回归仍归入本增量。

## 保持的边界

只迁移错误表示、传递、处理记录及直接受影响的调用。允许破坏性 API 更改，但不改变既有正常交互、状态通知、资源归属或业务算法；不加入与该错误路径无关的重构。

## 顺序衔接

前一项：[46 为 FileDialog 单次文件操作保留请求诊断](46-file-dialog-operation-errors.md)；后一项：[48 明确 FileDialog 运行时与界面的错误消费责任](48-file-dialog-consumption-errors.md)。编号规定唯一执行顺序，技术依赖以上文列出的能力为准。
