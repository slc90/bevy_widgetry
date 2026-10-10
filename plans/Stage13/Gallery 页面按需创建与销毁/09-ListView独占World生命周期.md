# 09｜按已确认的独占 World 流程迁移 ListView

## 目标

以 ListView 作为有数据源页面的参考实现，确定进入/退出顺序且不改变现有 10,000 条数据的同步构建行为。

## 范围

`gallery/src/pages/list_view.rs` 的 DemoSources、四种 Model、UI Scene、系统与颜色示例。

## 预期产出

ListView 每次访问新建四套模型与 UI，完全离开后没有遗留 DemoSources 或拥有的数据源实体；行为和初态与当前一致。

## 与前后方案的关系

依赖第 02 份统一承载协议；它与 ComboBox 无共享 DemoSources 依赖，可在唯一规定顺序上随后实施。

## 方案正文

### 当前演示与数据规模

`ListViewDemoPlugin::build()` 当前创建四个 `WidgetryListModel<DemoItem>`：kind 0 的 Small List 4 项、kind 1 的 Virtualized Large List **10,000 项**、kind 2 的 Disabled ListView 20 项、kind 3 的 Disabled ListViewItem 20 项（1/3 项禁用）。`DemoSources([Entity; 4])` 中的 `Entity` 仅是 Model 实体 ID，不是 Model 自身；所有四个 Model 均由该演示创建，没有外部借用源。`scene(sources)` 中的 2x2 Panel、`section()`、`operate()`、`record_change()` 和 `update_status()` 需要沿用。

### 进入页：严格、同步的独占 World 顺序

`ListViewDemoPlugin::build()` 保留 `WidgetryListViewPlugin` / `register_widgetry_list_view::<DemoItem>()` 等类型注册，只注册系统，不再触碰四个 Model 的实际构建。`OnEnter(GalleryPage::ListView)` 由一个 `fn enter_list_view(world: &mut World) -> Result` 形式的系统承载：同步分别执行原 `populate()`，spawn 四个 `WidgetryListModel` ECS 实体 → 插入 `DemoSources` → 将原 `pages::list_view(sources)` 包入公共 `page(ListView,...)` 外壳并同步挂到 `PageHost`。页面根须可由 Gallery 识别，用于明确退出销毁。BSN 需要数据 Entity 时源已经准备好，不依赖多个系统之间隐式的 Commands flush。

既然当前 Gallery 运行中 10,000 项初始化没有可感知卡顿，本次 **不**改为分帧、后台线程或异步加载；“Clear + repopulate”仍使用现有同步 `populate()`，不借重构之机改交互行为。首次页面切入/退出可能造成短暂主线程等待，但是否有实测卡顿须运行新 Gallery 后判断，不能提前宣称优化或卡死。

### 退出页：同步 World 明确顺序

`OnExit(GalleryPage::ListView)` 使用唯一独占 `World` 系统：定位并 `World::despawn()` **ListView 页 UI 根**，让四个 View、虚拟行及 ColorShowcase UI 同步消失；再从页面 `DemoSources` 取得四个被页面拥有的 Model Entity，并各自 `World::despawn()`；**最后**移除 `DemoSources` Resource。不要简单先 `remove_resource` 再假定 Model 自动删除，也不要使用 `DespawnOnExit` 来与 `OnExit` 隐式竞争销毁顺序。

顺序理由有源代码依据：`crates/list_view/src/view.rs::validate_sources` 会检查 `WidgetryListModel` 存活，`virtualization.rs::reconcile_root` 会从 Model 读取数据；若 View 尚存而 Model 已销毁，可能得到 source missing 或错误。`World::despawn()` 同步递归销毁 Children，可把 UI 销毁放在 Model 销毁之前，避免过渡状态。

### 更新系统、Observer 与异常路径

`initialize_disabled` 仍注册于 `Update`，但加 `run_if(in_state(GalleryPage::ListView))`；它的 `Added<WidgetryListView<DemoItem>>` 在新 Scene 第一次产生后重新执行。`update_status` 仍在 `PostUpdate.after(UiSystems::Layout)`，同样受当前页 State 门控。`operate`、`record_change` 是 UI 上的事件处理，不因为切页重复注册；正常触发时通过当前 View 的 source 查 Model。颜色示例 `list_view::color_examples` 会读取 `DemoSources[0]`，需保证只有当前页面 Resource 有效时可以被展开。

如果某个 `populate` 或 BSN Scene 应用失败，要报告 `BevyError` 并清理本次已成功创建的 Model、页面 Resource/部分 UI，不能留下无法从 Gallery 回收的独立 ECS 实体。

验证：反复 Button→ListView→Button→ListView；每次模型数量与 4/10,000/20/20 数据恢复，禁用/selection/scroll/changes 全部复位，离页没有 `DemoSources` 和四个 Model，颜色示例与主题切换正常。切换时实际观察帧停顿，不先引入性能优化。

**源代码：** [list_view.rs](https://github.com/slc90/bevy_widgetry/blob/main/gallery/src/pages/list_view.rs)、[crates/list_view/view.rs](https://github.com/slc90/bevy_widgetry/blob/main/crates/list_view/src/view.rs)、[virtualization.rs](https://github.com/slc90/bevy_widgetry/blob/main/crates/list_view/src/virtualization.rs)。
