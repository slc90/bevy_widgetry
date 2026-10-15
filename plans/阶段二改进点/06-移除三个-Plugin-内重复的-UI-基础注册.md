# 移除三个 Plugin 内重复的 UI 基础注册

## 目标与范围

删除 CheckBox、RadioGroup、Tooltip 的同一个 `build()` 中重复出现的 `WidgetryUiPlugin` 条件注册。对应位置为 `crates/check_box/src/lib.rs`、`crates/radio_group/src/lib.rs`、`crates/tooltip/src/style.rs`。

## 修改设计

当前三个位置都同时存在完整路径写法和已导入名称写法的相同检查。保留每个 Plugin 一处必要注册及正确的先后关系，删除确实重复的另一处，不把“上层已经可能安装过”作为删除其他必要依赖的理由。

CheckBox 的官方 CheckboxPlugin、三态输入与状态通知，RadioGroup 的官方 RadioGroupPlugin、Tab Navigation、主题能力，以及 Tooltip 的 Hover 和 Popover 能力都保持原状。安装幂等性和成功时一次注册完成日志不能改变。

本项不共享或重构 CheckBox／TriStateCheckBox Scene，不增加初始化 API，也不调整对外 Children 结构。它只是消除重复注册位置。

## 实施约束

涉及编译和运行的验证使用现有 Bevy 0.20、Windows 64 位 MSVC／DX12 环境，不以跨平台适配或绕过目标限制代替验证。

## 完成条件

分别单独安装三个 Plugin，并覆盖组合安装和已有基础 Plugin 的情况。现有初始化、真实输入、Disabled 和 Tooltip 展示测试保持通过；每个 Plugin 的成功注册事实不会重复记录。纯整理不需要制造新的失败测试。

## 预期产出

职责清楚且保持原有初始化保障的三个 Plugin 构建路径。

## 与前后方案的关系

顺序上接 [方案 05：通过可选 app_logging Feature 公开宿主日志能力](05-通过可选-app_logging-Feature-公开宿主日志能力.md)。

完成后进入 [方案 07：按职责拆分 TextField 的 Scene、输入与样式](07-按职责拆分-TextField-的-Scene、输入与样式.md)。

与相邻增量的联系主要是整体实施顺序，不虚构额外的技术依赖；本项可以在自身范围内完成和验收。
