# 10｜迁移 Tree 的 Model 与 ECS 节点子树生命周期

## 目标

确保 Tree 页面创建的四个 Model 和全部节点实体，包括延迟生成的节点，均随页面销毁。

## 范围

`gallery/src/pages/tree.rs` 的 sources、lazy Loading、WidgetryTreeEvent Observer、UI 状态更新。

## 预期产出

仅 Tree 当前页持有其 Model、节点根及后续动态子节点；离开时完整回收，重进显示初始树与加载状态。

## 与前后方案的关系

建立在已验证的 UI→数据→Resource 顺序之上；与相邻 ListView 页面没有跨页数据共用关系。

## 方案正文

### 为什么不只删四个 Model

`tree.rs::sources(world)` 对四类演示各创建一个**数据节点根 Entity**，在该根下通过 `ChildOf(root)` 建立 Basic、ECS Heterogeneous、Lazy Loading、Virtualized 四棵实体子树，最后分别 `world.spawn(WidgetryTreeModel::new(root))` 生成四个 Model 实体。Virtualized 树在 Folder 下创建 **10,000 个 File 节点**。这些节点 Entity **不是**四个 WidgetryTreeModel ECS 实体的 UI 子节点，只删除 `DemoSources([ModelEntity;4])` 指向的 Model，会遗留真实数据节点。

`load_children` 在异步模拟的 Lazy Loading 阶段为 Remote Folder 添加 `LoadDeadline`，时间到后还会用 `ChildOf(node)` 新增 12 个 File 节点。由于它们挂在原数据树子树下，销毁对应节点根可以递归回收全部后续创建的节点。Tree 页面专属 Resource 需要明确同时记录四个 `WidgetryTreeModel` Entity 及**其真正拥有的四个数据节点根**（可以扩展当前 DemoSources 或用等价页面 Resource，不另外创建通用资源管理框架）。不得把这些根误判为外部数据源；若未来传入非页面创建的源，则仅解除引用而不能销毁。

### 生命周期与注册

`TreeDemoPlugin::build()` 只执行 `register_renderer::<BasicNode/Folder/File>`，注册 `on_tree_event` 及系统，不提前调用 `sources(app.world_mut())`。`OnEnter(Tree)` 同步创建四个数据根和子节点，再创建 Model 并记录全部拥有的 Entity，插入页面 Resource，然后把 `pages::tree(sources)` 通过公共页壳挂至 PageHost。按已商定的原则保留同步初始化，即使 Virtualized 树 10,000 节点有潜在耗时，也等 Gallery 运行时实测再决定是否优化。

`OnExit(Tree)` 同步**先**销毁 Tree 页的 UI 根与展开的 ColorShowcase，**再**销毁四个 Model 及四个数据树根（递归回收 10,000 节点和 Lazy Loading 后来的子节点），**最后**移除页面 Resource。`LoadDeadline` 随节点销毁，不可以让前一次访问的请求在重新进入时“续载”。`load_children` 中 deferred 回调原本会在 Folder 已消失时返回，需保留这一防护，并验证切页边界下无幽灵节点。

`load_children` 仍在 Update，但加 `run_if(in_state(GalleryPage::Tree))`；`update_status` 仍位于 `PostUpdate.after(WidgetryListViewSystems::Reconcile).before(UiSystems::Prepare)` 并受 State 门控。`on_tree_event` 是全局 Observer，当前签名用强制 `Res<DemoSources>`：**退出页后该 Resource 不存在**，若旧事件仍到达就会失败；采用 `Option<Res<DemoSources>>` 并在没有当前来源或 source 不属于本页面时直接返回，不能单靠 `run_if` 给 Observer 加约束。颜色示例读取第一个 Model，也只能在 Tree 当前页存在时构造。

验证：懒加载未完成时切页、已加载 12 个子节点后切页、展开 10,000 节点树后切页，均无遗留节点、Model 或旧加载；重新进入 Unknown→Loading→Loaded 顺序重置，Tree Event 不因为 Resource 暂时不存在而报错。

**源代码：** [tree.rs](https://github.com/slc90/bevy_widgetry/blob/main/gallery/src/pages/tree.rs)。
