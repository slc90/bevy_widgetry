# Diagnostics

`bevy_widgetry_diagnostics` 默认是不依赖 Bevy、UI 或宿主日志配置的底层诊断载体。默认只依赖 `internal_log`，后者直接向 Tracing 发出事件。Job、原生能力与构建脚本可独立使用。

## 数据与创建

`WidgetryFailure<E>::new(kind, ErrorFrame)` 首次纳入真实领域错误，保存 E、共享的 `Arc<ErrorOrigin>` 和拥有的上下文。`ErrorOrigin` 保存进程内唯一 `ErrorId`、`#[track_caller]` 业务调用位置和 `Backtrace::force_capture()` 的原始线程栈，不依赖回溯环境变量。

ID 由受短期 Mutex 保护的可增长无符号计数器分配；进位增加字长，不静默回绕复用，不以诊断递归报告 ID 耗尽。只在真实失败创建时分配，不增加正常交互或逐帧工作。

`ErrorFrame::new(operation).field(name, value)` 保存操作及对象的展示上下文。具有业务意义的 Entity、PathBuf、请求号、数值等原始类型由 E 保存；frame 的拥有字符串不替代真实 source。E 实现 `std::error::Error`，`kind()` 可匹配领域分类，`source()` 暴露原始 E 及其原因链。构造和普通格式化使用错误的 Display 与 source 描述；领域错误应将必要结构化业务信息纳入可读说明。

## 传播、处理与生命周期

`with_context()` 增加传播语境，`map_kind()` 转换领域类型并保留同一 origin/ID/上下文，不重捕栈或发出日志。转换函数负责将原来的 E 保留在上层错误中，不能以字符串替换真实原因。没有对领域错误或 Backtrace 的 Clone/Eq 要求；满足 Send + Sync 的失败可经 Arc 跨线程共享。

源头通过统一入口发出一次 ERROR，包括 `error_id`、`phase = "source"`、完整可读原因链与当时已知 context。`record_handled(action, outcome)` 只在调用方实际采取措施后显式调用；它按同一 origin 记录首次处理的 DEBUG（`phase = "handled"`）。仅观察、转换和 Drop 都不构成处理。该标记只控制日志，不替调用方执行恢复，也不能证明宿主已经接收或持久化事件。宿主的最终未处理事件可使用 `phase = "unhandled"` 与同一 ID，供测试查询关联；库不自行产生终结事件。

普通 Display/Debug 不输出原始 Backtrace。最终宿主可显式读取 `origin().backtrace()`；捕获位置是当前线程首次纳入的位置，不代表外部依赖内部源头或跨线程完整执行轨迹。没有全局诊断队列；最后一个拥有者释放时诊断正常销毁。

此层不安装 Subscriber 或应用 Error Handler。日志接收、过滤、最终未处理报告与发布符号由宿主负责。

## 可选 Bevy ECS 边界

默认关闭的 `bevy` Feature 只增加 `bevy_ecs/std` 依赖，公开 `bevy` 模块。`into_bevy_error(failure)` 或 `WidgetryEcsError::into_bevy_error()` 显式转换为 `Severity::Error`，非泛型 `WidgetryEcsError` 让宿主一次 downcast 即可识别任意领域错误。不能用裸 `?` 的默认转换替代该入口。

`Diagnostic` 的 `origin()`、`contexts()` 及标准 source 链保留原始诊断。`WidgetryEcsError::from_shared()` 直接转交共享结果，`push_shared()` 附加已存在的共享诊断；`with_context()` 保留 origin，并在各诊断上添加拥有的上下文。`DiagnosticBatch` 接受已形成的失败或共享诊断；`into_bevy_error()` 对空集合返回 None，否则保持每项的身份、原因与原栈，不为集合生成新的源头或日志。标准 `source()` 只能指向首项，完整集合通过 `diagnostics()` 逐项读取。

`BoundaryFailure` 明确区分受管错误和原生 `BevyError`。`Native` 保持原始 Bevy 对象，补 context 使用 Bevy 自己的列表，保留 Severity、底层 downcast 及 panic payload；不要求 BevyError 实现标准 Error，不将其重建为字符串。

`dispatch_error(error, context, report_managed, report_native)` 将 Error Severity 的受管集合逐项交给宿主回调，同时提供原始外层 `&BevyError` 和 ECS `&ErrorContext`，保留 Bevy 自身附加的 context；其余错误完整交给原生策略回调。宿主可选择 Bevy 的 `match_severity`，保留外来错误与 Panic 语义。此函数不自行打最终日志，不配置 App，也不会用 Bevy 边界处的回溯替代 `origin().backtrace()`。第三方不透明包装必须在拥有它的适配层显式保留受管载体，不能假设标准 source 能穿透 Bevy 的私有内部表示。
