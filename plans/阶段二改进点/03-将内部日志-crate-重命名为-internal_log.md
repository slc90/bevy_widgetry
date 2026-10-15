# 将内部日志 crate 重命名为 internal_log

## 目标与范围

把 `crates/log/` 和 `bevy_widgetry_log` 统一改为 `crates/internal_log/`、`bevy_widgetry_internal_log`。同步 Workspace 成员、路径依赖、Rust 导入、锁文件及相关文档链接，使名称准确表达内部诊断适配层的职责。

## 现状与修改设计

现有 crate 只转发 `widgetry_info!`、`widgetry_warn!`、`widgetry_error!`，并统一日志 Target。`log` 的宽泛名称容易与通用日志库混淆；这是一项命名改进，不是功能故障。

重命名需要一次覆盖全部引用，不保留两套实现，也不增加长期兼容包。Macro 的名字、`$crate` 卫生性、结构化字段、调用位置与 `bevy_widgetry` 日志 Target 保持不变。仍由宿主安装 Subscriber，不能借重命名增加文件管理、过滤开关、业务日志状态或第二套去重设施。

`app_logging` 是独立的宿主输出配置能力，不随内部日志 crate 合并。Gallery 的应用日志继续使用官方 Bevy Macro。`docs/crates/log.md` 及架构索引改到与新目录一致的位置；源码清理不顺带重写错误处理策略。

## 实施约束

涉及编译和运行的验证使用现有 Bevy 0.20、Windows 64 位 MSVC／DX12 环境，不以跨平台适配或绕过目标限制代替验证。

## 完成条件

全局引用检查不再出现旧的依赖标识或旧目录链接；锁文件由正常 Cargo 解析同步。因本项跨越整个依赖图，在 Windows 目标下进行 Workspace 级编译和相关日志测试，验证 Macro 从不同依赖层调用时仍保留原始位置与 Target。不能仅以文本替换完成判断成功。

## 预期产出

唯一的内部日志 crate 名称和完整、可解析的依赖图，日志行为与对外 Macro 名字不变。

## 与前后方案的关系

顺序上接 [方案 02：为 FileDialog Runner 建立正式显示预算判定](02-为-FileDialog-Runner-建立正式显示预算判定.md)。

完成后进入 [方案 04：为 Facade 控件建立真正的可选编译边界](04-为-Facade-控件建立真正的可选编译边界.md)。

本项统一后续使用的内部日志名称，不改变诊断 Macro 或宿主日志的职责。
