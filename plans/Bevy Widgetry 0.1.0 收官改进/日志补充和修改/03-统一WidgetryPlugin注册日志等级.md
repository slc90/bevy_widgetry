# 统一 Widgetry Plugin 注册日志等级

## 目标与设计思路

将 Widgetry 库中 Plugin 注册完成这一正常生命周期 INFO 改为 DEBUG，使库记录自身实际执行阶段，而不再把 Plugin 注册误认为用户业务操作。消息的业务事实与发生时机不变，目标是统一等级，不是减少应有诊断或增加组件交互日志。

当前大量插件在 `build()` 完成时通过 `widgetry_info!` 输出 `"<PluginName> 注册完成"`。现有 `rules/logging.md` 已要求这条事实用 DEBUG 记录一次。涉及基础控件插件、`JobManagerPlugin`、`WidgetryTrayPlugin`、泛型 `TypedComboBoxPlugin<T>`、`TypedListViewPlugin<T,K>`、`TypedTablePlugin<T>` 等；实际实施需以当前 `crates/**/src` 中 Plugin 的 `build()` 及其日志调用为准，不能仅处理少数示例。

## 注册日志的统一行为

所有 Widgetry Plugin 在自己的 `build()` 确实完成后记录一次 `widgetry_debug!("<PluginName> 注册完成")`。保留已有原文，不替换成“初始化成功”，不额外打印 Resource、System、Observer、Cache、Loader 每个内部注册动作。对于泛型 Plugin，每个实际成功注册的泛型实例仍有自己的一条记录；同一实例被重复注册而没有重新执行 `build()` 时不得生成新的记录。

此变化不适用于 Gallery 的业务日志 Macro，也不适用于 `bevy_widgetry_app_logging` 自身。库诊断继续使用内部 Macro 和统一 `target: "bevy_widgetry"`，不得直接改成普通 Bevy `debug!`。不因等级变化修改 Plugin 注册流程、Feature 边界、注册先后或任何控件行为。

## 测试与文档

Plugin 的现有日志计数测试应继续通过，例如 `crates/combo_box/src/registration.rs`、`crates/list_view/src/registration.rs`、`crates/table/src/registration.rs` 中测试实际注册次数的用例；在必要位置增加 Level=DEBUG 和 Target 的断言。需要检查一次有效注册只输出一条、重复泛型注册不重复、日志位于成功执行 `build()` 的末端。用于捕获的 `LogCapture` 已支持记录 Level 和字段，不必新造工具。

校对确实受到 Plugin 注册日志 Level 变化影响的 crate 文档与注册测试；程序异常恢复的 INFO→DEBUG 调整归入 JobManager 的任务生命周期方案，避免跨目标混改。原有 ERROR 源头、处理 DEBUG、ECS 最终报告必须保持既有顺序和 ID，正常 Plugin 注册不应出现 INFO 或 WARN。

本增量不为各个 Widget 正常 Pointer、Focus、Drag、布局、Style 更新添加任何新日志；那类默认静默规则继续有效。
