# 14｜验证各页面按需生命周期与原有运行入口无回归

## 目标

在 Windows 64 位真实 Gallery 中验证完整状态切换、资源所有权、事件与基准测试约束，避免局部完成但主流程残留旧依赖。

## 范围

Gallery 应用的构建、UI/BRP 验证、导航场景及两个已有 benchmark 入口；本阶段不是修改控件 crate 的功能迭代。

## 预期产出

确认 11 页按需创建和退出释放、常驻 UI 不变、没有 Model/Tree 节点泄露、特殊窗口正确处理、原 benchmark 入口可用。

## 与前后方案的关系

全系列最后一份；以先前公共改造和每一个页面迁移全部完成为条件，不为它另建平行实现路线。

## 方案正文

### 构建与工程约束

项目目标环境固定 Windows x86_64、Bevy `=0.20.0`，运行与验证使用 PowerShell 7 (`pwsh`)；不为了当前不支持的 Linux/macOS/Web 加 fallback。实际施工应按照仓库 `AGENTS.md`、`rules/project-context.md` 以及受当前改动影响的 development/testing/gui-debugging/benchmark 等规则执行，并按仓库要求维护实施进度记录与独立 Code Review；本方案只定义检查条件，不在生成文档时执行这些动作。建议以 `cargo fmt --all -- --check`、`cargo check --workspace`、适用的 `cargo test -p widget_gallery`、`cargo run -p widget_gallery` 等验证编译与运行。需要可视化输入/截屏时沿用 BRP runtime 的现有方法，不自行绕过项目 GUI 验证规范。编译和运行的实测结果**当前均未知**，不能把“静态代码已检查”说成“已测试通过”。

### 状态与 Resource 生命周期验证

启动时应只创建 Gallery 主窗口、Camera、Header/主题 ComboBox、Sidebar、PageHost 及默认 Button 页面；之前未访问过的 ListView/ComboBox/Tree/Table/Waveform 的私有 Resource、Model、生成器均不应已在 World 中。Sidebar 仍只有 11 个真实演示入口，没有 Initializing 按钮；当前页重复点击应不重建，换页后旧 `GalleryPageContent` 即消失，每次只有一个当前页根。Theme Mode 改变时，Sidebar 边框和所有当前页面颜色仍同步更新，Header 的 ListModel 始终存在。

反复使用同样的切页顺序，并分别在退出前改变本页输入/选择/折叠/滚动/列宽/组件内容，再重新进入检查全部恢复原默认值。统计**页面自己拥有的**数据源实体，而不是统计全 World 同类 Widget Model：ListView 四个 Model（4/10,000/20/20 条）、ComboBox 四个 Model、Table 七个 Model 及各 Layout、Tree 四个 Model + 四个 ECS 节点根 + 10,000 File 节点及 Lazy Loading 12 个后续子节点、Waveform 的 Live/Replay Sources/Arc。退出后对应 Resource 缺失且 owned entities 不存活；外部提供的 Model 与 Header 的 Theme Model 不因页面退出而失效。可通过 BRP 实体数量及明确的 Name/Component 查询验证，比较“进入→退出→再次进入”稳定性，不能单纯看页面是否隐藏。

### 交互、事件和 deferred Scene 边界

重点检查 `color_showcase::refresh` 异步排队的 BSN content、Tree `on_tree_event` 在 Resource 缺失时是否安全、`load_children` 在页面销毁后是否不再创建节点、`ListView`/`ComboBox` Popup 与 Tab 焦点是否无遗留、Tooltip visible/candidate 随锚点销毁被清理、CheckBox Added 初始化是否每次执行、Table 内部七 Demo 导航是否仍有效。导航 UI Name（`ButtonNav`、`ListViewNav`、`WindowNav`、`WaveformNav` 等）及 `#XXXPage` 必要 Name 继续能被 BRP 定位。Bevy State 转移从旧页 `OnExit` 到新页 `OnEnter` 的 UI/Data 时序应有可验证的边界：不出现过渡期 source-missing 错误，也不回收不属于本页的数据。

### 原有特殊窗口、性能测量与验收

Window 示例页退出时验证 Independent Window/MessageBox 自行存活、FileDialog 通过 OrphanedDialog 在正确 Winit 阶段关闭；非模态与模态分别测。Waveform GUI benchmark 应在 `WaveformNav` 后等待其 `StressWaveform` 尺寸和状态就绪，再开始测量，保持 `GALLERY_WAVEFORM_BENCH_OUTPUT` 原有无包装布局、State 与 `WaveformDemoSystems::Status` 次序，以及 frames.csv、完成标记等输出协议。FileDialog benchmark 应通过 `WindowNav` 启动 Launcher，保持原 BRP 导航名称、`GALLERY_FILE_DIALOG_BENCH_OUTPUT` 和输出协议；不因新的按需构建让脚本与采样流程失败。性能方面只记录实际页面切换时是否有卡顿，不预先引入分帧/后台 Model 填充，也不宣称性能提高。

若任一验收条件未通过，仅修复实现本次 Gallery 生命周期所必需的局部缺口，不扩大至 unrelated Widget API、通用框架、跨平台支持或无关性能工程。所有代码变更仍由用户或实施者在仓库另行执行，此文档本身不修改真实项目。

**源代码/验证入口：** [gallery/main.rs](https://github.com/slc90/bevy_widgetry/blob/main/gallery/src/main.rs)、[gallery/gallery.rs](https://github.com/slc90/bevy_widgetry/blob/main/gallery/src/gallery.rs)、[waveform_benchmark.rs](https://github.com/slc90/bevy_widgetry/blob/main/gallery/src/waveform_benchmark.rs)、[file_dialog_benchmark.rs](https://github.com/slc90/bevy_widgetry/blob/main/gallery/src/file_dialog_benchmark.rs)、[AGENTS.md](https://github.com/slc90/bevy_widgetry/blob/main/AGENTS.md)。
