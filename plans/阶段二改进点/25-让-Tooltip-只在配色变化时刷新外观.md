# 让 Tooltip 只在配色变化时刷新外观

## 目标与范围

调整 `crates/tooltip/src/style.rs::update_tooltip_colors`，只优化 Popup 的 Background、Border 和 Foreground 同步。

## 修改设计

以新 Popup、所属 Anchor 的覆盖色变化、Theme 变化和相关绑定变化触发配色。Popup 首次构造时即使用当前有效颜色，动态富内容的 Foreground 仍按现有管理规则传播。

保留 Content Factory 每次显示返回新 BSN SceneList 的能力；这是 Tooltip 按次构造内容的现有契约，不因为“避免 Builder API”就改成持久保存一份 UI 或删除工厂接口。正常 Hover 持续期间不重复构造。

Hover 检测和真实时间推进继续运行：Cold 200 ms、Warm 50 ms、离开后的 300 ms Warm 窗口保持原值。保留全 App 单个 Tooltip、目标或 Window 变化、Cancel、失效清理和候选位置选择。Disabled 目标仍然能够显示提示，不引入额外的禁用隐藏或变灰行为。

## 实施约束

涉及编译和运行的验证使用现有 Bevy 0.20、Windows 64 位 MSVC／DX12 环境，不以跨平台适配或绕过目标限制代替验证。

改动 UI 时继续使用 BSN，并保持 Build、StyleOwners、Colors 与前景传播／UI 准备之间的有效顺序。通过受影响的 Gallery 场景进行 BRP 输入、ECS 状态查询和截图验证；持续交互的 60 FPS 目标不能只用 Headless 耗时证明。相关功能说明和已有 Integration Coverage 随实际改动同步，不重建测试体系。

保留现有错误传播边界：真实失败以 BevyError 的 Error 级别交给宿主，必要的中文诊断在能说明原因的位置记录；不靠主动 panic、吞掉错误或替换宿主 Handler 维持流程。

## 完成条件

覆盖已显示时切主题和覆盖色、隐藏后重新显示、新 Popup 富内容首次准备、快速切换 Anchor、Disabled 目标，以及 Owner／Window／Pointer 失效。新 Popup 的 Picking IGNORE 仍在首个有效帧生效。

在延时和显示保持不变的前提下，稳定 Tooltip 不反复执行相同颜色解析。相关计时测试使用可控真实时间；Gallery BRP 验证可见提示与交互不被阻挡。不能用减少必要 Hover 检查冒充配色优化。

## 预期产出

颜色变化驱动、但保留原有展示时序和内容生命周期的 Tooltip 样式更新。

## 与前后方案的关系

顺序上接 [方案 24：让 RadioGroup 选项样式真正由变化驱动](24-让-RadioGroup-选项样式真正由变化驱动.md)。

完成后进入 [方案 26：让 Tree 的样式归属随构造和层级变化维护](26-让-Tree-的样式归属随构造和层级变化维护.md)。

与相邻增量的联系主要是整体实施顺序，不虚构额外的技术依赖；本项可以在自身范围内完成和验收。
