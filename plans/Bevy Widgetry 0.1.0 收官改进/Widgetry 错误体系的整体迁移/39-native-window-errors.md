# 保留原生窗口与渲染初始化的失败现场

迁移透明DX12初始化与任务栏图标的真实失败。它们有些发生在App运行前，不能假定自然进入ECS Handler。

## 修改落点与前置能力

涉及：`crates/window/src/render.rs`、`crates/window/src/native_icon.rs`、`crates/window/src/lib.rs`、`Gallery启动阶段的相关调用`。

实际前置为[02 保留诊断身份地进入 Bevy 错误边界](02-bevy-error-bridge.md)、[05 区分持续异常状态与新的独立失败](05-persistent-failure-state.md)。这些引用表示本项直接使用的能力，不要求相邻编号之间存在调用关系。

## 初始化来源与原生source

transparent_render_creation的adapter/device请求失败保留wgpu source、backend和阶段，不以普通BevyError自动转换丢失既有诊断。native_icon_plugin与native_icon_from_rgba保留path、width/height、格式和image/winit source。

这些同步/async普通函数返回领域失败，调用Gallery的启动入口需要显式接管。对已开始运行的set_primary_window_icons System，缺少/歧义PrimaryWindow是现有真实失败；尚未创建对应native window是正常等待。

## 持续失败与主线程

替换Local<Option<String>>的显示文案去重，按主窗口/配置状态保留诊断身份。图标实际设置成功才清除持续状态并INFO。保留NonSendMarker、WINIT_WINDOWS主线程约束和不重复设置图标的行为。

## 诊断承接与调用方迁移

错误采用领域分类和统一诊断包装；同一次失败的 ID、source 与原始 Backtrace 在包装/跨线程传递时保持。源头 ERROR 不输出完整栈，纯传播不重复记录；真正消费失败后 DEBUG 记录实际措施，随后诊断可以释放。只有未处理错误在 Bevy 要求的边界显式转为 Severity::Error，最终由宿主输出原栈。正常等待、取消、合法 No-op 不人为转换为错误。

本增量改变的公开签名、回调或结果形状，必须在同一个增量更新所有直接消费者、测试、examples/benches 和所需 Facade 导出。不能用兼容 String 的临时入口掩盖未迁移调用，也不能把基本编译修复推迟到最终 Gallery 增量。

## 完成条件

- PNG解码/RGBA尺寸/adapter/device失败保留具体source与字段；启动调用方不依赖ECS自动兜底。
- 缺PrimaryWindow和两个PrimaryWindow在真实System边界上报；native window尚未创建没有ERROR。
- 重复失败、成功应用和恢复日志的次数与origin正确；无额外线程访问native句柄。

在 Windows x86_64-pc-windows-msvc / PowerShell 7 下执行本项相关测试：

```powershell
cargo test -p bevy_widgetry_window
```

对本项实际改动的包与直接消费者进行编译/Clippy 检查；触及 Gallery 时至少检查其编译。GUI、后台时序或性能敏感路径按本节具体场景复验，不重复运行未受影响的整套全仓验收。

同步 Window 原生初始化及startup错误返回契约。

代码、构建或测试行为变更按适用工程规则完成必要的独立 Code Review；修复和回归仍归入本增量。

## 保持的边界

只迁移错误表示、传递、处理记录及直接受影响的调用。允许破坏性 API 更改，但不改变既有正常交互、状态通知、资源归属或业务算法；不加入与该错误路径无关的重构。

## 顺序衔接

前一项：[38 区分 Table 编辑拒绝与编辑基础设施失败](38-table-edit-errors.md)；后一项：[40 使 Window 场景与生命周期失败保持拥有关系](40-window-scene-errors.md)。编号规定唯一执行顺序，技术依赖以上文列出的能力为准。
