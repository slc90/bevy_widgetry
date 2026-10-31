# 为主题切换提供可识别的领域失败

主题常量保持简单，只让实际可失败的主题切换接口进入统一诊断体系。

## 修改落点与前置能力

涉及：`crates/theme/src/theme.rs`、`crates/theme/src/lib.rs`、`crates/test_utils/src/theme.rs`、`Gallery 与控件的主题切换调用`。

实际前置为[02 保留诊断身份地进入 Bevy 错误边界](02-bevy-error-bridge.md)、[03 让测试能观察完整错误生命周期](03-diagnostic-test-support.md)。这些引用表示本项直接使用的能力，不要求相邻编号之间存在调用关系。

## 缺少主题资源与正常同值切换

WidgetryThemeMode::set_in_world 当前在缺少 WidgetryThemeMode 时记录文本错误。改为可匹配的缺少插件/资源错误，携带请求 mode 和操作名；在这一位置创建 origin、记录一次 ERROR。

相同主题继续 Ok(false)，不记录错误或已处理。set 的 Commands closure 负责 Bevy 转换；真实触发 WidgetryThemeChanged、world.flush 与通知顺序保持现有行为。主题颜色常量和纯 colors() 查表不新增错误类型。

## 诊断承接与调用方迁移

错误采用领域分类和统一诊断包装；同一次失败的 ID、source 与原始 Backtrace 在包装/跨线程传递时保持。源头 ERROR 不输出完整栈，纯传播不重复记录；真正消费失败后 DEBUG 记录实际措施，随后诊断可以释放。只有未处理错误在 Bevy 要求的边界显式转为 Severity::Error，最终由宿主输出原栈。正常等待、取消、合法 No-op 不人为转换为错误。

本增量改变的公开签名、回调或结果形状，必须在同一个增量更新所有直接消费者、测试、examples/benches 和所需 Facade 导出。不能用兼容 String 的临时入口掩盖未迁移调用，也不能把基本编译修复推迟到最终 Gallery 增量。

## 完成条件

- 缺失资源的同步与 queued 路径保持同一领域分类，queued 路径 Severity 为 Error。
- 同值切换没有诊断事件；成功切换仅产生原有主题通知。
- 适配测试辅助和 Gallery，直接 theme crate 依赖与 Facade 导出均编译。

在 Windows x86_64-pc-windows-msvc / PowerShell 7 下执行本项相关测试：

```powershell
cargo test -p bevy_widgetry_theme -p bevy_widgetry_test_utils
```

对本项实际改动的包与直接消费者进行编译/Clippy 检查；触及 Gallery 时至少检查其编译。GUI、后台时序或性能敏感路径按本节具体场景复验，不重复运行未受影响的整套全仓验收。

同步 docs/crates/theme.md 与错误路径覆盖。

代码、构建或测试行为变更按适用工程规则完成必要的独立 Code Review；修复和回归仍归入本增量。

## 保持的边界

只迁移错误表示、传递、处理记录及直接受影响的调用。允许破坏性 API 更改，但不改变既有正常交互、状态通知、资源归属或业务算法；不加入与该错误路径无关的重构。

## 顺序衔接

前一项：[08 使离屏目标失败保留资源归属与清理结果](08-offscreen-errors.md)；后一项：[10 记录光标资源失败与实际降级结果](10-cursor-fallback-errors.md)。编号规定唯一执行顺序，技术依赖以上文列出的能力为准。
