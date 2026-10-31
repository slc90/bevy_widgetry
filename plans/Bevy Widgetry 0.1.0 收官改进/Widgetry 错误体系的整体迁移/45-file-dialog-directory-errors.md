# 保留目录枚举与快照流水线中的真实失败

在目录读取线程和CPU快照工作之间传递受管诊断，明确正常取消、部分条目失败和整个目录操作失败的不同结局。

```mermaid
flowchart LR
  FS["文件系统读取"] --> IO["目录 I/O Job"]
  IO --> CPU["CPU 快照工作"]
  CPU --> R["带 token 的回执"]
  R --> O["Runtime / 会话 Owner"]
  IO --> P["条目失败后跳过：ERROR + DEBUG"]
  O --> H["实际消费或继续传播"]
```

## 修改落点与前置能力

涉及：`crates/file_dialog/src/filesystem.rs`、`crates/file_dialog/src/directory.rs`、`crates/file_dialog/src/snapshot.rs`、`crates/file_dialog/src/projection.rs`、`crates/file_dialog/src/retention.rs`、`crates/file_dialog/src/runtime.rs 的目录接收分支`。

实际前置为[12 让后台 Job 的失败跨线程保持原诊断](12-job-outcome-diagnostics.md)、[44 为 FileDialog 状态与公开操作建立领域错误](44-file-dialog-contract-errors.md)。这些引用表示本项直接使用的能力，不要求相邻编号之间存在调用关系。

## 从文件系统入口保存source

保留目录path、请求token、操作阶段及io::ErrorKind/原生source。自定义文件系统trait与native实现的结果类型如需改变，所有实现及直接调用同一增量迁移。第三方I/O的预期NotFound或已请求取消的Interrupted先按原语义分类，再决定是否进入错误体系。

BatchReply::Failed(String)、CpuSendError::Fault中的字符串以及Job完成结果不能成为唯一错误载体。CPU和I/O之间传递同一origin；通道Busy属于背压，不能生成错误日志。真实断连、容量违反或处理故障保留source。

## 部分成功与取消

enumerate中的单条entry读取失败已经采用跳过并继续的行为：源头记录ERROR，确认跳过后记录DEBUG，UI summary仍可只保留有限条可读摘要。摘要最多8条的显示策略不能变成静默丢掉其余真实失败日志。没有具体entry路径时记录已知目录/序号，不编造路径。

取消前未开始的工作是正常状态；晚到取消不能把已经产生的真实失败改写为Canceled并吞掉。保留批量128条/时间预算、CPU交接完成后再结束Job、snapshot生命周期与大对象释放策略。

## 诊断承接与调用方迁移

错误采用领域分类和统一诊断包装；同一次失败的 ID、source 与原始 Backtrace 在包装/跨线程传递时保持。源头 ERROR 不输出完整栈，纯传播不重复记录；真正消费失败后 DEBUG 记录实际措施，随后诊断可以释放。只有未处理错误在 Bevy 要求的边界显式转为 Severity::Error，最终由宿主输出原栈。正常等待、取消、合法 No-op 不人为转换为错误。

本增量改变的公开签名、回调或结果形状，必须在同一个增量更新所有直接消费者、测试、examples/benches 和所需 Facade 导出。不能用兼容 String 的临时入口掩盖未迁移调用，也不能把基本编译修复推迟到最终 Gallery 增量。

## 完成条件

- 真实枚举IO故障保留ErrorKind和路径；部分条目失败有ERROR+实际跳过DEBUG而不输出堆栈。
- 协作取消/Busy/Pending无伪ERROR，真实断连不能伪装成正常取消。
- 后台和CPU往返后origin相同；终结之前未消费的失败不只留String。
- 大目录/慢IO现有确定性工作量、快照保留和取消收割回归通过。

在 Windows x86_64-pc-windows-msvc / PowerShell 7 下执行本项相关测试：

```powershell
cargo test -p bevy_widgetry_file_dialog
```

对本项实际改动的包与直接消费者进行编译/Clippy 检查；触及 Gallery 时至少检查其编译。GUI、后台时序或性能敏感路径按本节具体场景复验，不重复运行未受影响的整套全仓验收。

同步目录部分成功、取消与后台错误的覆盖模型。

代码、构建或测试行为变更按适用工程规则完成必要的独立 Code Review；修复和回归仍归入本增量。

## 保持的边界

只迁移错误表示、传递、处理记录及直接受影响的调用。允许破坏性 API 更改，但不改变既有正常交互、状态通知、资源归属或业务算法；不加入与该错误路径无关的重构。

## 顺序衔接

前一项：[44 为 FileDialog 状态与公开操作建立领域错误](44-file-dialog-contract-errors.md)；后一项：[46 为 FileDialog 单次文件操作保留请求诊断](46-file-dialog-operation-errors.md)。编号规定唯一执行顺序，技术依赖以上文列出的能力为准。
