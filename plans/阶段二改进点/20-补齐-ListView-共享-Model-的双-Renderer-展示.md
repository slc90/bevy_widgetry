# 补齐 ListView 共享 Model 的双 Renderer 展示

## 目标与范围

扩展 `crates/list_view/tests/behavior.rs` 等相关回归，在 `gallery/src/pages/list_view.rs` 的当前模块组织中增加并排 Shared Model 场景。沿用已有 `WidgetryListModel<T>` 和逐 View Renderer API。

## 修改设计

两侧使用同一个 Model，但必须配置不同的逐 View Renderer：A 仅文字，B 图标加文字。图标使用已有语义素材接口；不为示例把 Gallery 专属素材搬进库，也不硬编码内建资源路径。

共享操作包括 Edit、Insert、Remove、Move。两侧对同一 Stable ID 反映数据变化，同时各自保持 Selection、Active、滚动与 Disabled 边界。一个 View 的用户选择不能自动替另一侧选择；删除后分别按各自既有规则修复无效 Selection／Active，自动 Repair 不额外制造通知。

“焦点独立”不能解释成同一输入上下文中两个 View 同时获得实际键盘焦点。InputFocus 仍按正常交互转移；需要保护的是另一侧的选择、导航位置和数据状态不因共享 Model 被同步覆盖。Disabled View 不消费用户输入，但仍显示共享 Model 的程序化更新。

现有 Coverage 已说明共享 Source 的基础隔离测试存在。因此只补不同 Renderer、组合变更和缺失的状态边界，不重建一套重复基础用例。Gallery 使用公开 API 和实际输入驱动，新增文案接入当前强类型中英文 Key，并给交互目标稳定 Name。

## 实施约束

涉及编译和运行的验证使用现有 Bevy 0.20、Windows 64 位 MSVC／DX12 环境，不以跨平台适配或绕过目标限制代替验证。

## 完成条件

分别操作两侧、单侧禁用、分别滚动，再执行共享数据 Edit／Insert／Remove／Move，检查内容同步和状态隔离。通知 Source 始终是实际发生选择的 View；稳定身份不因移动或 Renderer 差异变化。

测试使用真实 Picking／Keyboard 路径保护相关行为；Gallery 用 BRP 验证并排操作、滚动、语言切换及离开页面后的清理。Gallery 本身不被强制新增单元测试。更新 ListView Coverage 和功能／展示说明。

## 预期产出

一个可交互的双 Renderer 共享数据示例，以及不重复已有基础覆盖的新增回归。

## 与前后方案的关系

顺序上接 [方案 19：让 ListView 容器和可见行配色按变化更新](19-让-ListView-容器和可见行配色按变化更新.md)。

完成后进入 [方案 21：让 ComboBox 的样式归属只在必要时建立](21-让-ComboBox-的样式归属只在必要时建立.md)。

在 ListView 样式调整之后补齐独立的共享数据演示能力，使用现有逐 View Renderer，不引入新状态语义。
