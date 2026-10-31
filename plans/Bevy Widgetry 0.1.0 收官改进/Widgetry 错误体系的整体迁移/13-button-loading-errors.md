# 统一 Button 状态与结构失败的诊断

为 Button loading/颜色等状态 API 的真实失败保留 Root 和缺失内部部件，不改变 Loading、Disabled 和 Spinner 的职责。

## 修改落点与前置能力

涉及：`crates/button/src/loading.rs`、`crates/button/src/style.rs`、`crates/button/src/colors.rs`、`crates/button/src/lib.rs`。

实际前置为[07 统一 Core 共享状态操作的错误上下文](07-core-state-errors.md)、[10 记录光标资源失败与实际降级结果](10-cursor-fallback-errors.md)。这些引用表示本项直接使用的能力，不要求相邻编号之间存在调用关系。

## 区分无效目标和内部损坏

invalid_target 当前把不是 Button、缺少 LoadingState、content/spinner 缺失混为一句话。领域错误应保留目标与缺失部件；调用方非法目标和已确认 Button 的内部损坏可以分别匹配。

is_loading/set_loading_in_world 返回携带诊断的领域结果，queued setter 在 closure 转 BevyError。来自 Core disabled/颜色层的失败只附 Button 操作上下文，不再新建源头。成功同值、Spinner 未到绘制条件等原有正常分支不记录。

## 迁移边界

保持 loading 对内容可见性、Spinner 变换和内部忙碌光标标记的顺序，不能把失败日志作为改变业务状态的理由。组合 Button 的 Collapsible、ComboBox、Window、Canvas 控制与 Gallery 调用同步适配。LinkButton 打开外部程序的失败由下一增量处理。

## 诊断承接与调用方迁移

错误采用领域分类和统一诊断包装；同一次失败的 ID、source 与原始 Backtrace 在包装/跨线程传递时保持。源头 ERROR 不输出完整栈，纯传播不重复记录；真正消费失败后 DEBUG 记录实际措施，随后诊断可以释放。只有未处理错误在 Bevy 要求的边界显式转为 Severity::Error，最终由宿主输出原栈。正常等待、取消、合法 No-op 不人为转换为错误。

本增量改变的公开签名、回调或结果形状，必须在同一个增量更新所有直接消费者、测试、examples/benches 和所需 Facade 导出。不能用兼容 String 的临时入口掩盖未迁移调用，也不能把基本编译修复推迟到最终 Gallery 增量。

## 完成条件

- 无效目标、缺少 content/spinner/state 有不同结构化诊断，并保持失败前后既有状态契约。
- 同值 setter、正常加载动画与 disabled 组合不产错误；真实 Command 路径输出原ID。
- 现有 pointer/loading/theme 组合测试通过，下游调用编译。

在 Windows x86_64-pc-windows-msvc / PowerShell 7 下执行本项相关测试：

```powershell
cargo test -p bevy_widgetry_button
```

对本项实际改动的包与直接消费者进行编译/Clippy 检查；触及 Gallery 时至少检查其编译。GUI、后台时序或性能敏感路径按本节具体场景复验，不重复运行未受影响的整套全仓验收。

更新 Button loading API 与相关覆盖说明。

代码、构建或测试行为变更按适用工程规则完成必要的独立 Code Review；修复和回归仍归入本增量。

## 保持的边界

只迁移错误表示、传递、处理记录及直接受影响的调用。允许破坏性 API 更改，但不改变既有正常交互、状态通知、资源归属或业务算法；不加入与该错误路径无关的重构。

## 顺序衔接

前一项：[12 让后台 Job 的失败跨线程保持原诊断](12-job-outcome-diagnostics.md)；后一项：[14 使 LinkButton 打开失败可追溯且可标记已处理](14-link-button-errors.md)。编号规定唯一执行顺序，技术依赖以上文列出的能力为准。
