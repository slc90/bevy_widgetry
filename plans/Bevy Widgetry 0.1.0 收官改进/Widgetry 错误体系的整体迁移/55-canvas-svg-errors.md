# 保留 Canvas SVG 导出的完整原因链

让同步SVG导出保留已存在的I/O source，并消除InvalidGeometry(String)对下层诊断的抹除。

## 修改落点与前置能力

涉及：`crates/canvas/src/export.rs`、`crates/canvas/src/lib.rs`、`Gallery Canvas SVG导出调用`。

实际前置为[54 为 Canvas 文档与几何操作建立领域诊断](54-canvas-model-errors.md)。这些引用表示本项直接使用的能力，不要求相邻编号之间存在调用关系。

## 快照与文件输出

CanvasExportError现有Io/Encoding/Job等source处理有可保留价值，不应退化成纯文本。改用统一诊断包装；EmptyDocument、InvalidDocument、InvalidRegion、UnsupportedColor及几何校验分别保留document、shape、region、实际颜色和目标path。

snapshot调用model::validate_shape失败只增加导出语境，不to_string成InvalidGeometry。export_svg外层inspect_err不再重复源头日志。写文件错误在拥有path的首次观察处形成诊断。

## 同步失败的消费

保留先验证/生成SVG再写文件的既有顺序，保留参数矩形/椭圆/Path的真实SVG输出和颜色规则。不要附带改变文件覆盖/原子写行为；它不是本任务的存储重设计。

Gallery实际放弃/提示导出失败时才记录处理DEBUG；若继续向System返回则仅传播并保留原栈。

## 诊断承接与调用方迁移

错误采用领域分类和统一诊断包装；同一次失败的 ID、source 与原始 Backtrace 在包装/跨线程传递时保持。源头 ERROR 不输出完整栈，纯传播不重复记录；真正消费失败后 DEBUG 记录实际措施，随后诊断可以释放。只有未处理错误在 Bevy 要求的边界显式转为 Severity::Error，最终由宿主输出原栈。正常等待、取消、合法 No-op 不人为转换为错误。

本增量改变的公开签名、回调或结果形状，必须在同一个增量更新所有直接消费者、测试、examples/benches 和所需 Facade 导出。不能用兼容 String 的临时入口掩盖未迁移调用，也不能把基本编译修复推迟到最终 Gallery 增量。

## 完成条件

- model几何错误穿过snapshot/export仍同ID；颜色/region/path字段可用于诊断。
- IO失败保留io::Error，源头只一次；未处理或已处理路径日志区分正确。
- 三种图形的SVG几何与已有输出对照、边界颜色测试不回归。

在 Windows x86_64-pc-windows-msvc / PowerShell 7 下执行本项相关测试：

```powershell
cargo test -p bevy_widgetry_canvas
```

对本项实际改动的包与直接消费者进行编译/Clippy 检查；触及 Gallery 时至少检查其编译。GUI、后台时序或性能敏感路径按本节具体场景复验，不重复运行未受影响的整套全仓验收。

同步SVG导出类型与错误/输出覆盖。

代码、构建或测试行为变更按适用工程规则完成必要的独立 Code Review；修复和回归仍归入本增量。

## 保持的边界

只迁移错误表示、传递、处理记录及直接受影响的调用。允许破坏性 API 更改，但不改变既有正常交互、状态通知、资源归属或业务算法；不加入与该错误路径无关的重构。

## 顺序衔接

前一项：[54 为 Canvas 文档与几何操作建立领域诊断](54-canvas-model-errors.md)；后一项：[56 让 Canvas PNG 导出在 Render 与 Job 间保留诊断](56-canvas-png-errors.md)。编号规定唯一执行顺序，技术依赖以上文列出的能力为准。
