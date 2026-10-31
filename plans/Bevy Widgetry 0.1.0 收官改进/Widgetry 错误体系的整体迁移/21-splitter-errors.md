# 为 Splitter 比例与 Pane 失败保留数值现场

把比例校验和已拥有Pane/Handle结构损坏区分开，并去掉 constrain_ratio 传播处的重复 ERROR。

## 修改落点与前置能力

涉及：`crates/splitter/src/ratio.rs`、`crates/splitter/src/splitter.rs`、`crates/splitter/src/layout.rs`、`crates/splitter/src/input.rs`、`crates/splitter/src/style.rs`、`crates/splitter/src/colors.rs`。

实际前置为[07 统一 Core 共享状态操作的错误上下文](07-core-state-errors.md)。这些引用表示本项直接使用的能力，不要求相邻编号之间存在调用关系。

## 数值与结构是不同失败

ratio.rs 的有限且[0,1]校验保留实际请求ratio。构造时的两个Pane、Handle及其关联校验保存Root和部件身份；layout返回的错误不能在ratio::commit再次记录一条相同ERROR。

公开同步setter返回领域结果，queue返回口显式桥接；颜色等Core来源失败只追加Splitter语境。拖动越界按既有约束clamp、未变化ratio和正常布局等待不产生错误。

## 诊断承接与调用方迁移

错误采用领域分类和统一诊断包装；同一次失败的 ID、source 与原始 Backtrace 在包装/跨线程传递时保持。源头 ERROR 不输出完整栈，纯传播不重复记录；真正消费失败后 DEBUG 记录实际措施，随后诊断可以释放。只有未处理错误在 Bevy 要求的边界显式转为 Severity::Error，最终由宿主输出原栈。正常等待、取消、合法 No-op 不人为转换为错误。

本增量改变的公开签名、回调或结果形状，必须在同一个增量更新所有直接消费者、测试、examples/benches 和所需 Facade 导出。不能用兼容 String 的临时入口掩盖未迁移调用，也不能把基本编译修复推迟到最终 Gallery 增量。

## 完成条件

- 非法ratio、缺Pane/Handle、布局归属破坏有明确分类和字段。
- layout失败经commit再经Command只有一条源头ERROR；Handler读到原origin。
- 原最小尺寸约束、拖动与最终ValueChange通知保持既有时序。

在 Windows x86_64-pc-windows-msvc / PowerShell 7 下执行本项相关测试：

```powershell
cargo test -p bevy_widgetry_splitter
```

对本项实际改动的包与直接消费者进行编译/Clippy 检查；触及 Gallery 时至少检查其编译。GUI、后台时序或性能敏感路径按本节具体场景复验，不重复运行未受影响的整套全仓验收。

同步 Splitter 数值/结构错误契约及相关测试。

代码、构建或测试行为变更按适用工程规则完成必要的独立 Code Review；修复和回归仍归入本增量。

## 保持的边界

只迁移错误表示、传递、处理记录及直接受影响的调用。允许破坏性 API 更改，但不改变既有正常交互、状态通知、资源归属或业务算法；不加入与该错误路径无关的重构。

## 顺序衔接

前一项：[20 区分 ScrollArea 的布局等待与结构失败](20-scroll-area-errors.md)；后一项：[22 让 Tabs 配对结构失败保持实体关系](22-tabs-errors.md)。编号规定唯一执行顺序，技术依赖以上文列出的能力为准。
