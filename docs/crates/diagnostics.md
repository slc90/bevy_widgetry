# Diagnostics

`bevy_widgetry_diagnostics` 是不依赖 Bevy、UI 或宿主日志配置的底层诊断载体。当前只依赖 `internal_log`，后者直接向 Tracing 发出事件。Job、原生能力与构建脚本可独立使用。

## 数据与创建

`WidgetryFailure<E>::new(kind, ErrorFrame)` 首次纳入真实领域错误，保存 E、共享的 `Arc<ErrorOrigin>` 和拥有的上下文。`ErrorOrigin` 保存进程内唯一 `ErrorId`、`#[track_caller]` 业务调用位置和 `Backtrace::force_capture()` 的原始线程栈，不依赖回溯环境变量。

ID 由受短期 Mutex 保护的可增长无符号计数器分配；进位增加字长，不静默回绕复用，不以诊断递归报告 ID 耗尽。只在真实失败创建时分配，不增加正常交互或逐帧工作。

`ErrorFrame::new(operation).field(name, value)` 保存操作及对象的展示上下文。具有业务意义的 Entity、PathBuf、请求号、数值等原始类型由 E 保存；frame 的拥有字符串不替代真实 source。E 实现 `std::error::Error`，`kind()` 可匹配领域分类，`source()` 暴露原始 E 及其原因链。构造和普通格式化使用错误的 Display 与 source 描述；领域错误应将必要结构化业务信息纳入可读说明。

## 传播、处理与生命周期

`with_context()` 增加传播语境，`map_kind()` 转换领域类型并保留同一 origin/ID/上下文，不重捕栈或发出日志。转换函数负责将原来的 E 保留在上层错误中，不能以字符串替换真实原因。没有对领域错误或 Backtrace 的 Clone/Eq 要求；满足 Send + Sync 的失败可经 Arc 跨线程共享。

源头通过统一入口发出一次 ERROR，包括 `error_id`、完整可读原因链与当时已知 context。`record_handled(action, outcome)` 只在调用方实际采取措施后显式调用；它按同一 origin 记录首次处理的 DEBUG。仅观察、转换和 Drop 都不构成处理。该标记只控制日志，不替调用方执行恢复，也不能证明宿主已经接收或持久化事件。

普通 Display/Debug 不输出原始 Backtrace。最终宿主可显式读取 `origin().backtrace()`；捕获位置是当前线程首次纳入的位置，不代表外部依赖内部源头或跨线程完整执行轨迹。没有全局诊断队列；最后一个拥有者释放时诊断正常销毁。

此层不安装 Subscriber 或应用 Error Handler，也尚未提供 Bevy ECS 转换。日志接收、过滤、最终未处理报告与发布符号由宿主负责。
