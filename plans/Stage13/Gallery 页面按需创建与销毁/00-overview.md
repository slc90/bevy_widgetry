# Gallery 按需页面生命周期：拆分方案总览

## 整体目标

在 Bevy 0.20 的 Gallery 中，让 Sidebar 驱动 `GalleryPage` State；常驻外壳只创建一次，每次进入页面重新初始化、退出时释放页面拥有的 UI/Model/Resource，保留外部实体及独立窗口既有生命周期。实施边界限定 Gallery 与启用 State 所需的 Workspace Bevy feature。

完整背景、全局直接依赖和详细实质正文见 [master-plan.md](master-plan.md)。总览仅导航，执行时以对应小方案正文为准。

## 唯一线性执行顺序

01. [建立 Gallery Page State 与常驻外壳](01-Gallery状态与常驻外壳.md)：让 Gallery 主窗口、Sidebar、主题栏和 PageHost 只构建一次，同时引入用于页面切换的 Bevy State。
02. [建立统一的页面切换、挂载和退出协议](02-统一页面切换与承载协议.md)：将 Gallery 导航从切换隐藏节点改为 State 驱动，并确定所有页面共享的 BSN 容器和资源释放契约。
03. [将 Button 默认页接入按需生命周期](03-Button默认页.md)：让首次进入 Gallery 的 Button 页面由 `OnEnter(Button)` 单独创建，并在离开时完全释放页面 UI。
04. [将 CheckBox 页接入按需生命周期](04-CheckBox页面.md)：只在进入 CheckBox 时创建复选框 UI，并维持其初始化状态和事件行为。
05. [将 ScrollArea 页接入按需生命周期](05-ScrollArea页面.md)：仅在访问 ScrollArea 时创建滚动示例，使其各项滚动位置和演示布局在重进时复位。
06. [将 TextField 页接入按需生命周期](06-TextField页面.md)：按需创建 TextField 演示并确保每次重新访问时输入内容和焦点相关页面态重置。
07. [将 Tooltip 页接入按需生命周期](07-Tooltip页面.md)：使 Tooltip 演示页的锚点与悬停 UI 随页面进入/退出，同时保留控件自身的应用级指针状态处理。
08. [将 ComboBox 演示 Model 改为页面私有数据](08-ComboBox演示数据生命周期.md)：让 ComboBox 页访问时才创建四个示例 ListModel，并在离开时回收其资源，同时保持 Theme Header ComboBox 独立。
09. [按已确认的独占 World 流程迁移 ListView](09-ListView独占World生命周期.md)：以 ListView 作为有数据源页面的参考实现，确定进入/退出顺序且不改变现有 10,000 条数据的同步构建行为。
10. [迁移 Tree 的 Model 与 ECS 节点子树生命周期](10-Tree演示完整实体树清理.md)：确保 Tree 页面创建的四个 Model 和全部节点实体，包括延迟生成的节点，均随页面销毁。
11. [迁移 Table 的七个 Model 与页面布局状态](11-Table演示数据生命周期.md)：让 Table 的七个 Demo Model 和对应布局状态只随 Table 示例页创建，并在离开时释放。
12. [将 Waveform 生产者与页面状态按需创建并兼容基准测试](12-Waveform资源与测量生命周期.md)：让 Waveform 的 Live/Replay 生产者仅在页面激活期间运行和占用对应 Resource，同时不破坏现有测量系统。
13. [迁移 Window 示例页并保留三类窗口的现有退出语义](13-Window页面与对话框生命周期.md)：使 Gallery 中 Window 示例页自身随页面 State 销毁，同时保留 Independent Window、MessageBox 与 FileDialog 不同的真实窗口生命周期。
14. [验证各页面按需生命周期与原有运行入口无回归](14-Gallery状态生命周期终验.md)：在 Windows 64 位真实 Gallery 中验证完整状态切换、资源所有权、事件与基准测试约束，避免局部完成但主流程残留旧依赖。

## 顺序说明

01—02 是共享基础，必须在具体页面生命周期改造前形成稳定的状态、宿主、导航和页面容器契约。03 首先接入默认 Button；04—07 处理 UI-only 页面；08—11 处理 ECS Model 数据源页面；12 处理 Waveform Resource/benchmark；13 处理 Window 对话框生命周期；14 负责完整回归。03—13 各页面没有共享 DemoSources 的地方仅按集成顺序排列，不暗示虚构的数据依赖。

整个序列不包含 `crates/` 控件实现改动、通用生命周期框架、异步/分帧加载或不相关性能优化。
