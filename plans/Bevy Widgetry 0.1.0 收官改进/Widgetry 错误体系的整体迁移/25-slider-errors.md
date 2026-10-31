# 为 Slider 数值契约建立领域诊断

保留Slider现有数值模型和可达步长规则，改进非法配置与运行期目标的错误表达。

## 修改落点与前置能力

涉及：`crates/slider/src/value.rs`、`crates/slider/src/slider.rs`、`crates/slider/src/input.rs`、`crates/slider/src/gesture.rs`、`crates/slider/src/lifecycle.rs`、`crates/slider/src/style.rs`、`crates/slider/src/colors.rs`。

实际前置为[04 在 BSN 与 Scene 展开中保留原始诊断](04-scene-diagnostics.md)、[07 统一 Core 共享状态操作的错误上下文](07-core-state-errors.md)。这些引用表示本项直接使用的能力，不要求相邻编号之间存在调用关系。

## 数值场景不再丢失

min<max、有限span、正且有限step、value范围与端点可达性错误保存min/max/step/value和判定类别，不只返回“无效请求”。内部状态/Root缺失单独表达；泛型/浮点表示按原模型，不改变取整或端点递进规则。

状态读取和程序化设置返回领域失败；手势/键盘System与Commands在边界转换。Pointer撤销、Disabled、实际值未变化或正常clamp不制造错误。下层颜色/布局失败沿原ID传播。

## 诊断承接与调用方迁移

错误采用领域分类和统一诊断包装；同一次失败的 ID、source 与原始 Backtrace 在包装/跨线程传递时保持。源头 ERROR 不输出完整栈，纯传播不重复记录；真正消费失败后 DEBUG 记录实际措施，随后诊断可以释放。只有未处理错误在 Bevy 要求的边界显式转为 Severity::Error，最终由宿主输出原栈。正常等待、取消、合法 No-op 不人为转换为错误。

本增量改变的公开签名、回调或结果形状，必须在同一个增量更新所有直接消费者、测试、examples/benches 和所需 Facade 导出。不能用兼容 String 的临时入口掩盖未迁移调用，也不能把基本编译修复推迟到最终 Gallery 增量。

## 完成条件

- 非有限数、逆序边界、不可达step和越界value的诊断包含实际配置。
- 原数值边界、浮点舍入、拖动中间通知与最终确认通知回归通过。
- 真实损坏状态与正常输入Guard分开验证，所有边界为Severity::Error。

在 Windows x86_64-pc-windows-msvc / PowerShell 7 下执行本项相关测试：

```powershell
cargo test -p bevy_widgetry_slider
```

对本项实际改动的包与直接消费者进行编译/Clippy 检查；触及 Gallery 时至少检查其编译。GUI、后台时序或性能敏感路径按本节具体场景复验，不重复运行未受影响的整套全仓验收。

同步 Slider 数值错误契约和已有属性测试覆盖，不增加随机时间断言。

代码、构建或测试行为变更按适用工程规则完成必要的独立 Code Review；修复和回归仍归入本增量。

## 保持的边界

只迁移错误表示、传递、处理记录及直接受影响的调用。允许破坏性 API 更改，但不改变既有正常交互、状态通知、资源归属或业务算法；不加入与该错误路径无关的重构。

## 顺序衔接

前一项：[24 为 ProgressBar 校验保留实际配置](24-progress-bar-errors.md)；后一项：[26 为 TextField 内容与编辑器错误保留映射](26-text-field-errors.md)。编号规定唯一执行顺序，技术依赖以上文列出的能力为准。
