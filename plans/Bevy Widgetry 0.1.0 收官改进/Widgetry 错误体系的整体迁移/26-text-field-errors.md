# 为 TextField 内容与编辑器错误保留映射

迁移TextField与ReadOnlyTextField共用的内容、尺寸和样式失败，让组合控件能够保留原始编辑器诊断。

## 修改落点与前置能力

涉及：`crates/text_field/src/content.rs`、`crates/text_field/src/auto_size.rs`、`crates/text_field/src/composition.rs`、`crates/text_field/src/count.rs`、`crates/text_field/src/text_field.rs`、`crates/text_field/src/style.rs`、`crates/text_field/src/colors.rs`。

实际前置为[04 在 BSN 与 Scene 展开中保留原始诊断](04-scene-diagnostics.md)、[07 统一 Core 共享状态操作的错误上下文](07-core-state-errors.md)。这些引用表示本项直接使用的能力，不要求相邻编号之间存在调用关系。

## 映射失效的具体原因

content::invalid_target目前混合了无效Root、类型不匹配、Parts、EditableText与LengthLimit缺失。领域错误保存Root、editor/frame、请求操作与缺失类型；AutoSize配置保存原始参数。

content_api宏同时迁移正常和只读控件的editor/focus/text/set_text。来自Core的颜色/状态错误保持source。set_text中的IME清理、文本写入和计数投影可能分阶段发生，不能把后续失败描述成未做任何修改。

## 正常编辑不经过错误通道

字数截断、只读Guard、无变化文本、正常IME中间态和合法空文本保留原有行为。仅实际内部映射/资源损坏形成ERROR；Focus和其他事件过滤仍按其拥有关系判断。

NumberInput、ComboBox、Table编辑器和Gallery直接调用同一增量改签名，不能临时转成String以便编译。

## 诊断承接与调用方迁移

错误采用领域分类和统一诊断包装；同一次失败的 ID、source 与原始 Backtrace 在包装/跨线程传递时保持。源头 ERROR 不输出完整栈，纯传播不重复记录；真正消费失败后 DEBUG 记录实际措施，随后诊断可以释放。只有未处理错误在 Bevy 要求的边界显式转为 Severity::Error，最终由宿主输出原栈。正常等待、取消、合法 No-op 不人为转换为错误。

本增量改变的公开签名、回调或结果形状，必须在同一个增量更新所有直接消费者、测试、examples/benches 和所需 Facade 导出。不能用兼容 String 的临时入口掩盖未迁移调用，也不能把基本编译修复推迟到最终 Gallery 增量。

## 完成条件

- 分别破坏Parts/EditableText/LengthLimit，诊断能够区分且保留Root/editor。
- 只读与普通文本、IME提交、截断、计数和AutoSize原有用例继续通过。
- setter传播经NumberInput/ComboBox包装后不重复源头；queued版本的Handler读取原回溯。

在 Windows x86_64-pc-windows-msvc / PowerShell 7 下执行本项相关测试：

```powershell
cargo test -p bevy_widgetry_text_field
```

对本项实际改动的包与直接消费者进行编译/Clippy 检查；触及 Gallery 时至少检查其编译。GUI、后台时序或性能敏感路径按本节具体场景复验，不重复运行未受影响的整套全仓验收。

同步 TextField/ReadOnly API 与覆盖说明。

代码、构建或测试行为变更按适用工程规则完成必要的独立 Code Review；修复和回归仍归入本增量。

## 保持的边界

只迁移错误表示、传递、处理记录及直接受影响的调用。允许破坏性 API 更改，但不改变既有正常交互、状态通知、资源归属或业务算法；不加入与该错误路径无关的重构。

## 顺序衔接

前一项：[25 为 Slider 数值契约建立领域诊断](25-slider-errors.md)；后一项：[27 保留 NumberInput 的原数值类型与失败现场](27-number-input-errors.md)。编号规定唯一执行顺序，技术依赖以上文列出的能力为准。
