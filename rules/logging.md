# 日志规则

## 库与宿主的职责

Widgetry 库负责产生必要的诊断日志，包括选择日志 Level，以及提供定位问题所需的结构化信息。

日志订阅、过滤、格式、输出目标及其他日志配置由宿主 App 负责。Widgetry 库不得自行安装 Subscriber、配置宿主日志系统或增加内部日志开关。

`bevy_widgetry_app_logging` 是供宿主选择使用的应用日志配置能力，不属于 Widgetry 库的诊断实现，也不受库内部日志 Macro 和 Plugin 注册日志要求约束。

错误的构造、诊断信息、生命周期、传播和 ECS 兜底遵循 `rules/code.md`，本文不重复定义。

## Widgetry 库日志

### 统一入口

Widgetry 库的生产代码统一使用内部日志 Crate 提供的 Macro：

- `widgetry_debug!`
- `widgetry_info!`
- `widgetry_warn!`
- `widgetry_error!`

所有日志统一使用 `target: "bevy_widgetry"`。

库生产代码不得直接使用 Bevy 日志 Macro 绕过统一入口，也不得自行维护另一套日志订阅、过滤、持久化或全局去重机制。

### Level 与记录条件

**Error**

用于记录真实错误的首次发生。

所有按照 `rules/code.md` 定义的真实错误均须记录 ERROR，不因后续可能被处理而降低日志等级。

ERROR 日志应包含错误 ID、失败原因和当前已经掌握的必要业务上下文，不输出完整 Backtrace。

不得以记录 ERROR 代替错误处理或传播。

**Warn**

用于尚未构成真实错误，但值得关注的异常状态、环境风险或功能受限情况。

已经构成真实错误的失败，不得因其可以恢复或降级而改用 WARN。

**Info**

- 少量有明确意义的生命周期事件。
- 此前已记录的持续异常恢复正常。

**Debug**

用于记录真实错误首次被实际处理的结果，包括错误 ID 和具体处理措施。

不用于记录正常执行路径、内部状态变化或普通调试轨迹。

**Trace**

保留 Trace Level，但不作为当前库的常规永久日志，也不额外提供统一 Macro。

### Plugin 注册

**所有 Widgetry Plugin 都必须在 `build()` 完成时记录一次：**

```text
<PluginName> 注册完成
```

不使用“初始化成功”等替代描述，也不重复记录内部 Resource、System、Observer、Loader 或 Cache 的注册细节。

### 正常路径

正常执行路径不记录诊断日志，包括：

- 合法的 API 调用、同值设置和 No-op。
- 正常 Pointer、Keyboard、Hover、Focus、Selection、Drag 等交互及状态变化。
- 正常的程序化 Widget 操作。
- 正常 Asset 等待、Fallback、Observer 目标过滤及初始化过程。
- 内部 State 同步、Cache、Layout、Style 更新等执行轨迹。
- 未造成实际功能异常的普通环境信息。

日志不用于替代 Event、Message、Profiler 或临时调试工具。

不得仅为了日志增加持续轮询、每帧遍历或无业务意义的运行时状态。

### 错误日志关联与去重

同一次失败产生的日志应使用一致的 `error_id`。

不同阶段的日志应分别提供不同的诊断信息：

- 源头 ERROR：失败原因及发生时的业务现场。
- 处理 DEBUG：实际处理结果及采取的措施。
- ECS 兜底 ERROR：最终未处理状态、ECS 执行上下文及原始 Backtrace。

不得在错误传播过程中重复记录相同的失败原因。

不得因为源头已经记录 ERROR，就省略按照 `rules/code.md` 要求的最终兜底信息。

错误日志的生命周期及何时构成实际处理，统一遵循 `rules/code.md`。

### 持续异常与恢复

对于可能持续存在的同一异常状态，应按照实际状态变化记录，避免每帧重复输出：

- 正常 → 异常：记录一次。
- 异常持续存在：不重复记录。
- 异常 → 恢复正常：此前记录过异常时，记录一次 INFO。
- 恢复后再次异常：重新记录。

上述状态去重仅适用于同一异常状态持续存在的情况，不得用来抑制新产生的独立错误。

异常状态应由负责该行为的 Module 管理，不依赖 System 执行次数判断。

正常等待不代表异常已经恢复；Entity 销毁也不自动产生恢复日志。

持续异常恢复时的 INFO 与某次错误被实际处理时的 DEBUG 具有不同语义，不得混淆。

## Gallery 日志

Gallery 使用普通 Bevy `debug!`、`info!`、`warn!`、`error!` Macro，不使用 Widgetry 内部日志 Macro，也不固定 `bevy_widgetry` Target。

### Level 与语义

- **Debug**：实际处理错误时的处理结果与措施。
- **Info**：具有明确演示意义的离散操作及其结果。
- **Warn**：尚未构成真实错误的功能受限、环境风险或异常状态。
- **Error**：Gallery 自身产生的真实错误，包括 Bug、内部 Invariant 破坏和其他实际操作失败。

Gallery 自身产生的错误同样遵循 `rules/code.md` 的错误生命周期约定。

Gallery 应记录具有明确演示语义的操作，例如页面切换、Theme 切换、Widget 值确认、打开 Window / Dialog，以及外部操作的发起与结果。

日志描述业务操作，而不是底层输入机制。例如记录“打开独立窗口”，而不是“收到 Pointer Click”。

运行期具有明确演示意义的程序化操作，也应在 Gallery 调用处记录，并与用户操作区分。

异步或排队操作只能记录当前已经确认发生的事实，不得在结果尚未确定时提前记录成功。

正常初始化保持静默。Hover、Pointer Move、Press / Release、普通 Focus 变化、逐字符输入及内部更新轨迹等连续或中间状态默认不记录。

连续交互最终产生明确的确认、提交或选择结果时，可以在结果发生时记录一次。

日志应附着在现有业务处理路径上，不得为了日志额外增加 Polling System、全局输入监听或底层 Event Tracing，也不得改变原有交互和异步行为。

### 日志过滤与持久化

正式 Gallery 必须确保错误诊断所需的日志能够正常输出和保存，包括源头 ERROR、错误处理 DEBUG 以及 ECS 兜底 ERROR。

日志过滤规则不得意外过滤上述必要记录。

日志输出应优先保证诊断信息可靠保存。不得使用会在缓冲区满时静默丢弃必要错误记录的默认有损配置。

Backtrace 的捕获及最终输出要求遵循 `rules/code.md`。

## 日志内容与结构化字段

日志正文统一使用中文，动态信息使用英文命名的结构化字段，例如 `error_id`、`entity`、`path`、`error`、`action`、`width`、`height`。

正文说明发生了什么，结构化字段提供必要的诊断上下文。

需要关联同一次失败的日志必须使用相同的 `error_id`。

不手动添加 `file`、`line`、`module_path` 或 `crate_name` 等已有 Tracing Metadata，由宿主决定其显示方式。
