# 让 Tree 列表和展开按钮配色按变化更新

## 目标与范围

调整 `crates/tree/src/view.rs::update_colors` 和必要的 Model／投影失效连接，停止每帧重新解析所有 TreeView 与可见 Expander 的颜色。

## 修改设计

Model 的 Selection、展开／折叠、层级和业务数据变化可能影响所有引用它的 View；Hover、Pressed、Focus、有效 Disabled 和 View Override 则按实际所属 View 或节点处理。不能只监听 View 根组件，因为共享 Model 的真实变化并不要求该组件发生写入。

新可见节点、内部 ListView 行替换、Expander 创建与复用，都要使用当前主题和展开状态初始化。使用已维护的 StyleOwner 关系，把展开按钮交互和内部行变化传到负责组合配色的 TreeView；不要让被接管的 ListView／Button 各自使用默认 Theme 覆盖 Tree 颜色。

Selection 与展开继续由 Model 共享，RendererRegistry 仍按 Component 类型在 App 级注册。外部业务 Hierarchy／Component 的正常变化和静默 Repair 都要能触发正确投影，不能只订阅公开 API 发出的通知而漏掉这些路径。

只减少颜色解析和样式遍历，不停止必要的 Model 同步、懒加载状态推进或可见节点投影。配色变化不能导致业务 Node 重建。

## 实施约束

涉及编译和运行的验证使用现有 Bevy 0.20、Windows 64 位 MSVC／DX12 环境，不以跨平台适配或绕过目标限制代替验证。

改动 UI 时继续使用 BSN，并保持 Build、StyleOwners、Colors 与前景传播／UI 准备之间的有效顺序。通过受影响的 Gallery 场景进行 BRP 输入、ECS 状态查询和截图验证；持续交互的 60 FPS 目标不能只用 Headless 耗时证明。相关功能说明和已有 Integration Coverage 随实际改动同步，不重建测试体系。

保留现有错误传播边界：真实失败以 BevyError 的 Error 级别交给宿主，必要的中文诊断在能说明原因的位置记录；不靠主动 panic、吞掉错误或替换宿主 Handler 维持流程。

## 完成条件

覆盖共享 Model 多 View、展开／收起、新节点、滚动复用、业务 Component 更新、选择／清除、Focus、Hover／Pressed 移除、Disabled、Theme 和 Override 清除。禁用 View 仍反映共享 Model 更新，样式不改变选择或展开语义。

稳定树不重复扫描全部可见展开按钮做颜色计算，局部交互不扩散到无关 View。保留 view／rendering 与模型回归，验证当帧 Text／Icon 准备；性能测量固定可见规模并区分静态、局部输入和持续滚动。

## 预期产出

与共享 TreeModel 和动态投影一致的变化驱动组合配色。

## 与前后方案的关系

顺序上接 [方案 27：合并 Tree View 的重复结构诊断](27-合并-Tree-View-的重复结构诊断.md)。

完成后进入 [方案 29：补齐共享 TreeModel 的双 View 展示](29-补齐共享-TreeModel-的双-View-展示.md)。

承接 Tree 的归属与诊断调整，并保持现有 ListView 受管颜色入口；不改变共享 Selection 和展开状态。
