# 代码规则

## 主动 Panic

Widgetry 各 crate 的非测试代码禁止主动触发 panic，包括内部 invariant 被破坏或发生无法恢复的错误时。

测试代码允许使用断言、panic、`unwrap()` 和 `expect()`，但测试基础设施仍须正确处理运行期失败。

具体的禁止项及测试例外由 Workspace lint 配置执行。

## 错误处理与传播

不得静默吞掉真实错误。

- 当前层能够正确处理的错误，应在当前层处理。
- 当前层无法处理的错误，应保留必要的诊断信息并向上传播。
- 内部 invariant 被破坏或发生无法恢复的失败时，必须记录 ERROR 并传播错误，不得仅记录日志后忽略。

Widgetry 创建或转换的错误统一使用 `Severity::Error`，例如 `BevyError::error`。不得依赖可能产生 `Severity::Panic` 的默认转换。传播下层错误时，应保留原始错误信息并正确设置 Severity。

普通 function 通过 `Result` 向调用方传播错误；system、observer 和 command 应将无法处理的错误交给 Bevy error handler。Widgetry 库不得自行配置宿主 App 的 error handler。

错误日志应在掌握必要上下文的位置记录，传播过程中不重复记录相同失败。具体日志要求遵循 `rules/logging.md`。

正常的 `Option` 缺失、Asset 等待、Observer 目标过滤、交互 guard，以及公开 API 明确定义的合法 no-op，不应人为转换为错误。

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
