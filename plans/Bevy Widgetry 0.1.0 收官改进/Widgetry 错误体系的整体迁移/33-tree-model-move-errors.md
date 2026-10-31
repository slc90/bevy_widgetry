# 为 Tree 节点移动保留结构化拒绝原因

替换movement::resolve的静态字符串原因，保留TreeModel、节点和目标的完整关系，使程序化移动失败可匹配。

## 修改落点与前置能力

涉及：`crates/tree/src/model.rs`、`crates/tree/src/source.rs`、`crates/tree/src/movement.rs`、`crates/tree/src/lib.rs`、`调用节点移动与TreeModel接口的位置`。

实际前置为[30 让 ListView 数据与选择请求使用领域失败](30-list-model-errors.md)、[31 保留 ListView 虚拟化与渲染的完整失败集](31-list-runtime-errors.md)。这些引用表示本项直接使用的能力，不要求相邻编号之间存在调用关系。

## 移动计划的验证

resolve中的无效view/source、缺内部list、自身移动、跨树节点、disabled路径、缺父级/索引、未加载children、循环父链分别形成领域原因，并携带view/source/moving/target/placement及已知父级。不要把完整失败仅写成Tree node move rejected: reason。

validate/resolve内部可以使用纯拒绝分类作为计算结果；在真正的公开操作失败处形成诊断，避免拖放预览每次试探位置都产生ERROR。合法交互guard/不接受的拖放候选和公开调用违反前置条件要按原契约区分。

## 提交与下层失败

保持先验证MovePlan再修改层级的顺序；同父同索引Ok(false)。draggable/set_draggable使用一致分类，缺内部List与无效Root不再一处有日志一处无日志。调用ListView失败继承origin；queued接口负责Bevy转换。

## 诊断承接与调用方迁移

错误采用领域分类和统一诊断包装；同一次失败的 ID、source 与原始 Backtrace 在包装/跨线程传递时保持。源头 ERROR 不输出完整栈，纯传播不重复记录；真正消费失败后 DEBUG 记录实际措施，随后诊断可以释放。只有未处理错误在 Bevy 要求的边界显式转为 Severity::Error，最终由宿主输出原栈。正常等待、取消、合法 No-op 不人为转换为错误。

本增量改变的公开签名、回调或结果形状，必须在同一个增量更新所有直接消费者、测试、examples/benches 和所需 Facade 导出。不能用兼容 String 的临时入口掩盖未迁移调用，也不能把基本编译修复推迟到最终 Gallery 增量。

## 完成条件

- 自身/后代循环、跨Model、未加载目标、损坏parent/index分别匹配原因与对象字段；失败不改变树层级。
- 合法no-op和拖放预览过滤无错误日志，实际非法API请求有源头ERROR。
- 下层ListView配置错误不重复记录，真实queue路径携带原栈。

在 Windows x86_64-pc-windows-msvc / PowerShell 7 下执行本项相关测试：

```powershell
cargo test -p bevy_widgetry_tree
```

对本项实际改动的包与直接消费者进行编译/Clippy 检查；触及 Gallery 时至少检查其编译。GUI、后台时序或性能敏感路径按本节具体场景复验，不重复运行未受影响的整套全仓验收。

同步 TreeModel/移动契约和结构不变量覆盖。

代码、构建或测试行为变更按适用工程规则完成必要的独立 Code Review；修复和回归仍归入本增量。

## 保持的边界

只迁移错误表示、传递、处理记录及直接受影响的调用。允许破坏性 API 更改，但不改变既有正常交互、状态通知、资源归属或业务算法；不加入与该错误路径无关的重构。

## 顺序衔接

前一项：[32 让 ComboBox 组合失败继承下层诊断](32-combo-box-errors.md)；后一项：[34 为 Tree 视图与 Renderer 失败保留节点身份](34-tree-view-errors.md)。编号规定唯一执行顺序，技术依赖以上文列出的能力为准。
