# 统一 Core 共享状态操作的错误上下文

让 Core 的共享状态和颜色配置接口提供有定位价值的领域失败，而不是让上层重复解释缺失的基础结构。正常的输入路由过滤保持静默。

## 修改落点与前置能力

涉及：`crates/core/src/color.rs`、`crates/core/src/text.rs`、`crates/core/src/text/colors.rs`、`crates/core/src/disabled.rs`、`crates/core/src/foreground.rs`、`crates/core/src/exclusive_selection.rs`、`crates/core/src/pointer/`、`crates/core/src/font.rs`、`crates/core/src/ui.rs`。

实际前置为[04 在 BSN 与 Scene 展开中保留原始诊断](04-scene-diagnostics.md)、[05 区分持续异常状态与新的独立失败](05-persistent-failure-state.md)。这些引用表示本项直接使用的能力，不要求相邻编号之间存在调用关系。

## 共享失败在拥有信息的层形成

逐个区分颜色值校验、公开 setter 目标校验、已确认 Owner 的必需 Component 缺失，以及输入路由中的普通 Query 不命中。前两类和真实结构破坏建立 Core 领域错误，附操作、Entity、缺失类型或非法字段；普通路由、未聚焦和撤销输入继续使用原有正常分支。

上层控件调用 Core 的颜色/disabled/foreground 操作失败时，应继承 Core 的 origin，只补控件 Root 和操作语境。不能一处 Core 记录一次、每个父控件再记录一次。只迁移实际存在的失败通道，纯数值转换和常量不增加伪错误 API。

## 直接使用与复合控件一致

同步 World 操作暴露领域结果；Scene Template 和 command/system 返回口使用公共 Bevy 桥。若辅助函数使用早退、ok() 或默认值，应按业务契约确认是否是合法缺失，不能机械替换，也不能让已确认拥有的结构损坏继续被当成过滤。

直接消费者包括所有使用颜色覆盖、disabled 组合和前景继承的控件。公共签名变更同一增量完成机械调用适配，具体控件的源头领域建模仍留给对应方案。

## 诊断承接与调用方迁移

错误采用领域分类和统一诊断包装；同一次失败的 ID、source 与原始 Backtrace 在包装/跨线程传递时保持。源头 ERROR 不输出完整栈，纯传播不重复记录；真正消费失败后 DEBUG 记录实际措施，随后诊断可以释放。只有未处理错误在 Bevy 要求的边界显式转为 Severity::Error，最终由宿主输出原栈。正常等待、取消、合法 No-op 不人为转换为错误。

本增量改变的公开签名、回调或结果形状，必须在同一个增量更新所有直接消费者、测试、examples/benches 和所需 Facade 导出。不能用兼容 String 的临时入口掩盖未迁移调用，也不能把基本编译修复推迟到最终 Gallery 增量。

## 完成条件

- 真实缺失结构、无效颜色/目标分别验证 Entity/字段与原因链；传播到两个嵌套控件仍只有一个 source ERROR。
- 保留合法重复 setter、同值操作、没有有效输入目标以及普通 Pointer Cancel 的无日志行为。
- 检查相关 Core 集成测试及下游编译，不改变选择通知、颜色归属和输入路由。

在 Windows x86_64-pc-windows-msvc / PowerShell 7 下执行本项相关测试：

```powershell
cargo test -p bevy_widgetry_core
```

对本项实际改动的包与直接消费者进行编译/Clippy 检查；触及 Gallery 时至少检查其编译。GUI、后台时序或性能敏感路径按本节具体场景复验，不重复运行未受影响的整套全仓验收。

同步 core 共享 API 说明和受到影响的 tests/COVERAGE.md。

代码、构建或测试行为变更按适用工程规则完成必要的独立 Code Review；修复和回归仍归入本增量。

## 保持的边界

只迁移错误表示、传递、处理记录及直接受影响的调用。允许破坏性 API 更改，但不改变既有正常交互、状态通知、资源归属或业务算法；不加入与该错误路径无关的重构。

## 顺序衔接

前一项：[06 为 Icon 与 SVG 失败保留资产现场](06-core-icon-errors.md)；后一项：[08 使离屏目标失败保留资源归属与清理结果](08-offscreen-errors.md)。编号规定唯一执行顺序，技术依赖以上文列出的能力为准。
