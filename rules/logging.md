# 日志规则

## 库与宿主的职责

Widgetry 只负责产生诊断事件：判断是否值得记录，选择等级并提供结构化上下文。
subscriber、过滤、`RUST_LOG`、格式、时间、ANSI、源码位置和文件输出由宿主 App 负责。
库不得初始化 subscriber，不得提供自有日志开关、等级 Resource 或日志配置系统。

## Widgetry 库统一入口

库生产代码必须使用底层 `bevy_widgetry_log` 的 `widgetry_info!`、`widgetry_warn!`、`widgetry_error!`。
宏仅固定 `target: "bevy_widgetry"` 并转发 Bevy/tracing 语法，不维护配置、状态或去重。
禁止直接调用 Bevy 日志宏或用其他方式绕过统一 target。该 crate 不由 facade 导出。
统一宏不属于应用业务 API，Gallery 的入口规则见下文。

## Widgetry 库等级与责任

- `error`：Widgetry 自身 bug、内部不变量或必需内部状态被破坏、无法恢复的内部失败。
  如果这些错误最终导致 panic/assert，必须先记录 ERROR；panic 输出不能代替诊断事件。
- `warn`：程序仍运行，但出现被内部吸收且没有其他错误通道告知调用者的非预期外部失败，或实际功能不可用、关闭、降级。
- `info`：少量生命周期事实，以及此前已记录的持续异常恢复正常。
- `debug`、`trace`：等级保留；当前不提供对应宏，也不新增永久日志。

所有 Widgetry `impl Plugin` 必须在 `build()` 完成时记录一次 `<PluginName> 注册完成`。
不使用“初始化成功”，不记录版本号，不继续记录内部 Resource、system、observer、loader 或 cache 的注册细节。

## Widgetry 库不记录的情况

- 调用方非法参数、错误顺序或违反公开 API 前置条件，包括因此发生的 panic/assert。
- 已通过 Result、Event 等明确错误通道向上传递的失败，不重复记录。
- 正常异步资源等待、正常 fallback、全局 observer 的正常目标过滤。
- press、release、click、hover、focus、拖动、选择变化、展开关闭等正常交互。
- `set_selected` 等正常程序化控件操作；业务操作日志由宿主调用处记录，内部异常仍按诊断规则记录。
- Pending 变化、正常层级构建、资源生成、缓存命中、样式同步等执行轨迹。
- 单纯 OS、GPU、DPI、窗口环境信息；只有造成实际能力降级时才记录。

已确认属于 Widgetry 的对象缺失必需内部结构，不能作为正常 observer 过滤静默忽略。
日志不能代替事件系统、profiler 或临时调试。

- 不得仅为了日志新增每帧遍历控件结构的通用诊断 system；日志应附着于已有真实失败路径。
- 日志不得为了能够记录错误而改变原有 panic/assert/错误传播语义。

## Widgetry 库持续异常

由负责该行为的模块保存必要状态，不能依赖系统执行次数：

- 正常 → 异常：记录一次。
- 异常 → 同一异常：不重复。
- 异常 → 正常：仅在此前记录过异常时记录一次 INFO 恢复。
- 正常 → 再次异常：重新记录。

正常等待不等于已恢复，实体销毁不输出恢复。

## Gallery 日志

Gallery 只使用普通 Bevy `info!`、`warn!`、`error!` 宏，不固定 `target: "bevy_widgetry"`。
禁止使用 `bevy_widgetry_log` 或 `widgetry_*` 日志宏，也不得为 Gallery 引入该依赖。
`debug!`、`trace!` 不作为永久日志。

### 等级与责任

- `info!`：具有 Gallery 演示语义的离散操作及其必要结果，包括用户操作和运行期程序化改选。
- `warn!`：操作失败但 Gallery 仍可继续运行，或某项演示能力不可用、降级。
- `error!`：Gallery 自身 bug、内部不变量破坏或无法正常继续的失败。

出现相应失败路径时必须记录；不为了满足等级规则人为制造 ERROR 日志。

### 用户操作与结果

应记录按钮激活、页面切换、主题切换、ComboBox 选项变化、打开独立窗口、打开 MessageBox 及其结果、发起文件选择或保存及其完成或取消结果。
Button Demo 中激活本身就是演示语义，即使没有进一步修改应用状态也需要记录；禁用按钮不产生日志。
主题实际改变时记录，ComboBox 的有效用户选项变化记录；程序初始化保持静默。
程序化改选也必须由 Gallery 在 `set_selected` 等 API 的调用处使用 `info!` 记录，字段包含具体示例、目标实体与选项索引；不得冒充用户操作，也不得下沉到 Widgetry 库。
初始化调用由 Gallery 根据调用场景保持静默，不为日志增加库级静默 API、日志开关或初始化状态。
排队调用只能记录发起操作；在结果未确认前不得宣称改选成功，也不改变程序化选择不发送用户 `ValueChange` 的约定。

ComboBox 选中某项属于具有明确结果的离散语义操作，不属于高频输入；用户实际改选时必须记录 INFO，并用结构化字段标明具体示例和所选项。运行期程序化改选同样不属于初始化，应按上述规则记录。
类似的离散选择或确认操作也按此原则记录，不得仅因为它来自控件交互或值变化事件而省略日志。

记录操作语义，例如“打开独立窗口”“选择 ComboBox 示例项”，不记录“收到 Pointer<Click>”等底层输入事件。
以下高频或中间状态不记录：hover、pointer move、press / release 本身、普通 focus 变化、文本框逐字符输入及其他驱动控件内部状态的连续事件。
TextField 只有新增明确的 submit / confirm 等离散语义操作时才记录对应 INFO。

日志应附着在已有 Gallery 语义处理点，不得为日志新增轮询 system、全局输入监听或低层事件追踪。
Button / ComboBox 示例可以增加记录明确 Demo 操作的轻量事件处理。
不得因为日志改变现有异步流程或 UI 反馈行为。

## 共用消息和上下文

正文使用中文；动态信息采用英文名结构化字段，例如 `entity`、`path`、`error`、`width`、`height`。
正文说明发生了什么，字段提供定位所需数据。不得手工加入 `file`、`line`、`module_path`、`crate_name`；使用 tracing 自带 metadata，由宿主决定是否显示。
target 只承担过滤职责，Gallery 的终端和文件均隐藏 target。
