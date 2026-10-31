# 代码规则

## 主动 Panic

Widgetry 各 crate 的非测试代码禁止主动触发 panic，包括内部 invariant 被破坏或发生无法恢复的错误时。

测试代码允许使用断言、panic、`unwrap()` 和 `expect()`，但测试基础设施仍须正确处理运行期失败。

具体的禁止项及测试例外由 Workspace lint 配置执行。

## 错误处理与传播

### 错误语义

不得静默吞掉真实错误，也不得以日志记录代替错误传播。

- 当前层能够正确处理的错误，应在当前层处理；无法处理的错误必须向上传播。
- 内部 Invariant 被破坏或发生无法恢复的失败时，必须作为真实错误处理，不得主动触发 Panic，也不得仅记录日志后继续执行。
- 正常的 `Option` 缺失、Asset 等待、Observer 目标过滤、交互 Guard，以及公开 API 明确定义的合法 No-op，不应人为转换为错误。
- 不得为了避免产生 ERROR 日志而将真实失败伪装成正常状态。

### 错误类型与诊断信息

各 Crate 应根据自身业务语义定义领域错误类型，保留可供调用方识别和匹配的错误分类，以及必要的结构化业务字段。

Widgetry 使用统一的诊断包装机制承载错误信息，避免各 Crate 独立实现错误生命周期管理。

统一诊断信息至少包括：

- 错误唯一 ID，用于关联同一次失败的日志。
- 原始领域错误及其底层错误原因链。
- 错误首次产生的位置。
- 错误首次产生时捕获的 Backtrace。
- 错误传播过程中补充的上下文。

错误首次产生时必须捕获 Backtrace，且正式运行环境不依赖 `RUST_BACKTRACE` 环境变量。后续传播不得重新捕获或覆盖原始 Backtrace。

传播过程中可以补充上下文，但必须保留已有错误信息。不得仅通过 `to_string()` 转换而丢弃仍可保留的原始错误类型、结构化字段或错误原因链。

错误诊断数据应随错误对象管理生命周期，不要求为已处理的错误长期保留 Backtrace。

### 错误日志与生命周期

真实错误首次产生时，必须记录一次 ERROR 日志，包含错误 ID、失败原因和必要的现场信息。此时不要求输出完整 Backtrace。

来自外部依赖的错误，应在首次纳入 Widgetry 错误体系的位置记录，保留原始错误及来源信息。

错误日志遵循以下生命周期约定：

- **首次产生**：记录 ERROR，创建并保存完整诊断信息。
- **仅传播**：不重复记录同一次失败的 ERROR；允许补充上下文。
- **首次被实际处理**：记录 DEBUG，包含错误 ID 和明确的处理措施，例如恢复、降级、取消或终止本次操作。
- **继续传播至 ECS 边界**：由 Bevy Error Handler 进行最终兜底。

通过 `?`、`map_err()` 或其他方式传递、包装错误，不等同于成功处理错误，不得因此记录「已处理」日志。

错误被实际处理后，其对象及诊断信息可以正常释放，不要求输出 Backtrace。

### ECS 边界与全局错误处理器

普通函数通过 `Result` 向调用方传播领域错误。到达 System、Observer、Command 等 Bevy ECS 执行边界时，无法处理的错误必须转换为 `BevyError` 并继续传播。

Widgetry 创建或转换的 `BevyError` 统一使用 `Severity::Error`，不得依赖可能产生 `Severity::Panic` 的默认转换。

转换为 `BevyError` 时，必须保留原始领域错误、错误 ID、错误上下文与首次捕获的 Backtrace。

Widgetry 库不得自行配置宿主 App 的 Error Handler。

Gallery 应配置统一的兜底 Error Handler：

- 对于可识别的 Widgetry 错误，记录错误最终未被处理的事实、错误 ID、Bevy 提供的 `ErrorContext` 以及原始完整 Backtrace，不重复输出已在源头记录的错误描述。
- 对于非 Widgetry 错误，保留完整错误原因及必要的 ECS 诊断信息，不得因缺少 Widgetry 诊断包装而静默忽略。
- Backtrace 的记录不依赖启动时手动设置环境变量；正式 Gallery 独立运行时也必须启用对应能力。

错误日志的输出、过滤、持久化等由宿主 App 负责。具体日志约定遵循 `rules/logging.md`。

## BSN 与 Scene 错误

SceneComponent 的构造校验应通过可失败的 Template 或 Scene 展开路径处理，不得依赖 panic。

Deferred BSN 构造使用 `core` / `facade` Scene Module 提供的 `spawn_scene_with_error_handler` 和 `apply_scene_with_error_handler`，将失败显式交给宿主 error handler。

不得依赖原生 `Commands::spawn_scene` 的仅日志处理，或 `EntityCommands::apply_scene` 的默认 Panic Severity。

同步 `World` / `EntityWorldMut` Scene API 返回的错误必须由调用层处理，并以 `Severity::Error` 传播。

内部同步 Renderer 展开使用 `core::scene` 提供的失败清理适配，由对应业务 Owner 负责错误记录和传播。

Scene 构造失败时，应清理本次 Template 新建的空 Entity。独立业务 Entity 应在创建时携带必要 Component，避免失败后留下无意义的空 Entity。

Scene 构造失败不意味着自动回滚已经写入的业务 Component 或已有 Root 上的部分 Patch，不得假定整个 Scene 操作具有事务语义。

## Lint Suppression

只有当 lint 在当前语义下确实不适用，或存在无法合理消除的必要例外时，才允许使用 `#[allow(...)]` 等局部 suppression。

- 优先修正产生 lint 的代码，而不是 suppress lint。
- Suppression 必须限制在必要的最小范围。
- 不得通过大范围 suppression 掩盖实际问题。
- 根目录 `Cargo.toml` 中明确配置的 Workspace 级 lint 例外不受局部 suppression 限制。

不强制为每处 suppression 添加解释性注释；仅当例外原因不明显且有必要说明时，按照 `rules/documentation.md` 记录。

## 命名约定

- Component 类型不追加 `Component` 后缀。
- Resource 类型不追加 `Resource` 后缀。
- 实现 Bevy Plugin 的类型使用 `Plugin` 后缀。
- 表达 Style 数据或解析后 Style 值的类型使用 `Style` 后缀。

其他命名遵循 Rust 惯例，以实际语义清晰为准，不额外规定统一后缀或命名模板。
