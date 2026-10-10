# 01｜建立 Gallery Page State 与常驻外壳

## 目标

让 Gallery 主窗口、Sidebar、主题栏和 PageHost 只构建一次，同时引入用于页面切换的 Bevy State。

## 范围

`Cargo.toml`、`gallery/src/main.rs`、`gallery/src/gallery.rs` 的启动和应用级组织。只处理公共启动基础，不承担各页面具体数据的初始化。

## 预期产出

启动后存在唯一 Gallery 主窗口、Camera、常驻 Sidebar/主题栏及空 PageHost；初始 `Initializing` 能正常转入 Button State。

## 与前后方案的关系

全系列第一份方案，提供后续统一页面承载协议的宿主。第二份方案建立页面挂载/销毁能力，之后才接入单个演示页。

## 方案正文

### 当前问题与职责分离

目前 `gallery/src/main.rs::setup` 在 Startup 中使用 `Res<pages::ListViewDemoSources>`、`ComboBoxDemoSources`、`TreeDemoSources`、`TableDemoSources`、`WaveformDemoSources`，再将它们一次性交给 `gallery::scene(...)`。对应的页面数据在各 `DemoPlugin::build()` 提前建立。`gallery/src/gallery.rs::scene()` 则一次创建 Button 到 Waveform 共 11 个页面，靠 `Display::Flex/None` 选择显示，隐藏页的 Model/Arc 仍驻留。

新的职责是：`main.rs` 只搭建 Gallery 主窗口、Camera、标题与主题选择器；`gallery.rs` 只搭建 Sidebar 和空的 PageHost。Header 主题 ComboBox 使用 `WidgetryListModel<WidgetryThemeMode>`，这是**常驻应用资源**，不能与页面 Demo Model 混淆或被页面退出流程清理；`WidgetryThemeChanged` 与 Sidebar 主题边框刷新仍全局有效。Gallery 主窗口的 `widgetry_window` 和 UI Camera 不随页面状态销毁。

### Bevy 0.20 的 State 配置与启动时序

Workspace 的 Bevy 版本固定为 `=0.20.0`，根 `Cargo.toml` 采用 `default-features = false`，当前没有显式启用 `bevy_state`。在 Workspace 的统一 Bevy feature 列表启用 `bevy_state`（不要为各页面引入独立依赖或在多个 crate 重复维护 Bevy 版本）。由应用启动时安装的 `DefaultPlugins` 提供 State 调度机制，并为 `GalleryPage` 注册/初始化 States；避免重复安装 `StatesPlugin`。

把已有 GalleryPage 枚举用作 `States`，增加默认 `Initializing`（`#[default]`），其他 11 个枚举值及 Sidebar 导航意义不变。初始的 `OnEnter(Initializing)` **不能尝试创建页面**。Bevy 0.20 首次 `StateTransition` 发生在 `PreStartup/Startup` 之前：若默认值直接设成 Button，`OnEnter(Button)` 会先于 `PageHost` 的构造，这正是选用 `Initializing` 的原因。

`Startup::setup` 创建常驻外壳后调用 `NextState<GalleryPage>::set_if_different(GalleryPage::Button)`。Bevy 0.20 正确方法名是 `set_if_different`，不是 `set_if_neq`。常规状态切换于 `PreUpdate` 之后、`Update` 之前处理；首帧 Startup 完成之后亦会执行，因此首次真正的 `OnEnter(Button)` 能查找到已经创建的 PageHost。由于外壳采用 BSN `Commands` deferred apply，必须确认其在 Startup 结束时已完成构建，找不到 Host 时应报告错误，不能静默创建游离页面。

### 常驻外壳与识别

保留 Sidebar 现有导航按钮的 Name，例如 `ButtonNav`、`ListViewNav`、`WindowNav`、`WaveformNav`；给 `#PageHost` 挂上可查询的 `GalleryPageHost` Component，避免依赖 BSN 内部标签做运行时查询。`PageHost` 初始不创建任何页面子树。不因切换页面重建 Header/theme 选择器、Gallery 主窗口、Sidebar，也不更换 Camera。

**源代码依据：** [Cargo.toml](https://github.com/slc90/bevy_widgetry/blob/main/Cargo.toml)、[gallery/src/main.rs](https://github.com/slc90/bevy_widgetry/blob/main/gallery/src/main.rs)、[gallery/src/gallery.rs](https://github.com/slc90/bevy_widgetry/blob/main/gallery/src/gallery.rs)。

### 阶段验收边界

此阶段主要验证 State 初始化与常驻实体身份不变；因具体页面尚未迁入，短暂存在空 PageHost 是过渡状态，不把它当作完整 Gallery 的最终行为。
