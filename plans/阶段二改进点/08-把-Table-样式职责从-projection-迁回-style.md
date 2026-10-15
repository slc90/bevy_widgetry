# 把 Table 样式职责从 projection 迁回 style

## 目标与范围

整理 `crates/table/src/projection.rs` 与 `style.rs`，迁移 `update_colors`、`style_shell`、主题刷新及其样式辅助逻辑。保持公开 API、虚拟化算法和交互语义不变。

## 修改设计

当前 projection 同时负责可见 Cell／Header、几何投影和颜色解析，颜色路径还会写入 Selected 投影以及 Node 的 padding／border。迁移不能只按函数名剪切，而漏掉这些实际输出。

`style.rs` 负责颜色和由 `WidgetryTableStyle` 决定的样式属性；投影模块继续负责范围、实体复用、Canvas 尺寸和位置。Selected 的可观察投影保持与真实 Selection 一致，职责调整后仍在原有有效时机完成。纯整理阶段不同时引入新的变化驱动算法，先保留行为和调度边界。

保留几何更新中保护 padding／border 的处理，避免每帧先清空样式再写回。内部访问 Runtime 或区域信息所需的接口保持最小可见性，不向外公开投影细节，也不修改 App 级 Renderer Registry。

## 实施约束

涉及编译和运行的验证使用现有 Bevy 0.20、Windows 64 位 MSVC／DX12 环境，不以跨平台适配或绕过目标限制代替验证。

## 完成条件

既有四区布局、两个方向滚动、Header 对齐、Cell／Header 复用、Theme、Disabled、Selection／Focus 和列宽调整测试保持成立。新生成内容仍在首个有效帧获得样式，没有 padding／border 抖动或 Selected 投影滞后。

本项的完成依据是行为不变的模块整理，不以减少耗时作为尚未实施的成果。

## 预期产出

投影与样式边界清楚、可供后续定向样式更新使用的 Table 模块组织。

## 与前后方案的关系

顺序上接 [方案 07：按职责拆分 TextField 的 Scene、输入与样式](07-按职责拆分-TextField-的-Scene、输入与样式.md)。

完成后进入 [方案 09：清理库入口和 Gallery 入口的重复功能说明](09-清理库入口和-Gallery-入口的重复功能说明.md)。

为后面的 Table 样式变化驱动提供职责清楚的模块，但本项不包含该性能改动。
