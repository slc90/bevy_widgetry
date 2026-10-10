# 13｜迁移 Window 示例页并保留三类窗口的现有退出语义

## 目标

使 Gallery 中 Window 示例页自身随页面 State 销毁，同时保留 Independent Window、MessageBox 与 FileDialog 不同的真实窗口生命周期。

## 范围

`gallery/src/pages/window.rs`、`window/independent.rs`、`window/message_box.rs`、`window/file_dialog.rs` 及原有 FileDialog benchmark 调用。

## 预期产出

退出 Window 示例页不会关闭独立窗口和 MessageBox，但会通过已有 OrphanedDialog 机制关闭当前页打开的 FileDialog；重进是新示例页。

## 与前后方案的关系

依赖公共页面生命周期和 `PageHost`；之后的终验方案专门检查这些边界与 BRP/benchmark。

## 方案正文

### 区分页面与真实 OS Window

`GalleryPage::Window` 只是 Sidebar 中一个展示 `WindowPage` 的 `page_content`，它和 `widgetry_window` 的 Gallery **主窗口**是不同实体。`pages::window()` 内有 Independent Window、MessageBox、FileDialog 三个演示区域；`WindowDemoPlugin::build()` 注册 `WidgetryMessageBoxPlugin`、`FileDialogDemoPlugin` 及 `on_message_box_result`，这些注册属于应用生命周期，不能因为页面切走而移除。

`OnEnter(Window)` 只创建并挂载 Window 示例 UI 根。`OnExit(Window)` 在一个同步 `World` System 中销毁该**示例页 UI 根**；Gallery 主窗口、Camera、Header、Sidebar 不动，也不按原页面曾经打开过哪个窗口就粗暴遍历 Window 实体删除。

### 三种窗口的约定

**Independent Window**：`independent.rs::open_window` 通过 `commands.spawn_scene_with_error_handler(@owned_widgetry_window(...))` 创建新的真正 native Window/Widgetry 结构；它不是 Window 示例页的 ChildOf 子树。它的 Theme/Image Stretch/Cover、内容按钮和主题响应继续运行，Gallery 切去 Button 或 Table 不应把它关闭。

**MessageBox**：`message_box.rs::open_message_box` 使用 `@widgetry_message_box(primary_window,...)` 单独创建，结果通过全局 `on_message_box_result` 记录；页面切换后 MessageBox 保持自己的生命周期，按用户交互正常关闭，不回填到已销毁的页面状态。

**FileDialog**：控件在 `crates/file_dialog`，但 Gallery 层 `window/file_dialog.rs` 另有 `PageOwner`、`DialogRoute { owner, result, session }`、`DemoResult`、`OrphanedDialog`。当前 `owner_ended(On<Despawn<PageOwner>>)` 在示例页 Owner 销毁后给关联 FileDialog 标 `OrphanedDialog`，`cleanup_ended_owners` 在 `Last` 中链式应用 deferred、且在 `close_when_requested` 前清理 Native Window。**用户已明确同意保留该 Gallery 独有机制**：当 `OnExit(Window)` 同步销毁含 `PageOwner` 的页面 UI 时，此页发起且尚在的 FileDialog 要被自动关闭。其相关 Observer 和 Last 清理 System 必须全局继续注册，不被 `run_if(Window)` 禁掉。不要改动 `crates/file_dialog`，也不做 FileDialog 结果跨页缓存。

即：Independent Window 与 MessageBox 继续运行，FileDialog 是“切页仍自动关闭”的特例。三者都不归 `PageHost` 的普通页面子树管理；FileDialog 的关闭来自显式 Owner 销毁事件，而不是 Gallery 直接拥有 Dialog Root。

### 原有性能测量入口

`file_dialog_benchmark.rs` 的测量安装、`benchmark/file_dialog_trace` / `benchmark/file_dialog_export` 等 BRP 入口保持应用级。`gallery/benches/file_dialog_run.py` 会先通过 `WindowNav` 找到 Window 页，再按 `GalleryFileDialog{operation}Launcher`、`GalleryFileDialogModality`、`GalleryFileDialog{operation}Result` 等 Name 操作。迁入按需创建后必须维持这些 Name；允许导航后等待一帧或直到 Scene 建立，而不能因页面没提前存在就把点击解释为失败。`GALLERY_FILE_DIALOG_BENCH_OUTPUT` 仍触发公共无包装布局模式；FileDialog 仍按现有流程进行 result/session/fixture 验证。

### 验证

从 Window 页先各开一个 Independent Window、MessageBox 和非模态 FileDialog，然后导航离开：前两者仍可交互；FileDialog 被 OrphanedDialog 清理，并且没有错误 result 回填。返回 Window 页重新打开 FileDialog，应得到新的 PageOwner、Result 和 session，不与前一页的旧 Entity ID 串联；模态 FileDialog 与 benchmark 场景也应单独验证。

**源代码：** [window.rs](https://github.com/slc90/bevy_widgetry/blob/main/gallery/src/pages/window.rs)、[file_dialog.rs](https://github.com/slc90/bevy_widgetry/blob/main/gallery/src/pages/window/file_dialog.rs)、[file_dialog_benchmark.rs](https://github.com/slc90/bevy_widgetry/blob/main/gallery/src/file_dialog_benchmark.rs)。
