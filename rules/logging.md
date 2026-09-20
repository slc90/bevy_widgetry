# 日志规则

## 库与宿主的职责

Widgetry 只负责产生诊断 event：判断是否值得记录，选择 level 并提供 structured context。
subscriber、filter、RUST_LOG、格式、时间、ANSI、源码位置和文件输出由宿主 App 负责。
库不得初始化 subscriber，不得提供自有日志开关、level Resource 或日志配置系统。

## Widgetry 库统一入口

库生产代码必须使用底层 bevy_widgetry_log 的 widgetry_info!、widgetry_warn!、widgetry_error!。
macro 仅固定 `target: "bevy_widgetry"` 并转发 Bevy/tracing 语法，不维护配置、state 或去重。
禁止直接调用 Bevy 日志 macro 或用其他方式绕过统一 target。该 crate 不由 facade 导出。
统一 macro 不属于应用业务 API，Gallery 的入口规则见下文。

## Widgetry 库 level 与责任

- error：Widgetry 自身 bug、内部 invariant 或必需内部 state 被破坏、无法恢复的内部失败。
  如果这些错误最终导致 panic/assert，必须先记录 ERROR；panic 输出不能代替诊断 event。
- warn：程序仍运行，但出现被内部吸收且没有其他错误通道告知调用者的非预期外部失败，或实际功能不可用、关闭、降级。
- info：少量 lifecycle 事实，以及此前已记录的持续异常恢复正常。
- debug、trace：level 保留；当前不提供对应 macro，也不新增永久日志。

所有 Widgetry impl Plugin 必须在 build() 完成时记录一次 `<PluginName> 注册完成`。
不使用“初始化成功”，不记录版本号，不继续记录内部 Resource、system、observer、loader 或 cache 的注册细节。

## Widgetry 库不记录的情况

- 调用方非法参数、错误顺序或违反公开 API 前置条件，包括因此发生的 panic/assert。
- 已通过 Result、Event 等明确错误通道向上传递的失败，不重复记录。
- 正常异步 asset 等待、正常 fallback、全局 observer 的正常目标 filter。
- press、release、click、hover、focus、drag、selection 变化、展开关闭等正常交互。
- set_selected 等正常程序化 Widget 操作；业务操作日志由宿主调用处记录，内部异常仍按诊断规则记录。
- Pending 变化、正常 hierarchy 构建、asset 生成、cache hit、style 同步等执行轨迹。
- 单纯 OS、GPU、DPI、window 环境信息；只有造成实际能力降级时才记录。

已确认属于 Widgetry 的对象缺失必需内部结构，不能作为正常 observer filter 静默忽略。
日志不能代替 event system、profiler 或临时调试。

- 不得仅为了日志新增每帧遍历 Widget 结构的通用诊断 system；日志应附着于已有真实失败路径。
- 日志不得为了能够记录错误而改变原有 panic/assert/错误传播语义。

## Widgetry 库持续异常

由负责该行为的 module 保存必要 state，不能依赖 system 执行次数：

- 正常 → 异常：记录一次。
- 异常 → 同一异常：不重复。
- 异常 → 正常：仅在此前记录过异常时记录一次 INFO 恢复。
- 正常 → 再次异常：重新记录。

正常等待不等于已恢复，entity 销毁不输出恢复。

## Gallery 日志

Gallery 只使用普通 Bevy info!、warn!、error! macro，不固定 `target: "bevy_widgetry"`。

禁止使用 bevy_widgetry_log 或 widgetry_* 日志 macro，也不得为 Gallery 引入该依赖。

debug!、trace! 不作为永久日志。

### Level 与责任

* info!：具有明确 Gallery 演示语义的离散操作及其结果。
* warn!：操作失败但 Gallery 仍可继续运行，或某项演示能力不可用、关闭或降级。
* error!：Gallery 自身 bug、内部 invariant 破坏或无法正常继续的失败。

出现相应失败路径时必须记录；不得为了满足 level 规则人为制造日志。

### 操作与结果

Gallery 应记录具有明确演示语义的离散操作，例如页面切换、theme 切换、Widget 值确认、打开 window 或 dialog、发起外部操作以及这些操作的明确结果。
记录操作的业务语义，不记录底层输入机制。例如记录“打开独立窗口”，而不是“收到 Pointer\<Click>”。
运行期程序化操作如果具有明确演示意义，应在 Gallery 的调用处记录，并与用户操作区分。
程序初始化保持静默，不得为了区分初始化与运行期操作而给 Widgetry 库增加日志开关、静默参数或额外 state。
异步或排队操作只能记录当前已经确认发生的事实。在结果尚未确定前，不得提前记录成功结果。
以下连续或中间 state 默认不记录：

* hover、pointer move；
* press / release 等底层输入阶段；
* 普通 focus 变化；
* 文本逐字符输入；
* Widget 内部 state 同步、layout 更新及其他连续执行轨迹。

如果某个连续交互最终产生具有明确演示语义的确认、提交或选择结果，可以在该结果发生时记录一次。
日志应附着在已有 Gallery 语义处理点，不得为了日志新增 polling system、全局输入监听或低层 event tracing。
不得因为日志改变现有异步流程、Widget 行为或 UI 反馈。

## 共用消息和上下文

正文使用中文；动态信息采用英文名 structured field，例如 entity、path、error、width、height。
正文说明发生了什么，field 提供定位所需数据。不得手工加入 file、line、module_path、crate_name；使用 tracing 自带 metadata，由宿主决定是否显示。
target 只承担 filter 职责，Gallery 的终端和文件均隐藏 target。
