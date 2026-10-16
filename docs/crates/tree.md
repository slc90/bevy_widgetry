# Tree

## 功能概述

提供以 ECS Entity 表示业务 Node 的层级 Tree Model 与 TreeView，支持展开、Selection、Lazy Loading 与可见行虚拟化。

## 当前设计思路

`WidgetryTreeModel` 根据 Entity Hierarchy 组织可见顺序，展开与 Selection Authority 位于 Model。`WidgetryTreeView` 只投影可见 Node，并借助按 Component 类型注册的 Renderer 构造内容；业务 Node 不依赖其 Row Entity 的生命周期。

Tree 自身 ListView 与 Expander 的样式归属在构造时绑定，BSN 延后挂接、部件重建、层级变化及归属标记移除时，在 StyleOwners 阶段局部补偿。稳定 View 不再逐帧为已有部件发现归属。Expander 归最近的实际 TreeView 管理，不以共享 Model 作为颜色 Owner；嵌套 TreeView 独立维护部件，业务 Renderer 创建的普通 Button 不被接管。部件脱离或标记失效后清理派生归属，后续真实挂接 / 修复信号可重新绑定，失效节点不进入永久重试队列。

## 主要功能

- Root 下的 `WidgetryTreeNode` 表达业务层级和当前可见 Depth。
- 展开、收起、切换及设置 / 清空 Selection，通过 `WidgetryTreeEvent` 通知。
- Expander Icon、缩进、行高和 Theme / Disabled 风格可配置。
- 行 Pointer Selection 与 Keyboard Navigation。
- Unknown → Loading → Loaded 形式的 Lazy Children State，`ChildrenRequested` 通知调用方加载。
- 对异构业务 Component 注册 Renderer；按 Viewport 构造可见行。

## 行为与状态契约

同一个 Model 被多个 TreeView 使用时，Selection 与展开状态共享；各 View 的 Scroll 与 Disabled 边界独立。Selection Identity 是业务 Node Entity，不是行 Index；展开 / 收起影响可见性，不能销毁业务 Node。程序化 State 改变先提交再通知，同值不通知。外部 Hierarchy / Component 变化需要更新 Model 与可见投影。

## 边界情况与错误处理

Invalid Root / Node、缺失 Renderer、非法行高或缩进需要报告。Lazy Loading 由调用方响应 `ChildrenRequested`，Model 不替调用方执行文件系统 / 网络获取。Loading 期间重复展开不应重复发相同请求；数据删除后要修复失效 Selection。

额外的 Source / Hierarchy 诊断响应构造、引用 Source 的 Model 插入 / 移除、内部 ListView / Expander 及相关层级变化，在正常输入前和 Reconcile / Renderer 构造完成后的边界检查受影响对象。稳定 View 不组织额外全量诊断扫描；选择投影仍执行业务所需的当前 Source 与内部列表前提校验。Source、选择投影、View 层级与各 Expander 分别维护故障状态，同因持续异常不重复记录，本地结构错误不影响其他有效 View 的诊断；实际恢复后记录 INFO，再次故障重新报告，销毁不产生恢复信息。每次真实失败仍以 `Severity::Error` 交给宿主；归属补偿与层级检查共享 Expander 的同一故障状态。

调用方响应 Lazy Children 请求、添加业务 Children 后，通过 `set_loaded` 完成该请求；每个可见业务 Node 应匹配已注册的 Component Renderer。

## 已知限制

- Renderer Registry 当前按 Component 类型注册，不支持任意逐 View 相同类型 Renderer。
- 展开与 Selection 不属于逐 View 的独立状态。
- Lazy Node 的数据获取与失败重试策略最终由宿主实现。

