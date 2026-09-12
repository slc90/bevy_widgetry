# 日志规则

## 库与宿主的职责

Widgetry 只负责产生诊断事件：判断是否值得记录，选择等级并提供结构化上下文。
subscriber、过滤、`RUST_LOG`、格式、时间、ANSI、源码位置和文件输出由宿主 App 负责。
库不得初始化 subscriber，不得提供自有日志开关、等级 Resource 或日志配置系统。

## 统一入口

库生产代码必须使用底层 `bevy_widgetry_log` 的 `widgetry_info!`、`widgetry_warn!`、`widgetry_error!`。
宏仅固定 `target: "bevy_widgetry"` 并转发 Bevy/tracing 语法，不维护配置、状态或去重。
禁止直接调用 Bevy 日志宏或用其他方式绕过统一 target。该 crate 不由 facade 导出。
Gallery 是宿主，可使用正常 Bevy 日志 API；统一宏不属于应用业务 API。

## 等级与责任

- `error`：Widgetry 自身 bug、内部不变量或必需内部状态被破坏、无法恢复的内部失败。
  如果这些错误最终导致 panic/assert，必须先记录 ERROR；panic 输出不能代替诊断事件。
- `warn`：程序仍运行，但出现被内部吸收且没有其他错误通道告知调用者的非预期外部失败，或实际功能不可用、关闭、降级。
- `info`：少量生命周期事实，以及此前已记录的持续异常恢复正常。
- `debug`、`trace`：等级保留；当前不提供对应宏，也不新增永久日志。

所有 Widgetry `impl Plugin` 必须在 `build()` 完成时记录一次 `<PluginName> 注册完成`。
不使用“初始化成功”，不记录版本号，不继续记录内部 Resource、system、observer、loader 或 cache 的注册细节。

## 不记录的情况

- 调用方非法参数、错误顺序或违反公开 API 前置条件，包括因此发生的 panic/assert。
- 已通过 Result、Event 等明确错误通道向上传递的失败，不重复记录。
- 正常异步资源等待、正常 fallback、全局 observer 的正常目标过滤。
- press、release、click、hover、focus、拖动、选择变化、展开关闭等正常交互。
- Pending 变化、正常层级构建、资源生成、缓存命中、样式同步等执行轨迹。
- 单纯 OS、GPU、DPI、窗口环境信息；只有造成实际能力降级时才记录。

已确认属于 Widgetry 的对象缺失必需内部结构，不能作为正常 observer 过滤静默忽略。
日志不能代替事件系统、profiler 或临时调试。

## 持续异常

由负责该行为的模块保存必要状态，不能依赖系统执行次数：

- 正常 → 异常：记录一次。
- 异常 → 同一异常：不重复。
- 异常 → 正常：仅在此前记录过异常时记录一次 INFO 恢复。
- 正常 → 再次异常：重新记录。

正常等待不等于已恢复，实体销毁不输出恢复。

## 消息和上下文

正文使用中文；动态信息采用英文名结构化字段，例如 `entity`、`path`、`error`、`width`、`height`。
正文说明发生了什么，字段提供定位所需数据。不得手工加入 `file`、`line`、`module_path`、`crate_name`；使用 tracing 自带 metadata，由宿主决定是否显示。
target 只承担过滤职责，Gallery 的终端和文件均隐藏 target。
