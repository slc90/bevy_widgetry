# 补齐 Table 共享 Model 的独立布局展示

## 目标与范围

在 Table 相关测试和 `gallery/src/pages/table.rs` 的当前组织中增加 Shared Model 双 View 场景，展示同一个 `WidgetryTableModel<DemoRow>` 下的不同 `WidgetryTableLayout`、列宽和样式。

## 修改设计

两侧共享 Row、Column 和 Projection 定义，继续使用同一个 App 级按 Cell／Header 值类型注册的 Renderer。差异来自 View 的 Layout、列宽和区域样式，不来自逐 View Column Schema 或逐 View Renderer；后两种能力明确不在本轮范围。

共享 Edit、Insert、Remove、Move 应同时更新两侧数据。各自的 Selection、Focused Cell、滚动和 Disabled 不被另一侧的操作覆盖，单侧列宽调整不写到另一侧 Layout。实际键盘 Focus 可以正常转移；Focused Cell 的独立导航位置不等于两个 View 同时占用 InputFocus。

移动保持 Row／Column Stable ID，删除后分别修复各自无效的 Selection／Focused Cell。自动 Repair 仍保持既有静默规则。列被删除、Header 失效或 View 禁用时，正在进行的 Resize 仍按原有终止／取消契约收尾，不能把手势写到另一个 ID。

Gallery 使用公开状态和列宽 API，保留真实输入路径、稳定 Name 和强类型中英文文案。不能通过直接覆盖私有运行时状态制造演示结果。

## 实施约束

涉及编译和运行的验证使用现有 Bevy 0.20、Windows 64 位 MSVC／DX12 环境，不以跨平台适配或绕过目标限制代替验证。

## 完成条件

验证不同初始宽度、单侧拖动改宽、双向滚动、独立选择与 Focused Cell、单侧禁用，以及共享数据 Edit／Insert／Remove／Move。检查删除修复和身份稳定，已有 Cell Renderer 和虚拟化回归保持成立。

BRP 实际操作两侧并观察布局与结果，页面离开后回收临时数据和 View。更新 Table Coverage 与 Gallery 说明，不把集中展示缺失夸大成此前完全没有相关测试。

## 预期产出

共享数据而保持独立布局和交互的 Table 双 View 展示，以及对应契约回归。

## 与前后方案的关系

顺序上接 [方案 30：让 Table 可见区域样式按变化更新](30-让-Table-可见区域样式按变化更新.md)。

完成后进入 [方案 32：让 Window 标题栏与系统按钮配色按变化更新](32-让-Window-标题栏与系统按钮配色按变化更新.md)。

在 Table 样式调整后加入独立的共享数据、独立布局展示，继续使用现有 API。
