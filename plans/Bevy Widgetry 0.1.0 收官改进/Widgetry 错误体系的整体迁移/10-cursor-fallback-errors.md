# 记录光标资源失败与实际降级结果

把 Kamuri 光标加载/像素校验失败从单条 WARN 改为有身份的源头 ERROR 加实际 fallback 的 DEBUG，正常冷加载仍然静默。

## 修改落点与前置能力

涉及：`crates/cursor/src/presentation.rs`、`crates/cursor/src/coordination.rs`、`crates/cursor/src/lease.rs`、`crates/cursor/src/theme.rs`。

实际前置为[05 区分持续异常状态与新的独立失败](05-persistent-failure-state.md)、[06 为 Icon 与 SVG 失败保留资产现场](06-core-icon-errors.md)。这些引用表示本项直接使用的能力，不要求相邻编号之间存在调用关系。

## 失败和等待分开

presentation::usable 当前把 LoadState::Failed 转成 String，并用 Option<String> 记录持续 issue。改为保存资产身份、预期/实际尺寸、像素格式/长度和原加载原因；失败首次纳入时记录 ERROR，后续同状态观察不新捕栈。

只有真正选择静态图集或系统光标之后记录处理 DEBUG，注明选择结果。Loading、缺少尚未准备的运行资源以及正常主题切换不能直接当成失败；原始加载 source 已受管时不重建 ID。资源恢复继续 INFO。

## 维持已有响应节奏

保持冷加载期间的 RequestRedraw、失败/不可用后不空转、动画计时与窗口生命周期回收。CursorLease::dragging/native 的 Option 表示鼠标/窗口/位置条件，不机械转换为 Err。

同一图集被多个窗口消费时，由资源失败 Owner 定义一次失败身份；窗口实际选择 fallback 的信息放在消费动作中，不为每帧显示重复 ERROR。

## 诊断承接与调用方迁移

错误采用领域分类和统一诊断包装；同一次失败的 ID、source 与原始 Backtrace 在包装/跨线程传递时保持。源头 ERROR 不输出完整栈，纯传播不重复记录；真正消费失败后 DEBUG 记录实际措施，随后诊断可以释放。只有未处理错误在 Bevy 要求的边界显式转为 Severity::Error，最终由宿主输出原栈。正常等待、取消、合法 No-op 不人为转换为错误。

本增量改变的公开签名、回调或结果形状，必须在同一个增量更新所有直接消费者、测试、examples/benches 和所需 Facade 导出。不能用兼容 String 的临时入口掩盖未迁移调用，也不能把基本编译修复推迟到最终 Gallery 增量。

## 完成条件

- 加载中无错误、加载失败一条源头 ERROR、实际切换 fallback 有关联 DEBUG、恢复有 INFO。
- 像素维度/格式/长度错误能看到实际值；两个独立资源失败不会按文案合并。
- 保持热/冷加载、动画 redraw 和 pointer lease 的既有行为测试。

在 Windows x86_64-pc-windows-msvc / PowerShell 7 下执行本项相关测试：

```powershell
cargo test -p bevy_widgetry_cursor
```

对本项实际改动的包与直接消费者进行编译/Clippy 检查；触及 Gallery 时至少检查其编译。GUI、后台时序或性能敏感路径按本节具体场景复验，不重复运行未受影响的整套全仓验收。

同步 cursor 资源降级说明及 COVERAGE。

代码、构建或测试行为变更按适用工程规则完成必要的独立 Code Review；修复和回归仍归入本增量。

## 保持的边界

只迁移错误表示、传递、处理记录及直接受影响的调用。允许破坏性 API 更改，但不改变既有正常交互、状态通知、资源归属或业务算法；不加入与该错误路径无关的重构。

## 顺序衔接

前一项：[09 为主题切换提供可识别的领域失败](09-theme-errors.md)；后一项：[11 为 Job 提交与控制 API 保留结构化失败](11-job-control-errors.md)。编号规定唯一执行顺序，技术依赖以上文列出的能力为准。
