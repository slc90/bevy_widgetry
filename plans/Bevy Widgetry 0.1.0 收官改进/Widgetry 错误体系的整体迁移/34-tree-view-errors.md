# 为 Tree 视图与 Renderer 失败保留节点身份

迁移TreeView的投影、展开器和renderer错误，不改变懒加载children状态和ListView虚拟化组合。

## 修改落点与前置能力

涉及：`crates/tree/src/view.rs`、`crates/tree/src/view/style.rs`、`crates/tree/src/renderer.rs`、`crates/tree/src/behavior.rs`、`crates/tree/src/registration.rs`、`crates/tree/src/colors.rs`。

实际前置为[04 在 BSN 与 Scene 展开中保留原始诊断](04-scene-diagnostics.md)、[31 保留 ListView 虚拟化与渲染的完整失败集](31-list-runtime-errors.md)、[33 为 Tree 节点移动保留结构化拒绝原因](33-tree-model-move-errors.md)。这些引用表示本项直接使用的能力，不要求相邻编号之间存在调用关系。

## 视图上下文随失败传递

结构、节点renderer、expander及内部List的真实失败保留view/source/node/展开状态/部件实体。来自ListView和Scene的诊断不能被Tree重新String化，只补节点关系。

批量投影或样式工作继续处理多个节点时，保存每个未处理失败。children尚未加载、节点正常折叠或生命周期删除的合法状态保持原结果，不把等待当作恢复事件或错误。

## 跨层交互验证

保持展开/选择/active、keyboard和结构拖放桥接的既有逻辑。挂接式renderer失败清理只处理其本次创建实体；已写入业务Model不自动回滚。

## 诊断承接与调用方迁移

错误采用领域分类和统一诊断包装；同一次失败的 ID、source 与原始 Backtrace 在包装/跨线程传递时保持。源头 ERROR 不输出完整栈，纯传播不重复记录；真正消费失败后 DEBUG 记录实际措施，随后诊断可以释放。只有未处理错误在 Bevy 要求的边界显式转为 Severity::Error，最终由宿主输出原栈。正常等待、取消、合法 No-op 不人为转换为错误。

本增量改变的公开签名、回调或结果形状，必须在同一个增量更新所有直接消费者、测试、examples/benches 和所需 Facade 导出。不能用兼容 String 的临时入口掩盖未迁移调用，也不能把基本编译修复推迟到最终 Gallery 增量。

## 完成条件

- renderer/expander/List映射故障可定位node，多个错误全保留且不重复下层源头。
- 加载中、折叠、Model删除和展开回退仍按原契约运行。
- Tree与ListView组合的拖放、selection与焦点回归通过。

在 Windows x86_64-pc-windows-msvc / PowerShell 7 下执行本项相关测试：

```powershell
cargo test -p bevy_widgetry_tree
```

对本项实际改动的包与直接消费者进行编译/Clippy 检查；触及 Gallery 时至少检查其编译。GUI、后台时序或性能敏感路径按本节具体场景复验，不重复运行未受影响的整套全仓验收。

同步 TreeView renderer/投影说明与覆盖模型。

代码、构建或测试行为变更按适用工程规则完成必要的独立 Code Review；修复和回归仍归入本增量。

## 保持的边界

只迁移错误表示、传递、处理记录及直接受影响的调用。允许破坏性 API 更改，但不改变既有正常交互、状态通知、资源归属或业务算法；不加入与该错误路径无关的重构。

## 顺序衔接

前一项：[33 为 Tree 节点移动保留结构化拒绝原因](33-tree-model-move-errors.md)；后一项：[35 保留 Table 模型与投影请求的行列现场](35-table-model-errors.md)。编号规定唯一执行顺序，技术依赖以上文列出的能力为准。
