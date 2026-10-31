# 使离屏目标失败保留资源归属与清理结果

为 OffscreenTarget 的分配、绑定和更新提供结构化诊断，并保持失败时资源回收的原有边界。此能力同时被 Canvas 和 Waveform 使用。

## 修改落点与前置能力

涉及：`crates/core/src/offscreen.rs`、`crates/canvas/src/renderer.rs`、`crates/waveform/src/renderer.rs`、`相关导出路径的 OffscreenTarget 调用`。

实际前置为[07 统一 Core 共享状态操作的错误上下文](07-core-state-errors.md)。这些引用表示本项直接使用的能力，不要求相邻编号之间存在调用关系。

## 分配与绑定的所有权

将 layer 耗尽、缺少所需资源、非法 viewport/camera/owned scene、关联破坏、缺少 ComputedNode 等分别建模。错误保存 Owner、viewport、camera、scene、layer 或 image identity，以及检测阶段；不先释放资源再丢失这些定位字段。

bind 失败仍负责它已经接管的目标与拥有的 scene；不能清理调用者不属于本次操作的实体。release/Discard 的资源回收保持幂等边界，显式可失败清理要保留主失败与清理失败各自身份，不能拼接文本。

## 不可见状态不是错误

update 返回 None 表示零尺寸、不可见等正常停用，不创建错误和 DEBUG 处理记录。仅确认绑定结构已经成立后发生破坏才形成失败。

Canvas/Waveform 调用方接收 Core 原诊断并补业务 Owner；原来的更新/清理行为和图层分配域不变，不为错误机制重做渲染通道。

## 诊断承接与调用方迁移

错误采用领域分类和统一诊断包装；同一次失败的 ID、source 与原始 Backtrace 在包装/跨线程传递时保持。源头 ERROR 不输出完整栈，纯传播不重复记录；真正消费失败后 DEBUG 记录实际措施，随后诊断可以释放。只有未处理错误在 Bevy 要求的边界显式转为 Severity::Error，最终由宿主输出原栈。正常等待、取消、合法 No-op 不人为转换为错误。

本增量改变的公开签名、回调或结果形状，必须在同一个增量更新所有直接消费者、测试、examples/benches 和所需 Facade 导出。不能用兼容 String 的临时入口掩盖未迁移调用，也不能把基本编译修复推迟到最终 Gallery 增量。

## 完成条件

- 分配失败、绑定目标无效、关联被破坏、相机缺失均有明确字段和可识别 source。
- 绑定失败后图层/image/本次 owned scene 释放正确，既有其他 Owner 不受影响。
- 零尺寸/不可见/删除中的合法生命周期没有错误；重复创建销毁的资源数量不回归。

在 Windows x86_64-pc-windows-msvc / PowerShell 7 下执行本项相关测试：

```powershell
cargo test -p bevy_widgetry_core -p bevy_widgetry_canvas -p bevy_widgetry_waveform
```

对本项实际改动的包与直接消费者进行编译/Clippy 检查；触及 Gallery 时至少检查其编译。GUI、后台时序或性能敏感路径按本节具体场景复验，不重复运行未受影响的整套全仓验收。

更新 core offscreen 契约和相关资源生命周期测试说明。

代码、构建或测试行为变更按适用工程规则完成必要的独立 Code Review；修复和回归仍归入本增量。

## 保持的边界

只迁移错误表示、传递、处理记录及直接受影响的调用。允许破坏性 API 更改，但不改变既有正常交互、状态通知、资源归属或业务算法；不加入与该错误路径无关的重构。

## 顺序衔接

前一项：[07 统一 Core 共享状态操作的错误上下文](07-core-state-errors.md)；后一项：[09 为主题切换提供可识别的领域失败](09-theme-errors.md)。编号规定唯一执行顺序，技术依赖以上文列出的能力为准。
