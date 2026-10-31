# 保留 Tooltip 构造与 Popup 失败来源

迁移缺失内容工厂和Popup内部颜色结构错误，保留正常悬停、隐藏与取消行为。

## 修改落点与前置能力

涉及：`crates/tooltip/src/style.rs`、`crates/tooltip/src/headless.rs`、`crates/tooltip/src/colors.rs`。

实际前置为[04 在 BSN 与 Scene 展开中保留原始诊断](04-scene-diagnostics.md)、[07 统一 Core 共享状态操作的错误上下文](07-core-state-errors.md)。这些引用表示本项直接使用的能力，不要求相邻编号之间存在调用关系。

## 内容构造与显示的两层语境

TooltipContentFactory::build和Scene校验的缺失content使用同一明确领域分类，但每次独立无效请求各自形成诊断。构造入口保存可用参数，显示入口补anchor/popup身份；旧logged_error由新Scene适配替代。

show_tooltip与颜色刷新中的已拥有anchor颜色缺失，保留anchor和popup；下层Scene失败只追加这层定位。正常悬停候选改变、源实体销毁、Hide、无目标查询和已经显示的Popup属于状态机正常路径。

## 诊断承接与调用方迁移

错误采用领域分类和统一诊断包装；同一次失败的 ID、source 与原始 Backtrace 在包装/跨线程传递时保持。源头 ERROR 不输出完整栈，纯传播不重复记录；真正消费失败后 DEBUG 记录实际措施，随后诊断可以释放。只有未处理错误在 Bevy 要求的边界显式转为 Severity::Error，最终由宿主输出原栈。正常等待、取消、合法 No-op 不人为转换为错误。

本增量改变的公开签名、回调或结果形状，必须在同一个增量更新所有直接消费者、测试、examples/benches 和所需 Facade 导出。不能用兼容 String 的临时入口掩盖未迁移调用，也不能把基本编译修复推迟到最终 Gallery 增量。

## 完成条件

- 缺失content的Template与运行期build失败可分类；嵌套Scene source保持。
- 两个Popup样式异常均不丢失；普通hover/取消不产诊断事件。
- 冷/热显示延迟、Pointer切换和弹层清理保留现有测试。

在 Windows x86_64-pc-windows-msvc / PowerShell 7 下执行本项相关测试：

```powershell
cargo test -p bevy_widgetry_tooltip
```

对本项实际改动的包与直接消费者进行编译/Clippy 检查；触及 Gallery 时至少检查其编译。GUI、后台时序或性能敏感路径按本节具体场景复验，不重复运行未受影响的整套全仓验收。

同步 Tooltip 内容前置条件、Popup失败及覆盖说明。

代码、构建或测试行为变更按适用工程规则完成必要的独立 Code Review；修复和回归仍归入本增量。

## 保持的边界

只迁移错误表示、传递、处理记录及直接受影响的调用。允许破坏性 API 更改，但不改变既有正常交互、状态通知、资源归属或业务算法；不加入与该错误路径无关的重构。

## 顺序衔接

前一项：[28 区分 DatePicker 日期错误与 Today 降级](28-date-picker-errors.md)；后一项：[30 让 ListView 数据与选择请求使用领域失败](30-list-model-errors.md)。编号规定唯一执行顺序，技术依赖以上文列出的能力为准。
