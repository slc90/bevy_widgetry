# 为 Icon 与 SVG 失败保留资产现场

把图标路径、SVG 读取/解析、像素分配和图标更新失败统一为 Core 的领域诊断，覆盖同步 API、Template 和 AssetLoader 三种实际入口。

## 修改落点与前置能力

涉及：`crates/core/src/icon.rs`、`crates/core/src/icon/svg.rs`、`crates/core/src/icon/colors.rs`、`crates/asset/src/lib.rs`、`crates/asset/src/cursor_animation.rs`。

实际前置为[04 在 BSN 与 Scene 展开中保留原始诊断](04-scene-diagnostics.md)、[05 区分持续异常状态与新的独立失败](05-persistent-failure-state.md)。这些引用表示本项直接使用的能力，不要求相邻编号之间存在调用关系。

## 从资产读取到像素生成

SvgAssetLoader 的读取和解析错误保留原生 source、AssetPath 与操作阶段，先形成领域失败，再在 Bevy 要求的返回接口处转换。RasterizationError 目前只有 width/height，应保留这些结构化尺寸以及请求尺寸、缩放信息，不再只把说明 format 成字符串。

AssetLoader 返回失败不等于 ECS Handler 已经收到它。图标 Owner 读取 LoadState 时要识别既有诊断，按既有显示/降级契约接续处理或上报；不能重新把同一个加载失败构造成另一条源头 ERROR。正常加载等待和临时尚未物化不能转成错误。

## API 和持续失败

空初始化路径、目标不是 WidgetryIcon、缺少内部 ImageNode、更新 SVG 时缺少 AssetServer，分别保留目标 Entity、资产路径和缺失结构。Template 走新 Scene 适配；同步 setter 返回领域失败，queued setter 在最终 closure 转换。

栅格失败的持续状态使用 Owner 身份，不以 error.to_string() 判断同次失败；恢复 INFO 与真正 fallback 后的处理 DEBUG 分开。迁移时保留前景继承、覆盖色与 disabled 色的既有投影行为。

asset crate 的内建路径、图集元数据和嵌入常量本身没有相应运行期错误对象，不为它们增加空错误枚举。资产登记和加载消费者之间的职责保持原状；验证现有 AssetPlugin 前置条件，不借此重构资源生成工具。

## 诊断承接与调用方迁移

错误采用领域分类和统一诊断包装；同一次失败的 ID、source 与原始 Backtrace 在包装/跨线程传递时保持。源头 ERROR 不输出完整栈，纯传播不重复记录；真正消费失败后 DEBUG 记录实际措施，随后诊断可以释放。只有未处理错误在 Bevy 要求的边界显式转为 Severity::Error，最终由宿主输出原栈。正常等待、取消、合法 No-op 不人为转换为错误。

本增量改变的公开签名、回调或结果形状，必须在同一个增量更新所有直接消费者、测试、examples/benches 和所需 Facade 导出。不能用兼容 String 的临时入口掩盖未迁移调用，也不能把基本编译修复推迟到最终 Gallery 增量。

## 完成条件

- 无效 SVG 读取/解析、非法路径、像素尺寸失败分别验证 source/字段/源头 ID；Template 嵌套后仍可识别。
- 正常等待没有 ERROR；同一加载失败不因每次观察重新捕栈；恢复后新失败产生新 ID。
- 无效目标和缺失 ImageNode 走真实 queued/System 边界，Gallery 与直接 Core 用法均可编译。

在 Windows x86_64-pc-windows-msvc / PowerShell 7 下执行本项相关测试：

```powershell
cargo test -p bevy_widgetry_core -p bevy_widgetry_asset
```

对本项实际改动的包与直接消费者进行编译/Clippy 检查；触及 Gallery 时至少检查其编译。GUI、后台时序或性能敏感路径按本节具体场景复验，不重复运行未受影响的整套全仓验收。

更新 core 图标/SVG 契约及其 COVERAGE；asset 文档仅在职责事实确有改变时同步。

代码、构建或测试行为变更按适用工程规则完成必要的独立 Code Review；修复和回归仍归入本增量。

## 保持的边界

只迁移错误表示、传递、处理记录及直接受影响的调用。允许破坏性 API 更改，但不改变既有正常交互、状态通知、资源归属或业务算法；不加入与该错误路径无关的重构。

## 顺序衔接

前一项：[05 区分持续异常状态与新的独立失败](05-persistent-failure-state.md)；后一项：[07 统一 Core 共享状态操作的错误上下文](07-core-state-errors.md)。编号规定唯一执行顺序，技术依赖以上文列出的能力为准。
