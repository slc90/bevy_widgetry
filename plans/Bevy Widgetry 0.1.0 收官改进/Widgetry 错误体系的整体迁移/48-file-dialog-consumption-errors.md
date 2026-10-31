# 明确 FileDialog 运行时与界面的错误消费责任

把后台保真后的失败落实到Runtime和界面Owner：能够安全呈现/降级的真实失败记录处理DEBUG，其余原样送到ECS；清理资源不等于已经处理。

## 修改落点与前置能力

涉及：`crates/file_dialog/src/runtime.rs`、`crates/file_dialog/src/retention.rs`、`crates/file_dialog/src/behavior.rs`、`crates/file_dialog/src/controls.rs`、`crates/file_dialog/src/input.rs`、`crates/file_dialog/src/view.rs`、`crates/file_dialog/src/window.rs`、`crates/file_dialog/src/style.rs`、`crates/file_dialog/src/style/`、`crates/file_dialog/src/colors.rs`。

实际前置为[32 让 ComboBox 组合失败继承下层诊断](32-combo-box-errors.md)、[41 为 MessageBox 解析与样式失败保留诊断](41-message-box-errors.md)、[45 保留目录枚举与快照流水线中的真实失败](45-file-dialog-directory-errors.md)、[46 为 FileDialog 单次文件操作保留请求诊断](46-file-dialog-operation-errors.md)、[47 保留 FileDialog 偏好持久化与结果递送双重失败](47-file-dialog-preference-errors.md)。这些引用表示本项直接使用的能力，不要求相邻编号之间存在调用关系。

## 后台服务与回执消费

PreparedJob::poll不能把JobAccessError/TryRecvError转成contract_error(&error.to_string())。保留原Job诊断，只补FileDialog会话/操作；正常Empty与完成状态不报错。Runtime状态中的error字符串可显示，但还未被消费的诊断必须有Owner。

启动Retention失败、无服务降级、投影/选择/目录回执处理、retired job清理、wake失败分别定义实际结局。显示Failed、放弃操作、恢复交互可用状态后才记录处理DEBUG；继续返回Err的路径不记handled。移出World的Runtime即使失败也必须放回，不能因新早退丢资源。

## 组合界面失败

Window、MessageBox、ListView、ComboBox、TextField和Icon来源错误保留原identity，只增加FileDialog root/session/控制部件。真实内部缺失与过期UI事件过滤分开。多会话循环和清理同时失败时保留全部诊断。

样式/Owner模块的人工widgetry_error和字符串BevyError按领域入口迁移，旧contract_error/LoggedError剩余调用在本增量退出FileDialog。预算、公平轮询、快照释放和窗口关闭结果顺序不能被改写。

## 诊断承接与调用方迁移

错误采用领域分类和统一诊断包装；同一次失败的 ID、source 与原始 Backtrace 在包装/跨线程传递时保持。源头 ERROR 不输出完整栈，纯传播不重复记录；真正消费失败后 DEBUG 记录实际措施，随后诊断可以释放。只有未处理错误在 Bevy 要求的边界显式转为 Severity::Error，最终由宿主输出原栈。正常等待、取消、合法 No-op 不人为转换为错误。

本增量改变的公开签名、回调或结果形状，必须在同一个增量更新所有直接消费者、测试、examples/benches 和所需 Facade 导出。不能用兼容 String 的临时入口掩盖未迁移调用，也不能把基本编译修复推迟到最终 Gallery 增量。

## 完成条件

- 服务启动失败后没有虚假的“已处理”；完成安全降级后DEBUG关联原ID。
- 多个会话失败/清理失败均保留，正常Empty/过期回执/手动模式不误报。
- 大目录/偏好/候选/关闭的既有状态机回归通过，Runtime移出再放回的失败路径得到确定性保护。
- GUI/BRP验证错误提示后仍可进行既有恢复操作；正常场景不新增持续日志或轮询。

在 Windows x86_64-pc-windows-msvc / PowerShell 7 下执行本项相关测试：

```powershell
cargo test -p bevy_widgetry_file_dialog
```

对本项实际改动的包与直接消费者进行编译/Clippy 检查；触及 Gallery 时至少检查其编译。GUI、后台时序或性能敏感路径按本节具体场景复验，不重复运行未受影响的整套全仓验收。

同步FileDialog运行时消费责任、界面错误与COVERAGE。

代码、构建或测试行为变更按适用工程规则完成必要的独立 Code Review；修复和回归仍归入本增量。

## 保持的边界

只迁移错误表示、传递、处理记录及直接受影响的调用。允许破坏性 API 更改，但不改变既有正常交互、状态通知、资源归属或业务算法；不加入与该错误路径无关的重构。

## 顺序衔接

前一项：[47 保留 FileDialog 偏好持久化与结果递送双重失败](47-file-dialog-preference-errors.md)；后一项：[49 为 Waveform 数据契约保留可匹配错误](49-waveform-read-errors.md)。编号规定唯一执行顺序，技术依赖以上文列出的能力为准。
