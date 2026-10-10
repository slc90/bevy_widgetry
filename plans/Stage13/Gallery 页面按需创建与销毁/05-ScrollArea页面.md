# 05｜将 ScrollArea 页接入按需生命周期

## 目标

仅在访问 ScrollArea 时创建滚动示例，使其各项滚动位置和演示布局在重进时复位。

## 范围

`gallery/src/pages/scroll_area.rs` 中 `scene()`、可见内容切换、ScrollIntoView 事件。

## 预期产出

滚动示例随当前页创建/销毁，不形成跨页滚动状态缓存或独立的数据源清理任务。

## 与前后方案的关系

基于第 02 份统一封装，依次验证 UI-only 页的不同子树结构；下一份为 TextField。

## 方案正文

`scroll_area::scene()` 内包括 Vertical/Horizontal/Both、Always/Hidden、Auto transition、ScrollIntoView 等组合，按钮的 `toggle_auto_content`、`trigger_scroll_into_view` 依赖各自 UI 子树中的目标实体，并没有 Gallery 预先建立的 Model Resource。

`OnEnter(ScrollArea)` 复用原 `pages::scroll_area()`，通过 `page(ScrollArea, ...)` 挂到 PageHost；`OnExit(ScrollArea)` 同步销毁整个页面 UI 子树。滚动偏移、内容增删与布局状态由新 UI Component 自然重置，避免误清全局 Widgetry ScrollArea 插件的注册或其他页面的 ScrollArea。对点击后排队的 ScrollIntoView 操作，验证切页后不会访问已移除的目标；遇到已销毁实体应该遵循现有安全返回语义，不创建新通用任务调度器。

保留正常模式的公共页面 ScrollArea 包裹结构和颜色目录，不能因本页内部也使用 ScrollArea 而重复或遗漏包装；benchmark 模式仍由公共 `page()` 外壳决定。

**源代码：** [scroll_area.rs](https://github.com/slc90/bevy_widgetry/blob/main/gallery/src/pages/scroll_area.rs)。
