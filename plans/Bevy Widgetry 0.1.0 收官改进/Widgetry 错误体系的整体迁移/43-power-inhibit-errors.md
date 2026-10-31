# 保留电源请求主失败与清理失败的各自原因

把PowerError别名和拼接错误文本替换成明确的电源领域失败，保留Display/System请求与回滚状态。

## 修改落点与前置能力

涉及：`crates/power_inhibit/src/power.rs`、`crates/power_inhibit/src/power/native.rs`、`crates/power_inhibit/src/power/tests.rs`、`Gallery电源抑制调用`。

实际前置为[02 保留诊断身份地进入 Bevy 错误边界](02-bevy-error-bridge.md)、[03 让测试能观察完整错误生命周期](03-diagnostic-test-support.md)。这些引用表示本项直接使用的能力，不要求相邻编号之间存在调用关系。

## 主操作和回滚不能合并成字符串

PowerError不再只是BevyError别名。创建、set和clear失败保存RequestKind、操作阶段、相关请求状态及原OS错误；API外层report_error不再次重复原生层源头。

enable失败时，回滚若也失败，应保留主失败与cleanup失败两份诊断；disable中两个请求清除失败也不能format成一条文本。返回集合或领域组合载体必须让Handler逐项访问。成功回滚只表示清理完成，不代表原enable错误被消费。

## 析构兜底的实际措施

显式disable仍把无法处理的失败返回给拥有者。Drop不能传播Result或主动panic；其既有OwnedHandle释放是最后的资源清理措施，记录必须准确说明清除失败和实际采取的关闭句柄措施，不能凭“进入Drop”就宣称所有失败已恢复。

将需要观察结果的关闭尽量留在显式生命周期路径；仅当业务Owner实际消费并完成补救才记DEBUG。保持FFI仍限于native模块，不扩大unsafe允许范围。

## 诊断承接与调用方迁移

错误采用领域分类和统一诊断包装；同一次失败的 ID、source 与原始 Backtrace 在包装/跨线程传递时保持。源头 ERROR 不输出完整栈，纯传播不重复记录；真正消费失败后 DEBUG 记录实际措施，随后诊断可以释放。只有未处理错误在 Bevy 要求的边界显式转为 Severity::Error，最终由宿主输出原栈。正常等待、取消、合法 No-op 不人为转换为错误。

本增量改变的公开签名、回调或结果形状，必须在同一个增量更新所有直接消费者、测试、examples/benches 和所需 Facade 导出。不能用兼容 String 的临时入口掩盖未迁移调用，也不能把基本编译修复推迟到最终 Gallery 增量。

## 完成条件

- Display成功/System失败且clear失败时，主因与回滚原因均保留source/ID，不能只剩合并文本。
- disable两个clear同时失败全部保留；重试状态与句柄归属不变。
- 正常重复enable/disable无错误；Drop不panic，处理DEBUG只在实际补救后出现。

在 Windows x86_64-pc-windows-msvc / PowerShell 7 下执行本项相关测试：

```powershell
cargo test -p bevy_widgetry_power_inhibit
```

对本项实际改动的包与直接消费者进行编译/Clippy 检查；触及 Gallery 时至少检查其编译。GUI、后台时序或性能敏感路径按本节具体场景复验，不重复运行未受影响的整套全仓验收。

同步PowerInhibit失败/回滚/RAII语义和覆盖；Gallery同步改领域匹配。

代码、构建或测试行为变更按适用工程规则完成必要的独立 Code Review；修复和回归仍归入本增量。

## 保持的边界

只迁移错误表示、传递、处理记录及直接受影响的调用。允许破坏性 API 更改，但不改变既有正常交互、状态通知、资源归属或业务算法；不加入与该错误路径无关的重构。

## 顺序衔接

前一项：[42 保留 Tray 原生回调到 ECS 的未处理诊断](42-tray-errors.md)；后一项：[44 为 FileDialog 状态与公开操作建立领域错误](44-file-dialog-contract-errors.md)。编号规定唯一执行顺序，技术依赖以上文列出的能力为准。
