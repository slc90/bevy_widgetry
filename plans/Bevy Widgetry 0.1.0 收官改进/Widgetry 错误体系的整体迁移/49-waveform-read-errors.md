# 为 Waveform 数据契约保留可匹配错误

替换WaveformReadError(String)，将数据读取、数值配置和buffer验证的真实失败保留为领域分类与源头诊断。

## 修改落点与前置能力

涉及：`crates/waveform/src/source.rs`、`crates/waveform/src/config.rs`、`crates/waveform/src/session.rs`、`crates/waveform/src/reducer.rs`、`crates/waveform/src/lttb.rs`、`crates/waveform/src/lib.rs`、`实现WaveformSource的外部适配`。

实际前置为[02 保留诊断身份地进入 Bevy 错误边界](02-bevy-error-bridge.md)、[03 让测试能观察完整错误生命周期](03-diagnostic-test-support.md)。这些引用表示本项直接使用的能力，不要求相邻编号之间存在调用关系。

## 读取身份与数据验证

WaveformSource::metadata/read/read_reduced的失败保存reader_id、request_id、session/target generation、range、channel范围、output_len、algorithm及实际错误数据。source未提供reduced能力却被调用属于契约失败；合法Pending、数据缺口与取消不是错误。

PlanarBuffer::validate的通道数、完整帧长度、缺口排序/重叠/边界、有效样本有限性分别表达实际/预期值。buffer分配失败保留原分配source及请求规模，不变成字符串。配置、metadata及reducer数值验证保留原类型和单位上下文。

## 外部数据源与迁移

原始第三方错误在source适配器首次纳入时创建origin；自定义source返回已受管错误时不再在runtime重新创建。所有实现者，包括waveform_file与Gallery数据发生器/回放source，同一增量适配返回签名。

源协议要求丢弃过期失败时，必须区分未开始的正常过期请求和已经发生后被Owner明确废弃的失败；后者有处理DEBUG，而不是在文字转换中消失。

## 诊断承接与调用方迁移

错误采用领域分类和统一诊断包装；同一次失败的 ID、source 与原始 Backtrace 在包装/跨线程传递时保持。源头 ERROR 不输出完整栈，纯传播不重复记录；真正消费失败后 DEBUG 记录实际措施，随后诊断可以释放。只有未处理错误在 Bevy 要求的边界显式转为 Severity::Error，最终由宿主输出原栈。正常等待、取消、合法 No-op 不人为转换为错误。

本增量改变的公开签名、回调或结果形状，必须在同一个增量更新所有直接消费者、测试、examples/benches 和所需 Facade 导出。不能用兼容 String 的临时入口掩盖未迁移调用，也不能把基本编译修复推迟到最终 Gallery 增量。

## 完成条件

- 不同通道/帧/缺口/NaN错误可分类并包含请求身份，原始source不丢失。
- Pending、允许的数据gap、正常取消/过期请求不产伪ERROR。
- 自定义source和两种reducer现有数值正确性/单位转换回归保持，所有source消费者编译。

在 Windows x86_64-pc-windows-msvc / PowerShell 7 下执行本项相关测试：

```powershell
cargo test -p bevy_widgetry_waveform -p bevy_widgetry_waveform_file
```

对本项实际改动的包与直接消费者进行编译/Clippy 检查；触及 Gallery 时至少检查其编译。GUI、后台时序或性能敏感路径按本节具体场景复验，不重复运行未受影响的整套全仓验收。

同步WaveformSource/读取验证协议与覆盖。

代码、构建或测试行为变更按适用工程规则完成必要的独立 Code Review；修复和回归仍归入本增量。

## 保持的边界

只迁移错误表示、传递、处理记录及直接受影响的调用。允许破坏性 API 更改，但不改变既有正常交互、状态通知、资源归属或业务算法；不加入与该错误路径无关的重构。

## 顺序衔接

前一项：[48 明确 FileDialog 运行时与界面的错误消费责任](48-file-dialog-consumption-errors.md)；后一项：[50 让 Waveform 后台降采样失败保持原任务现场](50-waveform-runtime-errors.md)。编号规定唯一执行顺序，技术依赖以上文列出的能力为准。
