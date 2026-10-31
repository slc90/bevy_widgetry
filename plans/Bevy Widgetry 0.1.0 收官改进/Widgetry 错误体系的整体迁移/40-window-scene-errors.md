# 使 Window 场景与生命周期失败保持拥有关系

迁移窗口Root、相机、背景和modal/titlebar组合中的错误，保留正常关闭与尚未准备好的native状态。

## 修改落点与前置能力

涉及：`crates/window/src/scene.rs`、`crates/window/src/window_root.rs`、`crates/window/src/modal.rs`、`crates/window/src/input.rs`、`crates/window/src/background.rs`、`crates/window/src/title.rs`、`crates/window/src/title_bar.rs`、`crates/window/src/style.rs`、`crates/window/src/colors.rs`。

实际前置为[04 在 BSN 与 Scene 展开中保留原始诊断](04-scene-diagnostics.md)、[06 为 Icon 与 SVG 失败保留资产现场](06-core-icon-errors.md)、[13 统一 Button 状态与结构失败的诊断](13-button-loading-errors.md)、[39 保留原生窗口与渲染初始化的失败现场](39-native-window-errors.md)。这些引用表示本项直接使用的能力，不要求相邻编号之间存在调用关系。

## Root、native与相机绑定

真实校验失败保存UI Root、native window、camera、owned资源和失败阶段。Scene/下层Button或Icon错误继承原origin；所有直接构造字符串BevyError和额外日志检查迁入窗口领域分类。

owned_widgetry_window与普通widgetry_window的所有权不同，失败清理不能扩大到外部窗口。部分Scene patch仍非事务，不声称回滚已经提交的组件。

## 输入与关闭不是异常

正常窗口关闭、失焦、native尚未准备、外部事件过滤、相同title/背景等No-op保持原状态。确认属于Window的必需结构被破坏才报错；modal协调和背景渲染继续遵循现有时序。多窗口更新保留全部失败。

## 诊断承接与调用方迁移

错误采用领域分类和统一诊断包装；同一次失败的 ID、source 与原始 Backtrace 在包装/跨线程传递时保持。源头 ERROR 不输出完整栈，纯传播不重复记录；真正消费失败后 DEBUG 记录实际措施，随后诊断可以释放。只有未处理错误在 Bevy 要求的边界显式转为 Severity::Error，最终由宿主输出原栈。正常等待、取消、合法 No-op 不人为转换为错误。

本增量改变的公开签名、回调或结果形状，必须在同一个增量更新所有直接消费者、测试、examples/benches 和所需 Facade 导出。不能用兼容 String 的临时入口掩盖未迁移调用，也不能把基本编译修复推迟到最终 Gallery 增量。

## 完成条件

- 错误window/camera绑定和内部结构破坏可定位所属Root；多个窗口失败不只返回首个。
- owned场景失败回收自己创建的资源，外部native/相机不误删。
- 窗口关闭、modal、焦点、主题/标题更新和多窗口行为回归通过。

在 Windows x86_64-pc-windows-msvc / PowerShell 7 下执行本项相关测试：

```powershell
cargo test -p bevy_widgetry_window
```

对本项实际改动的包与直接消费者进行编译/Clippy 检查；触及 Gallery 时至少检查其编译。GUI、后台时序或性能敏感路径按本节具体场景复验，不重复运行未受影响的整套全仓验收。

同步 Window 场景/生命周期错误与覆盖说明。

代码、构建或测试行为变更按适用工程规则完成必要的独立 Code Review；修复和回归仍归入本增量。

## 保持的边界

只迁移错误表示、传递、处理记录及直接受影响的调用。允许破坏性 API 更改，但不改变既有正常交互、状态通知、资源归属或业务算法；不加入与该错误路径无关的重构。

## 顺序衔接

前一项：[39 保留原生窗口与渲染初始化的失败现场](39-native-window-errors.md)；后一项：[41 为 MessageBox 解析与样式失败保留诊断](41-message-box-errors.md)。编号规定唯一执行顺序，技术依赖以上文列出的能力为准。
