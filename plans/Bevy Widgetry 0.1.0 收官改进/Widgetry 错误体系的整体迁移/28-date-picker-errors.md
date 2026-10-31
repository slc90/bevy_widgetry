# 区分 DatePicker 日期错误与 Today 降级

让日期范围/结构错误与本地日期获取失败都保留原始现场，Today不可用后的降级明确记录为已处理，而不是只剩WARN文本。

## 修改落点与前置能力

涉及：`crates/date_picker/src/model.rs`、`crates/date_picker/src/today.rs`、`crates/date_picker/src/view.rs`、`crates/date_picker/src/colors.rs`。

实际前置为[04 在 BSN 与 Scene 展开中保留原始诊断](04-scene-diagnostics.md)、[05 区分持续异常状态与新的独立失败](05-persistent-failure-state.md)。这些引用表示本项直接使用的能力，不要求相邻编号之间存在调用关系。

## 日期校验和本地时钟

min/max、选中日期、显示年月及42格日期计算失败保留实际日期/范围和time的原始source。替换invalid(Display)和logged_error中的字符串化。

today::Clock目前用Result<Date,String>和failed布尔控制WARN/恢复。读取器改为可保留source的结果；第一次真实本地时间失败记录ERROR，实际采用无Today标记/约定回退后记录DEBUG。恢复有效Today时INFO，原一分钟检查和Reactive等待上限不变。

## 不能用 ok() 丢弃原因

model初始化中把本地日期读取转为Option的路径要区分预期“未提供日期”和真实读取失败。后者保留诊断，再采用既有min或UTC月份回退并记录处理措施；不是把同一时钟失败在多个使用者处新建多个origin。

清空选中、正常月份切换和无Today标记均不是新错误。

## 诊断承接与调用方迁移

错误采用领域分类和统一诊断包装；同一次失败的 ID、source 与原始 Backtrace 在包装/跨线程传递时保持。源头 ERROR 不输出完整栈，纯传播不重复记录；真正消费失败后 DEBUG 记录实际措施，随后诊断可以释放。只有未处理错误在 Bevy 要求的边界显式转为 Severity::Error，最终由宿主输出原栈。正常等待、取消、合法 No-op 不人为转换为错误。

本增量改变的公开签名、回调或结果形状，必须在同一个增量更新所有直接消费者、测试、examples/benches 和所需 Facade 导出。不能用兼容 String 的临时入口掩盖未迁移调用，也不能把基本编译修复推迟到最终 Gallery 增量。

## 完成条件

- 范围逆序、选中越界和日期计算边界保留精确值与来源。
- 本地时间失败→降级→连续失败→恢复：ERROR/DEBUG/INFO次数与ID符合语义，正常无Today不反复报错。
- 语言/月份/日期选择、日历42格和定时检查行为保持不变。

在 Windows x86_64-pc-windows-msvc / PowerShell 7 下执行本项相关测试：

```powershell
cargo test -p bevy_widgetry_date_picker
```

对本项实际改动的包与直接消费者进行编译/Clippy 检查；触及 Gallery 时至少检查其编译。GUI、后台时序或性能敏感路径按本节具体场景复验，不重复运行未受影响的整套全仓验收。

同步 DatePicker Today可用性和日期错误覆盖。

代码、构建或测试行为变更按适用工程规则完成必要的独立 Code Review；修复和回归仍归入本增量。

## 保持的边界

只迁移错误表示、传递、处理记录及直接受影响的调用。允许破坏性 API 更改，但不改变既有正常交互、状态通知、资源归属或业务算法；不加入与该错误路径无关的重构。

## 顺序衔接

前一项：[27 保留 NumberInput 的原数值类型与失败现场](27-number-input-errors.md)；后一项：[29 保留 Tooltip 构造与 Popup 失败来源](29-tooltip-errors.md)。编号规定唯一执行顺序，技术依赖以上文列出的能力为准。
