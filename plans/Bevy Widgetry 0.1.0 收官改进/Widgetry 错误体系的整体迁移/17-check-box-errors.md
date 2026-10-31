# 让 CheckBox 批量样式失败全部到达兜底

迁移 Checkbox/TriState 的样式诊断，消除只保留批量首个错误造成的信息丢失。

## 修改落点与前置能力

涉及：`crates/check_box/src/style.rs`、`crates/check_box/src/checkbox.rs`、`crates/check_box/src/tri_state.rs`、`crates/check_box/src/indicator.rs`、`crates/check_box/src/colors.rs`。

实际前置为[05 区分持续异常状态与新的独立失败](05-persistent-failure-state.md)、[07 统一 Core 共享状态操作的错误上下文](07-core-state-errors.md)。这些引用表示本项直接使用的能力，不要求相邻编号之间存在调用关系。

## 部件与持续状态

update_mark_geometry 和样式更新缺少 indicator、mark 或颜色部件时，保存 Root、部件实体、状态与操作阶段。StyleDiagnostics 的 mark/colors 使用公共持续失败规则；不把 Query 的底层错误裸 ? 成 Panic Severity。

继续更新多个 Root 的循环使用诊断集合保留所有未处理失败。原来局部失败不妨碍其他 Checkbox 更新的行为继续保留，不能为了容易返回一个错误而提前中止整批。

## 三态不是三种异常

Checked/Indeterminate 与其正常切换保持业务含义；普通无目标事件、创建中的正常等待、同值更新不产生 ERROR。样式失败只记录源头一次；若最终清理/降级消费该失败，按实际结果记 DEBUG，而不是在 FailureState.observe 中一律记已处理。

## 诊断承接与调用方迁移

错误采用领域分类和统一诊断包装；同一次失败的 ID、source 与原始 Backtrace 在包装/跨线程传递时保持。源头 ERROR 不输出完整栈，纯传播不重复记录；真正消费失败后 DEBUG 记录实际措施，随后诊断可以释放。只有未处理错误在 Bevy 要求的边界显式转为 Severity::Error，最终由宿主输出原栈。正常等待、取消、合法 No-op 不人为转换为错误。

本增量改变的公开签名、回调或结果形状，必须在同一个增量更新所有直接消费者、测试、examples/benches 和所需 Facade 导出。不能用兼容 String 的临时入口掩盖未迁移调用，也不能把基本编译修复推迟到最终 Gallery 增量。

## 完成条件

- 同一轮损坏两个 Checkbox 部件，两个不同ID及回溯均能到达Handler，健康第三个继续更新。
- 持续同一部件异常不每帧重新捕栈；恢复后再次损坏是新诊断。
- 三态点击/键盘/程序化状态切换与通知、主题/禁用投影保持既有回归。

在 Windows x86_64-pc-windows-msvc / PowerShell 7 下执行本项相关测试：

```powershell
cargo test -p bevy_widgetry_check_box
```

对本项实际改动的包与直接消费者进行编译/Clippy 检查；触及 Gallery 时至少检查其编译。GUI、后台时序或性能敏感路径按本节具体场景复验，不重复运行未受影响的整套全仓验收。

同步 CheckBox 结构失败、批量处理和持续异常覆盖。

代码、构建或测试行为变更按适用工程规则完成必要的独立 Code Review；修复和回归仍归入本增量。

## 保持的边界

只迁移错误表示、传递、处理记录及直接受影响的调用。允许破坏性 API 更改，但不改变既有正常交互、状态通知、资源归属或业务算法；不加入与该错误路径无关的重构。

## 顺序衔接

前一项：[16 保留 HoldButton 构造与材质失败的现场](16-hold-button-errors.md)；后一项：[18 为 RadioGroup 结构与选择失败保留归属](18-radio-group-errors.md)。编号规定唯一执行顺序，技术依赖以上文列出的能力为准。
