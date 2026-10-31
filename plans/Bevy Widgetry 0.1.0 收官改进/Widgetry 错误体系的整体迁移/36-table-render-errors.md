# 让 Table 虚拟化与单元格展开完整传播失败

迁移Table渲染器和视口投影中的结构失败，保留row/column及renderer类型语境，并避免首错之外的诊断丢失。

## 修改落点与前置能力

涉及：`crates/table/src/projection.rs`、`crates/table/src/registration.rs`、`crates/table/src/viewport.rs`、`crates/table/src/view.rs`、`crates/table/src/interaction.rs`、`crates/table/src/selection_drag.rs`、`crates/table/src/header.rs`、`crates/table/src/style.rs`、`crates/table/src/colors.rs`。

实际前置为[04 在 BSN 与 Scene 展开中保留原始诊断](04-scene-diagnostics.md)、[05 区分持续异常状态与新的独立失败](05-persistent-failure-state.md)、[35 保留 Table 模型与投影请求的行列现场](35-table-model-errors.md)。这些引用表示本项直接使用的能力，不要求相邻编号之间存在调用关系。

## 从模型到渲染实体

required/invariant类辅助在确定需要部件时创建有实际字段的失败，包含view、source、row/column、区域(冻结/普通/表头)与部件。用户注册renderer的失败继承原source，Scene展开补单元格位置，不能format后重建。

多行列或多个View继续更新时聚合所有未处理错误；合法无可见区域、空单元格、无目标事件和初始布局等待保持正常状态。

## 渲染与输入合同不变

保持虚拟化、冻结区域、选择拖动、header动作和大小调整的既有行为。renderer展开失败只清理新建实体，不以错误机制改写数据Model或假定全表事务回滚。

## 诊断承接与调用方迁移

错误采用领域分类和统一诊断包装；同一次失败的 ID、source 与原始 Backtrace 在包装/跨线程传递时保持。源头 ERROR 不输出完整栈，纯传播不重复记录；真正消费失败后 DEBUG 记录实际措施，随后诊断可以释放。只有未处理错误在 Bevy 要求的边界显式转为 Severity::Error，最终由宿主输出原栈。正常等待、取消、合法 No-op 不人为转换为错误。

本增量改变的公开签名、回调或结果形状，必须在同一个增量更新所有直接消费者、测试、examples/benches 和所需 Facade 导出。不能用兼容 String 的临时入口掩盖未迁移调用，也不能把基本编译修复推迟到最终 Gallery 增量。

## 完成条件

- 不同单元格的renderer/source失败在同帧全部保留row/column、ID和source栈。
- 破坏已拥有的表头/viewport/内容实体产生真实错误，普通事件过滤不误报。
- 冻结区、滚动、选择和重建清理的原回归通过，健康区域继续更新。

在 Windows x86_64-pc-windows-msvc / PowerShell 7 下执行本项相关测试：

```powershell
cargo test -p bevy_widgetry_table
```

对本项实际改动的包与直接消费者进行编译/Clippy 检查；触及 Gallery 时至少检查其编译。GUI、后台时序或性能敏感路径按本节具体场景复验，不重复运行未受影响的整套全仓验收。

同步 TableRenderer与虚拟化错误及COVERAGE。

代码、构建或测试行为变更按适用工程规则完成必要的独立 Code Review；修复和回归仍归入本增量。

## 保持的边界

只迁移错误表示、传递、处理记录及直接受影响的调用。允许破坏性 API 更改，但不改变既有正常交互、状态通知、资源归属或业务算法；不加入与该错误路径无关的重构。

## 顺序衔接

前一项：[35 保留 Table 模型与投影请求的行列现场](35-table-model-errors.md)；后一项：[37 保留 Table 复制回调和剪贴板失败的原因链](37-table-clipboard-errors.md)。编号规定唯一执行顺序，技术依赖以上文列出的能力为准。
