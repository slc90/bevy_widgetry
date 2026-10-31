# 保留 Table 模型与投影请求的行列现场

将TableModel及视图状态操作的错误改为携带稳定行列身份的领域失败，保留合法缺失查询。

## 修改落点与前置能力

涉及：`crates/table/src/model.rs`、`crates/table/src/view_state.rs`、`crates/table/src/layout.rs`、`crates/table/src/selection.rs`、`crates/table/src/column_reorder.rs`、`crates/table/src/resize.rs`。

实际前置为[02 保留诊断身份地进入 Bevy 错误边界](02-bevy-error-bridge.md)、[03 让测试能观察完整错误生命周期](03-diagnostic-test-support.md)、[20 区分 ScrollArea 的布局等待与结构失败](20-scroll-area-errors.md)。这些引用表示本项直接使用的能力，不要求相邻编号之间存在调用关系。

## 模型操作与布局参数

行/列插入越界、ID和revision耗尽、非法布局宽度、排序/选择投影请求分别保留row/column ID、index、len、revision和实际配置。Arc<dyn Any>的单元格数据仍保持原业务类型，诊断不复制整个数据集。

row/column/remove和原来合法返回None/false的API不机械改成Err。普通API返回领域失败，Commands边界显式转换；各renderer/editor/clipboard/Gallery调用在本增量改签名但不改业务。

## 失败后的不变量

保持稳定ID、model instance、结构版本、索引映射和selection range的原约束。只为当前确实可能失败的路径补错误，不为无错误的常量、映射、正常排序结果创建新抽象。

## 诊断承接与调用方迁移

错误采用领域分类和统一诊断包装；同一次失败的 ID、source 与原始 Backtrace 在包装/跨线程传递时保持。源头 ERROR 不输出完整栈，纯传播不重复记录；真正消费失败后 DEBUG 记录实际措施，随后诊断可以释放。只有未处理错误在 Bevy 要求的边界显式转为 Severity::Error，最终由宿主输出原栈。正常等待、取消、合法 No-op 不人为转换为错误。

本增量改变的公开签名、回调或结果形状，必须在同一个增量更新所有直接消费者、测试、examples/benches 和所需 Facade 导出。不能用兼容 String 的临时入口掩盖未迁移调用，也不能把基本编译修复推迟到最终 Gallery 增量。

## 完成条件

- 行列插入/revision/ID与布局错误包含实际字段，失败后索引和稳定ID不损坏。
- 空表、正常缺失、移除/重排/同值更改按原契约无ERROR。
- 泛型模型和注册回调调用方编译，原属性测试保留。

在 Windows x86_64-pc-windows-msvc / PowerShell 7 下执行本项相关测试：

```powershell
cargo test -p bevy_widgetry_table
```

对本项实际改动的包与直接消费者进行编译/Clippy 检查；触及 Gallery 时至少检查其编译。GUI、后台时序或性能敏感路径按本节具体场景复验，不重复运行未受影响的整套全仓验收。

同步 TableModel、布局和投影API与对应覆盖。

代码、构建或测试行为变更按适用工程规则完成必要的独立 Code Review；修复和回归仍归入本增量。

## 保持的边界

只迁移错误表示、传递、处理记录及直接受影响的调用。允许破坏性 API 更改，但不改变既有正常交互、状态通知、资源归属或业务算法；不加入与该错误路径无关的重构。

## 顺序衔接

前一项：[34 为 Tree 视图与 Renderer 失败保留节点身份](34-tree-view-errors.md)；后一项：[36 让 Table 虚拟化与单元格展开完整传播失败](36-table-render-errors.md)。编号规定唯一执行顺序，技术依赖以上文列出的能力为准。
