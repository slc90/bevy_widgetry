# 为 FileDialog 单次文件操作保留请求诊断

统一验证候选、创建目录和获取位置等单次后台操作的失败载体，并把业务拒绝与实际系统失败分开。

## 修改落点与前置能力

涉及：`crates/file_dialog/src/confirmation.rs`、`crates/file_dialog/src/folder.rs`、`crates/file_dialog/src/locations.rs`、`crates/file_dialog/src/filesystem.rs`、`crates/file_dialog/src/selection_task.rs`、`crates/file_dialog/src/runtime.rs 对应分支`、`crates/file_dialog/src/model.rs 回执类型`。

实际前置为[12 让后台 Job 的失败跨线程保持原诊断](12-job-outcome-diagnostics.md)、[44 为 FileDialog 状态与公开操作建立领域错误](44-file-dialog-contract-errors.md)、[45 保留目录枚举与快照流水线中的真实失败](45-file-dialog-directory-errors.md)。这些引用表示本项直接使用的能力，不要求相邻编号之间存在调用关系。

## 请求、回执与结果

回执中的Result<_,String>改为可保留原origin/source的结果。保存session/token、候选路径、目录操作、模式和实际失败阶段；多选候选可定位失败项而不复制整个目录snapshot。

保存目标不存在是可允许的候选判断，不是IO故障；正常取消、覆盖确认、非法用户输入导致的业务提示应由明确状态表达。真实权限/文件系统/Job递送失败属于诊断，不能为了显示提示先to_string丢掉对象。

## 处理发生在拥有行为的一层

从Job收到回执、将其排进待提交队列仅是传播。确认取消本次候选、恢复可编辑状态或放弃创建操作之后，实际Owner才记录DEBUG；未完成补救的错误继续到ECS。

过期回执不改变当前会话。若过期的是已经形成的真实失败，按明确废弃该旧操作的处理记录关联ID；不把过期正常结果制造成错误。

## 诊断承接与调用方迁移

错误采用领域分类和统一诊断包装；同一次失败的 ID、source 与原始 Backtrace 在包装/跨线程传递时保持。源头 ERROR 不输出完整栈，纯传播不重复记录；真正消费失败后 DEBUG 记录实际措施，随后诊断可以释放。只有未处理错误在 Bevy 要求的边界显式转为 Severity::Error，最终由宿主输出原栈。正常等待、取消、合法 No-op 不人为转换为错误。

本增量改变的公开签名、回调或结果形状，必须在同一个增量更新所有直接消费者、测试、examples/benches 和所需 Facade 导出。不能用兼容 String 的临时入口掩盖未迁移调用，也不能把基本编译修复推迟到最终 Gallery 增量。

## 完成条件

- 权限拒绝/创建失败/位置读取失败保留io/source与token，实际处理记录说明动作。
- 正常SavePath不存在、覆盖拒绝、用户取消和过期正常结果无ERROR。
- 多选验证失败项、晚回执和新的当前请求不会串ID或错误更新UI。

在 Windows x86_64-pc-windows-msvc / PowerShell 7 下执行本项相关测试：

```powershell
cargo test -p bevy_widgetry_file_dialog
```

对本项实际改动的包与直接消费者进行编译/Clippy 检查；触及 Gallery 时至少检查其编译。GUI、后台时序或性能敏感路径按本节具体场景复验，不重复运行未受影响的整套全仓验收。

同步单次后台操作与回执的失败/拒绝覆盖。

代码、构建或测试行为变更按适用工程规则完成必要的独立 Code Review；修复和回归仍归入本增量。

## 保持的边界

只迁移错误表示、传递、处理记录及直接受影响的调用。允许破坏性 API 更改，但不改变既有正常交互、状态通知、资源归属或业务算法；不加入与该错误路径无关的重构。

## 顺序衔接

前一项：[45 保留目录枚举与快照流水线中的真实失败](45-file-dialog-directory-errors.md)；后一项：[47 保留 FileDialog 偏好持久化与结果递送双重失败](47-file-dialog-preference-errors.md)。编号规定唯一执行顺序，技术依赖以上文列出的能力为准。
