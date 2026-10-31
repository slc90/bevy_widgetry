# 为 FileDialog 状态与公开操作建立领域错误

用明确的契约错误替换contract_error(message)，把会话、token和操作上下文带进返回值，先稳定FileDialog公共状态层。

## 修改落点与前置能力

涉及：`crates/file_dialog/src/model.rs`、`crates/file_dialog/src/api.rs`、`crates/file_dialog/src/filter.rs`、`crates/file_dialog/src/selection.rs`、`crates/file_dialog/src/storage.rs`、`crates/file_dialog/src/candidate.rs`。

实际前置为[04 在 BSN 与 Scene 展开中保留原始诊断](04-scene-diagnostics.md)、[12 让后台 Job 的失败跨线程保持原诊断](12-job-outcome-diagnostics.md)、[32 让 ComboBox 组合失败继承下层诊断](32-combo-box-errors.md)、[41 为 MessageBox 解析与样式失败保留诊断](41-message-box-errors.md)。这些引用表示本项直接使用的能力，不要求相邻编号之间存在调用关系。

## 公开前置条件与内部状态

scope长度、filter容量/重复ID、session ID耗尽、无效Root、缺少状态/运行资源、独立窗口不允许Reopen等分别保留Root、session/token、action与实际配置。来源不是契约错误的Job/I/O故障不能再次包装成“FileDialog拒绝无效操作”。

new/act/receive/apply等普通API返回领域失败；Template不再通过logged_error(error.to_string())重建，queued API使用统一桥。Token不匹配、已关闭会话、已销毁目标等既有合法过期结果仍返回正常无变化结果。

## 状态字段不是唯一诊断

Directory/Confirmation/Storage等面向UI的Failed文本可以是已处理结果的显示投影，但真正错误在进入状态前必须保留诊断并由实际处理Owner完成DEBUG。只把String塞进状态、继续返回Ok，不足以证明已处理。

本增量定义领域类型和公共结果契约，并适配直接调用；后台运输的保真与具体消费结果由后续FileDialog增量完成，不添加旧签名兼容层。

## 诊断承接与调用方迁移

错误采用领域分类和统一诊断包装；同一次失败的 ID、source 与原始 Backtrace 在包装/跨线程传递时保持。源头 ERROR 不输出完整栈，纯传播不重复记录；真正消费失败后 DEBUG 记录实际措施，随后诊断可以释放。只有未处理错误在 Bevy 要求的边界显式转为 Severity::Error，最终由宿主输出原栈。正常等待、取消、合法 No-op 不人为转换为错误。

本增量改变的公开签名、回调或结果形状，必须在同一个增量更新所有直接消费者、测试、examples/benches 和所需 Facade 导出。不能用兼容 String 的临时入口掩盖未迁移调用，也不能把基本编译修复推迟到最终 Gallery 增量。

## 完成条件

- 所有公开非法参数/状态能匹配领域类别，包含相关ID和数值；正常no-op/过期token无ERROR。
- Scene初始化失败保留新origin、清理符合Core契约。
- 状态克隆和日志关联不要求diagnostic参与Eq，直接Gallery/Window/控件组合调用编译。

在 Windows x86_64-pc-windows-msvc / PowerShell 7 下执行本项相关测试：

```powershell
cargo test -p bevy_widgetry_file_dialog
```

对本项实际改动的包与直接消费者进行编译/Clippy 检查；触及 Gallery 时至少检查其编译。GUI、后台时序或性能敏感路径按本节具体场景复验，不重复运行未受影响的整套全仓验收。

同步 FileDialog API/状态与错误分类覆盖。

代码、构建或测试行为变更按适用工程规则完成必要的独立 Code Review；修复和回归仍归入本增量。

## 保持的边界

只迁移错误表示、传递、处理记录及直接受影响的调用。允许破坏性 API 更改，但不改变既有正常交互、状态通知、资源归属或业务算法；不加入与该错误路径无关的重构。

## 顺序衔接

前一项：[43 保留电源请求主失败与清理失败的各自原因](43-power-inhibit-errors.md)；后一项：[45 保留目录枚举与快照流水线中的真实失败](45-file-dialog-directory-errors.md)。编号规定唯一执行顺序，技术依赖以上文列出的能力为准。
