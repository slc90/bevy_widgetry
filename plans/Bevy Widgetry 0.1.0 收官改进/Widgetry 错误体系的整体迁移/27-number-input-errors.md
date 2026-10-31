# 保留 NumberInput 的原数值类型与失败现场

迁移泛型数值控件的配置、提交和内部编辑器失败，不把正常用户编辑中间态当作异常。

## 修改落点与前置能力

涉及：`crates/number_input/src/value.rs`、`crates/number_input/src/editing.rs`、`crates/number_input/src/number.rs`、`crates/number_input/src/input.rs`、`crates/number_input/src/repeating.rs`、`crates/number_input/src/layout.rs`、`crates/number_input/src/style.rs`。

实际前置为[26 为 TextField 内容与编辑器错误保留映射](26-text-field-errors.md)。这些引用表示本项直接使用的能力，不要求相邻编号之间存在调用关系。

## 配置错误保存原类型信息

State::new/validate保存初始value、min/max、step、decimals、数值类型名以及失败类别。显式初值缺失、非有限/逆序范围、无效步长、整数误配小数位、越界值分开表达，不再仅靠reason字符串。不要为了诊断把所有T转换成f64而丢整数精度。

错误表示可以由本crate的领域类型保存T或精确值快照；同一规则必须覆盖现有支持的数值类型。Backtrace和身份属于外层，不参与数值相等性。

## 编辑行为与跨Crate传播

field/editor映射错误保留Root和内部文本字段。TextField失败继承原origin，只增加NumberInput动作。手动输入未完成、提交校验拒绝、步进clamp、只读/disabled和正常失焦清理按现有编辑契约表达，不为了诊断改成大量ERROR。

公开程序化setter违反前置条件是实际失败，与正常用户编辑拒绝区分。键盘、按钮长按与失焦提交保持原时序、值通知和真实精度。

## 诊断承接与调用方迁移

错误采用领域分类和统一诊断包装；同一次失败的 ID、source 与原始 Backtrace 在包装/跨线程传递时保持。源头 ERROR 不输出完整栈，纯传播不重复记录；真正消费失败后 DEBUG 记录实际措施，随后诊断可以释放。只有未处理错误在 Bevy 要求的边界显式转为 Severity::Error，最终由宿主输出原栈。正常等待、取消、合法 No-op 不人为转换为错误。

本增量改变的公开签名、回调或结果形状，必须在同一个增量更新所有直接消费者、测试、examples/benches 和所需 Facade 导出。不能用兼容 String 的临时入口掩盖未迁移调用，也不能把基本编译修复推迟到最终 Gallery 增量。

## 完成条件

- 大整数、浮点非有限数、边界/step/decimals错误保持精确输入；覆盖不同数值类别。
- 半成品十进制输入、回退显示、readonly、同值与端点clamp无伪错误。
- 破坏TextField内部部件时只产生一份原诊断；最终Command/System可取得两层操作上下文。

在 Windows x86_64-pc-windows-msvc / PowerShell 7 下执行本项相关测试：

```powershell
cargo test -p bevy_widgetry_number_input
```

对本项实际改动的包与直接消费者进行编译/Clippy 检查；触及 Gallery 时至少检查其编译。GUI、后台时序或性能敏感路径按本节具体场景复验，不重复运行未受影响的整套全仓验收。

同步 NumberInput 泛型失败及编辑状态覆盖。

代码、构建或测试行为变更按适用工程规则完成必要的独立 Code Review；修复和回归仍归入本增量。

## 保持的边界

只迁移错误表示、传递、处理记录及直接受影响的调用。允许破坏性 API 更改，但不改变既有正常交互、状态通知、资源归属或业务算法；不加入与该错误路径无关的重构。

## 顺序衔接

前一项：[26 为 TextField 内容与编辑器错误保留映射](26-text-field-errors.md)；后一项：[28 区分 DatePicker 日期错误与 Today 降级](28-date-picker-errors.md)。编号规定唯一执行顺序，技术依赖以上文列出的能力为准。
