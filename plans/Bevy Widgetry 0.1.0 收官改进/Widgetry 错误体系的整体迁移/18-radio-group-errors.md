# 为 RadioGroup 结构与选择失败保留归属

将 RadioGroup 的 Group/Option 结构失败与选择约束改为领域诊断，保留现有自动修复与独占选择语义。

## 修改落点与前置能力

涉及：`crates/radio_group/src/group.rs`、`crates/radio_group/src/option.rs`、`crates/radio_group/src/group_style.rs`、`crates/radio_group/src/option_style.rs`、`crates/radio_group/src/colors.rs`。

实际前置为[05 区分持续异常状态与新的独立失败](05-persistent-failure-state.md)、[07 统一 Core 共享状态操作的错误上下文](07-core-state-errors.md)。这些引用表示本项直接使用的能力，不要求相邻编号之间存在调用关系。

## 结构拥有者与选择请求

父 Group 不合法、Option 所属冲突、必要结构缺失、非法选择请求分别保留 Group、Option、目标值/索引和阶段。只有已确认属于控件的结构损坏才是 invariant 失败；普通事件祖先过滤保持正常。

GroupDiagnostics/OwnershipDiagnostics 使用稳定的状态键。initialize 继续检查多个 Group 时不能丢弃首项之后的错误；公共批量载体返回每个原诊断。

## 不改变选择修复

复用 Core exclusive_selection 的历史回退和 Checked 提交，不因为引入错误类型改变删除选项后的合法回退、初始选择或同值设置通知。下层 Core 原错误补充 Group 上下文而不重打ERROR；选择 API 的直接消费者同次适配。

## 诊断承接与调用方迁移

错误采用领域分类和统一诊断包装；同一次失败的 ID、source 与原始 Backtrace 在包装/跨线程传递时保持。源头 ERROR 不输出完整栈，纯传播不重复记录；真正消费失败后 DEBUG 记录实际措施，随后诊断可以释放。只有未处理错误在 Bevy 要求的边界显式转为 Severity::Error，最终由宿主输出原栈。正常等待、取消、合法 No-op 不人为转换为错误。

本增量改变的公开签名、回调或结果形状，必须在同一个增量更新所有直接消费者、测试、examples/benches 和所需 Facade 导出。不能用兼容 String 的临时入口掩盖未迁移调用，也不能把基本编译修复推迟到最终 Gallery 增量。

## 完成条件

- 多个非法父子归属同时发生全部被捕获；诊断可区分 Group 与 Option。
- 删除/重排/禁用触发的合法回退和原先允许的无选择场景不被误报。
- 同步、queued 和初始化路径严重程度/ID及状态通知一致。

在 Windows x86_64-pc-windows-msvc / PowerShell 7 下执行本项相关测试：

```powershell
cargo test -p bevy_widgetry_radio_group
```

对本项实际改动的包与直接消费者进行编译/Clippy 检查；触及 Gallery 时至少检查其编译。GUI、后台时序或性能敏感路径按本节具体场景复验，不重复运行未受影响的整套全仓验收。

同步 RadioGroup 结构及选择契约和 COVERAGE。

代码、构建或测试行为变更按适用工程规则完成必要的独立 Code Review；修复和回归仍归入本增量。

## 保持的边界

只迁移错误表示、传递、处理记录及直接受影响的调用。允许破坏性 API 更改，但不改变既有正常交互、状态通知、资源归属或业务算法；不加入与该错误路径无关的重构。

## 顺序衔接

前一项：[17 让 CheckBox 批量样式失败全部到达兜底](17-check-box-errors.md)；后一项：[19 为 Segmented 的严格单选结构接入诊断](19-segmented-errors.md)。编号规定唯一执行顺序，技术依赖以上文列出的能力为准。
