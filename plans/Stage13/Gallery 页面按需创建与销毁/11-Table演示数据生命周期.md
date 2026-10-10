# 11｜迁移 Table 的七个 Model 与页面布局状态

## 目标

让 Table 的七个 Demo Model 和对应布局状态只随 Table 示例页创建，并在离开时释放。

## 范围

`gallery/src/pages/table.rs` 的 TableDemoSources、renderers、七种演示与页面内部切换，含颜色示例。

## 预期产出

每次进入 Table 都重建七个 Model、初始 Column 布局及 UI；离页没有遗留 Table 数据源；保留原页面内部切换。

## 与前后方案的关系

依赖公共生命周期约定；相邻 Tree 与 Table 是独立迁移目标，不共享 Demo Model。

## 方案正文

`TableDemoSources` 当前包含 `sources: [Entity; 7]` 和七份 `WidgetryTableLayout`。`sources(world)` 构建 Basic、CellValue、Header、Selection、Column Layout、Virtualization、Disabled 七个 Model，通常 24 行，Virtualization 用 **2,000 行 × 40 列**的源数据结构，并配置 Fixed/Flexible 列宽。七个 `WidgetryTableModel` 都在 Gallery 的 Table Demo 内部 `world.spawn()`，归属于该页。

`TableDemoPlugin::build()` 中 `register_widgetry_table::<DemoRow>()`、Cell/ Header 各种 renderer 的注册要保持应用生命周期只执行一次；`WidgetryTableModel` 创建、`TableDemoSources` 插入移至 `OnEnter(Table)`。进入时先全部创建七个 Model，再保存 Resource，最后通过原 `pages::table(sources.clone())` 建立页面 Scene。`TableDemoSources` 中布局 clone 与场景参数的传递应维持现有设计，不变更 CellValue、图标、进度条、虚拟化行内容的 renderer 语义。

`on_table_event` 是全局 Observer，但当前通过 `Query<(&mut DemoEvents, &WidgetryTableLayout)>` 确认事件来源是现存 Table UI，Resource 不存在时不会强行读取。`update_status` 保留其在 `PostUpdate.after(UiSystems::Layout)` 的顺序，并添加 `run_if(in_state(GalleryPage::Table))`，不能在页面被销毁后用旧 UI 生成状态。`color_examples` 读取 TableDemoSources 及其中 Model Entity，只允许当页面存在时展开。

注意 `table.rs::change_demo` 在**Table 页内部**通过 `Display::Flex/None`、`TabIndex` 切换七种展示，这是页面自己演示的功能，**不等于 Gallery Sidebar 的跨页面导航**；保留这套逻辑，不擅自改成七个应用 State。页面退出时先同步销毁整个 Table UI（包括当前显示和隐藏的内部 Panel），再销毁七个 Model，最后移除 TableDemoSources，重进时内部默认选择、列宽调整和 Selection 计数都回到初态。

若七个 Model 构造中途失败（如 Model push 行列失败），负责回收已经 spawn 的独立数据源，避免半成品遗留。验收包括页面内切换七种 Demo、列宽调整后重进复位、2,000×40 虚拟滚动恢复、所有 Model 源退出时消失。

**源代码：** [table.rs](https://github.com/slc90/bevy_widgetry/blob/main/gallery/src/pages/table.rs)。
