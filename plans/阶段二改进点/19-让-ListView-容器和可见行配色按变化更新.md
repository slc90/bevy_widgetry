# 让 ListView 容器和可见行配色按变化更新

## 目标与范围

调整 `crates/list_view/src/style.rs::update<T>` 和必要的投影／状态变化连接，消除静止 View 对全部可见 Row 的重复颜色解析。

## 修改设计

容器颜色受有效 Disabled、Focus、Theme 和覆盖值影响；Row 还受 Hover、Pressed、Selected、Active 及业务 Item Disabled 影响。区分根级变化与局部行变化：全局配色或根状态变化才刷新该 View 的相关可见行，单行交互应尽量定位受影响的行。

首次构造、滚动产生的新可见行、实体复用后 Item ID 改变、Renderer 内容替换，都必须获得当前样式。不能仅用 Entity 没变、输出颜色没变或 Runtime 每帧写回的 Changed 标记推断无需初始化；身份变化与新增子内容是独立触发条件。

保留现有容器和 Row 的优先级，Active Border 与选中色的独立关系，以及 Focus 在两个 View 之间转移时旧、新目标的刷新。Row Shell 的样式不改写 Renderer 任意业务内容。owned 配色路径仍可由 ComboBox、Tree 等上层调用；内部行交互要能触达真正负责颜色的 Owner。

## 实施约束

涉及编译和运行的验证使用现有 Bevy 0.20、Windows 64 位 MSVC／DX12 环境，不以跨平台适配或绕过目标限制代替验证。

改动 UI 时继续使用 BSN，并保持 Build、StyleOwners、Colors 与前景传播／UI 准备之间的有效顺序。通过受影响的 Gallery 场景进行 BRP 输入、ECS 状态查询和截图验证；持续交互的 60 FPS 目标不能只用 Headless 耗时证明。相关功能说明和已有 Integration Coverage 随实际改动同步，不重建测试体系。

保留现有错误传播边界：真实失败以 BevyError 的 Error 级别交给宿主，必要的中文诊断在能说明原因的位置记录；不靠主动 panic、吞掉错误或替换宿主 Handler 维持流程。

## 完成条件

使用已有 style、behavior 和 virtualization 覆盖扩展回归：同一物理行换 ID、滚动回到旧数据、移除 Pressed／Disabled、清空 Selection、Active 迁移、Model Disabled 更新、主题／Override 改变和新内容当帧准备。保持单选 API、Stable ID、静默 Repair 与旧 Press 不作用于新条目的契约。

稳定 View 的颜色解析与层级遍历不再随每帧重复发生，且单行变化不会扩散到无关 View。性能验证固定数据量和可见行数，分别测静止、局部交互和持续滚动；不通过减少可见工作或只观察 `set_if_neq` 的写入次数证明提速。同步相关 Coverage 职责。

## 预期产出

覆盖虚拟化和复用生命周期的 ListView 变化驱动配色。

## 与前后方案的关系

顺序上接 [方案 18：让 ScrollArea 滚动条配色按变化更新](18-让-ScrollArea-滚动条配色按变化更新.md)。

完成后进入 [方案 20：补齐 ListView 共享 Model 的双 Renderer 展示](20-补齐-ListView-共享-Model-的双-Renderer-展示.md)。

按基础控件到数据视图的顺序推进，沿用既有 ScrollArea 接口；不要求增加新的滚动 API。
