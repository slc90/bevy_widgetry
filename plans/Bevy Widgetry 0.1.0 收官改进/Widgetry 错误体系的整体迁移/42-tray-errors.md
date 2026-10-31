# 保留 Tray 原生回调到 ECS 的未处理诊断

迁移托盘初始化和回调桥接，修正错误队列满时连源头日志也可能丢失的路径。

## 修改落点与前置能力

涉及：`crates/tray/src/plugin.rs`、`crates/tray/src/bridge.rs`、`crates/tray/src/lib.rs`、`Gallery托盘初始化与消息消费`。

实际前置为[02 保留诊断身份地进入 Bevy 错误边界](02-bevy-error-bridge.md)、[03 让测试能观察完整错误生命周期](03-diagnostic-test-support.md)、[05 区分持续异常状态与新的独立失败](05-persistent-failure-state.md)。这些引用表示本项直接使用的能力，不要求相邻编号之间存在调用关系。

## 初始化与异步桥接

tray_error(Display)改为保留ICO解码、RGBA转换、托盘创建/注册确认和wake错误的原source，附图标尺寸、操作、原生对象身份。初始化普通API和Startup边界分别承担返回与转换。

Bridge::fail当前只有errors.try_send成功才记录，容量1满时丢弃后续错误。改为源头构造先记录；真正未处理诊断必须由现有bridge所有者保留到ECS消费，不能将try_send失败视为处理完成。点击队列仍保留原容量与语义，诊断递送与业务点击递送分开；同状态故障按Owner维护，新的独立故障不按文案合并。具体待报容器需保证回调线程不因等待同一个主线程消费而死锁。

## 回调与释放

非本托盘、非左键释放、失效Weak路由和正常退出是合法过滤。路由锁故障、wake失败等实际异常使用明确领域错误，不to_string后丢弃。Registration::drop不能把普通Result假设交给Bevy；可显式关闭的失败交给其应用/生命周期Owner，确实完成安全放弃/释放才记录DEBUG。

保留一次安装原生事件路由、主线程图标生命周期和Reactive唤醒，不建立新的全局日志/去重系统。

## 诊断承接与调用方迁移

错误采用领域分类和统一诊断包装；同一次失败的 ID、source 与原始 Backtrace 在包装/跨线程传递时保持。源头 ERROR 不输出完整栈，纯传播不重复记录；真正消费失败后 DEBUG 记录实际措施，随后诊断可以释放。只有未处理错误在 Bevy 要求的边界显式转为 Severity::Error，最终由宿主输出原栈。正常等待、取消、合法 No-op 不人为转换为错误。

本增量改变的公开签名、回调或结果形状，必须在同一个增量更新所有直接消费者、测试、examples/benches 和所需 Facade 导出。不能用兼容 String 的临时入口掩盖未迁移调用，也不能把基本编译修复推迟到最终 Gallery 增量。

## 完成条件

- ICO/原生创建失败保留source；真实Startup接收同ID与原栈。
- 连续多个真实bridge失败不被单槽静默丢弃；健康点击仍按原语义转发，过载测试不死锁。
- 退出、失效路由和普通过滤无错误；Drop/显式关闭的处理记录只陈述实际结果。

在 Windows x86_64-pc-windows-msvc / PowerShell 7 下执行本项相关测试：

```powershell
cargo test -p bevy_widgetry_tray
```

对本项实际改动的包与直接消费者进行编译/Clippy 检查；触及 Gallery 时至少检查其编译。GUI、后台时序或性能敏感路径按本节具体场景复验，不重复运行未受影响的整套全仓验收。

同步 Tray 回调错误通道与覆盖，说明非ECS回调的责任归属。

代码、构建或测试行为变更按适用工程规则完成必要的独立 Code Review；修复和回归仍归入本增量。

## 保持的边界

只迁移错误表示、传递、处理记录及直接受影响的调用。允许破坏性 API 更改，但不改变既有正常交互、状态通知、资源归属或业务算法；不加入与该错误路径无关的重构。

## 顺序衔接

前一项：[41 为 MessageBox 解析与样式失败保留诊断](41-message-box-errors.md)；后一项：[43 保留电源请求主失败与清理失败的各自原因](43-power-inhibit-errors.md)。编号规定唯一执行顺序，技术依赖以上文列出的能力为准。
