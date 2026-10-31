# 为 Canvas 文档与几何操作建立领域诊断

让Canvas自己的几何/文档契约错误保留形状、变换和操作现场，不改写编辑工具行为或模型结构。

## 修改落点与前置能力

涉及：`crates/canvas/src/model.rs`、`crates/canvas/src/geometry.rs`、`crates/canvas/src/path.rs`、`crates/canvas/src/parameters.rs`、`crates/canvas/src/batch.rs`、`crates/canvas/src/order.rs`、`crates/canvas/src/selection.rs`、`crates/canvas/src/nodes.rs`、`crates/canvas/src/move_selection.rs`、`crates/canvas/src/rotate.rs`、`crates/canvas/src/scale.rs`、`crates/canvas/src/pen.rs`、`crates/canvas/src/view.rs`、`crates/canvas/src/renderer.rs`、`crates/canvas/src/mesh_cache.rs`、`crates/canvas/src/input.rs`。

实际前置为[04 在 BSN 与 Scene 展开中保留原始诊断](04-scene-diagnostics.md)、[05 区分持续异常状态与新的独立失败](05-persistent-failure-state.md)、[08 使离屏目标失败保留资源归属与清理结果](08-offscreen-errors.md)。这些引用表示本项直接使用的能力，不要求相邻编号之间存在调用关系。

## 结构、参数和几何来源

文档不存在、形状归属/排序结构损坏、非法Rectangle/Ellipse/Path参数、非有限变换/绘制配置和几何细分失败分别建模。保存document/shape/view、参数和具体命令/段位置；第三方几何错误保留真实source或其原结构化错误数据，不只String。

公开模型/编辑操作返回领域失败，Bevy输入/渲染入口在边界转换。Core offscreen和Scene错误沿原origin补Canvas上下文。多shape绘制/重建工作不丢掉首项之后的失败。

## 编辑中间态仍是正常状态

空选择、普通工具取消、未提交Pen draft、合法零工作量与无目标输入按照现有合同保持正常，不能把未完成图形当作非法持久模型。

保持先校验再提交、绘制缓存/排序和通知的不变量。发生后续投影失败时准确保留提交阶段，不扩大成事务回滚或重构工具机制。SVG/PNG输出分别由后续增量负责。

## 诊断承接与调用方迁移

错误采用领域分类和统一诊断包装；同一次失败的 ID、source 与原始 Backtrace 在包装/跨线程传递时保持。源头 ERROR 不输出完整栈，纯传播不重复记录；真正消费失败后 DEBUG 记录实际措施，随后诊断可以释放。只有未处理错误在 Bevy 要求的边界显式转为 Severity::Error，最终由宿主输出原栈。正常等待、取消、合法 No-op 不人为转换为错误。

本增量改变的公开签名、回调或结果形状，必须在同一个增量更新所有直接消费者、测试、examples/benches 和所需 Facade 导出。不能用兼容 String 的临时入口掩盖未迁移调用，也不能把基本编译修复推迟到最终 Gallery 增量。

## 完成条件

- 非法几何/变换/归属可匹配并包含document/shape/字段，第三方source保持。
- 多个shape/owner失败在允许继续的路径完整上报；健康几何与缓存不受无关错误影响。
- Pen中间态、选择/移动/旋转/缩放的合法操作与回退/通知回归保持。

在 Windows x86_64-pc-windows-msvc / PowerShell 7 下执行本项相关测试：

```powershell
cargo test -p bevy_widgetry_canvas
```

对本项实际改动的包与直接消费者进行编译/Clippy 检查；触及 Gallery 时至少检查其编译。GUI、后台时序或性能敏感路径按本节具体场景复验，不重复运行未受影响的整套全仓验收。

同步Canvas模型/几何/编辑错误与COVERAGE。

代码、构建或测试行为变更按适用工程规则完成必要的独立 Code Review；修复和回归仍归入本增量。

## 保持的边界

只迁移错误表示、传递、处理记录及直接受影响的调用。允许破坏性 API 更改，但不改变既有正常交互、状态通知、资源归属或业务算法；不加入与该错误路径无关的重构。

## 顺序衔接

前一项：[53 保留磁盘 Waveform Reader 跨线程失败](53-waveform-disk-reader-errors.md)；后一项：[55 保留 Canvas SVG 导出的完整原因链](55-canvas-svg-errors.md)。编号规定唯一执行顺序，技术依赖以上文列出的能力为准。
