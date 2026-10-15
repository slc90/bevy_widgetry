# 让 Table 可见区域样式按变化更新

## 目标与范围

在样式职责迁移后的 `crates/table/src/style.rs` 中优化 `update_colors<T>` 及相关区域更新。覆盖 Table Root、Corner、Cell、Column Header 与 Row Header，不改变二维虚拟化和 Renderer 注册方式。

## 修改设计

当前每帧克隆 Runtime／Style 并遍历全部可见 Cell 和 Header。没有相关变化时，应在进入这些收集、克隆和配色工作前停止；不能只在写入端保留 `set_if_neq`。

新 Shell、内容版本替换、Theme、Override、有效 Disabled、Hover、Selection、Focused Cell、实际 InputFocus，以及现有样式配置变化构成失效来源。选择 Row 或 Column 会影响一组可见 Cell，不能只更新被点击的那个 Entity；旧、新选中区域和焦点区域都要处理。保留当前已有视觉规则，不新创未定义的 Pressed 配色。

Style 的 padding／border 与纯投影位置分别维护。列宽和滚动改变不必然要求重算全部颜色，但由此生成或复用的 Shell 必须初始化。仍保证 Selected 投影、Foreground 和区域边框在正确阶段完成，不因减少扫描漏掉第一次显示的单元格。

各 View 的样式与布局状态独立；共享 Model 的内容变化只通过真实 Projection／Revision 影响相关实例。不能让一个 View 的缓存、Dirty 标记或 Runtime Entity 集合被另一 View 使用。

## 实施约束

涉及编译和运行的验证使用现有 Bevy 0.20、Windows 64 位 MSVC／DX12 环境，不以跨平台适配或绕过目标限制代替验证。

改动 UI 时继续使用 BSN，并保持 Build、StyleOwners、Colors 与前景传播／UI 准备之间的有效顺序。通过受影响的 Gallery 场景进行 BRP 输入、ECS 状态查询和截图验证；持续交互的 60 FPS 目标不能只用 Headless 耗时证明。相关功能说明和已有 Integration Coverage 随实际改动同步，不重建测试体系。

保留现有错误传播边界：真实失败以 BevyError 的 Error 级别交给宿主，必要的中文诊断在能说明原因的位置记录；不靠主动 panic、吞掉错误或替换宿主 Handler 维持流程。

## 完成条件

覆盖两个方向滚动、Header 对齐、新建和复用 Cell／Header、Row／Column／Cell Selection、清空、Focus 转移、Disabled 添加／移除、Theme、Override 清除及 Style 配置改变。列宽调整、取消和稳定 ID 的既有回归保持成立。

稳定大表格不再逐帧克隆和扫描全部可见区域进行配色。性能测量保持 Row／Column 总量、可见矩形和 Renderer 工作一致；区分无变化、局部 Hover、整行／整列选择和滚动，不用平均耗时掩盖长帧。同步相关 Coverage。

## 预期产出

在二维虚拟化和多区域状态下仍正确的 Table 变化驱动样式。

## 与前后方案的关系

顺序上接 [方案 29：补齐共享 TreeModel 的双 View 展示](29-补齐共享-TreeModel-的双-View-展示.md)。

完成后进入 [方案 31：补齐 Table 共享 Model 的独立布局展示](31-补齐-Table-共享-Model-的独立布局展示.md)。

依赖前面已完成的 Table 样式模块拆分；本项只改变触发和工作范围，不附带逐 View Schema 或 Renderer。
