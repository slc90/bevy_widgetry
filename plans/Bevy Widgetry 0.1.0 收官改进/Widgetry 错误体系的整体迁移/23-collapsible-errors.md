# 为 Collapsible 内部结构失败保持组合来源

使展开状态操作能区分自己的结构损坏与下层Button/Icon失败，保留常驻内容和焦点处理。

## 修改落点与前置能力

涉及：`crates/collapsible/src/state.rs`、`crates/collapsible/src/disabled.rs`、`crates/collapsible/src/style.rs`、`crates/collapsible/src/input.rs`、`crates/collapsible/src/collapsible.rs`、`crates/collapsible/src/colors.rs`。

实际前置为[06 为 Icon 与 SVG 失败保留资产现场](06-core-icon-errors.md)、[13 统一 Button 状态与结构失败的诊断](13-button-loading-errors.md)。这些引用表示本项直接使用的能力，不要求相邻编号之间存在调用关系。

## 组合部件与状态提交

structure校验保存Root、header/content/header_content/button实体和预期父级；缺Expanded/HeaderDisabled与非法Button不再合成一个字符串。set_expanded_in_world调用箭头/disabled失败时保留Core/Button原origin，只追加展开请求。

initialize原先只保留首个failure，改为保留整批未处理诊断。合法同值、正常折叠、普通输入过滤不是错误。失败后的状态遵循现有先更新箭头、再提交Expanded等顺序，不借错误迁移增加事务语义。

## 诊断承接与调用方迁移

错误采用领域分类和统一诊断包装；同一次失败的 ID、source 与原始 Backtrace 在包装/跨线程传递时保持。源头 ERROR 不输出完整栈，纯传播不重复记录；真正消费失败后 DEBUG 记录实际措施，随后诊断可以释放。只有未处理错误在 Bevy 要求的边界显式转为 Severity::Error，最终由宿主输出原栈。正常等待、取消、合法 No-op 不人为转换为错误。

本增量改变的公开签名、回调或结果形状，必须在同一个增量更新所有直接消费者、测试、examples/benches 和所需 Facade 导出。不能用兼容 String 的临时入口掩盖未迁移调用，也不能把基本编译修复推迟到最终 Gallery 增量。

## 完成条件

- 不同缺失部件可定位，两个Root异常全部保留；同次下层Icon失败不被重复记录。
- 折叠时焦点转移、内容常驻、通知次数、Header禁用状态保持不变。
- 同步读取/设置和deferred设置都保留原ID与栈。

在 Windows x86_64-pc-windows-msvc / PowerShell 7 下执行本项相关测试：

```powershell
cargo test -p bevy_widgetry_collapsible
```

对本项实际改动的包与直接消费者进行编译/Clippy 检查；触及 Gallery 时至少检查其编译。GUI、后台时序或性能敏感路径按本节具体场景复验，不重复运行未受影响的整套全仓验收。

同步 Collapsible 组合结构失败及覆盖说明。

代码、构建或测试行为变更按适用工程规则完成必要的独立 Code Review；修复和回归仍归入本增量。

## 保持的边界

只迁移错误表示、传递、处理记录及直接受影响的调用。允许破坏性 API 更改，但不改变既有正常交互、状态通知、资源归属或业务算法；不加入与该错误路径无关的重构。

## 顺序衔接

前一项：[22 让 Tabs 配对结构失败保持实体关系](22-tabs-errors.md)；后一项：[24 为 ProgressBar 校验保留实际配置](24-progress-bar-errors.md)。编号规定唯一执行顺序，技术依赖以上文列出的能力为准。
