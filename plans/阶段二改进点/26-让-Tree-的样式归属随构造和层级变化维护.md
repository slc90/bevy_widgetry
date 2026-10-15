# 让 Tree 的样式归属随构造和层级变化维护

## 目标与范围

调整 `crates/tree/src/view.rs::establish_style_owners`、`own_list`、`own_expander` 和相关注册，消除静止 Tree 对全部内部 ListView 与 Expander 的永久所有权扫描。

## 修改设计

当前每帧扫描全部 TreeView，再扫描全部 Expander 并查找祖先；已有 Add Observer 仍需要 BSN 挂接时序的补偿。将新 View、内部列表、Expander 创建和合法重建纳入局部待处理集合，在 StyleOwners 阶段完成归属，正常就绪后不再重走祖先链。

内部列表属于实际 TreeView；Expander 的 Button Owner 必须是它当前的 TreeView，而不是共享 TreeModel 或碰巧找到的外层树。新的虚拟化行、折叠后再展开、Renderer 子树替换都应触发正确维护。只管理控件自身的列表与展开按钮，不接管业务 Renderer 任意创建的 Button。

需要补偿 Add 早于 ChildOf 的情况，同时清理销毁或失效 Entity 的记录。不得为了省扫描只依赖一次 Observer，也不得把永远失败的节点放进永久逐帧遍历队列。保留现有错误报告和必要恢复，外部破坏内部结构仍是错误，不成为新公开能力。

## 实施约束

涉及编译和运行的验证使用现有 Bevy 0.20、Windows 64 位 MSVC／DX12 环境，不以跨平台适配或绕过目标限制代替验证。

测量使用相同真实负载和既有 Benchmark 规范；内部工作量计数不成为公共 API。涉及可见行为时增加对应 Gallery 的 BRP 输入、ECS 状态与截图验证，原生 Window／Mouse 行为仍需 Windows 原生证据。

保留现有错误传播边界：真实失败以 BevyError 的 Error 级别交给宿主，必要的中文诊断在能说明原因的位置记录；不靠主动 panic、吞掉错误或替换宿主 Handler 维持流程。

## 完成条件

覆盖多个 View 共享 Model、不同 Model、延后挂接、新建可见行、折叠／展开、滚动复用、子树替换和 View 销毁。Owner 在颜色消费前就绪，不串到另一 View，也不误覆盖业务内容的自主样式。

静止 Tree 不再为建立已有归属扫描所有 Expander；定向工作量测试和相应 Benchmark 能区分创建成本与稳定成本。仍保留 TreeModel、Renderer Registry 和 ListView 公共接口。

## 预期产出

完整覆盖动态投影、没有永久层级兜底的 Tree StyleOwner 维护。

## 与前后方案的关系

顺序上接 [方案 25：让 Tooltip 只在配色变化时刷新外观](25-让-Tooltip-只在配色变化时刷新外观.md)。

完成后进入 [方案 27：合并 Tree View 的重复结构诊断](27-合并-Tree-View-的重复结构诊断.md)。

先整理 Tree 的 Owner 生命周期，为后续诊断与颜色更新提供可靠信号，不在这里改变 Model Authority。
