# 日志规则

## 库与宿主的职责

Widgetry 库只负责产生必要的诊断日志，包括判断是否需要记录、选择日志 Level，以及提供定位问题所需的结构化信息。

日志订阅、过滤、格式、输出目标及其他日志配置由宿主 App 负责。Widgetry 库不得自行安装 Subscriber、配置宿主日志系统或增加内部日志开关。

`bevy_widgetry_app_logging` 是供宿主选择使用的应用日志配置能力，不属于 Widgetry 库的诊断实现，也不受库内部日志 Macro 和 Plugin 注册日志要求约束。

## Widgetry 库日志

### 统一入口

Widgetry 库的生产代码统一使用内部日志 Crate 提供的 Macro：

- `widgetry_info!`
- `widgetry_warn!`
- `widgetry_error!`

所有日志统一使用 `target: "bevy_widgetry"`。

库生产代码不得直接使用 Bevy 日志 Macro 绕过统一入口，也不得自行维护另一套日志配置、状态或去重机制。

### Level 与记录条件

**Error**

- Widgetry 自身 Bug、内部 Invariant 被破坏或无法恢复的内部失败。
- 调用方违反公开 API 前置条件，导致操作被拒绝的错误。

记录必要的失败原因及上下文，并按照 `rules/code.md` 传播错误。不得以记录日志代替错误传播。

**Warn**

- 被 Widgetry 内部吸收、没有其他错误通道告知调用方的非预期外部失败。
- 实际功能不可用、关闭或降级，但程序仍可继续运行。

**Info**

- 少量有明确意义的生命周期事件。
- 此前已记录的持续异常恢复正常。

`debug` 和 `trace` Level 保留，但不作为当前库的常规永久日志，也不额外引入对应的统一 Macro。

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

### 失败记录与传播

错误应在能够提供必要诊断上下文的位置记录。

已经记录的失败在后续 `Result` 传播过程中不重复记录，但不得因为错误最终会交给宿主 Error Handler，就省略必要的 Widgetry 错误日志。

已确认属于 Widgetry 管理的对象如果缺少必需内部结构，应按真实错误处理，不得伪装成正常的 Observer 过滤或静默忽略。

错误传播和 `Severity::Error` 的具体要求遵循 `rules/code.md`。

### 持续异常与恢复

对于可能持续发生的同一异常，应按照实际状态变化记录，避免每帧重复输出：

- 正常 → 异常：记录一次。
- 异常持续存在：不重复记录。
- 异常 → 恢复正常：此前记录过异常时，记录一次 Info。
- 恢复后再次异常：重新记录。

异常状态应由负责该行为的 Module 管理，不依赖 System 执行次数判断。

正常等待不代表异常已经恢复；Entity 销毁也不自动产生恢复日志。

## Gallery 日志

Gallery 使用普通 Bevy `info!`、`warn!`、`error!` Macro，不使用 Widgetry 内部日志 Macro，也不固定 `bevy_widgetry` Target。

### Level 与语义

- **Info**：具有明确演示意义的离散操作及其结果。
- **Warn**：操作失败但 Gallery 可以继续运行，或演示能力不可用、关闭、降级。
- **Error**：Gallery 自身 Bug、内部 Invariant 破坏或无法正常继续的失败。

Gallery 应记录具有明确演示语义的操作，例如页面切换、Theme 切换、Widget 值确认、打开 Window / Dialog，以及外部操作的发起与结果。

日志描述业务操作，而不是底层输入机制。例如记录“打开独立窗口”，而不是“收到 Pointer Click”。

运行期具有明确演示意义的程序化操作，也应在 Gallery 调用处记录，并与用户操作区分。

异步或排队操作只能记录当前已经确认发生的事实，不得在结果尚未确定时提前记录成功。

正常初始化保持静默。Hover、Pointer Move、Press / Release、普通 Focus 变化、逐字符输入及内部更新轨迹等连续或中间状态默认不记录。

连续交互最终产生明确的确认、提交或选择结果时，可以在结果发生时记录一次。

日志应附着在现有业务处理路径上，不得为了日志额外增加 Polling System、全局输入监听或底层 Event Tracing，也不得改变原有交互和异步行为。

## 日志内容与结构化字段

日志正文统一使用中文，动态信息使用英文命名的结构化字段，例如 `entity`、`path`、`error`、`width`、`height`。

正文说明发生了什么，结构化字段提供必要的诊断上下文。

不手动添加 `file`、`line`、`module_path` 或 `crate_name` 等已有 Tracing Metadata，由宿主决定其显示方式。
