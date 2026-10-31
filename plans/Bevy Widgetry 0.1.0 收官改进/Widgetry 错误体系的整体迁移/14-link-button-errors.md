# 使 LinkButton 打开失败可追溯且可标记已处理

区分 URL 请求无效与原生浏览器打开失败，保留原始系统原因；安全终止本次打开请求后记录 DEBUG，而不是仅用 WARN 隐藏失败。

## 修改落点与前置能力

涉及：`crates/button/src/link.rs`、`crates/button/src/link_colors.rs`、`crates/button/src/lib.rs`、`Gallery LinkButton 演示调用`。

实际前置为[13 统一 Button 状态与结构失败的诊断](13-button-loading-errors.md)。这些引用表示本项直接使用的能力，不要求相邻编号之间存在调用关系。

## URL 与原生调用两种失败

validate_url 保留解析 source、输入 URL 的必要定位信息和拒绝类别，维持现有 http(s)/host 安全限制。BrowserOpener 的 Result<(), String> 改为能承载真实原生失败及受管诊断的结果，ShellExecuteEx 失败不再先转 String。

激活 command 仍重新检查实体存在与 disabled，正常取消/过期不创建错误。原生打开失败在首次观察处形成 ERROR；若行为 Owner 的处理是安全放弃该次外部打开，在此记录 DEBUG 与实际 action，且不再向 ECS 假装未处理。无法安全处理的错误继续传递。

## 颜色与通知保持原契约

LinkButton 的颜色/Theme 分支保留下层 source，不再用 error.to_string() 重新构造 BevyError。迁移相关测试对 WARN 的旧预期；不要因为新日志增加按钮点击事件或另起外部进程重试策略。

## 诊断承接与调用方迁移

错误采用领域分类和统一诊断包装；同一次失败的 ID、source 与原始 Backtrace 在包装/跨线程传递时保持。源头 ERROR 不输出完整栈，纯传播不重复记录；真正消费失败后 DEBUG 记录实际措施，随后诊断可以释放。只有未处理错误在 Bevy 要求的边界显式转为 Severity::Error，最终由宿主输出原栈。正常等待、取消、合法 No-op 不人为转换为错误。

本增量改变的公开签名、回调或结果形状，必须在同一个增量更新所有直接消费者、测试、examples/benches 和所需 Facade 导出。不能用兼容 String 的临时入口掩盖未迁移调用，也不能把基本编译修复推迟到最终 Gallery 增量。

## 完成条件

- 无效 scheme/host/URL，原生打开失败，成功打开各走正确分类；打开失败具有一条 ERROR 和真正放弃后的 DEBUG。
- 禁用或实体已删除不调用 opener、不产生错误；失败不偷偷重试。
- 原生 source 经过 Command/Theme 边界不被字符串化；原来的 URL 安全检查全部保留。

在 Windows x86_64-pc-windows-msvc / PowerShell 7 下执行本项相关测试：

```powershell
cargo test -p bevy_widgetry_button
```

对本项实际改动的包与直接消费者进行编译/Clippy 检查；触及 Gallery 时至少检查其编译。GUI、后台时序或性能敏感路径按本节具体场景复验，不重复运行未受影响的整套全仓验收。

同步 LinkButton 外部操作失败行为及 COVERAGE。

代码、构建或测试行为变更按适用工程规则完成必要的独立 Code Review；修复和回归仍归入本增量。

## 保持的边界

只迁移错误表示、传递、处理记录及直接受影响的调用。允许破坏性 API 更改，但不改变既有正常交互、状态通知、资源归属或业务算法；不加入与该错误路径无关的重构。

## 顺序衔接

前一项：[13 统一 Button 状态与结构失败的诊断](13-button-loading-errors.md)；后一项：[15 使 ToggleButton 的目标与样式失败可识别](15-toggle-button-errors.md)。编号规定唯一执行顺序，技术依赖以上文列出的能力为准。
