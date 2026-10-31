# 让 Tabs 配对结构失败保持实体关系

迁移Tabs初始化和程序化选择错误，保留固定Header/Page配对、常驻页面和滚动层组合。

## 修改落点与前置能力

涉及：`crates/tabs/src/selection.rs`、`crates/tabs/src/tabs.rs`、`crates/tabs/src/scrolling.rs`、`crates/tabs/src/input.rs`、`crates/tabs/src/style.rs`、`crates/tabs/src/colors.rs`。

实际前置为[04 在 BSN 与 Scene 展开中保留原始诊断](04-scene-diagnostics.md)、[20 区分 ScrollArea 的布局等待与结构失败](20-scroll-area-errors.md)。这些引用表示本项直接使用的能力，不要求相邻编号之间存在调用关系。

## 配对约束的诊断字段

structure/prepare 中的Root/parts/container/header/page缺失、错误父级、初始selected越界、构造后配对变化应分别表达，携带实际/预期父级、索引和数量。替换invalid(reason)与logged_error字符串通道。

initialize处理Pending Roots时保留每个失败；Scene与ScrollArea/Core错误继承原ID，不在每层重复源头日志。配对迁移及选择投影已经写入的部分状态不自动具有事务回滚能力，诊断应指出失败阶段而非声称全部恢复。

## 诊断承接与调用方迁移

错误采用领域分类和统一诊断包装；同一次失败的 ID、source 与原始 Backtrace 在包装/跨线程传递时保持。源头 ERROR 不输出完整栈，纯传播不重复记录；真正消费失败后 DEBUG 记录实际措施，随后诊断可以释放。只有未处理错误在 Bevy 要求的边界显式转为 Severity::Error，最终由宿主输出原栈。正常等待、取消、合法 No-op 不人为转换为错误。

本增量改变的公开签名、回调或结果形状，必须在同一个增量更新所有直接消费者、测试、examples/benches 和所需 Facade 导出。不能用兼容 String 的临时入口掩盖未迁移调用，也不能把基本编译修复推迟到最终 Gallery 增量。

## 完成条件

- 非法配对/父级/索引和构造后改动分别验证现场；同时损坏两个Tabs时两个失败都到Handler。
- 正常选择、固定配对、页面隐藏、键盘/焦点与scroll-into-view不回归。
- 重复初始化和合法取消保持正常，源头Backtrace不被Scene边界覆盖。

在 Windows x86_64-pc-windows-msvc / PowerShell 7 下执行本项相关测试：

```powershell
cargo test -p bevy_widgetry_tabs
```

对本项实际改动的包与直接消费者进行编译/Clippy 检查；触及 Gallery 时至少检查其编译。GUI、后台时序或性能敏感路径按本节具体场景复验，不重复运行未受影响的整套全仓验收。

同步 Tabs 配对结构、错误传播和覆盖模型。

代码、构建或测试行为变更按适用工程规则完成必要的独立 Code Review；修复和回归仍归入本增量。

## 保持的边界

只迁移错误表示、传递、处理记录及直接受影响的调用。允许破坏性 API 更改，但不改变既有正常交互、状态通知、资源归属或业务算法；不加入与该错误路径无关的重构。

## 顺序衔接

前一项：[21 为 Splitter 比例与 Pane 失败保留数值现场](21-splitter-errors.md)；后一项：[23 为 Collapsible 内部结构失败保持组合来源](23-collapsible-errors.md)。编号规定唯一执行顺序，技术依赖以上文列出的能力为准。
