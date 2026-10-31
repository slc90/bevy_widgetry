# 区分 ScrollArea 的布局等待与结构失败

迁移滚动区域中真实的配置/内部部件错误，保持滚动消息过滤、未布局和零尺寸的正常行为。

## 修改落点与前置能力

涉及：`crates/scroll_area/src/layout.rs`、`crates/scroll_area/src/style.rs`、`crates/scroll_area/src/pointer.rs`、`crates/scroll_area/src/disabled.rs`、`crates/scroll_area/src/headless.rs`、`crates/scroll_area/src/colors.rs`。

实际前置为[05 区分持续异常状态与新的独立失败](05-persistent-failure-state.md)、[07 统一 Core 共享状态操作的错误上下文](07-core-state-errors.md)。这些引用表示本项直接使用的能力，不要求相邻编号之间存在调用关系。

## 只给真实契约失败建模

逐一核对 configure_geometry、solve_visibility、颜色Owner和scrollbar部件校验的失败路径。保存Root、Viewport、Content、轴、策略、缺失部件和相关尺寸；下层错误保留原source。

headless 的 keyboard/ScrollIntoView 查询不命中可能只是目标不属于滚动区域，不能统一改为Err。先验证归属，再判断必需部件是否损坏。未得到ComputedNode的合法初始化阶段和隐藏/零尺寸状态继续等待；已经建立的归属被破坏才报真实错误。

## 组合控件使用同一诊断

ListView、Tabs、Table、Waveform 的滚动层调用只增加自己的owner/request上下文，不把相同失败重新分类成另一源头。queued 操作和 System 返回口使用公共桥；多Owner更新保留全部未处理诊断。

## 诊断承接与调用方迁移

错误采用领域分类和统一诊断包装；同一次失败的 ID、source 与原始 Backtrace 在包装/跨线程传递时保持。源头 ERROR 不输出完整栈，纯传播不重复记录；真正消费失败后 DEBUG 记录实际措施，随后诊断可以释放。只有未处理错误在 Bevy 要求的边界显式转为 Severity::Error，最终由宿主输出原栈。正常等待、取消、合法 No-op 不人为转换为错误。

本增量改变的公开签名、回调或结果形状，必须在同一个增量更新所有直接消费者、测试、examples/benches 和所需 Facade 导出。不能用兼容 String 的临时入口掩盖未迁移调用，也不能把基本编译修复推迟到最终 Gallery 增量。

## 完成条件

- 正常加载/布局等待、空内容、零viewport、没有所属scroll区域的事件均无错误。
- 真实已拥有部件破坏可定位轴和Root，多个Root故障不丢失。
- 轮滚、方向键、scroll-into-view、thumb拖动和自动显隐保持原契约。

在 Windows x86_64-pc-windows-msvc / PowerShell 7 下执行本项相关测试：

```powershell
cargo test -p bevy_widgetry_scroll_area
```

对本项实际改动的包与直接消费者进行编译/Clippy 检查；触及 Gallery 时至少检查其编译。GUI、后台时序或性能敏感路径按本节具体场景复验，不重复运行未受影响的整套全仓验收。

同步 scroll_area 错误/正常等待界线及覆盖说明。

代码、构建或测试行为变更按适用工程规则完成必要的独立 Code Review；修复和回归仍归入本增量。

## 保持的边界

只迁移错误表示、传递、处理记录及直接受影响的调用。允许破坏性 API 更改，但不改变既有正常交互、状态通知、资源归属或业务算法；不加入与该错误路径无关的重构。

## 顺序衔接

前一项：[19 为 Segmented 的严格单选结构接入诊断](19-segmented-errors.md)；后一项：[21 为 Splitter 比例与 Pane 失败保留数值现场](21-splitter-errors.md)。编号规定唯一执行顺序，技术依赖以上文列出的能力为准。
