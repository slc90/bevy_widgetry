# 保留 HoldButton 构造与材质失败的现场

把长按按钮的非法配置和真实渲染结构失败变为领域诊断，输入取消和计时完成仍保持现有状态机。

## 修改落点与前置能力

涉及：`crates/hold_button/src/style.rs`、`crates/hold_button/src/headless.rs`、`crates/hold_button/src/colors.rs`、`crates/hold_button/src/lib.rs`。

实际前置为[04 在 BSN 与 Scene 展开中保留原始诊断](04-scene-diagnostics.md)、[07 统一 Core 共享状态操作的错误上下文](07-core-state-errors.md)。这些引用表示本项直接使用的能力，不要求相邻编号之间存在调用关系。

## 构造与运行期错误

直径必须有限且大于零的校验保留，并在错误中保存实际 diameter 和构造操作；Template 经公共 Scene 适配。样式/材质/颜色更新中确认属于该按钮的必要结构缺失，保存 Root、材质/实体身份和阶段；外部资产等待不算失败。

headless 的失焦、松开、disabled、Pointer Cancel 和窗口变化仍是正常取消，不为其捕栈。真实无法处理的失败按原 System/Observer 边界上报，不能借日志重置长按状态。

## 诊断承接与调用方迁移

错误采用领域分类和统一诊断包装；同一次失败的 ID、source 与原始 Backtrace 在包装/跨线程传递时保持。源头 ERROR 不输出完整栈，纯传播不重复记录；真正消费失败后 DEBUG 记录实际措施，随后诊断可以释放。只有未处理错误在 Bevy 要求的边界显式转为 Severity::Error，最终由宿主输出原栈。正常等待、取消、合法 No-op 不人为转换为错误。

本增量改变的公开签名、回调或结果形状，必须在同一个增量更新所有直接消费者、测试、examples/benches 和所需 Facade 导出。不能用兼容 String 的临时入口掩盖未迁移调用，也不能把基本编译修复推迟到最终 Gallery 增量。

## 完成条件

- NaN、无穷、零、负直径构造失败，错误保留输入值且无多余空实体。
- 模拟已拥有的材质/结构损坏，源头与 Handler ID 一致。
- 正常取消、重复输入和完成通知保持原有次数及无错误日志。

在 Windows x86_64-pc-windows-msvc / PowerShell 7 下执行本项相关测试：

```powershell
cargo test -p bevy_widgetry_hold_button
```

对本项实际改动的包与直接消费者进行编译/Clippy 检查；触及 Gallery 时至少检查其编译。GUI、后台时序或性能敏感路径按本节具体场景复验，不重复运行未受影响的整套全仓验收。

同步 HoldButton 参数及运行期错误覆盖。

代码、构建或测试行为变更按适用工程规则完成必要的独立 Code Review；修复和回归仍归入本增量。

## 保持的边界

只迁移错误表示、传递、处理记录及直接受影响的调用。允许破坏性 API 更改，但不改变既有正常交互、状态通知、资源归属或业务算法；不加入与该错误路径无关的重构。

## 顺序衔接

前一项：[15 使 ToggleButton 的目标与样式失败可识别](15-toggle-button-errors.md)；后一项：[17 让 CheckBox 批量样式失败全部到达兜底](17-check-box-errors.md)。编号规定唯一执行顺序，技术依赖以上文列出的能力为准。
