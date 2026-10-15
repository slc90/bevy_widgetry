# 合并 Tree View 的重复结构诊断

## 目标与范围

调整 `crates/tree/src/registration.rs` 中专门的 Source／Hierarchy 检查，以及 `view.rs::validate_sources`、`validate_hierarchy` 的触发。保持必要错误诊断，不重写 TreeModel 的业务同步或 Renderer 算法。

## 现状与修改设计

当前在 PreUpdate 检查 Source 和 Hierarchy，在 PostUpdate 的列表 Reconcile 后再次检查 Hierarchy；业务选择投影本身还会验证 Source。正常静止树因此承担额外全量遍历，不能把这些重复检查当作持续诊断必需条件。

利用实际构造、Model／Source 失效、内部列表和 Expander 生命周期、相关层级变化确定受影响 View。复用已经必须执行的业务投影前提校验，避免为同一事实再组织一轮独立全量扫描。输入前必须成立的约束仍在输入前成立；投影新建结构则在完成构造的有效边界检查，不能把半成品 BSN 当作永久坏结构。

保留 Source、Selection Projection、Hierarchy 等诊断的实际语义，不用一个粗糙总开关吞掉不同错误。每个 View 独立记录持续故障，真正恢复后才记录恢复，销毁不等于恢复。共享 Model 失效时，全部相关 View 仍需得到正确诊断；一个 View 的局部结构错误不应伪造其他 View 的恢复。

上一项的生命周期信号可以复用，但不为本项建设通用全局校验框架或新的公共 API。必要的 Model 更新、选择投影与输入状态推进保持现状。

## 实施约束

涉及编译和运行的验证使用现有 Bevy 0.20、Windows 64 位 MSVC／DX12 环境，不以跨平台适配或绕过目标限制代替验证。

测量使用相同真实负载和既有 Benchmark 规范；内部工作量计数不成为公共 API。涉及可见行为时增加对应 Gallery 的 BRP 输入、ECS 状态与截图验证，原生 Window／Mouse 行为仍需 Windows 原生证据。

保留现有错误传播边界：真实失败以 BevyError 的 Error 级别交给宿主，必要的中文诊断在能说明原因的位置记录；不靠主动 panic、吞掉错误或替换宿主 Handler 维持流程。

## 完成条件

覆盖 Source 的 Model 组件移除／恢复、无效 Source、内部列表缺失、Expander 所属失效、BSN 创建时序、共享 Model 多 View 和销毁。真实错误仍按 Error 传播；持续同因异常不重复记录，恢复与再次失败可观察。

用工作量证据证明稳定 View 不再进行额外重复层级检查，同时保留动态投影当帧正确性。相关 contract、view、rendering 测试和 Coverage 同步，不把“删除检查”本身当作优化验收。

## 预期产出

与真实生命周期和业务前提一致的 Tree 结构诊断触发方式。

## 与前后方案的关系

顺序上接 [方案 26：让 Tree 的样式归属随构造和层级变化维护](26-让-Tree-的样式归属随构造和层级变化维护.md)。

完成后进入 [方案 28：让 Tree 列表和展开按钮配色按变化更新](28-让-Tree-列表和展开按钮配色按变化更新.md)。

可以复用 Tree Owner 维护中的生命周期信号；本项完成后，颜色更新仍按原有契约工作，后续再优化其触发。
