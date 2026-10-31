# 让 ComboBox 组合失败继承下层诊断

以ComboBox自身的source/选中请求为领域入口，并完整保留ListView、TextField和Icon的错误身份。

## 修改落点与前置能力

涉及：`crates/combo_box/src/combo_box.rs`、`crates/combo_box/src/field.rs`、`crates/combo_box/src/popup.rs`、`crates/combo_box/src/search.rs`、`crates/combo_box/src/style.rs`、`crates/combo_box/src/colors.rs`、`crates/combo_box/src/registration.rs`。

实际前置为[06 为 Icon 与 SVG 失败保留资产现场](06-core-icon-errors.md)、[26 为 TextField 内容与编辑器错误保留映射](26-text-field-errors.md)、[31 保留 ListView 虚拟化与渲染的完整失败集](31-list-runtime-errors.md)。这些引用表示本项直接使用的能力，不要求相邻编号之间存在调用关系。

## Root、source与内部List的映射

update_error(root,reason)替换为具体的无效Root/模型类型/内部List缺失/选中ID不存在等变体，保存source、list、item_id或index。下层选择或文本操作失败只补combo根与操作，不再创建另一个源头。

ComboDiagnostics的projection/icon与公共持续异常语义一致。源头为ListView或TextField时，ComboBox只负责传播或实际收起/降级后的DEBUG，不因为看到了Err就记已处理。

## 事件顺序不随迁移改变

保留ForwardedChanges、queue_pending_changes、PreservePopup及源ID解析的排序约束。选择同值、关闭弹层、无搜索结果、disabled、失去焦点等正常状态不转为错误；内部已确认映射被破坏才报告。

## 诊断承接与调用方迁移

错误采用领域分类和统一诊断包装；同一次失败的 ID、source 与原始 Backtrace 在包装/跨线程传递时保持。源头 ERROR 不输出完整栈，纯传播不重复记录；真正消费失败后 DEBUG 记录实际措施，随后诊断可以释放。只有未处理错误在 Bevy 要求的边界显式转为 Severity::Error，最终由宿主输出原栈。正常等待、取消、合法 No-op 不人为转换为错误。

本增量改变的公开签名、回调或结果形状，必须在同一个增量更新所有直接消费者、测试、examples/benches 和所需 Facade 导出。不能用兼容 String 的临时入口掩盖未迁移调用，也不能把基本编译修复推迟到最终 Gallery 增量。

## 完成条件

- 无效source/selection和内部list映射失败包含具体实体，跨TextField/ListView后ID不变。
- 弹层、搜索、选择提交与父观察者排队修改的事件顺序保持。
- 重复观察同一状态不重复ERROR，独立新请求不能被此前失败去重。

在 Windows x86_64-pc-windows-msvc / PowerShell 7 下执行本项相关测试：

```powershell
cargo test -p bevy_widgetry_combo_box
```

对本项实际改动的包与直接消费者进行编译/Clippy 检查；触及 Gallery 时至少检查其编译。GUI、后台时序或性能敏感路径按本节具体场景复验，不重复运行未受影响的整套全仓验收。

同步 ComboBox 组合接口与跨控件错误覆盖。

代码、构建或测试行为变更按适用工程规则完成必要的独立 Code Review；修复和回归仍归入本增量。

## 保持的边界

只迁移错误表示、传递、处理记录及直接受影响的调用。允许破坏性 API 更改，但不改变既有正常交互、状态通知、资源归属或业务算法；不加入与该错误路径无关的重构。

## 顺序衔接

前一项：[31 保留 ListView 虚拟化与渲染的完整失败集](31-list-runtime-errors.md)；后一项：[33 为 Tree 节点移动保留结构化拒绝原因](33-tree-model-move-errors.md)。编号规定唯一执行顺序，技术依赖以上文列出的能力为准。
